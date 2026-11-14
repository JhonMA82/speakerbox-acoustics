//! Experimentos físicos para magnitudes identificables.
//!
//! Solo se devuelve lo que los datos permiten: masa móvil y compliancia en el
//! método de masa añadida; volumen equivalente `Vas` en el método de caja
//! sellada. No se deduce `Bl`, `Sd` ni `Cms` cuando el experimento no los
//! determina.

use crate::error::{MeasureError, MeasureResult};
use std::f64::consts::TAU;

/// Resultado del método de masa añadida.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AddedMassEstimate {
    /// Masa móvil total, en kg.
    pub mms_kg: f64,
    /// Compliancia mecánica, en m/N.
    pub cms_m_per_n: f64,
    /// Cociente `Fs_cargada / Fs_libre`.
    pub fs_ratio: f64,
}

/// Resultado del método de caja sellada.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SealedBoxEstimate {
    /// Volumen equivalente `Vas`, en m³.
    pub vas_m3: f64,
    /// Cociente `(Fs_caja / Fs_libre)² − 1`.
    pub ratio: f64,
}

/// Masa móvil y compliancia a partir de la bajada de `Fs` con una masa
/// conocida.
///
/// ```text
/// Mms = M_añadida · Fs_cargada² / (Fs_libre² − Fs_cargada²)
/// Cms = 1 / ((2π·Fs_libre)² · Mms)
/// ```
///
/// # Errores
///
/// - [`MeasureError::InvalidFrequency`] para frecuencias no finitas o no
///   positivas.
/// - [`MeasureError::InvalidExperiment`] si la masa no es positiva o si la
///   frecuencia cargada no baja de la libre.
pub fn added_mass(
    free_hz: f64,
    loaded_hz: f64,
    added_mass_kg: f64,
) -> MeasureResult<AddedMassEstimate> {
    check_frequency(free_hz)?;
    check_frequency(loaded_hz)?;
    if !added_mass_kg.is_finite() || added_mass_kg <= 0.0 {
        return Err(MeasureError::InvalidExperiment(
            "la masa añadida debe ser finita y > 0 kg",
        ));
    }
    if loaded_hz >= free_hz {
        return Err(MeasureError::InvalidExperiment(
            "la masa añadida debe reducir Fs (Fs_cargada < Fs_libre)",
        ));
    }
    let free_sq = free_hz * free_hz;
    let loaded_sq = loaded_hz * loaded_hz;
    let mms_kg = added_mass_kg * loaded_sq / (free_sq - loaded_sq);
    let omega = TAU * free_hz;
    let cms_m_per_n = 1.0 / (omega * omega * mms_kg);
    if !mms_kg.is_finite() || mms_kg <= 0.0 || !cms_m_per_n.is_finite() {
        return Err(MeasureError::NonFinite("masa o compliancia estimada"));
    }
    Ok(AddedMassEstimate {
        mms_kg,
        cms_m_per_n,
        fs_ratio: loaded_hz / free_hz,
    })
}

/// `Vas` a partir del desplazamiento de `Fs` al montar el altavoz en un
/// volumen conocido.
///
/// ```text
/// Vas = V_caja · ((Fs_caja / Fs_libre)² − 1)
/// ```
///
/// El resultado no necesita `Sd` ni la densidad del aire: es una relación de
/// volúmenes medida.
///
/// # Errores
///
/// - [`MeasureError::InvalidFrequency`] para frecuencias inválidas.
/// - [`MeasureError::InvalidExperiment`] si el volumen no es positivo o si la
///   caja no sube `Fs`.
pub fn sealed_box(
    free_hz: f64,
    box_hz: f64,
    box_volume_m3: f64,
) -> MeasureResult<SealedBoxEstimate> {
    check_frequency(free_hz)?;
    check_frequency(box_hz)?;
    if !box_volume_m3.is_finite() || box_volume_m3 <= 0.0 {
        return Err(MeasureError::InvalidExperiment(
            "el volumen de la caja debe ser finito y > 0 m³",
        ));
    }
    if box_hz <= free_hz {
        return Err(MeasureError::InvalidExperiment(
            "la caja sellada debe subir Fs (Fs_caja > Fs_libre)",
        ));
    }
    let ratio = (box_hz / free_hz).powi(2) - 1.0;
    let vas_m3 = box_volume_m3 * ratio;
    if !vas_m3.is_finite() || vas_m3 <= 0.0 {
        return Err(MeasureError::NonFinite("Vas estimado"));
    }
    Ok(SealedBoxEstimate { vas_m3, ratio })
}

fn check_frequency(freq_hz: f64) -> MeasureResult<()> {
    if !freq_hz.is_finite() || freq_hz <= 0.0 {
        return Err(MeasureError::InvalidFrequency(freq_hz));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn added_mass_follows_the_closed_form() {
        let estimate = added_mass(40.0, 30.0, 0.05).unwrap();
        let expected_mms = 0.05 * 900.0 / (1600.0 - 900.0);
        assert!((estimate.mms_kg - expected_mms).abs() < 1e-12);
        let expected_cms = 1.0 / ((TAU * 40.0).powi(2) * expected_mms);
        assert!((estimate.cms_m_per_n - expected_cms).abs() < 1e-15);
        assert!((estimate.fs_ratio - 0.75).abs() < 1e-15);
    }

    #[test]
    fn sealed_box_returns_vas_without_sd() {
        let estimate = sealed_box(40.0, 60.0, 0.01).unwrap();
        assert!((estimate.vas_m3 - 0.0125).abs() < 1e-12);
        assert!((estimate.ratio - 1.25).abs() < 1e-12);
    }

    #[test]
    fn incoherent_experiments_are_rejected() {
        assert!(added_mass(40.0, 40.0, 0.05).is_err());
        assert!(added_mass(40.0, 50.0, 0.05).is_err());
        assert!(added_mass(40.0, 30.0, 0.0).is_err());
        assert!(added_mass(f64::NAN, 30.0, 0.05).is_err());
        assert!(sealed_box(40.0, 40.0, 0.01).is_err());
        assert!(sealed_box(40.0, 30.0, 0.01).is_err());
        assert!(sealed_box(40.0, 60.0, 0.0).is_err());
        assert!(sealed_box(40.0, 60.0, f64::INFINITY).is_err());
    }
}
