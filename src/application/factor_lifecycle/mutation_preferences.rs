use std::collections::BTreeMap;

use serde::Serialize;

use crate::state::{load_state_or_default, FactorMutationRunRecord, FACTOR_MUTATION_RUNS_FILE};

#[derive(Debug, Clone, Serialize)]
pub struct FactorMutationHintEffectivenessSummary {
    pub hint: String,
    pub count: usize,
    pub accepted_runs: usize,
    pub acceptance_rate: f64,
    pub average_score_delta: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FactorMutationPerFactorHintSummary {
    pub base_factor: String,
    pub direction_hint_effectiveness: Vec<FactorMutationHintEffectivenessSummary>,
    pub step_size_hint_effectiveness: Vec<FactorMutationHintEffectivenessSummary>,
}

pub fn build_hint_effectiveness_summary(
    hint: &str,
    deltas: &[f64],
    accepted_runs: usize,
) -> FactorMutationHintEffectivenessSummary {
    FactorMutationHintEffectivenessSummary {
        hint: hint.to_string(),
        count: deltas.len(),
        accepted_runs,
        acceptance_rate: if deltas.is_empty() {
            0.0
        } else {
            accepted_runs as f64 / deltas.len() as f64
        },
        average_score_delta: if deltas.is_empty() {
            0.0
        } else {
            deltas.iter().sum::<f64>() / deltas.len() as f64
        },
    }
}

pub fn compare_hint_effectiveness(
    left: &FactorMutationHintEffectivenessSummary,
    right: &FactorMutationHintEffectivenessSummary,
) -> std::cmp::Ordering {
    left.acceptance_rate
        .partial_cmp(&right.acceptance_rate)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| {
            left.average_score_delta
                .partial_cmp(&right.average_score_delta)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .then_with(|| left.count.cmp(&right.count))
}

pub fn factor_specific_hint_preferences(
    state_dir: &str,
    symbol: &str,
    base_factor: &str,
) -> (BTreeMap<String, String>, BTreeMap<String, f64>) {
    let runs: Vec<FactorMutationRunRecord> =
        load_state_or_default(state_dir, symbol, FACTOR_MUTATION_RUNS_FILE).unwrap_or_default();
    let relevant_runs = runs
        .into_iter()
        .filter(|run| run.mutation_spec.base_factor == base_factor)
        .collect::<Vec<_>>();
    let mut direction_candidates =
        BTreeMap::<String, FactorMutationHintEffectivenessSummary>::new();
    let mut direction_buckets = BTreeMap::<String, BTreeMap<String, Vec<f64>>>::new();
    let mut direction_accepts = BTreeMap::<String, BTreeMap<String, usize>>::new();
    let mut step_candidates = BTreeMap::<String, FactorMutationHintEffectivenessSummary>::new();
    let mut step_buckets = BTreeMap::<String, BTreeMap<String, Vec<f64>>>::new();
    let mut step_accepts = BTreeMap::<String, BTreeMap<String, usize>>::new();
    for run in &relevant_runs {
        for (parameter, hint) in &run.mutation_spec.direction_hints {
            direction_buckets
                .entry(parameter.clone())
                .or_default()
                .entry(hint.clone())
                .or_default()
                .push(run.evaluation.score_delta);
            if run.evaluation.accepted {
                *direction_accepts
                    .entry(parameter.clone())
                    .or_default()
                    .entry(hint.clone())
                    .or_default() += 1;
            }
        }
        for (parameter, step) in &run.mutation_spec.step_size_hints {
            let label = format!("{:.4}", step);
            step_buckets
                .entry(parameter.clone())
                .or_default()
                .entry(label.clone())
                .or_default()
                .push(run.evaluation.score_delta);
            if run.evaluation.accepted {
                *step_accepts
                    .entry(parameter.clone())
                    .or_default()
                    .entry(label.clone())
                    .or_default() += 1;
            }
        }
    }
    for (parameter, entries) in direction_buckets {
        for (hint, deltas) in entries {
            let accepted = direction_accepts
                .get(&parameter)
                .and_then(|items| items.get(&hint))
                .copied()
                .unwrap_or_default();
            let summary = build_hint_effectiveness_summary(&hint, &deltas, accepted);
            let replace = direction_candidates
                .get(&parameter)
                .map(|existing| compare_hint_effectiveness(&summary, existing).is_gt())
                .unwrap_or(true);
            if replace {
                direction_candidates.insert(parameter.clone(), summary);
            }
        }
    }
    for (parameter, entries) in step_buckets {
        for (hint, deltas) in entries {
            let accepted = step_accepts
                .get(&parameter)
                .and_then(|items| items.get(&hint))
                .copied()
                .unwrap_or_default();
            let summary = build_hint_effectiveness_summary(&hint, &deltas, accepted);
            let replace = step_candidates
                .get(&parameter)
                .map(|existing| compare_hint_effectiveness(&summary, existing).is_gt())
                .unwrap_or(true);
            if replace {
                step_candidates.insert(parameter.clone(), summary);
            }
        }
    }
    (
        direction_candidates
            .into_iter()
            .map(|(parameter, summary)| (parameter, summary.hint))
            .collect(),
        step_candidates
            .into_iter()
            .filter_map(|(parameter, summary)| {
                summary
                    .hint
                    .parse::<f64>()
                    .ok()
                    .map(|value| (parameter, value))
            })
            .collect(),
    )
}
