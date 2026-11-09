//! Elementos acústicos concentrados: recintos, puertos y radiadores pasivos.
//!
//! El software de referencia documenta estas pérdidas en *System Losses*: las pérdidas son cero
//! salvo recinto con recubrimiento absorbente, material de relleno o recinto
//! ventilado; las fugas de una caja ventilada se modelan con el factor de
//! calidad `QL`.

use crate::air::Air;
use crate::omega;
use num_complex::Complex64;

/// Recubrimiento absorbente de un recinto.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Lining {
    /// Modelo simple: resistencia real `R = rho*c/(alpha*S)`.
    Absorption { absorption: f64, surface_area: f64 },
    /// Material poroso (Delany-Bazley): resistividad al flujo `Fr` en
    /// mks rayls/m y espesor en metros.
    Porous {
        flow_resistivity: f64,
        thickness: f64,
        surface_area: f64,
    },
}

impl Lining {
    pub fn new(absorption: f64, surface_area: f64) -> Self {
        Lining::Absorption {
            absorption: absorption.clamp(0.0, 1.0),
            surface_area: surface_area.max(0.0),
        }
    }

    pub fn porous(flow_resistivity: f64, thickness_m: f64, surface_area: f64) -> Self {
        Lining::Porous {
            flow_resistivity: flow_resistivity.max(1.0e-6),
            thickness: thickness_m.max(1.0e-6),
            surface_area: surface_area.max(1.0e-6),
        }
    }

    /// Impedancia acústica del revestimiento (Pa·s/m³).
    pub fn impedance(&self, air: &Air, freq_hz: f64) -> Complex64 {
        match *self {
            Lining::Absorption {
                absorption,
                surface_area,
            } => {
                if absorption <= 1.0e-9 || surface_area <= 0.0 {
                    Complex64::new(f64::INFINITY, 0.0)
                } else {
                    Complex64::new(air.rho * air.c / (absorption * surface_area), 0.0)
                }
            }
            Lining::Porous {
                flow_resistivity,
                thickness,
                surface_area,
            } => {
                // Delany-Bazley (1970).
                let x = air.rho * freq_hz / flow_resistivity;
                if x <= 1.0e-6 || freq_hz <= 0.0 {
                    return Complex64::new(f64::INFINITY, 0.0);
                }
                let zc = air.z0()
                    * Complex64::new(1.0 + 0.0571 * x.powf(-0.754), -0.087 * x.powf(-0.732));
                let k = (omega(freq_hz) / air.c)
                    * Complex64::new(1.0 + 0.0978 * x.powf(-0.700), -0.189 * x.powf(-0.595));
                let kd = k * thickness;
                // Capa porosa respaldada por pared rígida: Zs = -j Zc cot(kd).
                let zs = Complex64::new(0.0, -1.0) * zc * kd.cos() / kd.sin();
                zs / surface_area
            }
        }
    }

    /// Resistencia equivalente (solo para el modelo de absorción).
    pub fn resistance(&self, air: &Air) -> f64 {
        match *self {
            Lining::Absorption {
                absorption,
                surface_area,
            } => {
                if absorption <= 1.0e-9 || surface_area <= 0.0 {
                    f64::INFINITY
                } else {
                    air.rho * air.c / (absorption * surface_area)
                }
            }
            Lining::Porous { .. } => f64::NAN,
        }
    }
}

/// Recinto acústico (cámara de garganta, recinto trasero, cámara frontal…).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chamber {
    /// Volumen en m³.
    pub volume: f64,
    /// Recubrimiento absorbente opcional.
    pub lining: Option<Lining>,
}

impl Chamber {
    /// Recinto sin pérdidas.
    pub fn new(volume: f64) -> Self {
        Self {
            volume: volume.max(0.0),
            lining: None,
        }
    }

    /// Recinto con recubrimiento absorbente.
    pub fn lined(volume: f64, lining: Lining) -> Self {
        Self {
            volume: volume.max(0.0),
            lining: Some(lining),
        }
    }

    /// Compliancia acústica `C = V/(rho·c²)` (m³/Pa).
    pub fn compliance(&self, air: &Air) -> f64 {
        self.volume / air.bulk_modulus()
    }

    /// Impedancia acústica `Z = 1/(jωC)`, en paralelo con la resistencia de
    /// pérdidas si hay recubrimiento.
    pub fn impedance(&self, air: &Air, freq_hz: f64) -> Complex64 {
        if self.volume <= 0.0 {
            // Volumen nulo: cortocircuito acústico (presión nula).
            return Complex64::new(0.0, 0.0);
        }
        let c = self.compliance(air);
        let zc = Complex64::new(0.0, -1.0) / (omega(freq_hz) * c);
        match self.lining {
            None => zc,
            Some(l) => {
                let zl = l.impedance(air, freq_hz);
                if !zl.re.is_finite() {
                    zc
                } else {
                    zc * zl / (zc + zl)
                }
            }
        }
    }
}

