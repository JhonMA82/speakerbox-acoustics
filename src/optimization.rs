//! Optimización acotada desacoplada; los candidatos no mutan el diseño original.
use crate::error::SolveError;
#[derive(Clone, Copy, Debug)]
pub struct Bound {
    pub min: f64,
    pub max: f64,
    pub locked: Option<f64>,
}
#[derive(Clone, Copy, Debug)]
pub enum Algorithm {
    NelderMead,
    DifferentialEvolution,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    Converged,
    Budget,
    Cancelled,
    NoValidCandidate,
}
#[derive(Clone, Debug)]
pub struct Candidate {
    pub parameters: Vec<f64>,
    pub cost: f64,
}
#[derive(Clone, Debug)]
pub struct SearchResult {
    pub best: Option<Candidate>,
    pub evaluations: usize,
    pub iterations: usize,
    pub stop: Stop,
}
#[derive(Clone, Copy, Debug)]
pub struct SearchConfig {
    pub algorithm: Algorithm,
    pub max_evaluations: usize,
    pub max_iterations: usize,
    pub tolerance: f64,
    pub seed: u64,
    pub population: usize,
}
struct Evaluator<'a, F, C> {
    bounds: &'a [Bound],
    f: F,
    cancel: C,
    cache: Vec<(Vec<f64>, f64)>,
    best: Option<Candidate>,
    budget: usize,
    cancelled: bool,
}
impl<F: FnMut(&[f64]) -> Option<f64>, C: FnMut() -> bool> Evaluator<'_, F, C> {
    fn value(&mut self, x: &[f64]) -> f64 {
        if (self.cancel)() {
            self.cancelled = true;
            return f64::INFINITY;
        }
        if let Some((_, v)) = self.cache.iter().find(|(k, _)| k == x) {
            return *v;
        }
        if self.cache.len() >= self.budget {
            return f64::INFINITY;
        }
        let actual: Vec<_> = x
            .iter()
            .zip(self.bounds)
            .map(|(&x, b)| b.locked.unwrap_or(b.min + x * (b.max - b.min)))
            .collect();
        let v = (self.f)(&actual)
            .filter(|v| v.is_finite())
            .unwrap_or(f64::INFINITY);
        if v.is_finite() && self.best.as_ref().map_or(true, |b| v < b.cost) {
            self.best = Some(Candidate {
                parameters: actual,
                cost: v,
            });
        }
        self.cache.push((x.to_vec(), v));
        v
    }
    fn exhausted(&self) -> bool {
        self.cancelled || self.cache.len() >= self.budget
    }
}
fn random(seed: &mut u64) -> f64 {
    *seed = seed.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *seed;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((z ^ (z >> 31)) >> 11) as f64 / ((1u64 << 53) as f64)
}
/// Objetivos inválidos devuelven None. La cancelación se consulta antes de cada
/// evaluación (incluidas consultas cacheadas). Tolerancia en coordenadas normalizadas.
pub fn minimize<F: FnMut(&[f64]) -> Option<f64>, C: FnMut() -> bool>(
    bounds: &[Bound],
    start: &[f64],
    cfg: SearchConfig,
    f: F,
    cancel: C,
) -> Result<SearchResult, SolveError> {
    if bounds.is_empty()
        || bounds.len() != start.len()
        || cfg.max_evaluations == 0
        || cfg.max_iterations == 0
        || !cfg.tolerance.is_finite()
        || cfg.tolerance <= 0.0
    {
        return Err(SolveError::InvalidParameter("optimizer configuration"));
    }
    for (b, &x) in bounds.iter().zip(start) {
        if !b.min.is_finite()
            || !b.max.is_finite()
            || !(b.max - b.min).is_finite()
            || b.max < b.min
            || !x.is_finite()
            || x < b.min
            || x > b.max
            || b.locked
                .is_some_and(|v| !v.is_finite() || v < b.min || v > b.max)
        {
            return Err(SolveError::InvalidParameter("optimizer bounds"));
        }
    }
    let active: Vec<_> = bounds
        .iter()
        .enumerate()
        .filter(|(_, b)| b.locked.is_none() && b.max > b.min)
        .map(|(i, _)| i)
        .collect();
    let x: Vec<_> = start
        .iter()
        .zip(bounds)
        .map(|(&v, b)| {
            if b.max > b.min {
                (v - b.min) / (b.max - b.min)
            } else {
                0.
            }
        })
        .collect();
    let mut e = Evaluator {
        bounds,
        f,
        cancel,
        cache: Vec::new(),
        best: None,
        budget: cfg.max_evaluations,
        cancelled: false,
    };
    let mut converged = false;
    let mut iterations = 0;
    if active.is_empty() {
        e.value(&x);
        converged = true;
    } else {
        match cfg.algorithm {
            Algorithm::NelderMead => {
                let mut simplex = vec![(x.clone(), e.value(&x))];
                for &i in &active {
                    let mut p = x.clone();
                    p[i] = if p[i] < 0.9 { p[i] + 0.1 } else { p[i] - 0.1 };
                    let v = e.value(&p);
                    simplex.push((p, v));
                }
                for k in 0..cfg.max_iterations {
                    iterations = k + 1;
                    if e.exhausted() {
                        break;
                    }
                    simplex.sort_by(|a, b| a.1.total_cmp(&b.1));
                    let n = active.len();
                    let diameter = simplex
                        .iter()
                        .skip(1)
                        .flat_map(|p| active.iter().map(|&i| (p.0[i] - simplex[0].0[i]).abs()))
                        .fold(0., f64::max);
                    if diameter <= cfg.tolerance && simplex[0].1.is_finite() {
                        converged = true;
                        break;
                    }
                    let mut centroid = x.clone();
                    for &i in &active {
                        centroid[i] = simplex[..n].iter().map(|p| p.0[i]).sum::<f64>() / n as f64;
                    }
                    let proposal = |factor: f64| {
                        let mut p = centroid.clone();
                        for &i in &active {
                            p[i] = (centroid[i] + factor * (centroid[i] - simplex[n].0[i]))
                                .clamp(0., 1.);
                        }
                        p
                    };
                    let reflected = proposal(1.);
                    let rv = e.value(&reflected);
                    if rv < simplex[0].1 {
                        let expanded = proposal(2.);
                        let ev = e.value(&expanded);
                        simplex[n] = if ev < rv {
                            (expanded, ev)
                        } else {
                            (reflected, rv)
                        };
                    } else if rv < simplex[n - 1].1 {
                        simplex[n] = (reflected, rv);
                    } else {
                        let contracted = proposal(-0.5);
                        let cv = e.value(&contracted);
                        if cv < simplex[n].1 {
                            simplex[n] = (contracted, cv);
                        } else {
                            let best = simplex[0].0.clone();
                            for p in &mut simplex[1..] {
                                for &i in &active {
                                    p.0[i] = (p.0[i] + best[i]) / 2.;
                                }
                                p.1 = e.value(&p.0);
                            }
                        }
                    }
                }
            }
            Algorithm::DifferentialEvolution => {
                let count = cfg.population.max(4);
                let mut seed = cfg.seed;
                let mut pop = vec![x.clone()];
                for _ in 1..count {
                    let mut p = x.clone();
                    for &i in &active {
                        p[i] = random(&mut seed);
                    }
                    pop.push(p);
                }
                let mut costs: Vec<_> = pop.iter().map(|p| e.value(p)).collect();
                for k in 0..cfg.max_iterations {
                    iterations = k + 1;
                    if e.exhausted() {
                        break;
                    }
                    for i in 0..count {
                        if e.exhausted() {
                            break;
                        }
                        let mut ids = Vec::new();
                        while ids.len() < 3 {
                            let j = (random(&mut seed) * count as f64) as usize;
                            if j != i && !ids.contains(&j) {
                                ids.push(j);
                            }
                        }
                        let force = active[(random(&mut seed) * active.len() as f64) as usize];
                        let mut trial = pop[i].clone();
                        for &j in &active {
                            if j == force || random(&mut seed) < 0.9 {
                                trial[j] = (pop[ids[0]][j]
                                    + 0.8 * (pop[ids[1]][j] - pop[ids[2]][j]))
                                    .clamp(0., 1.);
                            }
                        }
                        let cost = e.value(&trial);
                        if cost < costs[i] {
                            pop[i] = trial;
                            costs[i] = cost;
                        }
                    }
                    let diameter = active
                        .iter()
                        .map(|&j| {
                            let lo = pop.iter().map(|p| p[j]).fold(f64::INFINITY, f64::min);
                            let hi = pop.iter().map(|p| p[j]).fold(f64::NEG_INFINITY, f64::max);
                            hi - lo
                        })
                        .fold(0., f64::max);
                    if diameter < cfg.tolerance && e.best.is_some() {
                        converged = true;
                        break;
                    }
                }
            }
        }
    }
    let stop = if e.cancelled {
        Stop::Cancelled
    } else if e.best.is_none() {
        Stop::NoValidCandidate
    } else if converged {
        Stop::Converged
    } else {
        Stop::Budget
    };
    Ok(SearchResult {
        best: e.best,
        evaluations: e.cache.len(),
        iterations,
        stop,
    })
}

