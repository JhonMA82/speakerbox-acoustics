//! Ajuste restringido del circuito equivalente a una curva de impedancia.
//!
//! Se ajustan los siete parámetros en **logaritmo**, de modo que la positividad
//! es estructural: `p = exp(x)`. El residual es complejo y relativo a la
//! impedancia medida:
//!
//! ```text
//! r_i = (Z_modelo(f_i) − Z_medida(f_i)) / |Z_medida(f_i)|
//! ```
//!
//! El residual complejo evita el salto artificial de ±180° que introduce un
//! residual de fase. El minimizador es Levenberg–Marquardt con Jacobiano por
//! diferencias centrales en el espacio logarítmico. No hay heurísticas de
//! división sin protección: un sistema normal singular se informa.

use crate::equiv::EquivalentImpedance;
use crate::error::{MeasureError, MeasureResult};
use num_complex::Complex64;
use speakerbox_circuits::ComplexCurve;
use std::f64::consts::{PI, TAU};

/// Límites físicos de cada parámetro, en el orden de
/// [`crate::equiv::PARAMETER_NAMES`].
const BOUNDS: [(f64, f64); 7] = [
    (1e-3, 1e6),
    (1e-6, 100.0),
    (1e-3, 1e6),
    (1e-9, 100.0),
    (1e-3, 1e6),
    (1e-6, 100.0),
    (1e-9, 1.0),
];

const MIN_POINTS: usize = 5;

/// Avisos del ajuste.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FitWarning {
    /// La curva medida no muestra un máximo interior: puede no haber
    /// resonancia o quedar fuera de la banda.
    NoResonance,
    /// El error relativo máximo supera el 20 %: el modelo no describe la curva.
    PoorFit,
    /// Un parámetro terminó pegado a su límite; no es identificable con estos
    /// datos.
    ParameterAtBound(&'static str),
}

/// Opciones del ajuste.
#[derive(Clone, Debug, PartialEq)]
pub struct FitOptions {
    /// Semilla. Sin valor, se usa [`initial_guess`].
    pub initial: Option<EquivalentImpedance>,
    /// Iteraciones máximas.
    pub max_iterations: usize,
    /// Tolerancia de mejora relativa del coste.
    pub tolerance: f64,
}

impl Default for FitOptions {
    fn default() -> Self {
        Self {
            initial: None,
            max_iterations: 200,
            tolerance: 1e-10,
        }
    }
}

/// Resultado completo del ajuste.
#[derive(Clone, Debug, PartialEq)]
pub struct FitReport {
    /// Parámetros ajustados.
    pub parameters: EquivalentImpedance,
    /// Semilla usada.
    pub initial: EquivalentImpedance,
    /// Iteraciones ejecutadas.
    pub iterations: usize,
    /// `true` si se alcanzó la tolerancia.
    pub converged: bool,
    /// Coste inicial (suma de cuadrados del residual).
    pub initial_cost: f64,
    /// Coste final.
    pub final_cost: f64,
    /// Error cuadrático medio del módulo, en ohmios.
    pub rmse_magnitude_ohm: f64,
    /// Error cuadrático medio circular de la fase, en grados.
    pub rmse_phase_deg: f64,
    /// Peor error relativo complejo.
    pub max_relative_error: f64,
    /// Avisos, sin duplicados.
    pub warnings: Vec<FitWarning>,
}

