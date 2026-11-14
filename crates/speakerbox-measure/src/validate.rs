//! Validación de la suma coherente contra un dataset de mediciones.
//!
//! El dataset y las tolerancias están en
//! [`docs/audits/MULTIWAY_PRESSURE_CONTRACT.md`](../../../docs/audits/MULTIWAY_PRESSURE_CONTRACT.md) §8.
//! No se ejecuta ninguna validación aquí: esta API compara las curvas cuando
//! existan y devuelve un informe con el veredicto.

use crate::curve_file::{parse_frd, ParsedCurve};
use num_complex::Complex64;
use speakerbox_circuits::ComplexCurve;
use std::f64::consts::TAU;
use std::fmt;
use std::path::Path;

/// Error de carga o de validación del dataset.
#[derive(Clone, Debug, PartialEq)]
pub struct DatasetError {
    /// Descripción del problema.
    pub message: String,
}

impl fmt::Display for DatasetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DatasetError {}

/// Metadatos declarados del dataset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DatasetMeta {
    /// Distancia de medida, en metros.
    pub distance_m: f64,
    /// Tensión de excitación, en V RMS.
    pub voltage_rms: f64,
    /// Polaridad de la vía aguda: `+1` o `-1`.
    pub polarity: i8,
    /// Ganancia lineal de la vía aguda.
    pub gain: f64,
    /// Retardo de la vía aguda, en segundos.
    pub delay_s: f64,
}

impl Default for DatasetMeta {
    fn default() -> Self {
        Self {
            distance_m: 1.0,
            voltage_rms: 2.83,
            polarity: 1,
            gain: 1.0,
            delay_s: 0.0,
        }
    }
}

/// Curvas y metadatos de un dataset.
#[derive(Clone, Debug, PartialEq)]
pub struct Dataset {
    /// Curva del grave.
    pub woofer: ParsedCurve,
    /// Curva del agudo.
    pub tweeter: ParsedCurve,
    /// Curva del sistema completo.
    pub system: ParsedCurve,
    /// Metadatos declarados.
    pub meta: DatasetMeta,
}

/// Informe de la comparación.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidationReport {
    /// Puntos comparados.
    pub points: usize,
    /// RMS del error de magnitud, en dB.
    pub rmse_db: f64,
    /// Error absoluto máximo de magnitud, en dB.
    pub max_db: f64,
    /// RMS del error circular de fase, en grados.
    pub rms_phase_deg: f64,
    /// Error absoluto máximo de fase, en grados.
    pub max_phase_deg: f64,
    /// `true` si cumple las tolerancias del contrato.
    pub passed: bool,
}

/// Carga `woofer.frd`, `tweeter.frd`, `system.frd` y `meta.txt` del directorio.
pub fn load_dataset(directory: &Path) -> Result<Dataset, DatasetError> {
    let read = |name: &str| {
        let path = directory.join(name);
        std::fs::read_to_string(&path).map_err(|error| DatasetError {
            message: format!("no se pudo leer {}: {error}", path.display()),
        })
    };
    let parse = |name: &str, text: &str| {
        parse_frd(text).map_err(|errors| DatasetError {
            message: format!("{name}: {}", format_errors(&errors)),
        })
    };
    let woofer_text = read("woofer.frd")?;
    let tweeter_text = read("tweeter.frd")?;
    let system_text = read("system.frd")?;
    let meta_text = read("meta.txt")?;
    Ok(Dataset {
        woofer: parse("woofer.frd", &woofer_text)?,
        tweeter: parse("tweeter.frd", &tweeter_text)?,
        system: parse("system.frd", &system_text)?,
        meta: parse_meta(&meta_text)?,
    })
}

