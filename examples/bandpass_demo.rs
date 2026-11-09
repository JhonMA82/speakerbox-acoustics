//! Demostración de BP4/BP6 a través de la fachada `Design`.
//!
//! ```text
//! cargo run --release --example bandpass_demo
//! ```

use speakerbox_acoustics::{Bandpass, Design, Driver, EndCorrection, Port};

fn driver() -> Driver {
    Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1)
}

fn port(area_cm2: f64, length_cm: f64) -> Port {
    Port::new(
        area_cm2 * 1.0e-4,
        length_cm * 1.0e-2,
        EndCorrection::UnflangedBoth,
    )
}

fn main() {
    let bp4 = Bandpass::new_bp4(driver(), 20.0e-3, 10.0e-3, port(50.0, 10.0));
    println!("BP4: Fb delantero = {:.1} Hz", bp4.front_tuning_hz());

    let bp6 = Bandpass::new_bp6(
        driver(),
        20.0e-3,
        10.0e-3,
        port(50.0, 10.0),
        port(40.0, 8.0),
    );
    println!(
        "BP6: Fb delantero = {:.1} Hz, Fb trasero = {:.1} Hz",
        bp6.front_tuning_hz(),
        bp6.rear_tuning_hz().unwrap_or(0.0)
    );

    for (name, design) in [
        ("BP4", Design::Bandpass(bp4)),
        ("BP6", Design::Bandpass(bp6)),
    ] {
        println!(
            "
{name}"
        );
        println!(
            "{:>8} {:>10} {:>10} {:>10} {:>10}",
            "f(Hz)", "SPL(dB)", "Ze(Ω)", "Xd(mm)", "v_puerto"
        );
        for p in design.sweep_log(20.0, 1000.0, 10) {
            println!(
                "{:>8.1} {:>10.2} {:>10.2} {:>10.3} {:>10.2}",
                p.freq_hz,
                p.spl_db,
                p.ze_mag,
                p.xd_mm,
                p.port_velocity.unwrap_or(0.0)
            );
        }
    }
}
