//! Round 2 §3.2 — Tucker driver over real state directories.
//!
//! Reads `<state_dir>/<symbol>/learning_state.json`, builds a
//! factor × regime × metric tensor, and feeds it to `fit_tucker_core`. This
//! is the minimal "how do we get a real tensor into the Tucker layer" path
//! that the Round 1 coverage left stubbed.
//!
//! The `metric` axis is synthetic but meaningful — we treat
//! `(multiplier, observations, avg_pnl)` as three pseudo-timeframes because
//! the current project state does not yet carry a per-factor × per-regime
//! × per-timeframe cube. Round 3 or Sprint 4 work will replace this axis
//! with real MTF stats; for now the shape matches the Tucker contract and
//! the reconstruction error is interpretable.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use ndarray::Array3;

use crate::factor_lab::{fit_tucker_core, TuckerCore};
use crate::state::{FactorLearningProfile, LearningState};

pub const DEFAULT_METRIC_AXIS_LABELS: &[&str] = &["multiplier", "observations", "avg_pnl"];

pub struct FactorTensor {
    pub tensor: Array3<f64>,
    pub factor_labels: Vec<String>,
    pub regime_labels: Vec<String>,
    pub metric_labels: Vec<String>,
}

/// Read `learning_state.json` from `<state_dir>/<symbol>/` and build a
/// `[n_factor × n_regime × n_metric]` tensor. Returns None when no factor
/// has any regime_stats (fresh state, nothing to decompose).
pub fn build_factor_tensor_from_state_dir(
    state_dir: &Path,
    symbol: &str,
) -> Result<Option<FactorTensor>> {
    let learning_path = state_dir.join(symbol).join("learning_state.json");
    if !learning_path.exists() {
        anyhow::bail!(
            "learning_state.json not found at {}",
            learning_path.display()
        );
    }
    let raw = fs::read_to_string(&learning_path)
        .with_context(|| format!("reading {}", learning_path.display()))?;
    let state: LearningState = serde_json::from_str(&raw).context("parsing learning_state.json")?;
    Ok(build_factor_tensor_from_learning_state(&state))
}

pub fn build_factor_tensor_from_learning_state(state: &LearningState) -> Option<FactorTensor> {
    let factor_labels: Vec<String> = state.factor_profiles.keys().cloned().collect();
    if factor_labels.is_empty() {
        return None;
    }

    // Collect the union of regime names across all factors — keeps the tensor
    // dense even when one factor was never observed in a regime.
    let mut regime_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for profile in state.factor_profiles.values() {
        regime_set.extend(profile.regime_stats.keys().cloned());
    }
    if regime_set.is_empty() {
        return None;
    }
    let regime_labels: Vec<String> = regime_set.into_iter().collect();
    let metric_labels: Vec<String> = DEFAULT_METRIC_AXIS_LABELS
        .iter()
        .map(|s| s.to_string())
        .collect();

    let nf = factor_labels.len();
    let nr = regime_labels.len();
    let nm = metric_labels.len();
    let mut tensor = Array3::<f64>::zeros((nf, nr, nm));

    for (i, factor) in factor_labels.iter().enumerate() {
        let profile = state
            .factor_profiles
            .get(factor)
            .cloned()
            .unwrap_or_else(FactorLearningProfile::default);
        for (j, regime) in regime_labels.iter().enumerate() {
            let stats = profile.regime_stats.get(regime);
            // multiplier: execution-sizing multiplier (0.8-1.2 range in real data)
            tensor[[i, j, 0]] = stats.map(|s| s.multiplier).unwrap_or(1.0);
            // observations: log-scaled so one heavy regime doesn't dominate
            tensor[[i, j, 1]] = stats
                .map(|s| (s.observations as f64 + 1.0).ln())
                .unwrap_or(0.0);
            // avg_pnl: scaled to bps so it's on the same order as multiplier
            tensor[[i, j, 2]] = stats.map(|s| s.avg_pnl * 10_000.0).unwrap_or(0.0);
        }
    }

    Some(FactorTensor {
        tensor,
        factor_labels,
        regime_labels,
        metric_labels,
    })
}

