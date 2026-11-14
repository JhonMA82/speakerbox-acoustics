//! Lectura y escritura de curvas FRD y ZMA.
//!
//! Formato: una línea por punto, tres columnas separadas por espacios.
//!
//! ```text
//! * FRD: frecuencia (Hz), magnitud (dB), fase (grados)
//! 1000 87.3 -12.5
//!
//! * ZMA: frecuencia (Hz), magnitud (ohm), fase (grados)
//! 1000 8.42 13.0
//! ```
//!
//! - FRD se convierte a amplitud compleja `10^(dB/20)·e^{jφ}`.
//! - ZMA se convierte a impedancia compleja `|Z|·e^{jφ}`.
//! - La fase se interpreta **siempre en grados**, como en los formatos de
//!   origen.
//! - Se ignoran líneas vacías y las que empiezan por `*`, `#` o `;`.
//! - Se rechazan columnas extra, valores no numéricos, frecuencias no
//!   crecientes y magnitudes no positivas. **No se asume calibración**: ni
//!   1 V, ni 2.83 V, ni 1 W.

use crate::error::CurveParseError;
use num_complex::Complex64;
use speakerbox_circuits::ComplexCurve;

/// Origen de una curva leída.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurveKind {
    /// Respuesta en dB SPL con fase en grados.
    SplDb,
    /// Impedancia en ohmios con fase en grados.
    ImpedanceOhm,
}

/// Curva leída de un fichero, con su tipo.
#[derive(Clone, Debug, PartialEq)]
pub struct ParsedCurve {
    /// Tipo de magnitud.
    pub kind: CurveKind,
    /// Valores complejos ya convertidos.
    pub curve: ComplexCurve,
}

