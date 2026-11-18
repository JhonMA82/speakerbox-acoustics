//! Geometría axisimétrica persistente, reutilizable en CAD y en Webster.
use std::{f64::consts::PI, sync::Arc};

use serde_json::{json, Value};

use crate::{error::require_positive, Flare, Horn, Segment, SolveError};

/// Ley del radio de un perfil circular.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HornProfileKind {
    Cylindrical,
    Conical,
    Exponential,
    Hypex {
        t: f64,
    },
    /// Tractrix de tangente constante ajustada a los tres datos de diseño.
    Tractrix,
    /// Tractrix completa con la coordenada axial escalada.
    ScaledTractrix,
    /// Contorno de guía oblate spheroidal. El solver sigue siendo Webster.
    OblateSpheroidal,
    /// Exponencial unido de forma tangente a un cono.
    ExponentialConical {
        /// Posición de unión dividida por la longitud total, estrictamente entre 0 y 1.
        join_fraction: f64,
    },
}

/// Parámetros explícitos en SI. Use `resolve` para preparar un perfil reutilizable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HornProfileSpec {
    pub kind: HornProfileKind,
    pub length_m: f64,
    /// Radio circular de garganta (m), finito y positivo.
    pub throat_radius_m: f64,
    /// Radio circular de boca (m), finito y positivo.
    pub mouth_radius_m: f64,
}

/// Sección circular para exportación geométrica.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HornProfileSample {
    /// Distancia desde la garganta (m).
    pub axial_m: f64,
    pub radius_m: f64,
    pub area_m2: f64,
}

/// Geometría validada con parámetros resueltos una sola vez.
#[derive(Clone, Debug)]
pub struct HornProfile {
    spec: HornProfileSpec,
    horn: Horn,
    volume_m3: f64,
}

impl HornProfileSpec {
    /// Longitud de la tractrix completa que termina con pared perpendicular al eje.
    pub fn natural_tractrix_length(throat_m: f64, mouth_m: f64) -> Result<f64, SolveError> {
        validate_ends(throat_m, mouth_m)?;
        let span = Tractrix::new(mouth_m, throat_m, mouth_m).span_m();
        require_positive(span, "tractrix natural length")?;
        Ok(span)
    }

