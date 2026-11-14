//! Circuito equivalente de impedancia de un altavoz.
//!
//! El modelo es el que la auditoría extrae del software histórico, escrito en
//! SI y sin heredar su código:
//!
//! ```text
//! Zeq(f) = (R1 ∥ jωL1a) + (Re + jωL1b) + (R2 ∥ jωL2 ∥ 1/(jωC2))
//! ```
//!
//! Es un modelo **eléctrico medible**: reproduce la curva de impedancia, no
//! afirma qué parte de la física del motor representa cada elemento. Por eso
//! no se convierte a `speakerbox_acoustics::Driver` ni se usa para inventar
//! `Bl`, `Sd` o masa.
//!
//! De la rama resonante salen estimaciones parciales:
//!
//! ```text
//! Fs  = 1 / (2π √(L2·C2))
//! Qms = R2 √(C2/L2)
//! Qes = Qms · Re/R2
//! Qts = Qms·Qes/(Qms+Qes)
//! ```
//!
//! `Qes` es una convención del propio circuito, no una medida independiente:
//! se documenta como estimación parcial, nunca como parámetro T/S completo.

use crate::error::{MeasureError, MeasureResult};
use num_complex::Complex64;
use std::f64::consts::TAU;

/// Nombres de los parámetros, en el orden interno del ajuste.
pub const PARAMETER_NAMES: [&str; 7] = [
    "r1_ohm", "l1a_h", "re_ohm", "l1b_h", "r2_ohm", "l2_h", "c2_f",
];

/// Parámetros del circuito equivalente de impedancia.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EquivalentImpedance {
    r1_ohm: f64,
    l1a_h: f64,
    re_ohm: f64,
    l1b_h: f64,
    r2_ohm: f64,
    l2_h: f64,
    c2_f: f64,
}

/// Estimaciones parciales derivadas de la rama resonante.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TsEstimate {
    /// Frecuencia de resonancia del tanque, en hercios.
    pub fs_hz: f64,
    /// Factor de calidad mecánico del circuito.
    pub qms: f64,
    /// Factor de calidad eléctrico según la convención del circuito.
    pub qes: f64,
    /// Factor de calidad total.
    pub qts: f64,
}

impl EquivalentImpedance {
    /// Construye el modelo validando cada parámetro.
    ///
    /// # Errores
    ///
    /// [`MeasureError::InvalidParameter`] si un valor no es finito o no es
    /// estrictamente positivo; `l1b_h` admite `0.0`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        r1_ohm: f64,
        l1a_h: f64,
        re_ohm: f64,
        l1b_h: f64,
        r2_ohm: f64,
        l2_h: f64,
        c2_f: f64,
    ) -> MeasureResult<Self> {
        let model = Self {
            r1_ohm,
            l1a_h,
            re_ohm,
            l1b_h,
            r2_ohm,
            l2_h,
            c2_f,
        };
        model.validate()?;
        Ok(model)
    }

    /// Resistencia de la rama de pérdidas, en ohmios.
    #[must_use]
    pub fn r1_ohm(&self) -> f64 {
        self.r1_ohm
    }

    /// Inductancia de la rama de pérdidas, en henrios.
    #[must_use]
    pub fn l1a_h(&self) -> f64 {
        self.l1a_h
    }

    /// Resistencia de la bobina, en ohmios.
    #[must_use]
    pub fn re_ohm(&self) -> f64 {
        self.re_ohm
    }

    /// Inductancia serie de la bobina, en henrios.
    #[must_use]
    pub fn l1b_h(&self) -> f64 {
        self.l1b_h
    }

    /// Resistencia del tanque resonante, en ohmios.
    #[must_use]
    pub fn r2_ohm(&self) -> f64 {
        self.r2_ohm
    }

    /// Inductancia del tanque, en henrios.
    #[must_use]
    pub fn l2_h(&self) -> f64 {
        self.l2_h
    }

    /// Capacidad del tanque, en faradios.
    #[must_use]
    pub fn c2_f(&self) -> f64 {
        self.c2_f
    }

    /// Impedancia del modelo a `freq_hz`.
    ///
    /// # Errores
    ///
    /// [`MeasureError::InvalidFrequency`] si la frecuencia no es finita o no es
    /// positiva.
    pub fn impedance(&self, freq_hz: f64) -> MeasureResult<Complex64> {
        if !freq_hz.is_finite() || freq_hz <= 0.0 {
            return Err(MeasureError::InvalidFrequency(freq_hz));
        }
        let omega = TAU * freq_hz;
        let losses = parallel(
            Complex64::new(self.r1_ohm, 0.0),
            Complex64::new(0.0, omega * self.l1a_h),
        );
        let coil = Complex64::new(self.re_ohm, omega * self.l1b_h);
        let tank = recip(
            recip(Complex64::new(self.r2_ohm, 0.0))
                + recip(Complex64::new(0.0, omega * self.l2_h))
                + recip(Complex64::new(0.0, -1.0 / (omega * self.c2_f))),
        );
        let total = losses + coil + tank;
        if !total.re.is_finite() || !total.im.is_finite() {
            return Err(MeasureError::NonFinite("impedancia del modelo"));
        }
        Ok(total)
    }

    /// Estimaciones parciales de T/S desde la rama resonante.
    #[must_use]
    pub fn ts_estimate(&self) -> TsEstimate {
        let fs_hz = 1.0 / (TAU * (self.l2_h * self.c2_f).sqrt());
        let qms = self.r2_ohm * (self.c2_f / self.l2_h).sqrt();
        let qes = qms * self.re_ohm / self.r2_ohm;
        let qts = qms * qes / (qms + qes);
        TsEstimate {
            fs_hz,
            qms,
            qes,
            qts,
        }
    }

    /// Parámetros en el orden de [`PARAMETER_NAMES`].
    pub(crate) fn to_array(self) -> [f64; 7] {
        [
            self.r1_ohm,
            self.l1a_h,
            self.re_ohm,
            self.l1b_h,
            self.r2_ohm,
            self.l2_h,
            self.c2_f,
        ]
    }

    /// Construye desde un array en el orden de [`PARAMETER_NAMES`].
    pub(crate) fn from_array(values: [f64; 7]) -> MeasureResult<Self> {
        Self::new(
            values[0], values[1], values[2], values[3], values[4], values[5], values[6],
        )
    }

    fn validate(&self) -> MeasureResult<()> {
        for (name, value) in [
            ("r1_ohm", self.r1_ohm),
            ("l1a_h", self.l1a_h),
            ("re_ohm", self.re_ohm),
            ("r2_ohm", self.r2_ohm),
            ("l2_h", self.l2_h),
            ("c2_f", self.c2_f),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(MeasureError::InvalidParameter(name));
            }
        }
        if !self.l1b_h.is_finite() || self.l1b_h < 0.0 {
            return Err(MeasureError::InvalidParameter("l1b_h"));
        }
        let ts = self.ts_estimate();
        if !ts.fs_hz.is_finite()
            || !ts.qms.is_finite()
            || !ts.qes.is_finite()
            || !ts.qts.is_finite()
        {
            return Err(MeasureError::InvalidParameter(
                "el circuito produce estimaciones T/S no finitas",
            ));
        }
        Ok(())
    }
}

