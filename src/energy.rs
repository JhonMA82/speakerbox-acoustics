//! Balance de potencias activas con fasores RMS y exp(jωt).
//! No se infiere potencia integral a partir de presión en un punto.
use crate::{driver::Driver, error::SolveError};
use num_complex::Complex64;

/// Potencia activa, W; p en Pa y U saliente de la red hacia la carga en m³/s.
pub fn active_power(pressure_pa: Complex64, volume_velocity: Complex64) -> f64 {
    (pressure_pa * volume_velocity.conj()).re
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminationKind {
    BaffledPiston,
    CalibratedPiston,
    MatchedAbsorber,
}
/// Estado físico de una terminación realmente incluida en la red.
#[derive(Clone, Copy, Debug)]
pub struct AcousticTermination {
    pub kind: TerminationKind,
    pub pressure_pa: Complex64,
    pub volume_velocity_m3_s: Complex64,
    pub impedance_pa_s_m3: Complex64,
}
impl AcousticTermination {
    pub fn absorbed_power_w(&self) -> f64 {
        active_power(self.pressure_pa, self.volume_velocity_m3_s)
    }
}

/// Alcance de la interpretación radiativa, separado de la absorción de cargas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadiationQuality {
    /// Potencia de la única carga de pistón en pantalla infinita de este modelo.
    SingleBaffledPiston,
    /// Suma de cargas independientes: no incluye impedancias mutuas espaciales.
    IndependentLoadsApproximation,
    /// Carga con factores empíricos: absorción exacta de red, radiación aproximada.
    CalibratedLoadApproximation,
    /// Alguna salida de SPL carece de carga radiativa en el circuito.
    IncompleteRadiationModel,
    /// Un absorbente adaptado no especifica un campo radiado exterior.
    MatchedAbsorber,
}
/// `None` es no disponible; no equivale a cero físicamente radiado.
#[derive(Clone, Copy, Debug)]
pub struct RadiatedPower {
    pub watts: Option<f64>,
    pub quality: RadiationQuality,
}

/// Todas son pérdidas disipativas identificadas, NO un residual de ajuste.
#[derive(Clone, Copy, Debug, Default)]
pub struct AcousticLosses {
    pub wall_w: f64,
    pub lining_w: f64,
    pub throat_series_w: f64,
    pub leakage_w: f64,
    pub chamber_ql_w: f64,
    pub passive_suspension_w: f64,
}
impl AcousticLosses {
    pub fn total_w(&self) -> f64 {
        self.wall_w
            + self.lining_w
            + self.throat_series_w
            + self.leakage_w
            + self.chamber_ql_w
            + self.passive_suspension_w
    }
}

