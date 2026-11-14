//! Oráculos analíticos independientes del solver.
//!
//! Cada valor esperado se calcula con la fórmula de teoría de circuitos, no
//! con este mismo solver.

use num_complex::Complex64;
use speakerbox_circuits::{Circuit, ComplexCurve, Component, NodeId, VoltageSource};
use std::f64::consts::TAU;

const TOL: f64 = 1e-12;

fn series_rlc(r: f64, l: f64, c: f64) -> Circuit {
    let mut circuit = Circuit::new(4);
    circuit
        .push_source(VoltageSource::new(
            NodeId(1),
            NodeId::GROUND,
            Complex64::new(1.0, 0.0),
        ))
        .unwrap();
    circuit
        .push_component(Component::resistor(NodeId(1), NodeId(2), r))
        .unwrap();
    circuit
        .push_component(Component::inductor(NodeId(2), NodeId(3), l, 0.0))
        .unwrap();
    circuit
        .push_component(Component::capacitor(NodeId(3), NodeId::GROUND, c))
        .unwrap();
    circuit
}

fn assert_complex_close(actual: Complex64, expected: Complex64, tol: f64, context: &str) {
    let scale = expected.norm().max(1.0);
    assert!(
        (actual - expected).norm() / scale < tol,
        "{context}: actual={actual:?} expected={expected:?}"
    );
}

#[test]
fn resistor_only_is_frequency_independent() {
    let mut circuit = Circuit::new(2);
    circuit
        .push_source(VoltageSource::new(
            NodeId(1),
            NodeId::GROUND,
            Complex64::new(1.0, 0.0),
        ))
        .unwrap();
    circuit
        .push_component(Component::resistor(NodeId(1), NodeId::GROUND, 8.2))
        .unwrap();
    for freq in [1.0, 20.0, 1_000.0, 20_000.0, 200_000.0] {
        assert_complex_close(
            circuit.input_impedance(freq).unwrap(),
            Complex64::new(8.2, 0.0),
            TOL,
            "R",
        );
    }
}

#[test]
fn rc_series_matches_closed_form() {
    let (r, c) = (1000.0, 1e-6);
    let mut circuit = Circuit::new(3);
    circuit
        .push_source(VoltageSource::new(
            NodeId(1),
            NodeId::GROUND,
            Complex64::new(1.0, 0.0),
        ))
        .unwrap();
    circuit
        .push_component(Component::resistor(NodeId(1), NodeId(2), r))
        .unwrap();
    circuit
        .push_component(Component::capacitor(NodeId(2), NodeId::GROUND, c))
        .unwrap();
    for freq in [10.0, 159.154_943_091, 1000.0, 10_000.0] {
        let omega = TAU * freq;
        let expected = Complex64::new(r, -1.0 / (omega * c));
        assert_complex_close(circuit.input_impedance(freq).unwrap(), expected, TOL, "RC");
    }
}

#[test]
fn rl_series_matches_closed_form() {
    let (r, l) = (4.7, 2.2e-3);
    let mut circuit = Circuit::new(3);
    circuit
        .push_source(VoltageSource::new(
            NodeId(1),
            NodeId::GROUND,
            Complex64::new(1.0, 0.0),
        ))
        .unwrap();
    circuit
        .push_component(Component::resistor(NodeId(1), NodeId(2), r))
        .unwrap();
    circuit
        .push_component(Component::inductor(NodeId(2), NodeId::GROUND, l, 0.0))
        .unwrap();
    for freq in [10.0, 100.0, 1000.0, 10_000.0] {
        let expected = Complex64::new(r, TAU * freq * l);
        assert_complex_close(circuit.input_impedance(freq).unwrap(), expected, TOL, "RL");
    }
}

#[test]
fn rlc_series_matches_closed_form_and_resonance() {
    let (r, l, c) = (10.0, 1e-3, 5e-6);
    let circuit = series_rlc(r, l, c);
    let f0 = 1.0 / (TAU * (l * c).sqrt());
    for freq in [0.1 * f0, f0, 2.0 * f0, 10.0 * f0] {
        let omega = TAU * freq;
        let expected = Complex64::new(r, omega * l - 1.0 / (omega * c));
        assert_complex_close(circuit.input_impedance(freq).unwrap(), expected, TOL, "RLC");
    }
    let at_resonance = circuit.input_impedance(f0).unwrap();
    assert!((at_resonance.re - r).abs() < 1e-9);
    assert!(at_resonance.im.abs() < 1e-9);
}

#[test]
fn resistive_divider_matches_voltage_ratio() {
    let mut circuit = Circuit::new(3);
    circuit
        .push_source(VoltageSource::new(
            NodeId(1),
            NodeId::GROUND,
            Complex64::new(2.0, 0.0),
        ))
        .unwrap();
    circuit
        .push_component(Component::resistor(NodeId(1), NodeId(2), 1000.0))
        .unwrap();
    circuit
        .push_component(Component::resistor(NodeId(2), NodeId::GROUND, 3000.0))
        .unwrap();
    let transfer = circuit
        .voltage_transfer(1000.0, NodeId(2), NodeId(1))
        .unwrap();
    assert_complex_close(transfer, Complex64::new(0.75, 0.0), TOL, "divisor");
    let source_voltage = circuit.solve(1000.0).unwrap().voltage(NodeId(1)).unwrap();
    assert_complex_close(source_voltage, Complex64::new(2.0, 0.0), TOL, "fuente");
}

