//! Postprocesado de fase y derivada en frecuencias no uniformes.
//! Una cancelación exacta no tiene fase ni retardo de grupo definidos.
use crate::error::SolveError;

/// Fase desenrollada y retardo opcionales; `None` indica dato indefinido.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhasePoint {
    pub phase_deg: Option<f64>,
    pub group_delay_ms: Option<f64>,
}

/// Calcula -dφ/dω con interpolación cuadrática local (secante para dos puntos).
/// Las entradas deben ser finitas, positivas y estrictamente crecientes.
/// `None` corta el desenrollado y evita derivar a través de cancelaciones.
/// Un salto entre muestras mayor de 180° requiere un muestreo más denso;
/// no es posible reconstruir vueltas perdidas a partir de fase envuelta.
pub fn phase_trace(freqs: &[f64], phases: &[Option<f64>]) -> Result<Vec<PhasePoint>, SolveError> {
    if freqs.len() != phases.len() {
        return Err(SolveError::InvalidParameter("phase_trace.length"));
    }
    for (i, &f) in freqs.iter().enumerate() {
        crate::error::check_frequency(f)?;
        if i > 0 && f <= freqs[i - 1] {
            return Err(SolveError::InvalidParameter("frequencies must increase"));
        }
        if phases[i].is_some_and(|p| !p.is_finite()) {
            return Err(SolveError::InvalidResponse("phase_deg"));
        }
    }
    let mut out = vec![
        PhasePoint {
            phase_deg: None,
            group_delay_ms: None
        };
        freqs.len()
    ];
    let mut start = 0;
    while start < freqs.len() {
        if phases[start].is_none() {
            start += 1;
            continue;
        }
        let mut end = start;
        while end < freqs.len() && phases[end].is_some() {
            let p = phases[end].unwrap();
            let unwrapped = if end > start {
                let prev = out[end - 1].phase_deg.unwrap();
                prev + (p - prev + 180.).rem_euclid(360.) - 180.
            } else {
                p
            };
            out[end].phase_deg = Some(unwrapped);
            end += 1;
        }
        if end - start >= 2 {
            let slope = |a: usize, b: usize| {
                (out[b].phase_deg.unwrap() - out[a].phase_deg.unwrap()) / (freqs[b] - freqs[a])
            };
            let derivatives: Vec<f64> = (start..end)
                .map(|i| {
                    if end - start == 2 {
                        return slope(start, start + 1);
                    }
                    let j = i.max(start + 1).min(end - 2);
                    let h0 = freqs[j] - freqs[j - 1];
                    let h1 = freqs[j + 1] - freqs[j];
                    let s0 = slope(j - 1, j);
                    let s1 = slope(j, j + 1);
                    // Derivada de la parábola que pasa por las tres muestras.
                    s0 + (s1 - s0) / (h0 + h1) * (2. * freqs[i] - freqs[j - 1] - freqs[j])
                })
                .collect();
            for (i, d) in (start..end).zip(derivatives) {
                out[i].group_delay_ms = Some(-d / 360. * 1000.);
            }
        }
        start = end;
    }
    Ok(out)
}

/// Adaptador legado: ordena índices, deduplica frecuencias para la derivada
/// y restaura el orden del llamador. NaN hace explícita la fase/derivada
/// indefinida; una sola muestra conserva el 0 legado (no calculado).
pub(crate) fn finalize(freqs: &[f64], phases: &mut [f64], spl: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..freqs.len()).collect();
    order.sort_by(|&a, &b| freqs[a].total_cmp(&freqs[b]));
    let mut unique = order.clone();
    unique.dedup_by(|a, b| freqs[*a] == freqs[*b]);
    let f: Vec<f64> = unique.iter().map(|&i| freqs[i]).collect();
    let p: Vec<Option<f64>> = unique
        .iter()
        .map(|&i| {
            if spl[i].is_finite() {
                Some(phases[i])
            } else {
                None
            }
        })
        .collect();
    let trace = phase_trace(&f, &p).unwrap_or_default();
    let mut delays = vec![f64::NAN; freqs.len()];
    for i in order {
        if let Ok(j) = f.binary_search_by(|v| v.total_cmp(&freqs[i])) {
            phases[i] = trace[j].phase_deg.unwrap_or(f64::NAN);
            delays[i] = trace[j]
                .group_delay_ms
                .unwrap_or(if f.len() == 1 && p[j].is_some() {
                    0.
                } else {
                    f64::NAN
                });
        }
    }
    delays
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nonuniform_quadratic_phase_has_exact_derivative() {
        let f = [1., 2., 4., 7.];
        let phases = f.map(|v| Some(-v * v));
        let trace = phase_trace(&f, &phases).unwrap();
        for (p, v) in trace.iter().zip(f) {
            assert!((p.group_delay_ms.unwrap() - 2. * v / 360. * 1000.).abs() < 1e-12);
        }
    }
    #[test]
    fn invalid_order_and_cancellation_are_explicit() {
        assert!(phase_trace(&[2., 1.], &[Some(0.), Some(0.)]).is_err());
        assert!(phase_trace(&[1., 1.], &[Some(0.), Some(0.)]).is_err());
        let t = phase_trace(&[1., 2., 3.], &[Some(0.), None, Some(180.)]).unwrap();
        assert!(t.iter().all(|p| p.group_delay_ms.is_none()));
    }
    #[test]
    fn unwraps_across_branch_cut() {
        let t = phase_trace(&[1., 2., 3.], &[Some(170.), Some(-170.), Some(-150.)]).unwrap();
        assert_eq!(t[2].phase_deg, Some(210.));
    }
}
