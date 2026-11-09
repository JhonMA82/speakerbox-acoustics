//! Errores de resolución acústica.
//!
//! Las funciones `response_at` son infalibles por compatibilidad: devuelven un
//! punto neutro cuando la entrada no es resoluble. Para consumidores que
//! necesitan distinguir una entrada inválida de un resultado cero, las
//! funciones `try_response_at` devuelven [`SolveError`].

use std::fmt;

/// Error de una consulta de respuesta acústica.
#[derive(Clone, Debug, PartialEq)]
pub enum SolveError {
    /// La frecuencia no es finita o no es positiva.
    InvalidFrequency(f64),
    /// La topología no es resoluble con los parámetros actuales.
    InvalidTopology(&'static str),
    /// El sistema lineal resultó singular o mal condicionado.
    SingularSystem,
    /// La respuesta contiene valores no finitos.
    NonFiniteResponse,
}

impl fmt::Display for SolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolveError::InvalidFrequency(freq) => {
                write!(f, "frecuencia inválida: {freq} Hz")
            }
            SolveError::InvalidTopology(reason) => {
                write!(f, "topología inválida: {reason}")
            }
            SolveError::SingularSystem => write!(f, "sistema acústico singular"),
            SolveError::NonFiniteResponse => {
                write!(f, "la respuesta contiene valores no finitos")
            }
        }
    }
}

impl std::error::Error for SolveError {}
