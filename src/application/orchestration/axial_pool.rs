//! Axial pooling for multi-timeframe feature aggregation.
//!
//! Borrows AViT's axial attention trick: instead of quadratic all-pairs
//! attention over a `timeframe × feature × time` tensor, compute softmax
//! weights independently along each of the three axes and multiply back.
//! Complexity drops from O(n²) to O(n√n) with no NN training required —
//! weights are either supplied by `factor_tucker_core` loadings or fall back
//! to uniform.

use ndarray::{Array2, Array3};
use serde::{Deserialize, Serialize};

/// Upper bound on the normalized entropy of the timeframe-axis weight
/// distribution before the execution tree is forced into `observe`. Above this
/// no timeframe is meaningfully dominant, so an aggressive fill bias would be
/// rolling dice.
pub const AXIAL_TIMEFRAME_ENTROPY_CAP: f64 = 0.85;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxialPoolConfig {
    pub timeframe_axis_weights: Vec<f64>,
    pub feature_axis_weights: Vec<f64>,
    pub softmax_temperature: f64,
}

impl Default for AxialPoolConfig {
    fn default() -> Self {
        Self {
            timeframe_axis_weights: Vec::new(),
            feature_axis_weights: Vec::new(),
            softmax_temperature: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AxialAttentionTrace {
    /// (axis_name, weight) for the timeframe axis, ordered by descending weight.
    pub timeframe_weights: Vec<(String, f64)>,
    /// (feature_name, weight) for the feature axis, ordered by descending weight.
    pub feature_weights: Vec<(String, f64)>,
    /// Normalized Shannon entropy of the timeframe axis weights. [0, 1].
    pub timeframe_entropy: f64,
    /// When true, the execution tree must force `branch = observe` because no
    /// timeframe is dominant. Consumed by `axial_branch_gate_triggers_observe`.
    pub force_observe: bool,
}

/// Aggregate a `[timeframe × feature × time]` tensor along the timeframe axis.
/// Returns a `[feature × time]` matrix plus a trace describing which
/// timeframes dominated the weighting.
pub fn axial_pool_mtf_features(
    tensor: &Array3<f64>,
    config: &AxialPoolConfig,
    timeframe_labels: &[String],
    feature_labels: &[String],
) -> (Array2<f64>, AxialAttentionTrace) {
    let (n_tf, n_feature, n_time) = tensor.dim();

    let tf_raw = fill_axis_weights(&config.timeframe_axis_weights, n_tf);
    let feature_raw = fill_axis_weights(&config.feature_axis_weights, n_feature);

    let temperature = if config.softmax_temperature > 0.0 {
        config.softmax_temperature
    } else {
        1.0
    };
    let tf_weights = softmax(&tf_raw, temperature);
    let feature_weights = softmax(&feature_raw, temperature);

    let mut pooled = Array2::<f64>::zeros((n_feature, n_time));
    for t in 0..n_tf {
        let w_tf = tf_weights[t];
        for f in 0..n_feature {
            let w_feat = feature_weights[f];
            let combined = w_tf * w_feat;
            for u in 0..n_time {
                pooled[[f, u]] += combined * tensor[[t, f, u]];
            }
        }
    }

    let mut tf_trace: Vec<(String, f64)> = tf_weights
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let label = timeframe_labels
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("tf{i}"));
            (label, *w)
        })
        .collect();
    tf_trace.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut feature_trace: Vec<(String, f64)> = feature_weights
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let label = feature_labels
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("feature{i}"));
            (label, *w)
        })
        .collect();
    feature_trace.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let timeframe_entropy = normalized_entropy(&tf_weights);
    let force_observe = timeframe_entropy > AXIAL_TIMEFRAME_ENTROPY_CAP;

    (
        pooled,
        AxialAttentionTrace {
            timeframe_weights: tf_trace,
            feature_weights: feature_trace,
            timeframe_entropy,
            force_observe,
        },
    )
}

/// True when the axial trace requires the execution tree to downgrade
/// `fill_viable` / `wait_for_reversion` decisions into `observe`. Kept as a
/// free function so callers can gate without owning the trace.
pub fn axial_branch_gate_triggers_observe(trace: &AxialAttentionTrace) -> bool {
    trace.force_observe
}

fn fill_axis_weights(config_weights: &[f64], axis_len: usize) -> Vec<f64> {
    if config_weights.len() == axis_len {
        return config_weights.to_vec();
    }
    vec![1.0_f64; axis_len]
}

