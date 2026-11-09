//! Validación del motor contra datos exportados por el software de referencia.
//!
//! El software de referencia exporta, desde cualquier ventana de resultados, un fichero
//! tabulado (`File > Export > All Chart Data`). [`OraclePoint::parse_tsv`]
//! lee ese formato y [`validate`] compara la respuesta del motor con él.

use crate::system::System;

/// Una fila del export del software de referencia.
#[derive(Clone, Copy, Debug)]
pub struct OraclePoint {
    pub freq_hz: f64,
    pub spl_db: f64,
    pub ze_mag: f64,
    pub ze_phase_deg: f64,
    pub xd_mm: f64,
}

impl OraclePoint {
    /// Lee un fichero tabulado exportado por el software de referencia.
    ///
    /// Columnas usadas: `Freq`, `SPL`, `Ze`, `Xd`, `ZePhase` (índices 0, 4,
    /// 5, 6 y 15). Acepta punto o coma como separador decimal. Las filas
    /// inválidas se descartan.
    pub fn parse_tsv(text: &str) -> Vec<Self> {
        text.lines()
            .skip(1)
            .filter_map(|line| {
                let cols: Vec<f64> = line
                    .split('\t')
                    .filter_map(|x| {
                        let text = x.trim();
                        text.parse()
                            .ok()
                            .or_else(|| text.replace(',', ".").parse().ok())
                    })
                    .collect();
                if cols.len() >= 16 {
                    Some(Self {
                        freq_hz: cols[0],
                        spl_db: cols[4],
                        ze_mag: cols[5],
                        ze_phase_deg: cols[15],
                        xd_mm: cols[6],
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Lee un CSV simple `freq,spl,ze,xd` (formato de los datos dorados).
    pub fn parse_csv(text: &str) -> Vec<Self> {
        text.lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .filter_map(|line| {
                let cols: Vec<f64> = line
                    .split(',')
                    .filter_map(|x| x.trim().parse().ok())
                    .collect();
                if cols.len() >= 4 {
                    Some(Self {
                        freq_hz: cols[0],
                        spl_db: cols[1],
                        ze_mag: cols[2],
                        ze_phase_deg: 0.0,
                        xd_mm: cols[3],
                    })
                } else {
                    None
                }
            })
            .collect()
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
    pub n: usize,
    pub spl_rmse_db: f64,
    pub spl_max_abs_db: f64,
    pub ze_rel_rmse: f64,
    pub ze_max_rel: f64,
    pub xd_rel_rmse: f64,
    pub xd_max_rel: f64,
    pub points: Vec<ValidationPoint>,
}

impl ValidationReport {
    /// ¿Cumple las tolerancias de producción indicadas (RMS)?
    pub fn passes(&self, spl_rmse_db: f64, spl_max_db: f64, ze_rel: f64, xd_rel: f64) -> bool {
        self.spl_rmse_db <= spl_rmse_db
            && self.spl_max_abs_db <= spl_max_db
            && self.ze_rel_rmse <= ze_rel
            && self.xd_rel_rmse <= xd_rel
    }

    /// ¿Cumple también las tolerancias **puntuales máximas** de `Ze` y `Xd`?
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
        self.spl_rmse_db <= spl_rmse_db
            && self.spl_max_abs_db <= spl_max_db
            && self.ze_rel_rmse <= ze_rel_rmse
            && self.ze_max_rel <= ze_rel_max
            && self.xd_rel_rmse <= xd_rel_rmse
            && self.xd_max_rel <= xd_rel_max
    }
}

/// Compara la respuesta del sistema con el oráculo.
///
/// `System` reporta desplazamiento de **pico**, igual que el software de referencia, así que no
/// se aplica ninguna conversión adicional. Los puntos con `Xd <= 0.02 mm` se
/// excluyen del error relativo de desplazamiento (el valor es demasiado
/// pequeño para una comparación significativa).
pub fn validate(system: &System, oracle: &[OraclePoint]) -> ValidationReport {
    let mut points = Vec::with_capacity(oracle.len());
    let mut sum_spl2 = 0.0;
    let mut max_spl = 0.0_f64;
    let mut sum_ze2 = 0.0;
    let mut max_ze = 0.0_f64;
    let mut sum_xd2 = 0.0;
    let mut max_xd = 0.0_f64;
    let mut n_xd = 0usize;

    for o in oracle {
        let p = system.response_at(o.freq_hz);
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
        points.push(ValidationPoint {
            freq_hz: o.freq_hz,
            spl_error_db: spl_error,
            ze_rel_error: ze_error,
            xd_rel_error: xd_error,
        });
    }

    let n = oracle.len().max(1);
    ValidationReport {
        n: oracle.len(),
        spl_rmse_db: (sum_spl2 / n as f64).sqrt(),
        spl_max_abs_db: max_spl,
        ze_rel_rmse: (sum_ze2 / n as f64).sqrt(),
        ze_max_rel: max_ze,
        xd_rel_rmse: if n_xd > 0 {
            (sum_xd2 / n_xd as f64).sqrt()
        } else {
            0.0
        },
        xd_max_rel: max_xd,
        points,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tsv_with_comma_decimals() {
        let text = "Freq (hertz)\tRa (norm)\tXa (norm)\tZa (norm)\tSPL (dB)\tZe (ohms)\tXd (mm)\tWPhase (deg)\tUPhase (deg)\tCPhase (deg)\tDelay (msec)\tEfficiency (%)\tEin (volts)\tPin (watts)\tIin (amps)\tZePhase (deg)\n\
                    10,00\t0,000926\t0,201729\t0,201731\t55,919659\t11,482729\t2,798048\t120,602443\t120,602443\t120,602443\t9,026094\t0,082910\t2,830000\t0,426616\t0,246457\t52,290425\n";
        let rows = OraclePoint::parse_tsv(text);
        assert_eq!(rows.len(), 1);
        assert!((rows[0].freq_hz - 10.0).abs() < 1.0e-12);
        assert!((rows[0].spl_db - 55.919659).abs() < 1.0e-9);
        assert!((rows[0].ze_mag - 11.482729).abs() < 1.0e-9);
        assert!((rows[0].xd_mm - 2.798048).abs() < 1.0e-9);
        assert!((rows[0].ze_phase_deg - 52.290425).abs() < 1.0e-9);
    }
}
