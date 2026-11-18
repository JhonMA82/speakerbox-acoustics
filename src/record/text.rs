//! Lectura y exportación de registros de texto Hornresp mediante plantilla.
use crate::network::MechanicalDamping;
use crate::{Air, Chamber, CoilModel, Driver, EndCorrection, Port, System};
use std::fmt;

const SOURCE: &str = "RADIATION, SOURCE AND MOUTH PARAMETER VALUES";
const MOTOR: &str = "TRADITIONAL DRIVER PARAMETER VALUES";
const CABINET: &str = "CHAMBER PARAMETER VALUES";
const COIL: &str = "ADVANCED DRIVER PARAMETER VALUES FOR SEMI-INDUCTANCE MODEL";
const FDD: &str = "ADVANCED DRIVER PARAMETER VALUES FOR FREQUENCY-DEPENDENT DAMPING MODEL";
const FLAGS: &str = "STATUS FLAGS";
const HORN: &str = "HORN PARAMETER VALUES";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextRecordError {
    Missing(&'static str),
    Invalid(&'static str),
    Unsupported(&'static str),
}

impl fmt::Display for TextRecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(key) => write!(f, "campo ausente o repetido: {key}"),
            Self::Invalid(key) => write!(f, "campo inválido: {key}"),
            Self::Unsupported(why) => write!(f, "registro fuera del alcance de conversión: {why}"),
        }
    }
}
impl std::error::Error for TextRecordError {}

/// Documento textual original. Las secciones y campos desconocidos se conservan.
#[derive(Clone, Debug)]
pub struct HornrespTextRecord<'a> {
    text: &'a str,
}

impl<'a> HornrespTextRecord<'a> {
    pub fn parse(text: &'a str) -> Result<Self, TextRecordError> {
        let record = Self { text };
        record.number("", "ID")?;
        Ok(record)
    }

