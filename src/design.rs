//! Fachada de alto nivel del módulo de acústica.
//!
//! Agrupa las topologías ya implementadas bajo un tipo común [`Design`] y una
//! respuesta común [`DesignPoint`], pensada para ser consumida por la UI
//! (egui) y por el futuro módulo de diseño paramétrico.
//!
//! ```no_run
//! use speakerbox_acoustics::{Design, System, Air, Driver};
//!
//! let driver = Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1);
//! let design = Design::System(System::new(Air::default(), driver));
//! let points = design.sweep_log(20.0, 2000.0, 100);
//! ```

use num_complex::Complex64;

use crate::bandpass::{Bandpass, BandpassPoint};
use crate::error::{check_frequency, SolveError};
use crate::offset::{OffsetHorn, OffsetPoint};
use crate::system::{ResponsePoint, System};
use crate::tapped::{TappedHorn, TappedPoint};

/// Punto de respuesta unificado para cualquier topología.
#[derive(Clone, Copy, Debug)]
pub struct DesignPoint {
    /// Frecuencia (Hz).
    pub freq_hz: f64,
    /// SPL de potencia a 1 m (dB).
    pub spl_db: f64,
    /// Fase reducida de presión de campo (grados, desenrollada en response).
    /// La presión estimada añade +90°; NaN en silencio.
    pub phase_deg: f64,
    /// Retardo de grupo (ms). Vale `0.0` en `response_at`; `Design::response`
    /// lo calcula a partir de la fase desenrollada.
    pub group_delay_ms: f64,
    /// Módulo de la impedancia eléctrica (Ω).
    pub ze_mag: f64,
    /// Fase de la impedancia eléctrica (grados).
    pub ze_phase_deg: f64,
    /// Desplazamiento de pico del diafragma (mm).
    pub xd_mm: f64,
    /// Potencia activa absorbida por las terminaciones explícitas (W).
    /// Puede ser un subtotal: consultar try_energy_at para calidad/radiación.
    /// Nunca se deduce de SPL ni incluye pérdidas mecánicas de un PR.
    pub power_w: f64,
    /// 100·potencia de terminaciones / potencia activa de la fuente (incluye Rg).
    /// No es η radiativa si el modelo está incompleto; NaN indica 0/0.
    /// try_energy_at distingue la potencia radiada disponible o aproximada.
    pub efficiency_pct: f64,
    /// Parte real de la impedancia acústica de referencia (Pa·s/m³).
    ///
    /// En `System` es la carga frontal del diafragma; en `TappedHorn` es la
    /// impedancia de cuello; en `OffsetHorn` es la impedancia en la toma.
    /// En `Bandpass` no se calcula y queda a `0.0`: **cero no significa
    /// impedancia nula**, sino «no aplicable».
    pub za_real: f64,
    /// Parte imaginaria de la impedancia acústica de referencia (Pa·s/m³).
    ///
    /// Misma convención que [`DesignPoint::za_real`]: en `Bandpass` no se
    /// calcula y queda a `0.0`.
    pub za_imag: f64,
    /// Velocidad del aire en el puerto, si la topología tiene puerto.
    ///
    /// `None` significa «no aplicable» (TH, OD sin puerto trasero o radiador
    /// directo sin puerto), no una velocidad desconocida.
    pub port_velocity: Option<f64>,
}

/// Observación de campo legado: una estimación bajo la hipótesis compacta,
/// no una solución espacial. matched_mouth no proporciona esta observación.
#[derive(Clone, Copy, Debug)]
pub struct AcousticObservation {
    pub pressure_estimate_pa: Option<Complex64>,
    pub spl_db: Option<f64>,
    /// Fase de presión física respecto a Eg, sin trayecto común al observador.
    pub phase_deg: Option<f64>,
    pub group_delay_ms: Option<f64>,
}

