//! Persistence for the factor × regime × timeframe Tucker core artifact.
//!
//! New `factor_tucker_core` artifact kind — writes a per-symbol
//! `factor_tucker_core.json` and an `artifact_ledger.json` entry with
//! `review_reason` that summarises the decomposition (rank triplet +
//! reconstruction error). Kept in the factor_lab crate so the caller that
//! owns the tensor also owns the write path.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::factor_lab::TuckerCore;
use crate::state::{
    append_artifact_ledger_entry, artifact_state_path, save_state, ArtifactLedgerEntry,
    RunProvenance,
};

pub const FACTOR_TUCKER_CORE_ARTIFACT_FILE: &str = "factor_tucker_core.json";
pub const FACTOR_TUCKER_CORE_ARTIFACT_KIND: &str = "factor_tucker_core";
pub const FACTOR_TUCKER_CORE_LEDGER_VERSION: usize = 1;
pub const FACTOR_TUCKER_CORE_REVIEW_RULE_VERSION: &str = "factor-tucker-core-artifact-v1";

/// Upper bound on reconstruction_error before attribution confidence is
/// downgraded. 0.30 = "core explains >=70% of the tensor Frobenius norm."
/// Below this the tucker core is informative; above this the SHAP consumer
/// must widen its top_k (per plan §2.3).
pub const TUCKER_ATTRIBUTION_CONFIDENCE_CAP: f64 = 0.30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorTuckerCoreArtifact {
    pub artifact_id: String,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub tucker: TuckerCore,
    pub factor_labels: Vec<String>,
    pub regime_labels: Vec<String>,
    pub timeframe_labels: Vec<String>,
    pub provenance: RunProvenance,
}

pub fn build_factor_tucker_core_artifact(
    symbol: &str,
    tucker: TuckerCore,
    factor_labels: Vec<String>,
    regime_labels: Vec<String>,
    timeframe_labels: Vec<String>,
    provenance: RunProvenance,
) -> FactorTuckerCoreArtifact {
    let generated_at = Utc::now();
    FactorTuckerCoreArtifact {
        artifact_id: format!(
            "factor-tucker-{}-{}",
            symbol,
            generated_at.timestamp_millis()
        ),
        generated_at,
        symbol: symbol.to_string(),
        tucker,
        factor_labels,
        regime_labels,
        timeframe_labels,
        provenance,
    }
}

/// True when the reconstruction error is low enough for SHAP attribution to
/// ride on the Tucker core without defensive widening. Consumed by the
/// reflection bundle when ranking SHAP features (plan §2.3).
pub fn tucker_attribution_confidence_is_high(error: f64) -> bool {
    error.is_finite() && error <= TUCKER_ATTRIBUTION_CONFIDENCE_CAP
}

