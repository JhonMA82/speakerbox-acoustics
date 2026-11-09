//! Integración de `DriverBank` con los modelos acústicos.

use speakerbox_acoustics::{Air, Chamber, Driver, DriverBank, Mechanical, System, Wiring};

fn base() -> Driver {
    Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1)
}

#[test]
fn four_driver_bank_builds_a_finite_sealed_system() {
    let bank = DriverBank::new(base(), 4)
        .with_wiring(Wiring::SeriesParallel)
        .with_mechanical(Mechanical::Standard);
    let driver = bank.equivalent_driver().expect("banco válido");
    let mut sys = System::new(Air::default(), driver);
    sys.solid_angle = std::f64::consts::TAU;
    sys.rear_chamber = Some(Chamber::new(40.0e-3));

    for f in [10.0, 20.0, 50.0, 100.0, 500.0, 1000.0] {
        let p = sys.response_at(f);
        assert!(p.spl_db.is_finite(), "SPL no finito a {f} Hz");
        assert!(p.ze_mag.is_finite() && p.ze_mag > 0.0);
        assert!(p.displacement_mm.is_finite() && p.displacement_mm >= 0.0);
    }
}

#[test]
fn bank_volume_scale_matches_mechanical_configuration() {
    let standard = DriverBank::new(base(), 4);
    let isobaric = DriverBank::new(base(), 4).with_mechanical(Mechanical::Isobaric);
    assert_eq!(standard.suggested_volume_scale().unwrap(), 4.0);
    assert_eq!(isobaric.suggested_volume_scale().unwrap(), 1.0);
}

#[test]
fn push_pull_does_not_change_the_linear_equivalent() {
    let standard = DriverBank::new(base(), 4)
        .with_wiring(Wiring::Parallel)
        .equivalent_driver()
        .unwrap();
    let push_pull = DriverBank::new(base(), 4)
        .with_mechanical(Mechanical::PushPull)
        .with_wiring(Wiring::Parallel)
        .equivalent_driver()
        .unwrap();
    assert_eq!(standard, push_pull);
}
