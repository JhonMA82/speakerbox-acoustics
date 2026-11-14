//! Integración: curva ZMA real → ajuste → estimaciones parciales.

use speakerbox_measure::{
    added_mass, fit_equivalent, parse_frd, parse_zma, sealed_box, FitOptions, FitWarning,
};

const ZMA: &str = include_str!("data/driver.zma");

fn relative(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected
}

#[test]
fn fixture_fits_the_known_circuit() {
    let parsed = parse_zma(ZMA).unwrap();
    let report = fit_equivalent(&parsed.curve, &FitOptions::default()).unwrap();
    assert!(report.converged, "iteraciones={}", report.iterations);
    assert!(
        report.max_relative_error < 1e-5,
        "error relativo máximo = {}",
        report.max_relative_error
    );
    let p = report.parameters;
    for (name, actual, expected) in [
        ("r1", p.r1_ohm(), 8.0),
        ("l1a", p.l1a_h(), 0.02),
        ("re", p.re_ohm(), 6.4),
        ("l1b", p.l1b_h(), 0.0015),
        ("r2", p.r2_ohm(), 45.0),
        ("l2", p.l2_h(), 0.09),
        ("c2", p.c2_f(), 1.759_048_327_123_92e-4),
    ] {
        assert!(
            relative(actual, expected) < 0.05,
            "{name}: {actual} vs {expected}"
        );
    }
    assert!(report.final_cost < report.initial_cost);
    assert!(!report.warnings.contains(&FitWarning::PoorFit));
}

#[test]
fn fitted_fixture_yields_the_expected_resonance() {
    let parsed = parse_zma(ZMA).unwrap();
    let report = fit_equivalent(&parsed.curve, &FitOptions::default()).unwrap();
    let ts = report.parameters.ts_estimate();
    assert!((ts.fs_hz - 40.0).abs() / 40.0 < 0.01, "fs={}", ts.fs_hz);
    assert!(ts.qms > 0.0 && ts.qts > 0.0);
    assert!(ts.qts < ts.qms);
}

#[test]
fn experiments_are_consistent_with_the_fitted_model() {
    let parsed = parse_zma(ZMA).unwrap();
    let report = fit_equivalent(&parsed.curve, &FitOptions::default()).unwrap();
    let fs = report.parameters.ts_estimate().fs_hz;
    let loaded = fs * 0.75;
    let mass = added_mass(fs, loaded, 0.05).unwrap();
    assert!(mass.mms_kg > 0.0 && mass.cms_m_per_n > 0.0);
    let boxed = sealed_box(fs, fs * 1.5, 0.01).unwrap();
    assert!((boxed.vas_m3 - 0.0125).abs() < 1e-9);
}

#[test]
fn frd_fixture_keeps_magnitude_and_phase() {
    let text = "* respuesta\n100 -6 0\n200 -3 45\n400 0 -90\n";
    let parsed = parse_frd(text).unwrap();
    assert_eq!(parsed.curve.len(), 3);
    let first = parsed.curve.values()[0];
    assert!((first.norm() - 10.0_f64.powf(-6.0 / 20.0)).abs() < 1e-12);
    assert!((first.arg()).abs() < 1e-12);
    let third = parsed.curve.values()[2];
    assert!((third.arg().to_degrees() + 90.0).abs() < 1e-12);
}

#[test]
fn fit_is_independent_of_point_order_for_the_same_grid() {
    let parsed = parse_zma(ZMA).unwrap();
    let first = fit_equivalent(&parsed.curve, &FitOptions::default()).unwrap();
    let second = fit_equivalent(&parsed.curve, &FitOptions::default()).unwrap();
    assert_eq!(first, second);
}
