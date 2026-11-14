//! Curvas complejas de impedancia o transferencia.
//!
//! Una [`ComplexCurve`] guarda pares frecuencia-valor complejo ordenados. La
//! interpolación es lineal sobre `log10(f)` en la parte real y en la
//! imaginaria por separado: es la convención habitual de las curvas de
//! altavoz, que se muestrean de forma logarítmica.
//!
//! La extrapolación está **prohibida**. Consultar fuera de la banda devuelve
//! [`CircuitError::CurveOutOfRange`]: una impedancia inventada fuera de la
//! medición es peor que una ausencia declarada.

use crate::error::{CircuitError, CircuitResult};
use num_complex::Complex64;

/// Curva de valores complejos indexada por frecuencia en hercios.
#[derive(Clone, Debug, PartialEq)]
pub struct ComplexCurve {
    freq_hz: Vec<f64>,
    values: Vec<Complex64>,
}

impl ComplexCurve {
    /// Construye una curva validada.
    ///
    /// # Errores
    ///
    /// - [`CircuitError::CurveTooShort`] con menos de dos puntos.
    /// - [`CircuitError::CurveNotIncreasing`] si una frecuencia no es finita,
    ///   no es positiva o no crece estrictamente.
    /// - [`CircuitError::InvalidComponent`] si un valor no es finito.
    pub fn new(freq_hz: Vec<f64>, values: Vec<Complex64>) -> CircuitResult<Self> {
        if freq_hz.len() < 2 || freq_hz.len() != values.len() {
            return Err(CircuitError::CurveTooShort);
        }
        for (index, freq) in freq_hz.iter().enumerate() {
            let monotonic =
                *freq > 0.0 && freq.is_finite() && (index == 0 || *freq > freq_hz[index - 1]);
            if !monotonic {
                return Err(CircuitError::CurveNotIncreasing { index });
            }
            let value = values[index];
            if !value.re.is_finite() || !value.im.is_finite() {
                return Err(CircuitError::InvalidComponent {
                    index,
                    field: "valor de curva no finito",
                });
            }
        }
        Ok(Self { freq_hz, values })
    }

    /// Frecuencias en hercios.
    #[must_use]
    pub fn freq_hz(&self) -> &[f64] {
        &self.freq_hz
    }

    /// Valores complejos en el mismo orden que [`ComplexCurve::freq_hz`].
    #[must_use]
    pub fn values(&self) -> &[Complex64] {
        &self.values
    }

    /// Número de puntos.
    #[must_use]
    pub fn len(&self) -> usize {
        self.freq_hz.len()
    }

