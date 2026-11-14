//! Contratos de error: toda entrada inválida devuelve un valor tipado.

use num_complex::Complex64;
use speakerbox_circuits::{
    parse_cir, to_cir, Circuit, CircuitError, ComplexCurve, Component, LoadImpedance, NodeId,
    VoltageSource,
};

fn one_volt() -> VoltageSource {
    VoltageSource::new(NodeId(1), NodeId::GROUND, Complex64::new(1.0, 0.0))
}

#[test]
fn empty_and_sourceless_circuits_are_rejected() {
    let mut empty = Circuit::new(2);
    assert_eq!(empty.solve(1000.0).unwrap_err(), CircuitError::EmptyCircuit);

    empty
        .push_component(Component::resistor(NodeId(1), NodeId::GROUND, 10.0))
        .unwrap();
    assert_eq!(empty.solve(1000.0).unwrap_err(), CircuitError::NoSource);
    assert_eq!(
        empty.input_impedance(1000.0).unwrap_err(),
        CircuitError::SourceCount { found: 0 }
    );
}

#[test]
fn invalid_frequencies_never_reach_the_solver() {
    let mut circuit = Circuit::new(2);
    circuit.push_source(one_volt()).unwrap();
    circuit
        .push_component(Component::resistor(NodeId(1), NodeId::GROUND, 10.0))
        .unwrap();
    for freq in [0.0, -1000.0] {
        assert_eq!(
            circuit.solve(freq).unwrap_err(),
            CircuitError::InvalidFrequency(freq)
        );
    }
    assert!(matches!(
        circuit.solve(f64::NAN).unwrap_err(),
        CircuitError::InvalidFrequency(found) if found.is_nan()
    ));
    assert!(matches!(
        circuit.solve(f64::INFINITY).unwrap_err(),
        CircuitError::InvalidFrequency(found) if found.is_infinite()
    ));
}

#[test]
fn structurally_broken_networks_are_singular() {
    let mut floating = Circuit::new(4);
    floating.push_source(one_volt()).unwrap();
    floating
        .push_component(Component::resistor(NodeId(1), NodeId::GROUND, 10.0))
        .unwrap();
    floating
        .push_component(Component::capacitor(NodeId(2), NodeId(3), 1e-6))
        .unwrap();
    assert_eq!(
        floating.solve(1000.0).unwrap_err(),
        CircuitError::Singular { freq_hz: 1000.0 }
    );
}

#[test]
fn curve_loads_outside_their_band_are_errors() {
    let curve = ComplexCurve::new(
        vec![100.0, 1000.0],
        vec![Complex64::new(8.0, 1.0), Complex64::new(12.0, 2.0)],
    )
    .unwrap();
    let mut circuit = Circuit::new(2);
    circuit.push_source(one_volt()).unwrap();
    circuit
        .push_component(Component::load_curve(NodeId(1), NodeId::GROUND, curve))
        .unwrap();
    assert_eq!(
        circuit.solve(99.0).unwrap_err(),
        CircuitError::CurveOutOfRange { freq_hz: 99.0 }
    );
    assert_eq!(
        circuit.solve(1000.1).unwrap_err(),
        CircuitError::CurveOutOfRange { freq_hz: 1000.1 }
    );
    assert!(circuit.solve(500.0).is_ok());
}

#[test]
fn zero_voltage_source_cannot_define_an_input_impedance() {
    let mut circuit = Circuit::new(2);
    circuit
        .push_source(VoltageSource::new(
            NodeId(1),
            NodeId::GROUND,
            Complex64::new(0.0, 0.0),
        ))
        .unwrap();
    circuit
        .push_component(Component::resistor(NodeId(1), NodeId::GROUND, 10.0))
        .unwrap();
    assert_eq!(
        circuit.input_impedance(1000.0).unwrap_err(),
        CircuitError::ZeroReference {
            freq_hz: 1000.0,
            node: NodeId(1)
        }
    );
}

#[test]
fn constructor_errors_are_typed_and_do_not_mutate_the_circuit() {
    let mut circuit = Circuit::new(2);
    assert_eq!(
        circuit
            .push_component(Component::resistor(NodeId(9), NodeId::GROUND, 1.0))
            .unwrap_err(),
        CircuitError::UnknownNode {
            node: NodeId(9),
            nodes: 2
        }
    );
    assert_eq!(
        circuit
            .push_component(Component::resistor(NodeId::GROUND, NodeId::GROUND, 1.0))
            .unwrap_err(),
        CircuitError::SelfLoop {
            node: NodeId::GROUND
        }
    );
    assert!(circuit.components().is_empty());
    assert_eq!(
        circuit
            .push_source(VoltageSource::new(
                NodeId(1),
                NodeId::GROUND,
                Complex64::new(f64::NAN, 0.0)
            ))
            .unwrap_err(),
        CircuitError::InvalidSource {
            index: 0,
            field: "la tensión debe ser finita"
        }
    );
    assert!(circuit.sources().is_empty());
}

#[test]
fn lonely_single_node_cannot_be_solved() {
    let mut circuit = Circuit::new(1);
    circuit.push_source(one_volt()).unwrap_err();
    circuit
        .push_component(Component::resistor(NodeId(1), NodeId::GROUND, 10.0))
        .unwrap_err();
    assert!(circuit.components().is_empty());
}

#[test]
fn garbage_netlists_never_panic() {
    let cases = [
        "",
        "   \n\t\n",
        "R1 1 0",
        "R1 1 0 abc",
        "R1 1 0 1 extra extra",
        "L1 1 0 1m 0 -1",
        "Z1 1 0 1 2 3",
        "C1 1 0 1e999",
        "V1 1 0 1\n* comentario\n",
        "V1 1 0 AC\n",
        "X1 1 0 1",
        "I1 1 0 1",
        "\u{0}R1 1 0 1",
    ];
    for case in cases {
        let _ = parse_cir(case);
    }
}

#[test]
fn netlist_errors_carry_line_numbers() {
    let errors = parse_cir("V1 1 0 1\nR1 1 0 nope\nR2 1 0 10\n").unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].line, 2);
}

#[test]
fn serialization_refuses_unsupported_values() {
    let curve = ComplexCurve::new(
        vec![100.0, 1000.0],
        vec![Complex64::new(8.0, 1.0), Complex64::new(12.0, 2.0)],
    )
    .unwrap();
    let mut circuit = Circuit::new(2);
    circuit.push_source(one_volt()).unwrap();
    circuit
        .push_component(Component::load_curve(NodeId(1), NodeId::GROUND, curve))
        .unwrap();
    assert!(matches!(
        to_cir(&circuit).unwrap_err(),
        CircuitError::NotSerializable { index: 0, .. }
    ));
}

#[test]
fn load_contract_rejects_reactive_and_negative_capacitance() {
    let mut circuit = Circuit::new(2);
    assert!(circuit
        .push_component(Component::new(
            NodeId(1),
            NodeId::GROUND,
            speakerbox_circuits::ComponentKind::Load(LoadImpedance::Constant(Complex64::new(
                -5.0, 1.0
            )))
        ))
        .is_err());
    assert!(circuit
        .push_component(Component::capacitor(NodeId(1), NodeId::GROUND, -1e-6))
        .is_err());
}
