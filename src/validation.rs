//! Validación del motor contra datos exportados por el software de referencia.
//!
//! El software de referencia exporta, desde cualquier ventana de resultados, un fichero
//! tabulado (`File > Export > All Chart Data`). [`OraclePoint::parse_tsv`]
//! lee ese formato y [`validate`] compara la respuesta del motor con él.
//!
//! ## Contrato de validación
//!
//! [`validate`] compara SPL, módulo de impedancia eléctrica (`Ze`) y
//! desplazamiento de pico (`Xd`). **No compara la fase de impedancia**
//! (`ze_phase_deg`): el campo se parsea y se conserva para trazabilidad, pero
//! no existe una tolerancia validada para esa magnitud. No se debe declarar
//! una validación completa de la impedancia si solo se compara su módulo.
//!
//! Un oráculo vacío, con filas incompletas o con valores no físicos no puede
//! aprobarse: [`ValidationReport::passes`] y
//! [`ValidationReport::passes_pointwise`] exigen `n > 0` y
//! `invalid_points == 0`.

use crate::system::System;

/// Una fila del export del software de referencia.
#[derive(Clone, Copy, Debug)]
pub struct OraclePoint {
    pub freq_hz: f64,
    pub spl_db: f64,
    pub ze_mag: f64,
    /// Fase de la impedancia eléctrica (grados).
    ///
    /// Se parsea desde el export, pero [`validate`] no la compara.
    pub ze_phase_deg: f64,
    pub xd_mm: f64,
}

/// Error de parseo o de contenido de un oráculo.
#[derive(Clone, Debug, PartialEq)]
pub enum ValidationError {
    /// El texto no contiene ninguna fila de datos.
    Empty,
    /// Una fila tiene menos columnas de las necesarias.
    MissingColumns {
        /// Línea del texto (1-based).
        line: usize,
        /// Columnas mínimas esperadas.
        expected: usize,
        /// Columnas encontradas.
        found: usize,
    },
    /// Una fila tiene un número de columnas distinto del esperado.
    InvalidColumns {
        /// Línea del texto (1-based).
        line: usize,
        /// Columnas exactas esperadas.
        expected: usize,
        /// Columnas encontradas.
        found: usize,
    },
    /// Una celda obligatoria no es un número finito.
    InvalidCell {
        /// Línea del texto (1-based).
        line: usize,
        /// Columna (1-based).
        column: usize,
        /// Texto original de la celda.
        value: String,
    },
    /// La frecuencia no es finita o no es estrictamente positiva.
    InvalidFrequency {
        /// Línea del texto (1-based).
        line: usize,
        /// Valor leído.
        value: f64,
    },
    /// Un valor de referencia no cumple su restricción física.
    InvalidValue {
        /// Línea del texto (1-based).
        line: usize,
        /// Columna (1-based).
        column: usize,
        /// Valor leído.
        value: f64,
    },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str("el oráculo no contiene filas de datos"),
            Self::MissingColumns {
                line,
                expected,
                found,
            } => write!(
                f,
                "línea {line}: se esperaban al menos {expected} columnas, hay {found}"
            ),
            Self::InvalidColumns {
                line,
                expected,
                found,
            } => write!(
                f,
                "línea {line}: se esperaban exactamente {expected} columnas, hay {found}"
            ),
            Self::InvalidCell {
                line,
                column,
                value,
            } => write!(
                f,
                "línea {line}, columna {column}: valor no numérico o no finito {value:?}"
            ),
            Self::InvalidFrequency { line, value } => {
                write!(f, "línea {line}: frecuencia inválida {value} Hz")
            }
            Self::InvalidValue {
                line,
                column,
                value,
            } => write!(f, "línea {line}, columna {column}: valor inválido {value}"),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Interpreta una celda numérica sin perder su posición.
fn numeric(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    trimmed
        .parse::<f64>()
        .ok()
        .or_else(|| trimmed.replace(',', ".").parse::<f64>().ok())
        .filter(|value| value.is_finite())
}

fn parse_cell(text: &str, line: usize, column: usize) -> Result<f64, ValidationError> {
    numeric(text).ok_or_else(|| ValidationError::InvalidCell {
        line,
        column,
        value: text.trim().to_string(),
    })
}

impl OraclePoint {
    /// Lee un fichero tabulado exportado por el software de referencia.
    ///
    /// Columnas usadas: `Freq`, `SPL`, `Ze`, `Xd`, `ZePhase` (índices 0, 4,
    /// 5, 6 y 15). Acepta punto o coma como separador decimal. La cabecera se
    /// detecta por la primera celda no numérica que contenga `Freq`.
    ///
    /// A diferencia de la versión anterior, no descarta celdas inválidas ni
    /// desplaza columnas: una celda obligatoria vacía o no finita devuelve
    /// [`ValidationError`]. Las líneas vacías y las que empiezan por `#` se
    /// ignoran.
    pub fn parse_tsv(text: &str) -> Result<Vec<Self>, ValidationError> {
        let mut rows = Vec::new();
        let mut first_line = true;

        for (index, line) in text.lines().enumerate() {
            let line_number = index + 1;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            if first_line {
                first_line = false;
                if numeric(cols[0]).is_none() && trimmed.to_ascii_lowercase().contains("freq") {
                    continue;
                }
            }
            if cols.len() < 16 {
                return Err(ValidationError::MissingColumns {
                    line: line_number,
                    expected: 16,
                    found: cols.len(),
                });
            }

            let freq_hz = parse_cell(cols[0], line_number, 1)?;
            let spl_db = parse_cell(cols[4], line_number, 5)?;
            let ze_mag = parse_cell(cols[5], line_number, 6)?;
            let xd_mm = parse_cell(cols[6], line_number, 7)?;
            let ze_phase_deg = parse_cell(cols[15], line_number, 16)?;

            if freq_hz <= 0.0 {
                return Err(ValidationError::InvalidFrequency {
                    line: line_number,
                    value: freq_hz,
                });
            }
            if ze_mag <= 0.0 {
                return Err(ValidationError::InvalidValue {
                    line: line_number,
                    column: 6,
                    value: ze_mag,
                });
            }
            if xd_mm < 0.0 {
                return Err(ValidationError::InvalidValue {
                    line: line_number,
                    column: 7,
                    value: xd_mm,
                });
            }

            rows.push(Self {
                freq_hz,
                spl_db,
                ze_mag,
                ze_phase_deg,
                xd_mm,
            });
        }

        if rows.is_empty() {
            Err(ValidationError::Empty)
        } else {
            Ok(rows)
        }
    }