    /// `true` si no hay puntos. Una curva válida nunca está vacía.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.freq_hz.is_empty()
    }

    /// Valor interpolado a `freq_hz`.
    ///
    /// En los puntos exactos devuelve el valor medido, sin interpolar.
    ///
    /// # Errores
    ///
    /// [`CircuitError::CurveOutOfRange`] fuera de la banda o con una
    /// frecuencia no finita.
    pub fn at(&self, freq_hz: f64) -> CircuitResult<Complex64> {
        if !freq_hz.is_finite() || freq_hz <= 0.0 {
            return Err(CircuitError::CurveOutOfRange { freq_hz });
        }
        let last = self.freq_hz.len() - 1;
        if freq_hz < self.freq_hz[0] || freq_hz > self.freq_hz[last] {
            return Err(CircuitError::CurveOutOfRange { freq_hz });
        }
        if freq_hz == self.freq_hz[0] {
            return Ok(self.values[0]);
        }
        if freq_hz == self.freq_hz[last] {
            return Ok(self.values[last]);
        }
        let upper = self
            .freq_hz
            .partition_point(|probe| *probe <= freq_hz)
            .min(last);
        let lower = upper - 1;
        let (f0, f1) = (self.freq_hz[lower], self.freq_hz[upper]);
        let (v0, v1) = (self.values[lower], self.values[upper]);
        if freq_hz == f1 {
            return Ok(v1);
        }
        let span = f1.log10() - f0.log10();
        if span == 0.0 {
            // Dos frecuencias distintas pueden compartir el mismo log10: la
            // banda es más estrecha que la resolución del interpolador. Se
            // devuelve el extremo más cercano en vez de 0/0.
            return Ok(if freq_hz <= f0 { v0 } else { v1 });
        }
        let t = (freq_hz.log10() - f0.log10()) / span;
        Ok(v0 + (v1 - v0) * t)
    }

    /// Magnitudes en decibelios re 1 (solo para presentación).
    #[must_use]
    pub fn magnitude_db(&self) -> Vec<f64> {
        self.values
            .iter()
            .map(|v| 20.0 * v.norm().log10())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn curve() -> ComplexCurve {
        ComplexCurve::new(
            vec![10.0, 100.0, 1000.0],
            vec![
                Complex64::new(1.0, 0.0),
                Complex64::new(10.0, 5.0),
                Complex64::new(100.0, 0.0),
            ],
        )
        .expect("curva válida")
    }

    #[test]
    fn exact_points_are_returned_uninterpolated() {
        let c = curve();
        assert_eq!(c.at(10.0).unwrap(), Complex64::new(1.0, 0.0));
        assert_eq!(c.at(100.0).unwrap(), Complex64::new(10.0, 5.0));
        assert_eq!(c.at(1000.0).unwrap(), Complex64::new(100.0, 0.0));
    }

    #[test]
    fn interpolation_is_linear_in_log_frequency() {
        let c = curve();
        // sqrt(10^1 * 10^2) = 10^1.5: punto medio geométrico.
        let mid = c.at(10f64.powf(1.5)).unwrap();
        assert!((mid.re - 5.5).abs() < 1e-12);
        assert!((mid.im - 2.5).abs() < 1e-12);
    }

    #[test]
    fn frequencies_with_the_same_log10_do_not_produce_nan() {
        let curve = ComplexCurve::new(
            vec![1000.0, 1000.0000000000003],
            vec![Complex64::new(8.0, 0.0), Complex64::new(9.0, 0.0)],
        )
        .unwrap();
        let value = curve.at(1000.0000000000001).unwrap();
        assert!(value.re.is_finite() && value.im.is_finite(), "{value:?}");
    }

    #[test]
    fn outside_band_never_extrapolates() {
        let c = curve();
        assert_eq!(
            c.at(9.999).unwrap_err(),
            CircuitError::CurveOutOfRange { freq_hz: 9.999 }
        );
        assert_eq!(
            c.at(1000.001).unwrap_err(),
            CircuitError::CurveOutOfRange { freq_hz: 1000.001 }
        );
        assert!(c.at(f64::NAN).is_err());
        assert!(c.at(-1.0).is_err());
    }

    #[test]
    fn rejects_short_non_monotonic_and_non_finite_curves() {
        assert_eq!(
            ComplexCurve::new(vec![10.0], vec![Complex64::new(1.0, 0.0)]).unwrap_err(),
            CircuitError::CurveTooShort
        );
        assert_eq!(
            ComplexCurve::new(
                vec![10.0, 10.0],
                vec![Complex64::new(1.0, 0.0), Complex64::new(1.0, 0.0)]
            )
            .unwrap_err(),
            CircuitError::CurveNotIncreasing { index: 1 }
        );
        assert!(ComplexCurve::new(
            vec![10.0, 0.0],
            vec![Complex64::new(1.0, 0.0), Complex64::new(1.0, 0.0)]
        )
        .is_err());
        assert!(ComplexCurve::new(
            vec![10.0, 100.0],
            vec![Complex64::new(f64::NAN, 0.0), Complex64::new(1.0, 0.0)]
        )
        .is_err());
    }

    #[test]
    fn magnitude_db_matches_definition() {
        let c = ComplexCurve::new(
            vec![10.0, 20.0],
            vec![Complex64::new(1.0, 0.0), Complex64::new(10.0, 0.0)],
        )
        .unwrap();
        assert_eq!(c.magnitude_db(), vec![0.0, 20.0]);
    }
}
