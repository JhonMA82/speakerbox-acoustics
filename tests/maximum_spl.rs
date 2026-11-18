use std::f64::consts::FRAC_PI_2;

use num_complex::Complex64 as C;
use speakerbox_acoustics::{
    maximum_spl::{MaximumSplLimits, OutputConstraint, PowerBasis, UnitDriveResponse},
    Air, Chamber, Design, Driver, System,
};

fn sample() -> UnitDriveResponse {
    UnitDriveResponse {
        frequency_hz: 50.,
        pressure_pa_per_v: C::new(0.3, 0.2),
        reference_pressure_pa: 2e-5,
        excursion_peak_m_per_v: 0.0002,
        terminal_w_per_v2: 0.2,
        coil_heat_w_per_v2: 0.08,
        radiation_quality: speakerbox_acoustics::energy::RadiationQuality::SingleBaffledPiston,
    }
}
#[test]
fn linear_limits_switch_and_zero_means_silence() {
    let sample = sample();
    let mut limits = MaximumSplLimits {
        excursion_peak_m: Some(0.002),
        power_limit: Some((PowerBasis::TerminalActive, 32.0)),
        amplifier_open_circuit_v_rms: Some(20.0),
    };
    let p = sample.maximum_spl(limits).unwrap();
    assert_eq!(p.dominant, OutputConstraint::Excursion);
    assert!((p.operating_voltage_rms - 10.).abs() < 1e-12);
    assert!((p.output.terminal_active_w - 20.).abs() < 1e-12);
    limits.power_limit = Some((PowerBasis::CoilHeat, 2.0));
    let p = sample.maximum_spl(limits).unwrap();
    assert_eq!(p.dominant, OutputConstraint::Power);
    assert!((p.operating_voltage_rms - 5.).abs() < 1e-12);
    assert!((p.output.coil_heat_w - 2.).abs() < 1e-12);
    limits.amplifier_open_circuit_v_rms = Some(3.);
    assert_eq!(
        sample.maximum_spl(limits).unwrap().dominant,
        OutputConstraint::AmplifierVoltage
    );
    limits.amplifier_open_circuit_v_rms = Some(0.);
    assert_eq!(
        sample.maximum_spl(limits).unwrap().output.spl_db,
        f64::NEG_INFINITY
    );
    limits.amplifier_open_circuit_v_rms = Some(f64::NAN);
    assert!(sample.maximum_spl(limits).is_err());
    assert!(sample.maximum_spl(MaximumSplLimits::default()).is_err());
}

#[test]
fn complex_drive_preserves_phase_and_real_power_at_terminals() {
    let mut sys = System::new(
        Air::default(),
        Driver::new(0.021, 0.0006, 0.03, 5.8, 8.5, 2.3, 0.002),
    );
    sys.rear_chamber = Some(Chamber::new(0.014));
    sys.rg = 2.0;
    for f in [30., 63., 180.] {
        let design = Design::System(sys.clone());
        let unit = UnitDriveResponse::from_design(&design, f).unwrap();
        let p = unit.at_voltage(C::from_polar(4.1, FRAC_PI_2)).unwrap();
        sys.eg = 4.1;
        let base = sys.try_response_at(f).unwrap();
        let energy = sys.try_energy_at(f).unwrap();
        assert!((p.spl_db - base.spl_db).abs() < 1e-10);
        assert!((p.excursion_peak_m - base.displacement_mm * 0.001).abs() < 1e-12);
        assert!((p.terminal_active_w - energy.driver_terminal_active_w).abs() < 1e-12);
        assert!((p.coil_heat_w - energy.coil_loss_w).abs() < 1e-12);
        assert!((p.pressure_pa / unit.pressure_pa_per_v - C::new(0.0, 4.1)).norm() < 1e-12);
    }
    assert!(UnitDriveResponse::from_design(&Design::System(sys), f64::NAN).is_err());
}
