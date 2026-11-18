//! Redes acústicas lineales de presión y caudal en SI, con fasores RMS exp(+jωt).
use crate::energy::{active_power, EnergyReport, RadiationQuality, TerminationKind};
use crate::{Air, Chamber, Driver, Port, Segment, SolveError, TransferMatrix};
use num_complex::Complex64 as C;
use std::fmt;

mod linear;
pub mod schema;
pub mod topology;

const MAX_UNKNOWNS: usize = 256;

/// Índice de un nodo en la red.
pub type AcousticNode = usize;

#[derive(Clone, Debug)]
pub enum Conduit {
    Distributed(Segment),
    Inertance(Port),
}

#[derive(Clone, Debug)]
pub struct AcousticDuct {
    pub inlet: AcousticNode,
    pub outlet: AcousticNode,
    pub conduit: Conduit,
}

#[derive(Clone, Copy, Debug)]
pub enum Boundary {
    Chamber(Chamber),
    /// Frontera ideal con p=0, sin potencia radiada.
    PressureRelease,
    /// Pistón equivalente. Factor de carga 1 para pantalla infinita.
    Opening {
        area_m2: f64,
        load_scale: f64,
    },
    Resistance {
        pa_s_m3: f64,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct AcousticBoundary {
    pub node: AcousticNode,
    pub boundary: Boundary,
}

#[derive(Clone, Copy, Debug)]
pub struct AcousticSource {
    pub node: AcousticNode,
    /// Caudal positivo inyectado hacia la red.
    pub flow_m3_s: C,
}

#[derive(Clone, Debug, Default)]
pub struct AcousticNetwork {
    pub nodes: usize,
    pub ducts: Vec<AcousticDuct>,
    pub boundaries: Vec<AcousticBoundary>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NetworkError {
    Model(SolveError),
    UnknownNode(AcousticNode),
    Disconnected(AcousticNode),
    MissingReference(AcousticNode),
    Invalid(&'static str),
    Singular,
    IllConditioned(f64),
    Residual(f64),
    NonFinite,
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Model(e) => write!(f, "{e}"),
            Self::UnknownNode(n) => write!(f, "nodo inexistente {n}"),
            Self::Disconnected(n) => write!(f, "nodo {n} desconectado de la excitación"),
            Self::MissingReference(n) => {
                write!(f, "componente con nodo {n} sin referencia acústica")
            }
            Self::Invalid(s) => write!(f, "red inválida: {s}"),
            Self::Singular => write!(f, "red singular"),
            Self::IllConditioned(r) => write!(f, "red mal condicionada: razón de pivotes {r:e}"),
            Self::Residual(r) => write!(f, "residuo relativo excesivo: {r:e}"),
            Self::NonFinite => write!(f, "estado no finito"),
        }
    }
}
impl std::error::Error for NetworkError {}
impl From<SolveError> for NetworkError {
    fn from(e: SolveError) -> Self {
        Self::Model(e)
    }
}

#[derive(Clone, Debug)]
pub struct NetworkState {
    pub pressure_pa: Vec<C>,
    /// Ambos caudales tienen dirección de entrada a salida.
    pub duct_flow_m3_s: Vec<(C, C)>,
    pub boundary_flow_m3_s: Vec<C>,
    pub relative_residual: f64,
    /// Indicador del condicionamiento tras el equilibrado.
    pub pivot_ratio: f64,
}

impl NetworkState {
    fn check_finite(&self) -> Result<(), NetworkError> {
        if self
            .pressure_pa
            .iter()
            .chain(&self.boundary_flow_m3_s)
            .chain(
                self.duct_flow_m3_s
                    .iter()
                    .flat_map(|(input, output)| [input, output]),
            )
            .any(|&v| !finite(v))
        {
            return Err(NetworkError::NonFinite);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NetworkDriver {
    pub driver: Driver,
    pub front: AcousticNode,
    pub rear: AcousticNode,
    /// Tensión en circuito abierto del generador.
    pub voltage_rms: C,
    pub source_resistance_ohm: f64,
    pub solid_angle: f64,
    pub mechanical_damping: Option<MechanicalDamping>,
}

/// Pérdida de compliancia a frecuencias positivas. Ams en m/N, Rms en kg/s.
#[derive(Clone, Copy, Debug)]
pub struct MechanicalDamping {
    pub rms_kg_s: f64,
    pub compliance_loss_m_n: f64,
}

#[derive(Clone, Debug)]
pub struct DrivenNetworkState {
    pub frequency_hz: f64,
    pub source_voltage_rms: C,
    pub reference_pressure_pa: f64,
    pub acoustic: NetworkState,
    /// Diferencia de presiones por unidad de caudal del diafragma.
    pub acoustic_impedance: C,
    /// Impedancia compleja en los bornes, sin resistencia de generador.
    pub electrical_impedance_ohm: C,
    pub diaphragm_flow_m3_s: C,
    pub peak_displacement_m: f64,
    /// Campo estimado a 1 m, presión física. Suma coherente de fuentes compactas.
    pub pressure_estimate_pa: Option<C>,
    pub spl_db: Option<f64>,
    pub energy: EnergyReport,
}

impl Conduit {
    fn validated_matrix(&self, air: &Air, f: f64) -> Result<TransferMatrix, NetworkError> {
        Ok(match self {
            Self::Distributed(seg) => seg.try_matrix(air, f)?,
            Self::Inertance(port) => {
                port.validate()?;
                TransferMatrix {
                    b: -port.impedance(air, f),
                    ..TransferMatrix::identity()
                }
            }
        })
    }
}

impl Boundary {
    fn impedance(self, air: &Air, f: f64) -> Result<C, NetworkError> {
        let z = match self {
            Self::PressureRelease => return Ok(C::default()),
            Self::Chamber(chamber) => {
                chamber.validate()?;
                chamber.impedance(air, f)
            }
            Self::Opening {
                area_m2,
                load_scale,
            } => {
                crate::error::require_positive(area_m2, "network.opening.area")?;
                crate::error::require_positive(load_scale, "network.opening.scale")?;
                crate::piston_radiation_impedance(air, area_m2, f, std::f64::consts::TAU)
                    * load_scale
            }
            Self::Resistance { pa_s_m3 } => C::from(pa_s_m3),
        };
        if !finite(z) || z.norm() == 0.0 || z.re < 0.0 {
            return Err(NetworkError::Invalid(
                "impedancia de frontera no pasiva o indefinida",
            ));
        }
        Ok(z)
    }
}

type Resolved = (NetworkState, Vec<C>);

impl AcousticNetwork {
    /// Calcula p y U sin invertir una matriz ni dividir por B.
    pub fn solve(
        &self,
        air: &Air,
        f: f64,
        excitation: &[AcousticSource],
    ) -> Result<NetworkState, NetworkError> {
        self.solve_inner(air, f, excitation).map(|(state, _)| state)
    }

    fn size(&self) -> usize {
        self.nodes.saturating_add(self.ducts.len()).saturating_add(
            self.boundaries
                .iter()
                .filter(|x| matches!(x.boundary, Boundary::PressureRelease))
                .count(),
        )
    }

    fn check_connectivity(&self, excitation: &[AcousticSource]) -> Result<(), NetworkError> {
        let check = |n| {
            if n < self.nodes {
                Ok(())
            } else {
                Err(NetworkError::UnknownNode(n))
            }
        };
        if self.nodes == 0 || self.size() > MAX_UNKNOWNS {
            return Err(NetworkError::Invalid(
                "se requieren entre 1 y 256 incógnitas",
            ));
        }
        let mut graph = vec![Vec::new(); self.nodes];
        let mut reference = vec![false; self.nodes];
        for duct in &self.ducts {
            let (a, b) = (duct.inlet, duct.outlet);
            check(a)?;
            check(b)?;
            if a == b {
                return Err(NetworkError::Invalid("conducto entre el mismo nodo"));
            }
            graph[a].push(b);
            graph[b].push(a);
            if matches!(&duct.conduit,Conduit::Distributed(seg) if seg.length() > 0.0) {
                reference[a] = true;
            }
        }
        for shunt in &self.boundaries {
            check(shunt.node)?;
            reference[shunt.node] = true;
        }
        let mut excited = vec![false; self.nodes];
        for source in excitation {
            check(source.node)?;
            if !finite(source.flow_m3_s) {
                return Err(NetworkError::NonFinite);
            }
            excited[source.node] = true;
        }
        let mut seen = vec![false; self.nodes];
        for root in 0..self.nodes {
            if seen[root] {
                continue;
            }
            let mut next = vec![root];
            let (mut has_ref, mut has_source) = (false, false);
            seen[root] = true;
            while let Some(node) = next.pop() {
                has_ref |= reference[node];
                has_source |= excited[node];
                for &adjacent in &graph[node] {
                    if !seen[adjacent] {
                        seen[adjacent] = true;
                        next.push(adjacent);
                    }
                }
            }
            if !has_source {
                return Err(NetworkError::Disconnected(root));
            }
            if !has_ref {
                return Err(NetworkError::MissingReference(root));
            }
        }
        Ok(())
    }

    fn solve_inner(
        &self,
        air: &Air,
        f: f64,
        excitation: &[AcousticSource],
    ) -> Result<Resolved, NetworkError> {
        crate::error::check_frequency(f)?;
        air.validate()?;
        self.check_connectivity(excitation)?;
        let size = self.size();
        // Q = Zref U pone las incógnitas de caudal en la escala de presión.
        let zref = air.z0() / 0.01;
        if !zref.is_finite() || zref <= 0.0 {
            return Err(NetworkError::NonFinite);
        }
        let mut a = vec![vec![C::default(); size]; size];
        let mut rhs = vec![C::default(); size];
        let mut matrices = Vec::with_capacity(self.ducts.len());
        for (i, duct) in self.ducts.iter().enumerate() {
            let m = duct.conduit.validated_matrix(air, f)?;
            let (p, q, u) = (duct.inlet, duct.outlet, self.nodes + i);
            a[p][u] += 1.0;
            a[q][p] -= m.c * zref;
            a[q][u] -= m.d;
            a[u][p] += m.a;
            a[u][q] -= 1.0;
            a[u][u] += m.b / zref;
            matrices.push(m);
        }
        let mut loads = Vec::with_capacity(self.boundaries.len());
        let mut release_index = self.nodes + self.ducts.len();
        let mut releases = Vec::with_capacity(self.boundaries.len());
        for shunt in &self.boundaries {
            let z = shunt.boundary.impedance(air, f)?;
            let release = if matches!(shunt.boundary, Boundary::PressureRelease) {
                a[shunt.node][release_index] = C::from(1.0);
                a[release_index][shunt.node] = C::from(1.0);
                release_index += 1;
                Some(release_index - 1)
            } else {
                a[shunt.node][shunt.node] += zref / z;
                None
            };
            loads.push(z);
            releases.push(release);
        }
        for source in excitation {
            rhs[source.node] += source.flow_m3_s * zref;
        }
        if a.iter().flatten().any(|&v| !finite(v)) || rhs.iter().any(|&v| !finite(v)) {
            return Err(NetworkError::NonFinite);
        }
        let (x, relative_residual, pivot_ratio) = linear::solve(a, rhs)?;
        let pressure_pa = x[..self.nodes].to_vec();
        let duct_flow_m3_s = self
            .ducts
            .iter()
            .zip(matrices)
            .zip(&x[self.nodes..])
            .map(|((d, m), &u)| {
                let u = u / zref;
                (u, m.apply(pressure_pa[d.inlet], u).1)
            })
            .collect();
        let boundary_flow_m3_s = self
            .boundaries
            .iter()
            .zip(&loads)
            .zip(releases)
            .map(|((d, &z), release)| match release {
                Some(index) => x[index] / zref,
                None => pressure_pa[d.node] / z,
            })
            .collect();
        let state = NetworkState {
            pressure_pa,
            duct_flow_m3_s,
            boundary_flow_m3_s,
            relative_residual,
            pivot_ratio,
        };
        state.check_finite()?;
        Ok((state, loads))
    }

    /// Excita las dos caras del mismo diafragma con signos opuestos. Cada
    /// cámara cerrada y cada abertura se resuelven en la misma red.
    pub fn solve_driver(
        &self,
        air: &Air,
        f: f64,
        drive: &NetworkDriver,
    ) -> Result<DrivenNetworkState, NetworkError> {
        drive.driver.validate()?;
        crate::error::require_non_negative(
            drive.source_resistance_ohm,
            "network.source_resistance",
        )?;
        crate::error::require_positive(drive.solid_angle, "network.solid_angle")?;
        if !finite(drive.voltage_rms) {
            return Err(NetworkError::NonFinite);
        }
        if drive.front == drive.rear {
            return Err(NetworkError::Invalid(
                "caras del diafragma en el mismo nodo",
            ));
        }
        let (mut state, loads) = self.solve_inner(
            air,
            f,
            &[
                AcousticSource {
                    node: drive.front,
                    flow_m3_s: C::from(1.0),
                },
                AcousticSource {
                    node: drive.rear,
                    flow_m3_s: C::from(-1.0),
                },
            ],
        )?;
        let za = state.pressure_pa[drive.front] - state.pressure_pa[drive.rear];
        let d = drive.driver;
        let z_mech = drive.mechanical_impedance(f)?;
        let total = z_mech + d.sd * d.sd * za;
        let z_coil = d.coil_impedance(f);
        let z_series = z_coil + drive.source_resistance_ohm;
        let den = z_series * total + d.bl * d.bl;
        let current = drive.voltage_rms * total / den;
        let ud = drive.voltage_rms * (d.sd * d.bl) / den;
        let ze = z_coil + (d.bl * d.bl) / total;
        if [za, ze, current, ud].iter().any(|&v| !finite(v)) {
            return Err(NetworkError::NonFinite);
        }
        state.pressure_pa.iter_mut().for_each(|v| *v *= ud);
        state.boundary_flow_m3_s.iter_mut().for_each(|v| *v *= ud);
        for (input, output) in &mut state.duct_flow_m3_s {
            *input *= ud;
            *output *= ud;
        }
        state.check_finite()?;
        let mut energy = EnergyReport::new(
            &d,
            f,
            0.0,
            drive.source_resistance_ohm,
            current,
            ud,
            za.re * ud.norm_sqr(),
        );
        energy.source_active_w = active_power(drive.voltage_rms, current);
        energy.driver_terminal_voltage_v =
            drive.voltage_rms - drive.source_resistance_ohm * current;
        energy.driver_terminal_active_w = active_power(energy.driver_terminal_voltage_v, current);
        energy.suspension_loss_w = z_mech.re * (ud / d.sd).norm_sqr();
        for (duct, &(u, _)) in self.ducts.iter().zip(&state.duct_flow_m3_s) {
            if let Conduit::Distributed(seg) = &duct.conduit {
                energy.acoustic_losses.wall_w +=
                    seg.dissipated_power_w(air, f, state.pressure_pa[duct.inlet], u)?;
            }
        }
        let mut pressure = None;
        let mut approximate = false;
        for ((boundary, &z), &u) in self
            .boundaries
            .iter()
            .zip(&loads)
            .zip(&state.boundary_flow_m3_s)
        {
            let p = state.pressure_pa[boundary.node];
            match boundary.boundary {
                Boundary::PressureRelease => (),
                Boundary::Chamber(_) => energy.acoustic_losses.lining_w += z.re * u.norm_sqr(),
                Boundary::Resistance { .. } => {
                    energy.acoustic_losses.leakage_w += z.re * u.norm_sqr()
                }
                Boundary::Opening {
                    area_m2,
                    load_scale,
                } => {
                    approximate |= load_scale != 1.0;
                    let kind = if load_scale == 1.0 {
                        TerminationKind::BaffledPiston
                    } else {
                        TerminationKind::CalibratedPiston
                    };
                    energy
                        .terminations
                        .push(crate::energy::termination(kind, p, u, z));
                    *pressure.get_or_insert(C::default()) += C::i()
                        * crate::system::radiator_pressure(air, area_m2, u, f, drive.solid_angle);
                }
            }
        }
        let has_unmodeled_radiation = self
            .boundaries
            .iter()
            .any(|x| matches!(x.boundary, Boundary::PressureRelease));
        let quality = match (approximate, energy.terminations.len()) {
            _ if has_unmodeled_radiation => RadiationQuality::IncompleteRadiationModel,
            (_, 0) => RadiationQuality::IncompleteRadiationModel,
            (true, _) => RadiationQuality::CalibratedLoadApproximation,
            (false, 1) => RadiationQuality::SingleBaffledPiston,
            _ => RadiationQuality::IndependentLoadsApproximation,
        };
        let peak_displacement_m = std::f64::consts::SQRT_2 * (ud / d.sd).norm() / crate::omega(f);
        let spl_db = pressure.map(|p| 20.0 * (p.norm() / air.p_ref).log10());
        if !peak_displacement_m.is_finite()
            || spl_db.is_some_and(|spl| spl.is_nan() || spl == f64::INFINITY)
        {
            return Err(NetworkError::NonFinite);
        }
        Ok(DrivenNetworkState {
            frequency_hz: f,
            source_voltage_rms: drive.voltage_rms,
            reference_pressure_pa: air.p_ref,
            acoustic: state,
            acoustic_impedance: za,
            electrical_impedance_ohm: ze,
            diaphragm_flow_m3_s: ud,
            peak_displacement_m,
            pressure_estimate_pa: pressure,
            spl_db,
            energy: energy.finish(quality)?,
        })
    }
}

impl NetworkDriver {
    fn mechanical_impedance(&self, f: f64) -> Result<C, NetworkError> {
        let Some(p) = self.mechanical_damping else {
            return Ok(self.driver.mechanical_impedance(f));
        };
        crate::error::require_non_negative(p.rms_kg_s, "network.FDD.rms")?;
        crate::error::require_non_negative(p.compliance_loss_m_n, "network.FDD.compliance_loss")?;
        let w = crate::omega(f);
        Ok(C::new(p.rms_kg_s, w * self.driver.mmd)
            + 1.0 / (w * C::new(p.compliance_loss_m_n, self.driver.cms)))
    }
}

fn finite(z: C) -> bool {
    z.re.is_finite() && z.im.is_finite()
}