    /// Valida extremos y resuelve parámetros internos en los perfiles implícitos.
    pub fn resolve(self) -> Result<HornProfile, SolveError> {
        let Self {
            kind,
            length_m: length,
            throat_radius_m: rt,
            mouth_radius_m: rm,
        } = self;
        require_positive(length, "horn profile length")?;
        let s1 = area(rt)?;
        let s2 = area(rm)?;
        let mut volume = None;
        let flare = match kind {
            HornProfileKind::Cylindrical => {
                volume = Some(length * s1);
                Flare::Cylindrical
            }
            HornProfileKind::Conical => {
                volume = Some(length * (s1 / 3.0 + rt * rm * (PI / 3.0) + s2 / 3.0));
                Flare::Conical
            }
            HornProfileKind::Exponential => {
                volume = Some(length * exponential_mean_area(s1, s2));
                Flare::Exponential
            }
            HornProfileKind::Hypex { t } => Flare::HyperbolicExponential { t },
            HornProfileKind::OblateSpheroidal => {
                if rm < rt {
                    return Err(SolveError::InvalidParameter("OS needs mouth >= throat"));
                }
                volume = Some(length * (s1 + (s2 - s1) / 3.0));
                Flare::Custom(Arc::new(move |x| s1 + (s2 - s1) * (x / length).powi(2)))
            }
            HornProfileKind::Tractrix | HornProfileKind::ScaledTractrix => {
                validate_ends(rt, rm)?;
                let full = Tractrix::new(rm, rt, rm);
                let natural = full.span_m();
                require_positive(natural, "tractrix natural length")?;
                let curve = if kind == HornProfileKind::Tractrix {
                    let relative = (length - natural) / natural;
                    if relative < -8.0 * f64::EPSILON {
                        return Err(SolveError::InvalidParameter(
                            "tractrix shorter than natural length",
                        ));
                    }
                    if relative <= 8.0 * f64::EPSILON {
                        full
                    } else {
                        Tractrix::fit(rt, rm, length)?
                    }
                } else {
                    full
                };
                volume = Some(curve.volume(length));
                Flare::Custom(Arc::new(move |x| {
                    if x == 0.0 {
                        return s1;
                    }
                    if x == length {
                        return s2;
                    }
                    let radius = curve.radius(x / length);
                    PI * radius * radius
                }))
            }
            HornProfileKind::ExponentialConical { join_fraction } => {
                if !join_fraction.is_finite()
                    || join_fraction <= 0.0
                    || join_fraction >= 1.0
                    || rm < rt
                {
                    return Err(SolveError::InvalidParameter(
                        "exponential-conical join or radii",
                    ));
                }
                let join = length * join_fraction;
                require_positive(join, "exponential length")?;
                require_positive(length - join, "conical length")?;
                let log_ratio = if rm - rt < rt {
                    ((rm - rt) / rt).ln_1p()
                } else {
                    rm.ln() - rt.ln()
                };
                let upper =
                    (log_ratio / join_fraction).min(log_ratio.exp_m1() / (1.0 - join_fraction));
                require_positive(1.0 + upper, "exponential-conical flare rate")?;
                let rate_l = bisect(0.0, upper, |u| {
                    u * join_fraction + (u * (1.0 - join_fraction)).ln_1p() < log_ratio
                });
                let log_rise = rate_l * join_fraction;
                let sj = s1 * (2.0 * log_rise).exp();
                let sj = if sj.is_finite() {
                    sj
                } else {
                    (s1.ln() + 2.0 * log_rise).exp()
                };
                let segments = vec![
                    Segment::exponential(join, s1, sj),
                    Segment::conical(length - join, sj, s2),
                ];
                let exp_mean = exponential_mean_area(s1, sj);
                let cone_mean = sj / 3.0 + sj.sqrt() * s2.sqrt() / 3.0 + s2 / 3.0;
                return self.finish(
                    segments,
                    Some(join * exp_mean + (length - join) * cone_mean),
                );
            }
        };
        self.finish(vec![Segment::new(flare, length, s1, s2)], volume)
    }

    fn finish(
        self,
        segments: Vec<Segment>,
        volume: Option<f64>,
    ) -> Result<HornProfile, SolveError> {
        let horn = Horn::new(segments);
        horn.validate()?;
        let volume_m3 = volume.unwrap_or_else(|| horn.volume());
        require_positive(volume_m3, "horn profile volume")?;
        Ok(HornProfile {
            spec: self,
            horn,
            volume_m3,
        })
    }

    /// Serializa la especificación validada al esquema JSON v1.
    pub fn to_json(self) -> Result<String, SolveError> {
        self.resolve()?;
        let mut value = json!({
            "version": 1,
            "length_m": self.length_m,
            "throat_radius_m": self.throat_radius_m,
            "mouth_radius_m": self.mouth_radius_m,
        });
        let name = match self.kind {
            HornProfileKind::Cylindrical => "cylindrical",
            HornProfileKind::Conical => "conical",
            HornProfileKind::Exponential => "exponential",
            HornProfileKind::Hypex { t } => {
                value["t"] = json!(t);
                "hypex"
            }
            HornProfileKind::Tractrix => "tractrix",
            HornProfileKind::ScaledTractrix => "scaled_tractrix",
            HornProfileKind::OblateSpheroidal => "oblate_spheroidal",
            HornProfileKind::ExponentialConical { join_fraction } => {
                value["join_fraction"] = json!(join_fraction);
                "exponential_conical"
            }
        };
        value["kind"] = json!(name);
        Ok(value.to_string())
    }

