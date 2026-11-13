//! # speakerbox-acoustics
//!
//! Motor de simulación acústica de altavoces: guías de onda por segmentos,
//! tapped horn, offset driver, bandpass BP4/BP6, radiador directo, recinto
//! sellado o ventilado, cámara de garganta, puerto y radiador pasivo.
//!
//! Es una implementación independiente basada en ecuaciones públicas de
//! acústica, validada punto por punto contra exports propios de la aplicación
//! de referencia. No copia código de esa aplicación; los exports se usan solo
//! como oráculo de validación.
//!
//! El motor resuelve la ecuación de Webster en cada segmento mediante
//! integración numérica RK4 y propaga impedancias con matrices de transmisión
//! 2×2. Eso permite usar cualquier perfil de área `S(x)`: cilíndrico, cónico,
//! exponencial, parabólico, hiperbólico-exponencial o un perfil definido por
//! el usuario.
//!
//! ## Inicio rápido
//!
//! ```rust
//! use speakerbox_acoustics::{Bandpass, Driver, EndCorrection, Port};
//!
//! let driver = Driver::from_ui_units(
//!     350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1,
//! );
//! let port = Port::new(50.0e-4, 0.10, EndCorrection::UnflangedBoth);
//! let bp = Bandpass::new_bp4(driver, 20.0e-3, 10.0e-3, port);
//! let p = bp.response_at(50.0);
//! assert!(p.spl_db.is_finite());
//! ```
//!
//! Para barrer varias topologías con una sola interfaz use [`Design`]:
//!
//! ```rust
//! use speakerbox_acoustics::{Design, Driver, Flare, Segment, TappedHorn};
//!
//! let driver = Driver::from_ui_units(
//!     350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1,
//! );
//! let th = TappedHorn::new_4seg_th(driver, vec![
//!     Segment::new(Flare::Conical, 0.40, 400e-4, 400e-4),
//!     Segment::new(Flare::Exponential, 0.20, 400e-4, 400e-4),
//!     Segment::new(Flare::Conical, 1.20, 400e-4, 1200e-4),
//!     Segment::new(Flare::Conical, 0.30, 1200e-4, 1500e-4),
//! ]);
//! let design = Design::TappedHorn(th);
//! let points = design.sweep_log(20.0, 200.0, 6);
//! assert_eq!(points.len(), 6);
//! ```
//!
//! ## Topologías
//!
//! | Topología | Tipo | Constructor |
//! |---|---|---|
//! | Radiador directo / sellada / ventilada / pasivo | [`System`] | [`System::new`] |
//! | Bocina normal | [`System`] + [`Horn`] | [`Horn::new`] |
//! | Tapped horn TH (S2/S4) | [`TappedHorn`] | [`TappedHorn::new_4seg_th`] |
//! | Tapped horn TH1 (S2/S3) | [`TappedHorn`] | [`TappedHorn::new_4seg_th1`] |
//! | Offset driver OD/OD1/OD2 | [`OffsetHorn`] | `new_4seg_od*` |
//! | BP4 | [`Bandpass`] | [`Bandpass::new_bp4`] |
//! | BP6 paralelo | [`Bandpass`] | [`Bandpass::new_bp6`] |
//! | BP6 serie | [`Bandpass`] | [`Bandpass::new_bp6_series`] |
//!
//! ## Bancos de drivers
//!
//! Para cajas dobles, torres con varios drivers o configuraciones isobáricas,
//! [`DriverBank`] combina N drivers idénticos y devuelve un [`Driver`]
//! equivalente:
//!
//! ```rust
//! use speakerbox_acoustics::{Driver, DriverBank, Wiring};
//!
//! let base = Driver::from_ui_units(
//!     350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1,
//! );
//! let bank = DriverBank::new(base, 4).with_wiring(Wiring::SeriesParallel);
//! let equivalent = bank.equivalent_driver().unwrap();
//! assert_eq!(equivalent.sd, base.sd * 4.0);
//! ```
//!
//! Soporta paralelo, serie, serie-paralelo, isobaric por pares, push-pull y
//! DVC paralelo/serie. Bessel y DVC Separate requieren capas adicionales.
//!
//! ## Versión y estabilidad
//!
//! Versión `0.2.0`, **pre-1.0**: la API pública puede cambiar entre versiones
//! menores. La etiqueta «v1» de `README.md` y `ROADMAP.md` se refiere al
//! alcance de topologías validado contra exports propios, no a estabilidad
//! SemVer 1.0.
//!
//! ## Importar drivers
//!
//! [`import_driver`] autodetecta ficheros WinISD `.wdr` y OpenISD `.owdr` a
//! partir de texto, por lo que funciona en nativo y en wasm. [`import_wdr`] e
//! [`import_owdr`] fuerzan el formato. [`import_driver_file`] lee desde disco
//! y solo se compila en nativo; en wasm use el lector de la aplicación y pase
//! el texto a [`import_driver`]. El resultado es un [`ImportedDriver`] con el
//! [`Driver`] y metadatos (marca, modelo, Xmax, Pe, número de bobinas).
//!
//! ## Convenciones
//!
//! - El API público usa **SI**: m², m, m³, kg, N/m, T·m, Ω, H, Pa. Los
//!   constructores `from_ui_units*` aceptan cm², cm, litros, gramos, mH y mm.
//! - `spl_db` es la presión de potencia a 1 m, dB re 20 µPa.
//! - `ze_mag`/`ze_phase_deg` son el módulo y la fase de la impedancia
//!   eléctrica de entrada.
//! - `xd_mm` es desplazamiento de **pico** (`√2`·RMS) en todas las
//!   topologías.
//! - `solid_angle` va en estereorradianes: `4π` espacio libre, `2π` media
//!   esfera, `π` cuarto de espacio, `π/2` octavo de espacio.
//! - `response_at` devuelve un punto neutro si el diseño no es resoluble;
//!   `try_response_at` devuelve [`SolveError`] y valida entrada, configuración
//!   y todos los campos del resultado.
//! - [`validate`] compara SPL, `Ze` y `Xd`; **no** compara la fase de
//!   impedancia (`ze_phase_deg`), que solo se conserva para trazabilidad.
//!
//! ## Documentación
//!
//! - `docs/INTEGRATION.md`: guía de integración con ejemplos por topología.
//! - `docs/AGENTS.md`: mapa del repositorio, comandos e invariantes para
//!   agentes.
//! - `docs/ALGORITHMS.md`: ecuaciones y calibración.
//! - `docs/VALIDATION.md`: validación de producción contra exports propios.
//! - `ROADMAP.md`: alcance v1 y topologías diferidas.
//!
//! La validación de producción se ejecuta con `cargo test --workspace`.
//! Los datos y oráculos están en `tests/data`.
//!
pub mod air;
pub mod bandpass;
pub mod calibration;
pub mod design;
pub mod driver;
pub mod driver_bank;
pub mod driver_import;
pub mod element;
pub mod error;
pub mod offset;
pub mod radiation;
pub mod record;
pub mod segment;
pub mod system;
pub mod tapped;
pub mod validation;

