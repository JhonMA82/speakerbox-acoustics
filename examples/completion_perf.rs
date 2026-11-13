//! New-capability release benchmark; reports elapsed time AND result accuracy.
use speakerbox_acoustics::optimization::*;
use speakerbox_acoustics::*;
use std::{hint::black_box, time::Instant};
fn driver() -> Driver {
    Driver::from_ui_units(350., 4e-4, 20., 6., 18., 4., 1., 1)
}
fn main() {
    for (name, variable, rf) in [
        ("TL uniform", false, 0.),
        ("TL variable", true, 0.),
        ("TL damped", false, 1000.),
        ("TL variable damped", true, 1000.),
    ] {
        let s = if variable {
            Segment::conical(1., 0.02, 0.04)
        } else {
            Segment::new(Flare::Cylindrical, 1., 0.02, 0.02)
        }
        .with_flow_resistivity(rf);
        let t = TransmissionLine::at_position(driver(), vec![s], 0.33).unwrap();
        for n in [500, 2000] {
            let start = Instant::now();
            let mut residual = 0_f64;
            for i in 0..n {
                let f = 20. * 10_f64.powf(i as f64 / (n - 1) as f64);
                let p = t.try_response_at(f).unwrap();
                black_box(p);
            }
            let elapsed = start.elapsed().as_secs_f64() * 1000.;
            for f in [20., 50., 100., 200.] {
                let e = t.try_energy_at(f).unwrap();
                residual = residual.max(e.balance_error_w().abs() / e.source_active_w);
            }
            println!("{name},{n},{elapsed:.6},energy_relative_max={residual:.9e}");
        }
    }
    let s = Segment::new(Flare::Cylindrical, 0.8, 0.02, 0.02).with_flow_resistivity(1000.);
    let start = Instant::now();
    for _ in 0..10000 {
        black_box(s.try_matrix(&Air::default(), 100.).unwrap());
    }
    println!(
        "complex uniform matrix,10000,{:.6}",
        start.elapsed().as_secs_f64() * 1000.
    );
    for alg in [Algorithm::NelderMead, Algorithm::DifferentialEvolution] {
        for budget in [100, 1000, 5000] {
            // Nonconvergent objective over an irrational smooth landscape, record actual
            // evaluations rather than asserting every search consumes its whole budget.
            let start = Instant::now();
            let r = minimize(
                &[Bound {
                    min: -5.,
                    max: 5.,
                    locked: None,
                }; 3],
                &[3., 2., 1.],
                SearchConfig {
                    algorithm: alg,
                    max_evaluations: budget,
                    max_iterations: 10000,
                    tolerance: 1e-12,
                    seed: 42,
                    population: 24,
                },
                |x| Some(x.iter().map(|v| v * v).sum()),
                || false,
            )
            .unwrap();
            println!(
                "optimizer {alg:?},budget={budget},actual={},ms={:.6},cost={:.9e},stop={:?}",
                r.evaluations,
                start.elapsed().as_secs_f64() * 1000.,
                r.best.unwrap().cost,
                r.stop
            );
        }
    }
    // Three-dimensional acoustic fit with an interior target. Report actual
    // evaluations if convergence occurs before a requested upper budget.
    let frequencies = [20., 50., 100.];
    let make = |x: &[f64]| {
        let mut s = System::new(Air::default(), driver());
        s.rear_chamber = Some(Chamber::new(x[0]));
        s.eg = x[1];
        s.rg = x[2];
        s
    };
    let reference = make(&[0.025, 2.83, 2.]);
    let targets: Vec<_> = frequencies
        .iter()
        .map(|&f| reference.try_response_at(f).unwrap().spl_db)
        .collect();
    for budget in [100, 1000, 5000] {
        let mut n = 0;
        let start = Instant::now();
        let r = minimize(
            &[
                Bound {
                    min: 0.005,
                    max: 0.1,
                    locked: None,
                },
                Bound {
                    min: 1.,
                    max: 4.,
                    locked: None,
                },
                Bound {
                    min: 0.,
                    max: 4.,
                    locked: None,
                },
            ],
            &[0.05, 2., 1.],
            SearchConfig {
                algorithm: Algorithm::DifferentialEvolution,
                max_evaluations: budget,
                max_iterations: 10000,
                tolerance: 1e-14,
                seed: 42,
                population: 24,
            },
            |x| {
                n += 1;
                let s = make(x);
                let mut cost = 0.;
                for (&f, &target) in frequencies.iter().zip(&targets) {
                    cost += (s.try_response_at(f).ok()?.spl_db - target).powi(2);
                }
                Some(cost)
            },
            || false,
        )
        .unwrap();
        println!(
            "acoustic DE,budget={budget},actual={n},ms={:.6},cost={:.9e}",
            start.elapsed().as_secs_f64() * 1000.,
            r.best.unwrap().cost
        );
    }
}
