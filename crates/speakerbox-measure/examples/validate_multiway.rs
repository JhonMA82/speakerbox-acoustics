//! Valida un dataset de mediciones multivía contra el contrato.
//!
//! ```text
//! cargo run -p speakerbox-measure --example validate_multiway -- ./mi-medicion
//! ```
//!
//! El directorio debe contener `woofer.frd`, `tweeter.frd`, `system.frd` y
//! `meta.txt`. Sale con código 1 si no cumple las tolerancias.

use speakerbox_measure::{load_dataset, validate_dataset};

fn main() {
    let directory = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
    let dataset = match load_dataset(std::path::Path::new(&directory)) {
        Ok(dataset) => dataset,
        Err(error) => {
            eprintln!("error de dataset: {error}");
            std::process::exit(2);
        }
    };
    let report = match validate_dataset(&dataset) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("error de validación: {error}");
            std::process::exit(2);
        }
    };
    println!(
        "{} puntos | magnitud RMS {:.2} dB, máximo {:.2} dB | fase RMS {:.1}°, máximo {:.1}°",
        report.points, report.rmse_db, report.max_db, report.rms_phase_deg, report.max_phase_deg
    );
    if report.passed {
        println!("PASA: cumple las tolerancias de MULTIWAY_PRESSURE_CONTRACT.md §8");
    } else {
        println!("FALLA: revisar las discrepancias; no ajustar el modelo para hacerla pasar");
        std::process::exit(1);
    }
}
