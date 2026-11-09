//! Modelo del altavoz (diafragma rígido de pistón).
//!
//! El software de referencia modela el diafragma como un **pistón circular plano rígido**
//! (ayuda, *Driver Diaphragm Model*). El modelo electromecánico es el clásico:
//!
//! ```text
//! Eg = I·(Re + Rg + jωLe) + Bl·v
//! (Rms + jωMmd + 1/(jωCms))·v = Bl·I − Sd·p_carga
//! ```
//!
//! donde `p_carga` es la presión acústica en la cara del diafragma y la carga
//! acústica se refleja al dominio mecánico multiplicando por `Sd²`.

use crate::air::Air;
use crate::omega;
use num_complex::Complex64;

/// Parámetros de un altavoz.
///
/// Todos los valores están en SI. `mmd` es la masa móvil **sin** la masa
/// añadida del aire (igual que el campo `Mmd` del software de referencia); el motor añade la
/// carga de radiación a través de la impedancia acústica.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Driver {
    /// Área efectiva del diafragma (m²).
    pub sd: f64,
    /// Compliancia mecánica (m/N).
    pub cms: f64,
    /// Masa móvil sin carga de aire (kg).
    pub mmd: f64,
    /// Resistencia eléctrica de la bobina (Ω).
    pub re: f64,
    /// Factor de fuerza (T·m).
    pub bl: f64,
    /// Resistencia mecánica de suspensión (kg/s).
    pub rms: f64,
    /// Inductancia de la bobina (H).
    pub le: f64,
    /// Número de unidades (informativo; el sistema usa los valores como un
    /// altavoz compuesto, igual que el software de referencia).
    pub nd: u32,
}

impl Driver {
    /// Construye un altavoz con valores SI.
    #[allow(clippy::too_many_arguments)]
    pub fn new(sd: f64, cms: f64, mmd: f64, re: f64, bl: f64, rms: f64, le: f64) -> Self {
        Self {
            sd,
            cms,
            mmd,
            re,
            bl,
            rms,
            le,
            nd: 1,
        }
    }

    /// Construye un altavoz a partir de los valores que muestra el software de referencia:
    /// `sd` en cm², `mmd` en gramos, `le` en mH.
    #[allow(clippy::too_many_arguments)]
    pub fn from_ui_units(
        sd_cm2: f64,
        cms: f64,
        mmd_g: f64,
        re: f64,
        bl: f64,
        rms: f64,
        le_mh: f64,
        nd: u32,
    ) -> Self {
        Self {
            sd: sd_cm2 * 1.0e-4,
            cms,
            mmd: mmd_g * 1.0e-3,
            re,
            bl,
            rms,
            le: le_mh * 1.0e-3,
            nd: nd.max(1),
        }
    }

    /// Radio equivalente del pistón (m).
    #[inline]
    pub fn radius(&self) -> f64 {
        (self.sd / std::f64::consts::PI).sqrt()
    }

    /// Masa añadida del aire a baja frecuencia para un pistón en pantalla
    /// infinita: `(8/3)·rho·a³`.
    pub fn air_load_mass(&self, air: &Air) -> f64 {
        let a = self.radius();
        (8.0 / 3.0) * air.rho * a * a * a
    }

    /// Masa móvil total en aire `Mms = Mmd + M_aire`.
    pub fn mms(&self, air: &Air) -> f64 {
        self.mmd + self.air_load_mass(air)
    }

    /// Frecuencia de resonancia en aire `Fs = 1/(2π·sqrt(Mms·Cms))`.
    pub fn fs(&self, air: &Air) -> f64 {
        1.0 / (std::f64::consts::TAU * (self.mms(air) * self.cms).sqrt())
    }

    /// Factor de calidad mecánico `Qms = 2π·Fs·Mms/Rms`.
    pub fn qms(&self, air: &Air) -> f64 {
        std::f64::consts::TAU * self.fs(air) * self.mms(air) / self.rms
    }

    /// Factor de calidad eléctrico `Qes = 2π·Fs·Mms·Re/Bl²`.
    pub fn qes(&self, air: &Air) -> f64 {
        std::f64::consts::TAU * self.fs(air) * self.mms(air) * self.re / (self.bl * self.bl)
    }

    /// Factor de calidad total `Qts`.
    pub fn qts(&self, air: &Air) -> f64 {
        let qms = self.qms(air);
        let qes = self.qes(air);
        qms * qes / (qms + qes)
    }

    /// Volumen equivalente de suspensión `Vas = rho·c²·Sd²·Cms` (m³).
    pub fn vas(&self, air: &Air) -> f64 {
        air.bulk_modulus() * self.sd * self.sd * self.cms
    }

