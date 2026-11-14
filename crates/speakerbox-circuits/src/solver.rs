//! Resolución por análisis nodal modificado (MNA).
//!
//! Cada frecuencia se resuelve como un sistema lineal complejo:
//!
//! ```text
//! A(ω) · x(ω) = b(ω)
//! ```
//!
//! donde las incógnitas `x` son las tensiones de los nodos no conectados a
//! tierra y las corrientes de cada fuente ideal. La eliminación es gaussiana
//! con pivoteo parcial: nunca se invierte la matriz, y un pivote nulo se
//! informa como [`CircuitError::Singular`].
//!
//! La calidad del resultado se comprueba con el residual relativo
//! `‖Ax − b‖∞ / (‖A‖∞·‖x‖∞ + ‖b‖∞)`. Si supera `1e-8`, la respuesta se
//! rechaza con [`CircuitError::Inaccurate`] en vez de devolver números que
//! parecen válidos.

use crate::error::{CircuitError, CircuitResult};
use crate::model::{Circuit, NodeId};
use num_complex::Complex64;

/// Límite del residual relativo aceptado.
const MAX_RELATIVE_RESIDUAL: f64 = 1e-8;

/// Resultado de resolver la red a una frecuencia.
#[derive(Clone, Debug, PartialEq)]
pub struct Solution {
    /// Frecuencia de la solución, en hercios.
    pub frequency_hz: f64,
    /// Tensión de cada nodo, indexada por [`NodeId`]. El nodo 0 vale cero.
    pub node_voltages: Vec<Complex64>,
    /// Corriente de cada fuente, en el mismo orden que [`Circuit::sources`].
    ///
    /// Es la corriente que entra en la fuente por su terminal positivo y sale
    /// por el negativo. Una fuente que entrega potencia a la red da un valor
    /// negativo. Por eso `Zin = −V / I`.
    pub source_currents: Vec<Complex64>,
    /// Residual relativo del sistema resuelto.
    pub relative_residual: f64,
    /// Cociente entre el pivote de menor y el de mayor magnitud.
    ///
    /// Es una estimación barata del condicionamiento, no un número de
    /// condición exacto. Un valor muy pequeño avisa de cancelación.
    pub pivot_ratio: f64,
}

impl Solution {
    /// Tensión del nodo `node`.
    ///
    /// # Errores
    ///
    /// [`CircuitError::UnknownNode`] si el nodo no existe en la solución.
    pub fn voltage(&self, node: NodeId) -> CircuitResult<Complex64> {
        self.node_voltages
            .get(node.index())
            .copied()
            .ok_or(CircuitError::UnknownNode {
                node,
                nodes: self.node_voltages.len(),
            })
    }
}

