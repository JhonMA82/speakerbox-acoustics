//! Medidas de la red: barridos, impedancia de entrada y transferencia.

use crate::curves::ComplexCurve;
use crate::error::{CircuitError, CircuitResult};
use crate::model::{Circuit, NodeId};
use crate::solver::Solution;
use num_complex::Complex64;

/// División compleja estable: `(a·conj(u))/(|u|²·escala)` con `u = b/escala`.
///
/// Evita que `|b|²` desborde o subdesborde y convierta un cociente finito en
/// `NaN` o infinito. Devuelve `None` si el divisor es cero o no finito.
fn stable_divide(a: Complex64, b: Complex64) -> Option<Complex64> {
    let scale = b.re.abs().max(b.im.abs());
    if scale == 0.0 || !scale.is_finite() {
        return None;
    }
    let normalized = b / scale;
    let denominator = normalized.norm_sqr() * scale;
    if denominator == 0.0 || !denominator.is_finite() {
        return None;
    }
    let result = a * normalized.conj() / denominator;
    if result.re.is_finite() && result.im.is_finite() {
        Some(result)
    } else {
        None
    }
}

impl Circuit {
    /// Resuelve la red en cada frecuencia de `freq_hz`.
    ///
    /// # Errores
    ///
    /// El primer error encontrado, con la frecuencia que lo produjo.
    pub fn sweep(&self, freq_hz: &[f64]) -> CircuitResult<Vec<Solution>> {
        freq_hz
            .iter()
            .map(|freq| self.solve(*freq))
            .collect::<CircuitResult<Vec<_>>>()
    }

    /// Impedancia vista por la única fuente del circuito a `freq_hz`.
    ///
    /// `Zin = −V / I`, donde `I` es la corriente de la fuente en
    /// [`Solution::source_currents`]. La red debe tener exactamente una fuente.
    ///
    /// # Errores
    ///
    /// - [`CircuitError::SourceCount`] si hay cero o más de una fuente.
    /// - [`CircuitError::ZeroReference`] si la fuente no entrega corriente
    ///   (circuito abierto).
    pub fn input_impedance(&self, freq_hz: f64) -> CircuitResult<Complex64> {
        if self.sources().len() != 1 {
            return Err(CircuitError::SourceCount {
                found: self.sources().len(),
            });
        }
        let solution = self.solve(freq_hz)?;
        let current = solution.source_currents[0];
        if current.norm() == 0.0 {
            return Err(CircuitError::ZeroReference {
                freq_hz,
                node: self.sources()[0].positive,
            });
        }
        stable_divide(-self.sources()[0].volts, current)
            .ok_or(CircuitError::NonFiniteResult { freq_hz })
    }

    /// Transferencia compleja `V(output) / V(reference)` a `freq_hz`.
    ///
    /// Es una relación de tensiones, no una respuesta acústica: no incluye
    /// polaridad, retardo ni calibración del altavoz.
    ///
    /// # Errores
    ///
    /// [`CircuitError::ZeroReference`] si la referencia vale cero.
    pub fn voltage_transfer(
        &self,
        freq_hz: f64,
        output: NodeId,
        reference: NodeId,
    ) -> CircuitResult<Complex64> {
        let solution = self.solve(freq_hz)?;
        let denominator = solution.voltage(reference)?;
        if denominator.norm() == 0.0 {
            return Err(CircuitError::ZeroReference {
                freq_hz,
                node: reference,
            });
        }
        stable_divide(solution.voltage(output)?, denominator)
            .ok_or(CircuitError::NonFiniteResult { freq_hz })
    }

