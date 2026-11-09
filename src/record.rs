//! Lector del formato de registro de 734 bytes (`record.dat`).
//!
//! El registro tiene **124 campos y 734 bytes**. Los campos numéricos
//! iniciales son de 8 caracteres; el comentario ocupa 40 bytes a partir del
//! offset 642 y los tipos de segmento se guardan en los últimos 16 bytes
//! (p. ej. `ConExpL34L45111A`).
//!
//! Unidades del fichero: áreas en cm², longitudes en cm, volúmenes en litros,
//! masas en gramos, inductancia en mH y desplazamientos en mm.
//! [`LegacyRecord::to_system`] convierte todo a SI.

use crate::air::Air;
use crate::calibration::Calibration;
use crate::driver::Driver;
use crate::element::Chamber;
use crate::segment::{Flare, Horn, Segment};
use crate::system::System;

/// Longitud fija del registro de 734 bytes.
pub const RECORD_LEN: usize = 734;

/// Error de lectura de un registro.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordError {
    /// El bloque no mide exactamente 734 bytes.
    BadLength(usize),
    /// Un campo numérico no se pudo interpretar.
    BadField { index: usize, value: String },
}

impl std::fmt::Display for RecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecordError::BadLength(n) => {
                write!(
                    f,
                    "un registro de 734 bytes debe medir {RECORD_LEN} bytes, no {n}"
                )
            }
            RecordError::BadField { index, value } => {
                write!(f, "campo {index} inválido: {value:?}")
            }
        }
    }
}

impl std::error::Error for RecordError {}

/// Registro de diseño legado de 734 bytes.
#[derive(Clone, Debug)]
pub struct LegacyRecord {
    pub raw: Vec<u8>,
    /// Ángulo de radiación en radianes (campo `Ang`).
    pub angle_rad: f64,
    /// Tensión de entrada (V).
    pub eg: f64,
    /// Ángulo tangente de la boca de la bocina (`Fta`, grados).
    pub fta_deg: f64,
    /// Circunferencia de la boca en longitudes de onda de corte (`Cir`).
    pub cir: f64,
    /// Áreas S1..S5 (cm²).
    pub areas_cm2: [f64; 5],
    /// Longitudes L12..L45 (cm).
    pub lengths_cm: [f64; 4],
    /// Frecuencias de corte F12..F45 (Hz).
    pub cutoffs_hz: [f64; 4],
    /// Área efectiva del diafragma (cm²).
    pub sd_cm2: f64,
    /// Compliancia mecánica (m/N).
    pub cms: f64,
    /// Masa móvil (g).
    pub mmd_g: f64,
    /// Resistencia de la bobina (Ω).
    pub re: f64,
    /// Factor de fuerza (T·m).
    pub bl: f64,
    /// Resistencia mecánica (kg/s).
    pub rms: f64,
    /// Inductancia (mH).
    pub le_mh: f64,
    /// Número de altavoces.
    pub nd: u32,
    /// Volumen del recinto trasero (L).
    pub vrc_l: f64,
    /// Longitud media del recinto trasero (`Lrc`, cm).
    pub lrc: f64,
    /// Resistividad al flujo del revestimiento del recinto trasero
    /// (`Fr`, mks rayls/m).
    pub fr_hz: f64,
    /// Espesor del revestimiento del recinto trasero (`Tal`, cm).
    pub tal: f64,
    /// Volumen de la cámara de garganta (`Vtc`, cc).
    pub vtc_cc: f64,
    /// Área de la cámara de garganta (cm²).
    pub atc_cm2: f64,
    /// Potencia máxima (W).
    pub pmax_w: f64,
    /// Desplazamiento máximo (mm).
    pub xmax_mm: f64,
    /// Comentario del registro (40 caracteres).
    pub comment: String,
    /// Tipo de cada segmento según las banderas finales (`Con`, `Exp`, ...).
    pub segment_types: [String; 4],
}

impl LegacyRecord {
    /// Lee un registro de 734 bytes.
    pub fn parse(bytes: &[u8]) -> Result<Self, RecordError> {
        if bytes.len() != RECORD_LEN {
            return Err(RecordError::BadLength(bytes.len()));
        }
        let field = |i: usize| -> &[u8] { &bytes[i * 8..i * 8 + 8] };
        let num = |i: usize| -> Result<f64, RecordError> {
            let text = String::from_utf8_lossy(field(i));
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return Ok(0.0);
            }
            trimmed.parse::<f64>().map_err(|_| RecordError::BadField {
                index: i,
                value: trimmed.to_string(),
            })
        };

