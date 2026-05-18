//! Soft-threshold sparse selection for factor weights.
//!
//! Mirrors the AFNO softshrink operator from `the_well`: weights below
//! `lambda` collapse to zero, weights above it get shrunk by `lambda`. The
//! result is a deterministic, auditable pruning pass that tells the MECE
//! recovery gate how much of the chosen factor set is actually doing work.
//!
//! This layer lives in `factor_lab` because the pruning decision is a
//! factor-lifecycle concern; the recovery loop calls into it at the end of
//! each search and records the result in `MeceRecoveryReport`.

use std::collections::BTreeMap;

/// Lower-bound ratio of kept/total factors. Below this the softshrink was
/// too aggressive — the selection is effectively one-factor and likely a
/// collapse of the search, not a real result. Consumed by the MECE recovery
/// hard gate alongside the accuracy threshold.
pub const MECE_SPARSITY_LOWER_BOUND: f64 = 0.10;

/// Upper-bound ratio of kept/total factors. Above this the softshrink pruned
/// almost nothing, meaning `lambda` was too small or every factor carries
/// identical weight (degenerate search outcome).
pub const MECE_SPARSITY_UPPER_BOUND: f64 = 0.90;

#[derive(Debug, Clone, Default)]
pub struct SparseSelection {
    pub kept_factors: Vec<String>,
    /// (factor_name, pre-shrink weight) for factors that softshrink zeroed.
    /// Ordered by descending pre-shrink weight so the trail is readable.
    pub pruned_factors: Vec<(String, f64)>,
    /// kept_count / total_count. Zero when input is empty.
    pub sparsity_ratio: f64,
    pub lambda: f64,
}

/// Apply `sign(w) * max(|w| - lambda, 0)` and split factors into kept/pruned
/// by the post-shrink magnitude. Zero-weight factors are always pruned
/// (and reported with their original 0.0 weight).
pub fn sparse_select_by_softshrink(
    weights: &BTreeMap<String, f64>,
    lambda: f64,
) -> SparseSelection {
    let total = weights.len();
    if total == 0 {
        return SparseSelection {
            kept_factors: Vec::new(),
            pruned_factors: Vec::new(),
            sparsity_ratio: 0.0,
            lambda: lambda.max(0.0),
        };
    }
    let lambda = lambda.max(0.0);

    let mut kept = Vec::new();
    let mut pruned = Vec::new();
    for (name, &weight) in weights.iter() {
        let magnitude = weight.abs();
        if magnitude > lambda {
            kept.push(name.clone());
        } else {
            pruned.push((name.clone(), weight));
        }
    }
    pruned.sort_by(|a, b| {
        b.1.abs()
            .partial_cmp(&a.1.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    SparseSelection {
        sparsity_ratio: kept.len() as f64 / total as f64,
        kept_factors: kept,
        pruned_factors: pruned,
        lambda,
    }
}

/// Pick lambda as `ratio * max(|w|)`. Scales with the data's natural peak so
/// the threshold is comparable across runs with different weight magnitudes.
/// Mirrors `SPECTRAL_DEFAULT_LAMBDA_RATIO` from the frequency-domain overlay —
/// both layers anchor lambda to their peak.
pub fn adaptive_lambda(weights: &BTreeMap<String, f64>, ratio: f64) -> f64 {
    if weights.is_empty() {
        return 0.0;
    }
    let peak = weights
        .values()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    peak * ratio.max(0.0)
}

/// True when `sparsity_ratio` sits in the healthy band. The MECE recovery
/// hard gate rejects selections outside this band regardless of accuracy.
pub fn sparsity_ratio_within_bounds(sparsity_ratio: f64) -> bool {
    (MECE_SPARSITY_LOWER_BOUND..=MECE_SPARSITY_UPPER_BOUND).contains(&sparsity_ratio)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weights(entries: &[(&str, f64)]) -> BTreeMap<String, f64> {
        entries
            .iter()
            .map(|(name, weight)| ((*name).to_string(), *weight))
            .collect()
    }

    #[test]
    fn empty_weights_yield_empty_selection() {
        let selection = sparse_select_by_softshrink(&BTreeMap::new(), 0.5);
        assert!(selection.kept_factors.is_empty());
        assert!(selection.pruned_factors.is_empty());
        assert_eq!(selection.sparsity_ratio, 0.0);
    }

    #[test]
    fn softshrink_zeroes_small_weights_and_keeps_large_ones() {
        let w = weights(&[("a", 0.05), ("b", 0.30), ("c", -0.45), ("d", 0.08)]);
        let selection = sparse_select_by_softshrink(&w, 0.10);
        assert_eq!(
            selection.kept_factors,
            vec!["b".to_string(), "c".to_string()]
        );
        assert_eq!(selection.pruned_factors[0].0, "d");
        assert_eq!(selection.pruned_factors[1].0, "a");
        assert!((selection.sparsity_ratio - 0.5).abs() < 1e-9);
    }

    #[test]
    fn zero_weights_are_always_pruned() {
        let w = weights(&[("a", 0.0), ("b", 1.0)]);
        let selection = sparse_select_by_softshrink(&w, 0.0);
        // lambda=0: a is magnitude 0.0 which is NOT > 0.0, so pruned; b is > 0 so kept.
        assert_eq!(selection.kept_factors, vec!["b".to_string()]);
        assert_eq!(selection.pruned_factors[0].0, "a");
    }

    #[test]
    fn adaptive_lambda_scales_with_peak() {
        let w = weights(&[("a", 0.05), ("b", 0.4), ("c", -1.2)]);
        let lambda = adaptive_lambda(&w, 0.05);
        assert!((lambda - 0.06).abs() < 1e-9);
    }

    #[test]
    fn sparsity_bounds_accept_healthy_band_and_reject_extremes() {
        assert!(sparsity_ratio_within_bounds(0.50));
        assert!(sparsity_ratio_within_bounds(MECE_SPARSITY_LOWER_BOUND));
        assert!(sparsity_ratio_within_bounds(MECE_SPARSITY_UPPER_BOUND));
        assert!(!sparsity_ratio_within_bounds(0.05));
        assert!(!sparsity_ratio_within_bounds(0.95));
    }

    #[test]
    fn all_pruned_when_lambda_above_peak() {
        let w = weights(&[("a", 0.1), ("b", 0.2)]);
        let selection = sparse_select_by_softshrink(&w, 0.5);
        assert!(selection.kept_factors.is_empty());
        assert_eq!(selection.pruned_factors.len(), 2);
        assert_eq!(selection.sparsity_ratio, 0.0);
    }

    #[test]
    fn nothing_pruned_when_lambda_zero_and_no_zero_weights() {
        let w = weights(&[("a", 0.1), ("b", -0.05)]);
        let selection = sparse_select_by_softshrink(&w, 0.0);
        assert_eq!(selection.kept_factors.len(), 2);
        assert!(selection.pruned_factors.is_empty());
        assert!((selection.sparsity_ratio - 1.0).abs() < 1e-9);
    }
}
