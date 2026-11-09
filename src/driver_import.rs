//! Importación de drivers WinISD (`.wdr`) y OpenISD (`.owdr`).
//!
//! - `.wdr`: INI con una sección `[Driver]`, valores en SI y coma o punto
//!   decimal.
//! - `.owdr`: JSON de OpenISD; los campos de `specs.woofer` pueden venir como
//!   `{ "value": ... }` o como `{ "readings": { ... "read_value": ... } }`.
//!
//! El resultado es un [`Driver`] con las mismas convenciones que el resto de
//! la librería: `Mmd` sin masa añadida de aire, unidades SI y `Le` en henrios.
//! Los parámetros que falten se derivan de `Fs`, `Qms`, `Qes`, `Vas` y `Sd`
//! cuando es posible.
//!
//! ```rust
//! use speakerbox_acoustics::driver_import::import_driver;
//!
//! let text = "[Driver]\r\nFs=50\r\nQms=2\r\nQes=0.5\r\nVas=0.05\r\n\
//!             Re=6\r\nSd=0.02\r\nMms=0.03\r\nCms=0.0004\r\n\
//!             BL=10\r\nRms=2\r\nLe=0.001\r\nnumVC=1\r\n";
//! let imported = import_driver(text).unwrap();
//! println!("{} {}", imported.brand, imported.model);
//! ```

use std::collections::HashMap;

use serde_json::Value;

use crate::air::Air;
use crate::driver::Driver;
use crate::error::SolveError;

/// Formato de origen del driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriverFormat {
    /// WinISD `.wdr`.
    WinIsd,
    /// OpenISD `.owdr`.
    OpenIsd,
}

/// Driver importado con metadatos del archivo.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportedDriver {
    /// Driver listo para usar en cualquier topología.
    pub driver: Driver,
    /// Formato de origen.
    pub format: DriverFormat,
    /// Marca.
    pub brand: String,
    /// Modelo.
    pub model: String,
    /// Comentario.
    pub comment: String,
    /// Número de bobinas (`numVC`).
    pub voice_coils: u32,
    /// Conexión de bobinas declarada (`VCCon`), si existe.
    pub vc_connection: Option<u32>,
    /// Impedancia nominal (Ω), si existe.
    pub nominal_ohms: Option<f64>,
    /// Xmax en mm, si existe.
    pub xmax_mm: Option<f64>,
    /// Potencia admisible en W, si existe.
    pub pe_w: Option<f64>,
    /// Avisos de importación (campos derivados, valores anómalos, etc.).
    pub warnings: Vec<String>,
}

/// Error de importación.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportError {
    /// El texto no contiene una sección `[Driver]`.
    NotWinIsd,
    /// El JSON no tiene `specs.woofer`.
    NotOpenIsd,
    /// Falta un parámetro imprescindible.
    MissingField(&'static str),
    /// Una clave numérica no se pudo interpretar.
    InvalidNumber { key: String, value: String },
    /// Un parámetro del fichero no es finito o no cumple su rango físico.
    InvalidParameter(&'static str),
    /// Dos parámetros del fichero son físicamente inconsistentes.
    Inconsistent {
        /// Parámetro implicado.
        key: &'static str,
        /// Motivo de la inconsistencia.
        reason: &'static str,
    },
    /// JSON inválido.
    InvalidJson(String),
    /// Error de lectura de fichero.
    Io(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotWinIsd => f.write_str("no es un fichero WinISD .wdr (falta [Driver])"),
            Self::NotOpenIsd => f.write_str("no es un fichero OpenISD .owdr (falta specs.woofer)"),
            Self::MissingField(name) => write!(f, "falta el parámetro {name}"),
            Self::InvalidNumber { key, value } => {
                write!(f, "valor numérico inválido para {key}: {value:?}")
            }
            Self::InvalidParameter(key) => {
                write!(f, "parámetro inválido en el fichero: {key}")
            }
            Self::Inconsistent { key, reason } => {
                write!(f, "parámetro inconsistente {key}: {reason}")
            }
            Self::InvalidJson(err) => write!(f, "JSON inválido: {err}"),
            Self::Io(err) => write!(f, "no se pudo leer el fichero: {err}"),
        }
    }
}

