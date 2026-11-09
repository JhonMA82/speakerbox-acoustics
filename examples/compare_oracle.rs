//! Valida el motor contra un export del software de referencia.
//!
//! ```text
//! cargo run --release --example compare_oracle -- \
//!   /ruta/record.dat /ruta/Export/datos.txt [calibrado|crudo]
//! ```
//!
//! El segundo argumento es el fichero tabulado de `File > Export > All Chart
//! Data`. Por defecto aplica la calibración `Calibration::validated_60_70()`.

use speakerbox_acoustics::{validate, Calibration, LegacyRecord, OraclePoint};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dat = args.get(1).unwrap_or_else(|| {
        eprintln!("uso: compare_oracle <record.dat> <export.txt> [calibrado|crudo]");
        std::process::exit(2);
    });
    let tsv = args.get(2).unwrap_or_else(|| {
        eprintln!("uso: compare_oracle <record.dat> <export.txt> [calibrado|crudo]");
        std::process::exit(2);
    });
    let calibrated = args.get(3).map(|s| s != "crudo").unwrap_or(true);

    let record = LegacyRecord::parse(&std::fs::read(dat).expect("no se pudo leer el .dat"))
        .expect("registro inválido");
    let oracle =
        OraclePoint::parse_tsv(&std::fs::read_to_string(tsv).expect("no se pudo leer el export"));
    let system = if calibrated {
        record.to_system_with(&Calibration::validated_60_70())
    } else {
        record.to_system()
    };
    let report = validate(&system, &oracle);

    println!(
        "N={}  SPL rms={:.2} dB (max {:.2})  Ze rel={:.1}% (max {:.1}%)  Xd rel={:.1}% (max {:.1}%)",
        report.n,
        report.spl_rmse_db,
        report.spl_max_abs_db,
        report.ze_rel_rmse * 100.0,
        report.ze_max_rel * 100.0,
        report.xd_rel_rmse * 100.0,
        report.xd_max_rel * 100.0
    );
    println!(
        "Tolerancias: {}",
        if report.passes_pointwise(2.0, 3.5, 0.15, 0.35, 0.15, 0.40) {
            "OK (SPL<=2 dB rms, 3.5 max; Ze<=15% rms/35% max; Xd<=15% rms/40% max)"
        } else {
            "FUERA DE TOLERANCIA"
        }
    );

    // Resumen por bandas.
    for (lo, hi) in [
        (20.0, 70.0),
        (70.0, 300.0),
        (300.0, 1000.0),
        (1000.0, 10000.0),
    ] {
        let points: Vec<_> = report
            .points
            .iter()
            .filter(|p| p.freq_hz >= lo && p.freq_hz < hi)
            .collect();
        if points.is_empty() {
            continue;
        }
        let n = points.len() as f64;
        let spl = (points
            .iter()
            .map(|p| p.spl_error_db * p.spl_error_db)
            .sum::<f64>()
            / n)
            .sqrt();
        let ze = (points
            .iter()
            .map(|p| p.ze_rel_error * p.ze_rel_error)
            .sum::<f64>()
            / n)
            .sqrt();
        println!(
            "{lo:6.0}-{hi:6.0} Hz  SPL rms={spl:5.2} dB  Ze={:5.1}%",
            ze * 100.0
        );
    }
}
