//! Demostración de la fachada `Design` para la UI.
//!
//! ```text
//! cargo run --release --example design_demo
//! ```

use speakerbox_acoustics::{Design, Driver, Flare, OffsetHorn, Segment};

fn main() {
    let driver = Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1);
    let mut od = OffsetHorn::new_4seg_od(
        driver,
        vec![
            Segment::new(Flare::Conical, 0.40, 400e-4, 400e-4),
            Segment::new(Flare::Exponential, 0.20, 400e-4, 400e-4),
            Segment::new(Flare::Conical, 1.20, 400e-4, 1200e-4),
            Segment::new(Flare::Conical, 0.30, 1200e-4, 1500e-4),
        ],
    );
    od.rear_chamber_m3 = Some(10.0e-3);

    let design = Design::OffsetHorn(od);
    println!(
        "{:>8} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "f(Hz)", "SPL(dB)", "Ze(Ω)", "Xd(mm)", "Za.Re", "Za.Im"
    );
    for p in design.sweep_log(20.0, 2000.0, 12) {
        println!(
            "{:>8.1} {:>10.2} {:>10.2} {:>10.3} {:>10.1} {:>10.1}",
            p.freq_hz, p.spl_db, p.ze_mag, p.xd_mm, p.za_real, p.za_imag
        );
    }
}