/// Topología acústica.
#[derive(Clone, Debug)]
pub enum Design {
    /// Red completa de radiador directo, caja sellada/ventilada o bocina
    /// cargada en la garganta.
    System(System),
    /// Tapped horn TH/TH1.
    TappedHorn(TappedHorn),
    /// Bocina con driver desplazado OD/OD1/OD2.
    OffsetHorn(OffsetHorn),
    /// Recinto bandpass de 4º o 6º orden.
    Bandpass(Bandpass),
    TransmissionLine(crate::TransmissionLine),
}

/// Respuesta nativa para consumidores que necesitan campos por topología.
#[derive(Clone, Copy, Debug)]
pub enum TopologyPoint {
    System(ResponsePoint),
    TappedHorn(TappedPoint),
    OffsetHorn(OffsetPoint),
    Bandpass(BandpassPoint),
    TransmissionLine(crate::LinePoint),
}

/// Configuración validada prestada; el solver sigue validando cada resultado.
pub struct PreparedDesign<'a> {
    design: &'a Design,
}
impl PreparedDesign<'_> {
    pub fn try_response_at(&self, f: f64) -> Result<DesignPoint, SolveError> {
        check_frequency(f)?;
        self.design.response_validated(f)
    }
}

impl DesignPoint {
    /// Comprueba que el punto unificado sea físicamente interpretable.
    ///
    /// Los campos no aplicables a una topología (`za_real`/`za_imag` en
    /// `Bandpass`, `group_delay_ms` en `response_at`, `port_velocity` en
    /// topologías sin puerto) se validan como valores neutros: `0.0` o `None`.
    pub fn validate(&self) -> Result<(), SolveError> {
        if !self.freq_hz.is_finite() || self.freq_hz <= 0.0 {
            return Err(SolveError::InvalidResponse("freq_hz"));
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
        if !(self.xd_mm.is_finite() && self.xd_mm >= 0.0) {
            return Err(SolveError::InvalidResponse("xd_mm"));
        }
        if !(self.power_w.is_finite() && self.power_w >= 0.0) {
            return Err(SolveError::InvalidResponse("power_w"));
        }
        if !((self.efficiency_pct.is_finite() && self.efficiency_pct >= 0.0)
            || (self.efficiency_pct.is_nan() && self.power_w == 0.0))
        {
            return Err(SolveError::InvalidResponse("efficiency_pct"));
        }
        if !self.za_real.is_finite() {
            return Err(SolveError::InvalidResponse("za_real"));
        }
        if !self.za_imag.is_finite() {
            return Err(SolveError::InvalidResponse("za_imag"));
        }
        if let Some(velocity) = self.port_velocity {
            if !(velocity.is_finite() && velocity >= 0.0) {
                return Err(SolveError::InvalidResponse("port_velocity"));
            }
        }
        Ok(())
    }
}

fn from_system(p: ResponsePoint) -> DesignPoint {
    DesignPoint {
        freq_hz: p.freq,
        spl_db: p.spl_db,
        phase_deg: p.phase_deg,
        group_delay_ms: p.group_delay_ms,
        ze_mag: p.ze_mag,
        ze_phase_deg: p.ze_phase_deg,
        xd_mm: p.displacement_mm,
        power_w: p.acoustical_power_w,
        efficiency_pct: p.efficiency_pct,
        za_real: p.za_real,
        za_imag: p.za_imag,
        port_velocity: p.port_velocity,
    }
}

fn from_tapped(p: TappedPoint, za: Complex64) -> DesignPoint {
    DesignPoint {
        freq_hz: p.freq,
        spl_db: p.spl_db,
        phase_deg: p.phase_deg,
        group_delay_ms: 0.0,
        ze_mag: p.ze_mag,
        ze_phase_deg: p.ze_phase_deg,
        xd_mm: p.displacement_mm,
        power_w: p.power_w,
        efficiency_pct: p.efficiency_pct,
        za_real: za.re,
        za_imag: za.im,
        port_velocity: None,
    }
}

fn from_bandpass(p: BandpassPoint) -> DesignPoint {
    let port_velocity = Some(
        p.front_port_velocity
            .max(p.rear_port_velocity.unwrap_or(0.0)),
    );
    DesignPoint {
        freq_hz: p.freq,
        spl_db: p.spl_db,
        phase_deg: p.phase_deg,
        group_delay_ms: 0.0,
        ze_mag: p.ze_mag,
        ze_phase_deg: p.ze_phase_deg,
        xd_mm: p.displacement_mm,
        power_w: p.power_w,
        efficiency_pct: p.efficiency_pct,
        za_real: 0.0,
        za_imag: 0.0,
        port_velocity,
    }
}

fn from_offset(p: OffsetPoint, za: Complex64) -> DesignPoint {
    DesignPoint {
        freq_hz: p.freq,
        spl_db: p.spl_db,
        phase_deg: p.phase_deg,
        group_delay_ms: 0.0,
        ze_mag: p.ze_mag,
        ze_phase_deg: p.ze_phase_deg,
        xd_mm: p.displacement_mm,
        power_w: p.power_w,
        efficiency_pct: p.efficiency_pct,
        za_real: za.re,
        za_imag: za.im,
        port_velocity: p.port_velocity,
    }
}

impl Design {
    /// Respuesta a una frecuencia. Devuelve un punto neutro si la entrada o el
    /// diseño no son resolubles; use [`Design::try_response_at`] para
    /// distinguir el error.
    pub fn response_at(&self, freq_hz: f64) -> DesignPoint {
        self.try_response_at(freq_hz)
            .unwrap_or_else(|_| Self::zero_point(freq_hz))
    }

