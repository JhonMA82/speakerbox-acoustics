use num_complex::Complex64 as C;
use speakerbox_acoustics::*;
use std::f64::consts::TAU;
fn driver() -> Driver {
    Driver::from_ui_units(350., 4e-4, 20., 6., 18., 4., 1., 1)
}
fn line(pos: f64, rf: f64) -> TransmissionLine {
    TransmissionLine::at_position(
        driver(),
        vec![Segment::new(Flare::Cylindrical, 1., 0.02, 0.02).with_flow_resistivity(rf)],
        pos,
    )
    .unwrap()
}
#[test]
fn quarter_wave_modes_and_driver_positions_are_analytic() {
    for pos in [0., 0.25, 0.33, 0.49, 1.] {
        let mut t = line(pos, 0.);
        t.termination = LineTermination::PressureRelease;
        for f in [20., 50., 100., 200., 300.] {
            let p = t.try_response_at(f).unwrap();
            let k = TAU * f / t.air.c;
            let z0 = t.air.rho * t.air.c / 0.02;
            let yl = C::new(0., (k * pos).tan() / z0);
            let yr = C::new(0., -1. / (z0 * (k * (1. - pos)).tan()));
            if pos < 1. {
                let z = C::new(1., 0.) / (yl + yr);
                assert!(
                    (z - C::new(p.summary.za_real, p.summary.za_imag)).norm()
                        < z.norm() * 1e-8 + 1e-6
                );
                assert!(
                    (p.upstream_volume_velocity_m3_s + p.downstream_volume_velocity_m3_s
                        - p.driver_volume_velocity_m3_s)
                        .norm()
                        < 1e-12
                );
                assert!(p.mouth_pressure_pa.norm() < 1e-8);
            }
        }
    }
    let t = line(0., 0.);
    // For a pressure-release tube, the closed-end input impedance has poles at
    // odd quarter waves (resonances of the open/closed air column).
    for mode in [1., 3., 5.] {
        let f = t.air.c / 4. * mode;
        let k = TAU * f / t.air.c;
        assert!(k.cos().abs() < 1e-14);
        let mut open = t.clone();
        open.termination = LineTermination::PressureRelease;
        let near = open.try_response_at(f * (1. + 1e-4)).unwrap();
        let closer = open.try_response_at(f * (1. + 1e-5)).unwrap();
        assert!(closer.summary.za_imag.abs() > near.summary.za_imag.abs() * 9.9);
        assert!(
            closer.driver_volume_velocity_m3_s.norm()
                < near.driver_volume_velocity_m3_s.norm() * 0.102
        );
    }
}
#[test]
fn closed_boundary_zones_folds_and_energy() {
    for term in [
        LineTermination::BaffledPiston,
        LineTermination::Closed,
        LineTermination::Resistance(20000.),
    ] {
        let segments = vec![
            Segment::conical(0.25, 0.02, 0.03).with_flow_resistivity(100.),
            Segment::conical(0.35, 0.03, 0.04).with_flow_resistivity(2000.),
            Segment::new(Flare::Cylindrical, 0.4, 0.04, 0.04),
        ];
        let mut t = TransmissionLine::new(driver(), segments, 1);
        t.termination = term;
        t.folds.push(Fold {
            after_segment: 1,
            acoustic_mass: 50.,
            resistance: 100.,
        });
        for f in [10., 50., 100., 400.] {
            let p = t.try_response_at(f).unwrap();
            let e = t.try_energy_at(f).unwrap();
            assert!(e.acoustic_losses.total_w() > 0.);
            assert!(
                e.balance_error_w().abs() < 1e-8 + 1e-5 * e.source_active_w,
                "{}",
                e.balance_error_w()
            );
            if matches!(term, LineTermination::Closed) {
                assert!(p.mouth_volume_velocity_m3_s.norm() < 1e-12);
            }
        }
        assert_eq!(
            Design::from(t).try_sweep_log(20., 200., 10).unwrap().len(),
            10
        );
    }
}
#[test]
fn subdivision_reciprocity_polarity_and_profile_are_preserved() {
    let a = vec![Segment::conical(1., 0.02, 0.08).with_flow_resistivity(1000.)];
    let t = TransmissionLine::at_position(driver(), a.clone(), 0.33).unwrap();
    assert!((t.segments[0].s2() - a[0].area(0.33)).abs() < 1e-14);
    let mut reversed = t.clone();
    reversed.polarity = -1.;
    for f in [20., 100., 300.] {
        let p = t.try_response_at(f).unwrap();
        let q = reversed.try_response_at(f).unwrap();
        assert!((p.pressure_estimate_pa + q.pressure_estimate_pa).norm() < 1e-10);
        for s in &t.segments {
            let m = s.try_matrix(&t.air, f).unwrap();
            assert!((m.a * m.d - m.b * m.c - C::new(1., 0.)).norm() < 1e-6);
        }
        let whole = a[0].try_matrix(&t.air, f).unwrap();
        let split = t.segments[0]
            .try_matrix(&t.air, f)
            .unwrap()
            .then(t.segments[1].try_matrix(&t.air, f).unwrap());
        assert!((whole.b - split.b).norm() / whole.b.norm() < 1e-6);
    }
}