#[test]
fn ladder_impedance_matches_hand_reduction() {
    // R1 + (R2 || (R3 + R4)) = 100 + 200 || 700 = 255.555...
    let mut circuit = Circuit::new(4);
    circuit
        .push_source(VoltageSource::new(
            NodeId(1),
            NodeId::GROUND,
            Complex64::new(1.0, 0.0),
        ))
        .unwrap();
    circuit
        .push_component(Component::resistor(NodeId(1), NodeId(2), 100.0))
        .unwrap();
    circuit
        .push_component(Component::resistor(NodeId(2), NodeId::GROUND, 200.0))
        .unwrap();
    circuit
        .push_component(Component::resistor(NodeId(2), NodeId(3), 300.0))
        .unwrap();
    circuit
        .push_component(Component::resistor(NodeId(3), NodeId::GROUND, 400.0))
        .unwrap();
    let expected = 100.0 + (200.0 * 700.0) / (200.0 + 700.0);
    assert_complex_close(
        circuit.input_impedance(1000.0).unwrap(),
        Complex64::new(expected, 0.0),
        TOL,
        "escalera",
    );
}

#[test]
fn balanced_bridge_has_no_current_through_the_bridge_element() {
    let mut circuit = Circuit::new(4);
    circuit
        .push_source(VoltageSource::new(
            NodeId(1),
            NodeId::GROUND,
            Complex64::new(1.0, 0.0),
        ))
        .unwrap();
    circuit
        .push_component(Component::resistor(NodeId(1), NodeId(2), 1000.0))
        .unwrap();
    circuit
        .push_component(Component::resistor(NodeId(2), NodeId::GROUND, 2000.0))
        .unwrap();
    circuit
        .push_component(Component::resistor(NodeId(1), NodeId(3), 3000.0))
        .unwrap();
    circuit
        .push_component(Component::resistor(NodeId(3), NodeId::GROUND, 6000.0))
        .unwrap();
    circuit
        .push_component(Component::resistor(NodeId(2), NodeId(3), 470.0))
        .unwrap();
    let solution = circuit.solve(1000.0).unwrap();
    let v2 = solution.voltage(NodeId(2)).unwrap();
    let v3 = solution.voltage(NodeId(3)).unwrap();
    assert!(
        (v2 - v3).norm() < 1e-12,
        "puente desequilibrado: {v2:?} vs {v3:?}"
    );
    let arm_a = 1000.0 + 2000.0;
    let arm_b = 3000.0 + 6000.0;
    let expected = arm_a * arm_b / (arm_a + arm_b);
    assert_complex_close(
        circuit.input_impedance(1000.0).unwrap(),
        Complex64::new(expected, 0.0),
        1e-10,
        "puente",
    );
}

#[test]
fn two_equal_loads_in_parallel_halve_the_impedance() {
    let z = Complex64::new(6.4, 2.1);
    let mut circuit = Circuit::new(2);
    circuit
        .push_source(VoltageSource::new(
            NodeId(1),
            NodeId::GROUND,
            Complex64::new(1.0, 0.0),
        ))
        .unwrap();
    circuit
        .push_component(Component::load(NodeId(1), NodeId::GROUND, z))
        .unwrap();
    circuit
        .push_component(Component::load(NodeId(1), NodeId::GROUND, z))
        .unwrap();
    assert_complex_close(
        circuit.input_impedance(1000.0).unwrap(),
        z / 2.0,
        TOL,
        "dos cargas",
    );
}

#[test]
fn measured_load_curve_is_used_inside_its_band_only() {
    let curve = ComplexCurve::new(
        vec![100.0, 1000.0, 10_000.0],
        vec![
            Complex64::new(4.0, 1.0),
            Complex64::new(8.0, 2.0),
            Complex64::new(16.0, 4.0),
        ],
    )
    .unwrap();
    let mut circuit = Circuit::new(2);
    circuit
        .push_source(VoltageSource::new(
            NodeId(1),
            NodeId::GROUND,
            Complex64::new(1.0, 0.0),
        ))
        .unwrap();
    circuit
        .push_component(Component::load_curve(NodeId(1), NodeId::GROUND, curve))
        .unwrap();
    assert_complex_close(
        circuit.input_impedance(1000.0).unwrap(),
        Complex64::new(8.0, 2.0),
        TOL,
        "carga medida",
    );
    assert!(circuit.input_impedance(50.0).is_err());
    assert!(circuit.input_impedance(20_000.0).is_err());
}

#[test]
fn passive_networks_never_show_negative_input_resistance() {
    let circuit = series_rlc(10.0, 1e-3, 5e-6);
    for freq in [10.0, 100.0, 1000.0, 2250.0, 10_000.0, 100_000.0] {
        let z = circuit.input_impedance(freq).unwrap();
        assert!(z.re >= -1e-12, "Re(Z)={} a {freq} Hz", z.re);
    }
}

#[test]
fn extreme_scales_stay_accurate() {
    for ohm in [1e-3, 1.0, 1e9] {
        let mut circuit = Circuit::new(2);
        circuit
            .push_source(VoltageSource::new(
                NodeId(1),
                NodeId::GROUND,
                Complex64::new(1.0, 0.0),
            ))
            .unwrap();
        circuit
            .push_component(Component::resistor(NodeId(1), NodeId::GROUND, ohm))
            .unwrap();
        let z = circuit.input_impedance(1000.0).unwrap();
        assert!((z.re - ohm).abs() / ohm < 1e-9, "escala {ohm}: Z={z:?}");
    }
}
