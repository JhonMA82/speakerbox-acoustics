//! Aproximación termoviscosa de capa límite delgada para conductos circulares.
use num_complex::Complex64;

use crate::{error::require_positive, omega, Air, SolveError};

/// Propiedades termodinámicas del fluido. Use valores coherentes con `Air`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThermoviscousAir {
    pub viscosity_pa_s: f64,
    pub thermal_conductivity_w_m_k: f64,
    pub heat_capacity_j_kg_k: f64,
    pub heat_capacity_ratio: f64,
}

impl Default for ThermoviscousAir {
    /// Aire nominal cerca de 20 °C, con propiedades constantes.
    fn default() -> Self {
        Self {
            viscosity_pa_s: 1.81e-5,
            thermal_conductivity_w_m_k: 0.0257,
            heat_capacity_j_kg_k: 1006.0,
            heat_capacity_ratio: 1.4,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct DuctCoefficients {
    pub series_per_m: Complex64,
    pub shunt_per_m: Complex64,
}

impl DuctCoefficients {
    pub fn propagation(self) -> Complex64 {
        (self.series_per_m * self.shunt_per_m).sqrt()
    }
}

impl ThermoviscousAir {
    pub(crate) fn validate(self) -> Result<(), SolveError> {
        require_positive(self.viscosity_pa_s, "thermoviscous viscosity")?;
        require_positive(
            self.thermal_conductivity_w_m_k,
            "thermoviscous thermal conductivity",
        )?;
        require_positive(self.heat_capacity_j_kg_k, "thermoviscous heat capacity")?;
        require_positive(
            self.heat_capacity_ratio - 1.0,
            "thermoviscous heat capacity ratio > 1",
        )
    }

    pub(crate) fn coefficients(
        self,
        air: &Air,
        area: f64,
        f: f64,
    ) -> Result<DuctCoefficients, SolveError> {
        let radius = (area / std::f64::consts::PI).sqrt();
        let w = omega(f);
        let viscous = (2.0 * self.viscosity_pa_s / (air.rho * w)).sqrt() / radius;
        let thermal = (2.0 * self.thermal_conductivity_w_m_k
            / (air.rho * self.heat_capacity_j_kg_k * w))
            .sqrt()
            / radius;
        if !(viscous.is_finite() && thermal.is_finite() && viscous.max(thermal) <= 0.1) {
            return Err(SolveError::InvalidParameter(
                "thermoviscous layer depth / radius exceeds 0.1",
            ));
        }
        let ka = air.wave_number(f) * radius;
        if !ka.is_finite() || ka > 1.5 {
            return Err(SolveError::InvalidParameter(
                "thermoviscous circular plane-mode ka exceeds 1.5",
            ));
        }
        let z = Complex64::new(viscous, -viscous);
        let y = Complex64::new(thermal, -thermal) * (self.heat_capacity_ratio - 1.0);
        let series_per_m = Complex64::new(0.0, w * air.rho / area) / (1.0 - z);
        let shunt_per_m = Complex64::new(0.0, w * area / air.bulk_modulus()) * (1.0 + y);
        if [series_per_m, shunt_per_m]
            .iter()
            .all(|v| v.re.is_finite() && v.re >= 0.0 && v.im.is_finite())
        {
            Ok(DuctCoefficients {
                series_per_m,
                shunt_per_m,
            })
        } else {
            Err(SolveError::InvalidResponse(
                "thermoviscous duct coefficients",
            ))
        }
    }
}
