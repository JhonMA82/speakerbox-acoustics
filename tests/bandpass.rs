//! Pruebas de integración de la API pública de bandpass.

use speakerbox_acoustics::{Bandpass, Design, Driver, EndCorrection, Port, SolveError};

fn driver() -> Driver {
    Driver::from_ui_units(211.20, 5.91e-4, 31.43, 5.90, 10.65, 4.11, 1.0, 1)
}

fn port(d_cm: f64, l_cm: f64) -> Port {
    let d = d_cm * 1.0e-2;
    Port::new(
        std::f64::consts::PI * d * d / 4.0,
        l_cm * 1.0e-2,
        EndCorrection::UnflangedBoth,
    )
}

#[test]
fn bp4_public_api_sweep_is_finite() {
    let bp = Bandpass::new_bp4(driver(), 20.0e-3, 10.0e-3, port(7.0, 15.0));
    let design = Design::Bandpass(bp);
    let pts = design.sweep_log(20.0, 1000.0, 40);
    assert_eq!(pts.len(), 40);
    assert!(pts
        .iter()
        .all(|p| p.spl_db.is_finite() && p.ze_mag > 0.0 && p.xd_mm >= 0.0));
}

#[test]
fn bp6_without_rear_port_reports_error() {
    let mut bp = Bandpass::new_bp6(driver(), 20.0e-3, 10.0e-3, port(7.0, 15.0), port(6.0, 20.0));
    bp.rear_port = None;
    let design = Design::Bandpass(bp);
    assert!(matches!(
        design.try_response_at(100.0),
        Err(SolveError::InvalidTopology(_))
    ));
}
