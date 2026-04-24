use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::domain::execution::classify_execution_gate;
use crate::domain::regime::{
    compute_rollout_segments, default_segment_bounds, MeceRegimeLabel, RolloutSegment,
};
use crate::factor_lab::{adaptive_lambda, sparse_select_by_softshrink};
use crate::factors::FactorRegistry;
use crate::state::RunProvenance;
use crate::types::Candle;

/// Softshrink lambda ratio used when pruning the best factor set. Matches
/// `SPECTRAL_DEFAULT_LAMBDA_RATIO` in spirit: anchor the threshold at 5% of
/// the peak weight, so the pruning decision is scale-invariant across
/// different registries or accuracy ranges.
pub const MECE_SOFTSHRINK_LAMBDA_RATIO: f64 = 0.05;

/// Output of `search_factors_for_mece_recovery` — feeds directly into the
/// Sprint 3.3 `MeceRecoveryArtifact` and the hard gate that requires
/// `accuracy >= 0.95`. The `execution_validity_histogram` enforces the dual
/// constraint from the plan: a regime recovery score is only credible if it
/// coexists with non-trivial coverage across `execution_ready` /
/// `execution_observe_only` / `execution_blocked`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MeceRecoveryReport {
    pub accuracy: f64,
    pub macro_f1: f64,
    pub confusion_matrix: BTreeMap<String, BTreeMap<String, usize>>,
    pub best_factor_set: Vec<String>,
    pub execution_validity_histogram: BTreeMap<String, usize>,
    /// Kept/total ratio after softshrink on the best factor set's
    /// removal-impact weights. 1.0 = every factor is pulling weight;
    /// < 0.1 or > 0.9 trips the sparsity hard gate.
    #[serde(default)]
    pub sparsity_ratio: f64,
    /// (factor_name, pre-shrink weight) for factors softshrink pruned from
    /// the best set. Ordered by descending |weight| so the top rows explain
    /// the most-impactful near-misses.
    #[serde(default)]
    pub pruned_factor_trail: Vec<(String, f64)>,
    /// Short / medium / long horizon recovery segments. Populated only when
    /// the search has at least three bars per segment; empty otherwise so
    /// callers can distinguish "blocked by segment gate" from "segment gate
    /// not applicable".
    #[serde(default)]
    pub segments: Vec<RolloutSegment>,
    pub provenance: RunProvenance,
}

const MAX_SUBSET_SIZE: usize = 5;
const LOOKBACK: usize = 10;

const ALL_LABELS: &[MeceRegimeLabel] = &[
    MeceRegimeLabel::Expansion,
    MeceRegimeLabel::Manipulation,
    MeceRegimeLabel::Reversion,
    MeceRegimeLabel::Compression,
    MeceRegimeLabel::TrendContinuation,
    MeceRegimeLabel::Unknown,
];

pub fn search_factors_for_mece_recovery(
    candles: &[Candle],
    labels: &[MeceRegimeLabel],
    registry: &FactorRegistry,
    provenance: RunProvenance,
) -> Result<MeceRecoveryReport> {
    if candles.len() != labels.len() {
        anyhow::bail!(
            "candle / label length mismatch: {} candles vs {} labels",
            candles.len(),
            labels.len()
        );
    }
    if registry.is_empty() {
        anyhow::bail!("factor registry is empty; no subsets to evaluate");
    }

    let factor_names: Vec<String> = registry
        .list()
        .into_iter()
        .map(|factor| factor.name.clone())
        .collect();

    let mut best: Option<(EvalOutcome, Vec<String>, Vec<MeceRegimeLabel>)> = None;
    for subset_indices in non_empty_subsets(factor_names.len(), MAX_SUBSET_SIZE) {
        let subset: Vec<String> = subset_indices
            .iter()
            .map(|idx| factor_names[*idx].clone())
            .collect();
        let predicted = predict_with_factor_subset(candles, &subset);
        let outcome = evaluate(&predicted, labels);
        match &best {
            Some((current, _, _)) if outcome.accuracy <= current.accuracy => {}
            _ => best = Some((outcome, subset, predicted)),
        }
    }

    let (outcome, best_factor_set, best_predicted) =
        best.context("no factor subsets were evaluated")?;
    let execution_validity_histogram = build_execution_histogram(candles);

    // Sparse pruning of the best factor set. Each factor's weight is the
    // *drop* in accuracy when that factor is removed — bigger drop means
    // the factor is doing more work. Softshrink then zeroes out factors
    // whose contribution is below 5% of the peak.
    let factor_weights =
        removal_impact_weights(candles, labels, &best_factor_set, outcome.accuracy);
    let lambda = adaptive_lambda(&factor_weights, MECE_SOFTSHRINK_LAMBDA_RATIO);
    let sparse = sparse_select_by_softshrink(&factor_weights, lambda);

    // Rollout segments on per-bar correctness + per-bar readiness.
    let per_bar_correct: Vec<bool> = best_predicted
        .iter()
        .zip(labels.iter())
        .map(|(pred, truth)| pred == truth)
        .collect();
    let per_bar_readiness: Vec<f64> = candles
        .iter()
        .enumerate()
        .map(|(idx, candle)| per_bar_execution_score(candles, idx, candle))
        .collect();
    let segments = if candles.len() >= 9 {
        let bounds = default_segment_bounds(candles.len());
        compute_rollout_segments(
            &bounds,
            &per_bar_correct,
            &per_bar_readiness,
            |(start, end)| (*start, *end),
        )
    } else {
        Vec::new()
    };

    Ok(MeceRecoveryReport {
        accuracy: outcome.accuracy,
        macro_f1: outcome.macro_f1,
        confusion_matrix: outcome.confusion_matrix,
        best_factor_set,
        execution_validity_histogram,
        sparsity_ratio: sparse.sparsity_ratio,
        pruned_factor_trail: sparse.pruned_factors,
        segments,
        provenance,
    })
}