    /// Lee un CSV simple `freq,spl,ze,xd` (formato de los datos dorados).
    ///
    /// Las líneas vacías y las que empiezan por `#` se ignoran. La fase de
    /// impedancia no está en este formato y se deja a `0.0`; por tanto, este
    /// parser no aporta información de fase.
    pub fn parse_csv(text: &str) -> Result<Vec<Self>, ValidationError> {
        let mut rows = Vec::new();
        let mut first_line = true;

        for (index, line) in text.lines().enumerate() {
            let line_number = index + 1;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let cols: Vec<&str> = line.split(',').collect();
            if first_line {
                first_line = false;
                if numeric(cols[0]).is_none() && trimmed.to_ascii_lowercase().contains("freq") {
                    continue;
                }
            }
            if cols.len() != 4 {
                return Err(ValidationError::InvalidColumns {
                    line: line_number,
                    expected: 4,
                    found: cols.len(),
                });
            }

            let freq_hz = parse_cell(cols[0], line_number, 1)?;
            let spl_db = parse_cell(cols[1], line_number, 2)?;
            let ze_mag = parse_cell(cols[2], line_number, 3)?;
            let xd_mm = parse_cell(cols[3], line_number, 4)?;

            if freq_hz <= 0.0 {
                return Err(ValidationError::InvalidFrequency {
                    line: line_number,
                    value: freq_hz,
                });
            }
            if ze_mag <= 0.0 {
                return Err(ValidationError::InvalidValue {
                    line: line_number,
                    column: 3,
                    value: ze_mag,
                });
            }
            if xd_mm < 0.0 {
                return Err(ValidationError::InvalidValue {
                    line: line_number,
                    column: 4,
                    value: xd_mm,
                });
            }

            rows.push(Self {
                freq_hz,
                spl_db,
                ze_mag,
                ze_phase_deg: 0.0,
                xd_mm,
            });
        }

        if rows.is_empty() {
            Err(ValidationError::Empty)
        } else {
            Ok(rows)
        }
    }
}

/// Error de un punto concreto.
#[derive(Clone, Copy, Debug)]
pub struct ValidationPoint {
    pub freq_hz: f64,
    pub spl_error_db: f64,
    pub ze_rel_error: f64,
    pub xd_rel_error: f64,
}

/// Resumen de la comparación.
#[derive(Clone, Debug)]
pub struct ValidationReport {
    /// Puntos realmente comparados (válidos).
    pub n: usize,
    /// Puntos del oráculo rechazados por no ser físicos, o puntos del sistema
    /// que no produjeron una respuesta válida.
    pub invalid_points: usize,
    pub spl_rmse_db: f64,
    pub spl_max_abs_db: f64,
    pub ze_rel_rmse: f64,
    pub ze_max_rel: f64,
    pub xd_rel_rmse: f64,
    pub xd_max_rel: f64,
    pub points: Vec<ValidationPoint>,
}

impl ValidationReport {
    /// ¿El informe comparó al menos un punto, no rechazó ninguno y todas las
    /// métricas son finitas?
    pub fn is_valid(&self) -> bool {
        self.n > 0
            && self.invalid_points == 0
            && self.spl_rmse_db.is_finite()
            && self.spl_max_abs_db.is_finite()
            && self.ze_rel_rmse.is_finite()
            && self.ze_max_rel.is_finite()
            && self.xd_rel_rmse.is_finite()
            && self.xd_max_rel.is_finite()
    }

