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
use crate::error::{
    check_frequency, require_finite, require_non_negative, require_positive, SolveError,
};
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
    /// Fase reducida de la presión total (grados): presión física añade +90°.
    pub phase_deg: f64,
    /// Retardo de grupo (ms).
    pub group_delay_ms: f64,
    /// Módulo de la impedancia eléctrica de entrada (Ω).
    pub ze_mag: f64,
    /// Fase de la impedancia eléctrica (grados).
    pub ze_phase_deg: f64,
    /// Desplazamiento de pico del diafragma (mm).
    pub displacement_mm: f64,
    /// Potencia activa absorbida por las terminaciones explícitas (W).
    /// Puede ser un subtotal: consultar try_energy_at para calidad/radiación.
    /// Nunca se deduce de SPL ni incluye pérdidas mecánicas de un PR.
    pub acoustical_power_w: f64,
    /// 100·potencia de terminaciones / potencia activa de la fuente (incluye Rg).
    /// No es η radiativa si el modelo está incompleto; NaN indica 0/0.
    /// try_energy_at distingue la potencia radiada disponible o aproximada.
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
    ///
    /// Si es `Some`, debe tener segmentos válidos ([`Horn::validate`]).
    pub horn: Option<Horn>,
    /// Cámara entre el diafragma y la garganta de la bocina.
    ///
    /// Requiere `horn: Some`; `validate` rechaza una cámara de garganta sin
    /// bocina.
    pub throat_chamber: Option<Chamber>,
    /// Recinto trasero.
    ///
    /// Es obligatorio para `port`, `passive_radiator` y `rear_ql`.
    pub rear_chamber: Option<Chamber>,
    /// Puerto del recinto trasero.
    ///
    /// Requiere `rear_chamber: Some`; `validate` rechaza un puerto sin recinto.
    pub port: Option<Port>,
    /// Radiador pasivo del recinto trasero.
    ///
    /// Requiere `rear_chamber: Some`.
    pub passive_radiator: Option<PassiveRadiator>,
    /// Factor QL del recinto trasero ventilado. `None` = sin pérdidas.
    ///
    /// Requiere `rear_chamber: Some` y, si existe, debe ser finito y positivo.
    pub rear_ql: Option<f64>,
    /// Ángulo sólido de radiación en estereorradianes.
    pub solid_angle: f64,
    /// Tensión de entrada (V); el signo invierte la polaridad.
    ///
    /// `System` no tiene campo `polarity`: usar `eg` negativo es la forma de
    /// invertir la fase del diafragma.
    pub eg: f64,
    /// Resistencia de la fuente (Ω).
    pub rg: f64,
    /// Corrección global del nivel de SPL (dB).
    pub spl_offset_db: f64,
    /// Fuga acústica en la garganta de la bocina (Pa·s/m³). `None` = sin fuga.
    ///
    /// Requiere `horn: Some`; si existe, debe ser finita y no negativa.
    pub horn_leak: Option<f64>,
    /// Resistencia serie en la garganta de la bocina (Pa·s/m³).
    ///
    /// Requiere `horn: Some`; si existe, debe ser finita y no negativa.
    pub horn_series: Option<f64>,
}

