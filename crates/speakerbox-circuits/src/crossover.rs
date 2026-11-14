//! Prototipos de filtro, síntesis pasiva y utilidades de crossover.
//!
//! ## Qué se sintetiza
//!
//! Solo la escalera **Butterworth** de orden 1–4, doblemente terminada, con
//! valores `g_k = 2·sin((2k−1)π/(2n))` en la normalización clásica de 1 Ω y
//! 1 rad/s. Es la familia cuyos coeficientes se pueden comprobar contra la
//! función de transferencia analítica; las demás quedan como respuesta
//! analítica o fuera de alcance.
//!
//! ## Qué se responde
//!
//! [`FilterResponse`] evalúa la transferencia compleja de Butterworth y de
//! Linkwitz-Riley (dos secciones iguales en cascada). Sirve como objetivo de
//! diseño y como oráculo de la escalera sintetizada; **no** es una respuesta
//! acústica: no incluye altavoz, ángulo sólido, retardo ni calibración.
//!
//! ## Utilidades
//!
//! - [`l_pad`]: red de atenuación que conserva la impedancia de carga.
//! - [`zobel`]: compensación de la inductancia de bobina.
//!
//! ## Limitaciones declaradas
//!
//! - No se sintetizan pasivamente Bessel, Chebyshev, Legendre, Gaussian ni
//!   Linear: sus tablas no están verificadas aquí y no se copian del legacy.
//! - Linkwitz-Riley pasivo requiere secciones activas o buffers: se rechaza
//!   con [`CircuitError::Unsupported`] en vez de aproximarlo.

use crate::error::{CircuitError, CircuitResult};
use crate::model::{Circuit, Component, NodeId, VoltageSource};
use num_complex::Complex64;
use std::f64::consts::{PI, TAU};

/// Familia de la respuesta de transferencia.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterFamily {
    /// Máximamente plana; `−3 dB` en la frecuencia de corte.
    Butterworth,
    /// Dos secciones iguales en cascada; `−6 dB` en la frecuencia de corte.
    LinkwitzRiley,
}

/// Tipo de filtro.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterKind {
    /// Pasa bajas.
    LowPass,
    /// Pasa altas.
    HighPass,
}

/// Especificación de un filtro pasivo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FilterSpec {
    /// Familia.
    pub family: FilterFamily,
    /// Tipo.
    pub kind: FilterKind,
    /// Orden eléctrico (1–4).
    pub order: usize,
    /// Frecuencia de corte, en hercios.
    pub cutoff_hz: f64,
    /// Impedancia de terminación, en ohmios.
    pub impedance_ohm: f64,
}

/// Respuesta analítica de un prototipo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FilterResponse {
    family: FilterFamily,
    kind: FilterKind,
    order: usize,
    cutoff_hz: f64,
}

impl FilterResponse {
    /// Construye la respuesta validando familia, orden y corte.
    ///
    /// # Errores
    ///
    /// [`CircuitError::InvalidValue`] o [`CircuitError::InvalidFrequency`].
    pub fn new(
        family: FilterFamily,
        kind: FilterKind,
        order: usize,
        cutoff_hz: f64,
    ) -> CircuitResult<Self> {
        validate_order(family, order)?;
        check_positive(cutoff_hz, "frecuencia de corte")?;
        Ok(Self {
            family,
            kind,
            order,
            cutoff_hz,
        })
    }

    /// Familia.
    #[must_use]
    pub fn family(&self) -> FilterFamily {
        self.family
    }

    /// Tipo.
    #[must_use]
    pub fn kind(&self) -> FilterKind {
        self.kind
    }

    /// Orden.
    #[must_use]
    pub fn order(&self) -> usize {
        self.order
    }

    /// Frecuencia de corte, en hercios.
    #[must_use]
    pub fn cutoff_hz(&self) -> f64 {
        self.cutoff_hz
    }

    /// Transferencia compleja normalizada a `freq_hz`.
    ///
    /// # Errores
    ///
    /// [`CircuitError::InvalidFrequency`] si la frecuencia no es finita o no
    /// es positiva.
    pub fn transfer(&self, freq_hz: f64) -> CircuitResult<Complex64> {
        if !freq_hz.is_finite() || freq_hz <= 0.0 {
            return Err(CircuitError::InvalidFrequency(freq_hz));
        }
        let ratio = freq_hz / self.cutoff_hz;
        let response = match self.family {
            FilterFamily::Butterworth => butterworth(self.kind, self.order, ratio),
            FilterFamily::LinkwitzRiley => {
                let half = butterworth(self.kind, self.order / 2, ratio);
                half * half
            }
        };
        Ok(response)
    }
}

