//! Tiempos de geometría y propagación para las capacidades nuevas.
use std::{f64::consts::TAU, hint::black_box, time::Instant};

use speakerbox_acoustics::{Air, HornProfileKind as Kind, HornProfileSpec, ThermoviscousAir};

fn main() {
    const COUNT: usize = 500;
    let air = Air::new(343.0, 1.225);
    println!("profile,samples,resolve_and_sample_ms,acoustic_sweep_ms,max_abs_z");
    for (name, kind, mouth_radius_m, thermoviscous) in [
        ("tube", Kind::Cylindrical, 0.025, false),
        ("tube_thermoviscous", Kind::Cylindrical, 0.025, true),
        ("cone", Kind::Conical, 0.12, false),
        ("os", Kind::OblateSpheroidal, 0.12, false),
        ("tractrix", Kind::Tractrix, 0.12, false),
        ("scaled_tractrix", Kind::ScaledTractrix, 0.12, false),
        (
            "compound",
            Kind::ExponentialConical { join_fraction: 0.3 },
            0.12,
            false,
        ),
    ] {
        let spec = HornProfileSpec {
            kind,
            length_m: 0.3,
            throat_radius_m: 0.025,
            mouth_radius_m,
        };
        let start = Instant::now();
        let profile = spec.resolve().unwrap();
        black_box(profile.sample(COUNT).unwrap());
        let geometry = start.elapsed().as_secs_f64() * 1000.0;
        let mut horn = profile.into_horn();
        if thermoviscous {
            horn.segments[0] = horn.segments[0]
                .clone()
                .with_thermoviscous_losses(ThermoviscousAir::default());
        }
        let start = Instant::now();
        let mut max: f64 = 0.0;
        for i in 0..COUNT {
            let f = 25.0 * (630.0_f64 / 25.0).powf(i as f64 / (COUNT - 1) as f64);
            let z = horn.input_impedance(&air, f, TAU).norm();
            assert!(z.is_finite() && z > 0.0);
            max = max.max(black_box(z));
        }
        let sweep = start.elapsed().as_secs_f64() * 1000.0;
        println!("{name},{COUNT},{geometry:.6},{sweep:.6},{max:.12e}");
    }
}
