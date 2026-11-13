//! Impedancia de radiación de un pistón circular.
//!
//! Para un pistón de radio `a` en pantalla infinita (2π estereorradianes):
//!
//! ```text
//! Zm = rho·c·S·(R1(x) + j·X1(x)),   x = 2·k·a
//! R1 = 1 − 2·J1(x)/x
//! X1 = 2·H1(x)/x
//! ```
//!
//! donde `J1` es la función de Bessel de primera especie y `H1` la función de
//! Struve. La impedancia acústica es `Za = Zm/S² = (rho·c/S)·(R1 + jX1)`.
//!
//! La carga implementada es SIEMPRE la del pistón en pantalla infinita (2π).
//! `solid_angle` se conserva por compatibilidad, pero NO modifica esta carga.
//! No es la solución de un pistón sin pantalla en 4π ni de imágenes en esquinas.
//! Los solvers usan Ω para estimar presión/potencia; ese escalado de presentación
//! no introduce automáticamente carga mecánica ni impedancias mutuas.

use crate::air::Air;
use num_complex::Complex64;

/// Función de Struve de orden 1.
///
/// Serie de potencias para `|x| < 25` y desarrollo asintótico para `|x| >= 25`
/// (DLMF 11.6.1). Precisión típica mejor que 1e-7 en el rango audible.
pub fn struve_h1(x: f64) -> f64 {
    let ax = x.abs();
    if ax == 0.0 {
        return 0.0;
    }
    if ax < 25.0 {
        // H1(x) = sum_{m>=0} (-1)^m (x/2)^(2m+2) / (Gamma(m+3/2) Gamma(m+5/2))
        let half_sq = 0.25 * x * x;
        let mut term = 1.0_f64;
        let mut sum = 1.0_f64;
        for m in 0..600 {
            let mf = m as f64;
            term *= -half_sq / ((mf + 1.5) * (mf + 2.5));
            sum += term;
            if m as f64 > 2.0 * ax && term.abs() < 1.0e-17 * sum.abs() {
                break;
            }
        }
        // Primer término: 2x²/(3π)
        (2.0 * x * x / (3.0 * std::f64::consts::PI)) * sum
    } else {
        // H1(x) ~ Y1(x) + (2/π) * sum_k c_k,
        // c_0 = 1,  c_{k+1}/c_k = (1 - 4k²)/x²
        // La serie asintótica es divergente: se trunca en el término mínimo.
        let y1 = libm::y1(ax);
        let mut c = 1.0_f64;
        let mut sum = 1.0_f64;
        let mut prev = 1.0_f64;
        for k in 0..200 {
            let kf = k as f64;
            c *= (1.0 - 4.0 * kf * kf) / (x * x);
            let ac = c.abs();
            if ac > prev {
                break;
            }
            sum += c;
            prev = ac;
        }
        y1 + (2.0 / std::f64::consts::PI) * sum
    }
}

/// Impedancia acústica de radiación de un pistón de área `area` (m²) a la
/// frecuencia `freq_hz`, radiando en un ángulo sólido `solid_angle`
/// (estereorradianes; 2π = pantalla infinita, 4π = espacio libre).
pub fn piston_radiation_impedance(
    air: &Air,
    area: f64,
    freq_hz: f64,
    solid_angle: f64,
) -> Complex64 {
    if freq_hz <= 0.0 || area <= 0.0 || solid_angle <= 0.0 {
        return Complex64::new(0.0, 0.0);
    }
    let a = (area / std::f64::consts::PI).sqrt();
    let k = air.wave_number(freq_hz);
    let x = 2.0 * k * a;

    let r1 = if x < 1.0e-3 {
        // Serie: evita cancelación catastrófica en 1-2J1(x)/x.
        let x2 = x * x;
        x2 * (0.125 + x2 * (-1.0 / 192.0 + x2 / 9216.0))
    } else {
        1.0 - 2.0 * libm::j1(x) / x
    };
    let x1 = if x < 1.0e-3 {
        // x=2ka, por lo que X1~4x/(3π), no 8x/(3π).
        let x2 = x * x;
        4.0 * x / (3.0 * std::f64::consts::PI) * (1.0 + x2 * (-1.0 / 15.0 + x2 / 525.0))
    } else {
        2.0 * struve_h1(x) / x
    };

    // La impedancia de un pistón en pantalla infinita es independiente del
    // ángulo de observación. El ángulo sólido entra en la presión lejana
    // (p² = P·rho·c/Ω), no en la carga acústica de la boca.
    let _ = solid_angle;
    Complex64::new(air.rho * air.c / area, 0.0) * Complex64::new(r1, x1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn struve_known_values() {
        // H1(1) = 0.19846..., H1(2) = 0.64676..., H1(5) = 0.80781...
        assert!((struve_h1(1.0) - 0.198_46).abs() < 2.0e-4);
        assert!((struve_h1(2.0) - 0.646_76).abs() < 2.0e-4);
        assert!((struve_h1(5.0) - 0.807_81).abs() < 2.0e-4);
    }

    #[test]
    fn series_and_asymptotic_match() {
        let a = struve_h1(24.999_999);
        let b = struve_h1(25.000_001);
        assert!((a - b).abs() < 1.0e-6, "{a} vs {b}");
    }

    #[test]
    fn low_frequency_asymptotes() {
        let air = Air::default();
        let area = 350.0e-4;
        let f = 1.0;
        let z = piston_radiation_impedance(&air, area, f, std::f64::consts::TAU);
        let a = (area / std::f64::consts::PI).sqrt();
        let ka = air.wave_number(f) * a;
        let ra_expected = air.rho * air.c / area * ka * ka / 2.0;
        let xa_expected = air.rho * air.c / area * 8.0 * ka / (3.0 * std::f64::consts::PI);
        assert!((z.re - ra_expected).abs() / ra_expected < 1.0e-3);
        assert!((z.im - xa_expected).abs() / xa_expected < 1.0e-3);
    }
}
