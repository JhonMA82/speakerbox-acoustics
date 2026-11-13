//! Regresiones de potencia independientes del SPL y de Hornresp.
use num_complex::Complex64 as C;
use speakerbox_acoustics::radiation::piston_radiation_impedance;
use speakerbox_acoustics::{
    Air, Bandpass, Driver, EndCorrection, Flare, Horn, OffsetHorn, Port, Segment, System,
    TappedHorn,
};
use std::f64::consts::{PI, TAU};
fn driver() -> Driver {
    Driver::from_ui_units(350., 4e-4, 20., 6., 18., 4., 1., 1)
}
fn segments() -> Vec<Segment> {
    vec![
        Segment::new(Flare::Cylindrical, 0.4, 0.02, 0.02),
        Segment::conical(0.3, 0.02, 0.02),
        Segment::conical(0.7, 0.02, 0.05),
        Segment::conical(0.2, 0.05, 0.08),
    ]
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-12 + 1e-8 * b.abs(), "{a} != {b}");
}
#[test]
fn ec001_matched_load_alone_does_not_define_spl() {
    let mut s = System::new(Air::default(), driver());
    let mut h = Horn::new(segments());
    h.matched_mouth = true;
    s.horn = Some(h);
    assert!(s.try_response_at(50.).is_err());
}
#[test]
fn ec001_piston_power_uses_scaled_effective_terminal_load() {
    let mut s = System::new(Air::default(), driver());
    let mut h = Horn::new(segments());
    h.radiation_scale = 2.;
    h.mouth_radiation_area_factor = 1.7;
    s.solid_angle = PI;
    s.horn = Some(h);
    for f in [20., 100., 300.] {
        let p = s.try_response_at(f).unwrap();
        let h = s.horn.as_ref().unwrap();
        let z = piston_radiation_impedance(
            &s.air,
            h.mouth_area() * h.mouth_radiation_area_factor,
            f,
            s.solid_angle,
        ) * h.radiation_scale;
        near(p.acoustical_power_w, p.mouth_volume_velocity.powi(2) * z.re);
    }
}
#[test]
fn ec002_tapped_power_is_terminal_flux_not_solid_angle_gain() {
    let t = TappedHorn::new_4seg_th(driver(), segments());
    for f in [20., 50., 200.] {
        let (p, z, _, _, _) = t.debug_at(f).unwrap();
        near(
            t.try_response_at(f).unwrap().power_w,
            (p * (p / z).conj()).re,
        );
    }
}
#[test]
fn ec002_offset_power_is_sum_of_actual_load_absorption() {
    let o = OffsetHorn::new_4seg_od(driver(), segments());
    for f in [20., 50., 200.] {
        let s = o.try_solution_at(f).unwrap();
        let z = piston_radiation_impedance(
            &o.air,
            o.segments.last().unwrap().s2() * o.mouth_radiation_area_factor,
            f,
            o.solid_angle,
        ) * o.mouth_radiation_scale;
        near(
            s.point.power_w,
            s.mouth_volume_velocity.norm_sqr() * z.re
                + s.driver_volume_velocity.norm_sqr() * o.rear_impedance(f).re,
        );
    }
}
#[test]
fn ec002_bandpass_pipe_power_uses_piston_termination() {
    let port = Port::new(0.005, 0.1, EndCorrection::None);
    let b = Bandpass::new_bp4(driver(), 0.02, 0.01, port);
    for f in [20., 50., 500.] {
        let s = b.try_solution_at(f).unwrap();
        let z = piston_radiation_impedance(&b.air, port.area, f, b.solid_angle);
        near(s.point.power_w, s.front_volume_velocity.norm_sqr() * z.re);
    }
}
#[test]
fn ec002_direct_power_does_not_depend_on_observation_angle() {
    let mut s = System::new(Air::default(), driver());
    let a = s.try_response_at(50.).unwrap();
    s.solid_angle = PI;
    let b = s.try_response_at(50.).unwrap();
    near(a.acoustical_power_w, b.acoustical_power_w);
}
#[test]
fn ec002_source_and_driver_terminal_efficiency_are_not_equivalent() {
    let mut s = System::new(Air::default(), driver());
    s.rg = 6.;
    let (zf, zr) = s.loads(50.);
    let u = s.driver.volume_velocity(50., s.eg, s.rg, zf + zr);
    let v = u / s.driver.sd;
    let i = (C::new(s.eg, 0.) - s.driver.bl * v) / (s.driver.coil_impedance(50.) + s.rg);
    let source = s.eg * i.re;
    let terminal = source - s.rg * i.norm_sqr();
    assert!(source > terminal && terminal > 0.);
    let _ = TAU;
}
#[test]
fn ec001_calibration_cannot_make_a_matched_load_reflect() {
    let mut h = Horn::new(segments());
    h.matched_mouth = true;
    h.radiation_scale = 2.;
    assert!(h.validate().is_err());
}
#[test]
fn ec004_silent_pressure_has_no_finite_phase() {
    let mut s = System::new(Air::default(), driver());
    s.eg = 0.;
    assert!(s.try_response_at(50.).unwrap().phase_deg.is_nan());
}
#[test]
fn ec002_zero_input_has_no_defined_efficiency() {
    let mut s = System::new(Air::default(), driver());
    s.eg = 0.;
    assert!(s.try_response_at(50.).unwrap().efficiency_pct.is_nan());
}
#[test]
fn ec001_equal_factors_cancel_without_overflow_in_a_matched_load() {
    let a = Air::default();
    let mut h = Horn::new(vec![Segment::new(Flare::Cylindrical, 0.3, 100., 100.)]);
    h.matched_mouth = true;
    h.radiation_scale = 1e308;
    h.mouth_radiation_area_factor = 1e308;
    h.validate().unwrap();
    let z = h.input_impedance(&a, 50., TAU);
    near(z.re, a.rho * a.c / 100.);
}
#[test]
fn ec003_offset_at_throat_still_has_a_radiating_rear_face() {
    let mut o = OffsetHorn::new_4seg_od(driver(), segments());
    o.tap = 0;
    let a = o.try_response_at(100.).unwrap();
    o.rear_radiation_scale = 0.;
    let b = o.try_response_at(100.).unwrap();
    assert!((a.spl_db - b.spl_db).abs() > 0.01);
}
