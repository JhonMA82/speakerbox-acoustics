//! Optimización restringida de componentes contra una respuesta objetivo.
//!
//! El optimizador recibe una red resuelta por este mismo crate, una lista de
//! variables (componente y campo), y un objetivo de magnitud en dB. Ajusta los
//! valores en **logaritmo**, de modo que la positividad es estructural, con
//! Levenberg–Marquardt y Jacobiano por diferencias centrales.
//!
//! Es un optimizador de **una salida**: la respuesta de un divisor de tensión
//! en un nodo. No modela un sistema multivía: combinar altavoces exige la
//! referencia de presión compleja que el proyecto todavía no ha fijado.
//!
//! ```text
//! r(f) = √w(f) · (20·log10|H(f)| − objetivo_db(f))
//! ```

use crate::error::{CircuitError, CircuitResult};
use crate::model::{Circuit, ComponentKind, NodeId};
use num_complex::Complex64;

/// Campo de un componente que se puede optimizar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VariableField {
    /// Resistencia en ohmios.
    Resistance,
    /// Inductancia en henrios.
    Inductance,
    /// Capacidad en faradios.
    Capacitance,
    /// Resistencia serie de una bobina, en ohmios.
    Dcr,
}

/// Variable de diseño: componente, campo y límites.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DesignVariable {
    /// Índice del componente en [`Circuit::components`].
    pub component: usize,
    /// Campo a variar.
    pub field: VariableField,
    /// Mínimo admisible, estrictamente positivo.
    pub min: f64,
    /// Máximo admisible, mayor que `min`.
    pub max: f64,
}

/// Objetivo de magnitud en dB sobre una malla de frecuencias.
#[derive(Clone, Debug, PartialEq)]
pub struct OptimizationTarget {
    /// Frecuencias en hercios, estrictamente crecientes.
    pub freq_hz: Vec<f64>,
    /// Magnitud objetivo en dB por punto.
    pub magnitude_db: Vec<f64>,
    /// Peso por punto; vacío equivale a peso 1.
    pub weights: Vec<f64>,
}

impl OptimizationTarget {
    /// Construye el objetivo validando longitudes, frecuencias y pesos.
    ///
    /// # Errores
    ///
    /// [`CircuitError::InvalidValue`] si algo no cumple su contrato.
    pub fn new(
        freq_hz: Vec<f64>,
        magnitude_db: Vec<f64>,
        weights: Vec<f64>,
    ) -> CircuitResult<Self> {
        if freq_hz.len() < 2 || freq_hz.len() != magnitude_db.len() {
            return Err(CircuitError::InvalidValue(
                "el objetivo necesita al menos dos puntos y magnitudes del mismo tamaño",
            ));
        }
        if !weights.is_empty() && weights.len() != freq_hz.len() {
            return Err(CircuitError::InvalidValue(
                "los pesos deben estar vacíos o tener el tamaño del objetivo",
            ));
        }
        if !weights.is_empty() && weights.iter().sum::<f64>() <= 0.0 {
            return Err(CircuitError::InvalidValue(
                "los pesos no pueden ser todos cero",
            ));
        }
        for (index, freq) in freq_hz.iter().enumerate() {
            if !freq.is_finite() || *freq <= 0.0 {
                return Err(CircuitError::InvalidFrequency(*freq));
            }
            if index > 0 && *freq <= freq_hz[index - 1] {
                return Err(CircuitError::InvalidValue(
                    "las frecuencias del objetivo deben crecer",
                ));
            }
            if !magnitude_db[index].is_finite() {
                return Err(CircuitError::InvalidValue(
                    "las magnitudes objetivo deben ser finitas",
                ));
            }
            if let Some(weight) = weights.get(index) {
                if !weight.is_finite() || *weight < 0.0 {
                    return Err(CircuitError::InvalidValue(
                        "los pesos deben ser finitos y no negativos",
                    ));
                }
            }
        }
        Ok(Self {
            freq_hz,
            magnitude_db,
            weights,
        })
    }
}

