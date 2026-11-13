//! Regresiones físicas independientes de los oráculos Hornresp.
use num_complex::Complex64 as C;
use speakerbox_acoustics::bandpass::PortModel;
use speakerbox_acoustics::element::parallel;
use speakerbox_acoustics::radiation::piston_radiation_impedance;
use speakerbox_acoustics::{
    Air, Bandpass, Chamber, Driver, EndCorrection, Flare, Horn, OffsetHorn, PassiveRadiator, Port,
    Segment, System,
};
use std::f64::consts::{PI, TAU};
fn driver() -> Driver {
    Driver::from_ui_units(350., 4e-4, 20., 6., 18., 4., 1., 1)
}
fn horn() -> Horn {
    Horn::new(vec![Segment::new(Flare::Cylindrical, 0.2, 0.02, 0.02)])
}
fn phase_error(a: f64, b: f64) -> f64 {
    ((a - b + 180.).rem_euclid(360.) - 180.).abs()
}
#[test]
fn ac001_series_load_and_energy_balance() {
    let mut s = System::new(Air::default(), driver());
    s.horn = Some(horn());
    for r in [0., 1e4, 1e10] {
        s.horn_series = Some(r);
        for f in [10., 50., 200.] {
            let zh = s
                .horn
                .as_ref()
                .unwrap()
                .input_impedance(&s.air, f, s.solid_angle);
            let (zf, zr) = s.loads(f);
            assert!((zf - zh - C::new(r, 0.)).norm() < 1e-10 * (1. + zf.norm()));
            let u = s.driver.volume_velocity(f, s.eg, s.rg, zf + zr);
            let v = u / s.driver.sd;
            let i = (C::new(s.eg, 0.) - s.driver.bl * v) / (s.driver.coil_impedance(f) + s.rg);
            let incoming = s.eg * i.re;
            let dissipated = i.norm_sqr() * (s.driver.re + s.rg)
                + v.norm_sqr() * s.driver.rms
                + u.norm_sqr() * r;
            let p = s.try_response_at(f).unwrap();
            assert!((incoming - dissipated - p.acoustical_power_w).abs() < 1e-8 * (1. + incoming));
        }
    }
}
#[test]
fn ac001_shunts_share_the_series_horn_branch() {
    let mut s = System::new(Air::default(), driver());
    s.horn = Some(horn());
    s.horn_series = Some(10000.);
    s.horn_leak = Some(30000.);
    s.throat_chamber = Some(Chamber::new(0.002));
    let f = 100.;
    let zh = s
        .horn
        .as_ref()
        .unwrap()
        .input_impedance(&s.air, f, s.solid_angle);
    let expected = parallel(
        parallel(zh + 10000., C::new(30000., 0.)),
        s.throat_chamber.unwrap().impedance(&s.air, f),
    );
    assert!((s.loads(f).0 - expected).norm() / expected.norm() < 1e-12);
}
#[test]
fn ac003_bandpass_polarity_preserves_electrical_solution() {
    let p = Port::new(0.005, 0.1, EndCorrection::None);
    for mut b in [
        Bandpass::new_bp4(driver(), 0.02, 0.01, p),
        Bandpass::new_bp6(driver(), 0.02, 0.01, p, p),
        Bandpass::new_bp6_series(driver(), 0.02, 0.01, p, p),
    ] {
        for f in [20., 50., 200.] {
            b.polarity = 1.;
            let a = b.try_response_at(f).unwrap();
            b.polarity = -1.;
            let z = b.try_response_at(f).unwrap();
            assert!(phase_error(z.phase_deg, a.phase_deg + 180.) < 1e-8);
            assert!((z.spl_db - a.spl_db).abs() < 1e-10);
            assert!((z.ze_mag - a.ze_mag).abs() < 1e-10);
            assert!((z.efficiency_pct - a.efficiency_pct).abs() < 1e-10);
        }
    }
}
#[test]
fn ac004_rejects_unreproducible_geometry() {
    assert!(Segment::new(Flare::Cylindrical, 1., 0.02, 0.01)
        .validate()
        .is_err());
    assert!(
        Segment::new(Flare::HyperbolicExponential { t: 0.5 }, 1., 0.02, 0.01)
            .validate()
            .is_err()
    );
    assert!(Segment::new(Flare::Conical, 0., 0.02, 0.01)
        .validate()
        .is_err());
}
#[test]
fn ac004_negative_t_has_a_real_contraction_branch() {
    let s = Segment::new(Flare::HyperbolicExponential { t: -0.8 }, 1., 0.02, 0.01);
    assert!(s.validate().is_ok());
    assert!((s.area(1.) - 0.01).abs() < 1e-12);
    assert!((0..100).all(|i| s.area(i as f64 / 100.) >= s.area((i + 1) as f64 / 100.)));
}
#[test]
fn ac005_parallel_poles_and_extreme_scales() {
    let pole = parallel(C::new(0., 10.), C::new(0., -10.));
    assert!(pole.re.is_infinite());
    let z = C::new(3., 4.);
    assert!((parallel(C::new(f64::INFINITY, 0.), z) - z).norm() < 1e-12);
    for scale in [1e-200, 1., 1e200] {
        let z = C::new(scale, scale);
        let got = parallel(z, z);
        assert!((got / scale - C::new(0.5, 0.5)).norm() < 1e-12);
    }
    assert_eq!(parallel(C::new(0., 0.), z), C::new(0., 0.));
}
#[test]
fn ac006_bp6_series_low_frequency_lumped_limit() {
    let p = Port::new(0.005, 0.1, EndCorrection::FlangedBoth);
    // Frente estrecho: su corrección exterior de pistón es <0.05% de L.
    let front = Port::new(1e-8, 0.1, EndCorrection::None);
    // Vpuerto/Vcámara también pequeño: condición del límite concentrado.
    let mut b = Bandpass::new_bp6_series(driver(), 2., 1., front, p);
    b.port_model = PortModel::Pipe;
    let a = b.try_response_at(1.).unwrap();
    b.port_model = PortModel::Lumped;
    let z = b.try_response_at(1.).unwrap();
    assert!((a.ze_mag - z.ze_mag).abs() / a.ze_mag < 0.001);
    assert!(
        (a.spl_db - z.spl_db).abs() < 0.01,
        "pipe={a:?} lumped={z:?}"
    );
    assert!(phase_error(a.phase_deg, z.phase_deg) < 0.1);
}
fn od() -> OffsetHorn {
    let mut o = OffsetHorn::new_4seg_od(
        driver(),
        vec![
            Segment::new(Flare::Cylindrical, 0.2, 0.02, 0.02),
            Segment::conical(0.5, 0.02, 0.05),
        ],
    );
    o.solid_angle = TAU;
    o.mouth_radiation_scale = 1.;
    o.mouth_radiation_area_factor = 1.;
    o.rear_radiation_scale = 1.;
    o.spl_offset_db = 0.;
    o.include_air_load_mass = false;
    o
}
fn od_expected(o: &OffsetHorn, f: f64) -> (C, f64) {
    let zh = o.horn_impedance(f);
    let zr = o.rear_impedance(f);
    let mut zm = o.driver.mechanical_impedance(f);
    if o.include_air_load_mass {
        zm += C::new(0., TAU * f * o.driver.air_load_mass(&o.air));
    }
    let zc = o.driver.coil_impedance(f) + o.rg;
    let ud = C::new(o.driver.bl * o.eg * o.polarity / o.driver.sd, 0.)
        / zc
        / ((zm + o.driver.bl * o.driver.bl / zc) / (o.driver.sd * o.driver.sd) + zh + zr);
    let mut p = zh * ud;
    let mut u = p / o.path_impedance(f);
    for seg in &o.segments[o.tap..] {
        (p, u) = seg.matrix(&o.air, f).apply(p, u);
    }
    let area = o.segments.last().unwrap().s2() * o.mouth_radiation_area_factor;
    let z = piston_radiation_impedance(&o.air, area, f, o.solid_angle) * o.mouth_radiation_scale;
    let ph = u * (z.re * TAU * o.air.rho * o.air.c / (o.solid_angle * o.solid_angle)).sqrt();
    let mut total = ph;
    if o.rear_chamber_m3.is_none() {
        let z = piston_radiation_impedance(&o.air, o.driver.sd, f, o.solid_angle);
        let r = z.re * o.driver.sd / (o.air.rho * o.air.c);
        let ka = TAU * f / o.air.c * (o.driver.sd / PI).sqrt();
        total += -ud
            * (o.air.rho * TAU * f / o.solid_angle * (2. * r).sqrt() / (ka * (1. + r))
                * o.rear_radiation_scale.sqrt());
    } else {
        let pr = -ud * zr;
        for (area, z) in o
            .rear_port
            .map(|p| (p.area, p.impedance(&o.air, f)))
            .into_iter()
            .chain(o.rear_passive_radiator.map(|p| (p.sd, p.impedance(f))))
        {
            let rad = piston_radiation_impedance(&o.air, area, f, o.solid_angle);
            total += pr / z
                * (rad.re * TAU * o.air.rho * o.air.c / (o.solid_angle * o.solid_angle)).sqrt();
        }
    }
    (total, 20. * (total.norm() / o.air.p_ref).log10())
}
#[test]
fn ac002_offset_radiates_rear_port_and_passive_radiator() {
    for pr in [false, true] {
        let mut o = od();
        o.rear_chamber_m3 = Some(0.03);
        if pr {
            o.rear_passive_radiator = Some(PassiveRadiator::new(0.02, 0.0004, 0.03, 2.));
        } else {
            o.rear_port = Some(Port::new(0.005, 0.1, EndCorrection::None));
        }
        for f in [20., 50., 100.] {
            let a = o.try_response_at(f).unwrap();
            let (p, spl) = od_expected(&o, f);
            assert!((a.spl_db - spl).abs() < 1e-9, "{f}: {} vs {spl}", a.spl_db);
            assert!(phase_error(a.phase_deg, p.arg().to_degrees()) < 1e-8);
        }
    }
}
#[test]
fn ac007_offset_phase_tracks_combined_output() {
    let o = od();
    for f in [20., 50., 200.] {
        let a = o.try_response_at(f).unwrap();
        let (p, spl) = od_expected(&o, f);
        assert!((a.spl_db - spl).abs() < 1e-9);
        assert!(phase_error(a.phase_deg, p.arg().to_degrees()) < 1e-8);
    }
}
#[test]
fn lossless_suspension_qts_is_qes() {
    let mut d = driver();
    d.rms = 0.;
    assert!((d.qts(&Air::default()) - d.qes(&Air::default())).abs() < 1e-12);
}
#[test]
fn ac008_radiation_small_argument_has_no_cancellation() {
    let a = Air::default();
    let area = 0.02;
    let x = 1e-8;
    let r = (area / PI).sqrt();
    let f = x * a.c / (2. * TAU * r);
    let z = piston_radiation_impedance(&a, area, f, TAU);
    let factor = a.rho * a.c / area;
    assert!((z.re / (factor * x * x / 8.) - 1.).abs() < 1e-8, "{z:?}");
    assert!((z.im / (factor * 4. * x / (3. * PI)) - 1.).abs() < 1e-8);
}
#[test]
fn struve_h1_is_even_on_real_axis() {
    use speakerbox_acoustics::radiation::struve_h1;
    assert!((struve_h1(-30.) - struve_h1(30.)).abs() < 1e-12);
}
#[test]
fn tapped_zero_drive_keeps_passive_electrical_impedance() {
    use speakerbox_acoustics::TappedHorn;
    let segs = vec![
        Segment::conical(0.4, 0.02, 0.02),
        Segment::conical(0.2, 0.02, 0.02),
        Segment::conical(1.2, 0.02, 0.05),
        Segment::conical(0.3, 0.05, 0.08),
    ];
    let mut t = TappedHorn::new_4seg_th(driver(), segs);
    let a = t.try_response_at(50.).unwrap();
    t.eg = 0.;
    let z = t.try_response_at(50.).unwrap();
    assert!((a.ze_mag - z.ze_mag).abs() < 1e-9);
    assert_eq!(z.displacement_mm, 0.);
}
#[test]
fn tapped_polarity_preserves_efficiency() {
    use speakerbox_acoustics::TappedHorn;
    let mut t = TappedHorn::new_4seg_th(
        driver(),
        vec![
            Segment::conical(0.4, 0.02, 0.02),
            Segment::conical(0.2, 0.02, 0.02),
            Segment::conical(1.2, 0.02, 0.05),
            Segment::conical(0.3, 0.05, 0.08),
        ],
    );
    let a = t.try_response_at(50.).unwrap();
    t.polarity = -1.;
    let z = t.try_response_at(50.).unwrap();
    assert!((a.efficiency_pct - z.efficiency_pct).abs() < 1e-9);
}
#[test]
fn custom_profile_does_not_hide_invalid_area() {
    use std::sync::Arc;
    let s = Segment::new(Flare::Custom(Arc::new(|_| f64::NAN)), 1., 0.02, 0.02);
    assert!(s.area(0.5).is_nan());
    assert!(s.validate().is_err());
}
#[test]
fn zero_resistance_leak_is_a_physical_short() {
    let mut s = System::new(Air::default(), driver());
    s.horn = Some(horn());
    s.horn_leak = Some(0.);
    assert_eq!(s.loads(50.).0, C::new(0., 0.));
}
#[test]
fn invalid_bank_driver_is_rejected_at_conversion() {
    use speakerbox_acoustics::DriverBank;
    let mut d = driver();
    d.sd = f64::NAN;
    assert!(DriverBank::new(d, 2).equivalent_driver().is_err());
}
#[test]
fn hypex_contraction_with_positive_t_is_rejected_independently() {
    let s = Segment::new(Flare::HyperbolicExponential { t: 0.5 }, 1., 0.02, 0.01);
    assert!(s.validate().is_err());
}
#[test]
fn group_delay_is_centered_on_nonuniform_frequency_grid() {
    let s = System::new(Air::default(), driver());
    let f = [20., 30., 50.];
    let p = f.map(|f| s.response_at(f));
    let slope0 = (p[1].phase_deg - p[0].phase_deg) / 10.;
    let slope1 = (p[2].phase_deg - p[1].phase_deg) / 20.;
    let expected = -(20. * slope0 + 10. * slope1) / 30. / 360. * 1000.;
    assert!((s.response(&f)[1].group_delay_ms - expected).abs() < 1e-10);
    let d = speakerbox_acoustics::Design::System(s);
    assert!((d.response(&f)[1].group_delay_ms - expected).abs() < 1e-10);
}