#[derive(Clone, Debug)]
pub struct EnergyReport {
    pub input_current_a: Complex64,
    pub driver_terminal_voltage_v: Complex64,
    pub source_active_w: f64,
    pub source_resistor_loss_w: f64,
    pub driver_terminal_active_w: f64,
    /// Total DC and frequency-dependent coil heat.
    pub coil_loss_w: f64,
    pub coil_dc_loss_w: f64,
    pub coil_excess_loss_w: f64,
    pub suspension_loss_w: f64,
    pub acoustic_input_w: f64,
    pub acoustic_losses: AcousticLosses,
    pub terminations: Vec<AcousticTermination>,
    pub radiated_power: RadiatedPower,
}
impl EnergyReport {
    pub(crate) fn new(
        driver: &Driver,
        frequency_hz: f64,
        eg: f64,
        rg: f64,
        current: Complex64,
        ud: Complex64,
        acoustic_input_w: f64,
    ) -> Self {
        let voltage = Complex64::new(eg, 0.0) - rg * current;
        Self {
            input_current_a: current,
            driver_terminal_voltage_v: voltage,
            source_active_w: eg * current.re,
            source_resistor_loss_w: rg * current.norm_sqr(),
            driver_terminal_active_w: active_power(voltage, current),
            coil_loss_w: driver.coil_impedance(frequency_hz).re * current.norm_sqr(),
            coil_dc_loss_w: driver.re * current.norm_sqr(),
            coil_excess_loss_w: (driver.coil_impedance(frequency_hz).re - driver.re) * current.norm_sqr(),
            suspension_loss_w: driver.rms * (ud / driver.sd).norm_sqr(),
            acoustic_input_w,
            acoustic_losses: AcousticLosses::default(),
            terminations: Vec::new(),
            radiated_power: RadiatedPower {
                watts: None,
                quality: RadiationQuality::IncompleteRadiationModel,
            },
        }
    }
    pub fn termination_absorbed_w(&self) -> f64 {
        self.terminations
            .iter()
            .map(AcousticTermination::absorbed_power_w)
            .sum()
    }
    /// Residual diagnóstico; jamás se añade a las pérdidas para cerrar el balance.
    pub fn balance_error_w(&self) -> f64 {
        self.source_active_w
            - self.source_resistor_loss_w
            - self.coil_loss_w
            - self.suspension_loss_w
            - self.acoustic_losses.total_w()
            - self.termination_absorbed_w()
    }
    pub fn acoustic_balance_error_w(&self) -> f64 {
        self.acoustic_input_w - self.acoustic_losses.total_w() - self.termination_absorbed_w()
    }
    /// η con potencia radiada (posiblemente aproximada según quality) / fuente.
    /// Indefinido para excitación nula o radiación no disponible.
    pub fn efficiency_vs_source_pct(&self) -> Option<f64> {
        self.radiated_power
            .watts
            .filter(|_| self.source_active_w > 0.0)
            .map(|p| 100.0 * p / self.source_active_w)
    }
    /// η con el mismo numerador / potencia en bornes, excluyendo Rg.
    pub fn efficiency_vs_driver_terminals_pct(&self) -> Option<f64> {
        self.radiated_power
            .watts
            .filter(|_| self.driver_terminal_active_w > 0.0)
            .map(|p| 100.0 * p / self.driver_terminal_active_w)
    }
    pub(crate) fn finish(mut self, quality: RadiationQuality) -> Result<Self, SolveError> {
        self.radiated_power = RadiatedPower {
            quality,
            watts: match quality {
                RadiationQuality::MatchedAbsorber | RadiationQuality::IncompleteRadiationModel => {
                    None
                }
                _ => Some(self.termination_absorbed_w()),
            },
        };
        // No clamping: potencia negativa de una carga pasiva se hace visible.
        for p in [
            self.source_active_w,
            self.source_resistor_loss_w,
            self.driver_terminal_active_w,
            self.coil_loss_w,
            self.suspension_loss_w,
            self.acoustic_input_w,
            self.acoustic_losses.wall_w,
            self.acoustic_losses.lining_w,
            self.acoustic_losses.throat_series_w,
            self.acoustic_losses.leakage_w,
            self.acoustic_losses.chamber_ql_w,
            self.acoustic_losses.passive_suspension_w,
        ] {
            if !p.is_finite() || p < 0.0 {
                return Err(SolveError::InvalidResponse(
                    "energy power is nonfinite or active",
                ));
            }
        }
        for t in &self.terminations {
            for z in [t.pressure_pa, t.volume_velocity_m3_s, t.impedance_pa_s_m3] {
                if !z.re.is_finite() || !z.im.is_finite() {
                    return Err(SolveError::NonFiniteResponse);
                }
            }
            if !t.absorbed_power_w().is_finite() || t.absorbed_power_w() < 0.0 {
                return Err(SolveError::InvalidResponse("termination active power"));
            }
        }
        for p in [
            self.termination_absorbed_w(),
            self.acoustic_losses.total_w(),
            self.balance_error_w(),
            self.acoustic_balance_error_w(),
        ] {
            if !p.is_finite() {
                return Err(SolveError::NonFiniteResponse);
            }
        }
        Ok(self)
    }
}

pub(crate) fn termination(
    kind: TerminationKind,
    p: Complex64,
    u: Complex64,
    z: Complex64,
) -> AcousticTermination {
    AcousticTermination {
        kind,
        pressure_pa: p,
        volume_velocity_m3_s: u,
        impedance_pa_s_m3: z,
    }
}
pub(crate) fn load_termination(
    kind: TerminationKind,
    u: Complex64,
    z: Complex64,
) -> AcousticTermination {
    termination(kind, z * u, u, z)
}
pub(crate) fn shunt_power(p: Complex64, z: Complex64) -> f64 {
    (Complex64::new(1.0, 0.0) / z).re * p.norm_sqr()
}
