//! Soluciones analíticas e invariantes energéticos, independientes de Hornresp.
use num_complex::Complex64 as C;
use speakerbox_acoustics::bandpass::PortModel;
use speakerbox_acoustics::element::Lining;
use speakerbox_acoustics::energy::{
    active_power, EnergyReport, RadiationQuality as Q, TerminationKind,
};
use speakerbox_acoustics::{
    Air, Bandpass, Chamber, Design, Driver, EndCorrection, Flare, Horn, OffsetHorn,
    PassiveRadiator, Port, Segment, System, TappedHorn,
};
use std::f64::consts::TAU;
fn d() -> Driver {
    Driver::from_ui_units(350., 4e-4, 20., 6., 18., 4., 1., 1)
}
fn port() -> Port {
    Port::new(0.005, 0.1, EndCorrection::FlangedBoth)
}
fn segs(loss: f64) -> Vec<Segment> {
    let mut ss = vec![
        Segment::new(Flare::Cylindrical, 0.4, 0.02, 0.02),
        Segment::conical(0.3, 0.02, 0.03),
        Segment::conical(0.7, 0.03, 0.05),
        Segment::conical(0.2, 0.05, 0.08),
    ];
    for s in &mut ss {
        s.loss = loss;
    }
    ss
}
fn close(a: f64, b: f64, rel: f64) {
    assert!(
        (a - b).abs() <= 1e-11 + rel * b.abs(),
        "{a:.14e} != {b:.14e}"
    );
}
fn balance(r: &EnergyReport) {
    println!(
        "source={} loads={} loss={} residual={}",
        r.source_active_w,
        r.termination_absorbed_w(),
        r.acoustic_losses.total_w(),
        r.balance_error_w()
    );
    close(
        r.source_active_w,
        r.source_resistor_loss_w + r.driver_terminal_active_w,
        1e-10,
    );
    assert!(
        r.balance_error_w().abs() <= 1e-10 + 2e-7 * r.source_active_w,
        "{r:#?}"
    );
    assert!(
        r.acoustic_balance_error_w().abs() <= 1e-10 + 2e-7 * r.source_active_w,
        "{r:#?}"
    );
    for t in &r.terminations {
        close(
            t.absorbed_power_w(),
            t.volume_velocity_m3_s.norm_sqr() * t.impedance_pa_s_m3.re,
            2e-7,
        );
        assert!(t.absorbed_power_w() >= 0.);
    }
    if let Some(eta) = r.efficiency_vs_source_pct() {
        assert!((0.0..=100. + 1e-6).contains(&eta));
    }
}
#[test]
fn matched_cylinder_absorption_and_no_reflection() {
    let air = Air::default();
    let area = 0.02;
    let z0 = air.rho * air.c / area;
    let mut s = System::new(air, d());
    let mut h = Horn::new(vec![Segment::new(Flare::Cylindrical, 0.7, area, area)]);
    h.matched_mouth = true;
    s.horn = Some(h);
    for f in [1., 20., 100., 1000., 5000.] {
        let h = s.horn.as_ref().unwrap();
        let zin = h.input_impedance(&air, f, s.solid_angle);
        close(zin.re, z0, 1e-12);
        assert!(zin.im.abs() < 1e-10 * z0);
        let e = s.try_energy_at(f).unwrap();
        balance(&e);
        let t = e.terminations[0];
        assert_eq!(t.kind, TerminationKind::MatchedAbsorber);
        assert_eq!(e.radiated_power.watts, None);
        assert_eq!(e.efficiency_vs_source_pct(), None);
        let incident = (t.pressure_pa + z0 * t.volume_velocity_m3_s) / 2.;
        let reflected = (t.pressure_pa - z0 * t.volume_velocity_m3_s) / 2.;
        assert!(reflected.norm() < 1e-11 * incident.norm());
        assert!(s.try_response_at(f).is_err());
    }
}
#[test]
fn single_piston_sealed_box_and_resistive_source_have_explicit_balance() {
    for rg in [0., 1., 10.] {
        let mut s = System::new(Air::default(), d());
        s.rg = rg;
        s.rear_chamber = Some(Chamber::new(0.04));
        for f in [10., 50., 100., 500.] {
            let r = s.try_energy_at(f).unwrap();
            balance(&r);
            assert_eq!(r.radiated_power.quality, Q::SingleBaffledPiston);
            close(
                s.try_response_at(f).unwrap().acoustical_power_w,
                r.termination_absorbed_w(),
                1e-12,
            );
            if rg > 0. {
                assert!(
                    r.efficiency_vs_driver_terminals_pct().unwrap()
                        > r.efficiency_vs_source_pct().unwrap()
                );
            }
        }
    }
}
#[test]
fn system_explicit_dissipation_in_lining_leak_series_and_walls() {
    let mut s = System::new(Air::default(), d());
    s.horn = Some(Horn::new(segs(0.03)));
    s.horn_series = Some(4000.);
    s.horn_leak = Some(30000.);
    s.throat_chamber = Some(Chamber::lined(0.002, Lining::new(0.2, 0.1)));
    s.rear_chamber = Some(Chamber::lined(0.04, Lining::new(0.15, 0.4)));
    for f in [20., 50., 100., 300.] {
        let r = s.try_energy_at(f).unwrap();
        balance(&r);
        assert!(r.acoustic_losses.wall_w > 0.);
        assert!(r.acoustic_losses.lining_w > 0.);
        assert!(r.acoustic_losses.leakage_w > 0.);
        assert!(r.acoustic_losses.throat_series_w > 0.);
    }
}
#[test]
fn vented_and_passive_models_mark_missing_radiation_instead_of_inventing_efficiency() {
    for pr in [false, true] {
        let mut s = System::new(Air::default(), d());
        s.rear_chamber = Some(Chamber::new(0.04));
        if pr {
            s.passive_radiator = Some(PassiveRadiator::new(0.03, 4e-4, 0.04, 2.));
        } else {
            s.port = Some(port());
            s.rear_ql = Some(7.);
        }
        for f in [20., 50., 100., 300.] {
            let r = s.try_energy_at(f).unwrap();
            balance(&r);
            assert_eq!(r.radiated_power.quality, Q::IncompleteRadiationModel);
            assert_eq!(r.radiated_power.watts, None);
            assert_eq!(r.efficiency_vs_source_pct(), None);
        }
    }
}
#[test]
fn bp4_parallel_and_series_balance_excludes_internal_port_radiation() {
    for mut b in [
        Bandpass::new_bp4(d(), 0.02, 0.01, port()),
        Bandpass::new_bp6(d(), 0.02, 0.01, port(), port()),
        Bandpass::new_bp6_series(d(), 0.02, 0.01, port(), port()),
    ] {
        b.set_ql(7.);
        b.rg = 2.;
        for f in [10., 30., 50., 100., 300.] {
            let r = b.try_energy_at(f).unwrap();
            balance(&r);
            close(
                b.try_response_at(f).unwrap().power_w,
                r.termination_absorbed_w(),
                1e-12,
            );
        }
        b.port_model = PortModel::Lumped;
        let r = b.try_energy_at(50.).unwrap();
        balance(&r);
        assert!(r.terminations.is_empty());
        assert_eq!(r.radiated_power.watts, None);
    }
}
#[test]
fn tapped_and_offset_with_and_without_wall_loss_are_passive() {
    for loss in [0., 0.02] {
        let mut t = TappedHorn::new_4seg_th(d(), segs(loss));
        t.rg = 1.;
        let mut o = OffsetHorn::new_4seg_od(d(), segs(loss));
        o.rg = 1.;
        for f in [20., 50., 100., 300.] {
            let r = t.try_energy_at(f).unwrap();
            balance(&r);
            close(
                t.try_response_at(f).unwrap().power_w,
                r.termination_absorbed_w(),
                1e-10,
            );
            let r = o.try_energy_at(f).unwrap();
            balance(&r);
            close(
                o.try_response_at(f).unwrap().power_w,
                r.termination_absorbed_w(),
                1e-10,
            );
        }
    }
}
#[test]
fn offset_passive_and_port_rear_have_explicit_loss_and_missing_total() {
    for pr in [false, true] {
        let mut o = OffsetHorn::new_4seg_od(d(), segs(0.));
        o.rear_chamber_m3 = Some(0.04);
        if pr {
            o.rear_passive_radiator = Some(PassiveRadiator::new(0.03, 4e-4, 0.04, 2.));
        } else {
            o.rear_port = Some(port());
        }
        for f in [20., 50., 100.] {
            let r = o.try_energy_at(f).unwrap();
            balance(&r);
            assert_eq!(r.radiated_power.watts, None);
        }
    }
}
#[test]
fn oscillator_circuit_and_helmholtz_recover_analytic_limits() {
    let driver = d();
    let air = Air::default();
    let f0 = 1. / (TAU * (driver.mmd * driver.cms).sqrt());
    let zm = driver.mechanical_impedance(f0);
    close(zm.re, driver.rms, 1e-12);
    assert!(zm.im.abs() < 1e-12);
    let i = C::new(2.83, 0.) / driver.input_impedance(f0, C::new(0., 0.));
    let u = driver.volume_velocity(f0, 2.83, 0., C::new(0., 0.));
    close(
        2.83 * i.re,
        driver.re * i.norm_sqr() + driver.rms * (u / driver.sd).norm_sqr(),
        1e-12,
    );
    let ch = Chamber::new(0.04);
    let p = port();
    let f = air.c / TAU * (p.area / (p.effective_length() * ch.volume)).sqrt();
    let y = C::new(1., 0.) / ch.impedance(&air, f) + C::new(1., 0.) / p.impedance(&air, f);
    assert!(y.norm() < 1e-18);
    close(active_power(C::new(3., 4.), C::new(3., 4.) / 5.), 5., 1e-12);
}
#[test]
fn lossless_transfer_preserves_flux_and_reciprocity() {
    let air = Air::default();
    for f in [10., 50., 500., 2000.] {
        for seg in segs(0.) {
            let m = seg.matrix(&air, f);
            assert!((m.a * m.d - m.b * m.c - C::new(1., 0.)).norm() < 1e-7);
            let p = C::new(3., 4.);
            let u = C::new(0.002, -0.001);
            let (p2, u2) = m.apply(p, u);
            close(active_power(p, u), active_power(p2, u2), 1e-7);
        }
    }
}
#[test]
fn zero_drive_does_not_fabricate_an_efficiency() {
    let mut s = System::new(Air::default(), d());
    s.eg = 0.;
    let r = s.try_energy_at(50.).unwrap();
    balance(&r);
    assert_eq!(r.efficiency_vs_source_pct(), None);
    assert_eq!(r.efficiency_vs_driver_terminals_pct(), None);
}
#[test]
fn design_preserves_the_energy_quality() {
    let b = Bandpass::new_bp4(d(), 0.02, 0.01, port());
    let r = b.try_energy_at(50.).unwrap();
    let x = Design::Bandpass(b).try_energy_at(50.).unwrap();
    close(r.source_active_w, x.source_active_w, 1e-15);
    assert_eq!(r.radiated_power.quality, x.radiated_power.quality);
}
#[test]
fn phase_of_log_linear_and_resonant_pressure_has_consistent_delay() {
    use speakerbox_acoustics::phase::complex_phase_trace;
    for f in [
        (0..81).map(|i| 10. + 2. * i as f64).collect::<Vec<_>>(),
        (0..81)
            .map(|i| 10. * 20.0_f64.powf(i as f64 / 80.))
            .collect(),
    ] {
        let tau = 0.001;
        let p = f
            .iter()
            .map(|&f| C::from_polar(1e-200, -TAU * f * tau))
            .collect::<Vec<_>>();
        let tr = complex_phase_trace(&f, &p, &vec![0.; f.len()]).unwrap();
        for t in tr {
            close(t.group_delay_ms.unwrap(), tau * 1000., 1e-11);
        }
    }
    let f = (0..101).map(|i| 49. + 0.02 * i as f64).collect::<Vec<_>>();
    let w0 = TAU * 50.;
    let zeta = 0.05;
    let p = f
        .iter()
        .map(|&f| {
            let w = TAU * f;
            C::new(1., 0.) / C::new(w0 * w0 - w * w, 2. * zeta * w0 * w)
        })
        .collect::<Vec<_>>();
    let tr = complex_phase_trace(&f, &p, &vec![0.; f.len()]).unwrap();
    close(tr[50].group_delay_ms.unwrap(), 1000. / (zeta * w0), 3e-5);
}
#[test]
fn observation_phase_handles_duplicates_polarity_zero_and_matched() {
    let mut s = System::new(Air::default(), d());
    let f = [50., 20., 100., 50.];
    let a = Design::System(s.clone())
        .try_observation_response(&f)
        .unwrap();
    s.eg = -s.eg;
    let b = Design::System(s.clone())
        .try_observation_response(&f)
        .unwrap();
    for (x, y) in a.iter().zip(&b) {
        close(
            x.pressure_estimate_pa.unwrap().norm(),
            y.pressure_estimate_pa.unwrap().norm(),
            1e-12,
        );
        assert!((x.pressure_estimate_pa.unwrap() + y.pressure_estimate_pa.unwrap()).norm() < 1e-12);
        close(x.group_delay_ms.unwrap(), y.group_delay_ms.unwrap(), 1e-12);
    }
    assert_eq!(a[0].group_delay_ms, a[3].group_delay_ms);
    s.eg = 0.;
    let zero = Design::System(s.clone())
        .try_observation_response(&f)
        .unwrap();
    assert!(zero
        .iter()
        .all(|p| p.phase_deg.is_none() && p.group_delay_ms.is_none()));
    let mut h = Horn::new(segs(0.));
    h.matched_mouth = true;
    s.horn = Some(h);
    let m = Design::System(s).try_observation_at(50.).unwrap();
    assert!(m.spl_db.is_none() && m.pressure_estimate_pa.is_none());
}
#[test]
fn exact_and_numerically_unresolved_cancellations_break_unwrap() {
    use speakerbox_acoustics::phase::complex_phase_trace;
    let f = [10., 20., 30.];
    let p = [C::new(1., 0.), C::new(1e-17, 0.), C::new(-1., 0.)];
    // La cota 2ε es la del redondeo de una resta de componentes de orden unidad.
    let t = complex_phase_trace(&f, &p, &[0., 2. * f64::EPSILON, 0.]).unwrap();
    assert!(t[1].phase_deg.is_none());
    assert!(t.iter().all(|p| p.group_delay_ms.is_none()));
    let t = complex_phase_trace(&f, &[C::new(1e-200, 0.); 3], &[0.; 3]).unwrap();
    assert!(t.iter().all(|p| p.phase_deg.is_some()));
    let b = Bandpass::new_bp6(d(), 0.01, 0.01, port(), port());
    let q = Design::Bandpass(b)
        .try_observation_response(&[20., 50., 100.])
        .unwrap();
    assert!(q
        .iter()
        .all(|p| p.phase_deg.is_none() && p.group_delay_ms.is_none()));
}
#[test]
fn wall_dissipation_converges_under_geometric_subdivision() {
    let air = Air::default();
    let mut seg = Segment::conical(1., 0.01, 0.07);
    seg.loss = 0.08;
    let f = 700.;
    let z = C::new(3000., 500.);
    let pin = C::new(2., 1.);
    let uin = pin / z;
    let coarse = seg.dissipated_power_w(&air, f, pin, uin).unwrap();
    let mut p = pin;
    let mut u = uin;
    let mut fine = 0.;
    for i in 0..4 {
        let mut part = Segment::conical(
            0.25,
            seg.area(i as f64 * 0.25),
            seg.area((i + 1) as f64 * 0.25),
        );
        part.loss = seg.loss;
        fine += part.dissipated_power_w(&air, f, p, u).unwrap();
        (p, u) = part.matrix(&air, f).apply(p, u);
    }
    let flux = active_power(pin, uin) - active_power(p, u);
    close(fine, flux, 2e-7);
    close(coarse, fine, 2e-6);
}
