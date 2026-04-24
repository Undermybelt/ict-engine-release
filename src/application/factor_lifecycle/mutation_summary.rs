use std::collections::BTreeMap;

use crate::state::FactorMutationEvaluation;

pub fn factor_mutation_priority_markets(evaluation: &FactorMutationEvaluation) -> Vec<String> {
    let mut items = evaluation.metrics_after.regressed_markets.clone();
    if items.is_empty() {
        items.extend(
            evaluation
                .metrics_after
                .regression_reasons_by_market
                .keys()
                .cloned(),
        );
    }
    items.truncate(3);
    items
}

pub fn factor_mutation_priority_reasons(evaluation: &FactorMutationEvaluation) -> Vec<String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for reasons in evaluation
        .metrics_after
        .regression_reasons_by_market
        .values()
    {
        for reason in reasons {
            *counts.entry(reason.clone()).or_default() += 1;
        }
    }
    let mut ordered = counts.into_iter().collect::<Vec<_>>();
    ordered.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut items = ordered
        .into_iter()
        .map(|(reason, _)| reason)
        .collect::<Vec<_>>();
    if items.is_empty() {
        items = evaluation.failure_tags.clone();
    }
    items.truncate(3);
    items
}

pub fn factor_mutation_recommended_focus(evaluation: &FactorMutationEvaluation) -> Vec<String> {
    let mut focus = evaluation.recommended_mutation_directions.clone();
    focus.truncate(3);
    focus
}
