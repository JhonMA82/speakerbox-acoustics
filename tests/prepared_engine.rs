//! Equivalencia de optimizaciones y contratos de integración; sin oráculos nuevos.
use num_complex::Complex64 as C;
use speakerbox_acoustics::{Air, Bandpass, Design, Driver, EndCorrection, Flare, Port, Segment};
use std::sync::Arc;
fn driver() -> Driver {
    Driver::from_ui_units(350., 4e-4, 20., 6., 18., 4., 1., 1)
}
#[test]
fn exact_constant_tube_matches_independent_rk4() {
    let air = Air::default();
    for area in [1e-4, 0.01, 1.] {
        for length in [0.001, 0.1, 2.] {
            for f in [1., 50., 1000., 10000.] {
                let exact = Segment::new(Flare::Cylindrical, length, area, area).matrix(&air, f);
                // Custom fuerza la ruta RK4 aun con área constante.
                let rk = Segment::new(Flare::Custom(Arc::new(move |_| area)), length, area, area)
                    .matrix(&air, f);
                let z0 = air.rho * air.c / area;
                assert!((exact.a - rk.a).norm() < 3e-6);
                assert!((exact.b - rk.b).norm() / z0 < 3e-6);
                assert!((exact.c - rk.c).norm() * z0 < 3e-6);
                assert!((exact.d - rk.d).norm() < 3e-6);
                assert!((exact.a * exact.d - exact.b * exact.c - C::new(1., 0.)).norm() < 1e-12);
            }
        }
    }
}
#[test]
fn prepared_sweep_and_native_ports_preserve_fields() {
    let port = Port::new(0.005, 0.1, EndCorrection::FlangedBoth);
    let bp = Bandpass::new_bp6(
        driver(),
        0.02,
        0.01,
        port,
        Port::new(0.004, 0.12, EndCorrection::None),
    );
    let d = Design::Bandpass(bp.clone());
    let prep = d.prepare().unwrap();
    for f in [20., 50., 100., 200.] {
        let a = d.try_response_at(f).unwrap();
        let b = prep.try_response_at(f).unwrap();
        assert_eq!(a.spl_db, b.spl_db);
        assert_eq!(a.ze_mag, b.ze_mag);
        let details = bp.try_solution_at(f).unwrap();
        let losses = details.input_current_a.norm_sqr() * (bp.driver.re + bp.rg)
            + (details.driver_volume_velocity / bp.driver.sd).norm_sqr() * bp.driver.rms;
        assert!(
            (details.electrical_input_power_w - losses - details.acoustic_network_power_w).abs()
                < 1e-10
        );

        assert!(
            (details.front_volume_velocity.norm() / port.area - details.point.front_port_velocity)
                .abs()
                < 1e-10
        );
        assert!((details.reduced_pressure_pa.arg().to_degrees() - a.phase_deg).abs() < 1e-10);
        assert!(
            (details.input_current_a * details.input_impedance - C::new(bp.eg, 0.)).norm() < 1e-12
        );
        assert!(d.try_reference_impedance_at(f).unwrap().is_none());
        if let speakerbox_acoustics::design::TopologyPoint::Bandpass(p) =
            d.try_topology_response_at(f).unwrap()
        {
            assert_eq!(p.rear_port_velocity, details.point.rear_port_velocity);
        } else {
            panic!("topología incorrecta");
        }
    }
}
#[test]
fn parametric_change_requires_a_fresh_borrow_and_recalculates() {
    let p = Port::new(0.005, 0.1, EndCorrection::None);
    let mut d = Design::Bandpass(Bandpass::new_bp4(driver(), 0.02, 0.01, p));
    let first = d.prepare().unwrap().try_response_at(50.).unwrap();
    if let Design::Bandpass(b) = &mut d {
        b.front_volume_m3 *= 2.;
    }
    let second = d.prepare().unwrap().try_response_at(50.).unwrap();
    assert!((first.spl_db - second.spl_db).abs() > 0.1);
}
#[test]
fn group_delay_restores_input_order_and_handles_duplicates() {
    let d = Design::System(speakerbox_acoustics::System::new(Air::default(), driver()));
    let sorted = d.response(&[20., 30., 50.]);
    let shuffled = d.response(&[50., 20., 30., 30.]);
    assert_eq!(sorted[2].group_delay_ms, shuffled[0].group_delay_ms);
    assert_eq!(sorted[1].group_delay_ms, shuffled[2].group_delay_ms);
    assert_eq!(shuffled[2].group_delay_ms, shuffled[3].group_delay_ms);
}
#[test]
fn sealed_offset_and_complex_output_polarity_are_consistent() {
    use speakerbox_acoustics::OffsetHorn;
    let mut od = OffsetHorn::new_4seg_od(
        driver(),
        vec![
            Segment::new(Flare::Cylindrical, 0.2, 0.02, 0.02),
            Segment::conical(0.5, 0.02, 0.05),
        ],
    );
    od.rear_chamber_m3 = Some(0.03);
    let a = od.try_solution_at(50.).unwrap();
    assert!(a.rear_port_volume_velocity.is_none());
    assert!(a.rear_passive_volume_velocity.is_none());
    od.polarity = -1.;
    let b = od.try_solution_at(50.).unwrap();
    assert!((a.reduced_pressure_pa + b.reduced_pressure_pa).norm() < 1e-10);
    assert!((a.point.efficiency_pct - b.point.efficiency_pct).abs() < 1e-10);
}
#[test]
fn passive_impedance_parallel_resonance_is_never_a_short() {
    use speakerbox_acoustics::element::{parallel, try_parallel};
    for e in [1e-2, 1e-6, 1e-10] {
        let z = parallel(C::new(e, 10.), C::new(e, -10.));
        assert!(z.re > 1. / e);
        assert!(z.im.abs() < 1e-8 * z.re);
    }
    assert!(try_parallel(C::new(0., 10.), C::new(0., -10.)).is_err());
}
#[test]
fn radiation_angle_does_not_change_baffled_piston_load() {
    use speakerbox_acoustics::radiation::piston_radiation_impedance;
    let a = Air::default();
    let z = piston_radiation_impedance(&a, 0.02, 100., std::f64::consts::TAU);
    for factor in [4., 2., 1., 0.5] {
        assert_eq!(
            z,
            piston_radiation_impedance(&a, 0.02, 100., factor * std::f64::consts::PI)
        );
    }
}
#[test]
fn impedance_phase_diagnostic_uses_circular_error() {
    use speakerbox_acoustics::validation::validate_impedance_phase;
    use speakerbox_acoustics::OraclePoint;
    let s = speakerbox_acoustics::System::new(Air::default(), driver());
    let p = s.try_response_at(50.).unwrap();
    let o = OraclePoint {
        freq_hz: 50.,
        spl_db: p.spl_db,
        ze_mag: p.ze_mag,
        xd_mm: p.displacement_mm,
        ze_phase_deg: p.ze_phase_deg + 359.,
    };
    let r = validate_impedance_phase(&s, &[o]);
    assert_eq!(r.n, 1);
    assert!((r.rmse_deg - 1.).abs() < 1e-10);
    assert!(validate_impedance_phase(&s, &[]).rmse_deg.is_nan());
}
