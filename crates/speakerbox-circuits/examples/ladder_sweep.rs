//! Barrido de impedancia de un filtro paso bajo de segundo orden con carga.
//!
//! Ejecutar con:
//!
//! ```text
//! cargo run -p speakerbox-circuits --example ladder_sweep
//! ```

use num_complex::Complex64;
use speakerbox_circuits::{Circuit, Component, NodeId, VoltageSource};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Paso bajo: R serie 4.7 Ω + L 1 mH, C 5 µF en paralelo con la carga.
    // Carga: impedancia medida de dos puntos (fuera de banda se rechaza).
    let mut circuit = Circuit::new(4);
    circuit.push_source(VoltageSource::new(
        NodeId(1),
        NodeId::GROUND,
        Complex64::new(1.0, 0.0),
    ))?;
    circuit.push_component(Component::resistor(NodeId(1), NodeId(2), 4.7))?;
    circuit.push_component(Component::inductor(NodeId(2), NodeId(3), 1e-3, 0.05))?;
    circuit.push_component(Component::capacitor(NodeId(3), NodeId::GROUND, 5e-6))?;
    circuit.push_component(Component::load(
        NodeId(3),
        NodeId::GROUND,
        Complex64::new(6.4, 2.1),
    ))?;

    println!("f/Hz\t|Zin|/ohm\targ(Zin)/deg\t|Vout/Vin|");
    for freq in [100.0, 500.0, 1000.0, 2000.0, 5000.0] {
        let z = circuit.input_impedance(freq)?;
        let transfer = circuit.voltage_transfer(freq, NodeId(3), NodeId(1))?;
        println!(
            "{freq:.0}\t{:.4}\t{:.2}\t{:.4}",
            z.norm(),
            z.arg().to_degrees(),
            transfer.norm()
        );
    }
    Ok(())
}