    /// Respuesta a una frecuencia, con error explícito si la entrada, la
    /// configuración o el resultado no son válidos.
    pub fn try_response_at(&self, freq_hz: f64) -> Result<DesignPoint, SolveError> {
        check_frequency(freq_hz)?;
        self.validate()?;
        self.response_validated(freq_hz)
    }

    fn response_validated(&self, freq_hz: f64) -> Result<DesignPoint, SolveError> {
        let point = match self {
            Design::TransmissionLine(tl) => tl.try_response_at(freq_hz)?.summary,
            Design::System(sys) => from_system(sys.response_validated(freq_hz)?),
            Design::TappedHorn(th) => {
                let p = th.response_validated(freq_hz)?;
                from_tapped(p, th.neck_impedance(freq_hz))
            }
            Design::OffsetHorn(od) => {
                let solution = od.solution_validated(freq_hz)?;
                from_offset(solution.point, solution.front_impedance)
            }
            Design::Bandpass(bp) => from_bandpass(bp.response_validated(freq_hz)?),
        };
        point.validate()?;
        Ok(point)
    }

    /// Valida una vez y presta inmutablemente toda la configuración.
    /// Mientras exista el préstamo no puede cambiar geometría, aire, driver,
    /// calibración ni excitación. No hay caché global ni invalidación manual.
    pub fn prepare(&self) -> Result<PreparedDesign<'_>, SolveError> {
        self.validate()?;
        Ok(PreparedDesign { design: self })
    }

    /// Respuesta nativa sin reducir los puertos a una velocidad máxima.
    pub fn try_topology_response_at(&self, f: f64) -> Result<TopologyPoint, SolveError> {
        match self {
            Self::TransmissionLine(s) => Ok(TopologyPoint::TransmissionLine(s.try_response_at(f)?)),
            Self::System(s) => Ok(TopologyPoint::System(s.try_response_at(f)?)),
            Self::TappedHorn(s) => Ok(TopologyPoint::TappedHorn(s.try_response_at(f)?)),
            Self::OffsetHorn(s) => Ok(TopologyPoint::OffsetHorn(s.try_response_at(f)?)),
            Self::Bandpass(s) => Ok(TopologyPoint::Bandpass(s.try_response_at(f)?)),
        }
    }