struct EvalOutcome {
    accuracy: f64,
    macro_f1: f64,
    confusion_matrix: BTreeMap<String, BTreeMap<String, usize>>,
}

fn removal_impact_weights(
    candles: &[Candle],
    labels: &[MeceRegimeLabel],
    best_factor_set: &[String],
    baseline_accuracy: f64,
) -> BTreeMap<String, f64> {
    let mut weights = BTreeMap::new();
    if best_factor_set.is_empty() {
        return weights;
    }
    for leave_out in best_factor_set {
        let reduced: Vec<String> = best_factor_set
            .iter()
            .filter(|name| *name != leave_out)
            .cloned()
            .collect();
        let predicted = predict_with_factor_subset(candles, &reduced);
        let accuracy = accuracy_of(&predicted, labels);
        // Positive drop → factor contributed. Negative drop → factor was
        // actually hurting (rare) and gets a near-zero weight in softshrink.
        let drop = (baseline_accuracy - accuracy).max(0.0);
        weights.insert(leave_out.clone(), drop);
    }
    weights
}

fn accuracy_of(predicted: &[MeceRegimeLabel], truth: &[MeceRegimeLabel]) -> f64 {
    let total = predicted.len();
    if total == 0 {
        return 0.0;
    }
    let correct = predicted
        .iter()
        .zip(truth.iter())
        .filter(|(pred, real)| pred == real)
        .count();
    correct as f64 / total as f64
}

fn evaluate(predicted: &[MeceRegimeLabel], truth: &[MeceRegimeLabel]) -> EvalOutcome {
    let total = predicted.len();
    let correct = predicted
        .iter()
        .zip(truth.iter())
        .filter(|(pred, real)| pred == real)
        .count();
    let accuracy = if total == 0 {
        0.0
    } else {
        correct as f64 / total as f64
    };

    let mut confusion_matrix: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for &truth_label in ALL_LABELS {
        let row = confusion_matrix
            .entry(label_str(truth_label).to_string())
            .or_default();
        for &predicted_label in ALL_LABELS {
            row.entry(label_str(predicted_label).to_string())
                .or_insert(0);
        }
    }
    for (pred, truth_label) in predicted.iter().zip(truth.iter()) {
        let row = confusion_matrix
            .entry(label_str(*truth_label).to_string())
            .or_default();
        let entry = row.entry(label_str(*pred).to_string()).or_insert(0);
        *entry += 1;
    }

    let mut f1_sum = 0.0;
    let mut f1_count = 0;
    for &label in ALL_LABELS {
        let mut tp = 0.0;
        let mut fp = 0.0;
        let mut false_negatives = 0.0;
        for (pred, truth_label) in predicted.iter().zip(truth.iter()) {
            match (*pred == label, *truth_label == label) {
                (true, true) => tp += 1.0,
                (true, false) => fp += 1.0,
                (false, true) => false_negatives += 1.0,
                _ => {}
            }
        }
        let denom = 2.0 * tp + fp + false_negatives;
        if denom > 0.0 {
            f1_sum += 2.0 * tp / denom;
            f1_count += 1;
        }
    }
    let macro_f1 = if f1_count == 0 {
        0.0
    } else {
        f1_sum / f1_count as f64
    };

    EvalOutcome {
        accuracy,
        macro_f1,
        confusion_matrix,
    }
}

