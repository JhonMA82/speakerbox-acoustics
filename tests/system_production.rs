//! Validación de producción de caja sellada, caja ventilada y radiador pasivo
//! contra exports propios del software de referencia 60.70.
//!
//! Tolerancias puntuales:
//! - SPL: RMS ≤ 2.0 dB, máximo ≤ 3.5 dB.
//! - Ze:  RMS ≤ 15 %, máximo ≤ 35 %.
//! - Xd:  RMS ≤ 15 %, máximo ≤ 40 % (se ignoran los puntos con Xd ≤ 0.02 mm).
//!
//! La banda de producción es 10 Hz–1 kHz. Los diseños usan el mismo driver que
//! las validaciones de TH/OD/BP.

use speakerbox_acoustics::{
    validate, Air, Chamber, Driver, EndCorrection, OraclePoint, PassiveRadiator, Port, System,
};

fn driver() -> Driver {
    Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1)
}

fn base() -> System {
    let mut sys = System::new(Air::default(), driver());
    sys.solid_angle = std::f64::consts::TAU;
    sys.eg = 2.83;
    sys.rg = 0.0;
    sys
}

fn sealed() -> System {
    let mut sys = base();
    sys.rear_chamber = Some(Chamber::new(50.0e-3));
    sys
}

fn vented() -> System {
    let mut sys = base();
    sys.rear_chamber = Some(Chamber::new(100.0e-3));
    // Corrección de extremo calibrada para el puerto del software de referencia:
    // extremo interior embridado + extremo exterior sin brida ≈ 1.5·a.
    sys.port = Some(Port::new(100.0e-4, 0.10, EndCorrection::Inner(1.5)));
    sys.rear_ql = Some(7.0);
    sys
}

fn passive_radiator() -> System {
    let mut sys = base();
    sys.rear_chamber = Some(Chamber::new(100.0e-3));
    // El registro del software de referencia usa Cmp = 4.00E-01 m/N.
    sys.passive_radiator = Some(PassiveRadiator::new(350.0e-4, 0.4, 40.0e-3, 4.0));
    sys
}

fn run(name: &str, sys: &System, oracle_path: &str) {
    let text = std::fs::read_to_string(oracle_path)
        .unwrap_or_else(|e| panic!("no se pudo leer {oracle_path}: {e}"));
    let oracle: Vec<OraclePoint> = OraclePoint::parse_tsv(&text)
        .into_iter()
        .filter(|o| o.freq_hz <= 1000.0)
        .collect();
    let report = validate(sys, &oracle);
    println!(
        "{name}: SPL {:.2}/{:.2} dB | Ze {:.1}/{:.1} % | Xd {:.1}/{:.1} %",
        report.spl_rmse_db,
        report.spl_max_abs_db,
        report.ze_rel_rmse * 100.0,
        report.ze_max_rel * 100.0,
        report.xd_rel_rmse * 100.0,
        report.xd_max_rel * 100.0
    );
    assert!(
        report.passes_pointwise(2.0, 3.5, 0.15, 0.35, 0.15, 0.40),
        "{name}: no cumple las tolerancias puntuales"
    );
}

#[test]
fn sealed_box_meets_production_tolerances() {
    run("SEALED-A", &sealed(), "tests/data/sealed_a_oracle.txt");
}

#[test]
fn vented_box_meets_production_tolerances() {
    run("VENTED-A", &vented(), "tests/data/vented_a_oracle.txt");
}

#[test]
fn passive_radiator_box_meets_production_tolerances() {
    run("PR-A", &passive_radiator(), "tests/data/pr_a_oracle.txt");
}
