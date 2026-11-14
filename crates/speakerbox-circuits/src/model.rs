//! Modelo de la red: nodos, componentes pasivos y fuentes de tensión.
//!
//! Las unidades son SI: ohmios, henrios, faradios, voltios y hercios. El nodo
//! [`NodeId::GROUND`] existe siempre y su tensión es cero por definición; no es
//! una incógnita del sistema.
//!
//! El modelo es puramente eléctrico. Una carga puede ser una impedancia
//! constante o una [`ComplexCurve`] medida: en ese caso la carga solo es
//! válida dentro de la banda medida.

use crate::curves::ComplexCurve;
use crate::error::{CircuitError, CircuitResult};
use num_complex::Complex64;
use std::f64::consts::TAU;

/// Identificador de nodo. `0` es siempre tierra.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u32);

impl NodeId {
    /// Nodo de referencia. Su tensión es cero y no se resuelve.
    pub const GROUND: NodeId = NodeId(0);

    /// Índice del nodo como entero.
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }

    /// `true` para el nodo de referencia.
    #[must_use]
    pub fn is_ground(self) -> bool {
        self.0 == 0
    }
}

/// Impedancia de una carga.
#[derive(Clone, Debug, PartialEq)]
pub enum LoadImpedance {
    /// Impedancia constante, en ohmios.
    Constant(Complex64),
    /// Impedancia medida, válida solo dentro de su banda.
    Curve(ComplexCurve),
}

impl LoadImpedance {
    /// Impedancia a `freq_hz`.
    ///
    /// # Errores
    ///
    /// [`CircuitError::CurveOutOfRange`] fuera de la banda de una curva.
    pub fn at(&self, freq_hz: f64) -> CircuitResult<Complex64> {
        match self {
            LoadImpedance::Constant(z) => Ok(*z),
            LoadImpedance::Curve(curve) => curve.at(freq_hz),
        }
    }
}

/// Naturaleza eléctrica de un componente de dos terminales.
#[derive(Clone, Debug, PartialEq)]
pub enum ComponentKind {
    /// Resistencia ideal, en ohmios.
    Resistor {
        /// Valor en ohmios.
        ohm: f64,
    },
    /// Inductancia con resistencia serie opcional, en henrios y ohmios.
    Inductor {
        /// Inductancia en henrios.
        henry: f64,
        /// Resistencia serie del devanado en ohmios; `0.0` es una bobina ideal.
        dcr_ohm: f64,
    },
    /// Condensador ideal, en faradios.
    Capacitor {
        /// Capacidad en faradios.
        farad: f64,
    },
    /// Carga de dos terminales, normalmente la impedancia de un altavoz.
    Load(LoadImpedance),
}

impl ComponentKind {
    /// Impedancia a `freq_hz`, en ohmios.
    ///
    /// # Errores
    ///
    /// - [`CircuitError::UnusableComponent`] si el resultado es nulo o no
    ///   finito a esa frecuencia.
    /// - [`CircuitError::CurveOutOfRange`] si una carga medida queda fuera de
    ///   su banda.
    pub fn impedance(&self, index: usize, freq_hz: f64) -> CircuitResult<Complex64> {
        let z = match self {
            ComponentKind::Resistor { ohm } => Complex64::new(*ohm, 0.0),
            ComponentKind::Inductor { henry, dcr_ohm } => {
                Complex64::new(*dcr_ohm, TAU * freq_hz * *henry)
            }
            ComponentKind::Capacitor { farad } => {
                Complex64::new(0.0, -1.0 / (TAU * freq_hz * *farad))
            }
            ComponentKind::Load(load) => load.at(freq_hz)?,
        };
        if !z.re.is_finite() || !z.im.is_finite() || z.norm() <= 0.0 {
            return Err(CircuitError::UnusableComponent { index, freq_hz });
        }
        Ok(z)
    }

    /// Admitancia a `freq_hz`, en siemens.
    ///
    /// # Errores
    ///
    /// Las mismas que [`ComponentKind::impedance`], más una admitancia no
    /// finita por desbordamiento numérico.
    pub fn admittance(&self, index: usize, freq_hz: f64) -> CircuitResult<Complex64> {
        let z = self.impedance(index, freq_hz)?;
        // 1/z = conj(u)/(scale·|u|²) con u = z/scale: evita que |z|² desborde
        // para impedancias enormes y convierta una rama en un circuito abierto
        // silencioso.
        let scale = z.re.abs().max(z.im.abs());
        if scale == 0.0 || !scale.is_finite() {
            return Err(CircuitError::UnusableComponent { index, freq_hz });
        }
        let normalized = z / scale;
        let y = normalized.conj() / normalized.norm_sqr() / scale;
        if !y.re.is_finite() || !y.im.is_finite() {
            return Err(CircuitError::UnusableComponent { index, freq_hz });
        }
        Ok(y)
    }

