//! Same source can be built at base/head; accuracy compared to constant operator exponential.
use num_complex::Complex64 as C;
use speakerbox_acoustics::{Air, Flare, Segment};
use std::{hint::black_box, time::Instant};
fn main() {
    let air = Air::default();
    let f = 100.;
    let length = 0.8;
    let rf = 1000.;
    let area = 0.02;
    let s = Segment::new(Flare::Cylindrical, length, area, area).with_flow_resistivity(rf);
    let w = std::f64::consts::TAU * f;
    let a = C::new(-rf, -w * air.rho) / area;
    let b = C::new(0., -w * area / (air.rho * air.c * air.c));
    let g = (a * b).sqrt();
    let exact_b = a * (g * length).sinh() / g;
    let start = Instant::now();
    for _ in 0..10000 {
        black_box(s.matrix(&air, f));
    }
    println!(
        "uniform Darcy,10000,ms={:.6},relative_B_error={:.12e}",
        start.elapsed().as_secs_f64() * 1000.,
        (s.matrix(&air, f).b - exact_b).norm() / exact_b.norm()
    );
}