pub fn persist_factor_tucker_core_artifact<P: AsRef<Path>>(
    dir: P,
    artifact: &FactorTuckerCoreArtifact,
    source_phase: &str,
    source_run_id: Option<String>,
    decision_hint: &str,
) -> Result<()> {
    save_state(
        &dir,
        &artifact.symbol,
        FACTOR_TUCKER_CORE_ARTIFACT_FILE,
        artifact,
    )?;
    let confidence = if tucker_attribution_confidence_is_high(artifact.tucker.reconstruction_error)
    {
        "high"
    } else {
        "low"
    };
    append_artifact_ledger_entry(
        &dir,
        &artifact.symbol,
        ArtifactLedgerEntry {
            entry_id: format!("ledger:{}", artifact.artifact_id),
            artifact_kind: FACTOR_TUCKER_CORE_ARTIFACT_KIND.to_string(),
            artifact_id: artifact.artifact_id.clone(),
            version: FACTOR_TUCKER_CORE_LEDGER_VERSION,
            generated_at: artifact.generated_at,
            symbol: artifact.symbol.clone(),
            source_phase: source_phase.to_string(),
            source_run_id,
            path: artifact_state_path(&dir, &artifact.symbol, FACTOR_TUCKER_CORE_ARTIFACT_FILE),
            status: confidence.to_string(),
            // Tucker core is a lineage artifact, not a gate — never promote by
            // itself. The SHAP consumer reads attribution_confidence from the
            // status field to decide whether to widen top_k.
            promote_candidate: false,
            actionable: false,
            decision_hint: decision_hint.to_string(),
            review_reason: format!(
                "rank=({},{},{});input_shape=({},{},{});reconstruction_error={:.4};attribution_confidence={}",
                artifact.tucker.rank_triplet.0,
                artifact.tucker.rank_triplet.1,
                artifact.tucker.rank_triplet.2,
                artifact.tucker.input_shape.0,
                artifact.tucker.input_shape.1,
                artifact.tucker.input_shape.2,
                artifact.tucker.reconstruction_error,
                confidence,
            ),
            review_rule_version: FACTOR_TUCKER_CORE_REVIEW_RULE_VERSION.to_string(),
            top_factor_name: artifact.factor_labels.first().cloned(),
            top_factor_action: None,
            family_scores: BTreeMap::from([(
                "reconstruction_error".to_string(),
                artifact.tucker.reconstruction_error,
            )]),
            supersedes_artifact_id: None,
            quality_score: ((1.0 - artifact.tucker.reconstruction_error.min(1.0)) * 100.0).round()
                as i32,
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
    use crate::factor_lab::fit_tucker_core;
    use ndarray::Array3;
    use std::fs;
    use tempfile::TempDir;

    fn sample_tucker() -> TuckerCore {
        // Low-rank tensor so reconstruction_error stays small.
        let tensor = Array3::<f64>::from_shape_fn((3, 2, 2), |(i, j, k)| {
            ((i + 1) * (j + 1) * (k + 1)) as f64
        });
        fit_tucker_core(&tensor, (1, 1, 1)).expect("rank-1 fit")
    }

    #[test]
    fn attribution_confidence_follows_reconstruction_error() {
        assert!(tucker_attribution_confidence_is_high(0.05));
        assert!(tucker_attribution_confidence_is_high(
            TUCKER_ATTRIBUTION_CONFIDENCE_CAP
        ));
        assert!(!tucker_attribution_confidence_is_high(0.40));
        assert!(!tucker_attribution_confidence_is_high(f64::NAN));
    }

    #[test]
    fn persist_writes_artifact_and_ledger_entry() {
        let tucker = sample_tucker();
        let artifact = build_factor_tucker_core_artifact(
            "NQ",
            tucker,
            vec!["f0".to_string(), "f1".to_string(), "f2".to_string()],
            vec!["r0".to_string(), "r1".to_string()],
            vec!["t0".to_string(), "t1".to_string()],
            RunProvenance::default(),
        );
        let dir = TempDir::new().unwrap();
        persist_factor_tucker_core_artifact(dir.path(), &artifact, "analyze", None, "test")
            .unwrap();

        let artifact_path = dir.path().join("NQ").join(FACTOR_TUCKER_CORE_ARTIFACT_FILE);
        assert!(artifact_path.exists(), "artifact file not written");
        let raw = fs::read_to_string(&artifact_path).unwrap();
        assert!(raw.contains("\"tucker\""));
        assert!(raw.contains("\"factor_labels\""));

        let ledger = fs::read_to_string(
            dir.path()
                .join("NQ")
                .join(crate::state::ARTIFACT_LEDGER_FILE),
        )
        .unwrap();
        assert!(ledger.contains("factor_tucker_core"));
        assert!(ledger.contains("factor-tucker-core-artifact-v1"));
        assert!(ledger.contains("reconstruction_error"));
        // Lineage artifact: never promotes or actionable.
        assert!(ledger.contains("\"promote_candidate\": false"));
        assert!(ledger.contains("\"actionable\": false"));
        let entries: Vec<ArtifactLedgerEntry> = serde_json::from_str(&ledger).unwrap();
        assert_eq!(
            entries[0].path,
            artifact_path.to_string_lossy(),
            "ledger path must point at the selected state_dir artifact"
        );
    }
}