impl ResponsePoint {
    /// Comprueba que el punto sea físicamente interpretable.
    ///
    /// - `freq` finita y positiva.
    /// - `spl_db` finito o `-∞` (silencio); nunca `NaN` ni `+∞`.
    /// - `phase_deg`, `group_delay_ms`, `ze_phase_deg`, `za_real` y `za_imag`
    ///   finitos.
    /// - `ze_mag` finito y positivo.
    /// - `displacement_mm`, potencia, rendimiento y velocidades finitos y no
    ///   negativos.
    pub fn validate(&self) -> Result<(), SolveError> {
        if !self.freq.is_finite() || self.freq <= 0.0 {
            return Err(SolveError::InvalidResponse("freq"));
        }
        if !(self.spl_db.is_finite() || self.spl_db == f64::NEG_INFINITY) {
            return Err(SolveError::InvalidResponse("spl_db"));
        }
        if !(self.phase_deg.is_finite()
            || (self.phase_deg.is_nan() && self.spl_db == f64::NEG_INFINITY))
        {
            return Err(SolveError::InvalidResponse("phase_deg"));
        }
        if !self.group_delay_ms.is_finite() {
            return Err(SolveError::InvalidResponse("group_delay_ms"));
        }
        if !(self.ze_mag.is_finite() && self.ze_mag > 0.0) {
            return Err(SolveError::InvalidResponse("ze_mag"));
        }
        if !self.ze_phase_deg.is_finite() {
            return Err(SolveError::InvalidResponse("ze_phase_deg"));
        }
        if !(self.displacement_mm.is_finite() && self.displacement_mm >= 0.0) {
            return Err(SolveError::InvalidResponse("displacement_mm"));
        }
        if !(self.acoustical_power_w.is_finite() && self.acoustical_power_w >= 0.0) {
            return Err(SolveError::InvalidResponse("acoustical_power_w"));
        }
        if !((self.efficiency_pct.is_finite() && self.efficiency_pct >= 0.0)
            || (self.efficiency_pct.is_nan() && self.acoustical_power_w == 0.0))
        {
            return Err(SolveError::InvalidResponse("efficiency_pct"));
        }
        if !self.za_real.is_finite() {
            return Err(SolveError::InvalidResponse("za_real"));
        }
        if !self.za_imag.is_finite() {
            return Err(SolveError::InvalidResponse("za_imag"));
        }
        if !(self.mouth_volume_velocity.is_finite() && self.mouth_volume_velocity >= 0.0) {
            return Err(SolveError::InvalidResponse("mouth_volume_velocity"));
        }
        if let Some(velocity) = self.port_velocity {
            if !(velocity.is_finite() && velocity >= 0.0) {
                return Err(SolveError::InvalidResponse("port_velocity"));
            }
        }
        Ok(())
    }
}

