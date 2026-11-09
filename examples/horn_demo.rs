//! Demostración del motor: lee un registro `record.dat` (734 bytes) o usa el
//! sistema por defecto, y escribe una tabla de respuesta.
//!
//! Uso:
//! ```text
//! cargo run --release --example horn_demo -- [ruta/record.dat]
//! ```

use speakerbox_acoustics::{Air, Driver, Flare, Horn, Segment, System};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(path) = args.get(1) {
        match std::fs::read(path) {
            Ok(bytes) => match speakerbox_acoustics::LegacyRecord::parse(&bytes) {
                Ok(rec) => {
                    print_record(&rec);
                    print_table(&rec.to_system());
                }
                Err(err) => eprintln!("Registro inválido: {err}"),
            },
            Err(err) => eprintln!("No se pudo leer {path}: {err}"),
        }
        return;
    }

    // Sistema por defecto: el mismo registro que crea el software de referencia al arrancar.
    let driver = Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1);
    let mut sys = System::new(Air::default(), driver);
    sys.solid_angle = 0.5 * std::f64::consts::PI;
    sys.eg = 2.83;
    sys.horn = Some(Horn::new(vec![
        Segment::new(Flare::Conical, 0.60, 80.0e-4, 350.0e-4),
        Segment::new(Flare::Exponential, 0.75, 350.0e-4, 2400.0e-4),
    ]));
    sys.throat_chamber = Some(speakerbox_acoustics::Chamber::new(4.0e-3));
    sys.rear_chamber = Some(speakerbox_acoustics::Chamber::new(14.0e-3));

    let air = sys.air;
    let d = sys.driver;
    println!("Altavoz por defecto del software de referencia");
    println!("  Fs  = {:.2} Hz", d.fs(&air));
    println!("  Qts = {:.3}", d.qts(&air));
    println!("  Vas = {:.2} L", d.vas(&air) * 1000.0);
    println!(
        "  SPL = {:.2} dB (1 W) / {:.2} dB (2.83 V)",
        d.spl_1w(&air),
        d.spl_28v(&air)
    );
    println!();
    print_table(&sys);
}

fn print_record(rec: &speakerbox_acoustics::LegacyRecord) {
    println!("Registro: {:?}", rec.comment);
    println!(
        "  Ang={:.3} rad  Eg={:.2} V  Fta={:.2}°  Cir={:.2}",
        rec.angle_rad, rec.eg, rec.fta_deg, rec.cir
    );
    println!(
        "  Áreas (cm²): S1={:.1} S2={:.1} S3={:.1} S4={:.1} S5={:.1}",
        rec.areas_cm2[0], rec.areas_cm2[1], rec.areas_cm2[2], rec.areas_cm2[3], rec.areas_cm2[4]
    );
    println!(
        "  Longitudes (cm): L12={:.1} L23={:.1} L34={:.1} L45={:.1}",
        rec.lengths_cm[0], rec.lengths_cm[1], rec.lengths_cm[2], rec.lengths_cm[3]
    );
    println!(
        "  Driver: Sd={:.1} cm²  Cms={:.3e} m/N  Mmd={:.1} g  Re={:.2} Ω  Bl={:.2} T·m",
        rec.sd_cm2, rec.cms, rec.mmd_g, rec.re, rec.bl
    );
    println!();
}

fn print_table(sys: &System) {
    let freqs: Vec<f64> = vec![
        10.0, 20.0, 30.0, 50.0, 70.0, 100.0, 150.0, 200.0, 300.0, 500.0, 700.0, 1000.0, 1500.0,
        2000.0, 3000.0, 5000.0, 7000.0, 10000.0, 15000.0, 20000.0,
    ];
    let pts = sys.response(&freqs);
    println!(
        "{:>8} {:>9} {:>9} {:>8} {:>10} {:>8} {:>10}",
        "f(Hz)", "SPL(dB)", "Fase(°)", "GD(ms)", "Re(Za)", "Ze(Ω)", "X(mm)"
    );
    for p in pts {
        println!(
            "{:>8.1} {:>9.2} {:>9.1} {:>8.2} {:>10.2} {:>8.2} {:>10.3}",
            p.freq, p.spl_db, p.phase_deg, p.group_delay_ms, p.za_real, p.ze_mag, p.displacement_mm
        );
    }
}
