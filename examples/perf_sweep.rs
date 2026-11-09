//! Medición reproducible de barridos de respuesta.
//!
//! Ejecute:
//!
//! ```bash
//! cargo run --release --example perf_sweep
//! ```
//!
//! El ejemplo no forma parte de la suite de tests: mide y reporta tiempos para
//! una bocina de 4 segmentos, un tapped horn (solver nodal) y un BP6 serie,
//! con barridos de 500 y 2.000 puntos, más cambios sucesivos de parámetro.

use std::hint::black_box;
use std::time::Instant;

use speakerbox_acoustics::{
    Air, Bandpass, Chamber, Design, DesignPoint, Driver, EndCorrection, Flare, Horn, Port, Segment,
    System, TappedHorn,
};

fn driver() -> Driver {
    Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1)
}

fn segments() -> Vec<Segment> {
    vec![
        Segment::new(Flare::Conical, 0.40, 400e-4, 400e-4),
        Segment::new(Flare::Exponential, 0.20, 400e-4, 400e-4),
        Segment::new(Flare::Conical, 1.20, 400e-4, 1200e-4),
        Segment::new(Flare::Conical, 0.30, 1200e-4, 1500e-4),
    ]
}

fn horn_system() -> System {
    let mut sys = System::new(Air::default(), driver());
    sys.solid_angle = std::f64::consts::TAU;
    sys.horn = Some(Horn::new(segments()));
    sys.throat_chamber = Some(Chamber::new(4.0e-3));
    sys.rear_chamber = Some(Chamber::new(14.0e-3));
    sys
}

fn port(area_cm2: f64, length_cm: f64) -> Port {
    Port::new(
        area_cm2 * 1.0e-4,
        length_cm * 1.0e-2,
        EndCorrection::UnflangedBoth,
    )
}

fn measure(name: &str, points: usize, run: impl FnOnce() -> Vec<DesignPoint>) -> f64 {
    let start = Instant::now();
    let out = run();
    let elapsed = start.elapsed();
    black_box(&out);
    let ms = elapsed.as_secs_f64() * 1000.0;
    println!(
        "{name:<28} {points:>6} {ms:>12.3} {:>12.2}",
        ms * 1000.0 / points as f64
    );
    ms
}

fn main() {
    let mode = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    println!("speakerbox-acoustics perf_sweep ({mode}); 20 Hz–200 Hz, barrido logarítmico\n");
    println!(
        "{:<28} {:>6} {:>12} {:>12}",
        "caso", "puntos", "total (ms)", "µs/punto"
    );

    let horn = Design::System(horn_system());
    let th = Design::TappedHorn(TappedHorn::new_4seg_th(driver(), segments()));
    let bp6s = Design::Bandpass(Bandpass::new_bp6_series(
        driver(),
        20.0e-3,
        10.0e-3,
        port(50.0, 10.0),
        port(40.0, 8.0),
    ));

    for points in [500usize, 2_000] {
        measure("System bocina 4 tramos", points, || {
            horn.sweep_log(20.0, 200.0, points)
        });
    }
    for points in [500usize, 2_000] {
        measure("TappedHorn 4 tramos", points, || {
            th.sweep_log(20.0, 200.0, points)
        });
    }
    for points in [500usize, 2_000] {
        measure("Bandpass BP6 serie", points, || {
            bp6s.sweep_log(20.0, 200.0, points)
        });
    }

    // Interacción tipo UI: 100 cambios de volumen con un barrido de 50 puntos.
    let iterations = 100usize;
    let points = 50usize;
    let mut design = Design::System(horn_system());
    let start = Instant::now();
    let mut checksum = 0.0;
    for i in 0..iterations {
        if let Design::System(sys) = &mut design {
            sys.rear_chamber = Some(Chamber::new(10.0e-3 + i as f64 * 1.0e-4));
        }
        let out = design.sweep_log(20.0, 200.0, points);
        checksum += out.iter().map(|p| p.spl_db).sum::<f64>();
    }
    let elapsed = start.elapsed();
    black_box(checksum);
    let ms = elapsed.as_secs_f64() * 1000.0;
    println!(
        "{:<28} {:>6} {ms:>12.3} {:>12.2}",
        "100 cambios × 50 puntos",
        iterations * points,
        ms * 1000.0 / (iterations * points) as f64
    );
    println!("\nTiempo total de los cambios de parámetro: {ms:.3} ms");
}