/// Opciones del optimizador.
#[derive(Clone, Debug, PartialEq)]
pub struct OptimizationOptions {
    /// Iteraciones máximas.
    pub max_iterations: usize,
    /// Tolerancia de mejora relativa del coste.
    pub tolerance: f64,
}

impl Default for OptimizationOptions {
    fn default() -> Self {
        Self {
            max_iterations: 120,
            tolerance: 1e-10,
        }
    }
}

/// Avisos del proceso.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OptimizationWarning {
    /// Un parámetro terminó en su límite.
    ParameterAtBound(usize),
    /// El error máximo supera 3 dB.
    PoorFit,
    /// No hubo mejora y el optimizador se detuvo.
    NoProgress,
}

/// Informe del ajuste.
#[derive(Clone, Debug, PartialEq)]
pub struct OptimizationReport {
    /// Valores finales en el orden de las variables.
    pub values: Vec<f64>,
    /// Valores de partida.
    pub initial_values: Vec<f64>,
    /// Iteraciones ejecutadas.
    pub iterations: usize,
    /// `true` si se alcanzó la tolerancia.
    pub converged: bool,
    /// Coste inicial.
    pub initial_cost: f64,
    /// Coste final.
    pub final_cost: f64,
    /// Error cuadrático medio en dB.
    pub rmse_db: f64,
    /// Error absoluto máximo en dB.
    pub max_error_db: f64,
    /// Avisos sin duplicados.
    pub warnings: Vec<OptimizationWarning>,
}

/// Optimiza las variables para aproximar el objetivo.
///
/// # Errores
///
/// - [`CircuitError::InvalidValue`] si las variables no son válidas o la red
///   inicial no se puede evaluar.
/// - [`CircuitError::SourceCount`] o errores del solver al evaluar la
///   transferencia.
pub fn optimize(
    circuit: &Circuit,
    variables: &[DesignVariable],
    target: &OptimizationTarget,
    output: NodeId,
    reference: NodeId,
    options: &OptimizationOptions,
) -> CircuitResult<OptimizationReport> {
    if variables.is_empty() {
        return Err(CircuitError::InvalidValue("no hay variables que optimizar"));
    }
    let initial_values = read_values(circuit, variables)?;
    let mut x: Vec<f64> = initial_values.iter().map(|value| value.ln()).collect();
    let bounds: Vec<(f64, f64)> = variables
        .iter()
        .map(|variable| {
            if !variable.min.is_finite()
                || !variable.max.is_finite()
                || variable.min <= 0.0
                || variable.max <= variable.min
            {
                Err(CircuitError::InvalidValue(
                    "cada variable exige 0 < min < max finitos",
                ))
            } else {
                Ok((variable.min.ln(), variable.max.ln()))
            }
        })
        .collect::<CircuitResult<Vec<_>>>()?;
    for (index, value) in x.iter().enumerate() {
        if *value < bounds[index].0 || *value > bounds[index].1 {
            return Err(CircuitError::InvalidValue(
                "el valor inicial de una variable está fuera de sus límites",
            ));
        }
    }

    let evaluation = Evaluation {
        circuit,
        variables,
        target,
        output,
        reference,
    };
    let mut cost = evaluation.cost(&x).ok_or(CircuitError::InvalidValue(
        "la red inicial no se puede evaluar en todo el objetivo",
    ))?;
    let initial_cost = cost;
    let mut lambda = 1e-3;
    let mut iterations = 0;
    let mut converged = false;

    while iterations < options.max_iterations {
        iterations += 1;
        let residual = match evaluation.residual(&x) {
            Some(residual) => residual,
            None => break,
        };
        let jacobian = match evaluation.jacobian(&x, 1e-5) {
            Some(jacobian) => jacobian,
            None => break,
        };
        let delta = match solve_normal(&jacobian, &residual, lambda) {
            Some(delta) => delta,
            None => break,
        };
        let candidate = clamp(&add(&x, &delta), &bounds);
        let candidate_cost = evaluation.cost(&candidate).unwrap_or(f64::INFINITY);
        if candidate_cost.is_finite() && candidate_cost < cost {
            let improvement = (cost - candidate_cost) / cost.max(f64::MIN_POSITIVE);
            x = candidate;
            cost = candidate_cost;
            lambda = (lambda / 3.0).max(1e-12);
            if improvement < options.tolerance || max_abs(&delta) < 1e-9 {
                converged = true;
                break;
            }
        } else {
            lambda *= 10.0;
            if lambda > 1e12 {
                break;
            }
        }
    }

    let values: Vec<f64> = x.iter().map(|value| value.exp()).collect();
    let stats = evaluation.statistics(&values)?;
    let mut warnings = Vec::new();
    for (index, value) in x.iter().enumerate() {
        if (*value - bounds[index].0).abs() < 1e-6 || (*value - bounds[index].1).abs() < 1e-6 {
            warnings.push(OptimizationWarning::ParameterAtBound(index));
        }
    }
    if stats.max_error_db > 3.0 {
        warnings.push(OptimizationWarning::PoorFit);
    }
    if !converged && cost >= initial_cost {
        warnings.push(OptimizationWarning::NoProgress);
    }

    Ok(OptimizationReport {
        values,
        initial_values,
        iterations,
        converged,
        initial_cost,
        final_cost: cost,
        rmse_db: stats.rmse_db,
        max_error_db: stats.max_error_db,
        warnings,
    })
}

