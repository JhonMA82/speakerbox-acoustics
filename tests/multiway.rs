//! Integración del contrato de presión compleja multivía.

use num_complex::Complex64;
use speakerbox_acoustics::{
    sum_at, sum_sweep, Air, Chamber, Design, Driver, Flare, Horn, MultiwayChannel, Segment, System,
    MULTIWAY_REFERENCE_VOLTS,
};

fn driver() -> Driver {
    Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1)
}

fn sealed(eg: f64, volume_m3: f64) -> Design {
    let mut system = System::new(Air::default(), driver());
    system.rear_chamber = Some(Chamber::new(volume_m3));
    system.eg = eg;
    Design::System(system)
}

#[test]
fn two_way_sum_matches_manual_channels() {
    let low = sealed(MULTIWAY_REFERENCE_VOLTS, 20.0e-3);
    let high = sealed(1.0, 5.0e-3);
    let freq = 1000.0;
    let gain = 0.5;
    let delay = 2.0e-4;
    let channels = [
        MultiwayChannel::new(&low),
        MultiwayChannel::new(&high)
            .with_gain(gain)
            .with_delay_s(delay),
    ];
    let observation = sum_at(&channels, freq).unwrap();

    let p_low = low
        .try_observation_at(freq)
        .unwrap()
        .pressure_estimate_pa
        .unwrap();
    let p_high = high
        .try_observation_at(freq)
        .unwrap()
        .pressure_estimate_pa
        .unwrap()
        * (MULTIWAY_REFERENCE_VOLTS / 1.0);
    let expected =
        p_low + p_high * gain * Complex64::from_polar(1.0, -std::f64::consts::TAU * freq * delay);
    assert!((observation.pressure_pa.unwrap() - expected).norm() < 1e-12 * expected.norm());
}

#[test]
fn negative_eg_flips_the_channel() {
    let positive = sealed(2.83, 20.0e-3);
    let negative = sealed(-2.83, 20.0e-3);
    let a = sum_at(&[MultiwayChannel::new(&positive)], 120.0)
        .unwrap()
        .pressure_pa
        .unwrap();
    let b = sum_at(&[MultiwayChannel::new(&negative)], 120.0)
        .unwrap()
        .pressure_pa
        .unwrap();
    assert!((a + b).norm() < 1e-12 * a.norm());
}

#[test]
fn matched_mouth_has_no_defined_observation() {
    let mut system = System::new(Air::default(), driver());
    system.horn = Some(Horn::new(vec![Segment::new(
        Flare::Conical,
        0.60,
        80.0e-4,
        350.0e-4,
    )]));
    if let Some(horn) = system.horn.as_mut() {
        horn.matched_mouth = true;
    }
    let design = Design::System(system);
    let observation = sum_at(&[MultiwayChannel::new(&design)], 100.0).unwrap();
    assert!(observation.pressure_pa.is_none());
    assert!(observation.spl_db.is_none());
    assert!(observation.phase_deg.is_none());
}

#[test]
fn sweep_and_invalid_frequency() {
    let design = sealed(MULTIWAY_REFERENCE_VOLTS, 20.0e-3);
    let channels = [MultiwayChannel::new(&design)];
    let freqs = [20.0, 100.0, 1000.0];
    let sweep = sum_sweep(&channels, &freqs).unwrap();
    assert_eq!(sweep.len(), freqs.len());
    assert!(sum_sweep(&channels, &[100.0, f64::NAN]).is_err());
    assert!(sum_sweep(&channels, &[0.0]).is_err());
}
