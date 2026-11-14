//! Escalera Butterworth de segundo orden y comparación con la analítica.
//!
//! ```text
//! cargo run -p speakerbox-circuits --example crossover_sweep
//! ```

use speakerbox_circuits::{synthesize, FilterFamily, FilterKind, FilterResponse, FilterSpec};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let spec = FilterSpec {
        family: FilterFamily::Butterworth,
        kind: FilterKind::LowPass,
        order: 2,
        cutoff_hz: 2000.0,
        impedance_ohm: 8.0,
    };
    let ladder = synthesize(&spec)?;
    let response = FilterResponse::new(spec.family, spec.kind, spec.order, spec.cutoff_hz)?;
    println!("f/Hz\t|H|/dB (red)\t|H|/dB (analítica)\tfase/deg");
    for ratio in [0.1, 0.25, 0.5, 1.0, 2.0, 4.0, 10.0] {
        let freq = spec.cutoff_hz * ratio;
        let simulated = ladder.normalized_transfer(freq)?;
        let analytic = response.transfer(freq)?;
        println!(
            "{freq:.0}\t{:.6}\t{:.6}\t{:.2}",
            20.0 * simulated.norm().log10(),
            20.0 * analytic.norm().log10(),
            simulated.arg().to_degrees()
        );
    }
    Ok(())
}
