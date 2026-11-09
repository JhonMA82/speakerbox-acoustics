//! Calibración empírica contra el software de referencia 60.70.
//!
//! El motor implementa el modelo físico documentado (Webster, pistón,
//! matrices de transmisión, red electromecánica). Para compensar las
//! simplificaciones que el software de referencia resuelve internamente (modos axiales de la
//! cámara, terminación de la boca y referencia de nivel), se ofrece una
//! calibración empírica obtenida minimizando el error frente al registro por
//! defecto exportado por la propia aplicación.
//!
//! La calibración por defecto es la de [`Calibration::default`] (sin ajustes).
//! Para el registro por defecto del software de referencia 60.70 use
//! [`Calibration::validated_60_70`].

/// Factores de calibración aplicados a un registro del software de referencia.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Calibration {
    /// Factor multiplicativo del volumen de la cámara de garganta (`Vtc`).
    pub throat_volume_factor: f64,
    /// Factor de escala de la impedancia de radiación de la boca de la bocina.
    pub mouth_radiation_scale: f64,
    /// Área efectiva de radiación de la boca como múltiplo del área geométrica.
    pub mouth_radiation_area_factor: f64,
    /// Corrección global del nivel de SPL (dB).
    pub spl_offset_db: f64,
    /// Pérdidas de pared de la bocina (factor adimensional).
    pub horn_loss: f64,
    /// Fuga acústica en la garganta (Pa·s/m³); `None` = sin fuga.
    pub horn_leak: Option<f64>,
    /// Resistencia serie en la garganta (Pa·s/m³); `None` = sin resistencia.
    pub horn_series: Option<f64>,
}

impl Default for Calibration {
    fn default() -> Self {
        Self {
            throat_volume_factor: 1.0,
            mouth_radiation_scale: 1.0,
            mouth_radiation_area_factor: 1.0,
            spl_offset_db: 0.0,
            horn_loss: 0.0,
            horn_leak: None,
            horn_series: None,
        }
    }
}

impl Calibration {
    /// Calibración validada contra el registro por defecto del software de referencia 60.70
    /// (bocina cónica + exponencial, `S1=80 cm²`, `S2=350 cm²`,
    /// `S3=2400 cm²`, `L12=60 cm`, `L23=75 cm`, `Vtc=900 cc`, `Vrc=14 L`).
    ///
    /// Resultados sobre 20 Hz–10 kHz: SPL RMS ≈ 1.35 dB (máx. ≈ 3 dB),
    /// `Ze` RMS ≈ 20 %, `Xd` RMS ≈ 15 %.
    pub fn validated_60_70() -> Self {
        Self {
            // El registro real guarda Vtc = 900 cc = 0.9 L; el ajuste
            // validado usa 1.10 L (factor 1.2222).
            throat_volume_factor: 1.10 / 0.90,
            mouth_radiation_scale: 2.20,
            mouth_radiation_area_factor: 2.00,
            spl_offset_db: -3.10,
            horn_loss: 0.0,
            horn_leak: None,
            horn_series: None,
        }
    }
}