    /// ¿Cumple las tolerancias de producción indicadas (RMS)?
    ///
    /// Un oráculo vacío o con puntos inválidos nunca aprueba.
    pub fn passes(&self, spl_rmse_db: f64, spl_max_db: f64, ze_rel: f64, xd_rel: f64) -> bool {
        self.is_valid()
            && self.spl_rmse_db <= spl_rmse_db
            && self.spl_max_abs_db <= spl_max_db
            && self.ze_rel_rmse <= ze_rel
            && self.xd_rel_rmse <= xd_rel
    }

    /// ¿Cumple también las tolerancias **puntuales máximas** de `Ze` y `Xd`?
    ///
    /// Un oráculo vacío o con puntos inválidos nunca aprueba.
    #[allow(clippy::too_many_arguments)]
    pub fn passes_pointwise(
        &self,
        spl_rmse_db: f64,
        spl_max_db: f64,
        ze_rel_rmse: f64,
        ze_rel_max: f64,
        xd_rel_rmse: f64,
        xd_rel_max: f64,
    ) -> bool {
        self.is_valid()
            && self.spl_rmse_db <= spl_rmse_db
            && self.spl_max_abs_db <= spl_max_db
            && self.ze_rel_rmse <= ze_rel_rmse
            && self.ze_max_rel <= ze_rel_max
            && self.xd_rel_rmse <= xd_rel_rmse
            && self.xd_max_rel <= xd_rel_max
    }
}

/// Comprueba que un punto del oráculo sea físicamente comparable.
fn valid_oracle_point(point: &OraclePoint) -> bool {
    point.freq_hz.is_finite()
        && point.freq_hz > 0.0
        && point.spl_db.is_finite()
        && point.ze_mag.is_finite()
        && point.ze_mag > 0.0
        && point.ze_phase_deg.is_finite()
        && point.xd_mm.is_finite()
        && point.xd_mm >= 0.0
}

/// Compara la respuesta del sistema con el oráculo.
///
/// `System` reporta desplazamiento de **pico**, igual que el software de referencia, así que no
/// se aplica ninguna conversión adicional. Los puntos con `Xd <= 0.02 mm` se
/// excluyen del error relativo de desplazamiento (el valor es demasiado
/// pequeño para una comparación significativa).
///
/// Los puntos del oráculo no finitos o no físicos, y los puntos en los que el
/// sistema no produce una respuesta válida, se cuentan en
/// [`ValidationReport::invalid_points`] y no se comparan. Un informe sin
/// puntos válidos o con puntos inválidos no aprueba.
///
/// La fase de impedancia (`ze_phase_deg`) **no se compara**; ver la
/// documentación del módulo.
pub fn validate(system: &System, oracle: &[OraclePoint]) -> ValidationReport {
    let mut points = Vec::with_capacity(oracle.len());
    let mut sum_spl2 = 0.0;
    let mut max_spl = 0.0_f64;
    let mut sum_ze2 = 0.0;
    let mut max_ze = 0.0_f64;
    let mut sum_xd2 = 0.0;
    let mut max_xd = 0.0_f64;
    let mut n = 0usize;
    let mut n_xd = 0usize;
    let mut invalid_points = 0usize;

    for o in oracle {
        if !valid_oracle_point(o) {
            invalid_points += 1;
            continue;
        }
        let p = system.response_at(o.freq_hz);
        if p.validate().is_err() {
            invalid_points += 1;
            continue;
        }

        let spl_error = p.spl_db - o.spl_db;
        let ze_error = if o.ze_mag.abs() > 1.0e-12 {
            (p.ze_mag - o.ze_mag) / o.ze_mag
        } else {
            0.0
        };
        let xd_peak = p.displacement_mm;
        let xd_error = if o.xd_mm > 0.02 {
            (xd_peak - o.xd_mm) / o.xd_mm
        } else {
            0.0
        };

        sum_spl2 += spl_error * spl_error;
        max_spl = max_spl.max(spl_error.abs());
        sum_ze2 += ze_error * ze_error;
        max_ze = max_ze.max(ze_error.abs());
        if o.xd_mm > 0.02 {
            sum_xd2 += xd_error * xd_error;
            max_xd = max_xd.max(xd_error.abs());
            n_xd += 1;
        }
        n += 1;
        points.push(ValidationPoint {
            freq_hz: o.freq_hz,
            spl_error_db: spl_error,
            ze_rel_error: ze_error,
            xd_rel_error: xd_error,
        });
    }

    let empty = n == 0;
    ValidationReport {
        n,
        invalid_points,
        spl_rmse_db: if empty {
            f64::NAN
        } else {
            (sum_spl2 / n as f64).sqrt()
        },
        spl_max_abs_db: if empty { f64::NAN } else { max_spl },
        ze_rel_rmse: if empty {
            f64::NAN
        } else {
            (sum_ze2 / n as f64).sqrt()
        },
        ze_max_rel: if empty { f64::NAN } else { max_ze },
        xd_rel_rmse: if empty {
            f64::NAN
        } else if n_xd > 0 {
            (sum_xd2 / n_xd as f64).sqrt()
        } else {
            0.0
        },
        xd_max_rel: if empty { f64::NAN } else { max_xd },
        points,
    }
}

/// Diagnóstico de fase eléctrica, separado de las tolerancias históricas.
/// No declara aprobación física por defecto: el usuario fija el criterio.
#[derive(Clone, Copy, Debug)]
pub struct PhaseValidationReport {
    pub n: usize,
    pub invalid_points: usize,
    pub rmse_deg: f64,
    pub max_abs_deg: f64,
}

/// Compara fase de impedancia con diferencia circular en [-180,180).
/// Requiere un oráculo con fase REAL (p.ej. TSV); CSV de cuatro columnas no
/// contiene fase y no debe usarse aquí. No se inventa una tolerancia.
pub fn validate_impedance_phase(system: &System, oracle: &[OraclePoint]) -> PhaseValidationReport {
    let mut sum = 0.0;
    let mut max_abs = 0.0_f64;
    let mut n = 0;
    let mut invalid_points = 0;
    for o in oracle {
        if !valid_oracle_point(o) {
            invalid_points += 1;
            continue;
        }
        match system.try_response_at(o.freq_hz) {
            Ok(p) => {
                let delta = (p.ze_phase_deg - o.ze_phase_deg + 180.).rem_euclid(360.) - 180.;
                sum += delta * delta;
                max_abs = max_abs.max(delta.abs());
                n += 1;
            }
            Err(_) => invalid_points += 1,
        }
    }
    PhaseValidationReport {
        n,
        invalid_points,
        rmse_deg: if n > 0 {
            (sum / n as f64).sqrt()
        } else {
            f64::NAN
        },
        max_abs_deg: if n > 0 { max_abs } else { f64::NAN },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Air, Driver};

    const HEADER: &str = "Freq (hertz)\tRa (norm)\tXa (norm)\tZa (norm)\tSPL (dB)\tZe (ohms)\tXd (mm)\tWPhase (deg)\tUPhase (deg)\tCPhase (deg)\tDelay (msec)\tEfficiency (%)\tEin (volts)\tPin (watts)\tIin (amps)\tZePhase (deg)\n";
    const ROW: &str = "10,00\t0,000926\t0,201729\t0,201731\t55,919659\t11,482729\t2,798048\t120,602443\t120,602443\t120,602443\t9,026094\t0,082910\t2,830000\t0,426616\t0,246457\t52,290425\n";

    fn system() -> System {
        System::new(
            Air::default(),
            Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1),
        )
    }

