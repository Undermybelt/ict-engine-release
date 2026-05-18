use ict_engine::application::execution::ExecutionPhysicsOverlay;
use ict_engine::application::orchestration::{
    DefaultExecutionTreeScorer, ExecutionTreeInput, ExecutionTreeScorer,
};
use ict_engine::domain::execution::ExecutionFeatures;
use ict_engine::domain::regime::IsingState;
use ict_engine::ict::PythagoreanExtensionMetrics;
use ict_engine::types::RegimeProbs;

/// Inert physics overlay so we isolate the prediction × execution dimension.
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

fn neutral_posterior() -> RegimeProbs {
    RegimeProbs {
        accumulation: 0.34,
        manipulation_expansion: 0.33,
        distribution: 0.33,
    }
}

fn features(readiness: f64) -> ExecutionFeatures {
    ExecutionFeatures {
        execution_readiness: readiness,
        execution_score: readiness,
        evidence_quality: 0.6,
        ..Default::default()
    }
}

fn score(
    prediction_score: f64,
    execution_readiness: f64,
) -> ict_engine::application::orchestration::ExecutionTreeOutput {
    let features = features(execution_readiness);
    let overlay = flat_overlay();
    let posterior = neutral_posterior();
    let input = ExecutionTreeInput {
        execution_features: &features,
        physics_overlay: &overlay,
        hmm_posterior: &posterior,
        mece_recovery_confidence: Some(0.96),
        prediction_vote_score: prediction_score,
        market_state_lineage: None,
        path_ranker_lineage: None,
        axial_trace: None,
    };
    DefaultExecutionTreeScorer
        .score(&input)
        .expect("scorer must succeed")
}

#[test]
fn strong_prediction_strong_execution_fills_aggressively() {
    let output = score(0.85, 0.85);
    assert_eq!(output.gate_status, "ready");
    assert_eq!(output.branch, "fill_viable");
    assert_eq!(output.execution_bias, "aggressive");
    assert_eq!(output.decision_hint, "execution_first_fill");
}

#[test]
fn strong_prediction_weak_execution_blocks() {
    let output = score(0.85, 0.30);
    assert_eq!(output.gate_status, "blocked");
    assert_eq!(output.branch, "block_crowded");
    assert_eq!(output.execution_bias, "skip");
    assert_eq!(
        output.decision_hint, "execution_blocked_regardless_of_prediction",
        "execution-first principle: strong prediction must NOT override weak execution"
    );
}

#[test]
fn weak_prediction_strong_execution_still_fills() {
    let output = score(0.20, 0.85);
    assert_eq!(output.gate_status, "ready");
    assert_eq!(output.branch, "fill_viable");
    assert_eq!(output.execution_bias, "aggressive");
    assert_eq!(
        output.decision_hint, "execution_first_fill_despite_weak_prediction",
        "execution-first principle: strong execution actionable even with weak prediction"
    );
}

#[test]
fn weak_prediction_weak_execution_blocks() {
    let output = score(0.20, 0.30);
    assert_eq!(output.gate_status, "blocked");
    assert_eq!(output.branch, "block_crowded");
    assert_eq!(output.execution_bias, "skip");
    assert_eq!(
        output.decision_hint,
        "execution_blocked_regardless_of_prediction"
    );
}

#[test]
fn medium_prediction_strong_execution_fills() {
    let output = score(0.50, 0.85);
    assert_eq!(output.gate_status, "ready");
    assert_eq!(output.branch, "fill_viable");
    assert_eq!(output.execution_bias, "aggressive");
    assert_eq!(
        output.decision_hint,
        "execution_first_fill_with_medium_prediction"
    );
}

#[test]
fn strong_prediction_medium_execution_observes() {
    let output = score(0.85, 0.55);
    assert_eq!(output.gate_status, "observe");
    assert_eq!(output.execution_bias, "passive");
    assert_eq!(
        output.decision_hint, "execution_observe_with_strong_prediction",
        "medium execution → never aggressive, even with strong prediction"
    );
}