fn label_str(label: MeceRegimeLabel) -> &'static str {
    match label {
        MeceRegimeLabel::Expansion => "expansion",
        MeceRegimeLabel::Manipulation => "manipulation",
        MeceRegimeLabel::Reversion => "reversion",
        MeceRegimeLabel::Compression => "compression",
        MeceRegimeLabel::TrendContinuation => "trend_continuation",
        MeceRegimeLabel::Unknown => "unknown",
    }
}

#[derive(Default)]
struct EnabledRules {
    manipulation: bool,
    compression: bool,
    expansion: bool,
    trend_continuation: bool,
    reversion: bool,
}

fn rules_from_factors(factor_subset: &[String]) -> EnabledRules {
    let mut rules = EnabledRules::default();
    for name in factor_subset {
        match name.as_str() {
            "trend_momentum" => {
                rules.trend_continuation = true;
            }
            "volatility_mean_reversion" => {
                rules.reversion = true;
                rules.compression = true;
            }
            "structure_ict" => {
                rules.manipulation = true;
                rules.expansion = true;
            }
            "cross_market_smt" => {
                rules.expansion = true;
                rules.trend_continuation = true;
            }
            "options_hedging" => {
                rules.compression = true;
                rules.reversion = true;
            }
            _ => {}
        }
    }
    rules
}

fn predict_with_factor_subset(
    candles: &[Candle],
    factor_subset: &[String],
) -> Vec<MeceRegimeLabel> {
    let rules = rules_from_factors(factor_subset);
    let mut out = Vec::with_capacity(candles.len());
    for (idx, candle) in candles.iter().enumerate() {
        if idx < LOOKBACK {
            out.push(MeceRegimeLabel::Unknown);
            continue;
        }
        let lookback = &candles[idx - LOOKBACK..idx];
        let avg_range = lookback
            .iter()
            .map(|c| (c.high - c.low).max(0.0))
            .sum::<f64>()
            / LOOKBACK as f64;
        if !avg_range.is_finite() || avg_range <= 0.0 {
            out.push(MeceRegimeLabel::Unknown);
            continue;
        }
        let prev_max_high = lookback
            .iter()
            .map(|c| c.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let prev_min_low = lookback.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);

        let range = (candle.high - candle.low).max(0.0);
        let body = (candle.close - candle.open).abs();
        let prev = &candles[idx - 1];
        let prev_dir = (prev.close - prev.open).signum();
        let curr_dir = (candle.close - candle.open).signum();

        if rules.manipulation && range > 0.0 {
            let swept_high =
                candle.high > prev_max_high && candle.close < (candle.high - 0.6 * range);
            let swept_low = candle.low < prev_min_low && candle.close > (candle.low + 0.6 * range);
            if swept_high || swept_low {
                out.push(MeceRegimeLabel::Manipulation);
                continue;
            }
        }
        if rules.compression && range < 0.5 * avg_range {
            out.push(MeceRegimeLabel::Compression);
            continue;
        }
        if rules.expansion && range > 1.5 * avg_range && body > 0.6 * range {
            out.push(MeceRegimeLabel::Expansion);
            continue;
        }
        if rules.trend_continuation && curr_dir != 0.0 && curr_dir == prev_dir && body > 0.5 * range
        {
            out.push(MeceRegimeLabel::TrendContinuation);
            continue;
        }
        if rules.reversion {
            let lookback_mean = lookback.iter().map(|c| c.close).sum::<f64>() / LOOKBACK as f64;
            if curr_dir != 0.0
                && curr_dir != prev_dir
                && (candle.close - lookback_mean).abs() < (candle.open - lookback_mean).abs()
            {
                out.push(MeceRegimeLabel::Reversion);
                continue;
            }
        }
        out.push(MeceRegimeLabel::Unknown);
    }
    out
}

fn build_execution_histogram(candles: &[Candle]) -> BTreeMap<String, usize> {
    let mut histogram = BTreeMap::new();
    histogram.insert("execution_ready".to_string(), 0);
    histogram.insert("execution_observe_only".to_string(), 0);
    histogram.insert("execution_blocked".to_string(), 0);
    for (idx, candle) in candles.iter().enumerate() {
        let score = per_bar_execution_score(candles, idx, candle);
        let bucket = classify_execution_gate(score);
        *histogram.entry(bucket.to_string()).or_insert(0) += 1;
    }
    histogram
}

