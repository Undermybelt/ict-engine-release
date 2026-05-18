use serde::Serialize;

use super::{
    build_postmortem_artifact, build_prior_artifact, PostmortemArtifact, PostmortemArtifactInput,
    PriorArtifact, PriorArtifactInput,
};
use crate::application::orchestration::ExecutionShapAttribution;
use crate::pda_sequence::{summarize_pda_sequence_artifact, PdaSequenceAnalysisArtifact};

#[derive(Debug, Clone, Serialize, Default)]
pub struct ReflectionBundle {
    pub prior: PriorArtifact,
    pub postmortem: PostmortemArtifact,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_edge_share: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prediction_edge_share: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_readiness: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub why_execution_dominates: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub why_prediction_is_demoted: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prediction_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ensemble_vote_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ensemble_vote_artifact_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ensemble_disagreement_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compare_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_setup_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_setup_guardrail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pda_sequence_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pda_sequence_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pda_cluster_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pda_cluster_confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub execution_shap_top_k: Vec<ExecutionShapAttribution>,
}

#[derive(Debug, Clone, Default)]
pub struct ReflectionBundleInput {
    pub symbol: String,
    pub timestamp: String,
    pub objective: String,
    pub expected_regime: String,
    pub expected_direction: String,
    pub realized_outcome: String,
    pub evidence: Vec<String>,
    pub next_candidates: Vec<String>,
}

pub fn build_reflection_bundle(input: ReflectionBundleInput) -> ReflectionBundle {
    let symbol = input.symbol;
    let timestamp = input.timestamp;
    let objective = input.objective;
    let expected_regime = input.expected_regime;
    let expected_direction = input.expected_direction;
    let realized_outcome = input.realized_outcome;
    let evidence = input.evidence;
    let next_candidates = input.next_candidates;

    ReflectionBundle {
        prior: build_prior_artifact(PriorArtifactInput {
            symbol: symbol.clone(),
            timestamp: timestamp.clone(),
            objective,
            expected_regime,
            expected_direction,
            expected_key_evidence: evidence.clone(),
            invalidation_conditions: next_candidates.clone(),
            freshness_expectation: "fresh_required".to_string(),
            notes: next_candidates.clone(),
        }),
        postmortem: build_postmortem_artifact(PostmortemArtifactInput {
            symbol,
            timestamp,
            expected_outcome: "unknown_expected_outcome".to_string(),
            realized_outcome,
            deviations: next_candidates.clone(),
            evidence_drift: evidence.clone(),
            what_worked: evidence,
            what_failed: next_candidates.clone(),
            next_candidates,
        }),
        execution_edge_share: None,
        prediction_edge_share: None,
        execution_readiness: None,
        why_execution_dominates: None,
        why_prediction_is_demoted: None,
        execution_summary: None,
        prediction_summary: None,
        ensemble_vote_summary: None,
        ensemble_vote_artifact_id: None,
        ensemble_disagreement_summary: None,
        compare_summary: None,
        execution_setup_summary: None,
        execution_setup_guardrail: None,
        pda_sequence_summary: None,
        pda_sequence_method: None,
        pda_cluster_label: None,
        pda_cluster_confidence: None,
        execution_shap_top_k: Vec::new(),
    }
}

pub fn apply_pda_sequence_artifact_to_reflection_bundle(
    bundle: &mut ReflectionBundle,
    artifact: &PdaSequenceAnalysisArtifact,
) {
    let summary = summarize_pda_sequence_artifact(artifact);
    bundle.pda_sequence_method = Some(summary.method.clone());
    bundle.pda_cluster_label = summary.primary_cluster_label.clone();
    bundle.pda_cluster_confidence = summary.primary_cluster_confidence;
    bundle.pda_sequence_summary = Some(format!(
        "pda_sequence method={} primary_cluster={} confidence={:.3} consistency={:.3} ensemble_mean_confidence={:.3} valid_sessions={} kmer_k={}",
        summary.method,
        summary
            .primary_cluster_label
            .unwrap_or_else(|| "unknown".to_string()),
        summary.primary_cluster_confidence.unwrap_or_default(),
        summary.consistency_ratio,
        summary.ensemble_mean_confidence,
        summary.valid_sessions,
        summary.kmer_k,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pda_sequence::PdaClusteringPacket;
    use crate::state::RunProvenance;

    #[test]
    fn reflection_bundle_contains_prior_and_postmortem() {
        let bundle = build_reflection_bundle(ReflectionBundleInput {
            symbol: "NQ".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            objective: "generic".to_string(),
            expected_regime: "bull".to_string(),
            expected_direction: "long".to_string(),
            realized_outcome: "win".to_string(),
            evidence: vec!["e1".to_string()],
            next_candidates: vec!["n1".to_string()],
        });
        assert_eq!(bundle.prior.symbol, "NQ");
        assert_eq!(bundle.postmortem.realized_outcome, "win");
        assert!(bundle.execution_edge_share.is_none());
        assert!(bundle.execution_readiness.is_none());
        assert!(bundle.why_execution_dominates.is_none());
        assert!(bundle.why_prediction_is_demoted.is_none());
        assert!(bundle.compare_summary.is_none());
    }

    #[test]
    fn reflection_bundle_can_include_pda_sequence_summary() {
        let mut bundle = build_reflection_bundle(ReflectionBundleInput {
            symbol: "NQ".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            objective: "generic".to_string(),
            expected_regime: "bull".to_string(),
            expected_direction: "long".to_string(),
            realized_outcome: "win".to_string(),
            evidence: vec!["e1".to_string()],
            next_candidates: vec!["n1".to_string()],
        });
        let artifact = PdaSequenceAnalysisArtifact {
            artifact_id: "pda-sequence-NQ-1".to_string(),
            generated_at: chrono::Utc::now(),
            symbol: "NQ".to_string(),
            method: "pda_sequence_analysis_v2".to_string(),
            k: 2,
            n_states: 3,
            kmer_k: 2,
            total_sessions: 8,
            valid_sessions: 8,
            silhouette_score: 0.6,
            consistency_ratio: 0.75,
            ensemble_mean_confidence: 0.83,
            dtw_packets: Vec::new(),
            hmm_classifications: Vec::new(),
            fcgr_labels: vec![0, 0],
            ensemble_packets: vec![PdaClusteringPacket {
                method: "pda_ensemble_majority_v1".to_string(),
                primary_cluster: 1,
                confidence: 1.0,
                vote_distribution: vec![0, 3],
                votes: [1, 1, 1],
                voter_names: ["dtw".to_string(), "hmm".to_string(), "fcgr".to_string()],
            }],
            provenance: RunProvenance::default(),
        };

        apply_pda_sequence_artifact_to_reflection_bundle(&mut bundle, &artifact);
        assert_eq!(
            bundle.pda_sequence_method.as_deref(),
            Some("pda_sequence_analysis_v2")
        );
        assert_eq!(bundle.pda_cluster_label.as_deref(), Some("cluster_1"));
        assert!(bundle
            .pda_sequence_summary
            .as_deref()
            .unwrap_or_default()
            .contains("consistency=0.750"));
    }
}
