//! Errores de lectura de curvas e identificación.

use std::fmt;

/// Resultado de una operación de medida.
pub type MeasureResult<T> = Result<T, MeasureError>;

/// Error de identificación o de lectura de una curva.
#[derive(Clone, Debug, PartialEq)]
pub enum MeasureError {
    /// La frecuencia no es finita o no es estrictamente positiva.
    InvalidFrequency(f64),
    /// Un parámetro no es finito o no cumple su restricción física.
    InvalidParameter(&'static str),
    /// La curva tiene menos puntos de los exigidos.
    CurveTooShort,
    /// Las frecuencias no crecen estrictamente.
    CurveNotIncreasing { index: usize },
    /// El experimento físico es incoherente (masa añadida, caja, frecuencias).
    InvalidExperiment(&'static str),
    /// No hay datos suficientes para estimar lo que se pide.
    NoData,
    /// El ajuste no pudo resolver el sistema normal.
    SingularFit,
    /// Un valor intermedio no es finito.
    NonFinite(&'static str),
}

impl fmt::Display for MeasureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MeasureError::InvalidFrequency(freq) => {
                write!(f, "frecuencia inválida: {freq} Hz (debe ser finita y > 0)")
            }
            MeasureError::InvalidParameter(field) => {
                write!(f, "parámetro inválido: {field}")
            }
            MeasureError::CurveTooShort => {
                write!(f, "la curva necesita al menos cinco puntos para ajustar")
            }
            MeasureError::CurveNotIncreasing { index } => {
                write!(f, "la frecuencia del punto {index} no crece")
            }
            MeasureError::InvalidExperiment(reason) => {
                write!(f, "experimento inválido: {reason}")
            }
            MeasureError::NoData => write!(f, "no hay datos para estimar"),
            MeasureError::SingularFit => write!(f, "sistema normal singular durante el ajuste"),
            MeasureError::NonFinite(field) => write!(f, "valor no finito: {field}"),
        }
    }
}

impl std::error::Error for MeasureError {}

/// Error de una línea de fichero FRD o ZMA.
#[derive(Clone, Debug, PartialEq)]
pub struct CurveParseError {
    /// Línea 1-based.
    pub line: usize,
    /// Descripción del problema.
    pub message: String,
}

impl fmt::Display for CurveParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "línea {}: {}", self.line, self.message)
    }
}

impl std::error::Error for CurveParseError {}
