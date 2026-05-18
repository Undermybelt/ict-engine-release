use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use crate::state::{
    append_artifact_ledger_entry, artifact_state_path, load_state, save_state, ArtifactLedgerEntry,
};

use super::analysis::PdaSequenceAnalysisArtifact;

pub const PDA_SEQUENCE_ARTIFACT_FILE: &str = "pda_sequence_artifact.json";

/// Silhouette / consistency thresholds for tagging the ledger entry's
/// status. Clustering is exploratory — it never `promote_candidate=true` on
/// its own; callers decide if `actionable=true` warrants downstream use.
pub const PDA_SEQUENCE_SILHOUETTE_ACTIONABLE: f64 = 0.30;
pub const PDA_SEQUENCE_CONSISTENCY_ACTIONABLE: f64 = 0.70;

pub fn classify_pda_sequence_ledger_status(
    silhouette_score: f64,
    consistency_ratio: f64,
) -> &'static str {
    if silhouette_score >= 0.50 && consistency_ratio >= 0.90 {
        "strong"
    } else if silhouette_score >= PDA_SEQUENCE_SILHOUETTE_ACTIONABLE
        && consistency_ratio >= PDA_SEQUENCE_CONSISTENCY_ACTIONABLE
    {
        "observe"
    } else {
        "weak"
    }
}