fn softmax(values: &[f64], temperature: f64) -> Vec<f64> {
    if values.is_empty() {
        return Vec::new();
    }
    let max = values
        .iter()
        .fold(f64::NEG_INFINITY, |acc, value| acc.max(*value));
    let exps: Vec<f64> = values
        .iter()
        .map(|value| ((value - max) / temperature).exp())
        .collect();
    let sum: f64 = exps.iter().sum();
    if sum <= 0.0 {
        return vec![1.0 / values.len() as f64; values.len()];
    }
    exps.into_iter().map(|value| value / sum).collect()
}

fn normalized_entropy(weights: &[f64]) -> f64 {
    if weights.len() <= 1 {
        return 0.0;
    }
    let max_entropy = (weights.len() as f64).ln();
    if max_entropy <= 0.0 {
        return 0.0;
    }
    let mut entropy = 0.0;
    for w in weights {
        if *w <= 0.0 {
            continue;
        }
        entropy -= w * w.ln();
    }
    (entropy / max_entropy).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tensor() -> Array3<f64> {
        // 3 timeframes, 2 features, 4 time steps. Timeframe 0 dominates.
        let mut tensor = Array3::<f64>::zeros((3, 2, 4));
        for f in 0..2 {
            for u in 0..4 {
                tensor[[0, f, u]] = 1.0;
                tensor[[1, f, u]] = 0.1;
                tensor[[2, f, u]] = 0.1;
            }
        }
        tensor
    }

    #[test]
    fn uniform_weights_produce_average_pool() {
        let tensor = sample_tensor();
        let config = AxialPoolConfig::default();
        let (pooled, trace) = axial_pool_mtf_features(
            &tensor,
            &config,
            &["ltf".to_string(), "mtf".to_string(), "htf".to_string()],
            &["body".to_string(), "wick".to_string()],
        );
        assert_eq!(pooled.dim(), (2, 4));
        // Uniform weights → each entry is mean across tf × feature
        let expected = (1.0 + 0.1 + 0.1) / 3.0 * 0.5;
        let got = pooled[[0, 0]];
        assert!(
            (got - expected).abs() < 1e-9,
            "got={got} expected={expected}"
        );
        assert!(trace.timeframe_entropy > 0.95); // near-uniform softmax
        assert!(trace.force_observe);
    }

    #[test]
    fn dominant_timeframe_concentrates_weight() {
        let tensor = sample_tensor();
        let config = AxialPoolConfig {
            timeframe_axis_weights: vec![5.0, 0.0, 0.0],
            feature_axis_weights: vec![1.0, 1.0],
            softmax_temperature: 1.0,
        };
        let (_pooled, trace) = axial_pool_mtf_features(
            &tensor,
            &config,
            &["ltf".to_string(), "mtf".to_string(), "htf".to_string()],
            &["body".to_string(), "wick".to_string()],
        );
        assert_eq!(trace.timeframe_weights[0].0, "ltf");
        assert!(trace.timeframe_weights[0].1 > 0.9);
        assert!(trace.timeframe_entropy < 0.6);
        assert!(!trace.force_observe);
    }

    #[test]
    fn high_entropy_forces_observe_gate() {
        let tensor = Array3::<f64>::from_elem((4, 2, 3), 1.0);
        let config = AxialPoolConfig::default(); // uniform → max entropy
        let (_pooled, trace) = axial_pool_mtf_features(
            &tensor,
            &config,
            &[
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
            ],
            &["x".to_string(), "y".to_string()],
        );
        assert!(axial_branch_gate_triggers_observe(&trace));
    }

    #[test]
    fn softmax_temperature_sharpens_distribution() {
        let tensor = Array3::<f64>::from_elem((3, 1, 1), 0.0);
        let hot = AxialPoolConfig {
            timeframe_axis_weights: vec![1.0, 2.0, 3.0],
            feature_axis_weights: vec![1.0],
            softmax_temperature: 0.1,
        };
        let cold = AxialPoolConfig {
            timeframe_axis_weights: vec![1.0, 2.0, 3.0],
            feature_axis_weights: vec![1.0],
            softmax_temperature: 10.0,
        };
        let (_, hot_trace) = axial_pool_mtf_features(
            &tensor,
            &hot,
            &["a".to_string(), "b".to_string(), "c".to_string()],
            &["x".to_string()],
        );
        let (_, cold_trace) = axial_pool_mtf_features(
            &tensor,
            &cold,
            &["a".to_string(), "b".to_string(), "c".to_string()],
            &["x".to_string()],
        );
        // Low temperature → sharper peak → lower entropy.
        assert!(hot_trace.timeframe_entropy < cold_trace.timeframe_entropy);
    }
}
