//! Errores del análisis de redes eléctricas.
//!
//! Todas las funciones públicas que pueden fallar devuelven
//! [`CircuitError`]. No se usa `panic!`, `unwrap` ni `expect` en rutas de
//! entrada: una malla mal formada, una frecuencia inválida o un sistema
//! singular se informan con un valor tipado.

use crate::model::NodeId;
use std::fmt;

/// Resultado de una operación sobre una red.
pub type CircuitResult<T> = Result<T, CircuitError>;

/// Error de construcción, resolución o serialización de una red.
#[derive(Clone, Debug, PartialEq)]
pub enum CircuitError {
    /// La frecuencia no es finita o no es estrictamente positiva.
    InvalidFrequency(f64),
    /// El circuito no declara componentes.
    EmptyCircuit,
    /// El circuito no declara ninguna fuente de tensión.
    NoSource,
    /// Un componente o fuente referencia un nodo fuera de rango.
    UnknownNode { node: NodeId, nodes: usize },
    /// Un componente o fuente conecta un nodo consigo mismo.
    SelfLoop { node: NodeId },
    /// Un componente tiene un valor no finito o fuera de su contrato.
    InvalidComponent { index: usize, field: &'static str },
    /// Una fuente tiene una tensión no finita.
    InvalidSource { index: usize, field: &'static str },
    /// Un componente no se puede evaluar a esta frecuencia.
    UnusableComponent { index: usize, freq_hz: f64 },
    /// Una curva compleja necesita al menos dos puntos.
    CurveTooShort,
    /// Las frecuencias de una curva no son finitas o no crecen.
    CurveNotIncreasing { index: usize },
    /// Se pidió una curva fuera de su banda y la extrapolación está prohibida.
    CurveOutOfRange { freq_hz: f64 },
    /// El sistema lineal es singular a esta frecuencia.
    Singular { freq_hz: f64 },
    /// El sistema se resolvió, pero el residual relativo supera el límite.
    Inaccurate {
        freq_hz: f64,
        relative_residual: f64,
    },
    /// El resultado contiene valores no finitos.
    NonFiniteResult { freq_hz: f64 },
    /// La operación exige exactamente una fuente y encontró otra cantidad.
    SourceCount { found: usize },
    /// El nodo de referencia de una transferencia vale cero.
    ZeroReference { freq_hz: f64, node: NodeId },
    /// Un valor no se puede representar en el formato de netlist.
    NotSerializable { index: usize, reason: &'static str },
    /// La operación pedida excede una capacidad declarada.
    Unsupported(&'static str),
}

impl fmt::Display for CircuitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CircuitError::InvalidFrequency(freq) => {
                write!(f, "frecuencia inválida: {freq} Hz (debe ser finita y > 0)")
            }
            CircuitError::EmptyCircuit => write!(f, "el circuito no tiene componentes"),
            CircuitError::NoSource => write!(f, "el circuito no tiene ninguna fuente"),
            CircuitError::UnknownNode { node, nodes } => write!(
                f,
                "nodo {node:?} fuera de rango: el circuito declara {nodes} nodos"
            ),
            CircuitError::SelfLoop { node } => {
                write!(f, "componente conectado a sí mismo en {node:?}")
            }
            CircuitError::InvalidComponent { index, field } => {
                write!(f, "componente {index}: {field}")
            }
            CircuitError::InvalidSource { index, field } => {
                write!(f, "fuente {index}: {field}")
            }
            CircuitError::UnusableComponent { index, freq_hz } => write!(
                f,
                "componente {index} no evaluable a {freq_hz} Hz (impedancia nula o no finita)"
            ),
            CircuitError::CurveTooShort => {
                write!(f, "una curva compleja necesita al menos dos puntos")
            }
            CircuitError::CurveNotIncreasing { index } => write!(
                f,
                "la frecuencia del punto {index} no es finita o no crece respecto al anterior"
            ),
            CircuitError::CurveOutOfRange { freq_hz } => write!(
                f,
                "frecuencia {freq_hz} Hz fuera de la banda de la curva; no se extrapola"
            ),
            CircuitError::Singular { freq_hz } => {
                write!(f, "sistema singular a {freq_hz} Hz")
            }
            CircuitError::Inaccurate {
                freq_hz,
                relative_residual,
            } => write!(
                f,
                "residual relativo {relative_residual:.3e} a {freq_hz} Hz supera el límite"
            ),
            CircuitError::NonFiniteResult { freq_hz } => {
                write!(f, "resultado no finito a {freq_hz} Hz")
            }
            CircuitError::SourceCount { found } => {
                write!(f, "la operación exige exactamente una fuente y hay {found}")
            }
            CircuitError::ZeroReference { freq_hz, node } => {
                write!(f, "la referencia {node:?} vale cero a {freq_hz} Hz")
            }
            CircuitError::NotSerializable { index, reason } => {
                write!(f, "componente {index} no serializable: {reason}")
            }
            CircuitError::Unsupported(reason) => {
                write!(f, "operación no soportada: {reason}")
            }
        }
    }
}

impl std::error::Error for CircuitError {}