    #[test]
    fn parses_tsv_with_comma_decimals() {
        let text = format!("{HEADER}{ROW}");
        let rows = OraclePoint::parse_tsv(&text).unwrap();
        assert_eq!(rows.len(), 1);
        assert!((rows[0].freq_hz - 10.0).abs() < 1.0e-12);
        assert!((rows[0].spl_db - 55.919659).abs() < 1.0e-9);
        assert!((rows[0].ze_mag - 11.482729).abs() < 1.0e-9);
        assert!((rows[0].xd_mm - 2.798048).abs() < 1.0e-9);
        assert!((rows[0].ze_phase_deg - 52.290425).abs() < 1.0e-9);
    }

    #[test]
    fn tsv_preserves_column_positions_with_an_empty_optional_cell() {
        // Columna 2 (Za norm) vacía: el parser anterior la descartaba y
        // desplazaba el resto de columnas.
        let row = "10,00\t0,000926\t\t0,201731\t55,919659\t11,482729\t2,798048\t120,602443\t120,602443\t120,602443\t9,026094\t0,082910\t2,830000\t0,426616\t0,246457\t52,290425\n";
        let rows = OraclePoint::parse_tsv(row).unwrap();
        assert_eq!(rows.len(), 1);
        assert!((rows[0].spl_db - 55.919659).abs() < 1.0e-9);
        assert!((rows[0].ze_mag - 11.482729).abs() < 1.0e-9);
        assert!((rows[0].xd_mm - 2.798048).abs() < 1.0e-9);
        assert!((rows[0].ze_phase_deg - 52.290425).abs() < 1.0e-9);
    }

