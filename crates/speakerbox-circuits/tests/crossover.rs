//! Integración de los prototipos de crossover con el resto del crate.

use num_complex::Complex64;
use speakerbox_circuits::{
    l_pad, optimize, parse_cir, synthesize, to_cir, zobel, Circuit, Component, DesignVariable,
    FilterFamily, FilterKind, FilterResponse, FilterSpec, NodeId, OptimizationOptions,
    OptimizationTarget, VariableField, VoltageSource,
};

fn butterworth_low_pass(order: usize) -> FilterSpec {
    FilterSpec {
        family: FilterFamily::Butterworth,
        kind: FilterKind::LowPass,
        order,
        cutoff_hz: 1000.0,
        impedance_ohm: 8.0,
    }
}

#[test]
fn synthesized_ladder_round_trips_through_cir() {
    let ladder = synthesize(&butterworth_low_pass(2)).unwrap();
    let text = to_cir(&ladder.circuit).unwrap();
    let reparsed = parse_cir(&text).unwrap();
    for freq in [100.0, 500.0, 1000.0, 5000.0] {
        let a = ladder
            .circuit
            .voltage_transfer(freq, ladder.output_node, ladder.source_node)
            .unwrap();
        let b = reparsed
            .voltage_transfer(freq, ladder.output_node, ladder.source_node)
            .unwrap();
        assert!((a - b).norm() < 1e-15, "f={freq}: {a:?} vs {b:?}");
    }
}

#[test]
fn l_pad_zero_db_is_a_wire() {
    let pad = l_pad(0.0, 8.0).unwrap();
    assert!(pad.is_bypass());
    let mut circuit = Circuit::new(2);
    circuit
        .push_source(VoltageSource::new(
            NodeId(1),
            NodeId::GROUND,
            Complex64::new(1.0, 0.0),
        ))
        .unwrap();
    let node = pad.insert(&mut circuit, NodeId(1), NodeId(2)).unwrap();
    assert_eq!(node, NodeId(1));
    circuit
        .push_component(Component::load(
            node,
            NodeId::GROUND,
            Complex64::new(8.0, 0.0),
        ))
        .unwrap();
    assert_eq!(
        circuit.voltage_transfer(1000.0, node, NodeId(1)).unwrap(),
        Complex64::new(1.0, 0.0)
    );
    assert_eq!(
        circuit.input_impedance(1000.0).unwrap(),
        Complex64::new(8.0, 0.0)
    );
}

#[test]
fn zobel_compensation_is_visible_in_the_input_impedance() {
    let (re, le) = (6.4, 1.5e-3);
    let mut uncompensated = Circuit::new(2);
    uncompensated
        .push_source(VoltageSource::new(
            NodeId(1),
            NodeId::GROUND,
            Complex64::new(1.0, 0.0),
        ))
        .unwrap();
    uncompensated
        .push_component(Component::inductor(NodeId(1), NodeId::GROUND, le, re))
        .unwrap();

    let mut compensated = Circuit::new(3);
    compensated
        .push_source(VoltageSource::new(
            NodeId(1),
            NodeId::GROUND,
            Complex64::new(1.0, 0.0),
        ))
        .unwrap();
    compensated
        .push_component(Component::inductor(NodeId(1), NodeId::GROUND, le, re))
        .unwrap();
    zobel(re, le)
        .unwrap()
        .insert(&mut compensated, NodeId(1), NodeId(2))
        .unwrap();

    let spread = |circuit: &Circuit| {
        let mut low = f64::INFINITY;
        let mut high = 0.0_f64;
        for index in 0..20 {
            let freq = 200.0 * 100.0_f64.powf(index as f64 / 19.0);
            let magnitude = circuit.input_impedance(freq).unwrap().norm();
            low = low.min(magnitude);
            high = high.max(magnitude);
        }
        high - low
    };
    assert!(spread(&compensated) < 0.3 * spread(&uncompensated));
}

#[test]
fn optimizer_improves_a_detuned_low_pass() {
    let reference = synthesize(&butterworth_low_pass(2)).unwrap();
    let mut circuit = reference.circuit.clone();
    let wrong = Component::inductor(NodeId(2), NodeId(3), 1.5e-3, 0.0);
    circuit.replace_component(1, wrong.kind).unwrap();
    let filter =
        FilterResponse::new(FilterFamily::Butterworth, FilterKind::LowPass, 2, 1000.0).unwrap();
    let freqs: Vec<f64> = (0..40)
        .map(|index| 200.0 * 50.0_f64.powf(index as f64 / 39.0))
        .collect();
    let goals: Vec<f64> = freqs
        .iter()
        .map(|freq| {
            20.0 * filter.transfer(*freq).unwrap().norm().max(1e-15).log10()
                - 20.0 * 2.0_f64.log10()
        })
        .collect();
    let target = OptimizationTarget::new(freqs, goals, Vec::new()).unwrap();
    let variables = vec![
        DesignVariable {
            component: 1,
            field: VariableField::Inductance,
            min: 1e-6,
            max: 1.0,
        },
        DesignVariable {
            component: 2,
            field: VariableField::Capacitance,
            min: 1e-9,
            max: 1e-3,
        },
    ];
    let report = optimize(
        &circuit,
        &variables,
        &target,
        reference.output_node,
        reference.source_node,
        &OptimizationOptions::default(),
    )
    .unwrap();
    assert!(report.final_cost < report.initial_cost);
    assert!(report.rmse_db < 0.1, "rmse={}", report.rmse_db);
    assert!(report
        .values
        .iter()
        .all(|value| value.is_finite() && *value > 0.0));
}