/// Objectifs en unités indiquées, pénalité quadratique et limite dure optionnelle.
#[derive(Clone, Copy, Debug)]
pub enum AcousticMetric {
    SplDb,
    ExcursionPeakMm,
    PortSpeedMps,
    RadiatedWatts,
    EfficiencyPct,
}
#[derive(Clone, Copy, Debug)]
pub struct AcousticGoal {
    pub frequency_hz: f64,
    pub metric: AcousticMetric,
    pub target: f64,
    pub weight: f64,
    pub minimum: bool,
    pub hard: bool,
}
/// Évalue un design candidat déjà construit, jamais le design original.
/// `minimum=false` pénalise les dépassements; true pénalise les déficits.
/// Uniformité/SPL cible se représentent par deux contraintes opposées par fréquence.
/// Les puissances approximatives ne sont pas admissibles comme objectif validé.
pub fn acoustic_cost(design: &crate::Design, goals: &[AcousticGoal]) -> Result<f64, SolveError> {
    let prepared = design.prepare()?;
    let mut cost = 0.;
    for g in goals {
        if !g.target.is_finite() || !g.weight.is_finite() || g.weight < 0. {
            return Err(SolveError::InvalidParameter("acoustic objective"));
        }
        let p = prepared.try_response_at(g.frequency_hz)?;
        let value = match g.metric {
            AcousticMetric::SplDb => p.spl_db,
            AcousticMetric::ExcursionPeakMm => p.xd_mm,
            AcousticMetric::PortSpeedMps => p.port_velocity.ok_or(SolveError::InvalidTopology(
                "no port for velocity objective",
            ))?,
            AcousticMetric::RadiatedWatts | AcousticMetric::EfficiencyPct => {
                let e = design.try_energy_at(g.frequency_hz)?;
                if e.radiated_power.quality != crate::energy::RadiationQuality::SingleBaffledPiston
                {
                    return Err(SolveError::InvalidResponse(
                        "objective requires validated radiated power",
                    ));
                }
                if matches!(g.metric, AcousticMetric::RadiatedWatts) {
                    e.radiated_power.watts
                } else {
                    e.efficiency_vs_source_pct()
                }
                .ok_or(SolveError::InvalidResponse("power objective unavailable"))?
            }
        };
        if !value.is_finite() {
            return Err(SolveError::InvalidResponse("objective nonfinite"));
        }
        let deviation = if g.minimum {
            g.target - value
        } else {
            value - g.target
        };
        if deviation > 0. {
            if g.hard {
                return Err(SolveError::InvalidResponse("hard acoustic constraint"));
            }
            cost += g.weight * deviation * deviation;
        }
    }
    if cost.is_finite() {
        Ok(cost)
    } else {
        Err(SolveError::InvalidResponse("objective overflow"))
    }
}
