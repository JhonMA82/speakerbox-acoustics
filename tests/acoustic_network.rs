use num_complex::Complex64 as C;
use speakerbox_acoustics::energy::active_power;
use speakerbox_acoustics::{
    network::*, Air, Bandpass, Chamber, Driver, EndCorrection, Flare, Port, Segment, System,
    TappedHorn,
};
use std::f64::consts::TAU;

fn cylinder(length: f64, area: f64) -> Segment {
    Segment::new(Flare::Cylindrical, length, area, area)
}
fn driver() -> Driver {
    Driver::new(0.02112, 0.000591, 0.03143, 5.9, 10.65, 4.11, 0.001)
}
fn duct(inlet: usize, outlet: usize, segment: Segment) -> AcousticDuct {
    AcousticDuct {
        inlet,
        outlet,
        conduit: Conduit::Distributed(segment),
    }
}
fn boundary(node: usize, boundary: Boundary) -> AcousticBoundary {
    AcousticBoundary { node, boundary }
}
fn opening(node: usize, area_m2: f64) -> AcousticBoundary {
    boundary(
        node,
        Boundary::Opening {
            area_m2,
            load_scale: 1.0,
        },
    )
}
fn drive(front: usize, rear: usize, driver: Driver) -> NetworkDriver {
    NetworkDriver {
        driver,
        front,
        rear,
        voltage_rms: C::from(2.83),
        source_resistance_ohm: 0.0,
        solid_angle: TAU,
        mechanical_damping: None,
    }
}
fn source(node: usize, flow: f64) -> AcousticSource {
    AcousticSource {
        node,
        flow_m3_s: C::from(flow),
    }
}
fn close(a: C, b: C, tolerance: f64) {
    assert!(
        (a - b).norm() <= tolerance * (a.norm().max(b.norm()) + 1e-12),
        "{a} vs {b}"
    );
}
fn scalar(a: f64, b: f64, tolerance: f64) {
    close(a.into(), b.into(), tolerance);
}
fn balance(s: &DrivenNetworkState) {
    assert!(s.energy.balance_error_w().abs() < 2e-7 * s.energy.source_active_w);
    assert!(s.energy.acoustic_balance_error_w().abs() < 2e-7 * s.energy.source_active_w);
}

#[test]
fn bp4_matches_the_existing_independent_solver() {
    let d = driver();
    let air = Air::default();
    let port = Port::new(0.004, 0.14, EndCorrection::Inner(0.82));
    let bp = Bandpass::new_bp4(d, 0.022, 0.016, port);
    let net = AcousticNetwork {
        nodes: 3,
        ducts: vec![duct(0, 2, cylinder(port.effective_length(), port.area))],
        boundaries: vec![
            boundary(0, Boundary::Chamber(Chamber::new(0.016))),
            boundary(1, Boundary::Chamber(Chamber::new(0.022))),
            opening(2, port.area),
        ],
    };
    for f in [14., 35., 66., 100., 155., 230.] {
        let old = bp.try_solution_at(f).unwrap();
        let n = net.solve_driver(&air, f, &drive(0, 1, d)).unwrap();
        close(n.electrical_impedance_ohm, old.input_impedance, 2e-10);
        close(n.diaphragm_flow_m3_s, old.driver_volume_velocity, 2e-10);
        close(
            n.acoustic.boundary_flow_m3_s[2],
            old.front_volume_velocity,
            2e-10,
        );
        let compact = n.acoustic.boundary_flow_m3_s[2] * (air.rho * TAU * f / bp.solid_angle);
        close(compact, old.reduced_pressure_pa, 2e-10);
        scalar(
            n.peak_displacement_m * 1000.0,
            old.point.displacement_mm,
            2e-10,
        );
        scalar(
            20.0 * (compact.norm() / air.p_ref).log10(),
            old.point.spl_db,
            2e-10,
        );
        scalar(
            n.energy.acoustic_input_w,
            old.acoustic_network_power_w,
            2e-10,
        );
        balance(&n);
    }
}