/// Escalera sintetizada y sus nodos de referencia.
#[derive(Clone, Debug, PartialEq)]
pub struct Ladder {
    /// Circuito con fuente de 1 V, resistencia de fuente y carga.
    pub circuit: Circuit,
    /// Nodo del terminal positivo de la fuente.
    pub source_node: NodeId,
    /// Nodo de salida, sobre la carga.
    pub output_node: NodeId,
    /// Impedancia de terminación, en ohmios.
    pub impedance_ohm: f64,
}

/// Sintetiza la escalera Butterworth doblemente terminada.
///
/// El circuito resultante incluye la resistencia de fuente `Z0` y la carga
/// `Z0`, de modo que la transferencia normalizada es
/// `2·V(salida)/V(fuente)`.
///
/// # Errores
///
/// - [`CircuitError::Unsupported`] para familias sin síntesis pasiva.
/// - [`CircuitError::InvalidValue`] para orden o impedancia fuera de rango.
/// - [`CircuitError::InvalidFrequency`] para un corte no positivo.
pub fn synthesize(spec: &FilterSpec) -> CircuitResult<Ladder> {
    if spec.family != FilterFamily::Butterworth {
        return Err(CircuitError::Unsupported(
            "la síntesis pasiva solo está implementada para Butterworth",
        ));
    }
    validate_order(spec.family, spec.order)?;
    check_positive(spec.cutoff_hz, "frecuencia de corte")?;
    check_positive(spec.impedance_ohm, "impedancia de terminación")?;

    let order = spec.order;
    let z0 = spec.impedance_ohm;
    let omega = TAU * spec.cutoff_hz;
    let mut nodes = Vec::with_capacity(order);
    let mut current = 2_u32;
    for index in 0..order {
        let series = index % 2 == 0;
        nodes.push((current, series));
        if series {
            current += 1;
        }
    }
    let output_node = current;
    let mut circuit = Circuit::new(output_node as usize + 1);
    circuit.push_source(VoltageSource::new(
        NodeId(1),
        NodeId::GROUND,
        Complex64::new(1.0, 0.0),
    ))?;
    circuit.push_component(Component::resistor(NodeId(1), NodeId(2), z0))?;

    for (index, (node, series)) in nodes.iter().enumerate() {
        let k = index + 1;
        let g = 2.0 * ((2.0 * k as f64 - 1.0) * PI / (2.0 * order as f64)).sin();
        let high_pass = spec.kind == FilterKind::HighPass;
        let component = match (series, high_pass) {
            (true, false) => {
                Component::inductor(NodeId(*node), NodeId(node + 1), g * z0 / omega, 0.0)
            }
            (true, true) => {
                Component::capacitor(NodeId(*node), NodeId(node + 1), 1.0 / (g * z0 * omega))
            }
            (false, false) => Component::capacitor(NodeId(*node), NodeId::GROUND, g / (z0 * omega)),
            (false, true) => {
                Component::inductor(NodeId(*node), NodeId::GROUND, z0 / (g * omega), 0.0)
            }
        };
        circuit.push_component(component)?;
    }
    circuit.push_component(Component::resistor(NodeId(output_node), NodeId::GROUND, z0))?;

    Ok(Ladder {
        circuit,
        source_node: NodeId(1),
        output_node: NodeId(output_node),
        impedance_ohm: z0,
    })
}

impl Ladder {
    /// Transferencia normalizada `2·V(salida)/V(fuente)` a `freq_hz`.
    ///
    /// # Errores
    ///
    /// Cualquier error del solver o una referencia nula.
    pub fn normalized_transfer(&self, freq_hz: f64) -> CircuitResult<Complex64> {
        let transfer =
            self.circuit
                .voltage_transfer(freq_hz, self.output_node, self.source_node)?;
        Ok(transfer * 2.0)
    }
}

/// Red L de atenuación que conserva la impedancia de carga.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LPad {
    /// Resistencia en serie, en ohmios. Cero en bypass.
    pub series_ohm: f64,
    /// Resistencia en paralelo, en ohmios. `None` en bypass.
    pub shunt_ohm: Option<f64>,
}

