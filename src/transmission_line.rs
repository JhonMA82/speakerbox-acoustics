//! Línea de Webster: extremo inicial cerrado, inyección lateral y extremo configurable.
//! Reutiliza Segment/TransferMatrix, el driver, radiación y balance RMS del núcleo.
use crate::energy::{load_termination, EnergyReport, RadiationQuality as Q, TerminationKind as K};
use crate::{Air, DesignPoint, Driver, Flare, Segment, SolveError, TransferMatrix};
use num_complex::Complex64 as C;
use std::{f64::consts::TAU, sync::Arc};
#[derive(Clone, Copy, Debug)]
pub enum LineTermination {
    BaffledPiston,
    Closed,
    PressureRelease,
    Resistance(f64),
}
/// Elemento lineal localizado entre segmentos. M en kg/m⁴ y R en Pa·s/m³.
/// La longitud geométrica del pliegue se incluye en los segmentos, no aquí.
#[derive(Clone, Copy, Debug)]
pub struct Fold {
    pub after_segment: usize,
    pub acoustic_mass: f64,
    pub resistance: f64,
}
#[derive(Clone, Debug)]
pub struct TransmissionLine {
    pub air: Air,
    pub driver: Driver,
    pub segments: Vec<Segment>,
    /// Índice del segmento que empieza en el nodo del driver; 0..=segments.len().
    pub tap: usize,
    pub termination: LineTermination,
    pub folds: Vec<Fold>,
    pub eg: f64,
    pub rg: f64,
    pub polarity: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct LinePoint {
    pub summary: DesignPoint,
    pub mouth_pressure_pa: C,
    pub mouth_volume_velocity_m3_s: C,
    pub driver_volume_velocity_m3_s: C,
    pub node_pressure_pa: C,
    pub upstream_volume_velocity_m3_s: C,
    pub downstream_volume_velocity_m3_s: C,
    pub impedance_ohm: C,
    pub pressure_estimate_pa: C,
}
struct State {
    point: LinePoint,
    matrices: Vec<(TransferMatrix, Option<usize>, Option<Fold>)>,
    tap: usize,
    closed_pressure: C,
    network: C,
    current: C,
}
impl TransmissionLine {
    pub fn new(driver: Driver, segments: Vec<Segment>, tap: usize) -> Self {
        Self {
            air: Air::default(),
            driver,
            segments,
            tap,
            termination: LineTermination::BaffledPiston,
            folds: Vec::new(),
            eg: 2.83,
            rg: 0.,
            polarity: 1.,
        }
    }
    /// Corta exactamente el perfil en x (m), conservando áreas y pérdidas.
    /// Defina las zonas de relleno mediante segmentos antes de este constructor.
    pub fn at_position(
        driver: Driver,
        segments: Vec<Segment>,
        position_m: f64,
    ) -> Result<Self, SolveError> {
        let length: f64 = segments.iter().map(Segment::length).sum();
        if !position_m.is_finite() || position_m < 0. || position_m > length {
            return Err(SolveError::InvalidParameter("TL driver position"));
        }
        let mut out = Vec::new();
        let mut x = 0.;
        let mut tap = 0;
        for seg in segments {
            let end = x + seg.length();
            if position_m > x && position_m < end {
                let cut = position_m - x;
                for (offset, l) in [(0., cut), (cut, seg.length() - cut)] {
                    let original = seg.clone();
                    let area1 = seg.area(offset);
                    let area2 = seg.area(offset + l);
                    let mut sub = if area1 == area2 && matches!(seg.flare(), Flare::Cylindrical) {
                        Segment::new(Flare::Cylindrical, l, area1, area2)
                    } else {
                        Segment::new(
                            Flare::Custom(Arc::new(move |z| original.area(offset + z))),
                            l,
                            area1,
                            area2,
                        )
                    }
                    .with_flow_resistivity(seg.flow_resistivity());
                    sub.loss = seg.loss;
                    out.push(sub);
                    if offset == 0. {
                        tap = out.len();
                    }
                }
            } else {
                out.push(seg);
                if end <= position_m {
                    tap = out.len();
                }
            }
            x = end;
        }
        let result = Self::new(driver, out, tap);
        result.validate()?;
        Ok(result)
    }
    pub fn validate(&self) -> Result<(), SolveError> {
        self.air.validate()?;
        self.driver.validate()?;
        crate::Horn::new(self.segments.clone()).validate()?;
        if self.segments.is_empty() || self.tap > self.segments.len() {
            return Err(SolveError::InvalidTopology("TL driver node"));
        }
        crate::error::require_non_negative(self.rg, "TL source resistance")?;
        crate::error::require_non_negative(self.eg, "TL RMS voltage")?;
        crate::error::require_polarity(self.polarity, "TL polarity")?;
        if let LineTermination::Resistance(r) = self.termination {
            crate::error::require_positive(r, "TL termination resistance")?;
        }
        for fold in &self.folds {
            if fold.after_segment >= self.segments.len() - 1 || fold.after_segment + 1 == self.tap {
                return Err(SolveError::InvalidTopology(
                    "fold must lie strictly inside a branch",
                ));
            }
            crate::error::require_non_negative(fold.acoustic_mass, "fold acoustic mass")?;
            crate::error::require_non_negative(fold.resistance, "fold resistance")?;
        }
        Ok(())
    }
    fn state(&self, f: f64) -> Result<State, SolveError> {
        crate::error::check_frequency(f)?;
        self.validate()?;
        crate::segment::check_cascade(&self.segments, &self.air, f)?;
        let mut matrices = Vec::new();
        let mut tap = 0;
        for (i, s) in self.segments.iter().enumerate() {
            if i == self.tap {
                tap = matrices.len();
            }
            matrices.push((s.try_matrix(&self.air, f)?, Some(i), None));
            for fold in self.folds.iter().filter(|b| b.after_segment == i) {
                let mut m = TransferMatrix::identity();
                m.b = -C::new(fold.resistance, TAU * f * fold.acoustic_mass);
                matrices.push((m, None, Some(*fold)));
            }
        }
        if self.tap == self.segments.len() {
            tap = matrices.len();
        }
        let mut left = TransferMatrix::identity();
        let mut right = left;
        for &(m, _, _) in &matrices[..tap] {
            left = left.then(m);
        }
        for &(m, _, _) in &matrices[tap..] {
            right = right.then(m);
        }
        let load = match self.termination {
            LineTermination::BaffledPiston => crate::piston_radiation_impedance(
                &self.air,
                self.segments.last().unwrap().s2(),
                f,
                TAU,
            ),
            LineTermination::Closed => C::new(f64::INFINITY, 0.),
            LineTermination::PressureRelease => C::new(0., 0.),
            LineTermination::Resistance(r) => C::new(r, 0.),
        };
        // Cross-multiply branch admittances: avoids dividing by zero at a
        // stub pressure node or a pressure-release terminal at the driver.
        let (g, h) = if matches!(self.termination, LineTermination::Closed) {
            (right.d, -right.c)
        } else {
            (right.d * load - right.b, right.a - right.c * load)
        };
        let den = left.a * h - left.c * g;
        let scale = (left.a * h).norm() + (left.c * g).norm();
        if den.norm() <= 16. * f64::EPSILON * scale || den == C::new(0., 0.) {
            return Err(SolveError::SingularSystem);
        }
        let network = left.a * g / den;
        let direct = crate::piston_radiation_impedance(&self.air, self.driver.sd, f, TAU);
        let ze = self.driver.input_impedance(f, network + direct) + self.rg;
        let current = C::new(self.eg, 0.) / ze;
        let ud = self
            .driver
            .volume_velocity(f, self.eg * self.polarity, self.rg, network + direct);
        let p = network * ud;
        let ul = -left.c * g / den * ud;
        let ur = left.a * h / den * ud;
        let (pm, um) = right.apply(p, ur);
        let reduced = self.air.rho
            * f
            * (if matches!(self.termination, LineTermination::Resistance(_)) {
                C::new(0., 0.)
            } else {
                um
            } - ud); // ρω/2π at 1 m; common propagation phase omitted.
        let pressure = C::new(0., 1.) * reduced;
        let source = self.eg * current.re;
        let power = if matches!(self.termination, LineTermination::Closed) {
            0.
        } else {
            load.re * um.norm_sqr()
        } + direct.re * ud.norm_sqr();
        let point = LinePoint {
            summary: DesignPoint {
                freq_hz: f,
                spl_db: if pressure.norm() > 0. {
                    20. * (pressure.norm() / self.air.p_ref).log10()
                } else {
                    f64::NEG_INFINITY
                },
                phase_deg: if reduced.norm() > 0. {
                    reduced.arg().to_degrees()
                } else {
                    f64::NAN
                },
                group_delay_ms: 0.,
                ze_mag: ze.norm(),
                ze_phase_deg: ze.arg().to_degrees(),
                xd_mm: 2_f64.sqrt() * 1000. * ud.norm() / (TAU * f * self.driver.sd),
                power_w: power,
                efficiency_pct: if source > 0. {
                    100. * power / source
                } else {
                    f64::NAN
                },
                za_real: network.re,
                za_imag: network.im,
                port_velocity: Some(um.norm() / self.segments.last().unwrap().s2()),
            },
            mouth_pressure_pa: pm,
            mouth_volume_velocity_m3_s: um,
            driver_volume_velocity_m3_s: ud,
            node_pressure_pa: p,
            upstream_volume_velocity_m3_s: ul,
            downstream_volume_velocity_m3_s: ur,
            impedance_ohm: ze,
            pressure_estimate_pa: pressure,
        };
        point.summary.validate()?;
        for z in [pm, um, ud, p, ul, ur, network, current] {
            if !z.re.is_finite() || !z.im.is_finite() {
                return Err(SolveError::SingularSystem);
            }
        }
        Ok(State {
            point,
            matrices,
            tap,
            closed_pressure: g / den * ud,
            network,
            current,
        })
    }
    pub fn try_response_at(&self, f: f64) -> Result<LinePoint, SolveError> {
        Ok(self.state(f)?.point)
    }
    pub fn try_energy_at(&self, f: f64) -> Result<EnergyReport, SolveError> {
        let s = self.state(f)?;
        let ud = s.point.driver_volume_velocity_m3_s;
        let direct = crate::piston_radiation_impedance(&self.air, self.driver.sd, f, TAU);
        let mut e = EnergyReport::new(
            &self.driver,
            f,
            self.eg,
            self.rg,
            s.current,
            ud,
            (s.network.re + direct.re) * ud.norm_sqr(),
        );
        e.terminations
            .push(load_termination(K::BaffledPiston, -ud, direct));
        if !matches!(self.termination, LineTermination::Closed) {
            let z = match self.termination {
                LineTermination::BaffledPiston => crate::piston_radiation_impedance(
                    &self.air,
                    self.segments.last().unwrap().s2(),
                    f,
                    TAU,
                ),
                LineTermination::Resistance(r) => C::new(r, 0.),
                _ => C::new(0., 0.),
            };
            e.terminations.push(load_termination(
                if matches!(self.termination, LineTermination::BaffledPiston) {
                    K::BaffledPiston
                } else {
                    K::PrescribedResistance
                },
                s.point.mouth_volume_velocity_m3_s,
                z,
            ));
        }
        for (range, mut p, mut u) in [
            (0..s.tap, s.closed_pressure, C::new(0., 0.)),
            (
                s.tap..s.matrices.len(),
                s.point.node_pressure_pa,
                s.point.downstream_volume_velocity_m3_s,
            ),
        ] {
            for &(m, index, fold) in &s.matrices[range] {
                if let Some(i) = index {
                    e.acoustic_losses.wall_w +=
                        self.segments[i].dissipated_power_w(&self.air, f, p, u)?;
                }
                if let Some(b) = fold {
                    e.acoustic_losses.throat_series_w += b.resistance * u.norm_sqr();
                }
                (p, u) = m.apply(p, u);
            }
        }
        // A resistive artificial terminal specifies absorption, not external radiation.
        e.finish(
            if matches!(
                self.termination,
                LineTermination::Resistance(_) | LineTermination::PressureRelease
            ) {
                Q::IncompleteRadiationModel
            } else if matches!(self.termination, LineTermination::Closed) {
                Q::SingleBaffledPiston
            } else {
                Q::IndependentLoadsApproximation
            },
        )
    }
}