/// Lee un FRD (magnitud en dB, fase en grados).
///
/// # Errores
///
/// Un `Vec<CurveParseError>` con una entrada por línea inválida.
pub fn parse_frd(text: &str) -> Result<ParsedCurve, Vec<CurveParseError>> {
    let (freqs, magnitudes_db, phases_deg, lines) = parse_rows(text)?;
    let mut errors = Vec::new();
    let mut values = Vec::with_capacity(freqs.len());
    for ((db, phase), line) in magnitudes_db.iter().zip(&phases_deg).zip(&lines) {
        let magnitude = 10.0_f64.powf(db / 20.0);
        if !magnitude.is_finite() || magnitude <= 0.0 {
            errors.push(CurveParseError {
                line: *line,
                message: "la magnitud en dB no es representable".to_string(),
            });
            continue;
        }
        values.push(from_modulus_deg(magnitude, *phase));
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let curve = ComplexCurve::new(freqs, values).map_err(|error| {
        vec![CurveParseError {
            line: 1,
            message: error.to_string(),
        }]
    })?;
    Ok(ParsedCurve {
        kind: CurveKind::SplDb,
        curve,
    })
}

/// Lee un ZMA (magnitud en ohmios, fase en grados).
///
/// # Errores
///
/// Un `Vec<CurveParseError>` con una entrada por línea inválida.
pub fn parse_zma(text: &str) -> Result<ParsedCurve, Vec<CurveParseError>> {
    let (freqs, magnitudes, phases_deg, lines) = parse_rows(text)?;
    let mut errors = Vec::new();
    let mut values = Vec::with_capacity(freqs.len());
    for ((magnitude, phase), line) in magnitudes.iter().zip(&phases_deg).zip(&lines) {
        if !magnitude.is_finite() || *magnitude <= 0.0 {
            errors.push(CurveParseError {
                line: *line,
                message: "la magnitud de impedancia debe ser > 0 Ω".to_string(),
            });
            continue;
        }
        values.push(from_modulus_deg(*magnitude, *phase));
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let curve = ComplexCurve::new(freqs, values).map_err(|error| {
        vec![CurveParseError {
            line: 1,
            message: error.to_string(),
        }]
    })?;
    Ok(ParsedCurve {
        kind: CurveKind::ImpedanceOhm,
        curve,
    })
}

/// Serializa una curva de impedancia como ZMA.
///
/// # Errores
///
/// [`crate::MeasureError::InvalidParameter`] si algún punto tiene magnitud
/// nula: el parser rechaza un ZMA con `|Z| = 0` y el serializador no debe
/// producir texto que no pueda releerse.
pub fn write_zma(curve: &ComplexCurve) -> crate::MeasureResult<String> {
    let mut out = String::from("* speakerbox-measure ZMA: frequency Hz, |Z| ohm, phase deg\n");
    for (freq, value) in curve.freq_hz().iter().zip(curve.values()) {
        let magnitude = value.norm();
        if magnitude <= 0.0 || !magnitude.is_finite() {
            return Err(crate::MeasureError::InvalidParameter(
                "la curva contiene una impedancia nula o no finita",
            ));
        }
        out.push_str(&format!(
            "{} {} {}\n",
            freq,
            magnitude,
            value.arg().to_degrees()
        ));
    }
    Ok(out)
}

/// Serializa una curva de respuesta como FRD (dB re 1).
///
/// # Errores
///
/// [`crate::MeasureError::InvalidParameter`] si la magnitud no se puede
/// representar en dB y volver a leerse como un valor finito.
pub fn write_frd(curve: &ComplexCurve) -> crate::MeasureResult<String> {
    let mut out = String::from("* speakerbox-measure FRD: frequency Hz, magnitude dB, phase deg\n");
    for (freq, value) in curve.freq_hz().iter().zip(curve.values()) {
        let magnitude = value.norm();
        let db = 20.0 * magnitude.log10();
        if magnitude <= 0.0 || !db.is_finite() || !10.0_f64.powf(db / 20.0).is_finite() {
            return Err(crate::MeasureError::InvalidParameter(
                "la curva contiene una magnitud no serializable",
            ));
        }
        out.push_str(&format!("{} {} {}\n", freq, db, value.arg().to_degrees()));
    }
    Ok(out)
}

fn from_modulus_deg(modulus: f64, phase_deg: f64) -> Complex64 {
    Complex64::from_polar(modulus, phase_deg.to_radians())
}

type ParsedRows = (Vec<f64>, Vec<f64>, Vec<f64>, Vec<usize>);

fn parse_rows(text: &str) -> Result<ParsedRows, Vec<CurveParseError>> {
    let mut errors = Vec::new();
    let mut freqs = Vec::new();
    let mut magnitudes = Vec::new();
    let mut phases = Vec::new();
    let mut lines = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let content = raw.trim();
        if content.is_empty()
            || content.starts_with('*')
            || content.starts_with('#')
            || content.starts_with(';')
        {
            continue;
        }
        let columns: Vec<&str> = content.split_whitespace().collect();
        if columns.len() != 3 {
            errors.push(CurveParseError {
                line,
                message: format!("se esperaban 3 columnas y hay {}", columns.len()),
            });
            continue;
        }
        let mut parsed = [0.0_f64; 3];
        let mut ok = true;
        for (slot, token) in parsed.iter_mut().zip(&columns) {
            match token.parse::<f64>() {
                Ok(value) if value.is_finite() => *slot = value,
                _ => {
                    errors.push(CurveParseError {
                        line,
                        message: format!("valor no numérico: {token}"),
                    });
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            continue;
        }
        if parsed[0] <= 0.0 {
            errors.push(CurveParseError {
                line,
                message: "la frecuencia debe ser > 0 Hz".to_string(),
            });
            continue;
        }
        if let Some(previous) = freqs.last() {
            if parsed[0] <= *previous {
                errors.push(CurveParseError {
                    line,
                    message: "la frecuencia no crece respecto a la línea anterior".to_string(),
                });
                continue;
            }
        }
        freqs.push(parsed[0]);
        magnitudes.push(parsed[1]);
        phases.push(parsed[2]);
        lines.push(line);
    }
    if errors.is_empty() && freqs.is_empty() {
        errors.push(CurveParseError {
            line: 1,
            message: "fichero vacío o solo con comentarios".to_string(),
        });
    }
    if errors.is_empty() {
        Ok((freqs, magnitudes, phases, lines))
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frd_uses_db_and_degrees() {
        let parsed = parse_frd("* cabecera\n100 20 90\n200 20 0\n").unwrap();
        assert_eq!(parsed.kind, CurveKind::SplDb);
        assert!((parsed.curve.values()[0].norm() - 10.0).abs() < 1e-12);
        assert!((parsed.curve.values()[0].arg() - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    }

    #[test]
    fn zma_keeps_the_impedance_modulus() {
        let parsed = parse_zma("100 8.0 0\n200 12.0 45\n").unwrap();
        assert_eq!(parsed.kind, CurveKind::ImpedanceOhm);
        assert!((parsed.curve.values()[0].norm() - 8.0).abs() < 1e-12);
        assert!((parsed.curve.values()[1].arg().to_degrees() - 45.0).abs() < 1e-12);
    }

    #[test]
    fn round_trips_zma_and_frd() {
        let zma = "100 8.0 0\n200 12.0 45\n1000 16.0 -30\n";
        let parsed = parse_zma(zma).unwrap();
        let reparsed = parse_zma(&write_zma(&parsed.curve).unwrap()).unwrap();
        for (a, b) in parsed.curve.values().iter().zip(reparsed.curve.values()) {
            assert!((a - b).norm() < 1e-12);
        }

        let frd = "* respuesta\n100 -6.0 10\n200 -3.0 -20\n";
        let parsed = parse_frd(frd).unwrap();
        let reparsed = parse_frd(&write_frd(&parsed.curve).unwrap()).unwrap();
        for (a, b) in parsed.curve.values().iter().zip(reparsed.curve.values()) {
            assert!((a - b).norm() < 1e-12);
        }
    }

    #[test]
    fn refuses_to_serialize_values_the_parser_would_reject() {
        let zero = ComplexCurve::new(
            vec![100.0, 1000.0],
            vec![Complex64::new(8.0, 0.0), Complex64::new(0.0, 0.0)],
        )
        .unwrap();
        assert!(write_zma(&zero).is_err());
        let huge = ComplexCurve::new(
            vec![100.0, 1000.0],
            vec![Complex64::new(1.0, 0.0), Complex64::new(f64::MAX, 0.0)],
        )
        .unwrap();
        assert!(write_frd(&huge).is_err());
    }

    #[test]
    fn rejects_malformed_rows_with_line_numbers() {
        let errors = parse_zma("100 8 0\n200 nope 0\n8 9 10\n").unwrap_err();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].line, 2);
        assert_eq!(errors[1].line, 3);
        assert!(parse_zma("100 8 0\n200 9 0\n300 10 0 extra\n").is_err());
        assert!(parse_zma("100 8 0\n100 9 0\n").is_err());
        assert!(parse_zma("100 0 0\n200 9 0\n").is_err());
        assert!(parse_zma("* solo comentario\n").is_err());
    }

    #[test]
    fn non_positive_magnitudes_report_their_own_line() {
        let errors = parse_zma("10 8 0\n20 9 0\n30 0 0\n40 -1 0\n").unwrap_err();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].line, 3);
        assert_eq!(errors[1].line, 4);
    }
}
