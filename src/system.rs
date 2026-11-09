//! Ensamblado del sistema completo y barrido de respuesta.
//!
//! La red acústica es:
//!
//! ```text
//!                 ┌── cámara de garganta ── bocina ── radiación
//!   diafragma ────┤
//!                 └── recinto trasero ── (puerto / radiador pasivo)
//! ```
//!
//! El diafragma ve `Z_carga = Z_frente + Z_trasera`. Con esa carga se resuelve
//! la velocidad volumétrica y de ahí la presión radiada, la impedancia
//! eléctrica, el desplazamiento y la potencia acústica.
//!
//! Para un radiador directo con recinto, la presión radiada suma las
//! contribuciones complejas del diafragma, del puerto y del radiador pasivo,
//! cada una con su área de radiación.
//!
//! Validación: bocina normal, caja sellada, caja ventilada y radiador pasivo
//! contra exports propios del software de referencia 60.70 (`tests/production_accuracy.rs`,
//! `tests/system_production.rs`).

use num_complex::Complex64;

use crate::air::Air;
use crate::driver::Driver;
use crate::element::{parallel, Chamber, PassiveRadiator, Port};
use crate::error::SolveError;
use crate::omega;
use crate::radiation::piston_radiation_impedance;
use crate::segment::Horn;

/// Un punto del barrido de respuesta.
#[derive(Clone, Copy, Debug)]
pub struct ResponsePoint {
    /// Frecuencia (Hz).
    pub freq: f64,
    /// Nivel de presión sonora de la respuesta de potencia a 1 m (dB SPL).
    pub spl_db: f64,
    /// Fase de la presión radiada (grados).
    pub phase_deg: f64,
    /// Retardo de grupo (ms).
    pub group_delay_ms: f64,
    /// Módulo de la impedancia eléctrica de entrada (Ω).
    pub ze_mag: f64,
    /// Fase de la impedancia eléctrica (grados).
    pub ze_phase_deg: f64,
    /// Desplazamiento de pico del diafragma (mm).
    pub displacement_mm: f64,
    /// Potencia acústica radiada (W).
    pub acoustical_power_w: f64,
    /// Rendimiento acústico (%).
    pub efficiency_pct: f64,
    /// Parte real de la impedancia acústica de entrada (Pa·s/m³).
    pub za_real: f64,
    /// Parte imaginaria de la impedancia acústica de entrada (Pa·s/m³).
    pub za_imag: f64,
    /// Velocidad volumétrica en la boca de la bocina (m³/s), si aplica.
    pub mouth_volume_velocity: f64,
    /// Velocidad del aire en el puerto (m/s), si aplica.
    pub port_velocity: Option<f64>,
}

/// Sistema acústico completo.
#[derive(Clone, Debug)]
pub struct System {
    /// Propiedades del aire.
    pub air: Air,
    /// Altavoz.
    pub driver: Driver,
    /// Bocina (si es `None`, el sistema es un radiador directo).
    pub horn: Option<Horn>,
    /// Cámara entre el diafragma y la garganta de la bocina.
    pub throat_chamber: Option<Chamber>,
    /// Recinto trasero.
    pub rear_chamber: Option<Chamber>,
    /// Puerto del recinto trasero.
    pub port: Option<Port>,
    /// Radiador pasivo del recinto trasero.
    pub passive_radiator: Option<PassiveRadiator>,
    /// Factor QL del recinto trasero ventilado. `None` = sin pérdidas.
    pub rear_ql: Option<f64>,
    /// Ángulo sólido de radiación en estereorradianes.
    pub solid_angle: f64,
    /// Tensión de entrada (V RMS).
    pub eg: f64,
    /// Resistencia de la fuente (Ω).
    pub rg: f64,
    /// Corrección global del nivel de SPL (dB).
    pub spl_offset_db: f64,
    /// Fuga acústica en la garganta de la bocina (Pa·s/m³). `None` = sin fuga.
    pub horn_leak: Option<f64>,
    /// Resistencia serie en la garganta de la bocina (Pa·s/m³).
    pub horn_series: Option<f64>,
}