/// Calcula un L-pad para `attenuation_db` sobre una carga `impedance_ohm`.
///
/// ```text
/// k = 10^(−dB/20)
/// R_serie = Z0·(1−k)
/// R_paralelo = Z0·k/(1−k)
/// ```
///
/// # Errores
///
/// [`CircuitError::InvalidValue`] si la atenuación o la impedancia no son
/// válidas. `0 dB` devuelve un bypass explícito, no una red degenerada.
pub fn l_pad(attenuation_db: f64, impedance_ohm: f64) -> CircuitResult<LPad> {
    check_positive(impedance_ohm, "impedancia de carga")?;
    if !attenuation_db.is_finite() || attenuation_db < 0.0 {
        return Err(CircuitError::InvalidValue(
            "la atenuación debe ser finita y >= 0 dB",
        ));
    }
    let k = 10.0_f64.powf(-attenuation_db / 20.0);
    if k <= 0.0 || !k.is_finite() {
        return Err(CircuitError::InvalidValue("atenuación fuera de rango"));
    }
    if (k - 1.0).abs() < 1e-15 {
        return Ok(LPad {
            series_ohm: 0.0,
            shunt_ohm: None,
        });
    }
    Ok(LPad {
        series_ohm: impedance_ohm * (1.0 - k),
        shunt_ohm: Some(impedance_ohm * k / (1.0 - k)),
    })
}

impl LPad {
    /// Añade la red entre `input` y `output` y devuelve el nodo donde debe ir
    /// la carga.
    ///
    /// En bypass no añade nada y devuelve `input`: el llamador conecta la
    /// carga ahí.
    ///
    /// # Errores
    ///
    /// Errores de construcción del circuito.
    pub fn insert(
        &self,
        circuit: &mut Circuit,
        input: NodeId,
        output: NodeId,
    ) -> CircuitResult<NodeId> {
        match self.shunt_ohm {
            Some(shunt) => {
                circuit.push_component(Component::resistor(input, output, self.series_ohm))?;
                circuit.push_component(Component::resistor(output, NodeId::GROUND, shunt))?;
                Ok(output)
            }
            None => Ok(input),
        }
    }

    /// `true` si la red no atenúa.
    #[must_use]
    pub fn is_bypass(&self) -> bool {
        self.shunt_ohm.is_none()
    }
}

/// Compensación Zobel de la impedancia de un altavoz.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zobel {
    /// Resistencia en serie con el condensador, en ohmios.
    pub resistance_ohm: f64,
    /// Capacidad, en faradios.
    pub capacitance_f: f64,
}

/// Calcula la red Zobel `R = Re`, `C = Le/Re²`.
///
/// # Errores
///
/// [`CircuitError::InvalidValue`] si `Re` no es positivo o `Le` no es finito
/// y no negativo.
pub fn zobel(re_ohm: f64, le_h: f64) -> CircuitResult<Zobel> {
    check_positive(re_ohm, "resistencia de bobina")?;
    if !le_h.is_finite() || le_h < 0.0 {
        return Err(CircuitError::InvalidValue(
            "la inductancia de bobina debe ser finita y >= 0 H",
        ));
    }
    Ok(Zobel {
        resistance_ohm: re_ohm,
        capacitance_f: le_h / (re_ohm * re_ohm),
    })
}

impl Zobel {
    /// Añade la red en paralelo con el altavoz conectado entre `node` y
    /// tierra, usando `middle` como nodo intermedio.
    ///
    /// El llamador reserva ese nodo al construir el circuito. No hace nada si
    /// `capacitance_f` es cero: sin inductancia no hay nada que compensar.
    ///
    /// # Errores
    ///
    /// Errores de construcción del circuito.
    pub fn insert(&self, circuit: &mut Circuit, node: NodeId, middle: NodeId) -> CircuitResult<()> {
        if self.capacitance_f <= 0.0 {
            return Ok(());
        }
        circuit.push_component(Component::resistor(node, middle, self.resistance_ohm))?;
        circuit.push_component(Component::capacitor(
            middle,
            NodeId::GROUND,
            self.capacitance_f,
        ))?;
        Ok(())
    }
}

fn butterworth(kind: FilterKind, order: usize, ratio: f64) -> Complex64 {
    let s = Complex64::new(0.0, ratio);
    let mut denominator = Complex64::new(1.0, 0.0);
    for k in 1..=order {
        let theta = PI * (2.0 * k as f64 + order as f64 - 1.0) / (2.0 * order as f64);
        let pole = Complex64::from_polar(1.0, theta);
        denominator *= s - pole;
    }
    let numerator = match kind {
        FilterKind::LowPass => Complex64::new(1.0, 0.0),
        FilterKind::HighPass => s.powi(order as i32),
    };
    numerator / denominator
}

