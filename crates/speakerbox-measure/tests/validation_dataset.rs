//! Pruebas del validador de dataset con datos sintéticos etiquetados como tales.

use num_complex::Complex64;
use speakerbox_circuits::ComplexCurve;
use speakerbox_measure::{
    load_dataset, parse_meta, validate_curves, validate_dataset, write_frd, DatasetError,
};
use std::path::PathBuf;

fn curve(freqs: &[f64], value: Complex64) -> ComplexCurve {
    ComplexCurve::new(freqs.to_vec(), vec![value; freqs.len()]).unwrap()
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("speakerbox-multiway-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn write(&self, name: &str, text: &str) {
        std::fs::write(self.0.join(name), text).unwrap();
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write_curve(directory: &TempDir, name: &str, curve: &ComplexCurve) {
    directory.write(name, &write_frd(curve).unwrap());
}

#[test]
fn synthetic_dataset_passes_with_zero_error() {
    let directory = TempDir::new("pass");
    let freqs = [100.0, 200.0, 400.0, 800.0, 1600.0, 3200.0];
    let woofer = curve(&freqs, Complex64::new(0.1, 0.0));
    let tweeter = curve(&freqs, Complex64::new(0.05, 0.0));
    let (gain, polarity, delay) = (0.5, -1.0, 1.0e-4);
    let system = ComplexCurve::new(
        freqs.to_vec(),
        freqs
            .iter()
            .map(|freq| {
                Complex64::new(0.1, 0.0)
                    + Complex64::new(0.05, 0.0)
                        * (gain * polarity)
                        * Complex64::from_polar(1.0, -std::f64::consts::TAU * freq * delay)
            })
            .collect(),
    )
    .unwrap();
    write_curve(&directory, "woofer.frd", &woofer);
    write_curve(&directory, "tweeter.frd", &tweeter);
    write_curve(&directory, "system.frd", &system);
    directory.write(
        "meta.txt",
        "distance_m=1.0\nvoltage_rms=2.83\npolarity=-1\ngain=0.5\ndelay_s=0.0001\n",
    );

    let dataset = load_dataset(&directory.0).unwrap();
    let report = validate_dataset(&dataset).unwrap();
    assert!(report.passed, "{report:?}");
    assert!(report.rmse_db < 1e-9);
    assert!(report.max_phase_deg < 1e-9);
}

#[test]
fn mismatched_system_fails() {
    let directory = TempDir::new("fail");
    let freqs = [100.0, 200.0, 400.0, 800.0, 1600.0, 3200.0];
    let woofer = curve(&freqs, Complex64::new(0.1, 0.0));
    let tweeter = curve(&freqs, Complex64::new(0.05, 0.0));
    let system = curve(&freqs, Complex64::new(0.3, 0.0));
    write_curve(&directory, "woofer.frd", &woofer);
    write_curve(&directory, "tweeter.frd", &tweeter);
    write_curve(&directory, "system.frd", &system);
    directory.write("meta.txt", "polarity=1\ngain=1\ndelay_s=0\n");
    let report = validate_dataset(&load_dataset(&directory.0).unwrap()).unwrap();
    assert!(!report.passed);
}

#[test]
fn out_of_band_channels_are_an_error() {
    let woofer = curve(&[200.0, 400.0, 800.0, 1600.0], Complex64::new(0.1, 0.0));
    let tweeter = curve(&[400.0, 800.0, 1600.0, 3200.0], Complex64::new(0.05, 0.0));
    let system = curve(
        &[100.0, 200.0, 400.0, 800.0, 1600.0, 3200.0],
        Complex64::new(0.12, 0.0),
    );
    let error = validate_curves(&woofer, &tweeter, &system, &Default::default()).unwrap_err();
    assert!(error.message.contains("fuera de banda"), "{error}");
}

#[test]
fn meta_rejects_unknown_keys_and_invalid_values() {
    assert!(parse_meta("polarity=0\n").is_err());
    assert!(parse_meta("gain=-1\n").is_err());
    assert!(parse_meta("delay_s=-0.1\n").is_err());
    assert!(parse_meta("desconocida=1\n").is_err());
    assert!(parse_meta("distance_m=uno\n").is_err());
    assert!(parse_meta("voltage_rms=2.83\ntime_reference=mic\nnote=prueba\n").is_ok());
    assert_eq!(
        parse_meta("polarity=-1\ngain=0.5\ndelay_s=0.0001\n")
            .unwrap()
            .polarity,
        -1
    );
    let error: DatasetError = parse_meta("sin_igual").unwrap_err();
    assert!(error.message.contains("clave=valor"));
}