/// Presión compleja de un radiador de área `area` para una velocidad
/// volumétrica `u`. La magnitud usa la resistencia de radiación del pistón;
/// la fase es la de `u` (el factor común `jωρ` no afecta a la suma).
fn radiator_pressure(
    air: &Air,
    area: f64,
    u: Complex64,
    freq_hz: f64,
    solid_angle: f64,
) -> Complex64 {
    if area <= 0.0 || solid_angle <= 0.0 {
        return Complex64::new(0.0, 0.0);
    }
    let z_rad = piston_radiation_impedance(air, area, freq_hz, solid_angle);
    let scale = (z_rad.re * std::f64::consts::TAU * air.rho * air.c / (solid_angle * solid_angle))
        .max(0.0)
        .sqrt();
    u * scale
}

impl System {
    /// Sistema a partir de un altavoz, con radiación a 2π y Eg = 2.83 V.
    pub fn new(air: Air, driver: Driver) -> Self {
        Self {
            air,
            driver,
            horn: None,
            throat_chamber: None,
            rear_chamber: None,
            port: None,
            passive_radiator: None,
            rear_ql: None,
            solid_angle: std::f64::consts::TAU,
            eg: 2.83,
            rg: 0.0,
            spl_offset_db: 0.0,
            horn_leak: None,
            horn_series: None,
        }
    }

    /// Frecuencia de sintonía del puerto trasero (Hz), si existe.
    pub fn rear_tuning_hz(&self) -> Option<f64> {
        let chamber = self.rear_chamber?;
        let port = self.port?;
        if chamber.volume <= 0.0 || port.area <= 0.0 {
            return None;
        }
        let l_eff = port.effective_length();
        if l_eff <= 0.0 {
            return None;
        }
        Some(self.air.c / std::f64::consts::TAU * (port.area / (l_eff * chamber.volume)).sqrt())
    }

    /// Impedancias de carga del diafragma `(Z_frente, Z_trasera)`.
    pub fn loads(&self, freq_hz: f64) -> (Complex64, Complex64) {
        let front = if let Some(horn) = &self.horn {
            let zh0 = horn.input_impedance(&self.air, freq_hz, self.solid_angle);
            let zh = match self.horn_leak {
                Some(r) if r > 0.0 => crate::element::parallel(zh0, Complex64::new(r, 0.0)),
                _ => zh0,
            };
            match self.throat_chamber {
                // La cámara de garganta es un volumen en derivación: su
                // compliancia está en paralelo con la impedancia de entrada
                // de la bocina, no en serie.
                Some(ch) => crate::element::parallel(ch.impedance(&self.air, freq_hz), zh),
                None => zh,
            }
        } else {
            piston_radiation_impedance(&self.air, self.driver.sd, freq_hz, self.solid_angle)
        };

        let rear = match self.rear_chamber {
            Some(ch) => {
                let mut z = ch.impedance(&self.air, freq_hz);
                if let Some(ql) = self.rear_ql {
                    if ql.is_finite() && ql > 0.0 {
                        if let Some(fb) = self.rear_tuning_hz() {
                            if fb > 0.0 {
                                let c = ch.compliance(&self.air);
                                let r = ql / (std::f64::consts::TAU * fb * c);
                                z = parallel(z, Complex64::new(r, 0.0));
                            }
                        }
                    }
                }
                if let Some(port) = &self.port {
                    z = parallel(z, port.impedance(&self.air, freq_hz));
                }
                if let Some(pr) = &self.passive_radiator {
                    z = parallel(z, pr.impedance(freq_hz));
                }
                z
            }
            None => Complex64::new(0.0, 0.0),
        };

        (front, rear)
    }

