//! Importación independiente FRD/ZMA. No modifica ni sustituye el modelo físico.
use num_complex::Complex64;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasurementKind {
    SplDb,
    PressurePa,
    ImpedanceOhm,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeasurementReference {
    pub provenance: Option<String>,
    pub distance_m: Option<f64>,
    pub voltage_rms: Option<f64>,
    pub time_reference_s: Option<f64>,
    pub note: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub frequency_hz: f64,
    pub magnitude: f64,
    pub phase_deg: Option<f64>,
}
impl Sample {
    /// None significa fase ausente; overflow y datos inválidos devuelven error.
    pub fn complex(&self, kind: MeasurementKind) -> Result<Option<Complex64>, DataError> {
        let error = || DataError {
            line: 0,
            reason: "complex measurement outside representable domain",
        };
        if !self.magnitude.is_finite()
            || kind != MeasurementKind::SplDb && self.magnitude < 0.
            || self.phase_deg.is_some_and(|p| !p.is_finite())
        {
            return Err(error());
        }
        let Some(phase) = self.phase_deg else {
            return Ok(None);
        };
        let amplitude = if kind == MeasurementKind::SplDb {
            20e-6 * 10_f64.powf(self.magnitude / 20.)
        } else {
            self.magnitude
        };
        if !amplitude.is_finite() || amplitude == 0. && self.magnitude != 0. {
            return Err(error());
        }
        let z = Complex64::from_polar(amplitude, phase.to_radians());
        if !z.re.is_finite() || !z.im.is_finite() {
            return Err(error());
        }
        Ok(Some(z))
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Measurement {
    pub kind: MeasurementKind,
    pub samples: Vec<Sample>,
    pub reference: MeasurementReference,
}
#[derive(Clone, Debug, PartialEq)]
pub struct DataError {
    pub line: usize,
    pub reason: &'static str,
}
impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "measurement line {}: {}", self.line, self.reason)
    }
}
impl std::error::Error for DataError {}
#[derive(Clone, Copy, Debug)]
pub struct ImportOptions {
    pub kind: MeasurementKind,
    pub frequency_scale_to_hz: f64,
    pub magnitude_scale: f64,
}
impl ImportOptions {
    pub fn frd() -> Self {
        Self {
            kind: MeasurementKind::SplDb,
            frequency_scale_to_hz: 1.,
            magnitude_scale: 1.,
        }
    }
    pub fn zma() -> Self {
        Self {
            kind: MeasurementKind::ImpedanceOhm,
            ..Self::frd()
        }
    }
}
impl Measurement {
    /// Decimal point; whitespace/comma/semicolon delimiters. Explicit scales avoid
    /// guessing units from an untrusted header. FRD dB reference is 20 µPa.
    pub fn parse(
        text: &str,
        options: ImportOptions,
        reference: MeasurementReference,
    ) -> Result<Self, DataError> {
        let err = |line, reason| DataError { line, reason };
        if !options.frequency_scale_to_hz.is_finite()
            || options.frequency_scale_to_hz <= 0.
            || !options.magnitude_scale.is_finite()
            || options.magnitude_scale <= 0.
        {
            return Err(err(0, "invalid unit scale"));
        }
        if options.kind == MeasurementKind::SplDb && options.magnitude_scale != 1. {
            return Err(err(0, "dB is logarithmic; use unit scale one"));
        }
        for x in [reference.distance_m, reference.voltage_rms] {
            if x.is_some_and(|v| !v.is_finite() || v <= 0.) {
                return Err(err(0, "invalid measurement reference"));
            }
        }
        if reference.time_reference_s.is_some_and(|v| !v.is_finite()) {
            return Err(err(0, "invalid time reference"));
        }
        let mut samples: Vec<Sample> = Vec::new();
        for (index, line) in text.trim_start_matches('\u{feff}').lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(['#', '*', '!']) || line.starts_with("//") {
                continue;
            }
            let clean = line
                .split('#')
                .next()
                .unwrap()
                .split("//")
                .next()
                .unwrap()
                .trim();
            let columns: Vec<_> = clean
                .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
                .filter(|v| !v.is_empty())
                .collect();
            if columns.is_empty() {
                continue;
            }
            let first = columns[0].parse::<f64>();
            if first.is_err() && samples.is_empty() {
                // Allow ordinary textual column headers, not corrupted numeric rows.
                if columns[0].chars().next().is_some_and(|c| c.is_alphabetic()) {
                    continue;
                }
            }
            if columns.len() != 2 && columns.len() != 3 {
                return Err(err(index + 1, "expected frequency magnitude [phase]"));
            }
            let number = |s: &str| {
                s.parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite())
                    .ok_or_else(|| err(index + 1, "invalid or nonfinite number"))
            };
            let f = number(columns[0])? * options.frequency_scale_to_hz;
            let magnitude = number(columns[1])? * options.magnitude_scale;
            if !f.is_finite()
                || f <= 0.
                || !magnitude.is_finite()
                || options.kind != MeasurementKind::SplDb && magnitude < 0.
            {
                return Err(err(index + 1, "frequency or magnitude outside domain"));
            }
            if samples.last().is_some_and(|s| f <= s.frequency_hz) {
                return Err(err(
                    index + 1,
                    "frequencies must strictly increase (no duplicates)",
                ));
            }
            samples.push(Sample {
                frequency_hz: f,
                magnitude,
                phase_deg: if columns.len() == 3 {
                    Some(number(columns[2])?)
                } else {
                    None
                },
            });
        }
        if samples.is_empty() {
            return Err(err(0, "no measurement samples"));
        }
        Ok(Self {
            kind: options.kind,
            samples,
            reference,
        })
    }
    /// Interpolación log-frecuencia. ZMA usa coordenadas cartesianas complejas;
    /// FRD usa dB/magnitud y fase desenrollada localmente, nunca fase envuelta lineal.
    /// Sin fase en cualquiera de los extremos, no se infiere fase intermedia.
    pub fn interpolate(&self, f: f64) -> Result<Sample, DataError> {
        let err = || DataError {
            line: 0,
            reason: "invalid samples or frequency outside measurement range",
        };
        if self.samples.is_empty()
            || !f.is_finite()
            || f < self.samples[0].frequency_hz
            || f > self.samples.last().unwrap().frequency_hz
        {
            return Err(err());
        }
        for (i, s) in self.samples.iter().enumerate() {
            if !s.frequency_hz.is_finite()
                || s.frequency_hz <= 0.
                || !s.magnitude.is_finite()
                || s.phase_deg.is_some_and(|p| !p.is_finite())
                || i > 0 && s.frequency_hz <= self.samples[i - 1].frequency_hz
            {
                return Err(err());
            }
        }
        let i = self.samples.partition_point(|s| s.frequency_hz < f);
        if i < self.samples.len() && self.samples[i].frequency_hz == f {
            return Ok(self.samples[i]);
        }
        let a = self.samples[i - 1];
        let b = self.samples[i];
        let t = (f.ln() - a.frequency_hz.ln()) / (b.frequency_hz.ln() - a.frequency_hz.ln());
        if self.kind == MeasurementKind::ImpedanceOhm {
            if let (Some(za), Some(zb)) = (a.complex(self.kind)?, b.complex(self.kind)?) {
                let z = za * (1. - t) + zb * t;
                if !z.re.is_finite() || !z.im.is_finite() {
                    return Err(err());
                }
                return Ok(Sample {
                    frequency_hz: f,
                    magnitude: z.norm(),
                    phase_deg: if z.norm() > 0. {
                        Some(z.arg().to_degrees())
                    } else {
                        None
                    },
                });
            }
        }
        let phase = match (a.phase_deg, b.phase_deg) {
            (Some(p), Some(q)) => {
                let delta = if p.abs() <= 180. && q.abs() <= 180. {
                    (q - p + 180.).rem_euclid(360.) - 180.
                } else {
                    q - p
                };
                Some(p + t * delta)
            }
            _ => None,
        };
        Ok(Sample {
            frequency_hz: f,
            magnitude: (1. - t) * a.magnitude + t * b.magnitude,
            phase_deg: phase,
        })
    }
}
