//! CLI mínimo sobre `speakerbox-acoustics`.
//!
//! ```text
//! speakerbox-cli --record record.dat [--csv] [--fmin 10] [--fmax 20000] [--points 200]
//! ```

use speakerbox_acoustics::{Air, Calibration, Driver, Flare, Horn, Segment, System};

fn main() {
    let mut record: Option<String> = None;
    let mut csv = false;
    let mut raw = false;
    let mut fmin: f64 = 10.0;
    let mut fmax: f64 = 20_000.0;
    let mut points = 120usize;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--record" | "-r" => {
                i += 1;
                record = args.get(i).cloned();
            }
            "--csv" => csv = true,
            "--raw" => raw = true,
            "--fmin" => {
                i += 1;
                fmin = args.get(i).and_then(|v| v.parse().ok()).unwrap_or(fmin);
            }
            "--fmax" => {
                i += 1;
                fmax = args.get(i).and_then(|v| v.parse().ok()).unwrap_or(fmax);
            }
            "--points" => {
                i += 1;
                points = args
                    .get(i)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(points)
                    .max(2);
            }
            "--help" | "-h" => {
                println!("uso: speakerbox-cli --record record.dat [--csv] [--raw] [--fmin 10] [--fmax 20000] [--points 120]");
                return;
            }
            other => eprintln!("opción desconocida: {other}"),
        }
        i += 1;
    }

    let sys = match record {
        Some(path) => match std::fs::read(&path) {
            Ok(bytes) => match speakerbox_acoustics::LegacyRecord::parse(&bytes) {
                Ok(rec) => {
                    eprintln!("registro: {:?}", rec.comment);
                    if raw {
                        rec.to_system()
                    } else {
                        rec.to_system_with(&Calibration::validated_60_70())
                    }
                }
                Err(err) => {
                    eprintln!("registro inválido: {err}");
                    std::process::exit(2);
                }
            },
            Err(err) => {
                eprintln!("no se pudo leer {path}: {err}");
                std::process::exit(2);
            }
        },
        None => default_system(),
    };

    // Barrido logarítmico.
    let freqs: Vec<f64> = (0..points)
        .map(|i| {
            let t = i as f64 / (points - 1) as f64;
            fmin * (fmax / fmin).powf(t)
        })
        .collect();
    let pts = sys.response(&freqs);

    if csv {
        println!("freq_hz,spl_db,phase_deg,group_delay_ms,ze_mag_ohm,ze_phase_deg,displacement_mm,power_w,efficiency_pct,za_real,za_imag");
        for p in pts {
            println!(
                "{:.3},{:.4},{:.3},{:.4},{:.4},{:.3},{:.6},{:.6},{:.6},{:.4},{:.4}",
                p.freq,
                p.spl_db,
                p.phase_deg,
                p.group_delay_ms,
                p.ze_mag,
                p.ze_phase_deg,
                p.displacement_mm,
                p.acoustical_power_w,
                p.efficiency_pct,
                p.za_real,
                p.za_imag
            );
        }
    } else {
        println!(
            "{:>9} {:>10} {:>10} {:>10} {:>10} {:>10}",
            "f(Hz)", "SPL(dB)", "Fase(°)", "GD(ms)", "Ze(Ω)", "X(mm)"
        );
        for p in pts {
            println!(
                "{:>9.2} {:>10.2} {:>10.1} {:>10.2} {:>10.2} {:>10.3}",
                p.freq, p.spl_db, p.phase_deg, p.group_delay_ms, p.ze_mag, p.displacement_mm
            );
        }
    }
}

fn default_system() -> System {
    let driver = Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1);
    let mut sys = System::new(Air::default(), driver);
    sys.solid_angle = 0.5 * std::f64::consts::PI;
    sys.eg = 2.83;
    sys.horn = Some(Horn::new(vec![
        Segment::new(Flare::Conical, 0.60, 80.0e-4, 350.0e-4),
        Segment::new(Flare::Exponential, 0.75, 350.0e-4, 2400.0e-4),
    ]));
    // Registro por defecto con la calibración validada.
    sys.throat_chamber = Some(speakerbox_acoustics::Chamber::new(1.10e-3));
    sys.rear_chamber = Some(speakerbox_acoustics::Chamber::new(14.0e-3));
    if let Some(h) = sys.horn.as_mut() {
        h.radiation_scale = 1.90;
    }
    sys.spl_offset_db = -1.57;
    sys
}
