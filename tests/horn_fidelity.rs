use speakerbox_acoustics::{
    optimization::{screen_and_refine, FidelityStage, ScreenBudget, ScreenCandidate},
    Air, Driver, HornProfileKind as Kind, HornProfileSpec, System,
};

#[test]
fn hard_filter_retains_exhaustive_best_and_diversity_within_budget() {
    let driver = Driver::new(0.031, 0.0004, 0.035, 6.1, 9.0, 1.5, 0.0006);
    let evaluate = |spec: &HornProfileSpec, frequencies: &[f64]| {
        let profile = spec.resolve().map_err(|_| "geometry")?;
        let mut system = System::new(Air::default(), driver);
        system.horn = Some(profile.into_horn());
        let score = |f| {
            system
                .try_response_at(f)
                .map(|p| (p.spl_db - 104.0).powi(2))
                .map_err(|_| "Webster response")
        };
        frequencies
            .iter()
            .try_fold(0.0, |sum, &f| Ok(sum + score(f)?))
    };
    let mut designs = Vec::new();
    for (diversity_group, kind) in [Kind::Conical, Kind::OblateSpheroidal, Kind::Tractrix]
        .into_iter()
        .enumerate()
    {
        for length_m in [0.08, 0.18, 0.3, 0.4] {
            designs.push(ScreenCandidate {
                diversity_group,
                design: HornProfileSpec {
                    kind,
                    length_m,
                    throat_radius_m: 0.035,
                    mouth_radius_m: 0.14,
                },
            });
        }
    }
    let permitted = |spec: &HornProfileSpec| {
        if spec.length_m > 0.35 {
            return Err("CAD length limit");
        }
        spec.resolve().map(|_| ()).map_err(|_| "geometry")
    };
    let exhaustive = |frequencies| {
        designs
            .iter()
            .enumerate()
            .filter(|(_, c)| c.design.length_m <= 0.35)
            .filter_map(|(i, c)| {
                evaluate(&c.design, frequencies)
                    .ok()
                    .map(|score| (i, score))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap()
    };
    let coarse_grid = [100.0, 200.0];
    let fine_grid = [85.0, 100.0, 115.0, 125.0, 150.0, 175.0, 200.0, 225.0];
    let initial_best = exhaustive(&coarse_grid[..]);
    let refined_best = exhaustive(&fine_grid[..]);
    let mut selected = Vec::new();
    let report = screen_and_refine(
        &designs,
        ScreenBudget {
            screen_evaluations: 12,
            refine_evaluations: 3,
        },
        permitted,
        |spec| evaluate(spec, &coarse_grid),
        |spec| {
            selected.push(spec.kind);
            evaluate(spec, &fine_grid)
        },
        || false,
    );
    assert_eq!(report.ranked[0].index, initial_best.0);
    assert_eq!(report.ranked[0].screen_cost, initial_best.1);
    let best_refined = report
        .ranked
        .iter()
        .filter_map(|row| row.refined_cost.map(|cost| (row.index, cost)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap();
    assert_eq!(best_refined, refined_best);
    assert_eq!(report.refine_evaluations, 3);
    assert_eq!(selected.len(), 3);
    assert!(
        selected.contains(&Kind::Conical)
            && selected.contains(&Kind::Tractrix)
            && selected.contains(&Kind::OblateSpheroidal)
    );
    assert!(report.rejections.len() >= 4);
    assert_eq!(
        report.screen_evaluations + report.rejections.len(),
        designs.len()
    );
    assert!(report
        .rejections
        .iter()
        .all(|r| r.stage == FidelityStage::HardConstraint));
}

#[test]
fn budgets_cancellation_and_errors_remain_distinct() {
    let input = [0, 1, 2, 3].map(|design| ScreenCandidate {
        design,
        diversity_group: 0,
    });
    let mut checks = 0;
    let report = screen_and_refine(
        &input,
        ScreenBudget {
            screen_evaluations: 2,
            refine_evaluations: 2,
        },
        |x| if *x == 0 { Err("constraint") } else { Ok(()) },
        |x| Ok(if *x == 1 { f64::NAN } else { 0.0 }),
        |_| Err("backend rejected mesh"),
        || {
            checks += 1;
            checks == 5
        },
    );
    assert_eq!(report.screen_evaluations, 2);
    assert_eq!(report.refine_evaluations, 1);
    assert_eq!(report.ranked[0].index, 2);
    assert_eq!(
        report
            .rejections
            .iter()
            .map(|r| r.stage)
            .collect::<Vec<_>>(),
        [
            FidelityStage::HardConstraint,
            FidelityStage::Screen,
            FidelityStage::Refine
        ]
    );
    assert!(!report.cancelled);

    let mut checks = 0;
    let cancelled = screen_and_refine(
        &input,
        ScreenBudget {
            screen_evaluations: 4,
            refine_evaluations: 2,
        },
        |_| Ok(()),
        |_| Ok(0.0),
        |_| panic!("cancelled before refinement"),
        || {
            checks += 1;
            checks == 2
        },
    );
    assert!(cancelled.cancelled);
    assert_eq!(cancelled.screen_evaluations, 1);
}
