//! Sprint 4 2.4 acceptance: axial pooling + observe-gate on timeframe entropy.

use ict_engine::application::orchestration::{
    axial_branch_gate_triggers_observe, axial_pool_mtf_features, AxialPoolConfig,
};
use ndarray::Array3;

fn labels(prefix: &str, n: usize) -> Vec<String> {
    (0..n).map(|i| format!("{prefix}{i}")).collect()
}

#[test]
fn single_dominant_timeframe_concentrates_top_weight() {
    // 3 timeframes, 2 features, 4 time steps.
    let mut tensor = Array3::<f64>::zeros((3, 2, 4));
    for f in 0..2 {
        for u in 0..4 {
            tensor[[0, f, u]] = 1.0;
            tensor[[1, f, u]] = 0.1;
            tensor[[2, f, u]] = 0.1;
        }
    }
    let config = AxialPoolConfig {
        timeframe_axis_weights: vec![4.0, 0.0, 0.0],
        feature_axis_weights: vec![1.0, 1.0],
        softmax_temperature: 1.0,
    };
    let (pooled, trace) =
        axial_pool_mtf_features(&tensor, &config, &labels("tf", 3), &labels("feat", 2));
    assert_eq!(pooled.dim(), (2, 4));
    assert_eq!(trace.timeframe_weights[0].0, "tf0");
    assert!(trace.timeframe_weights[0].1 > 0.9);
    assert!(!axial_branch_gate_triggers_observe(&trace));
}

#[test]
fn uniform_weights_force_observe_gate() {
    let tensor = Array3::<f64>::from_elem((5, 2, 3), 1.0);
    let config = AxialPoolConfig::default();
    let (_pooled, trace) =
        axial_pool_mtf_features(&tensor, &config, &labels("tf", 5), &labels("feat", 2));
    assert!(axial_branch_gate_triggers_observe(&trace));
}

#[test]
fn temperature_sharpens_timeframe_distribution() {
    let tensor = Array3::<f64>::from_elem((4, 1, 1), 0.0);
    let hot = AxialPoolConfig {
        timeframe_axis_weights: vec![1.0, 2.0, 3.0, 4.0],
        feature_axis_weights: vec![1.0],
        softmax_temperature: 0.2,
    };
    let cold = AxialPoolConfig {
        timeframe_axis_weights: vec![1.0, 2.0, 3.0, 4.0],
        feature_axis_weights: vec![1.0],
        softmax_temperature: 5.0,
    };
    let (_, hot_trace) =
        axial_pool_mtf_features(&tensor, &hot, &labels("tf", 4), &labels("feat", 1));
    let (_, cold_trace) =
        axial_pool_mtf_features(&tensor, &cold, &labels("tf", 4), &labels("feat", 1));
    assert!(hot_trace.timeframe_entropy < cold_trace.timeframe_entropy);
}

#[test]
fn pooling_is_deterministic_across_reruns() {
    let tensor = Array3::<f64>::from_shape_fn((3, 2, 2), |(t, f, u)| (t * 3 + f * 2 + u) as f64);
    let config = AxialPoolConfig {
        timeframe_axis_weights: vec![1.0, 2.0, 3.0],
        feature_axis_weights: vec![1.0, 0.5],
        softmax_temperature: 1.0,
    };
    let (pool_a, trace_a) =
        axial_pool_mtf_features(&tensor, &config, &labels("tf", 3), &labels("feat", 2));
    let (pool_b, trace_b) =
        axial_pool_mtf_features(&tensor, &config, &labels("tf", 3), &labels("feat", 2));
    assert_eq!(pool_a, pool_b);
    assert_eq!(trace_a.timeframe_entropy, trace_b.timeframe_entropy);
    assert_eq!(
        trace_a.timeframe_weights.len(),
        trace_b.timeframe_weights.len()
    );
}
