//! Pruebas de integración del API público.

use speakerbox_acoustics::{
    Air, Chamber, Driver, EndCorrection, Flare, Horn, PassiveRadiator, Port, Segment, System,
};

fn default_driver() -> Driver {
    Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1)
}

#[test]
fn horn_sweep_is_finite_and_ordered() {
    let driver = default_driver();
    let mut sys = System::new(Air::default(), driver);
    sys.solid_angle = std::f64::consts::PI;
    sys.horn = Some(Horn::new(vec![
        Segment::new(Flare::Conical, 0.60, 80.0e-4, 350.0e-4),
        Segment::new(Flare::Exponential, 0.75, 350.0e-4, 2400.0e-4),
    ]));
    sys.throat_chamber = Some(Chamber::new(4.0e-3));
    sys.rear_chamber = Some(Chamber::new(14.0e-3));

    let freqs: Vec<f64> = (0..40).map(|i| 10.0 * 1.15_f64.powi(i)).collect();
    let pts = sys.response(&freqs);
    assert_eq!(pts.len(), freqs.len());
    for (i, p) in pts.iter().enumerate() {
        assert!((p.freq - freqs[i]).abs() < 1.0e-9);
        assert!(p.spl_db.is_finite(), "SPL a {} Hz", p.freq);
        assert!(p.ze_mag > 0.0);
        assert!(p.acoustical_power_w >= 0.0);
        assert!(p.displacement_mm >= 0.0);
    }
}

#[test]
fn vented_box_with_port_and_passive_radiator() {
    let driver = default_driver();
    let mut sys = System::new(Air::default(), driver);
    sys.solid_angle = std::f64::consts::TAU;
    sys.rear_chamber = Some(Chamber::new(50.0e-3));
    sys.port = Some(Port::new(100.0e-4, 0.12, EndCorrection::Inner(0.85)));
    sys.passive_radiator = Some(PassiveRadiator::new(350.0e-4, 4.0e-4, 20.0e-3, 4.0));

    let p = sys.response_at(40.0);
    assert!(p.spl_db.is_finite());
    assert!(p.port_velocity.is_some());
    assert!(p.port_velocity.unwrap() >= 0.0);
}

#[test]
fn custom_flare_profile() {
    let area = |x: f64| 80.0e-4 * (1.0 + 3.0 * x).powi(2);
    let seg = Segment::new(
        Flare::Custom(std::sync::Arc::new(area)),
        0.5,
        80.0e-4,
        area(0.5),
    );
    assert!((seg.area(0.0) - 80.0e-4).abs() < 1.0e-12);
    assert!((seg.area(0.5) - area(0.5)).abs() < 1.0e-12);
    let horn = Horn::new(vec![seg]);
    let z = horn.input_impedance(&Air::default(), 200.0, std::f64::consts::TAU);
    assert!(z.re > 0.0);
}

#[test]
fn rear_tuning_matches_helmholtz() {
    let driver = default_driver();
    let mut sys = System::new(Air::default(), driver);
    let volume = 50.0e-3;
    sys.rear_chamber = Some(Chamber::new(volume));
    let port = Port::new(100.0e-4, 0.12, EndCorrection::Inner(0.85));
    sys.port = Some(port);
    let c_box = volume / (sys.air.rho * sys.air.c * sys.air.c);
    let fb = 1.0 / (std::f64::consts::TAU * (port.acoustic_mass(&sys.air) * c_box).sqrt());
    let got = sys.rear_tuning_hz().expect("sintonía trasera");
    assert!((got - fb).abs() / fb < 1.0e-12, "got={got}, fb={fb}");
}

#[test]
fn vented_box_impedance_minimum_near_tuning() {
    let driver = default_driver();
    let mut sys = System::new(Air::default(), driver);
    sys.solid_angle = std::f64::consts::TAU;
    let volume = 50.0e-3;
    sys.rear_chamber = Some(Chamber::new(volume));
    let port = Port::new(100.0e-4, 0.12, EndCorrection::Inner(0.85));
    sys.port = Some(port);

    // Frecuencia de sintonía del recinto: Fb = 1/(2pi·sqrt(Ma·C)).
    let c_box = volume / (sys.air.rho * sys.air.c * sys.air.c);
    let fb = 1.0 / (std::f64::consts::TAU * (port.acoustic_mass(&sys.air) * c_box).sqrt());

    let freqs: Vec<f64> = (0..80).map(|i| 10.0 * 1.06_f64.powi(i)).collect();
    let pts = sys.response(&freqs);
    let min = pts
        .iter()
        .min_by(|a, b| a.ze_mag.partial_cmp(&b.ze_mag).unwrap())
        .unwrap();
    // El mínimo de |Ze| debe caer cerca de la sintonía (red de puerto + caja).
    assert!(
        (min.freq - fb).abs() / fb < 0.10,
        "f_min={} Hz, Fb={} Hz",
        min.freq,
        fb
    );
}

#[test]
fn direct_radiator_electrical_impedance_has_resonance() {
    let driver = default_driver();
    let mut sys = System::new(Air::default(), driver);
    sys.solid_angle = std::f64::consts::TAU;
    let fs = driver_fs(&sys);
    let z_fs = sys.response_at(fs).ze_mag;
    let z_high = sys.response_at(20.0 * fs).ze_mag;
    // En resonancia la impedancia eléctrica es máxima (pico de motional Z).
    assert!(z_fs > z_high);
}

fn driver_fs(sys: &System) -> f64 {
    sys.driver.fs(&sys.air)
}
