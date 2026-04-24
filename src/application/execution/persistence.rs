use std::path::Path;

use anyhow::Result;

use crate::domain::execution::{
    classify_execution_gate, ExecutionArtifact, EXECUTION_GATE_OBSERVE, EXECUTION_GATE_READY,
};
use crate::state::{
    append_artifact_ledger_entry, artifact_state_path, save_state, ArtifactLedgerEntry,
};

pub const EXECUTION_ARTIFACT_FILE: &str = "execution_artifact.json";
/// Bumped to 2 when ExecutionFeatures gained spectral_metrics and the
/// dominant_cycle_energy / cycle_phase_alignment / spectral_entropy scalars.
/// Readers of old v1 entries must tolerate missing spectral fields.
pub const EXECUTION_ARTIFACT_LEDGER_VERSION: usize = 2;
pub const EXECUTION_ARTIFACT_REVIEW_RULE_VERSION: &str = "execution-artifact-v2";

pub fn persist_execution_artifact<P: AsRef<Path>>(
    dir: P,
    artifact: &ExecutionArtifact,
    source_phase: &str,
    source_run_id: Option<String>,
    decision_hint: &str,
) -> Result<()> {
    save_state(&dir, &artifact.symbol, EXECUTION_ARTIFACT_FILE, artifact)?;
    let spectral_summary = artifact
        .features
        .spectral_metrics
        .as_ref()
        .map(|metrics| {
            format!(
                ";spectral_entropy={:.3};dominant_cycle_energy={:.3}",
                metrics.spectral_entropy, metrics.dominant_cycle_energy,
            )
        })
        .unwrap_or_default();
    append_artifact_ledger_entry(
        &dir,
        &artifact.symbol,
        ArtifactLedgerEntry {
            entry_id: format!("ledger:{}", artifact.artifact_id),
            artifact_kind: "execution_artifact".to_string(),
            artifact_id: artifact.artifact_id.clone(),
            version: EXECUTION_ARTIFACT_LEDGER_VERSION,
            generated_at: artifact.generated_at,
            symbol: artifact.symbol.clone(),
            source_phase: source_phase.to_string(),
            source_run_id,
            path: artifact_state_path(&dir, &artifact.symbol, EXECUTION_ARTIFACT_FILE),
            status: classify_execution_gate(artifact.features.execution_readiness).to_string(),
            promote_candidate: artifact.features.execution_readiness >= EXECUTION_GATE_READY,
            actionable: artifact.features.execution_readiness >= EXECUTION_GATE_OBSERVE,
            decision_hint: decision_hint.to_string(),
            review_reason: format!(
                "execution_edge_share={:.3};prediction_edge_share={:.3}{}",
                artifact.features.execution_edge_share,
                artifact.features.prediction_edge_share,
                spectral_summary,
            ),
            review_rule_version: EXECUTION_ARTIFACT_REVIEW_RULE_VERSION.to_string(),
            top_factor_name: None,
            top_factor_action: None,
            family_scores: std::collections::BTreeMap::from([(
                "execution".to_string(),
                artifact.features.execution_score,
            )]),
            supersedes_artifact_id: None,
            quality_score: (artifact.features.execution_readiness * 100.0).round() as i32,
            consumed_by_update_run_id: None,
            consumed_at: None,
            consumed_outcome: None,
            regraded_at: None,
            consumption_regrade_status: None,
            consumption_regrade_reason: None,
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::execution::{ExecutionArtifact, ExecutionFeatures};
    use crate::state::RunProvenance;
    use chrono::Utc;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn persist_writes_artifact_and_ledger_entry_with_selected_state_dir_path() {
        let artifact = ExecutionArtifact {
            artifact_id: "execution-artifact-NQ-test".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            features: ExecutionFeatures {
                execution_score: 0.70,
                execution_readiness: 0.60,
                execution_edge_share: 0.55,
                prediction_edge_share: 0.45,
                ..ExecutionFeatures::default()
            },
            hard_gate_status: "observe".to_string(),
            provenance: RunProvenance::default(),
        };
        let dir = TempDir::new().unwrap();

        persist_execution_artifact(dir.path(), &artifact, "analyze", None, "test").unwrap();

        let artifact_path = dir.path().join("NQ").join(EXECUTION_ARTIFACT_FILE);
        assert!(artifact_path.exists(), "artifact file not written");
        let ledger = fs::read_to_string(
            dir.path()
                .join("NQ")
                .join(crate::state::ARTIFACT_LEDGER_FILE),
        )
        .unwrap();
        let entries: Vec<ArtifactLedgerEntry> = serde_json::from_str(&ledger).unwrap();
        assert_eq!(
            entries[0].path,
            artifact_path.to_string_lossy(),
            "ledger path must point at the selected state_dir artifact"
        );
    }
}