#[test]
fn vented_box_matches_legacy_pressure_release_and_observer() {
    let d = driver();
    let air = Air::default();
    let port = Port::new(0.004, 0.14, EndCorrection::Inner(1.5));
    let chamber = Chamber::new(0.03);
    let mut vent = System::new(air, d);
    vent.rear_chamber = Some(chamber);
    vent.port = Some(port);
    let net = AcousticNetwork {
        nodes: 3,
        ducts: vec![AcousticDuct {
            inlet: 1,
            outlet: 2,
            conduit: Conduit::Inertance(port),
        }],
        boundaries: vec![
            opening(0, d.sd),
            boundary(1, Boundary::Chamber(chamber)),
            boundary(2, Boundary::PressureRelease),
        ],
    };
    for f in [15., 35., 68., 105., 155.] {
        let p = vent.try_response_at(f).unwrap();
        let n = net.solve_driver(&air, f, &drive(0, 1, d)).unwrap();
        close(
            n.electrical_impedance_ohm,
            C::from_polar(p.ze_mag, p.ze_phase_deg.to_radians()),
            1e-10,
        );
        scalar(n.peak_displacement_m * 1000.0, p.displacement_mm, 1e-10);
        let port_flow = n.acoustic.boundary_flow_m3_s[2];
        scalar(
            port_flow.norm() / port.area,
            p.port_velocity.unwrap(),
            1e-10,
        );
        let z = speakerbox_acoustics::piston_radiation_impedance(&air, port.area, f, TAU);
        let observed =
            n.pressure_estimate_pa.unwrap() + C::i() * port_flow * (z.re * air.z0() / TAU).sqrt();
        close(
            observed,
            C::from_polar(
                air.p_ref * 10_f64.powf(p.spl_db / 20.0),
                (p.phase_deg + 90.0).to_radians(),
            ),
            1e-10,
        );
        let original_energy = vent.try_energy_at(f).unwrap();
        scalar(
            n.energy.source_active_w,
            original_energy.source_active_w,
            1e-10,
        );
        scalar(
            n.energy.acoustic_input_w,
            original_energy.acoustic_input_w,
            1e-10,
        );
        balance(&n);
    }
}

#[test]
fn tapped_horn_matches_at_equal_load_and_mass() {
    let d = driver();
    let air = Air::default();
    let segments = vec![
        Segment::conical(0.4, 0.02, 0.03),
        Segment::conical(0.55, 0.03, 0.04),
        Segment::conical(0.9, 0.04, 0.06),
        Segment::conical(0.2, 0.06, 0.07),
    ];
    let mut th = TappedHorn::new_4seg_th(d, segments.clone());
    th.solid_angle = TAU;
    th.mouth_radiation_scale = 1.0;
    th.mouth_radiation_area_factor = 1.0;
    th.spl_offset_db = 0.0;
    let net = AcousticNetwork {
        nodes: 5,
        ducts: segments
            .into_iter()
            .enumerate()
            .map(|(i, s)| duct(i, i + 1, s))
            .collect(),
        boundaries: vec![opening(4, 0.07)],
    };
    let mut equivalent = d;
    equivalent.mmd += d.air_load_mass(&air);
    for f in [18., 34., 51., 77., 109., 153.] {
        let p = th.try_response_at(f).unwrap();
        let n = net
            .solve_driver(&air, f, &drive(th.tap_a, th.tap_b, equivalent))
            .unwrap();
        close(
            n.electrical_impedance_ohm,
            C::from_polar(p.ze_mag, p.ze_phase_deg.to_radians()),
            1e-6,
        );
        scalar(n.peak_displacement_m * 1000.0, p.displacement_mm, 1e-6);
        scalar(n.spl_db.unwrap(), p.spl_db, 1e-6);
        let z = speakerbox_acoustics::piston_radiation_impedance(&air, 0.07, f, TAU);
        close(
            n.acoustic.boundary_flow_m3_s[0],
            th.debug_pressures(f).unwrap().last().unwrap() / z,
            1e-6,
        );
        scalar(
            n.energy.acoustic_input_w,
            th.try_energy_at(f).unwrap().acoustic_input_w,
            1e-6,
        );
        balance(&n);
    }
}