struct Evaluation<'a> {
    circuit: &'a Circuit,
    variables: &'a [DesignVariable],
    target: &'a OptimizationTarget,
    output: NodeId,
    reference: NodeId,
}

struct Statistics {
    rmse_db: f64,
    max_error_db: f64,
}

impl Evaluation<'_> {
    fn circuit_with(&self, x: &[f64]) -> CircuitResult<Circuit> {
        let mut circuit = self.circuit.clone();
        for (index, variable) in self.variables.iter().enumerate() {
            let component = circuit.components().get(variable.component).ok_or(
                CircuitError::UnknownComponent {
                    index: variable.component,
                },
            )?;
            let kind = set_field(&component.kind, variable.field, x[index].exp())?;
            circuit.replace_component(variable.component, kind)?;
        }
        Ok(circuit)
    }

    fn residual(&self, x: &[f64]) -> Option<Vec<f64>> {
        let circuit = self.circuit_with(x).ok()?;
        let mut residual = Vec::with_capacity(self.target.freq_hz.len());
        for (index, freq) in self.target.freq_hz.iter().enumerate() {
            let transfer = circuit
                .voltage_transfer(*freq, self.output, self.reference)
                .ok()?;
            let db = magnitude_db(transfer);
            let weight = self
                .target
                .weights
                .get(index)
                .copied()
                .unwrap_or(1.0)
                .sqrt();
            residual.push(weight * (db - self.target.magnitude_db[index]));
        }
        Some(residual)
    }

    fn cost(&self, x: &[f64]) -> Option<f64> {
        self.residual(x)
            .map(|residual| residual.iter().map(|value| value * value).sum())
    }

    fn jacobian(&self, x: &[f64], step: f64) -> Option<Vec<Vec<f64>>> {
        let n = x.len();
        let mut columns = vec![Vec::new(); n];
        for index in 0..n {
            let mut plus = x.to_vec();
            let mut minus = x.to_vec();
            plus[index] += step;
            minus[index] -= step;
            let up = self.residual(&plus)?;
            let down = self.residual(&minus)?;
            let span = plus[index] - minus[index];
            columns[index] = up.iter().zip(&down).map(|(a, b)| (a - b) / span).collect();
        }
        let rows = columns.first().map_or(0, Vec::len);
        Some(
            (0..rows)
                .map(|row| (0..n).map(|index| columns[index][row]).collect())
                .collect(),
        )
    }

    fn statistics(&self, values: &[f64]) -> CircuitResult<Statistics> {
        let x: Vec<f64> = values.iter().map(|value| value.ln()).collect();
        let circuit = self.circuit_with(&x)?;
        let mut squared = 0.0_f64;
        let mut sum_weights = 0.0_f64;
        let mut max_error_db = 0.0_f64;
        for (index, freq) in self.target.freq_hz.iter().enumerate() {
            let predicted = circuit
                .voltage_transfer(*freq, self.output, self.reference)
                .map_err(|_| CircuitError::InvalidValue("la red ajustada no se puede evaluar"))?;
            let error = magnitude_db(predicted) - self.target.magnitude_db[index];
            let weight = self.target.weights.get(index).copied().unwrap_or(1.0);
            squared += weight * error * error;
            sum_weights += weight;
            if weight > 0.0 {
                max_error_db = max_error_db.max(error.abs());
            }
        }
        // RMSE ponderado: Σw·e² / Σw. Con pesos uniformes coincide con el
        // promedio simple; escalar todos los pesos no cambia el valor.
        let denominator = if sum_weights > 0.0 { sum_weights } else { 1.0 };
        Ok(Statistics {
            rmse_db: (squared / denominator).sqrt(),
            max_error_db,
        })
    }
}

