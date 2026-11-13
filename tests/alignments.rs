use speakerbox_acoustics::alignments::*;
use speakerbox_acoustics::*;
use std::f64::consts::{PI, TAU};
fn driver(q: f64) -> Driver {
    let mut d = Driver::new(0.001, 4e-4, 0.02, 6., 1., 0.5, 0.);
    let air = Air::default();
    d.bl = (d.re * (TAU * d.fs(&air) * d.mms(&air) / q - d.rms)).sqrt();
    d
}
#[test]
fn b4_exact_polynomial_cutoff_and_full_solver() {
    let d = driver((PI / 8.).sin());
    let air = Air::default();
    let a = vented(&d, &air, VentedFamily::ButterworthB4).unwrap();
    assert!((a.alpha - 2_f64.sqrt()).abs() < 1e-12);
    assert!((a.tuning_hz - d.fs(&air)).abs() < 1e-10);
    assert!((a.f3_hz - d.fs(&air)).abs() < 1e-10);
    let fs = d.fs(&air);
    let freqs = [0.5 * fs, fs, 2. * fs, 3. * fs];
    for f in freqs {
        assert!((a.transfer(f).norm() - 1. / (1. + (fs / f).powi(8)).sqrt()).abs() < 1e-12);
    }
    let delta = a.solver_deviation_db(d, air, 1e-5, &freqs).unwrap();
    println!("B4 complete solver deviation dB: {delta:?}");
    assert!(delta.iter().all(|v| v.abs() < 0.01));
    assert!(a
        .to_system(d, air, 1e-10, EndCorrection::FlangedBoth)
        .is_err());
    assert!(vented(&driver(0.3), &air, VentedFamily::ButterworthB4).is_err());
}
#[test]
fn qb3_sc4_bessel_and_chebyshev_are_realizable_in_their_domains() {
    let air = Air::default();
    let d = driver(0.3);
    for fam in [VentedFamily::QuasiButterworth3, VentedFamily::SubChebyshev4] {
        let a = vented(&d, &air, fam).unwrap();
        assert!(a.volume_m3 > 0. && a.tuning_hz > 0.);
        let delta = a
            .solver_deviation_db(d, air, 1e-5, &[a.f3_hz, a.f3_hz * 2.])
            .unwrap();
        assert!(delta.iter().all(|v| v.abs() < 0.01));
        if matches!(fam, VentedFamily::QuasiButterworth3) {
            let c3 = 1. / a.qts;
            let c2 = 1. + a.alpha + (a.tuning_hz / a.fs_hz).powi(2);
            let h2 = (a.tuning_hz / a.fs_hz).powi(2);
            assert!((c3 * c3 - 2. * c2).abs() < 1e-10);
            assert!((c2 * c2 - 2. * c3 * h2 / a.qts + 2. * h2).abs() < 1e-10);
        }
    }
    let b = vented(&driver(1. / 10_f64.sqrt()), &air, VentedFamily::Bessel4).unwrap();
    assert!((b.alpha - 7. / 3.).abs() < 1e-10);
    let epsilon = (10_f64.powf(0.1) - 1.).sqrt();
    let k = ((1. / epsilon).asinh() / 4.).tanh();
    let theta = PI / 8.;
    let a = k * theta.sin();
    let b = k * (3. * theta).sin();
    let c = a * a + theta.cos().powi(2);
    let e = b * b + (3. * theta).cos().powi(2);
    let q = (c * e / (4. * (a + b) * (a * e + b * c))).sqrt();
    let a = vented(&driver(q), &air, VentedFamily::Chebyshev4 { ripple_db: 1. }).unwrap();
    let peak = (0..10000)
        .map(|i| {
            a.transfer(a.fs_hz * 10_f64.powf(-1. + 3. * i as f64 / 9999.))
                .norm()
        })
        .fold(0., f64::max);
    assert!((20. * peak.log10() - 1.).abs() < 1e-4);
}
#[test]
fn sealed_prototypes_and_ebs_proposal_use_real_solver() {
    let air = Air::default();
    let d = driver(0.3);
    for f in [
        SealedFamily::Butterworth,
        SealedFamily::Bessel,
        SealedFamily::Chebyshev { ripple_db: 1. },
    ] {
        let a = sealed(&d, &air, f).unwrap();
        assert!(a.volume_m3 > 0.);
        let x = a.f3_hz / a.resonance_hz;
        let h = x * x / ((1. - x * x).powi(2) + (x / a.qtc).powi(2)).sqrt();
        assert!((h - 1. / 2_f64.sqrt()).abs() < 1e-12);
    }
    let (sys, r) = extended_bass_shelf(
        d,
        air,
        (d.vas(&air) * 0.2, d.vas(&air) * 5.),
        (d.fs(&air) * 0.3, d.fs(&air) * 1.5),
        1e-5,
        d.fs(&air) * 0.5,
        -3.,
        250,
    )
    .unwrap();
    assert!(r.best.as_ref().unwrap().cost.is_finite());
    println!("EBS fitted mean squared dB error: {}", r.best.unwrap().cost);
    sys.validate().unwrap();
}