impl std::error::Error for ImportError {}

impl ImportedDriver {
    #[allow(clippy::too_many_arguments)]
    fn new(
        driver: Driver,
        format: DriverFormat,
        brand: String,
        model: String,
        comment: String,
        voice_coils: u32,
        vc_connection: Option<u32>,
        nominal_ohms: Option<f64>,
        xmax_mm: Option<f64>,
        pe_w: Option<f64>,
        warnings: Vec<String>,
    ) -> Self {
        Self {
            driver,
            format,
            brand,
            model,
            comment,
            voice_coils,
            vc_connection,
            nominal_ohms,
            xmax_mm,
            pe_w,
            warnings,
        }
    }
}

#[derive(Default)]
struct TsInput {
    fs: Option<f64>,
    qms: Option<f64>,
    qes: Option<f64>,
    qts: Option<f64>,
    vas_m3: Option<f64>,
    re: Option<f64>,
    sd_m2: Option<f64>,
    mms_kg: Option<f64>,
    cms_m_per_n: Option<f64>,
    bl: Option<f64>,
    rms: Option<f64>,
    le_h: Option<f64>,
}

fn positive(value: Option<f64>) -> Option<f64> {
    value.filter(|v| v.is_finite() && *v > 0.0)
}

impl TsInput {
    /// Rechaza cualquier parámetro presente que no sea finito y positivo.
    fn validate(&self) -> Result<(), ImportError> {
        for (key, value) in [
            ("Fs", self.fs),
            ("Qms", self.qms),
            ("Qes", self.qes),
            ("Qts", self.qts),
            ("Vas", self.vas_m3),
            ("Re", self.re),
            ("Sd", self.sd_m2),
            ("Mms", self.mms_kg),
            ("Cms", self.cms_m_per_n),
            ("BL", self.bl),
            ("Rms", self.rms),
        ] {
            if let Some(v) = value {
                if !v.is_finite() || v <= 0.0 {
                    return Err(ImportError::InvalidParameter(key));
                }
            }
        }
        if let Some(le) = self.le_h {
            if !le.is_finite() || le < 0.0 {
                return Err(ImportError::InvalidParameter("Le"));
            }
        }
        Ok(())
    }
}

/// Convierte T/S a [`Driver`], derivando lo que falte.
fn driver_from_ts(ts: TsInput, air: &Air) -> Result<(Driver, Vec<String>), ImportError> {
    ts.validate()?;
    let warnings = Vec::new();

    let sd = positive(ts.sd_m2).ok_or(ImportError::MissingField("Sd"))?;
    let re = positive(ts.re).ok_or(ImportError::MissingField("Re"))?;

    let cms = match positive(ts.cms_m_per_n) {
        Some(cms) => cms,
        None => {
            let vas = positive(ts.vas_m3).ok_or(ImportError::MissingField("Cms o Vas"))?;
            vas / (air.rho * air.c * air.c * sd * sd)
        }
    };

    let fs = positive(ts.fs);
    let mms = match positive(ts.mms_kg) {
        Some(mms) => mms,
        None => {
            let f = fs.ok_or(ImportError::MissingField("Mms o Fs"))?;
            1.0 / ((std::f64::consts::TAU * f).powi(2) * cms)
        }
    };

    let air_mass = (8.0 / 3.0) * air.rho * (sd / std::f64::consts::PI).sqrt().powi(3);
    if mms <= air_mass {
        return Err(ImportError::Inconsistent {
            key: "Mms",
            reason: "no supera la masa añadida del aire",
        });
    }
    let mmd = mms - air_mass;

    let rms = match positive(ts.rms) {
        Some(rms) => rms,
        None => {
            let f = fs.ok_or(ImportError::MissingField("Rms o Fs"))?;
            let qms = positive(ts.qms).ok_or(ImportError::MissingField("Rms o Qms"))?;
            std::f64::consts::TAU * f * mms / qms
        }
    };

    let bl = match positive(ts.bl) {
        Some(bl) => bl,
        None => {
            let f = fs.ok_or(ImportError::MissingField("BL o Fs"))?;
            let qes = positive(ts.qes).ok_or(ImportError::MissingField("BL o Qes"))?;
            (re * std::f64::consts::TAU * f * mms / qes).sqrt()
        }
    };

    let le = ts.le_h.unwrap_or(0.0);
    let driver = Driver::new(sd, cms, mmd, re, bl, rms, le);
    if let Err(err) = driver.validate() {
        let field = match err {
            SolveError::InvalidParameter(field) => field,
            _ => "driver",
        };
        return Err(ImportError::InvalidParameter(field));
    }
    Ok((driver, warnings))
}

