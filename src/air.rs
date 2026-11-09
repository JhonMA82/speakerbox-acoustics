//! Constantes y propiedades del aire.
//!
//! El software de referencia usa por defecto `c = 344 m/s` y `rho = 1.205 kg/m³`
//! (documentado en su ayuda, sección *Air Constants*). El programa permite
//! cambiarlas durante la sesión con `Ctrl+F5`.

/// Propiedades del medio (aire).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Air {
    /// Velocidad del sonido en m/s.
    pub c: f64,
    /// Densidad en kg/m³.
    pub rho: f64,
    /// Presión de referencia para el nivel de presión sonora (20 µPa).
    pub p_ref: f64,
}

impl Default for Air {
    /// Valores por defecto del software de referencia.
    fn default() -> Self {
        Self {
            c: 344.0,
            rho: 1.205,
            p_ref: 20.0e-6,
        }
    }
}

impl Air {
    /// Crea un medio con velocidad del sonido y densidad dadas.
    pub fn new(c: f64, rho: f64) -> Self {
        Self {
            c,
            rho,
            p_ref: 20.0e-6,
        }
    }

    /// Impedancia característica del aire `rho * c` (rayls).
    #[inline]
    pub fn z0(&self) -> f64 {
        self.rho * self.c
    }

    /// Número de onda `k = 2*pi*f/c`.
    #[inline]
    pub fn wave_number(&self, freq_hz: f64) -> f64 {
        std::f64::consts::TAU * freq_hz / self.c
    }

    /// Módulo de compresibilidad `rho * c²` (Pa).
    #[inline]
    pub fn bulk_modulus(&self) -> f64 {
        self.rho * self.c * self.c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_reference() {
        let air = Air::default();
        assert_eq!(air.c, 344.0);
        assert_eq!(air.rho, 1.205);
        assert!((air.z0() - 414.52).abs() < 0.01);
    }
}