    #[test]
    fn tsv_rejects_missing_columns_and_invalid_cells() {
        let short = "Freq\tSPL\tZe\tXd\n10\t50\t8\t1\n";
        assert!(matches!(
            OraclePoint::parse_tsv(short),
            Err(ValidationError::MissingColumns { .. })
        ));

        let mut cols: Vec<&str> = ROW.trim_end().split('\t').collect();
        cols[4] = "no-numérico";
        let bad = cols.join("\t");
        assert!(matches!(
            OraclePoint::parse_tsv(&bad),
            Err(ValidationError::InvalidCell { column: 5, .. })
        ));
    }

    #[test]
    fn tsv_rejects_invalid_frequencies_and_values() {
        let mut cols: Vec<&str> = ROW.trim_end().split('\t').collect();
        cols[0] = "0";
        let text = cols.join("\t");
        assert!(matches!(
            OraclePoint::parse_tsv(&text),
            Err(ValidationError::InvalidFrequency { .. })
        ));

        let mut cols: Vec<&str> = ROW.trim_end().split('\t').collect();
        cols[5] = "0";
        let text = cols.join("\t");
        assert!(matches!(
            OraclePoint::parse_tsv(&text),
            Err(ValidationError::InvalidValue { column: 6, .. })
        ));

        let mut cols: Vec<&str> = ROW.trim_end().split('\t').collect();
        cols[6] = "-1";
        let text = cols.join("\t");
        assert!(matches!(
            OraclePoint::parse_tsv(&text),
            Err(ValidationError::InvalidValue { column: 7, .. })
        ));
    }

