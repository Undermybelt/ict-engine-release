use std::collections::{BTreeMap, HashMap};

use chrono::Utc;

use crate::config::{family_history_window, trend_label_f64, trend_label_usize};
use crate::state::{
    DecisionHistorySummary, FactorFamilyDecision, FactorFamilyDiff, FactorFamilyHistory,
    PersistedFactorRanking, ProbabilityDiff, PromotionDecision, RankingDiffItem,
    RollbackRecommendation,
};

pub fn ranking_diffs(
    previous: &[PersistedFactorRanking],
    current: &[PersistedFactorRanking],
) -> Vec<RankingDiffItem> {
    let previous_map = previous
        .iter()
        .map(|item| (item.factor_name.clone(), item))
        .collect::<HashMap<_, _>>();
    let mut diffs = current
        .iter()
        .map(|item| {
            let previous = previous_map.get(&item.factor_name).copied();
            RankingDiffItem {
                factor_name: item.factor_name.clone(),
                previous_score: previous.map(|entry| entry.composite_score),
                new_score: item.composite_score,
                score_delta: item.composite_score
                    - previous.map(|entry| entry.composite_score).unwrap_or(0.0),
                previous_weight: previous.map(|entry| entry.weight),
                new_weight: item.weight,
                weight_delta: item.weight - previous.map(|entry| entry.weight).unwrap_or(0.0),
                previous_action: previous.map(|entry| entry.iteration_action.clone()),
                new_action: item.iteration_action.clone(),
            }
        })
        .collect::<Vec<_>>();
    diffs.sort_by(|a, b| {
        b.score_delta
            .abs()
            .partial_cmp(&a.score_delta.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    diffs
}

pub fn probability_diffs(
    previous: &Option<BTreeMap<String, f64>>,
    current: &BTreeMap<String, f64>,
) -> Vec<ProbabilityDiff> {
    let mut keys = current.keys().cloned().collect::<Vec<_>>();
    keys.sort();
    keys.into_iter()
        .map(|state| {
            let new = current.get(&state).copied().unwrap_or(0.0);
            let previous_value = previous.as_ref().and_then(|map| map.get(&state).copied());
            ProbabilityDiff {
                state,
                previous: previous_value,
                new,
                delta: new - previous_value.unwrap_or(0.0),
            }
        })
        .collect()
}

pub fn cpt_probability_diffs(
    previous: &BTreeMap<String, BTreeMap<String, f64>>,
    current: &BTreeMap<String, BTreeMap<String, f64>>,
) -> Vec<ProbabilityDiff> {
    let mut diffs = Vec::new();
    for (entry_quality, current_probs) in current {
        let previous_probs = previous.get(entry_quality).cloned();
        for diff in probability_diffs(&previous_probs, current_probs) {
            diffs.push(ProbabilityDiff {
                state: format!("{}:{}", entry_quality, diff.state),
                previous: diff.previous,
                new: diff.new,
                delta: diff.delta,
            });
        }
    }
    diffs
}

pub fn family_diffs(
    previous: &[FactorFamilyDecision],
    current: &[FactorFamilyDecision],
) -> Vec<FactorFamilyDiff> {
    let previous_map = previous
        .iter()
        .map(|item| (item.family.clone(), item))
        .collect::<HashMap<_, _>>();
    let mut diffs = current
        .iter()
        .map(|item| {
            let previous = previous_map.get(&item.family).copied();
            FactorFamilyDiff {
                family: item.family.clone(),
                previous_avg_score: previous.map(|entry| entry.avg_score),
                new_avg_score: item.avg_score,
                avg_score_delta: item.avg_score
                    - previous.map(|entry| entry.avg_score).unwrap_or(0.0),
                previous_replacement_count: previous
                    .map(|entry| entry.replacement_candidates.len())
                    .unwrap_or(0),
                new_replacement_count: item.replacement_candidates.len(),
            }
        })
        .collect::<Vec<_>>();
    diffs.sort_by(|a, b| {
        b.avg_score_delta
            .abs()
            .partial_cmp(&a.avg_score_delta.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    diffs
}

pub fn family_history_from_runs<I>(runs: I) -> Vec<FactorFamilyHistory>
where
    I: IntoIterator<Item = (String, chrono::DateTime<Utc>, Vec<FactorFamilyDecision>)>,
{
    let runs = runs.into_iter().collect::<Vec<_>>();
    let window_size = family_history_window();
    let mut grouped = BTreeMap::<
        String,
        (
            Vec<String>,
            Vec<chrono::DateTime<Utc>>,
            Vec<f64>,
            Vec<usize>,
        ),
    >::new();
    for (run_id, timestamp, run) in runs.into_iter().rev().take(window_size).rev() {
        for family in run {
            let entry = grouped.entry(family.family.clone()).or_default();
            entry.0.push(run_id.clone());
            entry.1.push(timestamp);
            entry.2.push(family.avg_score);
            entry.3.push(family.replacement_candidates.len());
        }
    }

    grouped
        .into_iter()
        .map(
            |(
                family,
                (recent_run_ids, recent_timestamps, recent_avg_scores, recent_replacement_counts),
            )| {
                let score_trend = trend_label_f64(&recent_avg_scores);
                let replacement_trend = trend_label_usize(&recent_replacement_counts);
                FactorFamilyHistory {
                    family,
                    window_size,
                    recent_run_ids,
                    recent_timestamps,
                    recent_avg_scores,
                    recent_replacement_counts,
                    score_trend,
                    replacement_trend,
                }
            },
        )
        .collect()
}

pub fn decision_history_summary<I>(runs: I) -> DecisionHistorySummary
where
    I: IntoIterator<Item = (PromotionDecision, RollbackRecommendation)>,
{
    let runs = runs.into_iter().collect::<Vec<_>>();
    let total_runs = runs.len();
    let promotion_approved_runs = runs
        .iter()
        .filter(|(promotion, _)| promotion.approved)
        .count();
    let rollback_recommended_runs = runs
        .iter()
        .filter(|(_, rollback)| rollback.should_rollback)
        .count();
    let latest_promotion_status = runs.last().map(|(promotion, _)| promotion.status.clone());
    let latest_rollback_scope = runs.last().map(|(_, rollback)| rollback.scope.clone());

    DecisionHistorySummary {
        total_runs,
        promotion_approved_runs,
        rollback_recommended_runs,
        latest_promotion_status,
        latest_rollback_scope,
    }
}
