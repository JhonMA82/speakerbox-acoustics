//! # speakerbox-circuits
//!
//! Análisis de redes eléctricas pasivas para filtros de altavoces.
//!
//! Este crate es una **reimplementación independiente** a partir de las
//! ecuaciones públicas del análisis nodal modificado (MNA) y de la definición
//! de impedancia de cada componente. No contiene código traducido de Speaker
//! Workshop ni de ninguna otra aplicación: las fórmulas proceden de la teoría
//! de circuitos y los oráculos son analíticos o propios.
//!
//! ## Qué hace
//!
//! - Modela nodos, resistencias, inductancias con resistencia serie,
//!   condensadores, cargas de impedancia constante o medida, y fuentes de
//!   tensión ideales.
//! - Resuelve la red a una frecuencia con MNA, pivoteo parcial y verificación
//!   del residual.
//! - Expone tensión por nodo, transferencia compleja entre nodos, impedancia
//!   de entrada y barridos en frecuencia.
//! - Lee y escribe netlists `.CIR` con sufijos SI.
//! - Sintetiza escaleras Butterworth LP/HP de orden 1–4, L-pad y Zobel.
//! - Optimiza valores de componentes contra una magnitud objetivo en dB.
//!
//! ## Qué no hace
//!
//! - No modela la respuesta acústica. Una transferencia de tensión no es SPL,
//!   ni fase de presión: esa referencia sigue siendo un contrato pendiente del
//!   proyecto acústico.
//! - No sintetiza pasivamente familias distintas de Butterworth.
//! - No modela sistemas multivía: falta la referencia de presión compleja.
//! - No importa `.SWD` ni datos de medición en bruto.
//!
//! ## Ejemplo
//!
//! ```
//! use speakerbox_circuits::{Circuit, Component, NodeId, VoltageSource};
//! use num_complex::Complex64;
//! use std::f64::consts::TAU;
//!
//! let mut circuit = Circuit::new(4);
//! circuit
//!     .push_source(VoltageSource::new(NodeId(1), NodeId::GROUND, Complex64::new(1.0, 0.0)))
//!     .unwrap();
//! circuit
//!     .push_component(Component::resistor(NodeId(1), NodeId(2), 10.0))
//!     .unwrap();
//! circuit
//!     .push_component(Component::inductor(NodeId(2), NodeId(3), 1e-3, 0.0))
//!     .unwrap();
//! circuit
//!     .push_component(Component::capacitor(NodeId(3), NodeId::GROUND, 5e-6))
//!     .unwrap();
//!
//! let f0 = 1.0 / (TAU * (1e-3_f64 * 5e-6).sqrt());
//! let z_in = circuit.input_impedance(f0).unwrap();
//! assert!((z_in.re - 10.0).abs() < 1e-9);
//! assert!(z_in.im.abs() < 1e-9);
//! ```
//!
//! ## Unidades
//!
//! Todas las magnitudes son SI: ohmios, henrios, faradios, voltios, hercios y
//! siemens. El nodo [`NodeId::GROUND`] es siempre cero.
//!
//! ## Compatibilidad
//!
//! Rust 1.75 y `wasm32-unknown-unknown`. La única dependencia de cálculo es
//! [`num_complex`].

pub mod analysis;
pub mod cir;
pub mod crossover;
pub mod curves;
pub mod error;
pub mod model;
pub mod optimize;
pub mod solver;

pub use cir::{parse as parse_cir, parse_si_value, to_cir, CirError};
pub use crossover::{
    l_pad, synthesize, zobel, FilterFamily, FilterKind, FilterResponse, FilterSpec, LPad, Ladder,
    Zobel,
};
pub use curves::ComplexCurve;
pub use error::{CircuitError, CircuitResult};
pub use model::{Circuit, Component, ComponentKind, LoadImpedance, NodeId, VoltageSource};
pub use optimize::{
    optimize, DesignVariable, OptimizationOptions, OptimizationReport, OptimizationTarget,
    OptimizationWarning, VariableField,
};
pub use solver::Solution;
