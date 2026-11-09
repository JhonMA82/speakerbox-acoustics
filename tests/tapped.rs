//! Comprobaciones geométricas del tapped horn contra el esquema del software de referencia.

use speakerbox_acoustics::{Driver, Flare, Segment, TappedHorn};

fn th2() -> TappedHorn {
    let driver = Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1);
    TappedHorn::new_4seg_th(
        driver,
        vec![
            Segment::new(Flare::Conical, 0.35, 150.0e-4, 400.0e-4),
            Segment::new(Flare::Exponential, 0.25, 400.0e-4, 400.0e-4),
            Segment::new(Flare::Conical, 1.00, 400.0e-4, 900.0e-4),
            Segment::new(Flare::Conical, 0.30, 900.0e-4, 1200.0e-4),
        ],
    )
}

#[test]
fn th2_volume_matches_reference_schematic() {
    let th = th2();
    let volume_l: f64 = th.segments.iter().map(|s| s.volume()).sum::<f64>() * 1000.0;
    // El Schematic Diagram del software de referencia muestra "System Volume 113.999 Litres".
    assert!(
        (volume_l - 113.999).abs() < 0.05,
        "volumen TH2 = {volume_l:.3} L"
    );
}

#[test]
fn th2_taps_are_s2_and_s4() {
    let th = th2();
    assert_eq!(th.segments.len(), 4);
    assert_eq!(th.tap_a, 1); // S2
    assert_eq!(th.tap_b, 3); // S4
}