    /// Impedancia de referencia opcional; BP no dispone de esa referencia.
    pub fn try_reference_impedance_at(&self, f: f64) -> Result<Option<Complex64>, SolveError> {
        let p = self.try_response_at(f)?;
        Ok(if matches!(self, Self::Bandpass(_)) {
            None
        } else {
            Some(Complex64::new(p.za_real, p.za_imag))
        })
    }

    /// Presión y fase estimadas del mismo SPL legado, con ausencia explícita
    /// ante matched_mouth y fase indefinida ante silencio/cancelación exacta.
    pub fn try_observation_at(&self, f: f64) -> Result<AcousticObservation, SolveError> {
        check_frequency(f)?;
        self.validate()?;
        if matches!(self,Self::System(s) if s.horn.as_ref().is_some_and(|h|h.matched_mouth)) {
            return Ok(AcousticObservation {
                pressure_estimate_pa: None,
                spl_db: None,
                phase_deg: None,
                group_delay_ms: None,
            });
        }
        let p = self.try_response_at(f)?;
        let pref = match self {
            Self::TransmissionLine(s) => s.air.p_ref,
            Self::System(s) => s.air.p_ref,
            Self::TappedHorn(s) => s.air.p_ref,
            Self::OffsetHorn(s) => s.air.p_ref,
            Self::Bandpass(s) => s.air.p_ref,
        };
        let silent = p.spl_db == f64::NEG_INFINITY;
        let pressure = if silent {
            Complex64::new(0.0, 0.0)
        } else {
            Complex64::from_polar(
                pref * 10.0_f64.powf(p.spl_db / 20.0),
                (p.phase_deg + 90.).to_radians(),
            )
        };
        if !pressure.re.is_finite() || !pressure.im.is_finite() {
            return Err(SolveError::NonFiniteResponse);
        }
        Ok(AcousticObservation {
            pressure_estimate_pa: Some(pressure),
            spl_db: Some(p.spl_db),
            phase_deg: if silent {
                None
            } else {
                Some(p.phase_deg + 90.)
            },
            group_delay_ms: None,
        })
    }

    /// Ordena solo para derivar y restaura el orden, incluidas frecuencias
    /// repetidas. Una muestra aislada o una cancelación no produce retardo.
    pub fn try_observation_response(
        &self,
        freqs: &[f64],
    ) -> Result<Vec<AcousticObservation>, SolveError> {
        let mut out = freqs
            .iter()
            .map(|&f| self.try_observation_at(f))
            .collect::<Result<Vec<_>, _>>()?;
        let mut indices: Vec<usize> = (0..freqs.len()).collect();
        indices.sort_by(|&a, &b| freqs[a].total_cmp(&freqs[b]));
        indices.dedup_by(|a, b| freqs[*a] == freqs[*b]);
        let f: Vec<_> = indices.iter().map(|&i| freqs[i]).collect();
        let phases: Vec<_> = indices.iter().map(|&i| out[i].phase_deg).collect();
        let trace = crate::phase::phase_trace(&f, &phases)?;
        for (i, o) in out.iter_mut().enumerate() {
            let j = f
                .binary_search_by(|v| v.total_cmp(&freqs[i]))
                .map_err(|_| SolveError::InvalidFrequency(freqs[i]))?;
            o.phase_deg = trace[j].phase_deg;
            o.group_delay_ms = trace[j].group_delay_ms;
        }
        Ok(out)
    }

    /// Balance científico sin deducir potencia de SPL ni descartar su calidad.
    pub fn try_energy_at(&self, f: f64) -> Result<crate::energy::EnergyReport, SolveError> {
        match self {
            Self::TransmissionLine(s) => s.try_energy_at(f),
            Self::System(s) => s.try_energy_at(f),
            Self::TappedHorn(s) => s.try_energy_at(f),
            Self::OffsetHorn(s) => s.try_energy_at(f),
            Self::Bandpass(s) => s.try_energy_at(f),
        }
    }