        let mut areas_cm2 = [0.0; 5];
        areas_cm2[0] = num(4)?;
        areas_cm2[1] = num(5)?;
        areas_cm2[2] = num(9)?;
        areas_cm2[3] = num(13)?;
        areas_cm2[4] = num(17)?;

        let mut lengths_cm = [0.0; 4];
        let mut cutoffs_hz = [0.0; 4];
        for i in 0..4 {
            lengths_cm[i] = num(6 + 4 * i)?;
            cutoffs_hz[i] = num(7 + 4 * i)?;
        }

        let mut segment_types: [String; 4] = Default::default();
        for (i, item) in segment_types.iter_mut().enumerate() {
            let start = 718 + 3 * i;
            *item = String::from_utf8_lossy(&bytes[start..start + 3])
                .trim()
                .to_string();
        }

        let comment = String::from_utf8_lossy(&bytes[642..682])
            .trim_matches(|c: char| c == '\0' || c.is_whitespace())
            .to_string();

        let angle_rad = parse_angle(&String::from_utf8_lossy(field(0)));

        Ok(Self {
            raw: bytes.to_vec(),
            angle_rad,
            eg: num(1)?,
            fta_deg: num(2)?,
            cir: num(3)?,
            areas_cm2,
            lengths_cm,
            cutoffs_hz,
            sd_cm2: num(20)?,
            cms: num(21)?,
            mmd_g: num(22)?,
            re: num(23)?,
            bl: num(24)?,
            rms: num(25)?,
            le_mh: num(26)?,
            nd: num(27)?.round().max(1.0) as u32,
            vrc_l: num(28)?,
            // El orden real del registro (documentado en `docs/RECORD_FORMAT.md`)
            // es: Vrc, Fr, Vtc, Lrc, Tal, Atc.
            fr_hz: num(29)?,
            vtc_cc: num(30)?,
            lrc: num(31)?,
            tal: num(32)?,
            atc_cm2: num(33)?,
            pmax_w: num(34)?,
            xmax_mm: num(35)?,
            comment,
            segment_types,
        })
    }

    /// Convierte el registro en un [`System`] en unidades SI, sin calibración
    /// empírica.
    pub fn to_system(&self) -> System {
        self.to_system_with(&Calibration::default())
    }

    /// Convierte el registro aplicando una calibración empírica.
    pub fn to_system_with(&self, cal: &Calibration) -> System {
        let driver = Driver::from_ui_units(
            self.sd_cm2,
            self.cms,
            self.mmd_g,
            self.re,
            self.bl,
            self.rms,
            self.le_mh,
            self.nd,
        );
        let mut sys = System::new(Air::default(), driver);
        sys.solid_angle = self.angle_rad;
        sys.eg = self.eg;

        let mut segments = Vec::new();
        for i in 0..4 {
            let length = self.lengths_cm[i] * 1.0e-2;
            let s1 = self.areas_cm2[i] * 1.0e-4;
            let s2 = self.areas_cm2[i + 1] * 1.0e-4;
            if length <= 0.0 || s1 <= 0.0 || s2 <= 0.0 {
                continue;
            }
            let flare = flare_from_code(&self.segment_types[i], self.cutoffs_hz[i]);
            segments.push(Segment::new(flare, length, s1, s2));
        }
        if !segments.is_empty() {
            sys.horn = Some(Horn::new(segments));
        }

        if self.vrc_l > 0.0 {
            sys.rear_chamber = Some(Chamber::new(self.vrc_l * 1.0e-3));
        }
        if self.vtc_cc > 0.0 {
            // El registro por defecto (Vtc = 4.00) solo produce resultados
            // coherentes con la app si se interpreta en litros: 4 cc bloquearía
            // el diafragma y la propia app muestra ~113 dB. Se usa 1e-3.
            sys.throat_chamber = Some(Chamber::new(
                self.vtc_cc * 1.0e-6 * cal.throat_volume_factor,
            ));
        }
        sys.spl_offset_db = cal.spl_offset_db;
        if let Some(horn) = sys.horn.as_mut() {
            horn.radiation_scale = cal.mouth_radiation_scale;
            horn.mouth_radiation_area_factor = cal.mouth_radiation_area_factor;
            horn.set_loss(cal.horn_loss);
        }
        sys.horn_leak = cal.horn_leak;
        sys.horn_series = cal.horn_series;
        sys
    }
}