fn validate_order(family: FilterFamily, order: usize) -> CircuitResult<()> {
    match family {
        FilterFamily::Butterworth if (1..=4).contains(&order) => Ok(()),
        FilterFamily::LinkwitzRiley if matches!(order, 2 | 4) => Ok(()),
        FilterFamily::Butterworth => Err(CircuitError::InvalidValue(
            "el orden Butterworth debe estar entre 1 y 4",
        )),
        FilterFamily::LinkwitzRiley => Err(CircuitError::InvalidValue(
            "Linkwitz-Riley solo admite orden 2 o 4",
        )),
    }
}

fn check_positive(value: f64, field: &'static str) -> CircuitResult<()> {
    if !value.is_finite() || value <= 0.0 {
        return Err(CircuitError::InvalidValue(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_1_SQRT_2;

    fn spec(family: FilterFamily, kind: FilterKind, order: usize) -> FilterSpec {
        FilterSpec {
            family,
            kind,
            order,
            cutoff_hz: 1000.0,
            impedance_ohm: 8.0,
        }
    }

    #[test]
    fn butterworth_magnitude_is_maximally_flat() {
        for order in 1..=4 {
            let response = FilterResponse::new(
                FilterFamily::Butterworth,
                FilterKind::LowPass,
                order,
                1000.0,
            )
            .unwrap();
            let at_cutoff = response.transfer(1000.0).unwrap().norm();
            assert!(
                (at_cutoff - FRAC_1_SQRT_2).abs() < 1e-12,
                "orden {order}: {at_cutoff}"
            );
            for ratio in [0.25_f64, 0.5, 2.0, 4.0] {
                let expected = 1.0 / (1.0 + ratio.powi(2 * order as i32)).sqrt();
                let found = response.transfer(1000.0 * ratio).unwrap().norm();
                assert!((found - expected).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn high_pass_mirrors_low_pass() {
        for order in 1..=4 {
            let low = FilterResponse::new(
                FilterFamily::Butterworth,
                FilterKind::LowPass,
                order,
                1000.0,
            )
            .unwrap();
            let high = FilterResponse::new(
                FilterFamily::Butterworth,
                FilterKind::HighPass,
                order,
                1000.0,
            )
            .unwrap();
            for ratio in [0.1, 0.5, 1.0, 2.0, 10.0] {
                let freq = 1000.0 * ratio;
                let sum = low.transfer(freq).unwrap().norm().powi(2)
                    + high.transfer(freq).unwrap().norm().powi(2);
                assert!((sum - 1.0).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn linkwitz_riley_is_the_square_of_butterworth() {
        let butter =
            FilterResponse::new(FilterFamily::Butterworth, FilterKind::LowPass, 2, 1000.0).unwrap();
        let lr = FilterResponse::new(FilterFamily::LinkwitzRiley, FilterKind::LowPass, 4, 1000.0)
            .unwrap();
        for freq in [100.0, 500.0, 1000.0, 2000.0] {
            let expected = butter.transfer(freq).unwrap().powi(2);
            assert!((lr.transfer(freq).unwrap() - expected).norm() < 1e-15);
        }
        assert!((lr.transfer(1000.0).unwrap().norm() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn invalid_specs_are_rejected() {
        assert!(
            FilterResponse::new(FilterFamily::Butterworth, FilterKind::LowPass, 0, 1000.0).is_err()
        );
        assert!(
            FilterResponse::new(FilterFamily::Butterworth, FilterKind::LowPass, 5, 1000.0).is_err()
        );
        assert!(
            FilterResponse::new(FilterFamily::LinkwitzRiley, FilterKind::LowPass, 3, 1000.0)
                .is_err()
        );
        assert!(
            FilterResponse::new(FilterFamily::Butterworth, FilterKind::LowPass, 2, 0.0).is_err()
        );
        assert!(
            FilterResponse::new(FilterFamily::Butterworth, FilterKind::LowPass, 2, 1000.0)
                .unwrap()
                .transfer(0.0)
                .is_err()
        );
        assert!(synthesize(&spec(FilterFamily::LinkwitzRiley, FilterKind::LowPass, 4)).is_err());
    }

    #[test]
    fn synthesized_ladder_matches_the_analytic_low_pass() {
        for order in 1..=4 {
            let spec = spec(FilterFamily::Butterworth, FilterKind::LowPass, order);
            let ladder = synthesize(&spec).unwrap();
            let response = FilterResponse::new(
                FilterFamily::Butterworth,
                FilterKind::LowPass,
                order,
                spec.cutoff_hz,
            )
            .unwrap();
            for ratio in [0.1, 0.25, 0.5, 1.0, 2.0, 4.0, 10.0] {
                let freq = spec.cutoff_hz * ratio;
                let actual = ladder.normalized_transfer(freq).unwrap();
                let expected = response.transfer(freq).unwrap();
                assert!(
                    (actual - expected).norm() < 1e-9,
                    "orden {order} a {ratio}·fc: {actual:?} vs {expected:?}"
                );
            }
        }
    }

    #[test]
    fn synthesized_ladder_matches_the_analytic_high_pass() {
        for order in 1..=4 {
            let spec = spec(FilterFamily::Butterworth, FilterKind::HighPass, order);
            let ladder = synthesize(&spec).unwrap();
            let response = FilterResponse::new(
                FilterFamily::Butterworth,
                FilterKind::HighPass,
                order,
                spec.cutoff_hz,
            )
            .unwrap();
            for ratio in [0.1, 0.25, 0.5, 1.0, 2.0, 4.0, 10.0] {
                let freq = spec.cutoff_hz * ratio;
                let actual = ladder.normalized_transfer(freq).unwrap();
                let expected = response.transfer(freq).unwrap();
                assert!(
                    (actual - expected).norm() < 1e-9,
                    "orden {order} a {ratio}·fc: {actual:?} vs {expected:?}"
                );
            }
        }
    }

    #[test]
    fn l_pad_keeps_the_load_and_sets_the_level() {
        let z0 = 8.0;
        for db in [0.0, 3.0, 6.0, 12.0] {
            let pad = l_pad(db, z0).unwrap();
            let nodes = if pad.is_bypass() { 2 } else { 3 };
            let mut circuit = Circuit::new(nodes);
            circuit
                .push_source(VoltageSource::new(
                    NodeId(1),
                    NodeId::GROUND,
                    Complex64::new(1.0, 0.0),
                ))
                .unwrap();
            let load_node = pad.insert(&mut circuit, NodeId(1), NodeId(2)).unwrap();
            circuit
                .push_component(Component::new(
                    load_node,
                    NodeId::GROUND,
                    crate::model::ComponentKind::Load(crate::model::LoadImpedance::Constant(
                        Complex64::new(z0, 0.0),
                    )),
                ))
                .unwrap();
            let transfer = circuit
                .voltage_transfer(1000.0, load_node, NodeId(1))
                .unwrap();
            let expected = 10.0_f64.powf(-db / 20.0);
            assert!((transfer.re - expected).abs() < 1e-12, "db={db}");
            let z_in = circuit.input_impedance(1000.0).unwrap();
            assert!((z_in.re - z0).abs() < 1e-9, "Zin={z_in:?} a {db} dB");
            assert!(z_in.im.abs() < 1e-9);
            assert_eq!(pad.is_bypass(), db == 0.0);
        }
    }

    #[test]
    fn zobel_flattens_the_inductive_driver() {
        let (re, le) = (6.4, 1.5e-3);
        let network = zobel(re, le).unwrap();
        assert!((network.resistance_ohm - re).abs() < 1e-15);
        assert!((network.capacitance_f - le / (re * re)).abs() < 1e-18);
        let mut circuit = Circuit::new(3);
        circuit
            .push_source(VoltageSource::new(
                NodeId(1),
                NodeId::GROUND,
                Complex64::new(1.0, 0.0),
            ))
            .unwrap();
        circuit
            .push_component(Component::inductor(NodeId(1), NodeId::GROUND, le, re))
            .unwrap();
        network.insert(&mut circuit, NodeId(1), NodeId(2)).unwrap();
        let mut min = f64::INFINITY;
        let mut max = 0.0_f64;
        for index in 0..24 {
            let freq = 100.0 * 200.0_f64.powf(index as f64 / 23.0);
            let z = circuit.input_impedance(freq).unwrap();
            assert!(z.re >= -1e-12);
            min = min.min(z.norm());
            max = max.max(z.norm());
        }
        assert!((max - min) / min < 0.03, "variación {min}..{max}");
    }

    #[test]
    fn invalid_network_parameters_are_rejected() {
        assert!(l_pad(-1.0, 8.0).is_err());
        assert!(l_pad(6.0, 0.0).is_err());
        assert!(zobel(0.0, 1e-3).is_err());
        assert!(zobel(6.4, -1e-3).is_err());
        assert!(zobel(6.4, f64::NAN).is_err());
    }
}