    /// Transferencia compleja sobre una malla de frecuencias.
    ///
    /// # Errores
    ///
    /// - Cualquier error de [`Circuit::voltage_transfer`].
    /// - [`CircuitError::CurveNotIncreasing`] si la malla no crece
    ///   estrictamente, porque el resultado se devuelve como
    ///   [`ComplexCurve`].
    pub fn transfer_curve(
        &self,
        freq_hz: &[f64],
        output: NodeId,
        reference: NodeId,
    ) -> CircuitResult<ComplexCurve> {
        let values = freq_hz
            .iter()
            .map(|freq| self.voltage_transfer(*freq, output, reference))
            .collect::<CircuitResult<Vec<_>>>()?;
        ComplexCurve::new(freq_hz.to_vec(), values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Component, VoltageSource};

    fn divider() -> Circuit {
        let mut circuit = Circuit::new(3);
        circuit
            .push_source(VoltageSource::new(
                NodeId(1),
                NodeId::GROUND,
                Complex64::new(1.0, 0.0),
            ))
            .unwrap();
        circuit
            .push_component(Component::resistor(NodeId(1), NodeId(2), 1000.0))
            .unwrap();
        circuit
            .push_component(Component::resistor(NodeId(2), NodeId::GROUND, 1000.0))
            .unwrap();
        circuit
            .push_component(Component::capacitor(NodeId(2), NodeId::GROUND, 1e-6))
            .unwrap();
        circuit
            .push_component(Component::inductor(NodeId(2), NodeId::GROUND, 1e-3, 0.0))
            .unwrap();
        circuit
    }

    #[test]
    fn sweep_preserves_order_and_frequency() {
        let circuit = divider();
        let freqs = [100.0, 200.0, 400.0];
        let solutions = circuit.sweep(&freqs).unwrap();
        assert_eq!(solutions.len(), 3);
        for (solution, freq) in solutions.iter().zip(freqs) {
            assert_eq!(solution.frequency_hz, freq);
        }
    }

    #[test]
    fn sweep_reports_the_failing_frequency() {
        let circuit = divider();
        assert_eq!(
            circuit.sweep(&[100.0, 0.0]).unwrap_err(),
            CircuitError::InvalidFrequency(0.0)
        );
    }

    #[test]
    fn input_impedance_requires_exactly_one_source() {
        let mut circuit = divider();
        circuit
            .push_source(VoltageSource::new(
                NodeId(2),
                NodeId::GROUND,
                Complex64::new(1.0, 0.0),
            ))
            .unwrap();
        assert_eq!(
            circuit.input_impedance(1000.0).unwrap_err(),
            CircuitError::SourceCount { found: 2 }
        );
    }

    #[test]
    fn transfer_curve_has_the_requested_grid() {
        let circuit = divider();
        let freqs = [100.0, 1000.0, 10_000.0];
        let curve = circuit
            .transfer_curve(&freqs, NodeId(2), NodeId(1))
            .unwrap();
        assert_eq!(curve.freq_hz(), &freqs);
        let direct = circuit
            .voltage_transfer(1000.0, NodeId(2), NodeId(1))
            .unwrap();
        assert!((curve.at(1000.0).unwrap() - direct).norm() < 1e-15);
    }

    #[test]
    fn subnormal_reference_is_a_typed_error_not_infinity() {
        let mut circuit = Circuit::new(3);
        circuit
            .push_source(VoltageSource::new(
                NodeId(1),
                NodeId::GROUND,
                Complex64::new(1e-320, 0.0),
            ))
            .unwrap();
        circuit
            .push_component(Component::resistor(NodeId(1), NodeId::GROUND, 1.0))
            .unwrap();
        circuit
            .push_source(VoltageSource::new(
                NodeId(2),
                NodeId::GROUND,
                Complex64::new(1.0, 0.0),
            ))
            .unwrap();
        circuit
            .push_component(Component::resistor(NodeId(2), NodeId::GROUND, 1.0))
            .unwrap();
        assert_eq!(
            circuit
                .voltage_transfer(1000.0, NodeId(2), NodeId(1))
                .unwrap_err(),
            CircuitError::NonFiniteResult { freq_hz: 1000.0 }
        );
    }

    #[test]
    fn zero_reference_is_an_error_not_an_infinity() {
        let circuit = divider();
        assert_eq!(
            circuit
                .voltage_transfer(1000.0, NodeId(2), NodeId::GROUND)
                .unwrap_err(),
            CircuitError::ZeroReference {
                freq_hz: 1000.0,
                node: NodeId::GROUND
            }
        );
    }
}