    /// Lee el esquema v1 y rechaza versiones, campos o geometrías inválidas.
    pub fn from_json(input: &str) -> Result<Self, SolveError> {
        const ERROR: SolveError = SolveError::InvalidParameter("horn profile JSON v1");
        let v: Value = serde_json::from_str(input).map_err(|_| ERROR)?;
        let fields = v.as_object().ok_or(ERROR)?;
        if v["version"].as_u64() != Some(1) {
            return Err(ERROR);
        }
        let number = |key| fields.get(key).and_then(Value::as_f64).ok_or(ERROR);
        let kind = match v["kind"].as_str() {
            Some("cylindrical") => HornProfileKind::Cylindrical,
            Some("conical") => HornProfileKind::Conical,
            Some("exponential") => HornProfileKind::Exponential,
            Some("hypex") => HornProfileKind::Hypex { t: number("t")? },
            Some("tractrix") => HornProfileKind::Tractrix,
            Some("scaled_tractrix") => HornProfileKind::ScaledTractrix,
            Some("oblate_spheroidal") => HornProfileKind::OblateSpheroidal,
            Some("exponential_conical") => HornProfileKind::ExponentialConical {
                join_fraction: number("join_fraction")?,
            },
            _ => return Err(ERROR),
        };
        let extra = matches!(
            kind,
            HornProfileKind::Hypex { .. } | HornProfileKind::ExponentialConical { .. }
        );
        if fields.len() != 5 + usize::from(extra) {
            return Err(ERROR);
        }
        let spec = Self {
            kind,
            length_m: number("length_m")?,
            throat_radius_m: number("throat_radius_m")?,
            mouth_radius_m: number("mouth_radius_m")?,
        };
        spec.resolve()?;
        Ok(spec)
    }
}

impl HornProfile {
    /// Recupera los parámetros nominales para guardar o editar el diseño.
    pub fn spec(&self) -> HornProfileSpec {
        self.spec
    }
    pub fn length_m(&self) -> f64 {
        self.spec.length_m
    }
    pub fn throat_area_m2(&self) -> f64 {
        self.horn.throat_area()
    }
    pub fn mouth_area_m2(&self) -> f64 {
        self.horn.mouth_area()
    }
    /// Volumen interior geométrico en m³, evaluado al resolver el perfil.
    pub fn volume_m3(&self) -> f64 {
        self.volume_m3
    }

    /// Radio y área en una posición de la geometría, con extremos incluidos.
    pub fn section_at(&self, axial_m: f64) -> Result<HornProfileSample, SolveError> {
        if !axial_m.is_finite() || axial_m < 0.0 || axial_m > self.length_m() {
            return Err(SolveError::InvalidParameter("horn profile axial position"));
        }
        let area_m2 = if axial_m == 0.0 {
            self.throat_area_m2()
        } else if axial_m == self.length_m() {
            self.mouth_area_m2()
        } else {
            let first = &self.horn.segments[0];
            match self.horn.segments.get(1) {
                Some(second) if axial_m > first.length() => second.area(axial_m - first.length()),
                _ => first.area(axial_m),
            }
        };
        Ok(sample(axial_m, area_m2))
    }

    /// Muestreo uniforme con ambos extremos. Use `section_at` para una malla adaptativa.
    pub fn sample(&self, count: usize) -> Result<Vec<HornProfileSample>, SolveError> {
        if count < 2 {
            return Err(SolveError::InvalidParameter("horn profile sample count"));
        }
        let mut points = Vec::new();
        points
            .try_reserve_exact(count)
            .map_err(|_| SolveError::InvalidParameter("horn profile sample count"))?;
        for i in 0..count {
            points.push(self.section_at(self.length_m() * (i as f64 / (count - 1) as f64))?);
        }
        Ok(points)
    }

    /// Devuelve los segmentos compatibles con los solvers existentes.
    pub fn into_horn(self) -> Horn {
        self.horn
    }
}

fn sample(axial_m: f64, area_m2: f64) -> HornProfileSample {
    HornProfileSample {
        axial_m,
        area_m2,
        radius_m: area_m2.sqrt() / PI.sqrt(),
    }
}

fn exponential_mean_area(a: f64, b: f64) -> f64 {
    let lo = a.min(b);
    let hi = a.max(b);
    let ratio = (hi - lo) / lo;
    let log = if ratio.is_finite() {
        ratio.ln_1p()
    } else {
        hi.ln() - lo.ln()
    };
    if log == 0.0 {
        lo
    } else {
        hi * (-(-log).exp_m1() / log)
    }
}

fn area(radius: f64) -> Result<f64, SolveError> {
    require_positive(radius, "horn profile radius")?;
    let result = PI * radius * radius;
    require_positive(result, "horn profile endpoint area")?;
    Ok(result)
}

