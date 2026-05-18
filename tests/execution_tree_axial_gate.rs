//! Round 2 §3.1 integration: axial observe gate on ExecutionTreeInput.

use ict_engine::application::execution::ExecutionPhysicsOverlay;
use ict_engine::application::orchestration::{
    AxialAttentionTrace, DefaultExecutionTreeScorer, ExecutionTreeInput, ExecutionTreeScorer,
};
use ict_engine::domain::execution::ExecutionFeatures;
use ict_engine::domain::regime::IsingState;
use ict_engine::ict::PythagoreanExtensionMetrics;
use ict_engine::types::RegimeProbs;

fn flat_overlay() -> ExecutionPhysicsOverlay {
    ExecutionPhysicsOverlay {
        ou: None,
        ising: Some(IsingState {
            magnetization: 0.0,
            coupling_strength: 0.2,
            phase_transition_risk: 0.2,
            herding_bias: 0.1,
        }),
        pythagorean: Some(PythagoreanExtensionMetrics {
            trendline_distance: 0.0,
            orthogonal_extension: 0.0,
            normalized_overstretch: 0.1,
        }),
        spectral: None,
    }
}

fn features(readiness: f64) -> ExecutionFeatures {
    ExecutionFeatures {
        execution_readiness: readiness,
        execution_score: readiness,
        evidence_quality: 0.7,
        ..Default::default()
    }
}

fn posterior() -> RegimeProbs {
    RegimeProbs {
        accumulation: 0.34,
        manipulation_expansion: 0.33,
        distribution: 0.33,
    }
}

fn concentrated_trace() -> AxialAttentionTrace {
    AxialAttentionTrace {
        timeframe_weights: vec![
            ("ltf".to_string(), 0.90),
            ("mtf".to_string(), 0.07),
            ("htf".to_string(), 0.03),
        ],
        feature_weights: vec![("close".to_string(), 1.0)],
        timeframe_entropy: 0.15,
        force_observe: false,
    }
}

fn uniform_trace() -> AxialAttentionTrace {
    AxialAttentionTrace {
        timeframe_weights: vec![
            ("ltf".to_string(), 0.25),
            ("mtf".to_string(), 0.25),
            ("htf".to_string(), 0.25),
            ("dtf".to_string(), 0.25),
        ],
        feature_weights: vec![("close".to_string(), 1.0)],
        timeframe_entropy: 1.0,
        force_observe: true,
    }
}

#[test]
fn force_observe_downgrades_aggressive_fill_to_passive() {
    let features = features(0.85);
    let overlay = flat_overlay();
    let posterior = posterior();
    let trace = uniform_trace();
    let input = ExecutionTreeInput {
        execution_features: &features,
        physics_overlay: &overlay,
        hmm_posterior: &posterior,
        mece_recovery_confidence: Some(0.97),
        prediction_vote_score: 0.80,
        market_state_lineage: None,
        path_ranker_lineage: None,
        axial_trace: Some(&trace),
    };
    let output = DefaultExecutionTreeScorer.score(&input).unwrap();
    assert_eq!(output.branch, "fill_viable", "branch itself unchanged");
    assert_eq!(
        output.execution_bias, "passive",
        "aggressive downgraded because axial entropy is uniform"
    );
    assert_eq!(
        output.decision_hint,
        "execution_observe_due_to_axial_entropy"
    );
    assert!(
        !output.axial_attention_trace.is_empty(),
        "trace must persist top-k weights"
    );
}

#[test]
fn concentrated_trace_leaves_aggressive_fill_intact() {
    let features = features(0.85);
    let overlay = flat_overlay();
    let posterior = posterior();
    let trace = concentrated_trace();
    let input = ExecutionTreeInput {
        execution_features: &features,
        physics_overlay: &overlay,
        hmm_posterior: &posterior,
        mece_recovery_confidence: Some(0.97),
        prediction_vote_score: 0.80,
        market_state_lineage: None,
        path_ranker_lineage: None,
        axial_trace: Some(&trace),
    };
    let output = DefaultExecutionTreeScorer.score(&input).unwrap();
    assert_eq!(output.branch, "fill_viable");
    assert_eq!(output.execution_bias, "aggressive");
    assert_eq!(output.decision_hint, "execution_first_fill");
    assert_eq!(output.axial_attention_trace[0].0, "ltf");
}

#[test]
fn absent_trace_preserves_legacy_behavior() {
    let features = features(0.85);
    let overlay = flat_overlay();
    let posterior = posterior();
    let input = ExecutionTreeInput {
        execution_features: &features,
        physics_overlay: &overlay,
        hmm_posterior: &posterior,
        mece_recovery_confidence: Some(0.97),
        prediction_vote_score: 0.80,
        market_state_lineage: None,
        path_ranker_lineage: None,
        axial_trace: None,
    };
    let output = DefaultExecutionTreeScorer.score(&input).unwrap();
    assert_eq!(output.execution_bias, "aggressive");
    assert!(output.axial_attention_trace.is_empty());
}

#[test]
fn force_observe_does_not_resurrect_blocked_executions() {
    // Weak readiness → already blocked. Axial force_observe must not flip
    // blocked back into observe/passive.
    let features = features(0.20);
    let overlay = flat_overlay();
    let posterior = posterior();
    let trace = uniform_trace();
    let input = ExecutionTreeInput {
        execution_features: &features,
        physics_overlay: &overlay,
        hmm_posterior: &posterior,
        mece_recovery_confidence: Some(0.97),
        prediction_vote_score: 0.80,
        market_state_lineage: None,
        path_ranker_lineage: None,
        axial_trace: Some(&trace),
    };
    let output = DefaultExecutionTreeScorer.score(&input).unwrap();
    assert_eq!(output.gate_status, "blocked");
    assert_eq!(output.execution_bias, "skip");
}
