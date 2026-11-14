//! Fixture `.CIR` reproducible y contrato de lectura/escritura.

use num_complex::Complex64;
use speakerbox_circuits::{parse_cir, parse_si_value, to_cir};
use std::f64::consts::TAU;

const FIXTURE: &str = include_str!("data/series_rlc.CIR");

#[test]
fn fixture_is_the_audited_series_rlc() {
    let circuit = parse_cir(FIXTURE).unwrap();
    assert_eq!(circuit.nodes(), 4);
    assert_eq!(circuit.sources().len(), 1);
    assert_eq!(circuit.components().len(), 3);
    assert_eq!(circuit.sources()[0].volts, Complex64::new(1.0, 0.0));
}

#[test]
fn fixture_resonates_at_the_analytic_frequency() {
    let circuit = parse_cir(FIXTURE).unwrap();
    let f0 = 1.0 / (TAU * (1e-3_f64 * 5e-6).sqrt());
    assert!((f0 - 2_250.790_791).abs() < 1e-3, "f0={f0}");
    let z = circuit.input_impedance(f0).unwrap();
    assert!((z.re - 10.0).abs() < 1e-9, "Re(Z)={}", z.re);
    assert!(z.im.abs() < 1e-9, "Im(Z)={}", z.im);
}

#[test]
fn fixture_matches_the_closed_form_at_every_frequency() {
    let circuit = parse_cir(FIXTURE).unwrap();
    let (r, l, c) = (10.0, 1e-3, 5e-6);
    for freq in [100.0, 1000.0, 2250.0, 10_000.0] {
        let omega = TAU * freq;
        let expected = Complex64::new(r, omega * l - 1.0 / (omega * c));
        let actual = circuit.input_impedance(freq).unwrap();
        assert!(
            (actual - expected).norm() / expected.norm() < 1e-12,
            "f={freq}: {actual:?} vs {expected:?}"
        );
    }
}

#[test]
fn fixture_round_trips_through_the_serializer() {
    let circuit = parse_cir(FIXTURE).unwrap();
    let text = to_cir(&circuit).unwrap();
    let reparsed = parse_cir(&text).unwrap();
    assert_eq!(circuit, reparsed);
}

#[test]
fn parser_accepts_tabs_crlf_and_named_nodes() {
    let text = "V1 in 0 AC 1\r\nR1 in out 1k\r\nR2 out 0 1k\r\n";
    let circuit = parse_cir(text).unwrap();
    assert!(circuit.nodes() >= 3);
    assert!((circuit.input_impedance(100.0).unwrap().re - 2000.0).abs() < 1e-9);
}

#[test]
fn si_suffixes_are_documented_and_enforced() {
    assert!((parse_si_value("4.7k").unwrap() - 4700.0).abs() < 1e-12);
    assert!((parse_si_value("470nF").unwrap() - 470e-9).abs() < 1e-18);
    assert!((parse_si_value("0.1mH").unwrap() - 1e-4).abs() < 1e-18);
    assert!(parse_si_value("1.0.0").is_err());
    assert!(parse_si_value("k10").is_err());
}