fn validate_ends(rt: f64, rm: f64) -> Result<(), SolveError> {
    area(rt)?;
    area(rm)?;
    if rt >= rm {
        return Err(SolveError::InvalidParameter(
            "tractrix needs mouth > throat",
        ));
    }
    Ok(())
}

fn bisect(mut lo: f64, mut hi: f64, below: impl Fn(f64) -> bool) -> f64 {
    for _ in 0..56 {
        let mid = lo + (hi - lo) * 0.5;
        if below(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo + (hi - lo) * 0.5
}

// r = a sech(t), z from the a-radius plane = a(t - tanh(t)).
#[derive(Clone, Copy)]
struct Tractrix {
    a: f64,
    rt: f64,
    rm: f64,
    tt: f64,
    tm: f64,
}

impl Tractrix {
    fn new(a: f64, rt: f64, rm: f64) -> Self {
        let parameter = |r: f64| {
            let s: f64 = (((a - r) / a) * (1.0 + r / a)).sqrt();
            if s < 0.5 {
                s.atanh()
            } else {
                a.ln() - f64::ln(r) + s.ln_1p()
            }
        };
        Self {
            a,
            rt,
            rm,
            tt: parameter(rt),
            tm: parameter(rm),
        }
    }

    fn g(t: f64) -> f64 {
        if t < 0.01 {
            let x = t * t;
            t * x * (1.0 / 3.0 + x * (-2.0 / 15.0 + x * (17.0 / 315.0 - x * 62.0 / 2835.0)))
        } else {
            t - t.tanh()
        }
    }

    fn span_m(self) -> f64 {
        self.a * (Self::g(self.tt) - Self::g(self.tm))
    }

    fn fit(rt: f64, rm: f64, length: f64) -> Result<Self, SolveError> {
        let mut hi = rm * 2.0;
        while hi.is_finite() && Self::new(hi, rt, rm).span_m() < length {
            hi *= 2.0;
        }
        if !hi.is_finite() {
            return Err(SolveError::InvalidParameter("tractrix tangent length"));
        }
        let a = bisect(rm, hi, |a| Self::new(a, rt, rm).span_m() < length);
        let fit = Self::new(a, rt, rm);
        if ((fit.span_m() - length) / length).abs() > 1e-8 || !fit.span_m().is_finite() {
            return Err(SolveError::InvalidParameter(
                "tractrix length cannot be resolved",
            ));
        }
        Ok(fit)
    }

    fn radius(self, fraction: f64) -> f64 {
        let target = Self::g(self.tm) + (1.0 - fraction) * (Self::g(self.tt) - Self::g(self.tm));
        let t = self.inverse_position(target);
        let exponential = (-t).exp();
        (self.a * exponential) * (2.0 / (1.0 + exponential * exponential))
    }

    fn inverse_position(self, target: f64) -> f64 {
        if target <= 0.0 {
            return self.tm;
        }
        // g(t) = t - tanh(t) es creciente y convexa. Newton avanza
        // hacia la raíz desde una cota superior. Cerca de la boca,
        // g(t) ~ t³/3 da una semilla que evita muchas iteraciones.
        let upper = (target + 1.0).min(self.tt);
        let cubic = (3.0 * target).cbrt();
        let seed = (cubic * (1.0 + cubic * cubic)).clamp(self.tm, upper);
        let mut t = if Self::g(seed) >= target { seed } else { upper };
        for _ in 0..16 {
            let residual = Self::g(t) - target;
            let next = (t - residual / t.tanh().powi(2)).clamp(self.tm, upper);
            if (next - t).abs() <= 4.0 * f64::EPSILON * t.max(1.0) {
                return next;
            }
            t = next;
        }
        bisect(self.tm, upper, |t| Self::g(t) < target)
    }

    fn volume(self, length: f64) -> f64 {
        let st = self.tt.tanh();
        let sm = self.tm.tanh();
        // Integral of S dz in the tractrix parameter, scaled by L / L_nat.
        // Compute the difference of squares from radii to avoid subtracting
        // values of tanh close to one. Use physical radii to cancel a².
        let mean_r2 = (self.rm - self.rt)
            * ((self.rm + self.rt) / 3.0)
            * ((st * st + st * sm + sm * sm) / ((st + sm) * (Self::g(self.tt) - Self::g(self.tm))));
        PI * mean_r2 * length
    }
}
