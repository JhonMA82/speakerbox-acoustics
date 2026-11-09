//! Validación de producción de OD, OD1 y OD2 contra exports propios de
//! Validación contra el software de referencia 60.70.
//!
//! Tolerancias puntuales:
//! - SPL: RMS ≤ 2.0 dB, máximo ≤ 3.5 dB.
//! - Ze:  RMS ≤ 15 %, máximo ≤ 35 %.
//! - Xd:  RMS ≤ 15 %, máximo ≤ 40 % (se ignoran los puntos con Xd ≤ 0.02 mm).
//!
//! La banda de producción de un subwoofer de bocina se evalúa de 10 Hz a
//! 1 kHz, igual que las validaciones de BP y TH.

use speakerbox_acoustics::{Driver, Flare, OffsetHorn, OraclePoint, Segment};

fn driver() -> Driver {
    Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1)
}

fn od() -> OffsetHorn {
    OffsetHorn::new_4seg_od(
        driver(),
        vec![
            Segment::new(Flare::Conical, 0.40, 400e-4, 400e-4),
            Segment::new(Flare::Exponential, 0.20, 400e-4, 400e-4),
            Segment::new(Flare::Conical, 1.20, 400e-4, 1200e-4),
            Segment::new(Flare::Conical, 0.30, 1200e-4, 1500e-4),
        ],
    )
}

fn od1() -> OffsetHorn {
    OffsetHorn::new_4seg_od1(
        driver(),
        vec![
            Segment::new(Flare::Conical, 0.25, 300e-4, 500e-4),
            Segment::new(Flare::Conical, 0.35, 500e-4, 500e-4),
            Segment::new(Flare::Conical, 0.90, 500e-4, 1200e-4),
            Segment::new(Flare::Conical, 0.30, 1200e-4, 1600e-4),
        ],
    )
}

fn od2() -> OffsetHorn {
    OffsetHorn::new_4seg_od2(
        driver(),
        vec![
            Segment::new(Flare::Conical, 0.20, 250e-4, 450e-4),
            Segment::new(Flare::Exponential, 0.30, 450e-4, 700e-4),
            Segment::new(Flare::Conical, 0.90, 700e-4, 700e-4),
            Segment::new(Flare::Conical, 0.60, 700e-4, 1600e-4),
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

fn metrics(od: &OffsetHorn, oracle: &[OraclePoint]) -> Metrics {
    let (mut es, mut ez, mut ex, mut nz, mut nx) = (0.0, 0.0, 0.0, 0.0, 0.0);
    let (mut ms, mut mz, mut mx) = (0.0f64, 0.0f64, 0.0f64);
    for o in oracle {
        if o.freq_hz > 1000.0 {
            continue;
        }
        let p = od.response_at(o.freq_hz);

        let ds = p.spl_db - o.spl_db;
        es += ds * ds;
        ms = ms.max(ds.abs());

        let dz = (p.ze_mag - o.ze_mag) / o.ze_mag;
        ez += dz * dz;
        mz = mz.max(dz.abs());
        nz += 1.0;

        if o.xd_mm > 0.02 {
            let dx = (p.displacement_mm - o.xd_mm) / o.xd_mm;
            ex += dx * dx;
            mx = mx.max(dx.abs());
            nx += 1.0;
        }
    }
    Metrics {
        spl_rms: (es / nz).sqrt(),
        spl_max: ms,
        ze_rms_pct: (ez / nz).sqrt() * 100.0,
        ze_max_pct: mz * 100.0,
        xd_rms_pct: (ex / nx).sqrt() * 100.0,
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

fn run(name: &str, od: OffsetHorn, oracle_path: &str) {
    let text = std::fs::read_to_string(oracle_path)
        .unwrap_or_else(|e| panic!("no se pudo leer {oracle_path}: {e}"));
    let oracle = OraclePoint::parse_tsv(&text);
    assert_production(name, metrics(&od, &oracle));
}

#[test]
fn od_a_meets_production_tolerances() {
    run("OD-A", od(), "tests/data/od_a_oracle.txt");
}

#[test]
fn od1_b_meets_production_tolerances() {
    run("OD1-B", od1(), "tests/data/od1_b_oracle.txt");
}

#[test]
fn od2_c_meets_production_tolerances() {
    run("OD2-C", od2(), "tests/data/od2_c_oracle.txt");
}