#[test]
fn branch_kcl_reciprocity_closed_ends_and_node_permutation() {
    let air = Air::default();
    let f = 65.;
    let duct_a = cylinder(0.40, 0.003);
    let duct_b = cylinder(0.53, 0.002);
    let net = AcousticNetwork {
        nodes: 3,
        ducts: vec![duct(0, 1, duct_a.clone()), duct(0, 2, duct_b.clone())],
        boundaries: vec![boundary(1, Boundary::Resistance { pa_s_m3: 1e7 })],
    };
    let a = net.solve(&air, f, &[source(0, 1.0)]).unwrap();
    let b = net.solve(&air, f, &[source(1, 1.0)]).unwrap();
    close(a.pressure_pa[1], b.pressure_pa[0], 1e-11);
    close(
        a.duct_flow_m3_s[0].0 + a.duct_flow_m3_s[1].0,
        C::from(1.0),
        1e-11,
    );
    assert!(a.duct_flow_m3_s[1].1.norm() < 1e-13);
    close(a.duct_flow_m3_s[0].1, a.boundary_flow_m3_s[0], 1e-11);
    let za = duct_a
        .try_matrix(&air, f)
        .unwrap()
        .transform_impedance(C::from(1e7));
    let zb = duct_b
        .try_matrix(&air, f)
        .unwrap()
        .transform_impedance(C::new(f64::INFINITY, 0.0));
    close(a.pressure_pa[0], 1.0 / (1.0 / za + 1.0 / zb), 1e-11);
    scalar(
        active_power(a.pressure_pa[0], 1.0.into()),
        active_power(a.pressure_pa[1], a.boundary_flow_m3_s[0]),
        1e-11,
    );
    let swap = [2, 0, 1];
    let perm = AcousticNetwork {
        nodes: 3,
        ducts: vec![duct(2, 0, duct_a), duct(2, 1, duct_b)],
        boundaries: vec![boundary(0, Boundary::Resistance { pa_s_m3: 1e7 })],
    };
    let p = perm.solve(&air, f, &[source(2, 1.0)]).unwrap();
    for (i, new) in swap.into_iter().enumerate() {
        close(a.pressure_pa[i], p.pressure_pa[new], 1e-11);
    }
}

#[test]
fn half_wave_and_large_impedance_ratio_are_stable_and_degeneracies_are_errors() {
    let air = Air::default();
    let f = air.c / 2.0;
    let net = AcousticNetwork {
        nodes: 2,
        ducts: vec![duct(0, 1, cylinder(1., 0.002))],
        boundaries: vec![boundary(1, Boundary::Resistance { pa_s_m3: 1e9 })],
    };
    let p = net.solve(&air, f, &[source(0, 1e-4)]).unwrap();
    close(p.pressure_pa[1], -p.pressure_pa[0], 1e-9);
    close(p.duct_flow_m3_s[0].1, C::from(-1e-4), 1e-9);
    let bad = AcousticNetwork {
        nodes: 3,
        ..net.clone()
    };
    assert!(matches!(
        bad.solve(&air, f, &[source(0, 1.)]),
        Err(NetworkError::Disconnected(2))
    ));
    let bad = AcousticNetwork {
        nodes: 2,
        ducts: vec![duct(0, 1, cylinder(0., 0.002))],
        boundaries: vec![],
    };
    assert!(matches!(
        bad.solve(&air, f, &[source(0, 1.)]),
        Err(NetworkError::MissingReference(0))
    ));
    let closed = AcousticNetwork {
        boundaries: vec![],
        ..net
    };
    assert!(matches!(
        closed.solve(&air, f, &[source(0, 1.)]),
        Err(NetworkError::IllConditioned(_) | NetworkError::Singular)
    ));
}

#[test]
fn network_json_round_trip_checks_structure_and_preserves_solution() {
    let original = network_example();
    let parsed = topology::NetworkLayout::from_json(&original.to_json().unwrap()).unwrap();
    let d = drive(original.front, original.rear, driver());
    let air = Air::default();
    let before = original.network.solve_driver(&air, 95.0, &d).unwrap();
    let after = parsed.network.solve_driver(&air, 95.0, &d).unwrap();
    assert_eq!(before.acoustic.pressure_pa, after.acoustic.pressure_pa);
    assert_eq!(before.spl_db, after.spl_db);
    let json = original.to_json().unwrap();
    assert!(
        topology::NetworkLayout::from_json(&json.replace("\"version\":1", "\"version\":3"))
            .is_err()
    );
    assert!(
        topology::NetworkLayout::from_json(&json.replace("\"nodes\":3", "\"nodes\":4")).is_err()
    );
}
fn network_example() -> topology::NetworkLayout {
    topology::NetworkLayout::from_json(include_str!("../docs/wavebox-integration/bp4-network.json"))
        .unwrap()
}
