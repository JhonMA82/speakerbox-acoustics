//! Mide presión, caudal, Ze y potencia de la red y el BP4 especializado.
use num_complex::Complex64;
use speakerbox_acoustics::{
    network::{topology::NetworkLayout, NetworkDriver},
    Air, Bandpass, Driver, EndCorrection, Flare, Port, Segment,
};
use std::{f64::consts::TAU, hint::black_box, time::Instant};

fn measure<T>(name: &str, run: impl Fn(f64) -> T) {
    let start = Instant::now();
    for i in 0..2000 {
        let f = 10.0 * 100_f64.powf(i as f64 / 1999.0);
        black_box(run(f));
    }
    println!("{name},{:.6}", start.elapsed().as_secs_f64() * 1000.0);
}

fn main() {
    let air = Air::default();
    let driver = Driver::new(0.02112, 0.000591, 0.03143, 5.9, 10.65, 4.11, 0.001);
    let bp4 = Bandpass::new_bp4(
        driver,
        0.022,
        0.016,
        Port::new(0.004, 0.18, EndCorrection::None),
    );
    let p = |length, s1, s2| Segment::new(Flare::Parabolic, length, s1, s2);
    let ph1 = NetworkLayout::paraflex_ph1(
        [
            p(0.4, 0.03, 0.06),
            p(0.8, 0.06, 0.06),
            p(1.2, 0.06, 0.09),
            p(0.6, 0.09, 0.09),
        ],
        [
            Segment::conical(0.30, 0.005, 0.000104),
            Segment::conical(0.001, 0.000104, 0.0001),
            Segment::conical(3.0, 0.0001, 0.2),
        ],
    )
    .unwrap();
    let bp4_layout =
        NetworkLayout::from_json(include_str!("../docs/wavebox-integration/bp4-network.json"))
            .unwrap();
    measure("bp4_legacy", |f| {
        (
            bp4.try_solution_at(f).unwrap(),
            bp4.try_energy_at(f).unwrap(),
        )
    });
    for (label, layout) in [("bp4_network", bp4_layout), ("ph1_network", ph1)] {
        let drive = NetworkDriver {
            driver,
            front: layout.front,
            rear: layout.rear,
            voltage_rms: Complex64::from(2.83),
            source_resistance_ohm: 0.0,
            solid_angle: TAU,
            mechanical_damping: None,
        };
        measure(label, |f| {
            layout.network.solve_driver(&air, f, &drive).unwrap()
        });
    }
}