/// Presión compleja de un radiador de área `area` para una velocidad
/// volumétrica `u`. La magnitud usa la resistencia de radiación del pistón;
/// la fase es la de `u` (el factor común `jωρ` no afecta a la suma).
pub(crate) fn radiator_pressure(
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
        let zh = self
            .horn
            .as_ref()
            .map(|h| h.input_impedance(&self.air, freq_hz, self.solid_angle));
        self.loads_with_horn(freq_hz, zh)
    }

    fn loads_with_horn(
        &self,
        freq_hz: f64,
        horn_impedance: Option<Complex64>,
    ) -> (Complex64, Complex64) {
        let front = if let Some(zh) = horn_impedance {
            // La resistencia pertenece a la rama de bocina; cámara y fuga
            // derivan desde el nodo del diafragma, antes de esa resistencia.
            let zh0 = zh + Complex64::new(self.horn_series.unwrap_or(0.0), 0.0);
            let zh = match self.horn_leak {
                Some(r) => crate::element::parallel(zh0, Complex64::new(r, 0.0)),
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
    ///
    /// Devuelve el punto neutro documentado (`spl_db = -∞`, `ze_mag = 0`) si
    /// la entrada, la configuración o la respuesta no son válidas. Use
    /// [`System::try_response_at`] para distinguir el error.
    pub fn response_at(&self, freq_hz: f64) -> ResponsePoint {
        self.try_response_at(freq_hz)
            .unwrap_or_else(|_| Self::zero_point(freq_hz))
    }

    /// Respuesta a una frecuencia, con error explícito si la entrada, la
    /// configuración o el resultado no son válidos.
    pub fn try_response_at(&self, freq_hz: f64) -> Result<ResponsePoint, SolveError> {
        check_frequency(freq_hz)?;
        self.validate()?;
        self.response_validated(freq_hz)
    }

    pub(crate) fn response_validated(&self, freq_hz: f64) -> Result<ResponsePoint, SolveError> {
        if self.horn.as_ref().is_some_and(|h| h.matched_mouth) {
            return Err(SolveError::InvalidResponse(
                "SPL unavailable for matched absorber; use try_energy_at",
            ));
        }
        let point = self.solve_point(freq_hz)?;
        point.validate()?;
        Ok(point)
    }

    /// Balance RMS de la red. Disponible también para matched_mouth, que no
    /// define SPL. No deduce pérdidas del residual ni potencia de la presión lejana.
    pub fn try_energy_at(&self, f: f64) -> Result<crate::energy::EnergyReport, SolveError> {
        use crate::energy::{
            load_termination, shunt_power, termination, EnergyReport, RadiationQuality as Q,
            TerminationKind as K,
        };
        check_frequency(f)?;
        self.validate()?;
        let prepared = self
            .horn
            .as_ref()
            .map(|h| h.prepare(&self.air, f, self.solid_angle));
        let (zf, zr) = self.loads_with_horn(f, prepared.as_ref().map(|h| h.input_impedance));
        let ud = self.driver.volume_velocity(f, self.eg, self.rg, zf + zr);
        let ze = self.driver.input_impedance(f, zf + zr) + self.rg;
        let current = Complex64::new(self.eg, 0.0) / ze;
        let mut report = EnergyReport::new(
            &self.driver,
            self.eg,
            self.rg,
            current,
            ud,
            (zf + zr).re * ud.norm_sqr(),
        );
        let mut quality = Q::SingleBaffledPiston;
        if let (Some(h), Some(prepared)) = (&self.horn, prepared) {
            let pn = zf * ud;
            let r = self.horn_series.unwrap_or(0.0);
            let uh = pn / (prepared.input_impedance + r);
            let mut p = pn - r * uh;
            let mut u = uh;
            report.acoustic_losses.throat_series_w = r * uh.norm_sqr();
            if let Some(r) = self.horn_leak {
                if r > 0.0 {
                    report.acoustic_losses.leakage_w += pn.norm_sqr() / r;
                }
            }
            if let Some(ch) = self.throat_chamber {
                if let Some(l) = ch.lining {
                    let z = l.impedance(&self.air, f);
                    if !z.re.is_infinite() {
                        report.acoustic_losses.lining_w += shunt_power(pn, z);
                    }
                }
            }
            for (seg, m) in h.segments.iter().zip(&prepared.matrices) {
                report.acoustic_losses.wall_w += seg.dissipated_power_w(&self.air, f, p, u)?;
                (p, u) = m.apply(p, u);
            }
            let kind = if h.matched_mouth {
                quality = Q::MatchedAbsorber;
                K::MatchedAbsorber
            } else if h.radiation_scale != 1.0 || h.mouth_radiation_area_factor != 1.0 {
                quality = Q::CalibratedLoadApproximation;
                K::CalibratedPiston
            } else {
                K::BaffledPiston
            };
            report
                .terminations
                .push(termination(kind, p, u, prepared.termination_impedance));
        } else {
            report
                .terminations
                .push(load_termination(K::BaffledPiston, ud, zf));
        }
        if let Some(ch) = self.rear_chamber {
            let p = -zr * ud;
            if let Some(l) = ch.lining {
                let z = l.impedance(&self.air, f);
                if !z.re.is_infinite() {
                    report.acoustic_losses.lining_w += shunt_power(p, z);
                }
            }
            if let (Some(ql), Some(fb)) = (self.rear_ql, self.rear_tuning_hz()) {
                let r = ql / (std::f64::consts::TAU * fb * ch.compliance(&self.air));
                report.acoustic_losses.chamber_ql_w = p.norm_sqr() / r;
            }
            if let Some(pr) = self.passive_radiator {
                let upr = p / pr.impedance(f);
                report.acoustic_losses.passive_suspension_w = pr.rms * (upr / pr.sd).norm_sqr();
            }
            if (self.port.is_some() || self.passive_radiator.is_some())
                && quality != Q::MatchedAbsorber
            {
                quality = Q::IncompleteRadiationModel;
            }
        }
        report.finish(quality)
    }

    /// Comprueba la configuración del sistema antes de calcular.
    ///
    /// Valida aire, driver, bocina, cámaras, puerto, radiador pasivo, pérdidas
    /// y coherencia estructural (por ejemplo, un puerto exige recinto trasero).
    pub fn validate(&self) -> Result<(), SolveError> {
        self.air.validate()?;
        self.driver.validate()?;
        if let Some(horn) = &self.horn {
            horn.validate()?;
        }
        if let Some(chamber) = &self.throat_chamber {
            chamber.validate()?;
        }
        if let Some(chamber) = &self.rear_chamber {
            chamber.validate()?;
        }
        if let Some(port) = &self.port {
            port.validate()?;
        }
        if let Some(radiator) = &self.passive_radiator {
            radiator.validate()?;
        }
        if let Some(ql) = self.rear_ql {
            require_positive(ql, "rear_ql")?;
        }
        if let Some(leak) = self.horn_leak {
            require_non_negative(leak, "horn_leak")?;
        }
        if let Some(series) = self.horn_series {
            require_non_negative(series, "horn_series")?;
        }
        require_positive(self.solid_angle, "solid_angle")?;
        require_finite(self.eg, "eg")?;
        require_non_negative(self.rg, "rg")?;
        require_finite(self.spl_offset_db, "spl_offset_db")?;

        if self.horn.is_none() {
            if self.throat_chamber.is_some() {
                return Err(SolveError::InvalidTopology("throat_chamber requiere horn"));
            }
            if self.horn_leak.is_some() {
                return Err(SolveError::InvalidTopology("horn_leak requiere horn"));
            }
            if self.horn_series.is_some() {
                return Err(SolveError::InvalidTopology("horn_series requiere horn"));
            }
        }
        if self.rear_chamber.is_none() {
            if self.port.is_some() {
                return Err(SolveError::InvalidTopology("port requiere rear_chamber"));
            }
            if self.passive_radiator.is_some() {
                return Err(SolveError::InvalidTopology(
                    "passive_radiator requiere rear_chamber",
                ));
            }
            if self.rear_ql.is_some() {
                return Err(SolveError::InvalidTopology("rear_ql requiere rear_chamber"));
            }
        }
        Ok(())
    }

    /// Punto neutro para una entrada o un sistema no resoluble.
    fn zero_point(freq_hz: f64) -> ResponsePoint {
        ResponsePoint {
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
        }
    }

    /// Resuelve la red a la frecuencia `freq_hz` sin validar la entrada.
    fn solve_point(&self, freq_hz: f64) -> Result<ResponsePoint, SolveError> {
        let prepared = self
            .horn
            .as_ref()
            .map(|h| h.prepare(&self.air, freq_hz, self.solid_angle));
        let (z_front, z_rear) =
            self.loads_with_horn(freq_hz, prepared.as_ref().map(|h| h.input_impedance));
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
        let u_mouth = if let Some(horn) = &prepared {
            let zh_horn = horn.input_impedance;
            // Presión en el nodo de garganta y corriente que entra realmente
            // en la bocina. Con una cámara de garganta en derivación, parte
            // del volumen del diafragma se va a la compliancia de la cámara:
            // no se puede usar u_d como velocidad de entrada de la bocina.
            let p_throat = z_front * u_d;
            let r_series = self.horn_series.unwrap_or(0.0);
            let u_horn = p_throat / (zh_horn + Complex64::new(r_series, 0.0));
            let p_horn = p_throat - Complex64::new(r_series, 0.0) * u_horn;
            horn.transmit(p_horn, u_horn)
        } else {
            u_d
        };
        // Presión radiada. En una bocina se usa la velocidad de boca; en un
        // radiador directo con puerto o radiador pasivo se suman las presiones
        // complejas del diafragma y de cada salida trasera.
        let (p_rms, _field_power, phase_deg) = if self.horn.is_some() {
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
            let mut pressure = Complex64::from_polar(p_rms, u_mouth.arg());
            if self.rear_chamber.is_some() {
                let p_rear = -u_d * z_rear;
                for (area, z) in self
                    .port
                    .map(|p| (p.area, p.impedance(&self.air, freq_hz)))
                    .into_iter()
                    .chain(
                        self.passive_radiator
                            .map(|pr| (pr.sd, pr.impedance(freq_hz))),
                    )
                {
                    pressure +=
                        radiator_pressure(&self.air, area, p_rear / z, freq_hz, self.solid_angle);
                }
            }
            if self.port.is_some() || self.passive_radiator.is_some() {
                let rms = pressure.norm();
                (
                    rms,
                    rms * rms * self.solid_angle / (self.air.rho * self.air.c),
                    pressure.arg().to_degrees(),
                )
            } else {
                (p_rms, p_ac, u_mouth.arg().to_degrees())
            }
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
        // El campo de observación no modifica la potencia de la carga resuelta.
        // Puertos lumped/PR no tienen aquí una terminación radiativa explícita.
        let p_ac = u_mouth.norm_sqr()
            * prepared
                .as_ref()
                .map_or(z_front.re, |h| h.termination_impedance.re);
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

        Ok(ResponsePoint {
            freq: freq_hz,
            spl_db: spl,
            phase_deg: if p_rms > 0.0 { phase_deg } else { f64::NAN },
            group_delay_ms: 0.0,
            ze_mag: ze_in.norm(),
            ze_phase_deg: ze_in.arg().to_degrees(),
            displacement_mm: disp_mm,
            acoustical_power_w: p_ac,
            efficiency_pct: if p_in > 0.0 {
                100.0 * p_ac / p_in
            } else {
                f64::NAN
            },
            za_real: z_front.re,
            za_imag: z_front.im,
            mouth_volume_velocity: u_mouth.norm(),
            port_velocity,
        })
    }

    /// Barrido de respuesta. Se eliminan las frecuencias no finitas o no
    /// positivas y se calcula la fase desenrollada y el retardo de grupo.
    pub fn response(&self, freqs: &[f64]) -> Vec<ResponsePoint> {
        let mut pts: Vec<ResponsePoint> = freqs
            .iter()
            .copied()
            .filter(|f| f.is_finite() && *f > 0.0)
            .map(|f| self.response_at(f))
            .collect();
        let freqs: Vec<f64> = pts.iter().map(|p| p.freq).collect();
        let mut phases: Vec<f64> = pts.iter().map(|p| p.phase_deg).collect();
        let spl: Vec<f64> = pts.iter().map(|p| p.spl_db).collect();
        let delays = crate::phase::finalize(&freqs, &mut phases, &spl);
        for (i, p) in pts.iter_mut().enumerate() {
            p.phase_deg = phases[i];
            p.group_delay_ms = delays[i];
        }

        pts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::Driver;
    use crate::element::EndCorrection;
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

    #[test]
    fn try_response_at_rejects_invalid_frequency() {
        let sys = default_system();
        for f in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(
                matches!(sys.try_response_at(f), Err(SolveError::InvalidFrequency(_))),
                "f={f}"
            );
        }
        assert!(sys.try_response_at(100.0).is_ok());
    }

    #[test]
    fn try_response_at_rejects_invalid_driver() {
        let mut sys = default_system();
        sys.driver.sd = 0.0;
        assert!(matches!(
            sys.try_response_at(100.0),
            Err(SolveError::InvalidParameter("driver.sd"))
        ));

        let mut sys = default_system();
        sys.driver.mmd = f64::NAN;
        assert!(matches!(
            sys.try_response_at(100.0),
            Err(SolveError::InvalidParameter("driver.mmd"))
        ));
    }

    #[test]
    fn try_response_at_rejects_incoherent_configuration() {
        let mut sys = default_system();
        sys.port = Some(Port::new(100.0e-4, 0.10, EndCorrection::Inner(0.85)));
        assert!(matches!(
            sys.try_response_at(100.0),
            Err(SolveError::InvalidTopology(_))
        ));

        let mut sys = default_system();
        sys.throat_chamber = Some(Chamber::new(4.0e-3));
        assert!(matches!(
            sys.try_response_at(100.0),
            Err(SolveError::InvalidTopology(_))
        ));
    }

    #[test]
    fn response_point_validation_rejects_invalid_fields() {
        let sys = default_system();
        let valid = sys.response_at(100.0);
        assert!(valid.validate().is_ok());

        let mut p = valid;
        p.ze_mag = f64::NAN;
        assert!(p.validate().is_err());

        let mut p = valid;
        p.ze_mag = 0.0;
        assert!(p.validate().is_err());

        let mut p = valid;
        p.spl_db = f64::INFINITY;
        assert!(p.validate().is_err());

        let mut p = valid;
        p.displacement_mm = -1.0;
        assert!(p.validate().is_err());

        let mut p = valid;
        p.port_velocity = Some(f64::NAN);
        assert!(p.validate().is_err());

        let mut p = valid;
        p.za_real = f64::NAN;
        assert!(p.validate().is_err());
    }

    #[test]
    fn response_filters_non_finite_frequencies() {
        let sys = default_system();
        let pts = sys.response(&[f64::NAN, f64::INFINITY, 0.0, -1.0, 100.0]);
        assert_eq!(pts.len(), 1);
        assert!((pts[0].freq - 100.0).abs() < 1.0e-12);
    }

    #[test]
    fn negative_eg_is_valid_and_inverts_phase() {
        let positive = default_system();
        let mut negative = default_system();
        negative.eg = -2.83;
        let a = positive.response_at(100.0);
        let b = negative
            .try_response_at(100.0)
            .expect("una tensión negativa es una inversión de polaridad válida");
        assert!((a.spl_db - b.spl_db).abs() < 1.0e-9);
        let mut diff = (b.phase_deg - a.phase_deg).abs();
        if diff > 180.0 {
            diff = 360.0 - diff;
        }
        assert!((diff - 180.0).abs() < 1.0e-6, "diferencia de fase={diff}");
    }

    #[test]
    fn response_at_returns_neutral_point_on_invalid_input() {
        let sys = default_system();
        let p = sys.response_at(f64::NAN);
        assert_eq!(p.spl_db, f64::NEG_INFINITY);
        assert_eq!(p.ze_mag, 0.0);
    }
}
