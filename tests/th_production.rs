//! Validación de producción de los tapped horns TH1 y TH2 contra los exports
//! reales del software de referencia 60.70.
//!
//! Tolerancias de producción (puntuales, no solo RMS):
//! - SPL: RMS ≤ 2.0 dB, máximo ≤ 3.5 dB.
//! - Ze:  RMS ≤ 15 %, máximo ≤ 35 %.
//! - Xd:  RMS ≤ 15 %, máximo ≤ 40 % (se ignoran los puntos con Xd ≤ 0.02 mm).

use speakerbox_acoustics::{Driver, Flare, OraclePoint, Segment, TappedHorn};

fn driver() -> Driver {
    Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1)
}

fn th1() -> TappedHorn {
    TappedHorn::new_4seg_th(
        driver(),
        vec![
            Segment::new(Flare::Conical, 0.40, 400e-4, 400e-4),
            Segment::new(Flare::Exponential, 0.20, 400e-4, 400e-4),
            Segment::new(Flare::Conical, 1.20, 400e-4, 1200e-4),
            Segment::new(Flare::Conical, 0.30, 1200e-4, 1500e-4),
        ],
    )
}

fn th2() -> TappedHorn {
    TappedHorn::new_4seg_th(
        driver(),
        vec![
            Segment::new(Flare::Conical, 0.35, 150e-4, 400e-4),
            Segment::new(Flare::Exponential, 0.25, 400e-4, 400e-4),
            Segment::new(Flare::Conical, 1.00, 400e-4, 900e-4),
            Segment::new(Flare::Conical, 0.30, 900e-4, 1200e-4),
        ],
    )
}

fn th1_s2s3() -> TappedHorn {
    TappedHorn::new_4seg_th1(
        driver(),
        vec![
            Segment::new(Flare::Conical, 0.40, 400e-4, 400e-4),
            Segment::new(Flare::Exponential, 0.20, 400e-4, 400e-4),
            Segment::new(Flare::Conical, 1.20, 400e-4, 1200e-4),
            Segment::new(Flare::Conical, 0.30, 1200e-4, 1500e-4),
        ],
    )
}

#[derive(Debug, Clone, Copy)]
struct Metrics {
    spl_rms: f64,
    spl_max: f64,
    ze_rms_pct: f64,
    ze_max_pct: f64,
    xd_rms_pct: f64,
    xd_max_pct: f64,
}

fn metrics(th: &TappedHorn, oracle: &[OraclePoint]) -> Metrics {
    let (mut es, mut ez, mut ex, mut n) = (0.0, 0.0, 0.0, 0.0);
    let (mut ms, mut mz, mut mx) = (0.0f64, 0.0f64, 0.0f64);
    for o in oracle {
        let p = th.response_at(o.freq_hz);

        let ds = p.spl_db - o.spl_db;
        es += ds * ds;
        ms = ms.max(ds.abs());

        let dz = (p.ze_mag - o.ze_mag) / o.ze_mag;
        ez += dz * dz;
        mz = mz.max(dz.abs());

        if o.xd_mm > 0.02 {
            let dx = (p.displacement_mm - o.xd_mm) / o.xd_mm;
            ex += dx * dx;
            mx = mx.max(dx.abs());
        }
        n += 1.0;
    }
    Metrics {
        spl_rms: (es / n).sqrt(),
        spl_max: ms,
        ze_rms_pct: (ez / n).sqrt() * 100.0,
        ze_max_pct: mz * 100.0,
        xd_rms_pct: (ex / n).sqrt() * 100.0,
        xd_max_pct: mx * 100.0,
    }
}

fn assert_production(name: &str, m: Metrics) {
    println!(
        "{name}: SPL {:.2}/{:.2} dB | Ze {:.1}/{:.1} % | Xd {:.1}/{:.1} %",
        m.spl_rms, m.spl_max, m.ze_rms_pct, m.ze_max_pct, m.xd_rms_pct, m.xd_max_pct
    );
    assert!(m.spl_rms <= 2.0, "{name}: SPL RMS {:.2} > 2.0", m.spl_rms);
    assert!(m.spl_max <= 3.5, "{name}: SPL max {:.2} > 3.5", m.spl_max);
    assert!(
        m.ze_rms_pct <= 15.0,
        "{name}: Ze RMS {:.1}% > 15%",
        m.ze_rms_pct
    );
    assert!(
        m.ze_max_pct <= 35.0,
        "{name}: Ze max {:.1}% > 35%",
        m.ze_max_pct
    );
    assert!(
        m.xd_rms_pct <= 15.0,
        "{name}: Xd RMS {:.1}% > 15%",
        m.xd_rms_pct
    );
    assert!(
        m.xd_max_pct <= 40.0,
        "{name}: Xd max {:.1}% > 40%",
        m.xd_max_pct
    );
}

#[test]
fn th1_meets_production_tolerances() {
    let oracle = OraclePoint::parse_tsv(
        &std::fs::read_to_string("tests/data/th1_oracle.txt").expect("th1_oracle.txt"),
    )
    .expect("oráculo TSV válido");
    assert_production("TH1", metrics(&th1(), &oracle));
}

#[test]
fn th2_meets_production_tolerances() {
    let oracle = OraclePoint::parse_tsv(
        &std::fs::read_to_string("tests/data/th2_oracle.txt").expect("th2_oracle.txt"),
    )
    .expect("oráculo TSV válido");
    assert_production("TH2", metrics(&th2(), &oracle));
}

#[test]
fn th1_s2s3_meets_production_tolerances() {
    let oracle = OraclePoint::parse_tsv(
        &std::fs::read_to_string("tests/data/th1_s2s3_oracle.txt").expect("th1_s2s3_oracle.txt"),
    )
    .expect("oráculo TSV válido");
    assert_production("TH1-S2S3", metrics(&th1_s2s3(), &oracle));
}
