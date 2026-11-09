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
use crate::error::SolveError;
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
    /// Fase de la velocidad volumétrica radiada (grados, desenrollada en
    /// `response`). La presión lejana añade +90°.
    pub phase_deg: f64,
    /// Retardo de grupo (ms).
    pub group_delay_ms: f64,
    /// Módulo de la impedancia eléctrica (Ω).
    pub ze_mag: f64,
    /// Fase de la impedancia eléctrica (grados).
    pub ze_phase_deg: f64,
    /// Desplazamiento de pico del diafragma (mm).
    pub xd_mm: f64,
    /// Potencia acústica radiada (W).
    pub power_w: f64,
    /// Rendimiento acústico (%).
    pub efficiency_pct: f64,
    /// Parte real de la impedancia acústica de referencia (Pa·s/m³).
    ///
    /// En `System` es la carga frontal del diafragma; en `TappedHorn` es la
    /// impedancia de cuello; en `OffsetHorn` es la impedancia en la toma. En
    /// `Bandpass` no se calcula y queda a 0.
    pub za_real: f64,
    /// Parte imaginaria de la impedancia acústica de referencia (Pa·s/m³).
    pub za_imag: f64,
    /// Velocidad del aire en el puerto, si la topología tiene puerto.
    pub port_velocity: Option<f64>,
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
    /// Respuesta a una frecuencia. Devuelve un punto neutro si la entrada no
    /// es resoluble; use [`Design::try_response_at`] para distinguir el error.
    pub fn response_at(&self, freq_hz: f64) -> DesignPoint {
        match self {
            Design::System(sys) => from_system(sys.response_at(freq_hz)),
            Design::TappedHorn(th) => {
                from_tapped(th.response_at(freq_hz), th.neck_impedance(freq_hz))
            }
            Design::OffsetHorn(od) => {
                from_offset(od.response_at(freq_hz), od.horn_impedance(freq_hz))
            }
            Design::Bandpass(bp) => from_bandpass(bp.response_at(freq_hz)),
        }
    }

    /// Respuesta a una frecuencia, con error explícito si la topología no es
    /// resoluble.
    pub fn try_response_at(&self, freq_hz: f64) -> Result<DesignPoint, SolveError> {
        match self {
            Design::System(sys) => sys.try_response_at(freq_hz).map(from_system),
            Design::TappedHorn(th) => {
                let p = th.try_response_at(freq_hz)?;
                Ok(from_tapped(p, th.neck_impedance(freq_hz)))
            }
            Design::OffsetHorn(od) => {
                let p = od.try_response_at(freq_hz)?;
                Ok(from_offset(p, od.horn_impedance(freq_hz)))
            }
            Design::Bandpass(bp) => bp.try_response_at(freq_hz).map(from_bandpass),
        }
    }

    /// Barrido de respuesta con fase desenrollada y retardo de grupo.
    pub fn response(&self, freqs: &[f64]) -> Vec<DesignPoint> {
        let mut pts: Vec<DesignPoint> = freqs
            .iter()
            .copied()
            .filter(|f| *f > 0.0)
            .map(|f| self.response_at(f))
            .collect();
        if pts.len() < 2 {
            return pts;
        }

        for i in 1..pts.len() {
            while pts[i].phase_deg - pts[i - 1].phase_deg > 180.0 {
                pts[i].phase_deg -= 360.0;
            }
            while pts[i].phase_deg - pts[i - 1].phase_deg < -180.0 {
                pts[i].phase_deg += 360.0;
            }
        }
        for i in 1..pts.len() {
            let df = pts[i].freq_hz - pts[i - 1].freq_hz;
            if df.abs() > 1.0e-12 {
                let dphi = (pts[i].phase_deg - pts[i - 1].phase_deg).to_radians();
                pts[i].group_delay_ms = -(dphi / (std::f64::consts::TAU * df)) * 1000.0;
            }
        }
        pts[0].group_delay_ms = pts[1].group_delay_ms;
        pts
    }

    /// Barrido logarítmico entre `fmin` y `fmax` con `points` muestras.
    pub fn sweep_log(&self, fmin: f64, fmax: f64, points: usize) -> Vec<DesignPoint> {
        let n = points.max(2);
        let freqs: Vec<f64> = (0..n)
            .map(|i| {
                let t = i as f64 / (n - 1) as f64;
                fmin * (fmax / fmin).powf(t)
            })
            .collect();
        self.response(&freqs)
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
}