impl Circuit {
    /// Resuelve la red a `freq_hz`.
    ///
    /// # Errores
    ///
    /// - [`CircuitError::InvalidFrequency`] si la frecuencia no es finita o no
    ///   es positiva.
    /// - [`CircuitError::EmptyCircuit`] o [`CircuitError::NoSource`].
    /// - [`CircuitError::UnknownNode`] / [`CircuitError::InvalidComponent`] si
    ///   el circuito se construyó sin validar.
    /// - [`CircuitError::Singular`] si la red no tiene solución única (nodo
    ///   flotante, fuente en corto, rama desconectada).
    /// - [`CircuitError::Inaccurate`] si el residual supera el límite.
    pub fn solve(&self, freq_hz: f64) -> CircuitResult<Solution> {
        if !freq_hz.is_finite() || freq_hz <= 0.0 {
            return Err(CircuitError::InvalidFrequency(freq_hz));
        }
        self.validate()?;
        if self.sources().is_empty() {
            return Err(CircuitError::NoSource);
        }
        let node_unknowns = self.nodes() - 1;
        let size = node_unknowns + self.sources().len();
        let mut matrix = vec![vec![Complex64::new(0.0, 0.0); size]; size];
        let mut rhs = vec![Complex64::new(0.0, 0.0); size];

        for (index, component) in self.components().iter().enumerate() {
            let y = component.kind.admittance(index, freq_hz)?;
            let a = Self::unknown_index(component.a);
            let b = Self::unknown_index(component.b);
            if let Some(i) = a {
                matrix[i][i] += y;
            }
            if let Some(j) = b {
                matrix[j][j] += y;
            }
            if let (Some(i), Some(j)) = (a, b) {
                matrix[i][j] -= y;
                matrix[j][i] -= y;
            }
        }

        for (k, source) in self.sources().iter().enumerate() {
            let row = node_unknowns + k;
            let p = Self::unknown_index(source.positive);
            let n = Self::unknown_index(source.negative);
            if let Some(i) = p {
                matrix[i][row] += Complex64::new(1.0, 0.0);
                matrix[row][i] += Complex64::new(1.0, 0.0);
            }
            if let Some(j) = n {
                matrix[j][row] -= Complex64::new(1.0, 0.0);
                matrix[row][j] -= Complex64::new(1.0, 0.0);
            }
            rhs[row] = source.volts;
        }

        let original = matrix.clone();
        let original_rhs = rhs.clone();
        let pivot_ratio = match gaussian_elimination(&mut matrix, &mut rhs) {
            Ok(ratio) => ratio,
            Err(()) => return Err(CircuitError::Singular { freq_hz }),
        };
        let unknowns = back_substitution(&matrix, &rhs);

        let mut node_voltages = vec![Complex64::new(0.0, 0.0); self.nodes()];
        for (offset, value) in unknowns.iter().take(node_unknowns).enumerate() {
            node_voltages[offset + 1] = *value;
        }
        let source_currents = unknowns[node_unknowns..].to_vec();
        if node_voltages
            .iter()
            .chain(source_currents.iter())
            .any(|value| !value.re.is_finite() || !value.im.is_finite())
        {
            return Err(CircuitError::NonFiniteResult { freq_hz });
        }

        let relative_residual = relative_residual(&original, &original_rhs, &unknowns);
        if !relative_residual.is_finite() || relative_residual > MAX_RELATIVE_RESIDUAL {
            return Err(CircuitError::Inaccurate {
                freq_hz,
                relative_residual,
            });
        }

        Ok(Solution {
            frequency_hz: freq_hz,
            node_voltages,
            source_currents,
            relative_residual,
            pivot_ratio,
        })
    }

    fn unknown_index(node: NodeId) -> Option<usize> {
        if node.is_ground() {
            None
        } else {
            Some(node.index() - 1)
        }
    }
}

/// Eliminación gaussiana con pivoteo parcial sobre una matriz compleja.
///
/// Devuelve el cociente entre el pivote mínimo y el máximo. Un pivote nulo o no
/// finito devuelve `Err(())`: el llamador decide el error tipado.
fn gaussian_elimination(matrix: &mut [Vec<Complex64>], rhs: &mut [Complex64]) -> Result<f64, ()> {
    let n = rhs.len();
    let mut min_pivot = f64::INFINITY;
    let mut max_pivot = 0.0_f64;
    for col in 0..n {
        let mut pivot_row = col;
        let mut pivot_norm = matrix[col][col].norm();
        for (offset, row_values) in matrix.iter().enumerate().skip(col + 1) {
            let candidate = row_values[col].norm();
            if candidate > pivot_norm {
                pivot_norm = candidate;
                pivot_row = offset;
            }
        }
        if pivot_norm == 0.0 || !pivot_norm.is_finite() {
            return Err(());
        }
        if pivot_row != col {
            matrix.swap(pivot_row, col);
            rhs.swap(pivot_row, col);
        }
        min_pivot = min_pivot.min(pivot_norm);
        max_pivot = max_pivot.max(pivot_norm);
        let pivot = matrix[col][col];
        for row in col + 1..n {
            let factor = matrix[row][col] / pivot;
            if factor.norm() == 0.0 {
                continue;
            }
            let (head, tail) = matrix.split_at_mut(row);
            let pivot_row = &head[col];
            let target = &mut tail[0];
            target[col] = Complex64::new(0.0, 0.0);
            for k in col + 1..n {
                target[k] -= factor * pivot_row[k];
            }
            rhs[row] -= factor * rhs[col];
        }
    }
    if max_pivot == 0.0 {
        return Err(());
    }
    Ok(min_pivot / max_pivot)
}

