//! Sprint 3 2.2 acceptance: sparse factor selection via softshrink.

use std::collections::BTreeMap;

use ict_engine::factor_lab::{
    adaptive_lambda, sparse_select_by_softshrink, sparsity_ratio_within_bounds,
    MECE_SPARSITY_LOWER_BOUND, MECE_SPARSITY_UPPER_BOUND,
};

fn weights(entries: &[(&str, f64)]) -> BTreeMap<String, f64> {
    entries
        .iter()
        .map(|(name, w)| ((*name).to_string(), *w))
        .collect()
}

#[test]
fn mixed_weights_produce_healthy_sparsity_band() {
    let w = weights(&[
        ("structure_ict", 0.42),
        ("volatility_mean_reversion", 0.30),
        ("trend_momentum", 0.08),
        ("cross_market_smt", 0.05),
        ("options_hedging", 0.02),
    ]);
    let lambda = adaptive_lambda(&w, 0.20);
    let selection = sparse_select_by_softshrink(&w, lambda);
    assert!(sparsity_ratio_within_bounds(selection.sparsity_ratio));
    assert!(!selection.kept_factors.is_empty());
    assert!(!selection.pruned_factors.is_empty());
}

#[test]
fn deterministic_across_reruns() {
    let w = weights(&[("a", 0.5), ("b", 0.3), ("c", 0.1)]);
    let s1 = sparse_select_by_softshrink(&w, 0.15);
    let s2 = sparse_select_by_softshrink(&w, 0.15);
    assert_eq!(s1.kept_factors, s2.kept_factors);
    assert_eq!(s1.sparsity_ratio, s2.sparsity_ratio);
    assert_eq!(s1.pruned_factors.len(), s2.pruned_factors.len());
}

#[test]
fn full_collapse_flagged_by_sparsity_bounds() {
    let w = weights(&[("a", 0.01), ("b", 0.02), ("c", 0.03), ("d", 1.0)]);
    let selection = sparse_select_by_softshrink(&w, 0.5);
    // Only d survives → sparsity_ratio = 0.25 which is inside bounds (>= 0.10).
    // Make it stricter: lambda above the peak zeroes everything.
    let total_prune = sparse_select_by_softshrink(&w, 2.0);
    assert_eq!(total_prune.sparsity_ratio, 0.0);
    assert!(!sparsity_ratio_within_bounds(total_prune.sparsity_ratio));
    assert!(selection.sparsity_ratio >= MECE_SPARSITY_LOWER_BOUND);
}

#[test]
fn zero_prune_flagged_as_degenerate() {
    let w = weights(&[("a", 1.0), ("b", 1.0), ("c", 1.0), ("d", 1.0), ("e", 1.0)]);
    let selection = sparse_select_by_softshrink(&w, 0.0);
    assert_eq!(selection.sparsity_ratio, 1.0);
    assert!(selection.sparsity_ratio > MECE_SPARSITY_UPPER_BOUND);
    assert!(!sparsity_ratio_within_bounds(selection.sparsity_ratio));
}

#[test]
fn pruned_trail_is_ordered_by_descending_magnitude() {
    let w = weights(&[("a", 0.05), ("b", -0.08), ("c", 0.02), ("d", 0.5)]);
    let selection = sparse_select_by_softshrink(&w, 0.10);
    assert_eq!(selection.pruned_factors.len(), 3);
    let mut last_mag = f64::INFINITY;
    for (_, weight) in &selection.pruned_factors {
        let magnitude = weight.abs();
        assert!(magnitude <= last_mag);
        last_mag = magnitude;
    }
}
