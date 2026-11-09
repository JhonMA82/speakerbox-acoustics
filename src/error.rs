//! Errores de resolución acústica.
//!
//! Las funciones `response_at` son infalibles por compatibilidad: devuelven un
//! punto neutro cuando la entrada no es resoluble. Para consumidores que
//! necesitan distinguir una entrada inválida de un resultado cero, las
//! funciones `try_response_at` devuelven [`SolveError`].
//!
//! La validación distingue tres capas:
//!
//! - [`SolveError::InvalidFrequency`]: frecuencia no finita o no positiva.
//! - [`SolveError::InvalidParameter`]: un parámetro de entrada no es finito o
//!   no cumple una restricción física.
//! - [`SolveError::InvalidResponse`]: un campo del resultado no es finito o no
//!   cumple su restricción (por ejemplo, `ze_mag <= 0`).
//!
//! [`SolveError::InvalidTopology`] queda para configuraciones estructuralmente
//! incoherentes (tomas fuera de rango, BP6 sin puerto trasero, etc.).

use std::fmt;

/// Error de una consulta de respuesta acústica.
#[derive(Clone, Debug, PartialEq)]
pub enum SolveError {
    /// La frecuencia no es finita o no es positiva.
    InvalidFrequency(f64),
    /// La topología no es resoluble con la configuración actual.
    InvalidTopology(&'static str),
    /// Un parámetro de entrada no es finito o no cumple su restricción física.
    InvalidParameter(&'static str),
    /// Un campo de la respuesta no es finito o no cumple su restricción física.
    InvalidResponse(&'static str),
    /// El sistema lineal resultó singular o mal condicionado.
    SingularSystem,
    /// La respuesta contiene valores no finitos.
    ///
    /// Se conserva por compatibilidad; la validación actual informa del campo
    /// concreto mediante [`SolveError::InvalidResponse`].
    NonFiniteResponse,
}

impl fmt::Display for SolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolveError::InvalidFrequency(freq) => {
                write!(f, "frecuencia inválida: {freq} Hz (debe ser finita y > 0)")
            }
            SolveError::InvalidTopology(reason) => {
                write!(f, "topología inválida: {reason}")
            }
            SolveError::InvalidParameter(field) => {
                write!(f, "parámetro inválido: {field}")
            }
            SolveError::InvalidResponse(field) => {
                write!(f, "respuesta inválida: {field}")
            }
            SolveError::SingularSystem => write!(f, "sistema acústico singular"),
            SolveError::NonFiniteResponse => {
                write!(f, "la respuesta contiene valores no finitos")
            }
        }
    }
}

impl std::error::Error for SolveError {}

/// Comprueba que una frecuencia sea finita y estrictamente positiva.
#[inline]
pub(crate) fn check_frequency(freq_hz: f64) -> Result<(), SolveError> {
    if freq_hz.is_finite() && freq_hz > 0.0 {
        Ok(())
    } else {
        Err(SolveError::InvalidFrequency(freq_hz))
    }
}

/// Comprueba que un parámetro de entrada sea finito.
#[inline]
pub(crate) fn require_finite(value: f64, field: &'static str) -> Result<(), SolveError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(SolveError::InvalidParameter(field))
    }
}

/// Comprueba que un parámetro de entrada sea finito y estrictamente positivo.
#[inline]
pub(crate) fn require_positive(value: f64, field: &'static str) -> Result<(), SolveError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(SolveError::InvalidParameter(field))
    }
}

/// Comprueba que un parámetro de entrada sea finito y no negativo.
#[inline]
pub(crate) fn require_non_negative(value: f64, field: &'static str) -> Result<(), SolveError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(SolveError::InvalidParameter(field))
    }
}

/// Comprueba que una polaridad sea exactamente `+1` o `-1`.
#[inline]
pub(crate) fn require_polarity(value: f64, field: &'static str) -> Result<(), SolveError> {
    if value == 1.0 || value == -1.0 {
        Ok(())
    } else {
        Err(SolveError::InvalidParameter(field))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_check_rejects_non_finite_and_non_positive() {
        for f in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(matches!(
                check_frequency(f),
                Err(SolveError::InvalidFrequency(_))
            ));
        }
        assert!(check_frequency(20.0).is_ok());
    }

    #[test]
    fn parameter_checks_distinguish_zero_and_negative() {
        assert!(require_positive(0.0, "x").is_err());
        assert!(require_positive(-1.0, "x").is_err());
        assert!(require_non_negative(0.0, "x").is_ok());
        assert!(require_non_negative(-1.0, "x").is_err());
        assert!(require_finite(0.0, "x").is_ok());
        assert!(require_finite(f64::NAN, "x").is_err());
        assert!(require_polarity(-1.0, "p").is_ok());
        assert!(require_polarity(0.5, "p").is_err());
    }

    #[test]
    fn display_names_the_field() {
        let err = SolveError::InvalidResponse("ze_mag");
        assert!(err.to_string().contains("ze_mag"));
    }
}