fn complete_q(ts: &mut TsInput) {
    match (ts.qms, ts.qes, ts.qts) {
        (Some(qms), None, Some(qts)) if qms > qts => {
            ts.qes = Some(qts * qms / (qms - qts));
        }
        (None, Some(qes), Some(qts)) if qes > qts => {
            ts.qms = Some(qts * qes / (qes - qts));
        }
        _ => {}
    }
}

/// Aplica un valor de aire opcional: ausente usa el valor por defecto;
/// presente debe ser finito y positivo.
fn air_parameter(value: Option<f64>, default: f64, key: &'static str) -> Result<f64, ImportError> {
    match value {
        None => Ok(default),
        Some(v) if v.is_finite() && v > 0.0 => Ok(v),
        Some(_) => Err(ImportError::InvalidParameter(key)),
    }
}

/// Como [`air_parameter`], pero distingue «campo ausente» de «campo presente
/// con valor no interpretable» en el JSON de OpenISD.
fn owdr_air_parameter(
    woofer: &serde_json::Map<String, Value>,
    key: &str,
    default: f64,
    field: &'static str,
) -> Result<f64, ImportError> {
    match woofer.get(key) {
        None => Ok(default),
        Some(_) => match owdr_number(woofer, key) {
            Some(v) if v.is_finite() && v > 0.0 => Ok(v),
            _ => Err(ImportError::InvalidParameter(field)),
        },
    }
}

/// Convierte un campo entero validando rango y truncamiento.
fn integer_field(
    value: Option<f64>,
    key: &'static str,
    minimum: f64,
) -> Result<Option<u32>, ImportError> {
    match value {
        None => Ok(None),
        Some(v) => {
            if !v.is_finite()
                || v < minimum
                || v > u32::MAX as f64
                || (v - v.round()).abs() > 1.0e-9
            {
                return Err(ImportError::InvalidParameter(key));
            }
            Ok(Some(v.round() as u32))
        }
    }
}

/// Como [`integer_field`], pero distingue «campo ausente» de «campo presente
/// con valor no interpretable» en el JSON de OpenISD.
fn owdr_integer_field(
    woofer: &serde_json::Map<String, Value>,
    key: &str,
    field: &'static str,
    minimum: f64,
) -> Result<Option<u32>, ImportError> {
    match woofer.get(key) {
        None => Ok(None),
        Some(_) => {
            let value = owdr_number(woofer, key).ok_or(ImportError::InvalidParameter(field))?;
            integer_field(Some(value), field, minimum)
        }
    }
}

/// Detecta el formato y lo importa.
pub fn import_driver(text: &str) -> Result<ImportedDriver, ImportError> {
    let trimmed = text.trim_start_matches('\u{feff}').trim_start();
    if trimmed.starts_with('{') {
        import_owdr(text)
    } else {
        import_wdr(text)
    }
}

/// Importa un fichero desde disco.
///
/// Solo se compila en nativo. En `wasm32-unknown-unknown` use
/// [`import_driver`] con el texto que haya leído la aplicación; el núcleo
/// acústico no exige acceso al filesystem.
#[cfg(not(target_arch = "wasm32"))]
pub fn import_driver_file(
    path: impl AsRef<std::path::Path>,
) -> Result<ImportedDriver, ImportError> {
    let text = std::fs::read_to_string(path).map_err(|e| ImportError::Io(e.to_string()))?;
    import_driver(&text)
}