    /// Comprueba la configuración de la topología antes de calcular.
    pub fn validate(&self) -> Result<(), SolveError> {
        match self {
            Design::TransmissionLine(s) => s.validate(),
            Design::System(sys) => sys.validate(),
            Design::TappedHorn(th) => th.validate(),
            Design::OffsetHorn(od) => od.validate(),
            Design::Bandpass(bp) => bp.validate(),
        }
    }

    /// Punto neutro para una entrada o un diseño no resoluble.
    fn zero_point(freq_hz: f64) -> DesignPoint {
        DesignPoint {
            freq_hz,
            spl_db: f64::NEG_INFINITY,
            phase_deg: 0.0,
            group_delay_ms: 0.0,
            ze_mag: 0.0,
            ze_phase_deg: 0.0,
            xd_mm: 0.0,
            power_w: 0.0,
            efficiency_pct: 0.0,
            za_real: 0.0,
            za_imag: 0.0,
            port_velocity: None,
        }
    }

    /// Barrido de respuesta con fase desenrollada y retardo de grupo.
    ///
    /// Se eliminan las frecuencias no finitas o no positivas. Los puntos de un
    /// diseño no resoluble se devuelven como puntos neutros; use
    /// [`Design::try_sweep_log`] o `try_response_at` para distinguir el error.
    pub fn response(&self, freqs: &[f64]) -> Vec<DesignPoint> {
        let mut pts: Vec<DesignPoint> = freqs
            .iter()
            .copied()
            .filter(|f| f.is_finite() && *f > 0.0)
            .map(|f| self.response_at(f))
            .collect();
        Self::finalize(&mut pts);
        pts
    }

    /// Desenrolla la fase y calcula el retardo de grupo.
    fn finalize(pts: &mut [DesignPoint]) {
        let freqs: Vec<f64> = pts.iter().map(|p| p.freq_hz).collect();
        let mut phases: Vec<f64> = pts.iter().map(|p| p.phase_deg).collect();
        let spl: Vec<f64> = pts.iter().map(|p| p.spl_db).collect();
        let delays = crate::phase::finalize(&freqs, &mut phases, &spl);
        for (i, p) in pts.iter_mut().enumerate() {
            p.phase_deg = phases[i];
            p.group_delay_ms = delays[i];
        }
    }

    /// Barrido logarítmico entre `fmin` y `fmax` con `points` muestras.
    ///
    /// Devuelve un vector vacío si los límites, el número de puntos o el
    /// diseño no son válidos. Use [`Design::try_sweep_log`] para obtener el
    /// error.
    pub fn sweep_log(&self, fmin: f64, fmax: f64, points: usize) -> Vec<DesignPoint> {
        self.try_sweep_log(fmin, fmax, points).unwrap_or_default()
    }

    /// Barrido logarítmico con error explícito.
    ///
    /// Exige `fmin` y `fmax` finitos y positivos, `fmax > fmin` y al menos dos
    /// puntos. Valida el diseño y cada punto; el primer error se propaga.
    pub fn try_sweep_log(
        &self,
        fmin: f64,
        fmax: f64,
        points: usize,
    ) -> Result<Vec<DesignPoint>, SolveError> {
        check_frequency(fmin)?;
        check_frequency(fmax)?;
        if fmax <= fmin {
            return Err(SolveError::InvalidParameter("fmax"));
        }
        if points < 2 {
            return Err(SolveError::InvalidParameter("points"));
        }
        let prepared = self.prepare()?;
        let mut out = Vec::with_capacity(points);
        for i in 0..points {
            let t = i as f64 / (points - 1) as f64;
            let f = fmin * (fmax / fmin).powf(t);
            out.push(prepared.try_response_at(f)?);
        }
        Self::finalize(&mut out);
        for point in &out {
            point.validate()?;
        }
        Ok(out)
    }
}

