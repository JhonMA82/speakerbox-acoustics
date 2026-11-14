//! Comprobación **aritmética** de `sum_at` con dos oráculos reales de un solo
//! driver (OD1 y OD2, export de Hornresp 60.70).
//!
//! No es una validación multivía: los dos diseños se exportaron por separado y
//! nunca se midieron juntos. Lo que se comprueba es que la suma, la ganancia,
//! la polaridad y el retardo componen las presiones de cada vía sin desviarse
//! más que la suma de los errores individuales del modelo frente a su oráculo.
//! Es decir: un fallo de signo, de convención de fase o de retardo rompe la
//! cota; el error físico del modelo queda acotado, no oculto.

use num_complex::Complex64;
use speakerbox_acoustics::{sum_at, Design, Driver, Flare, MultiwayChannel, OffsetHorn, Segment};
use std::f64::consts::TAU;

const P_REF: f64 = 20.0e-6;

fn driver() -> Driver {
    Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1)
}

fn od1() -> Design {
    Design::OffsetHorn(OffsetHorn::new_4seg_od1(
        driver(),
        vec![
            Segment::new(Flare::Conical, 0.25, 300e-4, 500e-4),
            Segment::new(Flare::Conical, 0.35, 500e-4, 500e-4),
            Segment::new(Flare::Conical, 0.90, 500e-4, 1200e-4),
            Segment::new(Flare::Conical, 0.30, 1200e-4, 1600e-4),
        ],
    ))
}

fn od2() -> Design {
    Design::OffsetHorn(OffsetHorn::new_4seg_od2(
        driver(),
        vec![
            Segment::new(Flare::Conical, 0.20, 250e-4, 450e-4),
            Segment::new(Flare::Exponential, 0.30, 450e-4, 700e-4),
            Segment::new(Flare::Conical, 0.90, 700e-4, 700e-4),
            Segment::new(Flare::Conical, 0.60, 700e-4, 1600e-4),
        ],
    ))
}

#[derive(Clone, Copy)]
struct OracleRow {
    freq_hz: f64,
    spl_db: f64,
    velocity_phase_deg: f64,
}

impl OracleRow {
    fn pressure_pa(&self) -> Complex64 {
        // La fase de presión física añade +90° a la fase de velocidad.
        Complex64::from_polar(
            P_REF * 10.0_f64.powf(self.spl_db / 20.0),
            (self.velocity_phase_deg + 90.0).to_radians(),
        )
    }
}

fn load_oracle(path: &str) -> Vec<OracleRow> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let mut rows = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if index == 0 || line.trim().is_empty() {
            continue;
        }
        let cells: Vec<&str> = line.split('\t').map(str::trim).collect();
        assert!(cells.len() >= 16, "{path}: línea {} incompleta", index + 1);
        let parse = |cell: &str| cell.replace(',', ".").parse::<f64>();
        rows.push(OracleRow {
            freq_hz: parse(cells[0]).expect("frecuencia"),
            spl_db: parse(cells[4]).expect("SPL"),
            velocity_phase_deg: parse(cells[7]).expect("WPhase"),
        });
    }
    rows
}

#[test]
fn arithmetic_sum_with_real_single_driver_oracles() {
    let oracle_a = load_oracle("tests/data/od1_b_oracle.txt");
    let oracle_b = load_oracle("tests/data/od2_c_oracle.txt");
    let design_a = od1();
    let design_b = od2();

    let gain = 0.5;
    let polarity = -1.0;
    let delay_s = 1.0e-3;
    let mut evaluated = 0_usize;
    let mut worst_margin = f64::NEG_INFINITY;
    let mut worst_relative_bound = 0.0_f64;
    let mut spl_rms_a = 0.0;
    let mut spl_rms_b = 0.0;

    for (index, (row_a, row_b)) in oracle_a.iter().zip(&oracle_b).enumerate() {
        let freq = row_a.freq_hz;
        assert!((freq - row_b.freq_hz).abs() < 1e-9, "rejillas distintas");
        if !(10.0..=1000.0).contains(&freq) || index % 8 != 0 {
            continue;
        }
        evaluated += 1;

        let channels = [MultiwayChannel::new(&design_a)];
        let model_a = sum_at(&channels, freq).unwrap().pressure_pa.unwrap();
        let model_b = sum_at(&[MultiwayChannel::new(&design_b)], freq)
            .unwrap()
            .pressure_pa
            .unwrap();
        let weighted_b = MultiwayChannel::new(&design_b)
            .with_gain(gain)
            .with_polarity(-1)
            .with_delay_s(delay_s);
        let model_sum = sum_at(&[MultiwayChannel::new(&design_a), weighted_b], freq)
            .unwrap()
            .pressure_pa
            .unwrap();

        let oracle_a_p = row_a.pressure_pa();
        let oracle_b_p = row_b.pressure_pa();
        let rotation = Complex64::from_polar(1.0, -TAU * freq * delay_s);
        let oracle_sum = oracle_a_p + oracle_b_p * (gain * polarity) * rotation;

        // La composición con los parámetros declarados es una identidad exacta
        // sobre las presiones del modelo; un signo o un retardo mal aplicado
        // la rompe.
        let manual_sum = model_a + model_b * (gain * polarity) * rotation;
        let identity_error = (model_sum - manual_sum).norm();
        assert!(
            identity_error <= 1e-12 * manual_sum.norm().max(1e-30),
            "f={freq}: composición no coincide con la suma manual ({identity_error:e})"
        );

        // Cerca de un nulo la suma es hipersensible a los errores individuales:
        // se acota la comparación con el oráculo a puntos no cancelados.
        if oracle_sum.norm() < 0.5 * (oracle_a_p.norm() + gain * oracle_b_p.norm()) {
            continue;
        }

        let error_a = (model_a - oracle_a_p).norm();
        let error_b = (model_b - oracle_b_p).norm();
        let bound = error_a + gain * error_b + 1e-12 * oracle_sum.norm();
        let deviation = (model_sum - oracle_sum).norm();
        assert!(
            deviation <= bound,
            "f={freq}: |suma_modelo−suma_oráculo|={deviation:e} > cota {bound:e}"
        );
        worst_margin = worst_margin.max(bound - deviation);
        worst_relative_bound = worst_relative_bound.max(bound / oracle_sum.norm());

        // Los oráculos corresponden de verdad a los diseños: sin esto la cota
        // sería vacua con errores enormes.
        let spl_a = 20.0 * (model_a.norm() / P_REF).log10();
        let spl_b = 20.0 * (model_b.norm() / P_REF).log10();
        spl_rms_a += (spl_a - row_a.spl_db).powi(2);
        spl_rms_b += (spl_b - row_b.spl_db).powi(2);
        assert!((spl_a - row_a.spl_db).abs() <= 3.5, "OD1 a {freq} Hz");
        assert!((spl_b - row_b.spl_db).abs() <= 3.5, "OD2 a {freq} Hz");
    }

    let count = evaluated as f64;
    assert!(evaluated >= 10, "pocos puntos evaluados: {evaluated}");
    let rms_a = (spl_rms_a / count).sqrt();
    let rms_b = (spl_rms_b / count).sqrt();
    println!(
        "oráculos reales: {evaluated} puntos, SPL RMS OD1={rms_a:.2} dB OD2={rms_b:.2} dB, \
         margen mínimo de la cota={worst_margin:.3e}, cota relativa máxima={worst_relative_bound:.3}"
    );
    assert!(rms_a <= 2.0 && rms_b <= 2.0);
}