pub fn persist_pda_sequence_analysis<P: AsRef<Path>>(
    dir: P,
    artifact: &PdaSequenceAnalysisArtifact,
    source_phase: &str,
    source_run_id: Option<String>,
) -> Result<()> {
    save_state(&dir, &artifact.symbol, PDA_SEQUENCE_ARTIFACT_FILE, artifact)?;
    let status =
        classify_pda_sequence_ledger_status(artifact.silhouette_score, artifact.consistency_ratio);
    let actionable = status == "strong" || status == "observe";
    append_artifact_ledger_entry(
        &dir,
        &artifact.symbol,
        ArtifactLedgerEntry {
            entry_id: format!("ledger:{}", artifact.artifact_id),
            artifact_kind: "pda_sequence_artifact".to_string(),
            artifact_id: artifact.artifact_id.clone(),
            version: 1,
            generated_at: artifact.generated_at,
            symbol: artifact.symbol.clone(),
            source_phase: source_phase.to_string(),
            source_run_id,
            path: artifact_state_path(&dir, &artifact.symbol, PDA_SEQUENCE_ARTIFACT_FILE),
            status: status.to_string(),
            // Clustering alone cannot promote — it is a diagnostic surface.
            promote_candidate: false,
            actionable,
            decision_hint: format!(
                "pda_sequence_analysis:k={};n_states={};valid_sessions={}",
                artifact.k, artifact.n_states, artifact.valid_sessions
            ),
            review_reason: format!(
                "silhouette={:.4};consistency={:.4};total_sessions={}",
                artifact.silhouette_score, artifact.consistency_ratio, artifact.total_sessions
            ),
            review_rule_version: "pda-sequence-artifact-v1".to_string(),
            top_factor_name: None,
            top_factor_action: None,
            family_scores: BTreeMap::from([
                ("pda_silhouette".to_string(), artifact.silhouette_score),
                ("pda_consistency".to_string(), artifact.consistency_ratio),
            ]),
            supersedes_artifact_id: None,
            quality_score: (artifact.silhouette_score * 100.0).round() as i32,
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

pub fn load_pda_sequence_analysis<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
) -> Result<PdaSequenceAnalysisArtifact> {
    load_state(dir, symbol, PDA_SEQUENCE_ARTIFACT_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pda_sequence::analysis::{analyze_pda_sequences, PdaSequenceAnalysisArtifact};
    use crate::state::RunProvenance;
    use crate::types::Candle;
    use chrono::{Duration, TimeZone, Utc};
    use std::fs;
    use tempfile::TempDir;

    fn ts(n: i64) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap() + Duration::minutes(n)
    }

    fn candle(idx: i64, open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            timestamp: ts(idx),
            open,
            high,
            low,
            close,
            volume: 1_000.0,
        }
    }

    fn trending_up(len: usize, seed: usize) -> Vec<Candle> {
        let mut c = Vec::with_capacity(len);
        let mut base = 100.0 + seed as f64 * 0.5;
        for i in 0..len {
            let gap = if i % 6 == 3 { 1.5 } else { 0.0 };
            let open = base + gap;
            let close = open + 1.0;
            let high = close + 0.2;
            let low = open - 0.2;
            c.push(candle(i as i64, open, high, low, close));
            base = close;
        }
        c
    }

    fn trending_down(len: usize, seed: usize) -> Vec<Candle> {
        let mut c = Vec::with_capacity(len);
        let mut base = 200.0 + seed as f64 * 0.5;
        for i in 0..len {
            let gap = if i % 6 == 3 { -1.5 } else { 0.0 };
            let open = base + gap;
            let close = open - 1.0;
            let high = open + 0.2;
            let low = close - 0.2;
            c.push(candle(i as i64, open, high, low, close));
            base = close;
        }
        c
    }

    fn sample_artifact() -> PdaSequenceAnalysisArtifact {
        let mut sessions = Vec::new();
        for seed in 0..4 {
            sessions.push(trending_up(60 + seed, seed));
        }
        for seed in 0..4 {
            sessions.push(trending_down(60 + seed, seed));
        }
        analyze_pda_sequences(
            "NQ",
            &sessions,
            2,
            3,
            crate::pda_sequence::PDA_SEQUENCE_DEFAULT_KMER_K,
            RunProvenance::default(),
        )
        .unwrap()
    }

    #[test]
    fn status_thresholds_are_ordered() {
        assert_eq!(classify_pda_sequence_ledger_status(0.7, 0.95), "strong");
        assert_eq!(classify_pda_sequence_ledger_status(0.35, 0.80), "observe");
        assert_eq!(classify_pda_sequence_ledger_status(0.10, 0.60), "weak");
    }

    #[test]
    fn persist_writes_artifact_and_ledger_entry() {
        let artifact = sample_artifact();
        let dir = TempDir::new().unwrap();
        persist_pda_sequence_analysis(dir.path(), &artifact, "analyze", None).unwrap();

        let artifact_path = dir.path().join("NQ").join(PDA_SEQUENCE_ARTIFACT_FILE);
        assert!(artifact_path.exists(), "artifact file not written");
        let raw = fs::read_to_string(&artifact_path).unwrap();
        assert!(raw.contains("\"consistency_ratio\""));
        assert!(raw.contains("\"silhouette_score\""));
        assert!(raw.contains("\"dtw_packets\""));
        assert!(raw.contains("\"hmm_classifications\""));

        let ledger_path = dir
            .path()
            .join("NQ")
            .join(crate::state::ARTIFACT_LEDGER_FILE);
        assert!(ledger_path.exists(), "ledger file not written");
        let ledger = fs::read_to_string(&ledger_path).unwrap();
        assert!(ledger.contains("\"pda_sequence_artifact\""));
        assert!(ledger.contains("\"pda-sequence-artifact-v1\""));
        // Clustering is never a promote source on its own.
        assert!(ledger.contains("\"promote_candidate\": false"));
        let entries: Vec<ArtifactLedgerEntry> = serde_json::from_str(&ledger).unwrap();
        assert_eq!(
            entries[0].path,
            artifact_path.to_string_lossy(),
            "ledger path must point at the selected state_dir artifact"
        );
    }

    #[test]
    fn load_round_trips_persisted_artifact() {
        let artifact = sample_artifact();
        let dir = TempDir::new().unwrap();
        persist_pda_sequence_analysis(dir.path(), &artifact, "analyze", None).unwrap();

        let loaded = load_pda_sequence_analysis(dir.path(), "NQ").unwrap();
        assert_eq!(loaded.method, artifact.method);
        assert_eq!(loaded.kmer_k, artifact.kmer_k);
        assert_eq!(loaded.fcgr_labels, artifact.fcgr_labels);
        assert_eq!(loaded.ensemble_packets, artifact.ensemble_packets);
    }
}