    fn validate(&self, index: usize) -> CircuitResult<()> {
        let fail = |field| Err(CircuitError::InvalidComponent { index, field });
        match self {
            ComponentKind::Resistor { ohm } => {
                if !ohm.is_finite() || *ohm <= 0.0 {
                    return fail("la resistencia debe ser finita y > 0 Ω");
                }
            }
            ComponentKind::Inductor { henry, dcr_ohm } => {
                if !henry.is_finite() || *henry <= 0.0 {
                    return fail("la inductancia debe ser finita y > 0 H");
                }
                if !dcr_ohm.is_finite() || *dcr_ohm < 0.0 {
                    return fail("la resistencia serie debe ser finita y >= 0 Ω");
                }
            }
            ComponentKind::Capacitor { farad } => {
                if !farad.is_finite() || *farad <= 0.0 {
                    return fail("la capacidad debe ser finita y > 0 F");
                }
            }
            ComponentKind::Load(load) => match load {
                LoadImpedance::Constant(z) => {
                    if !z.re.is_finite() || !z.im.is_finite() || z.norm() <= 0.0 {
                        return fail("la impedancia de carga debe ser finita y no nula");
                    }
                    if z.re <= 0.0 {
                        return fail("la carga debe ser pasiva: Re(Z) > 0 Ω");
                    }
                }
                LoadImpedance::Curve(curve) => {
                    if curve.len() < 2 {
                        return fail("la curva de carga necesita al menos dos puntos");
                    }
                    if curve
                        .values()
                        .iter()
                        .any(|z| !z.re.is_finite() || !z.im.is_finite() || z.re <= 0.0)
                    {
                        return fail(
                            "la carga medida debe ser pasiva: Re(Z) > 0 Ω en todos los puntos",
                        );
                    }
                }
            },
        }
        Ok(())
    }
}

/// Componente de dos terminales entre dos nodos.
#[derive(Clone, Debug, PartialEq)]
pub struct Component {
    /// Nodo de un extremo.
    pub a: NodeId,
    /// Nodo del otro extremo.
    pub b: NodeId,
    /// Naturaleza eléctrica.
    pub kind: ComponentKind,
}

impl Component {
    /// Crea un componente entre dos nodos.
    #[must_use]
    pub fn new(a: NodeId, b: NodeId, kind: ComponentKind) -> Self {
        Self { a, b, kind }
    }

    /// Resistencia en ohmios.
    #[must_use]
    pub fn resistor(a: NodeId, b: NodeId, ohm: f64) -> Self {
        Self::new(a, b, ComponentKind::Resistor { ohm })
    }

    /// Inductancia con resistencia serie; use `0.0` para una bobina ideal.
    #[must_use]
    pub fn inductor(a: NodeId, b: NodeId, henry: f64, dcr_ohm: f64) -> Self {
        Self::new(a, b, ComponentKind::Inductor { henry, dcr_ohm })
    }

    /// Condensador en faradios.
    #[must_use]
    pub fn capacitor(a: NodeId, b: NodeId, farad: f64) -> Self {
        Self::new(a, b, ComponentKind::Capacitor { farad })
    }

    /// Carga de impedancia constante.
    #[must_use]
    pub fn load(a: NodeId, b: NodeId, impedance: Complex64) -> Self {
        Self::new(
            a,
            b,
            ComponentKind::Load(LoadImpedance::Constant(impedance)),
        )
    }

    /// Carga de impedancia medida, válida solo dentro de su banda.
    #[must_use]
    pub fn load_curve(a: NodeId, b: NodeId, curve: ComplexCurve) -> Self {
        Self::new(a, b, ComponentKind::Load(LoadImpedance::Curve(curve)))
    }
}

/// Fuente de tensión ideal entre dos nodos.
#[derive(Clone, Debug, PartialEq)]
pub struct VoltageSource {
    /// Nodo del terminal positivo.
    pub positive: NodeId,
    /// Nodo del terminal negativo.
    pub negative: NodeId,
    /// Tensión compleja en voltios.
    pub volts: Complex64,
}

impl VoltageSource {
    /// Crea una fuente ideal.
    #[must_use]
    pub fn new(positive: NodeId, negative: NodeId, volts: Complex64) -> Self {
        Self {
            positive,
            negative,
            volts,
        }
    }
}