pub use air::Air;
pub use bandpass::{Bandpass, BandpassOrder, BandpassPoint, PortModel, RearPortRouting};
pub use calibration::Calibration;
pub use design::{Design, DesignPoint};
pub use driver::{CoilModel, Driver};
pub use driver_bank::{BankError, DriverBank, DvcWiring, Mechanical, Wiring};
#[cfg(not(target_arch = "wasm32"))]
pub use driver_import::import_driver_file;
pub use driver_import::{
    import_driver, import_owdr, import_wdr, DriverFormat, ImportError, ImportedDriver,
};
pub use element::{Chamber, EndCorrection, Lining, PassiveRadiator, Port};
pub use error::SolveError;
pub use offset::{OffsetHorn, OffsetPoint};
pub use radiation::piston_radiation_impedance;
pub use record::LegacyRecord;
pub use segment::{Flare, Horn, Segment, TransferMatrix};
pub use system::{ResponsePoint, System};
pub use tapped::{TappedHorn, TappedPoint};
pub use validation::{validate, OraclePoint, ValidationError, ValidationReport};

/// Frecuencia angular a partir de una frecuencia en hercios.
#[inline]
pub(crate) fn omega(freq_hz: f64) -> f64 {
    std::f64::consts::TAU * freq_hz
}

/// Aceleración debida a la gravedad (no se usa en el modelo acústico).
pub const G0: f64 = 9.806_65;

#[cfg(test)]
mod auto_traits {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn core_types_are_send_sync() {
        assert_send_sync::<Air>();
        assert_send_sync::<Driver>();
        assert_send_sync::<Segment>();
        assert_send_sync::<Horn>();
        assert_send_sync::<System>();
        assert_send_sync::<TappedHorn>();
        assert_send_sync::<OffsetHorn>();
        assert_send_sync::<Design>();
        assert_send_sync::<DesignPoint>();
    }
}

/// Fase desenrollada y retardo con representación explícita de cancelaciones.
pub mod phase;

/// Potencias activas de red y alcance de la interpretación radiativa.
pub mod energy;

/// Alineamiento sellado analítico y TL uniforme experimental.
pub mod reference_design;

pub mod transmission_line;
pub use transmission_line::{Fold, LinePoint, LineTermination, TransmissionLine};

pub mod optimization;

pub mod measured_data;

pub mod alignments;
