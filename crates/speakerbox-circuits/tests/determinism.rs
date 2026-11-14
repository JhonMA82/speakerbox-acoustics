//! El resultado no depende del orden de construcción ni de la repetición.

use num_complex::Complex64;
use speakerbox_circuits::{Circuit, Component, NodeId, VoltageSource};

fn components() -> Vec<Component> {
    vec![
        Component::resistor(NodeId(1), NodeId(2), 4.7),
        Component::inductor(NodeId(2), NodeId(3), 1e-3, 0.05),
        Component::capacitor(NodeId(3), NodeId::GROUND, 5e-6),
        Component::resistor(NodeId(2), NodeId::GROUND, 22.0),
        Component::load(NodeId(3), NodeId::GROUND, Complex64::new(6.4, 2.1)),
    ]
}

fn build(order: &[usize]) -> Circuit {
    let mut circuit = Circuit::new(4);
    circuit
        .push_source(VoltageSource::new(
            NodeId(1),
            NodeId::GROUND,
            Complex64::new(1.0, 0.0),
        ))
        .unwrap();
    let parts = components();
    for index in order {
        circuit.push_component(parts[*index].clone()).unwrap();
    }
    circuit
}

#[test]
fn component_order_does_not_change_the_result() {
    let forward = build(&[0, 1, 2, 3, 4]);
    let reversed = build(&[4, 3, 2, 1, 0]);
    for freq in [20.0, 200.0, 2000.0, 20_000.0] {
        let a = forward.solve(freq).unwrap();
        let b = reversed.solve(freq).unwrap();
        for (va, vb) in a.node_voltages.iter().zip(&b.node_voltages) {
            assert!((va - vb).norm() < 1e-14, "{va:?} vs {vb:?}");
        }
        let za = forward.input_impedance(freq).unwrap();
        let zb = reversed.input_impedance(freq).unwrap();
        assert!((za - zb).norm() / za.norm() < 1e-14);
    }
}

#[test]
fn repeated_solves_are_bit_identical() {
    let circuit = build(&[0, 1, 2, 3, 4]);
    let first = circuit.solve(1000.0).unwrap();
    for _ in 0..5 {
        assert_eq!(circuit.solve(1000.0).unwrap(), first);
    }
}

#[test]
fn sweeps_are_reproducible() {
    let circuit = build(&[0, 1, 2, 3, 4]);
    let freqs = [50.0, 500.0, 5000.0];
    let first = circuit.sweep(&freqs).unwrap();
    let second = circuit.sweep(&freqs).unwrap();
    assert_eq!(first, second);
}

#[test]
fn transfer_curve_matches_point_by_point_solves() {
    let circuit = build(&[0, 1, 2, 3, 4]);
    let freqs = [100.0, 1000.0, 10_000.0];
    let curve = circuit
        .transfer_curve(&freqs, NodeId(3), NodeId(1))
        .unwrap();
    for (freq, value) in freqs.iter().zip(curve.values()) {
        let direct = circuit
            .voltage_transfer(*freq, NodeId(3), NodeId(1))
            .unwrap();
        assert_eq!(direct, *value);
    }
}