fn read_values(circuit: &Circuit, variables: &[DesignVariable]) -> CircuitResult<Vec<f64>> {
    variables
        .iter()
        .map(|variable| {
            let component = circuit.components().get(variable.component).ok_or(
                CircuitError::UnknownComponent {
                    index: variable.component,
                },
            )?;
            get_field(&component.kind, variable.field).ok_or(CircuitError::InvalidValue(
                "el campo no corresponde al tipo de componente",
            ))
        })
        .collect()
}

fn get_field(kind: &ComponentKind, field: VariableField) -> Option<f64> {
    match (kind, field) {
        (ComponentKind::Resistor { ohm }, VariableField::Resistance) => Some(*ohm),
        (ComponentKind::Inductor { henry, .. }, VariableField::Inductance) => Some(*henry),
        (ComponentKind::Inductor { dcr_ohm, .. }, VariableField::Dcr) => Some(*dcr_ohm),
        (ComponentKind::Capacitor { farad }, VariableField::Capacitance) => Some(*farad),
        _ => None,
    }
}

fn set_field(
    kind: &ComponentKind,
    field: VariableField,
    value: f64,
) -> CircuitResult<ComponentKind> {
    if !value.is_finite() || value <= 0.0 {
        return Err(CircuitError::InvalidValue("valor optimizado no positivo"));
    }
    match (kind, field) {
        (ComponentKind::Resistor { .. }, VariableField::Resistance) => {
            Ok(ComponentKind::Resistor { ohm: value })
        }
        (ComponentKind::Inductor { dcr_ohm, .. }, VariableField::Inductance) => {
            Ok(ComponentKind::Inductor {
                henry: value,
                dcr_ohm: *dcr_ohm,
            })
        }
        (ComponentKind::Inductor { henry, .. }, VariableField::Dcr) => {
            Ok(ComponentKind::Inductor {
                henry: *henry,
                dcr_ohm: value,
            })
        }
        (ComponentKind::Capacitor { .. }, VariableField::Capacitance) => {
            Ok(ComponentKind::Capacitor { farad: value })
        }
        _ => Err(CircuitError::InvalidValue(
            "el campo no corresponde al tipo de componente",
        )),
    }
}

fn magnitude_db(transfer: Complex64) -> f64 {
    20.0 * transfer.norm().max(1e-15).log10()
}