/// Red eléctrica pasiva con fuentes de tensión ideales.
///
/// Los nodos se numeran `0..nodes`; `0` es tierra. Un circuito se construye
/// vacío y se rellena con [`Circuit::push_component`] y
/// [`Circuit::push_source`], que validan cada entrada.
///
/// ```
/// use speakerbox_circuits::{Circuit, Component, NodeId, VoltageSource};
/// use num_complex::Complex64;
///
/// let mut circuit = Circuit::new(2);
/// circuit
///     .push_source(VoltageSource::new(NodeId(1), NodeId::GROUND, Complex64::new(1.0, 0.0)))
///     .unwrap();
/// circuit
///     .push_component(Component::resistor(NodeId(1), NodeId::GROUND, 8.0))
///     .unwrap();
/// assert_eq!(circuit.input_impedance(1000.0).unwrap(), Complex64::new(8.0, 0.0));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Circuit {
    nodes: usize,
    components: Vec<Component>,
    sources: Vec<VoltageSource>,
}

impl Circuit {
    /// Circuito vacío con `nodes` nodos, incluida tierra.
    #[must_use]
    pub fn new(nodes: usize) -> Self {
        Self {
            nodes,
            components: Vec::new(),
            sources: Vec::new(),
        }
    }

    /// Número de nodos, tierra incluida.
    #[must_use]
    pub fn nodes(&self) -> usize {
        self.nodes
    }

    /// Componentes en orden de inserción.
    #[must_use]
    pub fn components(&self) -> &[Component] {
        &self.components
    }

    /// Fuentes en orden de inserción.
    #[must_use]
    pub fn sources(&self) -> &[VoltageSource] {
        &self.sources
    }

    /// Añade un componente validado.
    ///
    /// # Errores
    ///
    /// [`CircuitError::UnknownNode`], [`CircuitError::SelfLoop`] o
    /// [`CircuitError::InvalidComponent`].
    pub fn push_component(&mut self, component: Component) -> CircuitResult<()> {
        let index = self.components.len();
        self.check_nodes(component.a, component.b)?;
        component.kind.validate(index)?;
        self.components.push(component);
        Ok(())
    }

    /// Añade una fuente de tensión validada.
    ///
    /// # Errores
    ///
    /// [`CircuitError::UnknownNode`], [`CircuitError::SelfLoop`] o
    /// [`CircuitError::InvalidSource`].
    pub fn push_source(&mut self, source: VoltageSource) -> CircuitResult<()> {
        let index = self.sources.len();
        self.check_nodes(source.positive, source.negative)?;
        if !source.volts.re.is_finite() || !source.volts.im.is_finite() {
            return Err(CircuitError::InvalidSource {
                index,
                field: "la tensión debe ser finita",
            });
        }
        self.sources.push(source);
        Ok(())
    }

    /// Sustituye el valor de un componente existente, validándolo.
    ///
    /// # Errores
    ///
    /// [`CircuitError::UnknownComponent`] o [`CircuitError::InvalidComponent`].
    pub fn replace_component(&mut self, index: usize, kind: ComponentKind) -> CircuitResult<()> {
        if index >= self.components.len() {
            return Err(CircuitError::UnknownComponent { index });
        }
        kind.validate(index)?;
        self.components[index].kind = kind;
        Ok(())
    }

    /// Revalida el circuito completo.
    ///
    /// # Errores
    ///
    /// [`CircuitError::EmptyCircuit`] sin componentes, o cualquier error de
    /// nodo o de valor.
    pub fn validate(&self) -> CircuitResult<()> {
        if self.nodes == 0 {
            return Err(CircuitError::EmptyCircuit);
        }
        for (index, component) in self.components.iter().enumerate() {
            self.check_nodes(component.a, component.b)?;
            component.kind.validate(index)?;
        }
        for (index, source) in self.sources.iter().enumerate() {
            self.check_nodes(source.positive, source.negative)?;
            if !source.volts.re.is_finite() || !source.volts.im.is_finite() {
                return Err(CircuitError::InvalidSource {
                    index,
                    field: "la tensión debe ser finita",
                });
            }
        }
        if self.components.is_empty() {
            return Err(CircuitError::EmptyCircuit);
        }
        Ok(())
    }