    #[test]
    fn tsv_rejects_empty_or_header_only() {
        assert_eq!(
            OraclePoint::parse_tsv("").unwrap_err(),
            ValidationError::Empty
        );
        assert_eq!(
            OraclePoint::parse_tsv(HEADER).unwrap_err(),
            ValidationError::Empty
        );
    }

    #[test]
    fn csv_parses_and_rejects_invalid_rows() {
        let rows = OraclePoint::parse_csv("# freq,spl,ze,xd\n20,80,8,1\n").unwrap();
        assert_eq!(rows.len(), 1);
        assert!((rows[0].ze_phase_deg - 0.0).abs() < 1.0e-12);

        let rows = OraclePoint::parse_csv("freq,spl,ze,xd\n20,80,8,1\n").unwrap();
        assert_eq!(rows.len(), 1);

        assert!(matches!(
            OraclePoint::parse_csv("20,80,8\n"),
            Err(ValidationError::InvalidColumns { .. })
        ));
        assert!(matches!(
            OraclePoint::parse_csv("20,5,80,0,8,0,1,0\n"),
            Err(ValidationError::InvalidColumns {
                expected: 4,
                found: 8,
                ..
            })
        ));
        assert!(matches!(
            OraclePoint::parse_csv("20,80,8,1,extra\n"),
            Err(ValidationError::InvalidColumns { .. })
        ));
        assert!(matches!(
            OraclePoint::parse_csv("0,80,8,1\n"),
            Err(ValidationError::InvalidFrequency { .. })
        ));
        assert!(matches!(
            OraclePoint::parse_csv("20,80,0,1\n"),
            Err(ValidationError::InvalidValue { .. })
        ));
        assert_eq!(
            OraclePoint::parse_csv("# nada\n").unwrap_err(),
            ValidationError::Empty
        );
    }

    #[test]
    fn empty_oracle_never_passes() {
        let report = validate(&system(), &[]);
        assert_eq!(report.n, 0);
        assert!(!report.is_valid());
        assert!(report.spl_rmse_db.is_nan());
        assert!(!report.passes(2.0, 3.5, 0.15, 0.40));
        assert!(!report.passes_pointwise(2.0, 3.5, 0.15, 0.35, 0.15, 0.40));
    }

    #[test]
    fn invalid_oracle_points_never_pass() {
        let oracle = vec![OraclePoint {
            freq_hz: f64::NAN,
            spl_db: 80.0,
            ze_mag: 8.0,
            ze_phase_deg: 0.0,
            xd_mm: 1.0,
        }];
        let report = validate(&system(), &oracle);
        assert_eq!(report.n, 0);
        assert_eq!(report.invalid_points, 1);
        assert!(!report.is_valid());
        assert!(!report.passes(2.0, 3.5, 0.15, 0.40));
    }

    #[test]
    fn zero_drive_is_not_a_valid_report() {
        let mut sys = system();
        sys.eg = 0.0;
        let oracle = OraclePoint::parse_tsv(ROW).unwrap();
        let report = validate(&sys, &oracle);
        assert!(report.n > 0);
        assert_eq!(report.invalid_points, 0);
        assert!(!report.is_valid());
        assert!(report.spl_rmse_db.is_infinite());
        assert!(!report.passes(2.0, 3.5, 0.15, 0.40));
    }

    #[test]
    fn phase_is_parsed_but_not_compared() {
        let mut a = OraclePoint::parse_tsv(ROW).unwrap()[0];
        let mut b = a;
        b.ze_phase_deg += 180.0;
        let report_a = validate(&system(), &[a]);
        let report_b = validate(&system(), &[b]);
        assert_eq!(report_a.n, report_b.n);
        assert_eq!(report_a.ze_rel_rmse, report_b.ze_rel_rmse);
        a.ze_phase_deg = f64::NAN;
        let report_c = validate(&system(), &[a]);
        assert_eq!(report_c.invalid_points, 1);
    }
}
