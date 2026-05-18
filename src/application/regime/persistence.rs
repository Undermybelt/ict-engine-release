use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::path::Path;

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::application::regime::recovery::MeceRecoveryReport;
use crate::domain::regime::{
    classify_mece_recovery_combined_gate, classify_mece_recovery_gate, MeceRecoveryArtifact,
    MeceRegimeLabel,
};
use crate::hmm::init_hmm_params;
use crate::state::{
    append_artifact_ledger_entry, artifact_state_path, load_state, save_state, state_exists,
    ArtifactLedgerEntry,
};
use crate::types::HMMParams;

pub const MECE_RECOVERY_ARTIFACT_FILE: &str = "mece_recovery_artifact.json";
/// Bumped to 2 when the artifact gained sparsity_ratio / pruned_factor_trail
/// / segments. Readers of v1 entries must tolerate their absence.
pub const MECE_RECOVERY_ARTIFACT_LEDGER_VERSION: usize = 2;
pub const MECE_RECOVERY_ARTIFACT_REVIEW_RULE_VERSION: &str = "mece-recovery-artifact-v2";
pub const HMM_STATE_FILE: &str = "hmm_params.json";
pub const HMM_NUMERIC_TRAINER_ARTIFACT_FILE: &str = "hmm_numeric_trainer_artifact.json";
pub const HMM_NUMERIC_TRAINER_ARTIFACT_PROTOCOL_VERSION: &str = "hmm-numeric-trainer-artifact-v1";

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct HmmNumericTrainerParameterBound {
    pub name: String,
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct HmmNumericTrainerArtifact {
    pub protocol_version: String,
    pub parameter_vector: Vec<f64>,
    pub parameter_names: Vec<String>,
    pub bounds: Vec<HmmNumericTrainerParameterBound>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub objective_breakdown: BTreeMap<String, f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    pub split_id: String,
    pub best_iteration: usize,
    pub source_data_hash: String,
    pub state_count: usize,
}

fn normalize_transition_row(row: &mut [f64]) {
    let sum: f64 = row.iter().sum();
    if sum > f64::EPSILON {
        for value in row {
            *value = (*value / sum).clamp(0.0, 1.0);
        }
    }
}

fn apply_transition_smoothing(params: &mut HMMParams, smoothing: f64) {
    let smoothing = smoothing.clamp(0.0, 1.0);
    let uniform = 1.0 / params.n_states as f64;
    for row in &mut params.transition {
        for value in row.iter_mut() {
            *value = (1.0 - smoothing) * *value + smoothing * uniform;
        }
        normalize_transition_row(row);
    }
}

fn apply_emission_std_floor(params: &mut HMMParams, floor: f64) {
    let floor = floor.max(1e-6);
    for row in &mut params.emission_stds {
        for value in row.iter_mut() {
            *value = value.max(floor);
        }
    }
}

fn hmm_numeric_trainer_artifact_path<P: AsRef<Path>>(dir: P, symbol: &str) -> std::path::PathBuf {
    dir.as_ref()
        .join(symbol)
        .join(HMM_NUMERIC_TRAINER_ARTIFACT_FILE)
}

pub fn load_hmm_numeric_trainer_artifact<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
) -> Result<Option<HmmNumericTrainerArtifact>> {
    let path = hmm_numeric_trainer_artifact_path(dir, symbol);
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(path)?;
    let artifact = serde_json::from_str::<HmmNumericTrainerArtifact>(&raw)?;
    Ok(Some(artifact))
}

fn hmm_numeric_trainer_artifact_to_params(
    artifact: &HmmNumericTrainerArtifact,
    obs_dim: usize,
) -> Result<HMMParams> {
    if artifact.protocol_version.trim() != HMM_NUMERIC_TRAINER_ARTIFACT_PROTOCOL_VERSION {
        anyhow::bail!(
            "unsupported HMM numeric artifact protocol version '{}'",
            artifact.protocol_version
        );
    }
    if artifact.state_count != 3 {
        anyhow::bail!(
            "unsupported HMM numeric artifact state_count '{}'; expected 3",
            artifact.state_count
        );
    }
    if artifact.parameter_vector.len() != artifact.parameter_names.len() {
        anyhow::bail!(
            "HMM numeric artifact parameter_vector/parameter_names length mismatch: {} vs {}",
            artifact.parameter_vector.len(),
            artifact.parameter_names.len()
        );
    }
    let mut params = init_hmm_params(obs_dim);
    for (name, value) in artifact
        .parameter_names
        .iter()
        .zip(artifact.parameter_vector.iter().copied())
    {
        match name.trim() {
            "transition_smoothing" => apply_transition_smoothing(&mut params, value),
            "emission_std_floor" => apply_emission_std_floor(&mut params, value),
            "posterior_temperature" => {}
            _ => {}
        }
    }
    Ok(params)
}

