use super::ReflectionBundle;
use crate::application::orchestration::{
    ExecutionShapProvider, ExecutionTreeInput, ExecutionTreeOutput, StructuralExecutionShap,
};

/// Populate `reflection_bundle.execution_shap_top_k` using the provided
/// `ExecutionShapProvider`. Default to `StructuralExecutionShap { top_k: 5 }`
/// via `apply_default_execution_tree_shap_to_reflection_bundle`.
pub fn apply_execution_tree_shap_to_reflection_bundle<P: ExecutionShapProvider>(
    bundle: &mut ReflectionBundle,
    input: &ExecutionTreeInput<'_>,
    output: &ExecutionTreeOutput,
    provider: &P,
) {
    bundle.execution_shap_top_k = provider.attributions(input, output);
}

pub fn apply_default_execution_tree_shap_to_reflection_bundle(
    bundle: &mut ReflectionBundle,
    input: &ExecutionTreeInput<'_>,
    output: &ExecutionTreeOutput,
) {
    apply_execution_tree_shap_to_reflection_bundle(
        bundle,
        input,
        output,
        &StructuralExecutionShap::default(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::execution::ExecutionPhysicsOverlay;
    use crate::application::reflection::ReflectionBundle;
    use crate::domain::execution::ExecutionFeatures;
    use crate::domain::regime::IsingState;
    use crate::ict::PythagoreanExtensionMetrics;
    use crate::types::RegimeProbs;

    fn sample_output() -> ExecutionTreeOutput {
        ExecutionTreeOutput {
            execution_score: 0.82,
            branch: "fill_viable".to_string(),
            execution_bias: "aggressive".to_string(),
            gate_status: "ready".to_string(),
            branch_probability: 0.6,
            posterior_uncertainty: 0.4,
            split_reason_lineage: vec!["ready".to_string()],
            decision_hint: "execution_first_fill".to_string(),
            axial_attention_trace: Vec::new(),
            ..ExecutionTreeOutput::default()
        }
    }

    fn sample_overlay() -> ExecutionPhysicsOverlay {
        ExecutionPhysicsOverlay {
            ou: None,
            ising: Some(IsingState {
                magnetization: 0.1,
                coupling_strength: 0.2,
                phase_transition_risk: 0.3,
                herding_bias: 0.1,
            }),
            pythagorean: Some(PythagoreanExtensionMetrics {
                trendline_distance: 0.0,
                orthogonal_extension: 0.0,
                normalized_overstretch: 0.4,
            }),
            spectral: None,
        }
    }

    fn sample_features(readiness: f64) -> ExecutionFeatures {
        ExecutionFeatures {
            execution_readiness: readiness,
            execution_score: readiness,
            evidence_quality: 0.7,
            ..Default::default()
        }
    }

    #[test]
    fn populates_reflection_bundle_with_shap_top_k() {
        let overlay = sample_overlay();
        let features = sample_features(0.82);
        let posterior = RegimeProbs {
            accumulation: 0.33,
            manipulation_expansion: 0.34,
            distribution: 0.33,
        };
        let input = ExecutionTreeInput {
            execution_features: &features,
            physics_overlay: &overlay,
            hmm_posterior: &posterior,
            mece_recovery_confidence: Some(0.97),
            prediction_vote_score: 0.72,
            market_state_lineage: None,
            path_ranker_lineage: None,
            axial_trace: None,
        };
        let output = sample_output();

        let mut bundle = ReflectionBundle::default();
        apply_default_execution_tree_shap_to_reflection_bundle(&mut bundle, &input, &output);

        assert!(!bundle.execution_shap_top_k.is_empty());
        assert!(bundle.execution_shap_top_k.len() <= 5);
        // Stability: rerun gives identical vector.
        let mut second = ReflectionBundle::default();
        apply_default_execution_tree_shap_to_reflection_bundle(&mut second, &input, &output);
        assert_eq!(bundle.execution_shap_top_k, second.execution_shap_top_k);
    }

    #[test]
    fn empty_shap_vec_serializes_to_absent_field() {
        let bundle = ReflectionBundle::default();
        let json = serde_json::to_string(&bundle).unwrap();
        assert!(
            !json.contains("execution_shap_top_k"),
            "empty execution_shap_top_k must skip serialization to preserve backward-compat JSON: {json}"
        );
    }

    #[test]
    fn populated_shap_vec_serializes_as_array() {
        let overlay = sample_overlay();
        let features = sample_features(0.82);
        let posterior = RegimeProbs {
            accumulation: 0.33,
            manipulation_expansion: 0.34,
            distribution: 0.33,
        };
        let input = ExecutionTreeInput {
            execution_features: &features,
            physics_overlay: &overlay,
            hmm_posterior: &posterior,
            mece_recovery_confidence: Some(0.97),
            prediction_vote_score: 0.72,
            market_state_lineage: None,
            path_ranker_lineage: None,
            axial_trace: None,
        };
        let output = sample_output();

        let mut bundle = ReflectionBundle::default();
        apply_default_execution_tree_shap_to_reflection_bundle(&mut bundle, &input, &output);

        let json = serde_json::to_string(&bundle).unwrap();
        assert!(json.contains("\"execution_shap_top_k\""));
        assert!(json.contains("\"feature\""));
        assert!(json.contains("\"contribution\""));
    }
}