fn parse_angle(text: &str) -> f64 {
    let t = text.trim();
    if let Some(pos) = t.to_ascii_lowercase().find('x') {
        let number = t[..pos].trim().trim_end_matches('*').trim();
        number
            .parse::<f64>()
            .map(|v| v * std::f64::consts::PI)
            .unwrap_or(std::f64::consts::TAU)
    } else {
        t.parse::<f64>().unwrap_or(std::f64::consts::TAU)
    }
}

fn flare_from_code(code: &str, cutoff_hz: f64) -> Flare {
    match code {
        "Con" => Flare::Conical,
        "Exp" => Flare::Exponential,
        "Par" => Flare::Parabolic,
        "Cir" => Flare::Cylindrical,
        "Hyp" => {
            // El parámetro T no está en los campos numéricos iniciales del
            // registro; 1/√2 es el perfil de Le Cléac'h habitual.
            let _ = cutoff_hz;
            Flare::HyperbolicExponential {
                t: std::f64::consts::FRAC_1_SQRT_2,
            }
        }
        _ => Flare::Exponential,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_default_record() -> Vec<u8> {
        let values = [
            "0.5 x Pi", "2.83", "55.30", "1.00", "80.00", "350.00", "60.00", "0.00", "350.00",
            "2400.00", "75.00", "70.27", "0.00", "0.00", "0.00", "0.00", "0.00", "0.00", "0.00",
            "0.00", "350.00", "4.00E-04", "20.00", "6.00", "18.00", "4.00", "1.00", "1", "14.00",
            "40000.00", "900.00", "16.00", "4.00", "350.00", "100", "5.0",
        ];
        let mut buf = vec![b' '; RECORD_LEN];
        for (i, v) in values.iter().enumerate() {
            let start = i * 8;
            let bytes = format!("{v:>8}").into_bytes();
            buf[start..start + 8].copy_from_slice(&bytes[..8]);
        }
        let title = b"Default horn loudspeaker parameters     ";
        buf[642..682].copy_from_slice(title);
        let flags = b"ConExpL34L45111A";
        buf[718..734].copy_from_slice(flags);
        buf
    }

    #[test]
    fn parses_default_record() {
        let buf = build_default_record();
        let rec = LegacyRecord::parse(&buf).unwrap();
        assert!((rec.angle_rad - 0.5 * std::f64::consts::PI).abs() < 1.0e-12);
        assert_eq!(rec.eg, 2.83);
        assert_eq!(rec.areas_cm2, [80.0, 350.0, 2400.0, 0.0, 0.0]);
        assert_eq!(rec.lengths_cm, [60.0, 75.0, 0.0, 0.0]);
        assert_eq!(rec.sd_cm2, 350.0);
        assert_eq!(rec.comment, "Default horn loudspeaker parameters");
        assert_eq!(rec.segment_types[0], "Con");
        assert_eq!(rec.segment_types[1], "Exp");
    }

    #[test]
    fn builds_two_segment_system() {
        let buf = build_default_record();
        let rec = LegacyRecord::parse(&buf).unwrap();
        let sys = rec.to_system();
        let horn = sys.horn.as_ref().expect("bocina de dos segmentos");
        assert_eq!(horn.segments.len(), 2);
        assert!((horn.length() - 1.35).abs() < 1.0e-12);
        assert!((horn.throat_area() - 80.0e-4).abs() < 1.0e-12);
        assert!((horn.mouth_area() - 2400.0e-4).abs() < 1.0e-12);
        assert!(sys.throat_chamber.is_some());
        assert!(sys.rear_chamber.is_some());
        // La frecuencia de corte del segmento exponencial coincide con F23
        // almacenada en el registro (70.27 Hz).
        let fc = horn.segments[1].cutoff_hz(&sys.air);
        assert!((fc - rec.cutoffs_hz[1]).abs() < 0.05, "fc={fc}");
    }

    #[test]
    fn rejects_bad_length() {
        assert_eq!(
            LegacyRecord::parse(&[0u8; 10]).unwrap_err(),
            RecordError::BadLength(10)
        );
    }
}