fn hmm_params_compatible(params: &HMMParams, obs_dim: usize) -> bool {
    params.n_states == 3
        && params.transition.len() == params.n_states
        && params.initial_probs.len() == params.n_states
        && params.emission_means.len() == params.n_states
        && params.emission_stds.len() == params.n_states
        && params.emission_means.iter().all(|row| row.len() == obs_dim)
        && params.emission_stds.iter().all(|row| row.len() == obs_dim)
}

pub fn load_or_init_hmm_params_with_numeric_artifact(
    symbol: &str,
    state_dir: &str,
    obs_dim: usize,
) -> HMMParams {
    if let Ok(Some(artifact)) = load_hmm_numeric_trainer_artifact(state_dir, symbol) {
        match hmm_numeric_trainer_artifact_to_params(&artifact, obs_dim) {
            Ok(params) => return params,
            Err(err) => {
                eprintln!(
                    "warning: failed to apply HMM numeric trainer artifact for '{}' from '{}': {}",
                    symbol, state_dir, err
                );
            }
        }
    }
    if !state_exists(state_dir, symbol, HMM_STATE_FILE) {
        return init_hmm_params(obs_dim);
    }
    match load_state::<HMMParams, _>(state_dir, symbol, HMM_STATE_FILE) {
        Ok(params) if hmm_params_compatible(&params, obs_dim) => params,
        Ok(_) => init_hmm_params(obs_dim),
        Err(err) => {
            eprintln!(
                "warning: failed to load HMM state for '{}' from '{}': {}",
                symbol, state_dir, err
            );
            init_hmm_params(obs_dim)
        }
    }
}

pub fn build_mece_recovery_artifact(
    symbol: &str,
    report: &MeceRecoveryReport,
    viterbi_path: &[usize],
    labels: &[MeceRegimeLabel],
) -> MeceRecoveryArtifact {
    let generated_at = Utc::now();
    let artifact_id = format!(
        "mece-recovery-{}-{}",
        symbol,
        generated_at.timestamp_millis()
    );
    MeceRecoveryArtifact {
        artifact_id,
        generated_at,
        symbol: symbol.to_string(),
        accuracy: report.accuracy,
        macro_f1: report.macro_f1,
        selected_factors: report.best_factor_set.clone(),
        hmm_viterbi_hash: hash_viterbi_path(viterbi_path),
        label_hash: hash_labels(labels),
        execution_validity_summary: format_execution_summary(&report.execution_validity_histogram),
        sparsity_ratio: report.sparsity_ratio,
        pruned_factor_trail: report.pruned_factor_trail.clone(),
        segments: report.segments.clone(),
        provenance: report.provenance.clone(),
    }
}

