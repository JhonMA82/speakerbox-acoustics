use speakerbox_acoustics::{CoilModel, Driver, System};
fn driver() -> Driver {
    Driver::from_ui_units(350., 4e-4, 20., 6., 18., 4., 1., 1)
}
#[test]
fn fractional_coil_is_passive_and_has_correct_dimensions() {
    let d = driver().with_coil_model(CoilModel::SemiInductance { ke: 0.2 });
    assert_eq!(d.coil_impedance(0.).re, 6.);
    assert_eq!(d.coil_impedance(0.).im, 0.);
    for f in [1., 10., 100., 10000.] {
        let z = d.coil_impedance(f);
        let q = 0.2 * (std::f64::consts::PI * f).sqrt();
        assert!((z.re - 6. - q).abs() < 1e-12);
        assert!((z.im - q).abs() < 1e-12);
        assert!(z.re > 0.);
    }
    let mut altered = d;
    altered.le = 99.;
    assert_eq!(d.coil_impedance(100.), altered.coil_impedance(100.));
    assert!(driver()
        .with_coil_model(CoilModel::SemiInductance { ke: -1. })
        .validate()
        .is_err());
}
#[test]
fn fractional_coil_heat_is_in_energy_balance() {
    let mut s = System::new(
        speakerbox_acoustics::Air::default(),
        driver().with_coil_model(CoilModel::SemiInductance { ke: 0.3 }),
    );
    s.rg = 2.;
    for f in [10., 100., 1000.] {
        let e = s.try_energy_at(f).unwrap();
        assert!(e.coil_excess_loss_w > 0.);
        assert!((e.coil_loss_w - e.coil_dc_loss_w - e.coil_excess_loss_w).abs() < 1e-12);
        assert!(e.balance_error_w().abs() < 1e-9);
    }
}

#[test]
fn fractional_model_scales_in_banks_and_all_solvers_balance() {
    use speakerbox_acoustics::*;
    let d = driver().with_coil_model(CoilModel::SemiInductance { ke: 0.2 });
    for (wiring, factor) in [(Wiring::Parallel, 0.25), (Wiring::Series, 4.)] {
        let eq = DriverBank::new(d, 4)
            .with_wiring(wiring)
            .equivalent_driver()
            .unwrap();
        for f in [10., 1000.] {
            assert!((eq.coil_impedance(f) - d.coil_impedance(f) * factor).norm() < 1e-12);
        }
    }
    let segments = vec![Segment::new(Flare::Cylindrical, 0.3, 0.02, 0.02); 4];
    let mut od = OffsetHorn::new_4seg_od(d, segments.clone());
    od.tap = 1;
    let th = TappedHorn::new_4seg_th(d, segments);
    let bp = Bandpass::new_bp4(d, 0.02, 0.01, Port::new(0.005, 0.1, EndCorrection::None));
    for design in [Design::from(od), Design::from(th), Design::from(bp)] {
        for f in [30., 100., 300.] {
            let e = design.try_energy_at(f).unwrap();
            assert!(e.coil_excess_loss_w > 0.);
            assert!(e.balance_error_w().abs() < 1e-7 + 1e-5 * e.source_active_w);
        }
    }
    assert_eq!(driver().coil_model, CoilModel::Inductance);
}