fn solve_normal(jacobian: &[Vec<f64>], residual: &[f64], lambda: f64) -> Option<Vec<f64>> {
    let n = jacobian.first().map_or(0, Vec::len);
    if n == 0 {
        return None;
    }
    let mut normal = vec![vec![0.0_f64; n]; n];
    let mut gradient = vec![0.0_f64; n];
    for (row, entries) in jacobian.iter().enumerate() {
        for (i, entry_i) in entries.iter().enumerate() {
            gradient[i] += entry_i * residual[row];
            for (j, entry_j) in entries.iter().enumerate() {
                normal[i][j] += entry_i * entry_j;
            }
        }
    }
    for (i, row) in normal.iter_mut().enumerate() {
        row[i] = row[i] * (1.0 + lambda) + 1e-12;
    }
    let mut matrix = normal;
    let mut rhs: Vec<f64> = gradient.iter().map(|value| -value).collect();
    gaussian_solve(&mut matrix, &mut rhs)
}

fn gaussian_solve(matrix: &mut [Vec<f64>], rhs: &mut [f64]) -> Option<Vec<f64>> {
    let n = rhs.len();
    for col in 0..n {
        let mut pivot = col;
        let mut best = matrix[col][col].abs();
        for (row, entries) in matrix.iter().enumerate().skip(col + 1) {
            if entries[col].abs() > best {
                best = entries[col].abs();
                pivot = row;
            }
        }
        if best == 0.0 || !best.is_finite() {
            return None;
        }
        if pivot != col {
            matrix.swap(pivot, col);
            rhs.swap(pivot, col);
        }
        let divisor = matrix[col][col];
        for row in col + 1..n {
            let factor = matrix[row][col] / divisor;
            if factor == 0.0 {
                continue;
            }
            let (head, tail) = matrix.split_at_mut(row);
            let pivot_row = &head[col];
            let target = &mut tail[0];
            for (k, value) in target.iter_mut().enumerate().skip(col) {
                *value -= factor * pivot_row[k];
            }
            rhs[row] -= factor * rhs[col];
        }
    }
    let mut solution = vec![0.0_f64; n];
    for row in (0..n).rev() {
        let mut sum = rhs[row];
        for (col, value) in solution.iter().enumerate().skip(row + 1) {
            sum -= matrix[row][col] * value;
        }
        solution[row] = sum / matrix[row][row];
        if !solution[row].is_finite() {
            return None;
        }
    }
    Some(solution)
}

fn add(a: &[f64], b: &[f64]) -> Vec<f64> {
    a.iter().zip(b).map(|(x, y)| x + y).collect()
}

fn clamp(values: &[f64], bounds: &[(f64, f64)]) -> Vec<f64> {
    values
        .iter()
        .zip(bounds)
        .map(|(value, (low, high))| value.clamp(*low, *high))
        .collect()
}