pub fn persist_mece_recovery_artifact<P: AsRef<Path>>(
    dir: P,
    artifact: &MeceRecoveryArtifact,
    source_phase: &str,
    source_run_id: Option<String>,
    decision_hint: &str,
) -> Result<()> {
    save_state(
        &dir,
        &artifact.symbol,
        MECE_RECOVERY_ARTIFACT_FILE,
        artifact,
    )?;
    let accuracy_gate = classify_mece_recovery_gate(artifact.accuracy);
    let combined_gate = classify_mece_recovery_combined_gate(artifact);
    let promote = combined_gate == "promote";
    let segment_summary = if artifact.segments.is_empty() {
        "segments=none".to_string()
    } else {
        let parts: Vec<String> = artifact
            .segments
            .iter()
            .map(|segment| {
                format!(
                    "{}..{}:{:.3}",
                    segment.horizon_bars.0, segment.horizon_bars.1, segment.accuracy
                )
            })
            .collect();
        format!("segments={}", parts.join(","))
    };
    append_artifact_ledger_entry(
        &dir,
        &artifact.symbol,
        ArtifactLedgerEntry {
            entry_id: format!("ledger:{}", artifact.artifact_id),
            artifact_kind: "mece_recovery_artifact".to_string(),
            artifact_id: artifact.artifact_id.clone(),
            version: MECE_RECOVERY_ARTIFACT_LEDGER_VERSION,
            generated_at: artifact.generated_at,
            symbol: artifact.symbol.clone(),
            source_phase: source_phase.to_string(),
            source_run_id,
            path: artifact_state_path(&dir, &artifact.symbol, MECE_RECOVERY_ARTIFACT_FILE),
            status: combined_gate.to_string(),
            promote_candidate: promote,
            actionable: promote,
            decision_hint: decision_hint.to_string(),
            review_reason: format!(
                "accuracy={:.4};accuracy_gate={};macro_f1={:.4};sparsity_ratio={:.3};{};{}",
                artifact.accuracy,
                accuracy_gate,
                artifact.macro_f1,
                artifact.sparsity_ratio,
                segment_summary,
                artifact.execution_validity_summary
            ),
            review_rule_version: MECE_RECOVERY_ARTIFACT_REVIEW_RULE_VERSION.to_string(),
            top_factor_name: artifact.selected_factors.first().cloned(),
            top_factor_action: None,
            family_scores: BTreeMap::from([
                ("mece_accuracy".to_string(), artifact.accuracy),
                ("mece_macro_f1".to_string(), artifact.macro_f1),
                ("mece_sparsity_ratio".to_string(), artifact.sparsity_ratio),
            ]),
            supersedes_artifact_id: None,
            quality_score: (artifact.accuracy * 100.0).round() as i32,
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

fn hash_labels(labels: &[MeceRegimeLabel]) -> String {
    let mut hasher = DefaultHasher::new();
    for label in labels {
        std::mem::discriminant(label).hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

fn hash_viterbi_path(path: &[usize]) -> String {
    let mut hasher = DefaultHasher::new();
    for state in path {
        state.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

fn format_execution_summary(histogram: &BTreeMap<String, usize>) -> String {
    let mut parts = Vec::with_capacity(histogram.len());
    for (key, value) in histogram {
        parts.push(format!("{}={}", key, value));
    }
    parts.join(";")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::regime::recovery::search_factors_for_mece_recovery;
    use crate::config::FrameFeatures;
    use crate::domain::regime::{manual_mece_labeler, MECE_RECOVERY_ACCURACY_GATE};
    use crate::factors::FactorRegistry;
    use crate::state::{save_state, RunProvenance};
    use crate::types::Candle;
    use chrono::{Duration, TimeZone};
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

    fn synthetic_series() -> Vec<Candle> {
        let mut series: Vec<Candle> = (0..10)
            .map(|i| candle(i, 100.0, 100.5, 99.5, 100.0))
            .collect();
        series.push(candle(10, 100.0, 105.0, 99.5, 104.5));
        series.push(candle(11, 104.0, 108.0, 103.5, 104.2));
        series.push(candle(12, 104.0, 104.05, 103.95, 104.02));
        for i in 13..40 {
            let base = 104.0 + (i as f64 - 13.0) * 0.2;
            let bullish = i % 2 == 0;
            if bullish {
                series.push(candle(i, base, base + 0.6, base - 0.1, base + 0.4));
            } else {
                series.push(candle(i, base + 0.4, base + 0.5, base - 0.2, base));
            }
        }
        series
    }

    fn sample_report() -> MeceRecoveryReport {
        let candles = synthetic_series();
        let labels = manual_mece_labeler(&candles, &FrameFeatures::default());
        search_factors_for_mece_recovery(
            &candles,
            &labels,
            &FactorRegistry::default(),
            RunProvenance::default(),
        )
        .unwrap()
    }

    #[test]
    fn label_hash_is_stable_across_invocations() {
        let labels = vec![
            MeceRegimeLabel::Expansion,
            MeceRegimeLabel::Manipulation,
            MeceRegimeLabel::Unknown,
        ];
        let h1 = hash_labels(&labels);
        let h2 = hash_labels(&labels);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16);
    }

    #[test]
    fn label_hash_changes_when_labels_differ() {
        let a = vec![MeceRegimeLabel::Expansion, MeceRegimeLabel::Reversion];
        let b = vec![MeceRegimeLabel::Expansion, MeceRegimeLabel::Compression];
        assert_ne!(hash_labels(&a), hash_labels(&b));
    }

    #[test]
    fn viterbi_hash_is_stable() {
        let path = vec![0_usize, 1, 2, 1, 0];
        assert_eq!(hash_viterbi_path(&path), hash_viterbi_path(&path));
        assert_ne!(hash_viterbi_path(&path), hash_viterbi_path(&[0, 1, 2]));
    }

    #[test]
    fn execution_summary_serializes_all_buckets() {
        let mut histogram = BTreeMap::new();
        histogram.insert("execution_ready".to_string(), 5);
        histogram.insert("execution_observe_only".to_string(), 3);
        histogram.insert("execution_blocked".to_string(), 2);
        let summary = format_execution_summary(&histogram);
        assert!(summary.contains("execution_ready=5"));
        assert!(summary.contains("execution_observe_only=3"));
        assert!(summary.contains("execution_blocked=2"));
    }

    #[test]
    fn artifact_id_includes_symbol() {
        let report = sample_report();
        let artifact = build_mece_recovery_artifact("NQ", &report, &[0, 1, 2], &[]);
        assert!(artifact.artifact_id.starts_with("mece-recovery-NQ-"));
        assert_eq!(artifact.symbol, "NQ");
        assert_eq!(artifact.accuracy, report.accuracy);
        assert_eq!(artifact.selected_factors, report.best_factor_set);
    }

    #[test]
    fn persist_writes_artifact_and_ledger_entry() {
        let report = sample_report();
        let labels = manual_mece_labeler(&synthetic_series(), &FrameFeatures::default());
        let artifact = build_mece_recovery_artifact("NQ", &report, &[0, 1, 0, 2], &labels);
        let dir = TempDir::new().unwrap();
        persist_mece_recovery_artifact(dir.path(), &artifact, "analyze", None, "test").unwrap();

        let artifact_path = dir.path().join("NQ").join(MECE_RECOVERY_ARTIFACT_FILE);
        assert!(artifact_path.exists(), "artifact file not written");
        let raw = fs::read_to_string(&artifact_path).unwrap();
        assert!(raw.contains("\"artifact_id\""));
        assert!(raw.contains("\"label_hash\""));
        assert!(raw.contains("\"hmm_viterbi_hash\""));

        let ledger_path = dir
            .path()
            .join("NQ")
            .join(crate::state::ARTIFACT_LEDGER_FILE);
        assert!(ledger_path.exists(), "ledger file not written");
        let ledger = fs::read_to_string(&ledger_path).unwrap();
        assert!(ledger.contains("\"mece_recovery_artifact\""));
        assert!(ledger.contains("\"mece-recovery-artifact-v2\""));
        let entries: Vec<ArtifactLedgerEntry> = serde_json::from_str(&ledger).unwrap();
        assert_eq!(
            entries[0].path,
            artifact_path.to_string_lossy(),
            "ledger path must point at the selected state_dir artifact"
        );
    }

    #[test]
    fn ledger_entry_promotes_only_above_gate() {
        let report = sample_report();
        let labels = manual_mece_labeler(&synthetic_series(), &FrameFeatures::default());
        let mut artifact = build_mece_recovery_artifact("NQ", &report, &[], &labels);

        artifact.accuracy = MECE_RECOVERY_ACCURACY_GATE - 0.01;
        let dir = TempDir::new().unwrap();
        persist_mece_recovery_artifact(dir.path(), &artifact, "analyze", None, "test").unwrap();
        let ledger_path = dir
            .path()
            .join("NQ")
            .join(crate::state::ARTIFACT_LEDGER_FILE);
        let ledger = fs::read_to_string(&ledger_path).unwrap();
        assert!(ledger.contains("\"status\": \"blocked\""));
        assert!(ledger.contains("\"promote_candidate\": false"));

        let mut promote_artifact = artifact.clone();
        promote_artifact.accuracy = MECE_RECOVERY_ACCURACY_GATE;
        // Combined gate also checks sparsity & segments; make those healthy so
        // we isolate the accuracy gate behaviour in this test.
        promote_artifact.sparsity_ratio = 0.50;
        promote_artifact.segments.clear();
        promote_artifact.artifact_id = "mece-recovery-NQ-promote".to_string();
        let dir2 = TempDir::new().unwrap();
        persist_mece_recovery_artifact(dir2.path(), &promote_artifact, "analyze", None, "test")
            .unwrap();
        let ledger2 = fs::read_to_string(
            dir2.path()
                .join("NQ")
                .join(crate::state::ARTIFACT_LEDGER_FILE),
        )
        .unwrap();
        assert!(ledger2.contains("\"status\": \"promote\""));
        assert!(ledger2.contains("\"promote_candidate\": true"));
    }

    #[test]
    fn load_or_init_hmm_params_with_numeric_artifact_prefers_artifact_when_valid() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("NQ")).unwrap();
        let artifact = HmmNumericTrainerArtifact {
            protocol_version: HMM_NUMERIC_TRAINER_ARTIFACT_PROTOCOL_VERSION.to_string(),
            parameter_vector: vec![1.0, 2.5, 0.8],
            parameter_names: vec![
                "transition_smoothing".to_string(),
                "emission_std_floor".to_string(),
                "posterior_temperature".to_string(),
            ],
            bounds: vec![
                HmmNumericTrainerParameterBound {
                    name: "transition_smoothing".to_string(),
                    lower: 0.0,
                    upper: 1.0,
                },
                HmmNumericTrainerParameterBound {
                    name: "emission_std_floor".to_string(),
                    lower: 0.1,
                    upper: 5.0,
                },
            ],
            objective_breakdown: BTreeMap::from([
                ("accuracy".to_string(), 0.96),
                ("macro_f1".to_string(), 0.94),
            ]),
            seed: Some(7),
            split_id: "demo-split".to_string(),
            best_iteration: 12,
            source_data_hash: "demo-hash".to_string(),
            state_count: 3,
        };
        save_state(
            dir.path(),
            "NQ",
            HMM_NUMERIC_TRAINER_ARTIFACT_FILE,
            &artifact,
        )
        .unwrap();

        let params =
            load_or_init_hmm_params_with_numeric_artifact("NQ", dir.path().to_str().unwrap(), 4);

        for row in &params.transition {
            for value in row {
                assert!((value - (1.0 / 3.0)).abs() < 1e-9);
            }
        }
        for row in &params.emission_stds {
            for value in row {
                assert!(*value >= 2.5);
            }
        }
    }

    #[test]
    fn load_or_init_hmm_params_with_numeric_artifact_falls_back_to_saved_state_when_artifact_invalid(
    ) {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("NQ")).unwrap();
        let artifact = HmmNumericTrainerArtifact {
            protocol_version: HMM_NUMERIC_TRAINER_ARTIFACT_PROTOCOL_VERSION.to_string(),
            parameter_vector: vec![0.5],
            parameter_names: vec!["transition_smoothing".to_string()],
            bounds: Vec::new(),
            objective_breakdown: BTreeMap::new(),
            seed: Some(7),
            split_id: "demo-split".to_string(),
            best_iteration: 12,
            source_data_hash: "demo-hash".to_string(),
            state_count: 4,
        };
        save_state(
            dir.path(),
            "NQ",
            HMM_NUMERIC_TRAINER_ARTIFACT_FILE,
            &artifact,
        )
        .unwrap();
        let mut saved = HMMParams::new_3state(4);
        saved.initial_probs = vec![0.9, 0.05, 0.05];
        save_state(dir.path(), "NQ", HMM_STATE_FILE, &saved).unwrap();

        let params =
            load_or_init_hmm_params_with_numeric_artifact("NQ", dir.path().to_str().unwrap(), 4);

        assert_eq!(params.initial_probs, saved.initial_probs);
    }
}