/// Ajusta el circuito equivalente a una curva de impedancia medida.
///
/// # Errores
///
/// - [`MeasureError::CurveTooShort`] con menos de cinco puntos.
/// - [`MeasureError::InvalidParameter`] si la curva contiene una impedancia
///   nula o no finita.
/// - [`MeasureError::SingularFit`] si el sistema normal no se puede resolver.
pub fn fit_equivalent(curve: &ComplexCurve, options: &FitOptions) -> MeasureResult<FitReport> {
    let (freqs, values) = prepare(curve)?;
    let initial = match options.initial {
        Some(model) => model,
        None => initial_guess(curve)?,
    };
    let mut x = log_bounds(initial.to_array());
    let mut cost = cost_of(&x, &freqs, &values);
    let initial_cost = cost;
    let mut lambda = 1e-3;
    let mut iterations = 0;
    let mut converged = false;

    while iterations < options.max_iterations {
        iterations += 1;
        let residual = match residual(&x, &freqs, &values) {
            Some(values) => values,
            None => break,
        };
        let jacobian = match jacobian(&x, &freqs, &values) {
            Some(jacobian) => jacobian,
            None => break,
        };
        let delta = match solve_normal(&jacobian, &residual, lambda) {
            Some(delta) => delta,
            None => return Err(MeasureError::SingularFit),
        };
        let candidate = clamp_bounds(&add(&x, &delta));
        let candidate_cost = cost_of(&candidate, &freqs, &values);
        if candidate_cost.is_finite() && candidate_cost < cost {
            let improvement = (cost - candidate_cost) / cost.max(f64::MIN_POSITIVE);
            x = candidate;
            cost = candidate_cost;
            lambda = (lambda / 3.0).max(1e-12);
            if improvement < options.tolerance || norm(&delta) < 1e-9 {
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

    let parameters = params_from_x(&x)?;
    let stats = statistics(&parameters, &freqs, &values);
    let mut warnings = Vec::new();
    if !has_interior_peak(&values) {
        warnings.push(FitWarning::NoResonance);
    }
    if stats.max_relative_error > 0.2 {
        warnings.push(FitWarning::PoorFit);
    }
    for (index, name) in crate::equiv::PARAMETER_NAMES.iter().enumerate() {
        let (low, high) = BOUNDS[index];
        if (x[index] - low.ln()).abs() < 1e-6 || (x[index] - high.ln()).abs() < 1e-6 {
            warnings.push(FitWarning::ParameterAtBound(name));
        }
    }

    Ok(FitReport {
        parameters,
        initial,
        iterations,
        converged,
        initial_cost,
        final_cost: cost,
        rmse_magnitude_ohm: stats.rmse_magnitude_ohm,
        rmse_phase_deg: stats.rmse_phase_deg,
        max_relative_error: stats.max_relative_error,
        warnings,
    })
}

/// Semilla heurística a partir de la propia curva.
///
/// Toma la resistencia de continua del mínimo medido, el pico como `Re + R2`
/// y supone el tanque resonante en la frecuencia del pico. Es solo un punto de
/// partida: el ajuste decide.
///
/// # Errores
///
/// [`MeasureError::CurveTooShort`] o [`MeasureError::InvalidParameter`].
pub fn initial_guess(curve: &ComplexCurve) -> MeasureResult<EquivalentImpedance> {
    let (freqs, values) = prepare(curve)?;
    let mut min_magnitude = f64::INFINITY;
    let mut peak_index = 0;
    let mut peak_magnitude = 0.0_f64;
    for (index, value) in values.iter().enumerate() {
        let magnitude = value.norm();
        min_magnitude = min_magnitude.min(magnitude);
        if magnitude > peak_magnitude {
            peak_magnitude = magnitude;
            peak_index = index;
        }
    }
    let re = (0.5 * min_magnitude).max(1e-3);
    let r2 = (peak_magnitude - re).max(1e-3);
    let fs = freqs[peak_index];
    let c2 = 1e-4;
    let l2 = 1.0 / ((TAU * fs).powi(2) * c2);
    EquivalentImpedance::new(re, 1e-3, re, 1e-6, r2, l2, c2)
}

struct Statistics {
    rmse_magnitude_ohm: f64,
    rmse_phase_deg: f64,
    max_relative_error: f64,
}

fn statistics(model: &EquivalentImpedance, freqs: &[f64], values: &[Complex64]) -> Statistics {
    let mut magnitude_sq = 0.0;
    let mut phase_sq = 0.0;
    let mut max_relative = 0.0_f64;
    for (freq, measured) in freqs.iter().zip(values) {
        let predicted = match model.impedance(*freq) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let magnitude_error = predicted.norm() - measured.norm();
        magnitude_sq += magnitude_error * magnitude_error;
        let phase_error = wrap(predicted.arg() - measured.arg());
        phase_sq += phase_error * phase_error;
        let relative = (predicted - measured).norm() / measured.norm();
        if relative > max_relative {
            max_relative = relative;
        }
    }
    let count = freqs.len() as f64;
    Statistics {
        rmse_magnitude_ohm: (magnitude_sq / count).sqrt(),
        rmse_phase_deg: (phase_sq / count).sqrt().to_degrees(),
        max_relative_error: max_relative,
    }
}

fn has_interior_peak(values: &[Complex64]) -> bool {
    if values.len() < 3 {
        return false;
    }
    (1..values.len() - 1).any(|index| {
        let magnitude = values[index].norm();
        magnitude > values[index - 1].norm() && magnitude >= values[index + 1].norm()
    })
}

fn wrap(angle: f64) -> f64 {
    (angle + PI).rem_euclid(TAU) - PI
}

fn prepare(curve: &ComplexCurve) -> MeasureResult<(Vec<f64>, Vec<Complex64>)> {
    if curve.len() < MIN_POINTS {
        return Err(MeasureError::CurveTooShort);
    }
    let mut freqs = Vec::with_capacity(curve.len());
    let mut values = Vec::with_capacity(curve.len());
    for (index, (freq, value)) in curve.freq_hz().iter().zip(curve.values()).enumerate() {
        if !freq.is_finite() || *freq <= 0.0 || (index > 0 && *freq <= freqs[index - 1]) {
            return Err(MeasureError::CurveNotIncreasing { index });
        }
        if value.norm() <= 0.0 || !value.re.is_finite() || !value.im.is_finite() {
            return Err(MeasureError::InvalidParameter(
                "la impedancia medida debe ser finita y no nula",
            ));
        }
        if value.re <= 0.0 {
            return Err(MeasureError::InvalidParameter(
                "la impedancia medida debe ser pasiva: Re(Z) > 0",
            ));
        }
        freqs.push(*freq);
        values.push(*value);
    }
    Ok((freqs, values))
}

fn log_bounds(values: [f64; 7]) -> [f64; 7] {
    let mut out = [0.0; 7];
    for (index, value) in values.iter().enumerate() {
        let (low, high) = BOUNDS[index];
        out[index] = value.clamp(low, high).ln();
    }
    out
}

fn clamp_bounds(x: &[f64; 7]) -> [f64; 7] {
    let mut out = *x;
    for index in 0..7 {
        let (low, high) = BOUNDS[index];
        out[index] = out[index].clamp(low.ln(), high.ln());
    }
    out
}

fn params_from_x(x: &[f64; 7]) -> MeasureResult<EquivalentImpedance> {
    let mut values = [0.0; 7];
    for (index, value) in x.iter().enumerate() {
        values[index] = value.exp();
        if !values[index].is_finite() {
            return Err(MeasureError::NonFinite("parámetro ajustado"));
        }
    }
    EquivalentImpedance::from_array(values)
}

fn residual(x: &[f64; 7], freqs: &[f64], values: &[Complex64]) -> Option<Vec<f64>> {
    let model = params_from_x(x).ok()?;
    let mut out = Vec::with_capacity(freqs.len() * 2);
    for (freq, measured) in freqs.iter().zip(values) {
        let predicted = model.impedance(*freq).ok()?;
        let error = (predicted - measured) / measured.norm();
        out.push(error.re);
        out.push(error.im);
    }
    Some(out)
}

fn cost_of(x: &[f64; 7], freqs: &[f64], values: &[Complex64]) -> f64 {
    match residual(x, freqs, values) {
        Some(residual) => residual.iter().map(|value| value * value).sum(),
        None => f64::INFINITY,
    }
}

/// Jacobiano del residual respecto de los log-parámetros, por diferencias
/// centrales.
fn jacobian(x: &[f64; 7], freqs: &[f64], values: &[Complex64]) -> Option<Vec<[f64; 7]>> {
    const STEP: f64 = 1e-5;
    let rows = freqs.len() * 2;
    let mut columns = vec![vec![0.0_f64; rows]; 7];
    for (index, column) in columns.iter_mut().enumerate() {
        let mut plus = *x;
        let mut minus = *x;
        plus[index] += STEP;
        minus[index] -= STEP;
        let plus = clamp_bounds(&plus);
        let minus = clamp_bounds(&minus);
        let span = plus[index] - minus[index];
        if span == 0.0 {
            return None;
        }
        let up = residual(&plus, freqs, values)?;
        let down = residual(&minus, freqs, values)?;
        for row in 0..rows {
            column[row] = (up[row] - down[row]) / span;
        }
    }
    let mut matrix = vec![[0.0; 7]; rows];
    for (row, entries) in matrix.iter_mut().enumerate() {
        for (index, entry) in entries.iter_mut().enumerate() {
            *entry = columns[index][row];
        }
    }
    Some(matrix)
}

fn solve_normal(jacobian: &[[f64; 7]], residual: &[f64], lambda: f64) -> Option<[f64; 7]> {
    let mut normal = [[0.0_f64; 7]; 7];
    let mut gradient = [0.0_f64; 7];
    for (row, entries) in jacobian.iter().enumerate() {
        for (i, entry_i) in entries.iter().enumerate() {
            gradient[i] += entry_i * residual[row];
            let normal_row = &mut normal[i];
            for (j, entry_j) in entries.iter().enumerate() {
                normal_row[j] += entry_i * entry_j;
            }
        }
    }
    for (i, normal_row) in normal.iter_mut().enumerate() {
        normal_row[i] = normal_row[i] * (1.0 + lambda) + 1e-12;
    }
    let mut matrix = normal;
    let mut rhs = [0.0_f64; 7];
    for i in 0..7 {
        rhs[i] = -gradient[i];
    }
    let solution = gaussian_solve(&mut matrix, &mut rhs)?;
    Some(solution)
}

/// Eliminación gaussiana con pivoteo parcial para el sistema normal 7×7.
fn gaussian_solve(matrix: &mut [[f64; 7]; 7], rhs: &mut [f64; 7]) -> Option<[f64; 7]> {
    for col in 0..7 {
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
        for row in col + 1..7 {
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
    let mut solution = [0.0_f64; 7];
    for row in (0..7).rev() {
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

fn add(a: &[f64; 7], b: &[f64; 7]) -> [f64; 7] {
    let mut out = [0.0; 7];
    for index in 0..7 {
        out[index] = a[index] + b[index];
    }
    out
}

fn norm(values: &[f64; 7]) -> f64 {
    values
        .iter()
        .fold(0.0_f64, |acc, value| acc.max(value.abs()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known() -> EquivalentImpedance {
        EquivalentImpedance::new(
            8.0,
            0.02,
            6.4,
            0.0015,
            45.0,
            0.09,
            1.0 / ((TAU * 40.0_f64).powi(2) * 0.09),
        )
        .unwrap()
    }

    fn synthetic(model: &EquivalentImpedance, points: usize) -> ComplexCurve {
        let mut freqs = Vec::with_capacity(points);
        let mut values = Vec::with_capacity(points);
        for index in 0..points {
            let t = index as f64 / (points - 1) as f64;
            let freq = 5.0 * 4000.0_f64.powf(t);
            freqs.push(freq);
            values.push(model.impedance(freq).unwrap());
        }
        ComplexCurve::new(freqs, values).unwrap()
    }

    fn perturbed(base: &EquivalentImpedance, factor: f64) -> EquivalentImpedance {
        let values = base.to_array();
        let mut out = [0.0; 7];
        for (index, value) in values.iter().enumerate() {
            let sign = if index % 2 == 0 { 1.0 } else { -1.0 };
            out[index] = value * (1.0 + sign * (factor - 1.0));
        }
        EquivalentImpedance::from_array(out).unwrap()
    }

    #[test]
    fn recovers_known_parameters_from_exact_data() {
        let model = known();
        let curve = synthetic(&model, 80);
        let options = FitOptions {
            initial: Some(perturbed(&model, 1.3)),
            ..FitOptions::default()
        };
        let report = fit_equivalent(&curve, &options).unwrap();
        let expected = model.to_array();
        let found = report.parameters.to_array();
        for (index, (want, got)) in expected.iter().zip(found).enumerate() {
            let error = (got - want).abs() / want;
            assert!(
                error < 0.05,
                "{}: {got} vs {want} (rel {error})",
                crate::equiv::PARAMETER_NAMES[index]
            );
        }
        assert!(
            report.converged,
            "no convergió en {} iteraciones",
            report.iterations
        );
        assert!(
            report.max_relative_error < 1e-6,
            "{}",
            report.max_relative_error
        );
        assert!(report.final_cost < report.initial_cost);
    }

    #[test]
    fn crude_initial_guess_still_reduces_the_cost() {
        let model = known();
        let curve = synthetic(&model, 40);
        let report = fit_equivalent(&curve, &FitOptions::default()).unwrap();
        assert!(report.final_cost < report.initial_cost);
        assert!(report.parameters.re_ohm() > 0.0);
    }

    #[test]
    fn noisy_data_reports_a_finite_fit() {
        let model = known();
        let clean = synthetic(&model, 60);
        let mut seed = 12345_u64;
        let noisy = ComplexCurve::new(
            clean.freq_hz().to_vec(),
            clean
                .values()
                .iter()
                .map(|value| {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                    let unit = ((seed >> 33) as f64 / (1u64 << 31) as f64) - 1.0;
                    value * (1.0 + 0.02 * unit)
                })
                .collect(),
        )
        .unwrap();
        let report = fit_equivalent(
            &noisy,
            &FitOptions {
                initial: Some(perturbed(&model, 1.2)),
                ..FitOptions::default()
            },
        )
        .unwrap();
        assert!(report.final_cost.is_finite());
        assert!(report.rmse_magnitude_ohm < 1.0);
        assert!(report.max_relative_error < 0.1);
    }

    #[test]
    fn flat_curve_warns_about_missing_resonance() {
        let freqs: Vec<f64> = (0..20).map(|i| 10.0 * 1.2_f64.powi(i)).collect();
        let values = vec![Complex64::new(8.0, 0.0); freqs.len()];
        let curve = ComplexCurve::new(freqs, values).unwrap();
        let report = fit_equivalent(&curve, &FitOptions::default()).unwrap();
        assert!(report.warnings.contains(&FitWarning::NoResonance));
    }

    #[test]
    fn deterministic_for_the_same_input() {
        let model = known();
        let curve = synthetic(&model, 50);
        let options = FitOptions {
            initial: Some(perturbed(&model, 1.1)),
            ..FitOptions::default()
        };
        assert_eq!(
            fit_equivalent(&curve, &options).unwrap(),
            fit_equivalent(&curve, &options).unwrap()
        );
    }

    #[test]
    fn non_passive_curves_are_rejected() {
        let curve = ComplexCurve::new(
            vec![100.0, 200.0, 400.0, 800.0, 1600.0],
            vec![
                Complex64::new(-8.0, 1.0),
                Complex64::new(8.0, 1.0),
                Complex64::new(8.0, 1.0),
                Complex64::new(8.0, 1.0),
                Complex64::new(8.0, 1.0),
            ],
        )
        .unwrap();
        assert_eq!(
            fit_equivalent(&curve, &FitOptions::default()).unwrap_err(),
            MeasureError::InvalidParameter("la impedancia medida debe ser pasiva: Re(Z) > 0")
        );
    }

    #[test]
    fn short_or_zero_curves_are_errors() {
        let short =
            ComplexCurve::new(vec![10.0, 20.0, 30.0], vec![Complex64::new(8.0, 0.0); 3]).unwrap();
        assert_eq!(
            fit_equivalent(&short, &FitOptions::default()).unwrap_err(),
            MeasureError::CurveTooShort
        );
        let zero = ComplexCurve::new(
            vec![10.0, 20.0, 30.0, 40.0, 50.0],
            vec![Complex64::new(0.0, 0.0); 5],
        )
        .unwrap();
        assert!(fit_equivalent(&zero, &FitOptions::default()).is_err());
    }
}