    /// Devuelve los bytes del texto importado, incluidos finales de línea.
    pub fn original_text(&self) -> &'a str {
        self.text
    }

    /// Un ID de sección vacío consulta el encabezado antes de la primera sección.
    pub fn value(&self, section: &str, key: &'static str) -> Result<&'a str, TextRecordError> {
        self.optional_value(section, key)?
            .ok_or(TextRecordError::Missing(key))
    }

    fn optional_value(
        &self,
        section: &str,
        key: &'static str,
    ) -> Result<Option<&'a str>, TextRecordError> {
        let mut current = "";
        let mut value = None;
        for line in self.text.lines() {
            let line = line.trim().trim_start_matches('\u{feff}');
            if let Some(next) = section_header(line) {
                current = next;
            } else if current == section {
                if let Some((k, v)) = line.split_once('=') {
                    if k.trim() == key && value.replace(v.trim()).is_some() {
                        return Err(TextRecordError::Missing(key));
                    }
                }
            }
        }
        Ok(value)
    }

    fn parse_number(value: &str, key: &'static str) -> Result<f64, TextRecordError> {
        value
            .replace(',', ".")
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or(TextRecordError::Invalid(key))
    }

    pub fn number(&self, section: &str, key: &'static str) -> Result<f64, TextRecordError> {
        Self::parse_number(self.value(section, key)?, key)
    }

    fn optional_zero(&self, section: &str, key: &'static str) -> Result<f64, TextRecordError> {
        self.optional_value(section, key)?
            .map_or(Ok(0.0), |v| Self::parse_number(v, key))
    }

    /// Devuelve los valores del circuito avanzado en SI cuando la bandera lo activa.
    pub fn advanced_coil(&self) -> Result<Option<CoilModel>, TextRecordError> {
        if self.optional_zero(FLAGS, "Semi-Inductance Model Flag")? == 0.0 {
            return Ok(None);
        }
        Ok(Some(CoilModel::ExtendedSemiInductance {
            re_prime_ohm: self.number(COIL, "Re'")?,
            leb_h: self.number(COIL, "Leb")? * 1e-3,
            le_h: self.number(COIL, "Le")? * 1e-3,
            rss_ohm: self.number(COIL, "Rss")?,
            ke_ohm_sqrt_s: self.number(COIL, "Ke")?,
        }))
    }

    pub fn mechanical_damping(&self) -> Result<Option<MechanicalDamping>, TextRecordError> {
        if self.optional_zero(FLAGS, "Damping Model Flag")? == 0.0 {
            return Ok(None);
        }
        Ok(Some(MechanicalDamping {
            rms_kg_s: self.number(FDD, "Rms")?,
            compliance_loss_m_n: self.number(FDD, "Ams")?,
        }))
    }

    fn check_simple(&self) -> Result<bool, TextRecordError> {
        const SCOPE: TextRecordError = TextRecordError::Unsupported(
            "se requiere una plantilla sellada o ventilada de driver tradicional",
        );
        if self.value(MOTOR, "Nd")? != "1"
            || !(0.0..=0.01).contains(&self.optional_zero(CABINET, "Vtc")?)
        {
            return Err(SCOPE);
        }
        for key in ["L12", "L23", "L34", "L45"] {
            if self.number(HORN, key)? != 0.0 {
                return Err(SCOPE);
            }
        }
        for (section, keys) in [
            (
                FLAGS,
                &[
                    "Lossy Inductance Model Flag",
                    "Semi-Inductance Model Flag",
                    "Damping Model Flag",
                    "Closed Mouth Flag",
                ][..],
            ),
            (CABINET, &["Fr", "Sp1"][..]),
            (
                "PASSIVE RADIATOR PARAMETER VALUE",
                &["Added Mass", "Sp1"][..],
            ),
            (
                "ACTIVE BAND PASS FILTER PARAMETER VALUES",
                &["High Pass Frequency", "Low Pass Frequency"][..],
            ),
            (
                "EQUALISER FILTER PARAMETER VALUES",
                &[
                    "Band 1 Frequency",
                    "Band 2 Frequency",
                    "Band 3 Frequency",
                    "Band 4 Frequency",
                    "Band 5 Frequency",
                    "Band 6 Frequency",
                ][..],
            ),
        ] {
            for &key in keys {
                if self.optional_zero(section, key)? != 0.0 {
                    return Err(SCOPE);
                }
            }
        }
        Ok(self.optional_value(CABINET, "Ap")?.is_some())
    }

    /// Traduce el subconjunto tradicional de caja sellada/ventilada a un System.
    pub fn simple_system(&self) -> Result<System, TextRecordError> {
        let has_port = self.check_simple()?;
        let mut system = System::new(
            Air::default(),
            Driver::from_ui_units(
                self.number(MOTOR, "Sd")?,
                self.number(MOTOR, "Cms")?,
                self.number(MOTOR, "Mmd")?,
                self.number(MOTOR, "Re")?,
                self.number(MOTOR, "Bl")?,
                self.number(MOTOR, "Rms")?,
                self.number(MOTOR, "Le")?,
                1,
            ),
        );
        system.eg = self.number(SOURCE, "Eg")?;
        system.rg = self.number(SOURCE, "Rg")?;
        let mut angle = self.value(SOURCE, "Ang")?.split('x');
        let multiples = angle
            .next()
            .and_then(|v| Self::parse_number(v.trim(), "Ang").ok())
            .ok_or(TextRecordError::Invalid("Ang"))?;
        if angle.next().map(str::trim) != Some("Pi") || angle.next().is_some() {
            return Err(TextRecordError::Invalid("Ang"));
        }
        system.solid_angle = multiples * std::f64::consts::PI;
        system.rear_chamber = Some(Chamber::new(self.number(CABINET, "Vrc")? * 1e-3));
        if has_port {
            let flag = self.number(FLAGS, "End Correction Flag")?;
            let correction = if flag == 0.0 {
                EndCorrection::None
            } else if flag == 1.0 {
                EndCorrection::Inner(1.5)
            } else {
                return Err(TextRecordError::Invalid("End Correction Flag"));
            };
            system.port = Some(Port::new(
                self.number(CABINET, "Ap")? * 1e-4,
                self.number(CABINET, "Lpt")? * 1e-2,
                correction,
            ));
        }
        system
            .validate()
            .map_err(|_| TextRecordError::Invalid("System"))?;
        Ok(system)
    }

    /// Actualiza campos representables en una plantilla de la misma topología.
    /// Los campos desconocidos y Lrc permanecen como en la plantilla.
    pub fn export_simple_system(&self, system: &System) -> Result<String, TextRecordError> {
        let vented = self.check_simple()?;
        system
            .validate()
            .map_err(|_| TextRecordError::Invalid("System"))?;
        let rear = system
            .rear_chamber
            .ok_or(TextRecordError::Unsupported("requiere cámara posterior"))?;
        if vented != system.port.is_some()
            || rear.lining.is_some()
            || system.horn.is_some()
            || system.passive_radiator.is_some()
            || system.rear_ql.is_some()
            || system.spl_offset_db != 0.0
            || system.driver.nd != 1
            || system.driver.coil_model != CoilModel::Inductance
            || system.air != Air::default()
        {
            return Err(TextRecordError::Unsupported(
                "diseño no representable en esta plantilla",
            ));
        }
        let mut updates = Vec::new();
        let mut update_number = |section, key, value: f64, si_per_unit: f64| {
            if self.number(section, key)? * si_per_unit != value {
                let display_value = value / si_per_unit;
                if !display_value.is_finite() {
                    return Err(TextRecordError::Invalid(key));
                }
                updates.push((section, key, display_value.to_string()));
            }
            Ok(())
        };
        let d = &system.driver;
        for (section, key, value, factor) in [
            (SOURCE, "Eg", system.eg, 1.0),
            (SOURCE, "Rg", system.rg, 1.0),
            (MOTOR, "Sd", d.sd, 1e-4),
            (MOTOR, "Cms", d.cms, 1.0),
            (MOTOR, "Mmd", d.mmd, 1e-3),
            (MOTOR, "Re", d.re, 1.0),
            (MOTOR, "Bl", d.bl, 1.0),
            (MOTOR, "Rms", d.rms, 1.0),
            (MOTOR, "Le", d.le, 1e-3),
            (CABINET, "Vrc", rear.volume, 1e-3),
        ] {
            update_number(section, key, value, factor)?;
        }
        if let Some(port) = system.port {
            let correction = if port.end_correction == EndCorrection::None {
                0.0
            } else if port.end_correction == EndCorrection::Inner(1.5) {
                1.0
            } else {
                return Err(TextRecordError::Unsupported(
                    "corrección de puerto no representable",
                ));
            };
            update_number(CABINET, "Ap", port.area, 1e-4)?;
            update_number(CABINET, "Lpt", port.length, 1e-2)?;
            update_number(FLAGS, "End Correction Flag", correction, 1.0)?;
        }
        let angle = system.solid_angle / std::f64::consts::PI;
        if ![0.5, 1.0, 2.0, 4.0].contains(&angle) {
            return Err(TextRecordError::Unsupported("ángulo sólido Hornresp"));
        }
        if self.simple_system()?.solid_angle != system.solid_angle {
            updates.push((SOURCE, "Ang", format!("{angle:.1} x Pi")));
        }
        let mut current = "";
        let mut out = String::with_capacity(self.text.len());
        for line in self.text.split_inclusive('\n') {
            let content = line.trim_end_matches(['\r', '\n']);
            if let Some(next) = section_header(content.trim()) {
                current = next;
            } else if let Some((key, _)) = content.split_once('=') {
                if let Some((_, _, replacement)) = updates
                    .iter()
                    .find(|(s, k, _)| *s == current && *k == key.trim())
                {
                    out.push_str(key);
                    out.push_str("= ");
                    out.push_str(replacement);
                    out.push_str(&line[content.len()..]);
                    continue;
                }
            }
            out.push_str(line);
        }
        // Validate representable values at the same boundary used on import.
        HornrespTextRecord::parse(&out)?.simple_system()?;
        Ok(out)
    }
}

fn section_header(line: &str) -> Option<&str> {
    line.strip_prefix('|')
        .map(|section| section.trim_end_matches(':').trim())
}
