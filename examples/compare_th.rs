use speakerbox_acoustics::{Driver, Flare, OraclePoint, Segment, TappedHorn};
fn driver() -> Driver {
    Driver::from_ui_units(350.0, 4e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1)
}
fn th1(b: usize) -> TappedHorn {
    let mut t = TappedHorn::new_4seg_th(
        driver(),
        vec![
            Segment::new(Flare::Conical, 0.40, 400e-4, 400e-4),
            Segment::new(Flare::Exponential, 0.20, 400e-4, 400e-4),
            Segment::new(Flare::Conical, 1.20, 400e-4, 1200e-4),
            Segment::new(Flare::Conical, 0.30, 1200e-4, 1500e-4),
        ],
    );
    t.tap_b = b;
    t
}
fn th2(b: usize) -> TappedHorn {
    let mut t = TappedHorn::new_4seg_th(
        driver(),
        vec![
            Segment::new(Flare::Conical, 0.35, 150e-4, 400e-4),
            Segment::new(Flare::Exponential, 0.25, 400e-4, 400e-4),
            Segment::new(Flare::Conical, 1.00, 400e-4, 900e-4),
            Segment::new(Flare::Conical, 0.30, 900e-4, 1200e-4),
        ],
    );
    t.tap_b = b;
    t
}
fn run(name: &str, th: &TappedHorn, path: &str) {
    let oracle = OraclePoint::parse_tsv(&std::fs::read_to_string(path).unwrap());
    let mut es = 0.0;
    let mut ez = 0.0;
    let mut ex = 0.0;
    let mut n = 0.0;
    let mut maxs = 0.0f64;
    let mut maxz = 0.0f64;
    let mut maxx = 0.0f64;
    for o in &oracle {
        let p = th.response_at(o.freq_hz);
        let ds = p.spl_db - o.spl_db;
        es += ds * ds;
        maxs = maxs.max(ds.abs());
        let dz = (p.ze_mag - o.ze_mag) / o.ze_mag;
        ez += dz * dz;
        maxz = maxz.max(dz.abs());
        if o.xd_mm > 0.02 {
            let dx = (p.displacement_mm - o.xd_mm) / o.xd_mm;
            ex += dx * dx;
            maxx = maxx.max(dx.abs());
        }
        n += 1.0;
    }
    println!(
        "{name}: N={} SPL rms={:.2} max={:.2} | Ze {:.1}% max {:.1}% | Xd {:.1}% max {:.1}%",
        n,
        (es / n).sqrt(),
        maxs,
        (ez / n).sqrt() * 100.0,
        maxz * 100.0,
        (ex / n).sqrt() * 100.0,
        maxx * 100.0
    );
}
fn main() {
    run("TH1 (S2/S4)", &th1(3), "tests/data/th1_oracle.txt");
    run("TH2 (S2/S4)", &th2(3), "tests/data/th2_oracle.txt");
}