fn per_bar_execution_score(candles: &[Candle], idx: usize, candle: &Candle) -> f64 {
    let range = (candle.high - candle.low).max(0.0);
    if range <= 0.0 {
        return 0.0;
    }
    let body_fraction = ((candle.close - candle.open).abs() / range).clamp(0.0, 1.0);
    let alignment = if idx == 0 {
        0.5
    } else {
        let prev = &candles[idx - 1];
        let prev_dir = (prev.close - prev.open).signum();
        let curr_dir = (candle.close - candle.open).signum();
        if curr_dir == 0.0 || prev_dir == 0.0 {
            0.5
        } else if curr_dir == prev_dir {
            1.0
        } else {
            0.0
        }
    };
    (body_fraction * 0.7 + alignment * 0.3).clamp(0.0, 1.0)
}

fn non_empty_subsets(n: usize, max_size: usize) -> Vec<Vec<usize>> {
    let mut subsets = Vec::new();
    let total = 1usize << n;
    for mask in 1..total {
        let indices: Vec<usize> = (0..n).filter(|i| mask & (1 << i) != 0).collect();
        if indices.len() <= max_size {
            subsets.push(indices);
        }
    }
    subsets
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FrameFeatures;
    use crate::domain::regime::manual_mece_labeler;
    use chrono::{Duration, TimeZone, Utc};

    fn ts(n: i64) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap() + Duration::minutes(n)
    }

    fn candle(idx: i64, open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            timestamp: ts(idx),
            open,
            high,
            low,
            close,
            volume: 1_000.0,
        }
    }

    fn synthetic_series() -> Vec<Candle> {
        // 10 flat bars (compression baseline)
        let mut series: Vec<Candle> = (0..10)
            .map(|i| candle(i, 100.0, 100.5, 99.5, 100.0))
            .collect();
        // Wide directional bar (expansion)
        series.push(candle(10, 100.0, 105.0, 99.5, 104.5));
        // Sweep + reject (manipulation): pierces prior max_high (100.5) but closes back inside
        series.push(candle(11, 104.0, 108.0, 103.5, 104.2));
        // Tight bar (compression)
        series.push(candle(12, 104.0, 104.05, 103.95, 104.02));
        // Aligned bullish bar (trend continuation following bar 12 — but bar 12 has zero body, so use index 13 carefully)
        // Build several more bars to give the search variety
        for i in 13..40 {
            let base = 104.0 + (i as f64 - 13.0) * 0.2;
            let bullish = i % 2 == 0;
            if bullish {
                series.push(candle(i, base, base + 0.6, base - 0.1, base + 0.4));
            } else {
                series.push(candle(i, base + 0.4, base + 0.5, base - 0.2, base));
            }
        }
        series
    }

    #[test]
    fn errors_on_length_mismatch() {
        let candles = synthetic_series();
        let labels = vec![MeceRegimeLabel::Unknown; candles.len() - 1];
        let result = search_factors_for_mece_recovery(
            &candles,
            &labels,
            &FactorRegistry::default(),
            RunProvenance::default(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn recovers_synthetic_series_above_baseline() {
        let candles = synthetic_series();
        let labels = manual_mece_labeler(&candles, &FrameFeatures::default());
        let report = search_factors_for_mece_recovery(
            &candles,
            &labels,
            &FactorRegistry::default(),
            RunProvenance::default(),
        )
        .expect("recovery search should succeed");

        assert!(
            report.accuracy >= 0.5,
            "accuracy {} below baseline 0.5",
            report.accuracy
        );
        assert!(
            !report.best_factor_set.is_empty(),
            "best factor set must be non-empty"
        );
    }

    #[test]
    fn execution_histogram_covers_three_buckets() {
        let candles = synthetic_series();
        let labels = manual_mece_labeler(&candles, &FrameFeatures::default());
        let report = search_factors_for_mece_recovery(
            &candles,
            &labels,
            &FactorRegistry::default(),
            RunProvenance::default(),
        )
        .unwrap();

        for bucket in [
            "execution_ready",
            "execution_observe_only",
            "execution_blocked",
        ] {
            assert!(
                report.execution_validity_histogram.contains_key(bucket),
                "histogram missing {bucket}"
            );
        }
        let total: usize = report.execution_validity_histogram.values().sum();
        assert_eq!(total, candles.len(), "histogram count must equal candles");
    }

    #[test]
    fn confusion_matrix_has_row_per_label() {
        let candles = synthetic_series();
        let labels = manual_mece_labeler(&candles, &FrameFeatures::default());
        let report = search_factors_for_mece_recovery(
            &candles,
            &labels,
            &FactorRegistry::default(),
            RunProvenance::default(),
        )
        .unwrap();

        for &label in ALL_LABELS {
            assert!(
                report.confusion_matrix.contains_key(label_str(label)),
                "confusion matrix missing row for {}",
                label_str(label)
            );
        }
    }
}