// ---------------------------------------------------------------------------
// WinISD .wdr
// ---------------------------------------------------------------------------

fn parse_wdr_map(text: &str) -> Result<HashMap<String, String>, ImportError> {
    let text = text.trim_start_matches('\u{feff}');
    let mut section = String::new();
    let mut map = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_ascii_lowercase();
            continue;
        }
        if section != "driver" {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    if map.is_empty() {
        return Err(ImportError::NotWinIsd);
    }
    Ok(map)
}

fn wdr_number(map: &HashMap<String, String>, key: &str) -> Result<Option<f64>, ImportError> {
    let Some(value) = map.get(key) else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    value
        .replace(',', ".")
        .parse::<f64>()
        .map(Some)
        .map_err(|_| ImportError::InvalidNumber {
            key: key.to_string(),
            value: value.to_string(),
        })
}

fn wdr_string(map: &HashMap<String, String>, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| map.get(*key).cloned())
        .unwrap_or_default()
}

/// Importa un driver WinISD `.wdr`.
pub fn import_wdr(text: &str) -> Result<ImportedDriver, ImportError> {
    let map = parse_wdr_map(text)?;

    let mut ts = TsInput {
        fs: wdr_number(&map, "fs")?,
        qms: wdr_number(&map, "qms")?,
        qes: wdr_number(&map, "qes")?,
        qts: wdr_number(&map, "qts")?,
        vas_m3: wdr_number(&map, "vas")?,
        re: wdr_number(&map, "re")?,
        sd_m2: wdr_number(&map, "sd")?,
        mms_kg: wdr_number(&map, "mms")?,
        cms_m_per_n: wdr_number(&map, "cms")?,
        bl: wdr_number(&map, "bl")?,
        rms: wdr_number(&map, "rms")?,
        le_h: wdr_number(&map, "le")?,
    };
    complete_q(&mut ts);

    let c = air_parameter(wdr_number(&map, "c")?, 344.0, "c")?;
    let rho = air_parameter(wdr_number(&map, "roo")?, 1.205, "rho")?;
    let air = Air::new(c, rho);

    let (mut driver, warnings) = driver_from_ts(ts, &air)?;

    let voice_coils = integer_field(wdr_number(&map, "numvc")?, "numVC", 1.0)?.unwrap_or(1);
    driver.nd = voice_coils;

    Ok(ImportedDriver::new(
        driver,
        DriverFormat::WinIsd,
        wdr_string(&map, &["brand", "manufacturer"]),
        wdr_string(&map, &["model"]),
        wdr_string(&map, &["comment"]),
        voice_coils,
        integer_field(wdr_number(&map, "vccon")?, "VCCon", 0.0)?,
        positive(wdr_number(&map, "znom")?),
        positive(wdr_number(&map, "xmax")?).map(|v| v * 1000.0),
        positive(wdr_number(&map, "pe")?),
        warnings,
    ))
}

// ---------------------------------------------------------------------------
// OpenISD .owdr
// ---------------------------------------------------------------------------

fn json_number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str()?.replace(',', ".").parse().ok())
}

fn owdr_number(obj: &serde_json::Map<String, Value>, key: &str) -> Option<f64> {
    let field = obj.get(key)?;
    if let Some(value) = field.get("value").and_then(json_number) {
        return Some(value);
    }
    if let Some(readings) = field.get("readings").and_then(Value::as_object) {
        for reading in readings.values() {
            if let Some(value) = reading.get("read_value").and_then(json_number) {
                return Some(value);
            }
        }
    }
    None
}

fn owdr_first(obj: &serde_json::Map<String, Value>, keys: &[(&str, f64)]) -> Option<f64> {
    keys.iter()
        .find_map(|(key, scale)| owdr_number(obj, key).map(|value| value * scale))
}