    /// Calcula un punto de respuesta a la frecuencia `freq_hz`.
    pub fn response_at(&self, freq_hz: f64) -> ResponsePoint {
        let zero = ResponsePoint {
            freq: freq_hz,
            spl_db: f64::NEG_INFINITY,
            phase_deg: 0.0,
            group_delay_ms: 0.0,
            ze_mag: 0.0,
            ze_phase_deg: 0.0,
            displacement_mm: 0.0,
            acoustical_power_w: 0.0,
            efficiency_pct: 0.0,
            za_real: 0.0,
            za_imag: 0.0,
            mouth_volume_velocity: 0.0,
            port_velocity: None,
        };
        if freq_hz <= 0.0 {
            return zero;
        }

        let (z_front, z_rear) = self.loads(freq_hz);
        let z_load = z_front + z_rear;
        let u_d = self
            .driver
            .volume_velocity(freq_hz, self.eg, self.rg, z_load);
        let v_d = u_d / self.driver.sd;

        // Propagación hasta la boca y potencia radiada.
        let mouth_area = self
            .horn
            .as_ref()
            .map(|h| h.mouth_area())
            .unwrap_or(self.driver.sd);
        let u_mouth = if let Some(horn) = &self.horn {
            let zh_horn = horn.input_impedance(&self.air, freq_hz, self.solid_angle);
            // Presión en el nodo de garganta y corriente que entra realmente
            // en la bocina. Con una cámara de garganta en derivación, parte
            // del volumen del diafragma se va a la compliancia de la cámara:
            // no se puede usar u_d como velocidad de entrada de la bocina.
            let p_throat = z_front * u_d;
            let r_series = self.horn_series.unwrap_or(0.0).max(0.0);
            let u_horn = p_throat / (zh_horn + Complex64::new(r_series, 0.0));
            let p_horn = p_throat - Complex64::new(r_series, 0.0) * u_horn;
            let (_, um) = horn.transmit(&self.air, freq_hz, self.solid_angle, p_horn, u_horn);
            um
        } else {
            u_d
        };
        // Presión radiada. En una bocina se usa la velocidad de boca; en un
        // radiador directo con puerto o radiador pasivo se suman las presiones
        // complejas del diafragma y de cada salida trasera.
        let (p_rms, p_ac, phase_deg) = if self.horn.is_some() {
            let z_rad =
                piston_radiation_impedance(&self.air, mouth_area, freq_hz, self.solid_angle);
            // La impedancia de radiación es la de pantalla infinita (2π). Para
            // radiar en un ángulo sólido Ω, la potencia se escala por 2π/Ω.
            let p_ac = u_mouth.norm_sqr() * z_rad.re * (std::f64::consts::TAU / self.solid_angle);
            let p_rms = if p_ac > 0.0 {
                (p_ac * self.air.rho * self.air.c / self.solid_angle).sqrt()
            } else {
                0.0
            };
            (p_rms, p_ac, u_mouth.arg().to_degrees())
        } else {
            let mut p_total =
                radiator_pressure(&self.air, self.driver.sd, u_d, freq_hz, self.solid_angle);
            if self.rear_chamber.is_some() {
                let p_rear = -u_d * z_rear;
                if let Some(port) = &self.port {
                    let u_port = p_rear / port.impedance(&self.air, freq_hz);
                    p_total +=
                        radiator_pressure(&self.air, port.area, u_port, freq_hz, self.solid_angle);
                }
                if let Some(pr) = &self.passive_radiator {
                    let u_pr = p_rear / pr.impedance(freq_hz);
                    p_total += radiator_pressure(&self.air, pr.sd, u_pr, freq_hz, self.solid_angle);
                }
            }
            let p_rms = p_total.norm();
            let p_ac = p_rms * p_rms * self.solid_angle / (self.air.rho * self.air.c);
            (p_rms, p_ac, p_total.arg().to_degrees())
        };
        let spl = if p_rms > 0.0 {
            20.0 * (p_rms / self.air.p_ref).log10() + self.spl_offset_db
        } else {
            f64::NEG_INFINITY
        };

        // Impedancia eléctrica e intensidad.
        let ze_coil = self.driver.coil_impedance(freq_hz) + Complex64::new(self.rg, 0.0);
        let zm = self.driver.mechanical_impedance(freq_hz);
        let z_mot = Complex64::new(self.driver.bl * self.driver.bl, 0.0)
            / (zm + self.driver.sd * self.driver.sd * z_load);
        let ze_in = ze_coil + z_mot;
        let current =
            (Complex64::new(self.eg, 0.0) - Complex64::new(self.driver.bl, 0.0) * v_d) / ze_coil;
        // Potencia eléctrica de entrada real: Re(Eg · I*) con Eg real.
        // |I|²·(Re+Rg) ignora la fem mecánica y subestima la potencia.
        let p_in = self.eg * current.re;

        // Desplazamiento de pico (√2·RMS), como el software de referencia.
        let disp_mm = (v_d.norm() / omega(freq_hz)) * 1000.0 * std::f64::consts::SQRT_2;

        // Velocidad del puerto.
        let port_velocity = match (&self.port, &self.rear_chamber) {
            (Some(port), Some(_)) => {
                let p_rear = -u_d * z_rear;
                let u_port = p_rear / port.impedance(&self.air, freq_hz);
                Some((u_port / port.area).norm())
            }
            _ => None,
        };

        ResponsePoint {
            freq: freq_hz,
            spl_db: spl,
            phase_deg,
            group_delay_ms: 0.0,
            ze_mag: ze_in.norm(),
            ze_phase_deg: ze_in.arg().to_degrees(),
            displacement_mm: disp_mm,
            acoustical_power_w: p_ac,
            efficiency_pct: if p_in > 0.0 { 100.0 * p_ac / p_in } else { 0.0 },
            za_real: z_front.re,
            za_imag: z_front.im,
            mouth_volume_velocity: u_mouth.norm(),
            port_velocity,
        }
    }

