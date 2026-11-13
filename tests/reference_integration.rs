use num_complex::Complex64 as C;
use speakerbox_acoustics::reference_design::{sealed_alignment, uniform_transmission_line};
use speakerbox_acoustics::{Air, Driver, Flare, Segment};
#[test]
fn alignments_satisfy_half_power_polynomial() {
    for q in [1. / 2_f64.sqrt(), 1. / 3_f64.sqrt(), 0.957] {
        let a = sealed_alignment(50., 0.3, 0.04, q).unwrap();
        let x = a.f3_hz / a.resonance_hz;
        let gain = x.powi(4) / ((1. - x * x).powi(2) + x * x / (q * q));
        assert!((gain - 0.5).abs() < 1e-12);
        assert!((0.3 * (1. + 0.04 / a.volume_m3).sqrt() - q).abs() < 1e-12);
    }
}
#[test]
fn darcy_matrix_and_independent_dissipation() {
    let air = Air::default();
    let rf = 300.;
    let area = 0.02;
    let l = 0.8;
    for f in [10., 100., 1000.] {
        let s = Segment::new(Flare::Cylindrical, l, area, area).with_flow_resistivity(rf);
        let w = std::f64::consts::TAU * f;
        let root = C::new(1., -rf / (w * air.rho)).sqrt();
        let k = w / air.c * root;
        let z = air.rho * air.c / area * root;
        let m = s.matrix(&air, f);
        assert!((m.a - (k * l).cos()).norm() < 1e-6);
        assert!((m.b - C::new(0., -1.) * z * (k * l).sin()).norm() / z.norm() < 1e-6);
        let p = C::new(2., 1.);
        let u = p / z;
        let (p2, u2) = m.apply(p, u);
        let loss = s.dissipated_power_w(&air, f, p, u).unwrap();
        let flux = (p * u.conj()).re - (p2 * u2.conj()).re;
        assert!(loss > 0.);
        assert!((loss - flux).abs() < 1e-10 + 1e-6 * loss);
    }
}
#[test]
fn zero_resistivity_is_exactly_compatible() {
    let a = Air::default();
    let s = Segment::new(Flare::Cylindrical, 1., 0.02, 0.02);
    assert_eq!(
        s.matrix(&a, 100.),
        s.clone().with_flow_resistivity(0.).matrix(&a, 100.)
    );
    assert!(s.with_flow_resistivity(-1.).validate().is_err());
}
#[test]
fn offset_tl_has_analytic_modal_coupling_and_energy() {
    let d = Driver::from_ui_units(350., 4e-4, 20., 6., 18., 4., 1., 1);
    for x in [0., 0.25, 1. / 3., 0.49] {
        for rf in [0., 300.] {
            let t = uniform_transmission_line(d, 1.2, 0.02, x, rf).unwrap();
            for f in [30., 70., 210.] {
                let s = t.try_solution_at(f).unwrap();
                let r = t.try_energy_at(f).unwrap();
                assert!(r.balance_error_w().abs() < 1e-9 + 1e-6 * r.source_active_w);
                if rf == 0. {
                    let z0 = t.air.rho * t.air.c / 0.02;
                    let k = t.air.wave_number(f);
                    let zr = speakerbox_acoustics::radiation::piston_radiation_impedance(
                        &t.air,
                        0.02,
                        f,
                        t.solid_angle,
                    );
                    let expected = (k * 1.2 * x).cos()
                        / (C::new((k * 1.2).cos(), 0.)
                            + C::new(0., 1.) * zr / z0 * (k * 1.2).sin());
                    assert!(
                        (s.mouth_volume_velocity / s.driver_volume_velocity - expected).norm()
                            < 1e-8
                    );
                }
            }
        }
    }
}