/// Pick ranks as `(min(nf, 3), min(nr, 3), min(nm, 3))`. Deliberately small —
/// we want the Tucker core to summarise dominant patterns, not reproduce the
/// tensor exactly. Caller can override by invoking `fit_tucker_core` directly.
pub fn default_ranks(tensor: &Array3<f64>) -> (usize, usize, usize) {
    let (nf, nr, nm) = tensor.dim();
    (nf.clamp(1, 3), nr.clamp(1, 3), nm.clamp(1, 3))
}

/// End-to-end helper: reads state dir → builds tensor → fits tucker core.
/// Returns `(tucker, factor_labels, regime_labels, metric_labels)`. When the
/// state dir has no regime_stats (fresh project) returns Ok(None).
#[allow(clippy::type_complexity)]
pub fn fit_tucker_core_from_state_dir(
    state_dir: &Path,
    symbol: &str,
) -> Result<Option<(TuckerCore, Vec<String>, Vec<String>, Vec<String>)>> {
    let Some(factor_tensor) = build_factor_tensor_from_state_dir(state_dir, symbol)? else {
        return Ok(None);
    };
    let ranks = default_ranks(&factor_tensor.tensor);
    let tucker = fit_tucker_core(&factor_tensor.tensor, ranks)
        .context("tucker fit failed — see ranks vs dimensions")?;
    Ok(Some((
        tucker,
        factor_tensor.factor_labels,
        factor_tensor.regime_labels,
        factor_tensor.metric_labels,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{FactorLearningProfile, RegimeFactorStats};

    fn sample_learning_state() -> LearningState {
        let mut state = LearningState::default();
        for (name, seed) in [("trend_momentum", 1.0), ("volatility_mean_reversion", 0.8)].iter() {
            let mut profile = FactorLearningProfile::default();
            for regime in ["accumulation", "distribution", "manipulation_expansion"] {
                profile.regime_stats.insert(
                    regime.to_string(),
                    RegimeFactorStats {
                        observations: 1000,
                        wins: 500,
                        avg_pnl: 0.00001 * seed,
                        multiplier: 0.85 + 0.1 * seed,
                    },
                );
            }
            state.factor_profiles.insert(name.to_string(), profile);
        }
        state
    }

    #[test]
    fn returns_none_for_empty_learning_state() {
        let state = LearningState::default();
        assert!(build_factor_tensor_from_learning_state(&state).is_none());
    }

    #[test]
    fn builds_dense_tensor_from_learning_state() {
        let state = sample_learning_state();
        let factor_tensor = build_factor_tensor_from_learning_state(&state).expect("non-empty");
        assert_eq!(factor_tensor.factor_labels.len(), 2);
        assert_eq!(factor_tensor.regime_labels.len(), 3);
        assert_eq!(factor_tensor.metric_labels.len(), 3);
        assert_eq!(factor_tensor.tensor.dim(), (2, 3, 3));
    }

    #[test]
    fn default_ranks_cap_at_three_or_dim() {
        let small = Array3::<f64>::zeros((2, 3, 2));
        assert_eq!(default_ranks(&small), (2, 3, 2));
        let large = Array3::<f64>::zeros((10, 8, 5));
        assert_eq!(default_ranks(&large), (3, 3, 3));
    }

    #[test]
    fn fits_tucker_core_from_synthetic_learning_state() {
        let state = sample_learning_state();
        let factor_tensor = build_factor_tensor_from_learning_state(&state).expect("non-empty");
        let ranks = default_ranks(&factor_tensor.tensor);
        let tucker = fit_tucker_core(&factor_tensor.tensor, ranks).expect("fit");
        assert!(tucker.reconstruction_error < 1e-6);
    }
}