fn max_abs(values: &[f64]) -> f64 {
    values
        .iter()
        .fold(0.0_f64, |acc, value| acc.max(value.abs()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crossover::{synthesize, FilterFamily, FilterKind, FilterSpec};
    use std::f64::consts::TAU;

    fn ladder_at(cutoff_hz: f64) -> (Circuit, Vec<DesignVariable>, OptimizationTarget) {
        let spec = FilterSpec {
            family: FilterFamily::Butterworth,
            kind: FilterKind::LowPass,
            order: 2,
            cutoff_hz: 1000.0,
            impedance_ohm: 8.0,
        };
        let reference = synthesize(&spec).unwrap();
        let mut circuit = reference.circuit.clone();
        // Perturba la red: el objetivo es la respuesta correcta.
        circuit
            .replace_component(
                1,
                ComponentKind::Inductor {
                    henry: 1.2 * (2.0 * 8.0 / (TAU * cutoff_hz)) * 0.5,
                    dcr_ohm: 0.0,
                },
            )
            .unwrap();
        let target = OptimizationTarget::new(
            (0..40)
                .map(|index| 100.0 * 10.0_f64.powf(index as f64 / 39.0))
                .collect(),
            (0..40)
                .map(|index| {
                    let freq = 100.0 * 10.0_f64.powf(index as f64 / 39.0);
                    let response = crate::crossover::FilterResponse::new(
                        FilterFamily::Butterworth,
                        FilterKind::LowPass,
                        2,
                        1000.0,
                    )
                    .unwrap();
                    // El divisor de la escalera entrega H/2: −6,0206 dB.
                    magnitude_db(response.transfer(freq).unwrap()) - 20.0 * 2.0_f64.log10()
                })
                .collect(),
            Vec::new(),
        )
        .unwrap();
        let variables = vec![
            DesignVariable {
                component: 1,
                field: VariableField::Inductance,
                min: 1e-6,
                max: 1.0,
            },
            DesignVariable {
                component: 2,
                field: VariableField::Capacitance,
                min: 1e-9,
                max: 1e-3,
            },
        ];
        (circuit, variables, target)
    }

    #[test]
    fn recovers_the_target_response() {
        let (circuit, variables, target) = ladder_at(1000.0);
        let report = optimize(
            &circuit,
            &variables,
            &target,
            NodeId(3),
            NodeId(1),
            &OptimizationOptions::default(),
        )
        .unwrap();
        assert!(report.final_cost < report.initial_cost);
        assert!(report.rmse_db < 0.05, "rmse={}", report.rmse_db);
        assert!(report.values[0] > 0.0 && report.values[1] > 0.0);
    }

    #[test]
    fn jacobian_is_stable_across_step_sizes() {
        let (circuit, variables, target) = ladder_at(1000.0);
        let evaluation = Evaluation {
            circuit: &circuit,
            variables: &variables,
            target: &target,
            output: NodeId(3),
            reference: NodeId(1),
        };
        let x: Vec<f64> = read_values(&circuit, &variables)
            .unwrap()
            .iter()
            .map(|value| value.ln())
            .collect();
        let fine = evaluation.jacobian(&x, 1e-5).unwrap();
        let coarse = evaluation.jacobian(&x, 1e-3).unwrap();
        for (row_fine, row_coarse) in fine.iter().zip(&coarse) {
            for (a, b) in row_fine.iter().zip(row_coarse) {
                assert!((a - b).abs() <= 1e-2 * a.abs().max(1.0), "{a} vs {b}");
            }
        }
    }

    #[test]
    fn zero_weights_and_weight_scaling_are_handled() {
        let (circuit, variables, target) = ladder_at(1000.0);
        assert!(OptimizationTarget::new(
            target.freq_hz.clone(),
            target.magnitude_db.clone(),
            vec![0.0; target.freq_hz.len()]
        )
        .is_err());

        let scaled = OptimizationTarget::new(
            target.freq_hz.clone(),
            target.magnitude_db.clone(),
            vec![0.25; target.freq_hz.len()],
        )
        .unwrap();
        let base = optimize(
            &circuit,
            &variables,
            &target,
            NodeId(3),
            NodeId(1),
            &OptimizationOptions::default(),
        )
        .unwrap();
        let weighted = optimize(
            &circuit,
            &variables,
            &scaled,
            NodeId(3),
            NodeId(1),
            &OptimizationOptions::default(),
        )
        .unwrap();
        assert!((base.rmse_db - weighted.rmse_db).abs() < 1e-9);
    }

    #[test]
    fn invalid_variables_are_rejected() {
        let (circuit, variables, target) = ladder_at(1000.0);
        let mut bad = variables.clone();
        bad[0].field = VariableField::Capacitance;
        assert!(optimize(
            &circuit,
            &bad,
            &target,
            NodeId(3),
            NodeId(1),
            &OptimizationOptions::default()
        )
        .is_err());
        assert!(optimize(
            &circuit,
            &[],
            &target,
            NodeId(3),
            NodeId(1),
            &OptimizationOptions::default()
        )
        .is_err());
        let mut out_of_range = variables;
        out_of_range[0].min = 0.0;
        assert!(optimize(
            &circuit,
            &out_of_range,
            &target,
            NodeId(3),
            NodeId(1),
            &OptimizationOptions::default()
        )
        .is_err());
    }
}
