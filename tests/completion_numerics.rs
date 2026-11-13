use speakerbox_acoustics::{Air, Driver, Flare, Horn, Segment, System};
#[test]
fn extreme_attenuation_must_not_return_an_apparently_valid_response() {
    let d = Driver::from_ui_units(350., 4e-4, 20., 6., 18., 4., 1., 1);
    let mut s = System::new(Air::default(), d);
    s.horn = Some(Horn::new(vec![Segment::new(
        Flare::Cylindrical,
        0.8,
        0.02,
        0.02,
    )
    .with_flow_resistivity(1e8)]));
    assert!(s.try_response_at(10.).is_err());
}

#[test]
fn subdivision_cannot_hide_ill_conditioned_total_attenuation() {
    let d = Driver::from_ui_units(350., 4e-4, 20., 6., 18., 4., 1., 1);
    let segment = Segment::new(Flare::Cylindrical, 0.01, 0.02, 0.02).with_flow_resistivity(1e8);
    assert!(segment.try_matrix(&Air::default(), 10.).is_ok());
    let mut s = System::new(Air::default(), d);
    s.horn = Some(Horn::new(vec![segment; 80]));
    assert!(s.try_response_at(10.).is_err());
    assert!(s.try_energy_at(10.).is_err());
}

#[test]
fn checked_matrix_rejects_invalid_custom_area_seen_during_integration() {
    use std::sync::Arc;
    let profile = Flare::Custom(Arc::new(|x| {
        if (x - 1. / 384.).abs() < 1e-10 {
            0.
        } else {
            0.02
        }
    }));
    let segment = Segment::new(profile, 1., 0.02, 0.02);
    // The invalid point lies between the preliminary validation samples.
    assert!(segment.validate().is_ok());
    assert!(segment.try_matrix(&Air::default(), 10.).is_err());
}
