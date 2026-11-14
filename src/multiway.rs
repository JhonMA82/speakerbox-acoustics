//! Suma coherente de presiones complejas de varias vías.
//!
//! Implementa el contrato de
//! [`docs/audits/MULTIWAY_PRESSURE_CONTRACT.md`](../../docs/audits/MULTIWAY_PRESSURE_CONTRACT.md):
//!
//! ```text
//! p_total(f) = Σ s_i · g_i · p_i(f) · e^(−j·2π·f·τ_i)
//! ```
//!
//! Cada `p_i` es la presión compleja de un [`Design`] normalizada a
//! [`MULTIWAY_REFERENCE_VOLTS`] (2.83 V RMS, 1 W en 8 Ω), con la misma
//! referencia de aire. `s_i` es la polaridad declarada, `g_i` una ganancia
//! lineal y `τ_i` un retardo en segundos (positivo = llega más tarde).
//!
//! La suma es una **presión coherente**, no una suma de potencias. Presupone
//! fuentes compactas, misma distancia, misma referencia temporal y un único
//! punto de observación. No modela off-axis, directividad, sala ni el trayecto
//! de 1 m.
//!
//! Si una vía no tiene observación definida (boca adaptada), el total es
//! `None`: nunca se devuelve una suma parcial.

use crate::design::Design;
use crate::error::{check_frequency, SolveError};
use crate::Air;
use num_complex::Complex64;
use std::f64::consts::TAU;

/// Tensión de referencia de la suma: 2.83 V RMS (1 W en 8 Ω).
pub const MULTIWAY_REFERENCE_VOLTS: f64 = 2.83;

/// Una vía de la suma.
#[derive(Clone, Copy, Debug)]
pub struct MultiwayChannel<'a> {
    /// Diseño de la vía.
    pub design: &'a Design,
    /// Polaridad declarada: `+1` o `-1`.
    pub polarity: i8,
    /// Ganancia lineal no negativa (atenuación pasiva o ajuste de DSP).
    pub gain: f64,
    /// Retardo en segundos; positivo significa que esta vía llega más tarde.
    pub delay_s: f64,
}

impl<'a> MultiwayChannel<'a> {
    /// Vía con polaridad positiva, ganancia unidad y sin retardo.
    #[must_use]
    pub fn new(design: &'a Design) -> Self {
        Self {
            design,
            polarity: 1,
            gain: 1.0,
            delay_s: 0.0,
        }
    }

    /// Declara la polaridad (`+1` o `-1`).
    #[must_use]
    pub fn with_polarity(mut self, polarity: i8) -> Self {
        self.polarity = polarity;
        self
    }

    /// Declara la ganancia lineal.
    #[must_use]
    pub fn with_gain(mut self, gain: f64) -> Self {
        self.gain = gain;
        self
    }

    /// Declara el retardo en segundos.
    #[must_use]
    pub fn with_delay_s(mut self, delay_s: f64) -> Self {
        self.delay_s = delay_s;
        self
    }
}

/// Resultado de la suma en una frecuencia.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MultiwayObservation {
    /// Presión compleja a 1 m, en pascales. `None` si falta una vía.
    pub pressure_pa: Option<Complex64>,
    /// SPL re 20 µPa. `-∞` si la cancelación es exacta. `None` si falta una vía.
    pub spl_db: Option<f64>,
    /// Fase de la presión total, en grados y envuelta a `(-180, 180]`.
    /// `None` en cancelación exacta o si falta una vía.
    pub phase_deg: Option<f64>,
}

