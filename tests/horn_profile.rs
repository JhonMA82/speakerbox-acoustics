use std::f64::consts::PI;

use speakerbox_acoustics::{Air, HornProfileKind as Kind, HornProfileSpec};

fn near(actual: f64, expected: f64, relative: f64) {
    assert!(
        (actual - expected).abs() <= relative * expected.abs().max(f64::MIN_POSITIVE),
        "{actual:.12e} != {expected:.12e} [relative tolerance {relative:.2e}]"
    );
}

fn spec(kind: Kind, length_m: f64) -> HornProfileSpec {
    HornProfileSpec {
        kind,
        length_m,
        throat_radius_m: 0.025,
        mouth_radius_m: 0.12,
    }
}

// Independent calculation from the tangent property: dz/dr = sqrt(a²-r²)/r.
fn axial_span(a: f64, rt: f64, rm: f64) -> f64 {
    let x = |r: f64| {
        let tangent_projection = ((a - r) * (a + r)).sqrt();
        a * ((a + tangent_projection) / r).ln() - tangent_projection
    };
    x(rt) - x(rm)
}

#[test]
fn native_and_compound_profiles_preserve_ends_and_volume() {
    let natural = HornProfileSpec::natural_tractrix_length(0.025, 0.12).unwrap();
    for kind in [
        Kind::Conical,
        Kind::Exponential,
        Kind::Hypex { t: 0.5 },
        Kind::Tractrix,
        Kind::ScaledTractrix,
        Kind::OblateSpheroidal,
        Kind::ExponentialConical {
            join_fraction: 0.37,
        },
    ] {
        let specification = spec(kind, natural);
        assert_eq!(
            HornProfileSpec::from_json(&specification.to_json().unwrap()).unwrap(),
            specification
        );
        let profile = specification.resolve().unwrap();
        let points = profile.sample(1025).unwrap();
        let first = points.first().unwrap();
        let last = points.last().unwrap();
        assert_eq!(last.axial_m, profile.length_m());
        near(first.radius_m, specification.throat_radius_m, 1e-15);
        near(last.radius_m, specification.mouth_radius_m, 1e-15);
        near(
            first.area_m2,
            PI * specification.throat_radius_m.powi(2),
            1e-15,
        );
        near(
            last.area_m2,
            PI * specification.mouth_radius_m.powi(2),
            1e-15,
        );
        assert!(
            points.windows(2).all(|w| w[0].radius_m < w[1].radius_m),
            "{kind:?}"
        );
        let integrate = |stride: usize| {
            points
                .iter()
                .step_by(stride)
                .collect::<Vec<_>>()
                .windows(2)
                .map(|w| (w[1].axial_m - w[0].axial_m) * (w[1].area_m2 + w[0].area_m2) * 0.5)
                .sum::<f64>()
        };
        let fine = integrate(1);
        let coarse = integrate(8);
        let volume = profile.volume_m3();
        assert!((fine - volume).abs() < (coarse - volume).abs(), "{kind:?}");
        near(fine, volume, 3e-5);
        let horn = profile.into_horn();
        near(horn.volume(), volume, 3e-5);
        assert!(horn
            .segments
            .iter()
            .all(|s| s.try_matrix(&Air::default(), 200.0).is_ok()));
    }
}

#[test]
fn tractrix_geometry_matches_tangent_construction() {
    let (rt, rm) = (0.025, 0.12);
    for a in [rm, 0.19, 0.30] {
        let length = if a == rm {
            HornProfileSpec::natural_tractrix_length(rt, rm).unwrap()
        } else {
            axial_span(a, rt, rm)
        };
        near(length, axial_span(a, rt, rm), 2e-15);
        let profile = spec(Kind::Tractrix, length).resolve().unwrap();
        for radius in [0.035, 0.052, 0.08, 0.1199] {
            let x = axial_span(a, rt, radius);
            near(profile.section_at(x).unwrap().radius_m, radius, 1e-12);
        }
    }

    let natural = axial_span(rm, rt, rm);
    let original = HornProfileSpec::natural_tractrix_length(rt, rm).unwrap();
    let full = spec(Kind::Tractrix, original).resolve().unwrap();
    let scaled = spec(Kind::ScaledTractrix, 0.08).resolve().unwrap();
    near(scaled.volume_m3(), full.volume_m3() * 0.08 / natural, 2e-15);
    near(
        scaled.section_at(0.04).unwrap().radius_m,
        full.section_at(original * 0.5).unwrap().radius_m,
        1e-14,
    );
}

#[test]
fn os_and_exponential_cone_match_independent_geometry() {
    let os = spec(Kind::OblateSpheroidal, 0.3).resolve().unwrap();
    let half_angle = ((0.12_f64.powi(2) - 0.025_f64.powi(2)).sqrt() / 0.3).atan();
    near(
        os.section_at(0.173).unwrap().radius_m,
        0.025f64.hypot(0.173 * half_angle.tan()),
        2e-15,
    );

    let (r0, m, join, length): (f64, f64, f64, f64) = (0.025, 8.4, 0.10, 0.30);
    let rj = r0 * (m * join).exp();
    let mut compound = spec(
        Kind::ExponentialConical {
            join_fraction: join / length,
        },
        length,
    );
    compound.mouth_radius_m = rj + m * rj * (length - join);
    let compound = compound.resolve().unwrap();
    let radius = |x| compound.section_at(x).unwrap().radius_m;
    near(radius(0.05), r0 * (m * 0.05).exp(), 1e-14);
    near(radius(join), rj, 1e-14);
    near(radius(0.2), rj + m * rj * (0.2 - join), 1e-14);
    let h = 1e-7;
    near(
        (radius(join) - radius(join - h)) / h,
        (radius(join + h) - radius(join)) / h,
        1e-6,
    );
}

#[test]
fn invalid_dimensions_and_unknown_persisted_shapes_are_rejected() {
    for bad in [
        spec(Kind::Tractrix, 0.01),
        spec(Kind::Conical, 0.0),
        spec(Kind::Conical, f64::INFINITY),
        spec(Kind::Cylindrical, 0.3),
        spec(Kind::Hypex { t: f64::NAN }, 0.3),
        spec(Kind::ExponentialConical { join_fraction: 1.0 }, 0.3),
        HornProfileSpec {
            mouth_radius_m: 0.01,
            ..spec(Kind::OblateSpheroidal, 0.3)
        },
        HornProfileSpec {
            throat_radius_m: 0.0,
            ..spec(Kind::Tractrix, 0.3)
        },
    ] {
        assert!(bad.resolve().is_err(), "{bad:?}");
        assert!(bad.to_json().is_err(), "{bad:?}");
    }
    let profile = spec(Kind::Conical, 0.3).resolve().unwrap();
    assert!(profile.section_at(f64::NAN).is_err());
    assert!(profile.section_at(0.31).is_err());
    assert!(profile.sample(1).is_err());
    let json = profile.spec().to_json().unwrap();
    assert!(HornProfileSpec::from_json(&json.replace("conical", "lecleach")).is_err());
    assert!(HornProfileSpec::from_json(&json.replace("\"version\":1", "\"version\":2")).is_err());
    assert!(HornProfileSpec::from_json(
        &json.replace("\"version\":1", "\"version\":1,\"unknown\":0")
    )
    .is_err());
}
