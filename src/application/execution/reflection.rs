use crate::application::reflection::ReflectionBundle;
use crate::domain::execution::ExecutionArtifact;

fn build_execution_explanation(artifact: &ExecutionArtifact) -> (Option<String>, Option<String>) {
    if artifact.features.execution_edge_share <= artifact.features.prediction_edge_share {
        return (None, None);
    }

    let mut dominant_reasons = vec![
        format!("execution_score={:.3}", artifact.features.execution_score),
        format!("evidence_quality={:.3}", artifact.features.evidence_quality),
    ];
    if let Some(distance) = artifact.features.overextension_distance {
        dominant_reasons.push(format!("overextension_distance={distance:.3}"));
    }
    if let Some(speed) = artifact.features.reversion_speed {
        dominant_reasons.push(format!("reversion_speed={speed:.3}"));
    }
    // Spectral evidence lands in why_execution_dominates when the series has
    // rhythmic structure (dominant mode + bounded entropy). The chaotic path
    // is already reflected in execution_readiness via the spectral penalty,
    // so we don't duplicate that failure mode here.
    if let Some(energy) = artifact.features.dominant_cycle_energy {
        dominant_reasons.push(format!("dominant_cycle_energy={energy:.3}"));
    }
    if let Some(alignment) = artifact.features.cycle_phase_alignment {
        dominant_reasons.push(format!("cycle_phase_alignment={alignment:.3}"));
    }
    if let Some(entropy) = artifact.features.spectral_entropy {
        dominant_reasons.push(format!("spectral_entropy={entropy:.3}"));
    }

    (
        Some(format!(
            "execution dominates because {}",
            dominant_reasons.join(", ")
        )),
        Some(format!(
            "prediction is demoted because prediction_score={:.3} trails execution_edge={:.3} vs prediction_edge={:.3}",
            artifact.features.prediction_score,
            artifact.features.execution_edge_share,
            artifact.features.prediction_edge_share
        )),
    )
}

pub fn apply_execution_artifact_to_reflection_bundle(
    bundle: &mut ReflectionBundle,
    artifact: &ExecutionArtifact,
) {
    let (why_execution_dominates, why_prediction_is_demoted) =
        build_execution_explanation(artifact);
    bundle.execution_edge_share = Some(artifact.features.execution_edge_share);
    bundle.prediction_edge_share = Some(artifact.features.prediction_edge_share);
    bundle.execution_readiness = Some(artifact.features.execution_readiness);
    bundle.why_execution_dominates = why_execution_dominates;
    bundle.why_prediction_is_demoted = why_prediction_is_demoted;
    bundle.execution_summary = Some(format!(
        "execution_edge={:.3}; readiness={:.3}; gate={}",
        artifact.features.execution_edge_share,
        artifact.features.execution_readiness,
        artifact.hard_gate_status
    ));
    bundle.prediction_summary = Some(format!(
        "prediction_edge={:.3}; prediction_score={:.3}",
        artifact.features.prediction_edge_share, artifact.features.prediction_score,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    use crate::domain::execution::ExecutionFeatures;
    use crate::state::RunProvenance;

    fn artifact(execution_edge_share: f64, prediction_edge_share: f64) -> ExecutionArtifact {
        ExecutionArtifact {
            artifact_id: "execution:test".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            features: ExecutionFeatures {
                execution_score: 0.75,
                prediction_score: 0.42,
                execution_edge_share,
                prediction_edge_share,
                execution_readiness: 0.81,
                aggression_bias: 0.2,
                completion_pressure: 0.7,
                liquidity_absorption_bias: 0.6,
                evidence_quality: 0.88,
                overextension_distance: Some(0.14),
                reversion_speed: Some(0.33),
                ..ExecutionFeatures::default()
            },
            hard_gate_status: "execution_ready".to_string(),
            provenance: RunProvenance::default(),
        }
    }

    #[test]
    fn adds_explanations_when_execution_dominates() {
        let mut bundle = ReflectionBundle::default();
        apply_execution_artifact_to_reflection_bundle(&mut bundle, &artifact(0.70, 0.30));

        assert!(bundle.why_execution_dominates.is_some());
        assert!(bundle.why_prediction_is_demoted.is_some());
    }

    #[test]
    fn omits_explanations_when_prediction_dominates() {
        let mut bundle = ReflectionBundle::default();
        apply_execution_artifact_to_reflection_bundle(&mut bundle, &artifact(0.35, 65.0 / 100.0));

        assert!(bundle.why_execution_dominates.is_none());
        assert!(bundle.why_prediction_is_demoted.is_none());
    }

    #[test]
    fn includes_spectral_evidence_when_execution_dominates() {
        let mut art = artifact(0.72, 0.28);
        art.features.dominant_cycle_energy = Some(0.88);
        art.features.cycle_phase_alignment = Some(0.55);
        art.features.spectral_entropy = Some(0.12);
        let mut bundle = ReflectionBundle::default();
        apply_execution_artifact_to_reflection_bundle(&mut bundle, &art);
        let why = bundle.why_execution_dominates.expect("execution dominates");
        assert!(why.contains("dominant_cycle_energy"));
        assert!(why.contains("cycle_phase_alignment"));
        assert!(why.contains("spectral_entropy"));
    }
}
