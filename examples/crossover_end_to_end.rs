//! Crossover pasivo de segundo orden cargado por la impedancia real de una caja.
//!
//! ```text
//! cargo run --release --example crossover_end_to_end
//! ```
//!
//! Cadena completa entre las dos crates:
//!
//! 1. la caja ventilada del motor acústico entrega su impedancia de bornes
//!    (`Driver::input_impedance` sobre las cargas que devuelve `System::loads`);
//! 2. esa impedancia, compleja y dependiente de la frecuencia, termina el
//!    filtro resuelto por análisis nodal (`speakerbox-circuits`);
//! 3. la transferencia compleja `H = V_carga/V_fuente` escala la presión de la
//!    caja, porque el modelo acústico es lineal en la tensión de bornes:
//!    `SPL filtrado = SPL caja + 20·log|H|`.
//!
//! El filtro se sintetiza para una carga resistiva de 6 Ω y una fuente ideal.
//! La comparación con la carga real muestra el error de diseñar con la
//! impedancia nominal: la inductancia de bobina del driver resuena con el
//! condensador en derivación. Una impedancia de salida de amplificador no
//! pertenece al filtro: se declara en `System::rg`.

use std::error::Error;
use std::f64::consts::{SQRT_2, TAU};

use num_complex::Complex64;
use speakerbox_acoustics::{Air, Chamber, Driver, EndCorrection, Port, System};
use speakerbox_circuits::{
    Circuit, ComplexCurve, Component, FilterFamily, FilterKind, FilterResponse, LoadImpedance,
    NodeId, VoltageSource,
};

/// Banda de la comparación. Las curvas del MNA exigen frecuencias crecientes.
const FREQS: [f64; 11] = [
    20.0, 40.0, 80.0, 150.0, 300.0, 600.0, 1000.0, 1500.0, 2000.0, 3000.0, 4000.0,
];

/// Frecuencia de corte del prototipo Butterworth.
const CUTOFF_HZ: f64 = 2000.0;

/// Impedancia de terminación del prototipo y valor nominal del driver.
const Z_NOMINAL: f64 = 6.0;

fn db(value: f64) -> f64 {
    20.0 * value.log10()
}

/// Caja ventilada de 50 L con el driver de referencia del workspace.
fn boxed_system() -> System {
    let driver = Driver::from_ui_units(350.0, 4.0e-4, 40.0, 6.0, 9.0, 4.0, 1.0, 1);
    let mut system = System::new(Air::default(), driver);
    system.rear_chamber = Some(Chamber::new(50.0e-3));
    system.port = Some(Port::new(50.0e-4, 0.10, EndCorrection::UnflangedBoth));
    system.rear_ql = Some(7.0);
    system
}

/// Butterworth LP2 para fuente ideal y carga `z0`:
/// `H(s) = 1/(s²LC + sL/z0 + 1)`, de donde `L = √2·z0/ωc` y `C = 1/(√2·z0·ωc)`.
fn low_pass_2nd(cutoff_hz: f64, z0: f64) -> (f64, f64) {
    let omega = TAU * cutoff_hz;
    (SQRT_2 * z0 / omega, 1.0 / (SQRT_2 * z0 * omega))
}

/// Fuente ideal de 1 V, inductancia serie, condensador a masa y carga en el nodo 2.
fn filter(inductor: f64, capacitor: f64, load: LoadImpedance) -> Circuit {
    let mut circuit = Circuit::new(3);
    circuit
        .push_source(VoltageSource::new(
            NodeId(1),
            NodeId::GROUND,
            Complex64::new(1.0, 0.0),
        ))
        .expect("fuente ideal de 1 V");
    circuit
        .push_component(Component::inductor(NodeId(1), NodeId(2), inductor, 0.0))
        .expect("inductancia serie");
    circuit
        .push_component(Component::capacitor(NodeId(2), NodeId::GROUND, capacitor))
        .expect("condensador en derivación");
    let component = match load {
        LoadImpedance::Constant(z) => Component::load(NodeId(2), NodeId::GROUND, z),
        LoadImpedance::Curve(curve) => Component::load_curve(NodeId(2), NodeId::GROUND, curve),
    };
    circuit.push_component(component).expect("carga");
    circuit
}