/// Suma las vías a `freq_hz`, normalizando cada una a 2.83 V RMS.
///
/// # Errores
///
/// - [`SolveError::InvalidFrequency`] si la frecuencia no es finita o no es
///   positiva.
/// - [`SolveError::InvalidParameter`] si no hay vías, la polaridad no es ±1,
///   la ganancia o el retardo son inválidos, `eg` es cero, o las vías no
///   comparten el mismo aire de referencia.
/// - Cualquier error de [`Design::try_observation_at`].
/// - [`SolveError::NonFiniteResponse`] si la suma no es finita.
pub fn sum_at(
    channels: &[MultiwayChannel<'_>],
    freq_hz: f64,
) -> Result<MultiwayObservation, SolveError> {
    check_frequency(freq_hz)?;
    if channels.is_empty() {
        return Err(SolveError::InvalidParameter("channels"));
    }

    let mut total = Complex64::new(0.0, 0.0);
    let mut reference: Option<(f64, f64, f64)> = None;
    for channel in channels {
        if channel.polarity != 1 && channel.polarity != -1 {
            return Err(SolveError::InvalidParameter("polarity"));
        }
        if !channel.gain.is_finite() || channel.gain < 0.0 {
            return Err(SolveError::InvalidParameter("gain"));
        }
        if !channel.delay_s.is_finite() || channel.delay_s < 0.0 {
            return Err(SolveError::InvalidParameter("delay_s"));
        }
        let (eg, air) = reference_of(channel.design);
        if !eg.is_finite() || eg == 0.0 {
            return Err(SolveError::InvalidParameter("eg"));
        }
        if !air.p_ref.is_finite()
            || air.p_ref <= 0.0
            || !air.rho.is_finite()
            || air.rho <= 0.0
            || !air.c.is_finite()
            || air.c <= 0.0
        {
            return Err(SolveError::InvalidParameter("air"));
        }
        let signature = (air.p_ref, air.rho, air.c);
        match reference {
            None => reference = Some(signature),
            Some(expected) if expected != signature => {
                return Err(SolveError::InvalidParameter("air"));
            }
            Some(_) => {}
        }

        let observation = channel.design.try_observation_at(freq_hz)?;
        let Some(pressure) = observation.pressure_estimate_pa else {
            return Ok(MultiwayObservation {
                pressure_pa: None,
                spl_db: None,
                phase_deg: None,
            });
        };
        let scaled = pressure * (MULTIWAY_REFERENCE_VOLTS / eg.abs());
        let delay = Complex64::from_polar(1.0, -TAU * freq_hz * channel.delay_s);
        let weight = f64::from(channel.polarity) * channel.gain;
        total += scaled * delay * weight;
    }

    if !total.re.is_finite() || !total.im.is_finite() {
        return Err(SolveError::NonFiniteResponse);
    }
    let (p_ref, _, _) = reference.ok_or(SolveError::InvalidParameter("channels"))?;
    let magnitude = total.norm();
    let (spl_db, phase_deg) = if magnitude > 0.0 {
        (
            Some(20.0 * (magnitude / p_ref).log10()),
            Some(total.arg().to_degrees()),
        )
    } else {
        (Some(f64::NEG_INFINITY), None)
    };
    Ok(MultiwayObservation {
        pressure_pa: Some(total),
        spl_db,
        phase_deg,
    })
}

/// Suma las vías en cada frecuencia de `freq_hz`.
///
/// # Errores
///
/// El primer error de [`sum_at`], con su frecuencia.
pub fn sum_sweep(
    channels: &[MultiwayChannel<'_>],
    freq_hz: &[f64],
) -> Result<Vec<MultiwayObservation>, SolveError> {
    freq_hz.iter().map(|freq| sum_at(channels, *freq)).collect()
}

fn reference_of(design: &Design) -> (f64, &Air) {
    match design {
        Design::System(system) => (system.eg, &system.air),
        Design::TappedHorn(system) => (system.eg, &system.air),
        Design::OffsetHorn(system) => (system.eg, &system.air),
        Design::Bandpass(system) => (system.eg, &system.air),
        Design::TransmissionLine(system) => (system.eg, &system.air),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Air, Chamber, Driver, System};

    fn sealed_system(eg: f64) -> Design {
        let driver = Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1);
        let mut system = System::new(Air::default(), driver);
        system.rear_chamber = Some(Chamber::new(20.0e-3));
        system.eg = eg;
        Design::System(system)
    }

    #[test]
    fn two_identical_vias_add_six_db() {
        let design = sealed_system(MULTIWAY_REFERENCE_VOLTS);
        let one = sum_at(&[MultiwayChannel::new(&design)], 100.0).unwrap();
        let two = sum_at(
            &[MultiwayChannel::new(&design), MultiwayChannel::new(&design)],
            100.0,
        )
        .unwrap();
        let delta = two.spl_db.unwrap() - one.spl_db.unwrap();
        assert!((delta - 20.0 * 2.0_f64.log10()).abs() < 1e-12);
    }

    #[test]
    fn opposite_polarity_cancels_exactly() {
        let design = sealed_system(MULTIWAY_REFERENCE_VOLTS);
        let observation = sum_at(
            &[
                MultiwayChannel::new(&design),
                MultiwayChannel::new(&design).with_polarity(-1),
            ],
            100.0,
        )
        .unwrap();
        assert_eq!(observation.pressure_pa.unwrap().norm(), 0.0);
        assert_eq!(observation.spl_db, Some(f64::NEG_INFINITY));
        assert_eq!(observation.phase_deg, None);
    }

    #[test]
    fn half_period_delay_cancels() {
        let design = sealed_system(MULTIWAY_REFERENCE_VOLTS);
        let freq = 100.0;
        let observation = sum_at(
            &[
                MultiwayChannel::new(&design),
                MultiwayChannel::new(&design).with_delay_s(1.0 / (2.0 * freq)),
            ],
            freq,
        )
        .unwrap();
        let single = sum_at(&[MultiwayChannel::new(&design)], freq).unwrap();
        let ratio = observation.pressure_pa.unwrap().norm() / single.pressure_pa.unwrap().norm();
        assert!(ratio < 1e-12, "ratio={ratio}");
    }

    #[test]
    fn gain_scales_the_pressure_linearly() {
        let design = sealed_system(MULTIWAY_REFERENCE_VOLTS);
        let single = sum_at(&[MultiwayChannel::new(&design)], 250.0).unwrap();
        let half = sum_at(&[MultiwayChannel::new(&design).with_gain(0.5)], 250.0).unwrap();
        let ratio = half.pressure_pa.unwrap().norm() / single.pressure_pa.unwrap().norm();
        assert!((ratio - 0.5).abs() < 1e-12);
    }

    #[test]
    fn eg_is_normalized_to_the_reference_voltage() {
        let at_two_point_eight_three = sealed_system(2.83);
        let at_one_volt = sealed_system(1.0);
        let a = sum_at(&[MultiwayChannel::new(&at_two_point_eight_three)], 80.0).unwrap();
        let b = sum_at(&[MultiwayChannel::new(&at_one_volt)], 80.0).unwrap();
        let difference = (a.pressure_pa.unwrap() - b.pressure_pa.unwrap()).norm();
        let scale = a.pressure_pa.unwrap().norm();
        assert!(
            difference / scale < 1e-12,
            "diferencia relativa={}",
            difference / scale
        );
    }

    #[test]
    fn mixed_air_references_are_rejected() {
        let mut other = sealed_system(MULTIWAY_REFERENCE_VOLTS);
        if let Design::System(ref mut system) = other {
            system.air.p_ref = 40.0e-6;
        }
        let design = sealed_system(MULTIWAY_REFERENCE_VOLTS);
        assert_eq!(
            sum_at(
                &[MultiwayChannel::new(&design), MultiwayChannel::new(&other)],
                100.0
            )
            .unwrap_err(),
            SolveError::InvalidParameter("air")
        );
    }

    #[test]
    fn invalid_channels_are_rejected() {
        let design = sealed_system(MULTIWAY_REFERENCE_VOLTS);
        assert!(sum_at(&[], 100.0).is_err());
        assert!(sum_at(&[MultiwayChannel::new(&design).with_polarity(0)], 100.0).is_err());
        assert!(sum_at(&[MultiwayChannel::new(&design).with_gain(-1.0)], 100.0).is_err());
        assert!(sum_at(&[MultiwayChannel::new(&design).with_gain(f64::NAN)], 100.0).is_err());
        assert!(sum_at(&[MultiwayChannel::new(&design).with_delay_s(-0.1)], 100.0).is_err());
        let zero_eg = sealed_system(0.0);
        assert!(sum_at(&[MultiwayChannel::new(&zero_eg)], 100.0).is_err());
    }

    #[test]
    fn sweep_matches_point_by_point() {
        let design = sealed_system(MULTIWAY_REFERENCE_VOLTS);
        let channels = [MultiwayChannel::new(&design)];
        let freqs = [50.0, 100.0, 200.0];
        let sweep = sum_sweep(&channels, &freqs).unwrap();
        for (observation, freq) in sweep.iter().zip(freqs) {
            assert_eq!(*observation, sum_at(&channels, freq).unwrap());
        }
    }
}
