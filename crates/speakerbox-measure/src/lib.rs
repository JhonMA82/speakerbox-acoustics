//! # speakerbox-measure
//!
//! Lectura de curvas de altavoz e identificación del circuito equivalente de
//! impedancia.
//!
//! Es una **reimplementación independiente** de las matemáticas descritas en
//! la auditoría de portabilidad: no contiene código de Speaker Workshop. Las
//! fórmulas son el circuito equivalente público, las relaciones del tanque
//! resonante y dos experimentos físicos clásicos (masa añadida y caja
//! sellada).
//!
//! ## Qué hace
//!
//! - Lee FRD (dB, grados) y ZMA (ohmios, grados) conservando magnitud y fase.
//! - Evalúa el circuito equivalente `R1∥L1a + Re+jωL1b + R2∥L2∥C2`.
//! - Ajusta los siete parámetros con Levenberg–Marquardt restringido a valores
//!   positivos, con residual complejo e informe de convergencia.
//! - Estima `Fs`, `Qms`, `Qes` y `Qts` desde la rama resonante.
//! - Calcula masa y compliancia con una masa añadida, y `Vas` con una caja
//!   sellada.
//!
//! ## Qué no hace
//!
//! - No convierte el resultado en un
//!   [`speakerbox_acoustics::Driver`](https://docs.rs/speakerbox-acoustics):
//!   faltan `Bl`, `Sd` y una base física para repartir los elementos del
//!   circuito entre motor y suspensión.
//! - No inventa parámetros que el experimento no determine.
//! - No asume calibración de las curvas: ni 1 V, ni 2.83 V, ni 1 W.
//!
//! ## Ejemplo
//!
//! ```
//! use num_complex::Complex64;
//! use speakerbox_circuits::ComplexCurve;
//! use speakerbox_measure::{fit_equivalent, initial_guess, FitOptions};
//!
//! // Curva sintética de cinco puntos alrededor de la resonancia.
//! let freqs = vec![10.0, 20.0, 40.0, 80.0, 160.0, 320.0, 640.0];
//! let values = vec![
//!     Complex64::new(6.4, 0.5),
//!     Complex64::new(9.0, 4.0),
//!     Complex64::new(59.4, 0.0),
//!     Complex64::new(9.0, -4.0),
//!     Complex64::new(6.4, -0.5),
//!     Complex64::new(6.6, -0.2),
//!     Complex64::new(6.8, -0.1),
//! ];
//! let curve = ComplexCurve::new(freqs, values).unwrap();
//! let seed = initial_guess(&curve).unwrap();
//! let report = fit_equivalent(&curve, &FitOptions { initial: Some(seed), ..FitOptions::default() }).unwrap();
//! assert!(report.final_cost <= report.initial_cost);
//! ```
//!
//! ## Compatibilidad
//!
//! Rust 1.75 y `wasm32-unknown-unknown`. Depende de `num-complex` y de
//! `speakerbox-circuits` (curvas y tipos complejos), nunca del crate acústico.

pub mod curve_file;
pub mod equiv;
pub mod error;
pub mod experiments;
pub mod fit;
pub mod validate;

pub use curve_file::{parse_frd, parse_zma, write_frd, write_zma, CurveKind, ParsedCurve};
pub use equiv::{EquivalentImpedance, TsEstimate, PARAMETER_NAMES};
pub use error::{CurveParseError, MeasureError, MeasureResult};
pub use experiments::{added_mass, sealed_box, AddedMassEstimate, SealedBoxEstimate};
pub use fit::{fit_equivalent, initial_guess, FitOptions, FitReport, FitWarning};
pub use validate::{
    load_dataset, parse_meta, validate_curves, validate_dataset, Dataset, DatasetError,
    DatasetMeta, ValidationReport,
};
