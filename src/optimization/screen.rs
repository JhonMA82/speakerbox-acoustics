use std::collections::BTreeSet;

/// Candidato y grupo de diversidad definido por el consumidor, por ejemplo
/// familia de perfil o rango de longitud. El índice de entrada es su identidad.
#[derive(Clone, Debug)]
pub struct ScreenCandidate<T> {
    pub design: T,
    pub diversity_group: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FidelityStage {
    HardConstraint,
    Screen,
    Refine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FidelityRejection {
    pub index: usize,
    pub stage: FidelityStage,
    pub reason: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FidelityCandidate {
    pub index: usize,
    pub screen_cost: f64,
    pub refined_cost: Option<f64>,
}

/// Límites de llamadas a cada backend. Cero omite esa etapa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenBudget {
    pub screen_evaluations: usize,
    pub refine_evaluations: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FidelityReport {
    /// Orden ascendente por coste inicial, con desempate por índice.
    pub ranked: Vec<FidelityCandidate>,
    pub rejections: Vec<FidelityRejection>,
    pub screen_evaluations: usize,
    pub refine_evaluations: usize,
    pub cancelled: bool,
}

/// Filtro y reevaluación desacoplados del algoritmo que generó los candidatos.
///
/// Solo el filtro duro elimina candidatos antes de la evaluación inicial. Se
/// puede usar Webster para el coste inicial y un backend externo para el
/// refinado. Se prueban primero los mejores representantes de grupos distintos
/// y después se completa el presupuesto por coste inicial. Ambos costes se
/// guardan por separado. Si hay presupuesto inicial insuficiente, se evalúa
/// en orden de entrada. La cancelación se consulta entre candidatos y etapas.
pub fn screen_and_refine<T>(
    candidates: &[ScreenCandidate<T>],
    budget: ScreenBudget,
    mut hard_constraint: impl FnMut(&T) -> Result<(), &'static str>,
    mut screen: impl FnMut(&T) -> Result<f64, &'static str>,
    mut refine: impl FnMut(&T) -> Result<f64, &'static str>,
    mut cancel: impl FnMut() -> bool,
) -> FidelityReport {
    let mut report = FidelityReport::default();
    for (index, candidate) in candidates.iter().enumerate() {
        if report.screen_evaluations == budget.screen_evaluations {
            break;
        }
        if cancel() {
            report.cancelled = true;
            break;
        }
        if let Err(reason) = hard_constraint(&candidate.design) {
            report.rejections.push(FidelityRejection {
                index,
                stage: FidelityStage::HardConstraint,
                reason,
            });
            continue;
        }
        report.screen_evaluations += 1;
        match finite_cost(screen(&candidate.design)) {
            Ok(screen_cost) => report.ranked.push(FidelityCandidate {
                index,
                screen_cost,
                refined_cost: None,
            }),
            Err(reason) => report.rejections.push(FidelityRejection {
                index,
                stage: FidelityStage::Screen,
                reason,
            }),
        }
    }
    report.ranked.sort_by(|a, b| {
        a.screen_cost
            .total_cmp(&b.screen_cost)
            .then(a.index.cmp(&b.index))
    });
    if report.cancelled || budget.refine_evaluations == 0 {
        return report;
    }

    let mut groups = BTreeSet::new();
    let (representatives, others): (Vec<_>, Vec<_>) = (0..report.ranked.len())
        .partition(|&i| groups.insert(candidates[report.ranked[i].index].diversity_group));
    for i in representatives
        .into_iter()
        .chain(others)
        .take(budget.refine_evaluations)
    {
        if cancel() {
            report.cancelled = true;
            break;
        }
        let entry = &mut report.ranked[i];
        report.refine_evaluations += 1;
        match finite_cost(refine(&candidates[entry.index].design)) {
            Ok(cost) => entry.refined_cost = Some(cost),
            Err(reason) => report.rejections.push(FidelityRejection {
                index: entry.index,
                stage: FidelityStage::Refine,
                reason,
            }),
        }
    }
    report
}

fn finite_cost(cost: Result<f64, &'static str>) -> Result<f64, &'static str> {
    cost.and_then(|v| {
        if v.is_finite() {
            Ok(v)
        } else {
            Err("nonfinite cost")
        }
    })
}