fn parallel(a: Complex64, b: Complex64) -> Complex64 {
    a * b / (a + b)
}

fn recip(value: Complex64) -> Complex64 {
    Complex64::new(1.0, 0.0) / value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> EquivalentImpedance {
        EquivalentImpedance::new(8.0, 0.02, 6.4, 0.0015, 45.0, 0.09, c2_for(40.0, 0.09)).unwrap()
    }

    #[test]
    fn dc_branch_is_the_coil_resistance() {
        let z = model().impedance(1e-6).unwrap();
        assert!((z - Complex64::new(6.4, 0.0)).norm() < 1e-6);
    }

    fn c2_for(fs_hz: f64, l2_h: f64) -> f64 {
        1.0 / ((TAU * fs_hz).powi(2) * l2_h)
    }

    #[test]
    fn resonance_cancels_the_tank_reactance() {
        let m = model();
        let fs = m.ts_estimate().fs_hz;
        let omega = TAU * fs;
        // En paralelo, L y C se anulan cuando ωC = 1/(ωL): el tanque queda R2.
        let susceptance = omega * m.c2_f() - 1.0 / (omega * m.l2_h());
        assert!(susceptance.abs() < 1e-9, "susceptancia={susceptance}");
        let tank = Complex64::new(m.r2_ohm(), 0.0);
        let total = m.impedance(fs).unwrap();
        let rest = total - tank;
        let expected_rest = parallel(
            Complex64::new(m.r1_ohm(), 0.0),
            Complex64::new(0.0, omega * m.l1a_h()),
        ) + Complex64::new(m.re_ohm(), omega * m.l1b_h());
        assert!((rest - expected_rest).norm() < 1e-9);
    }

    #[test]
    fn ts_relations_match_the_circuit() {
        let ts = model().ts_estimate();
        assert!((ts.fs_hz - 40.0).abs() < 1e-9, "fs={}", ts.fs_hz);
        let expected_qms = 45.0 * (c2_for(40.0, 0.09) / 0.09).sqrt();
        assert!((ts.qms - expected_qms).abs() < 1e-9);
        assert!((ts.qes - expected_qms * 6.4 / 45.0).abs() < 1e-9);
        assert!(ts.qts > 0.0 && ts.qts < ts.qms.min(ts.qes));
    }

    #[test]
    fn parameters_whose_ts_estimates_overflow_are_rejected() {
        assert!(
            EquivalentImpedance::new(8.0, 0.02, 6.4, 0.0015, 1.0e308, 1.0e-308, 1.0e308).is_err()
        );
    }

    #[test]
    fn invalid_parameters_are_rejected() {
        for values in [
            [0.0, 0.02, 6.4, 0.0015, 45.0, 0.09, 1.7e-4],
            [8.0, -1.0, 6.4, 0.0015, 45.0, 0.09, 1.7e-4],
            [8.0, 0.02, 6.4, -1.0, 45.0, 0.09, 1.7e-4],
            [8.0, 0.02, f64::NAN, 0.0015, 45.0, 0.09, 1.7e-4],
        ] {
            assert!(EquivalentImpedance::from_array(values).is_err());
        }
        assert!(model().impedance(0.0).is_err());
        assert!(model().impedance(f64::NAN).is_err());
    }

    #[test]
    fn impedance_is_finite_across_the_audio_band() {
        let m = model();
        for freq in [1.0, 20.0, 40.0, 100.0, 1000.0, 20_000.0] {
            let z = m.impedance(freq).unwrap();
            assert!(z.re.is_finite() && z.im.is_finite());
        }
    }
}