fn back_substitution(matrix: &[Vec<Complex64>], rhs: &[Complex64]) -> Vec<Complex64> {
    let n = rhs.len();
    let mut x = vec![Complex64::new(0.0, 0.0); n];
    for row in (0..n).rev() {
        let mut sum = rhs[row];
        for (col, value) in x.iter().enumerate().skip(row + 1) {
            sum -= matrix[row][col] * value;
        }
        x[row] = sum / matrix[row][row];
    }
    x
}

fn relative_residual(matrix: &[Vec<Complex64>], rhs: &[Complex64], x: &[Complex64]) -> f64 {
    let mut residual = 0.0_f64;
    let mut matrix_norm = 0.0_f64;
    let mut rhs_norm = 0.0_f64;
    let mut x_norm = 0.0_f64;
    for (row, value) in rhs.iter().enumerate() {
        rhs_norm = rhs_norm.max(value.norm());
        x_norm = x_norm.max(x[row].norm());
        let mut row_norm = 0.0_f64;
        let mut sum = *value;
        for (col, coefficient) in matrix[row].iter().enumerate() {
            sum -= coefficient * x[col];
            row_norm += coefficient.norm();
        }
        residual = residual.max(sum.norm());
        matrix_norm = matrix_norm.max(row_norm);
    }
    let scale = matrix_norm * x_norm + rhs_norm;
    if scale > 0.0 {
        residual / scale
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Component, ComponentKind, VoltageSource};
    use std::f64::consts::TAU;

    fn driven_series(freq_hz: f64) -> (Circuit, Complex64) {
        let mut circuit = Circuit::new(4);
        circuit
            .push_source(VoltageSource::new(
                NodeId(1),
                NodeId::GROUND,
                Complex64::new(1.0, 0.0),
            ))
            .unwrap();
        circuit
            .push_component(Component::resistor(NodeId(1), NodeId(2), 10.0))
            .unwrap();
        circuit
            .push_component(Component::inductor(NodeId(2), NodeId(3), 1e-3, 0.0))
            .unwrap();
        circuit
            .push_component(Component::capacitor(NodeId(3), NodeId::GROUND, 5e-6))
            .unwrap();
        let z = Complex64::new(10.0, TAU * freq_hz * 1e-3 - 1.0 / (TAU * freq_hz * 5e-6));
        (circuit, z)
    }

    #[test]
    fn resistor_only_matches_ohm() {
        let mut circuit = Circuit::new(2);
        circuit
            .push_source(VoltageSource::new(
                NodeId(1),
                NodeId::GROUND,
                Complex64::new(1.0, 0.0),
            ))
            .unwrap();
        circuit
            .push_component(Component::resistor(NodeId(1), NodeId::GROUND, 8.0))
            .unwrap();
        let solution = circuit.solve(1000.0).unwrap();
        assert_eq!(
            solution.voltage(NodeId(1)).unwrap(),
            Complex64::new(1.0, 0.0)
        );
        assert!((solution.source_currents[0].re + 0.125).abs() < 1e-15);
        assert_eq!(
            solution.voltage(NodeId::GROUND).unwrap(),
            Complex64::new(0.0, 0.0)
        );
        assert!(solution.relative_residual < 1e-15);
    }

    #[test]
    fn series_rlc_matches_closed_form() {
        for freq in [20.0, 100.0, 1000.0, 2250.0, 10_000.0] {
            let (circuit, expected) = driven_series(freq);
            let solution = circuit.solve(freq).unwrap();
            let current = -solution.source_currents[0];
            let z = Complex64::new(1.0, 0.0) / current;
            assert!((z - expected).norm() / expected.norm() < 1e-12, "f={freq}");
        }
    }

    #[test]
    fn resonant_frequency_has_zero_imaginary_part() {
        let f0 = 1.0 / (TAU * (1e-3_f64 * 5e-6).sqrt());
        let (circuit, _) = driven_series(f0);
        let solution = circuit.solve(f0).unwrap();
        let z = -Complex64::new(1.0, 0.0) / solution.source_currents[0];
        assert!(z.im.abs() < 1e-9, "Im(Z)={}", z.im);
        assert!((z.re - 10.0).abs() < 1e-9);
    }

    #[test]
    fn resistive_divider_matches_voltage_ratio() {
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
            .push_component(Component::resistor(NodeId(2), NodeId::GROUND, 3000.0))
            .unwrap();
        let solution = circuit.solve(1000.0).unwrap();
        let ratio = solution.voltage(NodeId(2)).unwrap() / solution.voltage(NodeId(1)).unwrap();
        assert!((ratio.re - 0.75).abs() < 1e-15);
        assert!(ratio.im.abs() < 1e-15);
    }

    #[test]
    fn floating_nodes_and_shorted_sources_are_singular() {
        let mut floating = Circuit::new(5);
        floating
            .push_source(VoltageSource::new(
                NodeId(1),
                NodeId::GROUND,
                Complex64::new(1.0, 0.0),
            ))
            .unwrap();
        floating
            .push_component(Component::resistor(NodeId(1), NodeId(2), 10.0))
            .unwrap();
        floating
            .push_component(Component::capacitor(NodeId(3), NodeId(4), 1e-6))
            .unwrap();
        assert_eq!(
            floating.solve(1000.0).unwrap_err(),
            CircuitError::Singular { freq_hz: 1000.0 }
        );

        let mut shorted = Circuit::new(2);
        shorted
            .push_source(VoltageSource::new(
                NodeId(1),
                NodeId::GROUND,
                Complex64::new(1.0, 0.0),
            ))
            .unwrap();
        shorted
            .push_component(Component::resistor(NodeId(1), NodeId::GROUND, 10.0))
            .unwrap();
        assert!(shorted
            .push_component(Component::resistor(NodeId(1), NodeId::GROUND, 0.0))
            .is_err());
        let mut duplicated = shorted.clone();
        duplicated
            .push_source(VoltageSource::new(
                NodeId(1),
                NodeId::GROUND,
                Complex64::new(0.5, 0.0),
            ))
            .unwrap();
        assert_eq!(
            duplicated.solve(1000.0).unwrap_err(),
            CircuitError::Singular { freq_hz: 1000.0 }
        );
    }

    #[test]
    fn rejects_non_finite_inputs_without_panicking() {
        let (mut circuit, _) = driven_series(1000.0);
        circuit
            .push_component(Component::new(
                NodeId(2),
                NodeId::GROUND,
                ComponentKind::Load(crate::model::LoadImpedance::Constant(Complex64::new(
                    f64::NAN,
                    0.0,
                ))),
            ))
            .unwrap_err();
        let solution = circuit.solve(1000.0).unwrap();
        assert!(solution.node_voltages.iter().all(|v| v.re.is_finite()));
    }

    #[test]
    fn ground_node_is_not_an_unknown() {
        let mut circuit = Circuit::new(2);
        circuit
            .push_source(VoltageSource::new(
                NodeId(1),
                NodeId::GROUND,
                Complex64::new(2.0, 0.0),
            ))
            .unwrap();
        circuit
            .push_component(Component::resistor(NodeId(1), NodeId::GROUND, 4.0))
            .unwrap();
        let solution = circuit.solve(50.0).unwrap();
        assert_eq!(solution.node_voltages.len(), 2);
        assert_eq!(
            solution.voltage(NodeId::GROUND).unwrap(),
            Complex64::new(0.0, 0.0)
        );
        assert_eq!(
            solution.voltage(NodeId(1)).unwrap(),
            Complex64::new(2.0, 0.0)
        );
    }

    #[test]
    fn unknown_voltage_node_is_an_error() {
        let (circuit, _) = driven_series(1000.0);
        let solution = circuit.solve(1000.0).unwrap();
        assert_eq!(
            solution.voltage(NodeId(7)).unwrap_err(),
            CircuitError::UnknownNode {
                node: NodeId(7),
                nodes: 4
            }
        );
    }
}
