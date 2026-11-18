//! Ecualización analógica de segundo orden, con transferencia compleja.
use crate::error::{CircuitError, CircuitResult};
use num_complex::Complex64 as C;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EqualizerKind {
    Peaking,
    LowShelf,
    HighShelf,
}

/// Frecuencia central (pico) o de transición (shelf); Q adimensional.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnalogEqualizer {
    pub kind: EqualizerKind,
    pub frequency_hz: f64,
    pub quality_factor: f64,
    pub gain_db: f64,
}

impl AnalogEqualizer {
    pub fn response(self, frequency_hz: f64) -> CircuitResult<C> {
        if !frequency_hz.is_finite() || frequency_hz <= 0.0 {
            return Err(CircuitError::InvalidFrequency(frequency_hz));
        }
        let a = 10.0_f64.powf(self.gain_db / 40.0);
        let ratio = frequency_hz / self.frequency_hz;
        if !self.frequency_hz.is_finite()
            || self.frequency_hz <= 0.0
            || !self.quality_factor.is_finite()
            || self.quality_factor <= 0.0
            || !a.is_finite()
            || a <= 0.0
            || !ratio.is_finite()
        {
            return Err(CircuitError::Unsupported(
                "parámetros de ecualización analógica inválidos",
            ));
        }
        // Polinomios en s amortiguados por Q. Normalizar por ratio²
        // en la rama alta evita desbordar el término cuadrático.
        let scale = ratio.max(1.0);
        let constant = (1.0 / scale).powi(2);
        let linear = C::new(0.0, (ratio / scale) / scale / self.quality_factor);
        let quadratic = -(ratio / scale).powi(2);
        let root = a.sqrt();
        let h = match self.kind {
            EqualizerKind::Peaking => {
                (quadratic + linear * a + constant) / (quadratic + linear / a + constant)
            }
            EqualizerKind::LowShelf => {
                a * (quadratic + linear * root + a * constant)
                    / (a * quadratic + linear * root + constant)
            }
            EqualizerKind::HighShelf => {
                a * (a * quadratic + linear * root + constant)
                    / (quadratic + linear * root + a * constant)
            }
        };
        if !h.re.is_finite() || !h.im.is_finite() {
            return Err(CircuitError::NonFiniteResult {
                freq_hz: frequency_hz,
            });
        }
        Ok(h)
    }
}

/// Producto de transferencias de etapas ideales aisladas, en orden de entrada.
/// Un circuito pasivo cargado debe evaluarse con el MNA de la crate.
pub fn equalizer_response(sections: &[AnalogEqualizer], frequency_hz: f64) -> CircuitResult<C> {
    if !frequency_hz.is_finite() || frequency_hz <= 0.0 {
        return Err(CircuitError::InvalidFrequency(frequency_hz));
    }
    let h = sections.iter().try_fold(C::from(1.0), |acc, &section| {
        section.response(frequency_hz).map(|x| acc * x)
    })?;
    if !h.re.is_finite() || !h.im.is_finite() {
        return Err(CircuitError::NonFiniteResult {
            freq_hz: frequency_hz,
        });
    }
    Ok(h)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn analog_eq_has_expected_gain_and_phase() {
        let g = 20.0_f64.log10() * 20.0;
        let section = |kind| AnalogEqualizer {
            kind,
            frequency_hz: 100.0,
            quality_factor: 0.707,
            gain_db: g,
        };
        let peak = section(EqualizerKind::Peaking);
        let low = section(EqualizerKind::LowShelf);
        let high = section(EqualizerKind::HighShelf);
        let check = |v: C, expected: f64| assert!((v - expected).norm() < 1e-6, "{v:?}");
        check(peak.response(100.0).unwrap(), 20.0);
        check(low.response(1e-8).unwrap(), 20.0);
        check(low.response(1e12).unwrap(), 1.0);
        check(high.response(1e-8).unwrap(), 1.0);
        check(high.response(1e12).unwrap(), 20.0);
        assert_ne!(low.response(75.0).unwrap().arg(), 0.0);
        let inverse = AnalogEqualizer {
            gain_db: -g,
            ..peak
        };
        check(equalizer_response(&[peak, inverse], 75.0).unwrap(), 1.0);
        assert!(equalizer_response(&[], 0.0).is_err());
    }
}