fn owdr_string(root: &Value, key: &str) -> Option<String> {
    let field = root.get(key)?;
    if let Some(value) = field.get("value").and_then(Value::as_str) {
        return Some(value.to_string());
    }
    if let Some(readings) = field.get("readings").and_then(Value::as_object) {
        for reading in readings.values() {
            if let Some(value) = reading.get("read_value").and_then(Value::as_str) {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Importa un driver OpenISD `.owdr`.
pub fn import_owdr(text: &str) -> Result<ImportedDriver, ImportError> {
    let root: Value =
        serde_json::from_str(text).map_err(|e| ImportError::InvalidJson(e.to_string()))?;
    let specs = root
        .get("specs")
        .and_then(Value::as_object)
        .ok_or(ImportError::NotOpenIsd)?;
    let woofer = specs
        .get("woofer")
        .and_then(Value::as_object)
        .or_else(|| specs.values().find_map(Value::as_object))
        .ok_or(ImportError::NotOpenIsd)?;

    let mut ts = TsInput {
        fs: owdr_first(woofer, &[("Fs_hz", 1.0), ("Fs", 1.0)]),
        qms: owdr_number(woofer, "Qms"),
        qes: owdr_number(woofer, "Qes"),
        qts: owdr_number(woofer, "Qts"),
        vas_m3: owdr_first(woofer, &[("Vas_m3", 1.0), ("Vas_l", 1.0e-3)]),
        re: owdr_first(woofer, &[("Re_ohm", 1.0), ("Re", 1.0)]),
        sd_m2: owdr_first(woofer, &[("Sd_m2", 1.0), ("Sd_cm2", 1.0e-4)]),
        mms_kg: owdr_first(woofer, &[("Mms_kg", 1.0), ("Mms_g", 1.0e-3)]),
        cms_m_per_n: owdr_first(woofer, &[("Cms_m_per_N", 1.0), ("Cms_mm_per_N", 1.0e-3)]),
        bl: owdr_first(woofer, &[("BL_Tm", 1.0), ("BL", 1.0)]),
        rms: owdr_first(woofer, &[("Rms_kg_per_s", 1.0), ("Rms", 1.0)]),
        le_h: owdr_first(woofer, &[("Le_H", 1.0), ("Le_mH", 1.0e-3)]),
    };
    complete_q(&mut ts);

    let c = owdr_air_parameter(woofer, "c_m_per_s", 344.0, "c")?;
    let rho = owdr_air_parameter(woofer, "roo_kg_per_m3", 1.205, "rho")?;
    let air = Air::new(c, rho);

    let (mut driver, warnings) = driver_from_ts(ts, &air)?;

    let voice_coils = owdr_integer_field(woofer, "numVC", "numVC", 1.0)?.unwrap_or(1);
    driver.nd = voice_coils;

    let brand = owdr_string(&root, "brand")
        .or_else(|| owdr_string(&root, "manufacturer"))
        .unwrap_or_default();
    let model = owdr_string(&root, "model").unwrap_or_default();
    let comment = owdr_string(&root, "comment").unwrap_or_default();

    Ok(ImportedDriver::new(
        driver,
        DriverFormat::OpenIsd,
        brand,
        model,
        comment,
        voice_coils,
        owdr_integer_field(woofer, "VCCon", "VCCon", 0.0)?,
        owdr_first(woofer, &[("Znom_ohm", 1.0), ("Znom", 1.0)]),
        owdr_first(woofer, &[("Xmax_m", 1.0), ("Xmax_mm", 1.0e-3)]).map(|v| v * 1000.0),
        owdr_number(woofer, "Pe_W"),
        warnings,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WDR: &str = "\u{feff}[Driver]\r\n\
Brand=Test\r\nModel=T1\r\nComment=hello\r\n\
Fs=50\r\nQms=2\r\nQes=0.5\r\nVas=0.05\r\nRe=6\r\nSd=0.02\r\n\
Mms=0.03\r\nCms=0.0004\r\nBL=10\r\nRms=2\r\nLe=0.001\r\n\
Xmax=0.005\r\nPe=100\r\nZnom=8\r\nnumVC=1\r\nVCCon=1\r\n";

    #[test]
    fn imports_wdr_and_preserves_si_values() {
        let imported = import_driver(WDR).unwrap();
        assert_eq!(imported.format, DriverFormat::WinIsd);
        assert_eq!(imported.brand, "Test");
        assert_eq!(imported.model, "T1");
        assert_eq!(imported.voice_coils, 1);
        assert!((imported.driver.sd - 0.02).abs() < 1.0e-15);
        assert!((imported.driver.cms - 4.0e-4).abs() < 1.0e-15);
        assert!((imported.driver.re - 6.0).abs() < 1.0e-15);
        assert!((imported.driver.bl - 10.0).abs() < 1.0e-15);
        assert!((imported.driver.rms - 2.0).abs() < 1.0e-15);
        assert!((imported.driver.le - 1.0e-3).abs() < 1.0e-15);
        let air = Air::default();
        let total_mms = imported.driver.mmd + imported.driver.air_load_mass(&air);
        assert!((total_mms - 0.03).abs() < 1.0e-6);
    }

    #[test]
    fn wdr_accepts_comma_decimals_and_derives_missing() {
        let text = "[Driver]\r\nFs=50\r\nQms=2\r\nQes=0,5\r\nVas=0,05\r\n\
                    Re=6\r\nSd=0,02\r\nMms=0,03\r\nCms=0,0004\r\n\
                    BL=10\r\nRms=2\r\n";
        let imported = import_wdr(text).unwrap();
        assert!((imported.driver.cms - 4.0e-4).abs() < 1.0e-15);
        assert!((imported.driver.rms - 2.0).abs() < 1.0e-15);
        assert!((imported.driver.bl - 10.0).abs() < 1.0e-15);
    }

    #[test]
    fn wdr_derives_from_ts() {
        let text = "[Driver]\r\nFs=50\r\nQms=2\r\nQes=0.5\r\n\
                    Vas=0.05\r\nRe=6\r\nSd=0.02\r\n";
        let imported = import_wdr(text).unwrap();
        assert!(imported.driver.cms > 0.0);
        assert!(imported.driver.mmd > 0.0);
        assert!(imported.driver.rms > 0.0);
        assert!(imported.driver.bl > 0.0);
    }

    const OWDR: &str = r#"{
      "brand": {"value": "OpenBrand"},
      "model": {"value": "O1"},
      "comment": {"value": "open"},
      "specs": {
        "woofer": {
          "Fs_hz": {"value": 50.0},
          "Qms": {"value": 2.0},
          "Qes": {"value": 0.5},
          "Vas_m3": {"value": 0.05},
          "Re_ohm": {"value": 6.0},
          "Sd_m2": {"value": 0.02},
          "Mms_kg": {"value": 0.03},
          "Cms_m_per_N": {"value": 0.0004},
          "BL_Tm": {"value": 10.0},
          "Rms_kg_per_s": {"value": 2.0},
          "Le_H": {"value": 0.001},
          "Xmax_m": {"value": 0.005},
          "Pe_W": {"value": 100.0},
          "Znom_ohm": {"value": 8.0},
          "numVC": {"value": 1},
          "VCCon": {"value": 1}
        }
      }
    }"#;

    #[test]
    fn imports_owdr_value_layout() {
        let imported = import_driver(OWDR).unwrap();
        assert_eq!(imported.format, DriverFormat::OpenIsd);
        assert_eq!(imported.brand, "OpenBrand");
        assert_eq!(imported.model, "O1");
        assert!((imported.driver.sd - 0.02).abs() < 1.0e-15);
        assert!((imported.driver.cms - 4.0e-4).abs() < 1.0e-15);
        assert!((imported.driver.bl - 10.0).abs() < 1.0e-15);
        assert_eq!(imported.voice_coils, 1);
    }

    #[test]
    fn imports_owdr_readings_layout_and_derives() {
        let text = r#"{
          "specs": {
            "woofer": {
              "Fs_hz": {"readings": {"ds": {"read_value": 50.0}}},
              "Qms": {"readings": {"ds": {"read_value": 2.0}}},
              "Qes": {"readings": {"ds": {"read_value": 0.5}}},
              "Vas_m3": {"readings": {"ds": {"read_value": 0.05}}},
              "Re_ohm": {"readings": {"ds": {"read_value": 6.0}}},
              "Sd_m2": {"readings": {"ds": {"read_value": 0.02}}}
            }
          }
        }"#;
        let imported = import_owdr(text).unwrap();
        assert!(imported.driver.cms > 0.0);
        assert!(imported.driver.mmd > 0.0);
        assert!(imported.driver.rms > 0.0);
        assert!(imported.driver.bl > 0.0);
    }

    #[test]
    fn rejects_unrelated_text() {
        assert_eq!(import_driver("hello").unwrap_err(), ImportError::NotWinIsd);
        assert!(matches!(import_driver("{}"), Err(ImportError::NotOpenIsd)));
    }

    #[test]
    fn rejects_mms_not_above_air_mass() {
        let text = WDR.replace("Mms=0.03", "Mms=0.001");
        assert!(matches!(
            import_wdr(&text),
            Err(ImportError::Inconsistent { key: "Mms", .. })
        ));
    }

    #[test]
    fn rejects_invalid_air_properties() {
        let text = WDR.replace("Fs=50", "Fs=50\r\nc=0");
        assert!(matches!(
            import_wdr(&text),
            Err(ImportError::InvalidParameter("c"))
        ));

        let text = WDR.replace("Fs=50", "Fs=50\r\nroo=-1");
        assert!(matches!(
            import_wdr(&text),
            Err(ImportError::InvalidParameter("rho"))
        ));
    }

    #[test]
    fn rejects_invalid_numvc_and_vccon() {
        for bad in ["1.5", "0", "1e30", "NaN"] {
            let text = WDR.replace("numVC=1", &format!("numVC={bad}"));
            assert!(
                matches!(
                    import_wdr(&text),
                    Err(ImportError::InvalidParameter("numVC"))
                ),
                "numVC={bad}"
            );
        }

        let text = WDR.replace("VCCon=1", "VCCon=1.5");
        assert!(matches!(
            import_wdr(&text),
            Err(ImportError::InvalidParameter("VCCon"))
        ));
    }

    #[test]
    fn rejects_negative_le() {
        let text = WDR.replace("Le=0.001", "Le=-0.001");
        assert!(matches!(
            import_wdr(&text),
            Err(ImportError::InvalidParameter("Le"))
        ));
    }

    #[test]
    fn owdr_rejects_invalid_air_numvc_and_mms() {
        let text = OWDR.replace(
            "\"Fs_hz\": {\"value\": 50.0},",
            "\"Fs_hz\": {\"value\": 50.0},\n          \"c_m_per_s\": {\"value\": 0.0},",
        );
        assert!(matches!(
            import_owdr(&text),
            Err(ImportError::InvalidParameter("c"))
        ));

        let text = OWDR.replace("\"numVC\": {\"value\": 1}", "\"numVC\": {\"value\": 1.5}");
        assert!(matches!(
            import_owdr(&text),
            Err(ImportError::InvalidParameter("numVC"))
        ));

        let text = OWDR.replace(
            "\"numVC\": {\"value\": 1}",
            "\"numVC\": {\"value\": \"dos\"}",
        );
        assert!(matches!(
            import_owdr(&text),
            Err(ImportError::InvalidParameter("numVC"))
        ));

        let text = OWDR.replace("\"VCCon\": {\"value\": 1}", "\"VCCon\": {\"value\": 1.5}");
        assert!(matches!(
            import_owdr(&text),
            Err(ImportError::InvalidParameter("VCCon"))
        ));

        let text = OWDR.replace(
            "\"Mms_kg\": {\"value\": 0.03}",
            "\"Mms_kg\": {\"value\": 0.001}",
        );
        assert!(matches!(
            import_owdr(&text),
            Err(ImportError::Inconsistent { key: "Mms", .. })
        ));
    }

    #[test]
    fn rejects_non_finite_derived_driver() {
        let text = "[Driver]\r\nFs=1e-200\r\nQms=2\r\nQes=0.5\r\nVas=0.05\r\n\
                    Re=6\r\nSd=0.02\r\n";
        assert!(matches!(
            import_wdr(text),
            Err(ImportError::InvalidParameter("driver.mmd"))
        ));
    }
}
