//! Validación de precisión de producción contra el registro por defecto de
//! Validación contra el software de referencia 60.70.
//!
//! Datos: `tests/data/default_record.bin` (registro real de 734 bytes) y
//! `tests/data/default_oracle.csv` (44 puntos exportados por el software de referencia con
//! `File > Export > All Chart Data`, 20 Hz–10 kHz).
//!
//! Criterio: SPL RMS ≤ 2.0 dB, SPL máximo ≤ 3.5 dB, Ze relativo ≤ 30 %,
//! Xd relativo ≤ 30 %. Con la calibración `validated_60_70()` se obtiene
//! SPL RMS ≈ 1.35 dB, Ze ≈ 20 %, Xd ≈ 15 %.

use speakerbox_acoustics::{validate, Calibration, LegacyRecord, OraclePoint};

fn read_data(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/data/{}", env!("CARGO_MANIFEST_DIR"), name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("no se pudo leer {path}: {e}"))
}

#[test]
fn default_record_meets_production_tolerances() {
    let record = LegacyRecord::parse(&read_data("default_record.bin")).unwrap();
    let csv = String::from_utf8(read_data("default_oracle.csv")).unwrap();
    let oracle = OraclePoint::parse_csv(&csv);
    assert!(oracle.len() >= 40, "datos dorados insuficientes");

    let system = record.to_system_with(&Calibration::validated_60_70());
    let report = validate(&system, &oracle);

    println!(
        "Validación del software de referencia 60.70: N={} SPL rms={:.2} dB max={:.2} dB | Ze rel={:.1}% max={:.1}% | Xd rel={:.1}% max={:.1}%",
        report.n,
        report.spl_rmse_db,
        report.spl_max_abs_db,
        report.ze_rel_rmse * 100.0,
        report.ze_max_rel * 100.0,
        report.xd_rel_rmse * 100.0,
        report.xd_max_rel * 100.0
    );

    assert!(
        report.passes_pointwise(2.0, 3.5, 0.15, 0.35, 0.15, 0.40),
        "fuera de tolerancias: {report:?}"
    );
}

#[test]
fn calibration_reduces_error_versus_uncalibrated() {
    let record = LegacyRecord::parse(&read_data("default_record.bin")).unwrap();
    let csv = String::from_utf8(read_data("default_oracle.csv")).unwrap();
    let oracle = OraclePoint::parse_csv(&csv);

    let raw = validate(&record.to_system(), &oracle);
    let cal = validate(
        &record.to_system_with(&Calibration::validated_60_70()),
        &oracle,
    );

    println!(
        "Sin calibrar: SPL rms={:.2} dB ; calibrado: {:.2} dB",
        raw.spl_rmse_db, cal.spl_rmse_db
    );
    assert!(
        cal.spl_rmse_db < raw.spl_rmse_db,
        "la calibración debe reducir el error"
    );
}

#[test]
fn second_design_meets_production_tolerances() {
    // Segundo diseño real, creado en la UI del software de referencia con Add + edición:
    // S1=100, S2=500, S3=3000 cm²; L12=50, L23=80 cm; Mmd=30 g.
    // El .dat de la app no persistió los campos editados (siguió guardando el
    // registro 1), pero el cálculo y el export `design2_oracle.txt` usaron los
    // valores mostrados en la UI; `design2_record.bin` los reconstruye.
    // La misma calibración global del diseño 1 se aplica sin reajustar.
    let record = LegacyRecord::parse(&read_data("design2_record.bin")).unwrap();
    let tsv = String::from_utf8(read_data("design2_oracle.txt")).unwrap();
    let oracle = OraclePoint::parse_tsv(&tsv);
    assert!(oracle.len() >= 500, "export completo incompleto");

    let system = record.to_system_with(&Calibration::validated_60_70());
    let report = validate(&system, &oracle);

    println!(
        "Diseño 2: N={} SPL rms={:.2} dB max={:.2} dB | Ze rel={:.1}% max={:.1}% | Xd rel={:.1}% max={:.1}%",
        report.n,
        report.spl_rmse_db,
        report.spl_max_abs_db,
        report.ze_rel_rmse * 100.0,
        report.ze_max_rel * 100.0,
        report.xd_rel_rmse * 100.0,
        report.xd_max_rel * 100.0
    );

    assert!(
        report.passes_pointwise(2.0, 3.5, 0.15, 0.35, 0.15, 0.40),
        "diseño 2 fuera de tolerancias: {report:?}"
    );
}

#[test]
fn third_design_meets_production_tolerances() {
    // Tercer diseño real, exportado con la UI del software de referencia:
    // S1=120, S2=600, S3=3200 cm²; L12=55, L23=70 cm;
    // Sd=400 cm², Mmd=25 g; Vrc=18 L, Vtc=1200 cc, Atc=400 cm².
    let record = LegacyRecord::parse(&read_data("design3_record.bin")).unwrap();
    let tsv = String::from_utf8(read_data("design3_oracle.txt")).unwrap();
    let oracle = OraclePoint::parse_tsv(&tsv);
    assert!(oracle.len() >= 500, "export completo incompleto");

    let system = record.to_system_with(&Calibration::validated_60_70());
    let report = validate(&system, &oracle);

    println!(
        "Diseño 3: N={} SPL rms={:.2} dB max={:.2} dB | Ze rel={:.1}% max={:.1}% | Xd rel={:.1}% max={:.1}%",
        report.n,
        report.spl_rmse_db,
        report.spl_max_abs_db,
        report.ze_rel_rmse * 100.0,
        report.ze_max_rel * 100.0,
        report.xd_rel_rmse * 100.0,
        report.xd_max_rel * 100.0
    );

    assert!(
        report.passes_pointwise(2.0, 3.5, 0.15, 0.35, 0.15, 0.40),
        "diseño 3 fuera de tolerancias: {report:?}"
    );
}