/// Lee los metadatos `clave=valor`.
///
/// Claves admitidas: `distance_m`, `voltage_rms`, `polarity`, `gain`,
/// `delay_s`, `time_reference` y `note` (las dos últimas se ignoran).
pub fn parse_meta(text: &str) -> Result<DatasetMeta, DatasetError> {
    let mut meta = DatasetMeta::default();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line.split_once('=').ok_or_else(|| DatasetError {
            message: format!("meta.txt línea {}: se esperaba clave=valor", index + 1),
        })?;
        let key = key.trim();
        let value = value.trim();
        let number = || {
            value.parse::<f64>().map_err(|_| DatasetError {
                message: format!("meta.txt línea {}: valor no numérico para {key}", index + 1),
            })
        };
        match key {
            "distance_m" => meta.distance_m = number()?,
            "voltage_rms" => meta.voltage_rms = number()?,
            "polarity" => {
                let parsed = number()?;
                if parsed != 1.0 && parsed != -1.0 {
                    return Err(DatasetError {
                        message: "meta.txt: polarity debe ser +1 o -1".to_string(),
                    });
                }
                meta.polarity = if parsed > 0.0 { 1 } else { -1 };
            }
            "gain" => meta.gain = number()?,
            "delay_s" => meta.delay_s = number()?,
            "time_reference" | "note" => {}
            other => {
                return Err(DatasetError {
                    message: format!("meta.txt: clave desconocida {other}"),
                })
            }
        }
    }
    for (name, value) in [
        ("distance_m", meta.distance_m),
        ("voltage_rms", meta.voltage_rms),
        ("gain", meta.gain),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(DatasetError {
                message: format!("meta.txt: {name} debe ser finito y > 0"),
            });
        }
    }
    if !meta.delay_s.is_finite() || meta.delay_s < 0.0 {
        return Err(DatasetError {
            message: "meta.txt: delay_s debe ser finito y >= 0".to_string(),
        });
    }
    Ok(meta)
}

/// Compara la suma de las vías con la curva del sistema completo.
pub fn validate_dataset(dataset: &Dataset) -> Result<ValidationReport, DatasetError> {
    validate_curves(
        &dataset.woofer.curve,
        &dataset.tweeter.curve,
        &dataset.system.curve,
        &dataset.meta,
    )
}

/// Compara `woofer + tweeter·polarity·gain·e^(−j2πfτ)` con `system`.
///
/// La comparación usa solo las magnitudes de las curvas (dB re 1), así que no
/// depende de `p_ref`. Se excluyen los puntos a más de 12 dB del máximo medido
/// y los de magnitud nula.
pub fn validate_curves(
    woofer: &ComplexCurve,
    tweeter: &ComplexCurve,
    system: &ComplexCurve,
    meta: &DatasetMeta,
) -> Result<ValidationReport, DatasetError> {
    if system.len() < 2 {
        return Err(DatasetError {
            message: "system.frd necesita al menos dos puntos".to_string(),
        });
    }
    let peak = system
        .values()
        .iter()
        .map(|value| value.norm())
        .fold(0.0_f64, f64::max);
    if peak <= 0.0 {
        return Err(DatasetError {
            message: "system.frd no tiene magnitud positiva".to_string(),
        });
    }
    let floor = peak * 10.0_f64.powf(-12.0 / 20.0);
    let mut count = 0_usize;
    let mut squared_db = 0.0;
    let mut max_db = 0.0_f64;
    let mut squared_phase = 0.0;
    let mut max_phase = 0.0_f64;
    for (freq, measured) in system.freq_hz().iter().zip(system.values()) {
        if measured.norm() < floor {
            continue;
        }
        let at = |curve: &ComplexCurve| {
            curve.at(*freq).map_err(|error| DatasetError {
                message: format!("frecuencia {freq} fuera de banda: {error}"),
            })
        };
        let low = at(woofer)?;
        let high = at(tweeter)?;
        let rotation = Complex64::from_polar(1.0, -TAU * freq * meta.delay_s);
        let predicted = low + high * (f64::from(meta.polarity) * meta.gain) * rotation;
        if predicted.norm() <= 0.0 || measured.norm() <= 0.0 {
            continue;
        }
        let error_db = 20.0 * (predicted.norm() / measured.norm()).log10();
        squared_db += error_db * error_db;
        max_db = max_db.max(error_db.abs());
        let phase = (predicted.arg() - measured.arg() + std::f64::consts::PI).rem_euclid(TAU)
            - std::f64::consts::PI;
        let phase_deg = phase.to_degrees();
        squared_phase += phase_deg * phase_deg;
        max_phase = max_phase.max(phase_deg.abs());
        count += 1;
    }
    if count < 5 {
        return Err(DatasetError {
            message: format!("solo {count} puntos comparables; se necesitan al menos 5"),
        });
    }
    let divisor = count as f64;
    let report = ValidationReport {
        points: count,
        rmse_db: (squared_db / divisor).sqrt(),
        max_db,
        rms_phase_deg: (squared_phase / divisor).sqrt(),
        max_phase_deg: max_phase,
        passed: false,
    };
    let passed = report.rmse_db <= 1.5
        && report.max_db <= 3.0
        && report.rms_phase_deg <= 15.0
        && report.max_phase_deg <= 30.0;
    Ok(ValidationReport { passed, ..report })
}

fn format_errors(errors: &[crate::CurveParseError]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}
