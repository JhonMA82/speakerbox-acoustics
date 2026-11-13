use speakerbox_acoustics::measured_data::*;
fn parse(s: &str) -> Result<Measurement, DataError> {
    Measurement::parse(s, ImportOptions::frd(), MeasurementReference::default())
}
#[test]
fn synthetic_separators_headers_and_wrapped_phase() {
    let data = parse(
        "# synthetic, no measured provenance\nFrequency SPL Phase\n100;80;170\n1000,82,-170 # note",
    )
    .unwrap();
    let mid = data.interpolate(100_f64.sqrt() * 1000_f64.sqrt()).unwrap();
    assert!((mid.magnitude - 81.).abs() < 1e-12);
    assert!((mid.phase_deg.unwrap() - 180.).abs() < 1e-12);
    assert!(data.reference.provenance.is_none());
    let d = parse("100 80 360\n1000 80 720").unwrap();
    assert!(
        (d.interpolate(100_f64.sqrt() * 1000_f64.sqrt())
            .unwrap()
            .phase_deg
            .unwrap()
            - 540.)
            .abs()
            < 1e-10
    );
}
#[test]
fn missing_phase_is_not_invented_and_invalid_data_is_rejected() {
    let d = parse("100\t80\n1000\t90\t20").unwrap();
    assert!(d.interpolate(200.).unwrap().phase_deg.is_none());
    assert!(d.samples[0].complex(d.kind).unwrap().is_none());
    for bad in [
        "100 80\n100 90",
        "100 80\n90 90",
        "100",
        "100 NaN",
        "100 90 inf",
        "-1 80",
        "0 80",
        "100 80\nnope 80",
        "100 80 1 2",
        "",
    ] {
        assert!(parse(bad).is_err(), "{bad}");
    }
    assert!(d.interpolate(1.).is_err());
}
#[test]
fn impedance_interpolation_is_cartesian_and_units_explicit() {
    let d = Measurement::parse(
        "0.1,1,170\n1,1,-170",
        ImportOptions {
            frequency_scale_to_hz: 1000.,
            magnitude_scale: 1000.,
            ..ImportOptions::zma()
        },
        MeasurementReference {
            provenance: Some("synthetic".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let s = d.interpolate(100_f64.sqrt() * 1000_f64.sqrt()).unwrap();
    let z = s.complex(d.kind).unwrap().unwrap();
    assert!(z.im.abs() < 1e-10);
    assert!((z.re - 1000. * 170_f64.to_radians().cos()).abs() < 1e-10);
}

#[test]
fn extreme_units_do_not_silently_overflow() {
    let d = parse("1e-300 0 0\n1e300 100 0").unwrap();
    assert!((d.interpolate(1.).unwrap().magnitude - 50.).abs() < 1e-10);
    let invalid = Sample {
        frequency_hz: 1.,
        magnitude: 1e6,
        phase_deg: Some(0.),
    };
    assert!(invalid.complex(MeasurementKind::SplDb).is_err());
}