    /// Rendimiento de referencia `eta0 = (4π²/c³)·(Fs³·Vas/Qes)`.
    pub fn eta0(&self, air: &Air) -> f64 {
        let fs = self.fs(air);
        let vas = self.vas(air);
        let qes = self.qes(air);
        (4.0 * std::f64::consts::PI * std::f64::consts::PI / air.c.powi(3))
            * (fs.powi(3) * vas / qes)
    }

    /// Sensibilidad referida a 1 W a 1 m: `112.2 + 10·log10(eta0)`.
    pub fn spl_1w(&self, air: &Air) -> f64 {
        112.2 + 10.0 * self.eta0(air).log10()
    }

    /// Sensibilidad referida a 2.83 V a 1 m.
    pub fn spl_28v(&self, air: &Air) -> f64 {
        self.spl_1w(air) + 10.0 * (8.0 / self.re).log10()
    }

    /// Desplazamiento máximo de pico `Xmech = 1.5·Xmax` (convención habitual
    /// en software de simulación).
    pub fn xmech(&self, xmax: f64) -> f64 {
        1.5 * xmax
    }

    /// Volumen desplazado `Vd = Sd·Xmax` (m³).
    pub fn vd(&self, xmax: f64) -> f64 {
        self.sd * xmax
    }

    /// Impedancia mecánica del diafragma `Zm = Rms + jωMmd + 1/(jωCms)`.
    pub fn mechanical_impedance(&self, freq_hz: f64) -> Complex64 {
        let w = omega(freq_hz);
        Complex64::new(self.rms, w * self.mmd) + Complex64::new(0.0, -1.0) / (w * self.cms)
    }

    /// Impedancia eléctrica de la bobina `Ze = Re + jωLe`.
    pub fn coil_impedance(&self, freq_hz: f64) -> Complex64 {
        Complex64::new(self.re, omega(freq_hz) * self.le)
    }

    /// Impedancia de entrada (eléctrica) para una carga acústica dada.
    ///
    /// `Zin = Ze + Bl²/(Zm + Sd²·Zac)`
    pub fn input_impedance(&self, freq_hz: f64, z_acoustic: Complex64) -> Complex64 {
        let zm = self.mechanical_impedance(freq_hz);
        let reflected = self.sd * self.sd * z_acoustic;
        self.coil_impedance(freq_hz) + Complex64::new(self.bl * self.bl, 0.0) / (zm + reflected)
    }

    /// Velocidad volumétrica del diafragma para una tensión de entrada y una
    /// carga acústica dadas (fasores RMS).
    ///
    /// `U = Sd·Bl·Eg / (Ze·(Zm + Sd²·Zac) + Bl²)`
    pub fn volume_velocity(
        &self,
        freq_hz: f64,
        eg: f64,
        source_resistance: f64,
        z_acoustic: Complex64,
    ) -> Complex64 {
        let ze = self.coil_impedance(freq_hz) + Complex64::new(source_resistance, 0.0);
        let zm = self.mechanical_impedance(freq_hz);
        let reflected = self.sd * self.sd * z_acoustic;
        let den = ze * (zm + reflected) + Complex64::new(self.bl * self.bl, 0.0);
        Complex64::new(self.sd * self.bl * eg, 0.0) / den
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_driver() -> Driver {
        Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1)
    }

    #[test]
    fn mechanical_impedance_is_capacitive_below_resonance() {
        let d = default_driver();
        let z = d.mechanical_impedance(10.0);
        // Por debajo de Fs domina la suspensión: parte imaginaria negativa.
        assert!(z.im < 0.0, "Zm = {z:?}");
    }

    #[test]
    fn electrical_impedance_at_mechanical_resonance() {
        let d = default_driver();
        // Resonancia del conjunto Mmd-Cms (sin masa de aire añadida).
        let f0 = 1.0 / (std::f64::consts::TAU * (d.mmd * d.cms).sqrt());
        let ze = d.input_impedance(f0, Complex64::new(0.0, 0.0));
        let expected = d.re + d.bl * d.bl / d.rms;
        let x_le = std::f64::consts::TAU * f0 * d.le;
        assert!((ze.re - expected).abs() / expected < 1.0e-9, "Ze={ze:?}");
        assert!((ze.im - x_le).abs() / x_le < 1.0e-9, "Ze={ze:?}");
    }

    #[test]
    fn ts_parameters_are_physical() {
        let d = default_driver();
        let air = Air::default();
        let fs = d.fs(&air);
        assert!(fs > 30.0 && fs < 90.0, "Fs = {fs}");
        assert!(d.qts(&air) > 0.1 && d.qts(&air) < 2.0);
        assert!(d.vas(&air) > 0.0);
        // La masa añadida debe ser positiva y pequeña frente a Mmd.
        assert!(d.air_load_mass(&air) > 0.0);
        assert!(d.air_load_mass(&air) < d.mmd);
    }
}