/// Corrección de extremo de un tubo/puerto.
///
/// El radio equivalente es `a = sqrt(S/π)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EndCorrection {
    /// Sin corrección.
    None,
    /// Corrección interior con coeficiente `k` (el software de referencia usa ~0.85):
    /// `ΔL = k·a`.
    Inner(f64),
    /// Ambos extremos con brida (pantalla): `ΔL = 2·0.8216·a`.
    FlangedBoth,
    /// Ambos extremos sin brida: `ΔL = 2·0.6133·a`.
    UnflangedBoth,
}

impl EndCorrection {
    /// Longitud añadida en metros para un área `area`.
    pub fn delta(self, area: f64) -> f64 {
        let a = (area / std::f64::consts::PI).sqrt();
        match self {
            EndCorrection::None => 0.0,
            EndCorrection::Inner(k) => k * a,
            EndCorrection::FlangedBoth => 2.0 * 0.8216 * a,
            EndCorrection::UnflangedBoth => 2.0 * 0.6133 * a,
        }
    }
}

/// Puerto (tubo) acústico.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Port {
    /// Área interior (m²).
    pub area: f64,
    /// Longitud física (m).
    pub length: f64,
    /// Corrección de extremo aplicada.
    pub end_correction: EndCorrection,
}

impl Port {
    pub fn new(area: f64, length: f64, end_correction: EndCorrection) -> Self {
        Self {
            area: area.max(1.0e-12),
            length: length.max(0.0),
            end_correction,
        }
    }

    /// Longitud efectiva `L + ΔL` (m).
    pub fn effective_length(&self) -> f64 {
        self.length + self.end_correction.delta(self.area)
    }

    /// Masa acústica `M = rho·L_eff/S` (kg/m⁴).
    pub fn acoustic_mass(&self, air: &Air) -> f64 {
        air.rho * self.effective_length() / self.area
    }

    /// Impedancia acústica del puerto `Z = jωM`.
    pub fn impedance(&self, air: &Air, freq_hz: f64) -> Complex64 {
        Complex64::new(0.0, omega(freq_hz) * self.acoustic_mass(air))
    }
}

/// Radiador pasivo (membrana sin motor).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PassiveRadiator {
    /// Área efectiva (m²).
    pub sd: f64,
    /// Compliancia mecánica (m/N).
    pub cms: f64,
    /// Masa móvil (kg).
    pub mmd: f64,
    /// Resistencia mecánica (kg/s).
    pub rms: f64,
}

impl PassiveRadiator {
    pub fn new(sd: f64, cms: f64, mmd: f64, rms: f64) -> Self {
        Self {
            sd: sd.max(1.0e-12),
            cms: cms.max(1.0e-12),
            mmd: mmd.max(0.0),
            rms: rms.max(0.0),
        }
    }

    /// Masa acústica `M = Mmd/Sd²`.
    pub fn acoustic_mass(&self) -> f64 {
        self.mmd / (self.sd * self.sd)
    }

    /// Compliancia acústica `C = Cms·Sd²`.
    pub fn acoustic_compliance(&self) -> f64 {
        self.cms * self.sd * self.sd
    }

    /// Impedancia acústica `Z = R + jωM + 1/(jωC)`.
    pub fn impedance(&self, freq_hz: f64) -> Complex64 {
        let w = omega(freq_hz);
        Complex64::new(self.rms / (self.sd * self.sd), w * self.acoustic_mass())
            + Complex64::new(0.0, -1.0) / (w * self.acoustic_compliance())
    }
}

/// Combinación en paralelo de dos impedancias.
pub fn parallel(z1: Complex64, z2: Complex64) -> Complex64 {
    let den = z1 + z2;
    if den.norm() < 1.0e-30 {
        Complex64::new(0.0, 0.0)
    } else {
        z1 * z2 / den
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chamber_compliance_is_physical() {
        let air = Air::default();
        let c = Chamber::new(10.0e-3);
        let expected = 10.0e-3 / (air.rho * air.c * air.c);
        assert!((c.compliance(&air) - expected).abs() < 1.0e-15);
    }

    #[test]
    fn chamber_impedance_is_capacitive() {
        let air = Air::default();
        let z = Chamber::new(10.0e-3).impedance(&air, 100.0);
        // 1/(jωC) debe tener parte imaginaria negativa.
        assert!(z.im < 0.0, "Z = {z:?}");
    }

    #[test]
    fn port_mass_scales_with_length() {
        let air = Air::default();
        let p = Port::new(100.0e-4, 0.1, EndCorrection::None);
        assert!((p.acoustic_mass(&air) - air.rho * 0.1 / 100.0e-4).abs() < 1.0e-12);
    }

    #[test]
    fn end_correction_positive() {
        assert!(EndCorrection::Inner(0.85).delta(100.0e-4) > 0.0);
        assert!(
            EndCorrection::FlangedBoth.delta(100.0e-4) > EndCorrection::Inner(0.85).delta(100.0e-4)
        );
    }
}