    /// Respuesta a una frecuencia, con error explícito si la entrada o la
    /// respuesta no son válidas.
    pub fn try_response_at(&self, freq_hz: f64) -> Result<ResponsePoint, SolveError> {
        if !freq_hz.is_finite() || freq_hz <= 0.0 {
            return Err(SolveError::InvalidFrequency(freq_hz));
        }
        let p = self.response_at(freq_hz);
        if p.ze_mag.is_finite() && p.ze_mag > 0.0 {
            Ok(p)
        } else {
            Err(SolveError::NonFiniteResponse)
        }
    }

    /// Barrido de respuesta. Se eliminan las frecuencias no positivas y se
    /// calcula la fase desenrollada y el retardo de grupo.
    pub fn response(&self, freqs: &[f64]) -> Vec<ResponsePoint> {
        let mut pts: Vec<ResponsePoint> = freqs
            .iter()
            .copied()
            .filter(|f| *f > 0.0)
            .map(|f| self.response_at(f))
            .collect();
        if pts.is_empty() {
            return pts;
        }

        // Fase desenrollada.
        for i in 1..pts.len() {
            while pts[i].phase_deg - pts[i - 1].phase_deg > 180.0 {
                pts[i].phase_deg -= 360.0;
            }
            while pts[i].phase_deg - pts[i - 1].phase_deg < -180.0 {
                pts[i].phase_deg += 360.0;
            }
        }

        // Retardo de grupo = -d(phi)/d(omega).
        for i in 1..pts.len() {
            let df = pts[i].freq - pts[i - 1].freq;
            if df.abs() > 1.0e-9 {
                let dphi = (pts[i].phase_deg - pts[i - 1].phase_deg).to_radians();
                pts[i].group_delay_ms = -(dphi / (std::f64::consts::TAU * df)) * 1000.0;
            }
        }
        if pts.len() > 1 {
            pts[0].group_delay_ms = pts[1].group_delay_ms;
        }

        pts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::Driver;
    use crate::segment::{Flare, Horn, Segment};

    fn default_system() -> System {
        let driver = Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1);
        let mut sys = System::new(Air::default(), driver);
        sys.solid_angle = 2.0 * std::f64::consts::PI;
        sys
    }

    #[test]
    fn direct_radiator_has_finite_spl() {
        let sys = default_system();
        let p = sys.response_at(100.0);
        assert!(p.spl_db.is_finite());
        assert!(p.ze_mag > 0.0);
    }

    #[test]
    fn horn_system_runs_and_is_finite() {
        let mut sys = default_system();
        sys.horn = Some(Horn::new(vec![
            Segment::new(Flare::Conical, 0.60, 80.0e-4, 350.0e-4),
            Segment::new(Flare::Exponential, 0.75, 350.0e-4, 2400.0e-4),
        ]));
        sys.throat_chamber = Some(Chamber::new(4.0e-3));
        sys.rear_chamber = Some(Chamber::new(14.0e-3));
        for f in [20.0, 50.0, 100.0, 500.0, 2000.0, 10000.0] {
            let p = sys.response_at(f);
            assert!(p.ze_mag.is_finite() && p.ze_mag > 0.0);
            assert!(p.spl_db.is_finite(), "SPL no finito a {f} Hz");
            assert!(p.acoustical_power_w >= 0.0);
        }
    }

    #[test]
    fn group_delay_is_finite() {
        let sys = default_system();
        let freqs: Vec<f64> = (0..20).map(|i| 20.0 * 1.2_f64.powi(i)).collect();
        let pts = sys.response(&freqs);
        assert_eq!(pts.len(), freqs.len());
        assert!(pts.iter().all(|p| p.group_delay_ms.is_finite()));
    }
}