fn main() -> Result<(), Box<dyn Error>> {
    let boxed = boxed_system();
    boxed.validate()?;
    let driver = boxed.driver;
    let air = boxed.air;

    println!(
        "Driver: Fs {:.1} Hz, Qts {:.2}, Vas {:.1} L | caja 50 L, fb {:.1} Hz, QL 7\n",
        driver.fs(&air),
        driver.qts(&air),
        driver.vas(&air) * 1.0e3,
        boxed.rear_tuning_hz().expect("la caja tiene puerto"),
    );

    // 1. Caja sola: SPL de referencia y Z de bornes (sin Rg) en la misma malla.
    let mut z_terminal = Vec::with_capacity(FREQS.len());
    let mut spl_box = Vec::with_capacity(FREQS.len());
    for freq in FREQS {
        let (front, rear) = boxed.loads(freq);
        z_terminal.push(driver.input_impedance(freq, front + rear));
        spl_box.push(boxed.try_response_at(freq)?.spl_db);
    }
    let z_curve = ComplexCurve::new(FREQS.to_vec(), z_terminal.clone())?;

    // 2. Prototipo y red bajo prueba.
    let (inductor, capacitor) = low_pass_2nd(CUTOFF_HZ, Z_NOMINAL);
    println!(
        "Filtro LP2 @ {CUTOFF_HZ} Hz para Z0 = {Z_NOMINAL} Ω: L = {:.3} mH, C = {:.3} µF\n",
        inductor * 1.0e3,
        capacitor * 1.0e6,
    );
    let real = filter(inductor, capacitor, LoadImpedance::Curve(z_curve));
    let nominal = filter(
        inductor,
        capacitor,
        LoadImpedance::Constant(Complex64::new(Z_NOMINAL, 0.0)),
    );
    let analytic =
        FilterResponse::new(FilterFamily::Butterworth, FilterKind::LowPass, 2, CUTOFF_HZ)?;

    // 3. El MNA con carga resistiva debe reproducir el prototipo analítico.
    println!(
        "{:>7} {:>12} {:>11} {:>8}",
        "f/Hz", "|H| MNA 6Ω", "|H| analít.", "error"
    );
    let mut max_error: f64 = 0.0;
    for freq in FREQS {
        let mna = nominal.voltage_transfer(freq, NodeId(2), NodeId(1))?;
        let reference = analytic.transfer(freq)?;
        let error = db(mna.norm()) - db(reference.norm());
        max_error = max_error.max(error.abs());
        println!(
            "{:>7.0} {:>12.3} {:>11.3} {:>8.4}",
            freq,
            db(mna.norm()),
            db(reference.norm()),
            error,
        );
    }
    println!("Error máximo MNA vs analítico: {max_error:.2e} dB\n");

    // 4. Misma red, carga real: el error de la impedancia nominal.
    println!(
        "{:>7} {:>7} {:>7} {:>9} {:>9} {:>7} {:>9} {:>10}",
        "f/Hz", "|Z|/Ω", "∠Z/°", "|H|real", "|H|6Ω", "Δ/dB", "SPL caja", "SPL filtro"
    );
    for (index, freq) in FREQS.iter().copied().enumerate() {
        let z = z_terminal[index];
        let h_real = real.voltage_transfer(freq, NodeId(2), NodeId(1))?;
        let h_nominal = nominal.voltage_transfer(freq, NodeId(2), NodeId(1))?;
        let (gain_real, gain_nominal) = (db(h_real.norm()), db(h_nominal.norm()));
        println!(
            "{:>7.0} {:>7.2} {:>7.1} {:>9.2} {:>9.2} {:>7.2} {:>9.2} {:>10.2}",
            freq,
            z.norm(),
            z.arg().to_degrees(),
            gain_real,
            gain_nominal,
            gain_real - gain_nominal,
            spl_box[index],
            spl_box[index] + gain_real,
        );
    }

    // 5. Impedancia que presenta el conjunto al amplificador.
    println!();
    for freq in [80.0, CUTOFF_HZ] {
        let z_in = real.input_impedance(freq)?;
        println!(
            "Zin(filtro+driver) @ {freq:>5.0} Hz = {:>6.2} Ω ∠ {:>5.1}°",
            z_in.norm(),
            z_in.arg().to_degrees(),
        );
    }
    Ok(())
}
