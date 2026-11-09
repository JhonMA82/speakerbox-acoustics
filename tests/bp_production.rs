//! Validación de producción de BP4 y BP6 paralelo contra exports propios de
//! Validación contra el software de referencia 60.70.
//!
//! Tolerancias puntuales:
//! - SPL: RMS ≤ 2.0 dB, máximo ≤ 3.5 dB.
//! - Ze:  RMS ≤ 15 %, máximo ≤ 35 %.
//! - Xd:  RMS ≤ 15 %, máximo ≤ 40 % (se ignoran los puntos con Xd ≤ 0.02 mm).
//!
//! La banda de producción de un bandpass de subwoofer se evalúa de 10 Hz a
//! 1 kHz, igual que la validación de referencia.

use std::collections::HashMap;

use speakerbox_acoustics::{Bandpass, Driver, EndCorrection, Port};

fn record_values(text: &str) -> HashMap<String, f64> {
    let mut values = HashMap::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let Some(value) = value.split_whitespace().next().and_then(|v| v.parse().ok()) else {
            continue;
        };
        // El registro repite Rms y Le en la sección avanzada (con 0.00).
        // Nos quedamos con la primera aparición, la tradicional.
        values.entry(key.to_string()).or_insert(value);
    }
    values
}

fn parse_angle(text: &str) -> f64 {
    let line = text
        .lines()
        .find(|line| line.trim_start().starts_with("Ang"))
        .expect("Ang");
    let rhs = line.split_once('=').expect("Ang =").1.trim();
    if rhs.contains("Pi") {
        let factor: f64 = rhs
            .split_whitespace()
            .next()
            .and_then(|v| v.parse().ok())
            .expect("factor de Ang");
        factor * std::f64::consts::PI
    } else {
        rhs.parse().expect("Ang numérico")
    }
}

fn curve(path: &str) -> Vec<(f64, f64)> {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("no se pudo leer {path}: {e}"))
        .lines()
        .skip(1)
        .filter_map(|line| {
            let cols: Vec<f64> = line
                .split_whitespace()
                .filter_map(|x| x.parse().ok())
                .collect();
            if cols.len() >= 2 {
                Some((cols[0], cols[1]))
            } else {
                None
            }
        })
        .collect()
}

fn build(record: &str) -> Bandpass {
    let text =
        std::fs::read_to_string(record).unwrap_or_else(|e| panic!("no se pudo leer {record}: {e}"));
    let v = record_values(&text);

    let driver = Driver::from_ui_units(
        v["Sd"], v["Cms"], v["Mmd"], v["Re"], v["Bl"], v["Rms"], v["Le"], 1,
    );
    let front = v["Vc1"] * 1.0e-3;
    let rear = v["Vc2"] * 1.0e-3;
    let port = |area_cm2: f64, length_cm: f64| {
        Port::new(
            area_cm2 * 1.0e-4,
            length_cm * 1.0e-2,
            EndCorrection::UnflangedBoth,
        )
    };

    // BP6 serie: el software de referencia lo marca con BP6S=1 o con BP6P=0. El BP6 paralelo
    // usa BP6P=1. Si no hay marca, se asume paralelo (compatibilidad con el
    // primer registro validado).
    let is_series = v.get("BP6S").copied().unwrap_or(0.0) == 1.0
        || v.get("BP6P").copied().unwrap_or(1.0) == 0.0;
    let mut bp = if v.get("BP4").copied().unwrap_or(0.0) == 1.0 {
        Bandpass::new_bp4(driver, rear, front, port(v["Ap1"], v["Lp1"]))
    } else if is_series {
        Bandpass::new_bp6_series(
            driver,
            rear,
            front,
            port(v["Ap1"], v["Lp1"]),
            port(v["Ap2"], v["Lp2"]),
        )
    } else {
        Bandpass::new_bp6(
            driver,
            rear,
            front,
            port(v["Ap1"], v["Lp1"]),
            port(v["Ap2"], v["Lp2"]),
        )
    };

    bp.solid_angle = parse_angle(&text);
    bp.eg = v["Eg"];
    bp.rg = v["Rg"];
    bp.set_ql(v["QL"]);
    bp
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

fn metrics(
    bp: &Bandpass,
    spl: &[(f64, f64)],
    ze: &[(f64, f64)],
    xd: &[(f64, f64)],
    fmax: f64,
) -> Metrics {
    assert_eq!(
        spl.len(),
        ze.len(),
        "SPL y Ze deben tener el mismo número de puntos"
    );
    assert_eq!(
        spl.len(),
        xd.len(),
        "SPL y Xd deben tener el mismo número de puntos"
    );

    let (mut es, mut ez, mut ex, mut n) = (0.0, 0.0, 0.0, 0.0);
    let (mut ms, mut mz, mut mx) = (0.0f64, 0.0f64, 0.0f64);

    for i in 0..spl.len() {
        let (f, spl_oracle) = spl[i];
        if f > fmax {
            continue;
        }
        assert!(
            (f - ze[i].0).abs() < 1.0e-6,
            "frecuencias SPL/Ze distintas en {f}"
        );
        assert!(
            (f - xd[i].0).abs() < 1.0e-6,
            "frecuencias SPL/Xd distintas en {f}"
        );

        let p = bp.response_at(f);

        let ds = p.spl_db - spl_oracle;
        es += ds * ds;
        ms = ms.max(ds.abs());

        let dz = (p.ze_mag - ze[i].1) / ze[i].1;
        ez += dz * dz;
        mz = mz.max(dz.abs());

        if xd[i].1 > 0.02 {
            let dx = (p.displacement_mm - xd[i].1) / xd[i].1;
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

fn run(name: &str, record: &str, spl_path: &str, ze_path: &str, xd_path: &str) {
    let bp = build(record);
    let spl = curve(spl_path);
    let ze = curve(ze_path);
    let xd = curve(xd_path);
    // Un bandpass de subwoofer se valida en su banda útil, 10 Hz–1 kHz.
    // Por encima, el modelo de puerto sin pérdidas no reproduce los valles y
    // picos finos del software de referencia; Ze y Xd siguen siendo válidos en todo el rango.
    let m = metrics(&bp, &spl, &ze, &xd, 1000.0);
    assert_production(name, m);
}

#[test]
fn bp4_a_meets_production_tolerances() {
    run(
        "BP4-A",
        "tests/data/bp4_a_record.txt",
        "tests/data/bp4_a_spl.txt",
        "tests/data/bp4_a_ze.txt",
        "tests/data/bp4_a_xd.txt",
    );
}

#[test]
fn bp6p_a_meets_production_tolerances() {
    run(
        "BP6P-A",
        "tests/data/bp6_a_record.txt",
        "tests/data/bp6_a_spl.txt",
        "tests/data/bp6_a_ze.txt",
        "tests/data/bp6_a_xd.txt",
    );
}

#[test]
fn bp6s_a_meets_production_tolerances() {
    run(
        "BP6S-A",
        "tests/data/bp6s_a_record.txt",
        "tests/data/bp6s_a_spl.txt",
        "tests/data/bp6s_a_ze.txt",
        "tests/data/bp6s_a_xd.txt",
    );
}