    fn check_nodes(&self, a: NodeId, b: NodeId) -> CircuitResult<()> {
        for node in [a, b] {
            if node.index() >= self.nodes {
                return Err(CircuitError::UnknownNode {
                    node,
                    nodes: self.nodes,
                });
            }
        }
        if a == b {
            return Err(CircuitError::SelfLoop { node: a });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_nodes_and_self_loops() {
        let mut circuit = Circuit::new(2);
        assert_eq!(
            circuit
                .push_component(Component::resistor(NodeId(5), NodeId::GROUND, 1.0))
                .unwrap_err(),
            CircuitError::UnknownNode {
                node: NodeId(5),
                nodes: 2
            }
        );
        assert_eq!(
            circuit
                .push_component(Component::resistor(NodeId(1), NodeId(1), 1.0))
                .unwrap_err(),
            CircuitError::SelfLoop { node: NodeId(1) }
        );
    }

    #[test]
    fn rejects_physically_invalid_values() {
        let mut circuit = Circuit::new(2);
        for kind in [
            ComponentKind::Resistor { ohm: 0.0 },
            ComponentKind::Resistor { ohm: -1.0 },
            ComponentKind::Resistor { ohm: f64::NAN },
            ComponentKind::Inductor {
                henry: 0.0,
                dcr_ohm: 0.0,
            },
            ComponentKind::Inductor {
                henry: 1.0,
                dcr_ohm: -0.1,
            },
            ComponentKind::Capacitor {
                farad: f64::INFINITY,
            },
            ComponentKind::Load(LoadImpedance::Constant(Complex64::new(-1.0, 0.0))),
            ComponentKind::Load(LoadImpedance::Constant(Complex64::new(0.0, 0.0))),
        ] {
            let before = circuit.components().len();
            assert!(circuit
                .push_component(Component::new(NodeId(1), NodeId::GROUND, kind))
                .is_err());
            assert_eq!(circuit.components().len(), before);
        }
    }

    #[test]
    fn rejects_measured_loads_with_non_positive_real_part() {
        let curve = crate::curves::ComplexCurve::new(
            vec![100.0, 1000.0],
            vec![Complex64::new(-8.0, 1.0), Complex64::new(8.0, 1.0)],
        )
        .unwrap();
        let mut circuit = Circuit::new(2);
        assert!(circuit
            .push_component(Component::load_curve(NodeId(1), NodeId::GROUND, curve))
            .is_err());
        assert!(circuit.components().is_empty());
    }

    #[test]
    fn huge_impedance_stays_finite_instead_of_becoming_zero_admittance() {
        let mut circuit = Circuit::new(2);
        circuit
            .push_source(VoltageSource::new(
                NodeId(1),
                NodeId::GROUND,
                Complex64::new(1.0, 0.0),
            ))
            .unwrap();
        circuit
            .push_component(Component::load(
                NodeId(1),
                NodeId::GROUND,
                Complex64::new(1e200, 0.0),
            ))
            .unwrap();
        let z = circuit.input_impedance(1000.0).unwrap();
        assert!(z.re.is_finite() && z.re > 0.0);
        assert!((z.re - 1e200).abs() / 1e200 < 1e-12, "Zin={}", z.re);
    }

    #[test]
    fn accepts_ideal_inductor_and_reactive_load() {
        let mut circuit = Circuit::new(2);
        circuit
            .push_component(Component::inductor(NodeId(1), NodeId::GROUND, 1e-3, 0.0))
            .unwrap();
        circuit
            .push_component(Component::load(
                NodeId(1),
                NodeId::GROUND,
                Complex64::new(6.0, 3.0),
            ))
            .unwrap();
        assert_eq!(circuit.components().len(), 2);
    }

    #[test]
    fn impedance_uses_physics_conventions() {
        let r = ComponentKind::Resistor { ohm: 10.0 };
        assert_eq!(r.impedance(0, 1000.0).unwrap(), Complex64::new(10.0, 0.0));
        let l = ComponentKind::Inductor {
            henry: 1e-3,
            dcr_ohm: 0.5,
        };
        let zl = l.impedance(0, 1000.0).unwrap();
        assert!((zl.re - 0.5).abs() < 1e-15);
        assert!((zl.im - TAU).abs() < 1e-12);
        let c = ComponentKind::Capacitor { farad: 5e-6 };
        let zc = c.impedance(0, 1000.0).unwrap();
        assert!((zc.im + 1.0 / (TAU * 1000.0 * 5e-6)).abs() < 1e-12);
    }

    #[test]
    fn invalid_frequency_is_never_accepted_silently() {
        let mut circuit = Circuit::new(2);
        circuit
            .push_component(Component::resistor(NodeId(1), NodeId::GROUND, 10.0))
            .unwrap();
        circuit
            .push_source(VoltageSource::new(
                NodeId(1),
                NodeId::GROUND,
                Complex64::new(1.0, 0.0),
            ))
            .unwrap();
        for freq in [0.0, -1.0] {
            assert_eq!(
                circuit.solve(freq).unwrap_err(),
                CircuitError::InvalidFrequency(freq)
            );
        }
        for freq in [f64::NAN, f64::INFINITY] {
            assert!(matches!(
                circuit.solve(freq).unwrap_err(),
                CircuitError::InvalidFrequency(found) if found.to_bits() == freq.to_bits()
            ));
        }
    }
}
