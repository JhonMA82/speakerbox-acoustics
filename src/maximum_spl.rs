//! Escalado lineal de presión, excursión y potencia a partir de un punto resuelto.
use crate::energy::RadiationQuality;
use crate::network::DrivenNetworkState;
use crate::{Design, SolveError};
use num_complex::Complex64 as C;

/// Respuesta lineal referida a 1 V RMS en circuito abierto del generador.
#[derive(Clone, Copy, Debug)]
pub struct UnitDriveResponse {
    pub frequency_hz: f64,
    /// Presión compleja física, con una referencia de 1 V real positivo.
    pub pressure_pa_per_v: C,
    pub reference_pressure_pa: f64,
    pub excursion_peak_m_per_v: f64,
    pub terminal_w_per_v2: f64,
    pub coil_heat_w_per_v2: f64,
    pub radiation_quality: RadiationQuality,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearDrivePoint {
    pub pressure_pa: C,
    pub spl_db: f64,
    pub excursion_peak_m: f64,
    pub terminal_active_w: f64,
    pub coil_heat_w: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerBasis {
    TerminalActive,
    CoilHeat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputConstraint {
    Excursion,
    Power,
    AmplifierVoltage,
}

/// None designa un límite sin caracterizar. Un cero suministrado es un límite.
#[derive(Clone, Copy, Debug, Default)]
pub struct MaximumSplLimits {
    pub excursion_peak_m: Option<f64>,
    pub power_limit: Option<(PowerBasis, f64)>,
    pub amplifier_open_circuit_v_rms: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaximumSplPoint {
    pub frequency_hz: f64,
    pub dominant: OutputConstraint,
    pub operating_voltage_rms: f64,
    pub excursion_voltage_rms: Option<f64>,
    pub power_voltage_rms: Option<f64>,
    pub amplifier_voltage_rms: Option<f64>,
    pub uncharacterized: Vec<OutputConstraint>,
    pub radiation_quality: RadiationQuality,
    pub output: LinearDrivePoint,
}

impl UnitDriveResponse {
    /// Ejecuta los solvers existentes a 1 V RMS y en el dominio que admiten.
    pub fn from_design(design: &Design, f: f64) -> Result<Self, SolveError> {
        let mut unit = design.clone();
        match &mut unit {
            Design::System(m) => m.eg = 1.0,
            Design::TappedHorn(m) => m.eg = 1.0,
            Design::OffsetHorn(m) => m.eg = 1.0,
            Design::Bandpass(m) => m.eg = 1.0,
            Design::TransmissionLine(m) => m.eg = 1.0,
        }
        let pressure = unit
            .try_observation_at(f)?
            .pressure_estimate_pa
            .ok_or(SolveError::InvalidResponse("radiated SPL is unavailable"))?;
        let response = unit.try_response_at(f)?;
        let energy = unit.try_energy_at(f)?;
        Ok(Self {
            frequency_hz: f,
            pressure_pa_per_v: pressure,
            reference_pressure_pa: match &unit {
                Design::System(m) => m.air.p_ref,
                Design::TappedHorn(m) => m.air.p_ref,
                Design::OffsetHorn(m) => m.air.p_ref,
                Design::Bandpass(m) => m.air.p_ref,
                Design::TransmissionLine(m) => m.air.p_ref,
            },
            excursion_peak_m_per_v: response.xd_mm * 1e-3,
            terminal_w_per_v2: energy.driver_terminal_active_w,
            coil_heat_w_per_v2: energy.coil_loss_w,
            radiation_quality: energy.radiated_power.quality,
        })
    }

    /// Normaliza un resultado de red realmente excitado por una tensión conocida.
    pub fn from_network(solution: &DrivenNetworkState) -> Result<Self, SolveError> {
        let v = solution.source_voltage_rms;
        crate::error::require_positive(v.norm(), "network source voltage")?;
        let p = solution
            .pressure_estimate_pa
            .ok_or(SolveError::InvalidResponse("radiated SPL is unavailable"))?;
        let value = Self {
            frequency_hz: solution.frequency_hz,
            pressure_pa_per_v: p / v,
            reference_pressure_pa: solution.reference_pressure_pa,
            excursion_peak_m_per_v: solution.peak_displacement_m / v.norm(),
            terminal_w_per_v2: solution.energy.driver_terminal_active_w / v.norm_sqr(),
            coil_heat_w_per_v2: solution.energy.coil_loss_w / v.norm_sqr(),
            radiation_quality: solution.energy.radiated_power.quality,
        };
        value.validate()?;
        Ok(value)
    }

    fn validate(&self) -> Result<(), SolveError> {
        crate::error::check_frequency(self.frequency_hz)?;
        crate::error::require_non_negative(self.pressure_pa_per_v.norm(), "pressure / V")?;
        crate::error::require_positive(self.reference_pressure_pa, "pressure reference")?;
        crate::error::require_non_negative(self.excursion_peak_m_per_v, "excursion / V")?;
        crate::error::require_non_negative(self.terminal_w_per_v2, "terminal power / V²")?;
        crate::error::require_non_negative(self.coil_heat_w_per_v2, "coil heat / V²")
    }

    /// Una transferencia compleja puede multiplicar la tensión de excitación.
    /// Las potencias dependen del módulo al cuadrado, la presión de la fase.
    pub fn at_voltage(&self, voltage_rms: C) -> Result<LinearDrivePoint, SolveError> {
        self.validate()?;
        let v = voltage_rms.norm();
        crate::error::require_non_negative(v, "amplifier voltage")?;
        let pressure_pa = self.pressure_pa_per_v * voltage_rms;
        let point = LinearDrivePoint {
            pressure_pa,
            spl_db: 20.0 * (pressure_pa.norm() / self.reference_pressure_pa).log10(),
            excursion_peak_m: self.excursion_peak_m_per_v * v,
            terminal_active_w: (self.terminal_w_per_v2.sqrt() * v).powi(2),
            coil_heat_w: (self.coil_heat_w_per_v2.sqrt() * v).powi(2),
        };
        if point.spl_db.is_nan()
            || point.spl_db == f64::INFINITY
            || [
                point.excursion_peak_m,
                point.terminal_active_w,
                point.coil_heat_w,
            ]
            .iter()
            .any(|v| !v.is_finite())
        {
            return Err(SolveError::NonFiniteResponse);
        }
        Ok(point)
    }

    /// Máximo lineal estacionario. Un límite empírico de la zona lineal
    /// puede restringir la excursión, potencia o tensión suministradas.
    pub fn maximum_spl(&self, limits: MaximumSplLimits) -> Result<MaximumSplPoint, SolveError> {
        self.validate()?;
        let voltage_limit = |limit: f64, slope: f64| -> Result<f64, SolveError> {
            crate::error::require_non_negative(limit, "maximum SPL limit")?;
            Ok(if slope == 0.0 {
                f64::INFINITY
            } else {
                limit / slope
            })
        };
        let x = limits
            .excursion_peak_m
            .map(|x| voltage_limit(x, self.excursion_peak_m_per_v))
            .transpose()?;
        let p = limits
            .power_limit
            .map(|(basis, p)| {
                crate::error::require_non_negative(p, "power limit")?;
                let slope = match basis {
                    PowerBasis::TerminalActive => self.terminal_w_per_v2,
                    PowerBasis::CoilHeat => self.coil_heat_w_per_v2,
                };
                voltage_limit(p.sqrt(), slope.sqrt())
            })
            .transpose()?;
        let v = limits
            .amplifier_open_circuit_v_rms
            .map(|v| voltage_limit(v, 1.0))
            .transpose()?;
        let bounds = [
            (OutputConstraint::Excursion, x),
            (OutputConstraint::Power, p),
            (OutputConstraint::AmplifierVoltage, v),
        ];
        let (dominant, operating_voltage_rms) = bounds
            .into_iter()
            .filter_map(|(c, v)| v.filter(|v| v.is_finite()).map(|v| (c, v)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .ok_or(SolveError::InvalidParameter(
                "maximum SPL needs at least one finite binding limit",
            ))?;
        let uncharacterized = bounds
            .into_iter()
            .filter_map(|(c, v)| v.is_none().then_some(c))
            .collect();
        Ok(MaximumSplPoint {
            frequency_hz: self.frequency_hz,
            dominant,
            operating_voltage_rms,
            excursion_voltage_rms: x,
            power_voltage_rms: p,
            amplifier_voltage_rms: v,
            uncharacterized,
            radiation_quality: self.radiation_quality,
            output: self.at_voltage(operating_voltage_rms.into())?,
        })
    }
}