impl From<System> for Design {
    fn from(value: System) -> Self {
        Design::System(value)
    }
}

impl From<TappedHorn> for Design {
    fn from(value: TappedHorn) -> Self {
        Design::TappedHorn(value)
    }
}

impl From<OffsetHorn> for Design {
    fn from(value: OffsetHorn) -> Self {
        Design::OffsetHorn(value)
    }
}

impl From<Bandpass> for Design {
    fn from(value: Bandpass) -> Self {
        Design::Bandpass(value)
    }
}

impl From<crate::TransmissionLine> for Design {
    fn from(t: crate::TransmissionLine) -> Self {
        Self::TransmissionLine(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bandpass::Bandpass;
    use crate::driver::Driver;
    use crate::element::{EndCorrection, Port};
    use crate::segment::{Flare, Segment};
    use crate::Air;

    fn driver() -> Driver {
        Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1)
    }

    #[test]
    fn system_variant_preserves_response() {
        let sys = System::new(Air::default(), driver());
        let direct = sys.response_at(100.0);
        let design = Design::System(sys);
        let p = design.response_at(100.0);
        assert!((p.spl_db - direct.spl_db).abs() < 1.0e-12);
        assert!((p.ze_mag - direct.ze_mag).abs() < 1.0e-12);
    }

    #[test]
    fn tapped_horn_variant_preserves_response() {
        let th = TappedHorn::new_4seg_th(
            driver(),
            vec![
                Segment::new(Flare::Conical, 0.40, 400e-4, 400e-4),
                Segment::new(Flare::Exponential, 0.20, 400e-4, 400e-4),
                Segment::new(Flare::Conical, 1.20, 400e-4, 1200e-4),
                Segment::new(Flare::Conical, 0.30, 1200e-4, 1500e-4),
            ],
        );
        let direct = th.response_at(50.0);
        let design = Design::TappedHorn(th);
        let p = design.response_at(50.0);
        assert!((p.spl_db - direct.spl_db).abs() < 1.0e-12);
    }

    #[test]
    fn offset_horn_variant_preserves_response() {
        let od = OffsetHorn::new_4seg_od(
            driver(),
            vec![
                Segment::new(Flare::Conical, 0.40, 400e-4, 400e-4),
                Segment::new(Flare::Exponential, 0.20, 400e-4, 400e-4),
                Segment::new(Flare::Conical, 1.20, 400e-4, 1200e-4),
                Segment::new(Flare::Conical, 0.30, 1200e-4, 1500e-4),
            ],
        );
        let direct = od.response_at(50.0);
        let design = Design::OffsetHorn(od);
        let p = design.response_at(50.0);
        assert!((p.spl_db - direct.spl_db).abs() < 1.0e-12);
    }

    #[test]
    fn try_response_at_reports_invalid_frequencies() {
        let design = Design::System(System::new(Air::default(), driver()));
        for f in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                design.try_response_at(f),
                Err(SolveError::InvalidFrequency(_))
            ));
        }
        assert!(design.try_response_at(100.0).is_ok());
    }

    #[test]
    fn try_response_at_reports_invalid_tap() {
        let mut od = OffsetHorn::new_4seg_od(
            driver(),
            vec![
                Segment::new(Flare::Conical, 0.40, 400e-4, 400e-4),
                Segment::new(Flare::Exponential, 0.20, 400e-4, 400e-4),
                Segment::new(Flare::Conical, 1.20, 400e-4, 1200e-4),
                Segment::new(Flare::Conical, 0.30, 1200e-4, 1500e-4),
            ],
        );
        od.tap = 99;
        let design = Design::OffsetHorn(od);
        assert!(matches!(
            design.try_response_at(100.0),
            Err(SolveError::InvalidTopology(_))
        ));
    }

    #[test]
    fn bandpass_variant_preserves_response() {
        let port = Port::new(50.0e-4, 10.0e-2, EndCorrection::UnflangedBoth);
        let bp = Bandpass::new_bp4(driver(), 20.0e-3, 10.0e-3, port);
        let direct = bp.response_at(100.0);
        let design = Design::Bandpass(bp);
        let p = design.response_at(100.0);
        assert!((p.spl_db - direct.spl_db).abs() < 1.0e-12);
    }

    #[test]
    fn log_sweep_is_monotonic_and_finite() {
        let design = Design::System(System::new(Air::default(), driver()));
        let pts = design.sweep_log(20.0, 2000.0, 32);
        assert_eq!(pts.len(), 32);
        assert!(pts.windows(2).all(|w| w[1].freq_hz > w[0].freq_hz));
        assert!(pts.iter().all(|p| p.spl_db.is_finite() && p.ze_mag > 0.0));
        assert!(pts.iter().all(|p| p.group_delay_ms.is_finite()));
    }

    #[test]
    fn sweep_log_rejects_invalid_bounds_and_point_counts() {
        let design = Design::System(System::new(Air::default(), driver()));
        for (fmin, fmax, points) in [
            (0.0, 100.0, 10),
            (-1.0, 100.0, 10),
            (f64::NAN, 100.0, 10),
            (20.0, f64::INFINITY, 10),
            (100.0, 20.0, 10),
            (20.0, 100.0, 0),
            (20.0, 100.0, 1),
        ] {
            assert!(
                design.try_sweep_log(fmin, fmax, points).is_err(),
                "fmin={fmin}, fmax={fmax}, points={points}"
            );
            assert!(design.sweep_log(fmin, fmax, points).is_empty());
        }
        assert!(design.try_sweep_log(20.0, 100.0, 2).is_ok());
    }

    #[test]
    fn try_response_at_rejects_invalid_frequency_and_design() {
        let design = Design::System(System::new(Air::default(), driver()));
        for f in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                design.try_response_at(f),
                Err(SolveError::InvalidFrequency(_))
            ));
        }

        let mut sys = System::new(Air::default(), driver());
        sys.driver.sd = 0.0;
        let design = Design::System(sys);
        assert!(matches!(
            design.try_response_at(100.0),
            Err(SolveError::InvalidParameter("driver.sd"))
        ));
    }

    #[test]
    fn design_point_validation_rejects_invalid_fields() {
        let design = Design::System(System::new(Air::default(), driver()));
        let valid = design.response_at(100.0);
        assert!(valid.validate().is_ok());

        let mut p = valid;
        p.ze_mag = f64::NAN;
        assert!(p.validate().is_err());

        let mut p = valid;
        p.port_velocity = Some(-1.0);
        assert!(p.validate().is_err());

        let mut p = valid;
        p.za_imag = f64::INFINITY;
        assert!(p.validate().is_err());
    }

    #[test]
    fn response_filters_non_finite_frequencies() {
        let design = Design::System(System::new(Air::default(), driver()));
        let pts = design.response(&[f64::NAN, f64::INFINITY, 0.0, -1.0, 100.0]);
        assert_eq!(pts.len(), 1);
        assert!((pts[0].freq_hz - 100.0).abs() < 1.0e-12);
    }

    #[test]
    fn try_sweep_log_matches_sweep_log() {
        let design = Design::System(System::new(Air::default(), driver()));
        let a = design.sweep_log(20.0, 200.0, 8);
        let b = design.try_sweep_log(20.0, 200.0, 8).unwrap();
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(&b) {
            assert!((x.freq_hz - y.freq_hz).abs() < 1.0e-12);
            assert!((x.spl_db - y.spl_db).abs() < 1.0e-12);
            assert!((x.group_delay_ms - y.group_delay_ms).abs() < 1.0e-12);
        }
    }
}
