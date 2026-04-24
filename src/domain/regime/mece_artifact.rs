use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::regime::{classify_mece_recovery_segments_gate, RolloutSegment};
use crate::factor_lab::sparsity_ratio_within_bounds;
use crate::state::RunProvenance;

/// Hard-gate threshold used by `classify_mece_recovery_gate` and the artifact
/// ledger. Sprint 3 acceptance condition: an MECE recovery report is only
/// allowed to promote downstream artifacts when accuracy >= this value.
pub const MECE_RECOVERY_ACCURACY_GATE: f64 = 0.95;

/// Persistent record of an MECE recovery run. Carries the accuracy / macro_f1
/// pair, the selected factor subset, and stable hashes of the underlying HMM
/// Viterbi path + label sequence so reruns can be diffed bit-for-bit. The
/// `execution_validity_summary` line preserves the dual constraint
/// (regime recovery only counts when execution coverage is non-degenerate)
/// directly inside the ledger row.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MeceRecoveryArtifact {
    pub artifact_id: String,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub accuracy: f64,
    pub macro_f1: f64,
    pub selected_factors: Vec<String>,
    pub hmm_viterbi_hash: String,
    pub label_hash: String,
    pub execution_validity_summary: String,
    /// Softshrink kept/total ratio of the best factor set. See sparse.rs.
    #[serde(default)]
    pub sparsity_ratio: f64,
    /// (factor, pre-shrink weight) pairs for factors pruned from the best
    /// set. Mirrors `MeceRecoveryReport::pruned_factor_trail`.
    #[serde(default)]
    pub pruned_factor_trail: Vec<(String, f64)>,
    /// Rollout segments: short / medium / long. Empty when series too short.
    #[serde(default)]
    pub segments: Vec<RolloutSegment>,
    pub provenance: RunProvenance,
}

pub fn classify_mece_recovery_gate(accuracy: f64) -> &'static str {
    if accuracy >= MECE_RECOVERY_ACCURACY_GATE {
        "promote"
    } else {
        "blocked"
    }
}

/// Combined hard gate: promote only when *every* sub-gate passes.
/// - accuracy meets `MECE_RECOVERY_ACCURACY_GATE`
/// - sparsity_ratio sits in the healthy band
/// - rollout segments gate promotes (or is empty → skipped for small series)
///
/// Any failure short-circuits to `"blocked"`. Returns a stable `&'static str`
/// so the ledger row serialization stays byte-stable.
pub fn classify_mece_recovery_combined_gate(artifact: &MeceRecoveryArtifact) -> &'static str {
    if classify_mece_recovery_gate(artifact.accuracy) == "blocked" {
        return "blocked";
    }
    if !sparsity_ratio_within_bounds(artifact.sparsity_ratio) {
        return "blocked";
    }
    if !artifact.segments.is_empty()
        && classify_mece_recovery_segments_gate(&artifact.segments) == "blocked"
    {
        return "blocked";
    }
    "promote"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_artifact() -> MeceRecoveryArtifact {
        MeceRecoveryArtifact {
            artifact_id: "mece-recovery-NQ-test".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            accuracy: 0.97,
            macro_f1: 0.92,
            selected_factors: vec!["structure_ict".to_string()],
            hmm_viterbi_hash: "abc".to_string(),
            label_hash: "def".to_string(),
            execution_validity_summary:
                "execution_ready=10;execution_observe_only=5;execution_blocked=2".to_string(),
            sparsity_ratio: 0.5,
            pruned_factor_trail: Vec::new(),
            segments: Vec::new(),
            provenance: RunProvenance::default(),
        }
    }

    #[test]
    fn promotes_at_or_above_threshold() {
        assert_eq!(
            classify_mece_recovery_gate(MECE_RECOVERY_ACCURACY_GATE),
            "promote"
        );
        assert_eq!(classify_mece_recovery_gate(0.99), "promote");
    }

    #[test]
    fn blocks_below_threshold() {
        assert_eq!(
            classify_mece_recovery_gate(MECE_RECOVERY_ACCURACY_GATE - 0.0001),
            "blocked"
        );
        assert_eq!(classify_mece_recovery_gate(0.0), "blocked");
    }

    #[test]
    fn combined_gate_blocks_on_any_subgate_failure() {
        let mut artifact = base_artifact();
        assert_eq!(classify_mece_recovery_combined_gate(&artifact), "promote");

        artifact.accuracy = 0.5;
        assert_eq!(classify_mece_recovery_combined_gate(&artifact), "blocked");
        artifact.accuracy = 0.97;

        artifact.sparsity_ratio = 0.02;
        assert_eq!(classify_mece_recovery_combined_gate(&artifact), "blocked");
        artifact.sparsity_ratio = 0.50;

        artifact.segments = vec![
            RolloutSegment {
                accuracy: 0.40,
                sample_count: 10,
                ..Default::default()
            },
            RolloutSegment {
                accuracy: 0.40,
                sample_count: 10,
                ..Default::default()
            },
            RolloutSegment {
                accuracy: 0.40,
                sample_count: 10,
                ..Default::default()
            },
        ];
        assert_eq!(classify_mece_recovery_combined_gate(&artifact), "blocked");
    }
}
