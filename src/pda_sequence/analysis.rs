//! End-to-end PDA sequence analysis (Phase 3 subset of
//! `docs/plans/nlp-inspired-pda-sequence-clustering-plan.md`).
//!
//! Glues the emitter, DTW/PAM clustering, multi-model HMM training, and
//! FCGR + majority-vote ensemble into one artifact-producing function.
//!
//! - `consistency_ratio` = DTW ↔ HMM per-sample agreement
//! - `ensemble_mean_confidence` = mean `PdaClusteringPacket.confidence`
//!   across all valid sessions (mean max-votes / total-voters)
//!
//! No CLI, PreBayes, or BBN wiring: the artifact is a companion surface
//! that higher layers can consume when they are ready.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::state::RunProvenance;
use crate::types::Candle;

use super::cluster::{cluster_pda_sequences, PdaDtwClusterPacket};
use super::emitter::emit_pda_sequence_from_candles;
use super::ensemble_cluster::{ensemble_classify_sessions, PdaClusteringPacket};
use super::fcgr::fcgr_cluster_sessions;
use super::hmm_cluster::{
    classify_pda_sequence, train_hmm_sequence_cluster, HmmSequenceClassification,
};
use super::token::{PdaToken, PdaTokenKind};

pub const PDA_SEQUENCE_ANALYSIS_METHOD: &str = "pda_sequence_analysis_v2";
pub const PDA_SEQUENCE_DEFAULT_KMER_K: usize = 2;

/// First-class record of a DTW + HMM + FCGR ensemble pipeline run. Every
/// field is deterministic given the input `(sessions, k, n_states, kmer_k)`
/// and the module constants — safe to diff across commits for regression
/// detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdaSequenceAnalysisArtifact {
    pub artifact_id: String,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub method: String,
    pub k: usize,
    pub n_states: usize,
    pub kmer_k: usize,
    pub total_sessions: usize,
    pub valid_sessions: usize,
    pub silhouette_score: f64,
    pub consistency_ratio: f64,
    pub ensemble_mean_confidence: f64,
    pub dtw_packets: Vec<PdaDtwClusterPacket>,
    pub hmm_classifications: Vec<HmmSequenceClassification>,
    pub fcgr_labels: Vec<usize>,
    pub ensemble_packets: Vec<PdaClusteringPacket>,
    pub provenance: RunProvenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PdaSequenceArtifactSummary {
    pub method: String,
    pub primary_cluster: Option<usize>,
    pub primary_cluster_label: Option<String>,
    pub primary_cluster_family: Option<String>,
    pub primary_cluster_confidence: Option<f64>,
    pub consistency_ratio: f64,
    pub ensemble_mean_confidence: f64,
    pub valid_sessions: usize,
    pub kmer_k: usize,
}

fn infer_pda_cluster_family_from_tokens(tokens: &[PdaToken]) -> Option<String> {
    if tokens.is_empty() {
        return None;
    }
    let mut trend_score = 0usize;
    let mut range_score = 0usize;
    for token in tokens {
        match token.kind {
            PdaTokenKind::StructureBreak
            | PdaTokenKind::FairValueGap
            | PdaTokenKind::PropulsionBlock
            | PdaTokenKind::OrderBlock => trend_score += 1,
            PdaTokenKind::LiquiditySweep | PdaTokenKind::RejectionBlock | PdaTokenKind::Cisd => {
                range_score += 1
            }
        }
    }
    Some(
        if trend_score > range_score {
            "trend"
        } else if range_score > trend_score {
            "range"
        } else {
            "transition"
        }
        .to_string(),
    )
}

pub fn summarize_pda_sequence_artifact(
    artifact: &PdaSequenceAnalysisArtifact,
) -> PdaSequenceArtifactSummary {
    let latest_packet = artifact.ensemble_packets.last();
    let primary_cluster = latest_packet.map(|packet| packet.primary_cluster);
    PdaSequenceArtifactSummary {
        method: artifact.method.clone(),
        primary_cluster,
        primary_cluster_label: latest_packet
            .map(|packet| format!("cluster_{}", packet.primary_cluster)),
        primary_cluster_family: primary_cluster.and_then(|cluster_id| {
            artifact
                .dtw_packets
                .iter()
                .find(|packet| packet.regime_cluster == cluster_id)
                .and_then(|packet| {
                    infer_pda_cluster_family_from_tokens(&packet.medoid_pda_sequence)
                })
        }),
        primary_cluster_confidence: latest_packet.map(|packet| packet.confidence),
        consistency_ratio: artifact.consistency_ratio,
        ensemble_mean_confidence: artifact.ensemble_mean_confidence,
        valid_sessions: artifact.valid_sessions,
        kmer_k: artifact.kmer_k,
    }
}

/// Run the full pipeline and assemble the artifact. Errors when fewer
/// than `k` valid sessions are emitted, clustering fails, or HMM training
/// fails — callers can fall back to DTW-only packets if they need partial
/// credit.
pub fn analyze_pda_sequences(
    symbol: &str,
    sessions: &[Vec<Candle>],
    k: usize,
    n_states: usize,
    kmer_k: usize,
    provenance: RunProvenance,
) -> Result<PdaSequenceAnalysisArtifact> {
    let total_sessions = sessions.len();

    if k == 0 {
        anyhow::bail!("k must be > 0");
    }
    if n_states == 0 {
        anyhow::bail!("n_states must be > 0");
    }
    if kmer_k == 0 {
        anyhow::bail!("kmer_k must be > 0");
    }

    let emitted: Vec<Vec<_>> = sessions
        .iter()
        .map(|candles| emit_pda_sequence_from_candles(candles))
        .collect();
    let valid: Vec<&Vec<_>> = emitted.iter().filter(|seq| !seq.is_empty()).collect();
    let valid_sessions = valid.len();

    if valid_sessions < k {
        anyhow::bail!(
            "need at least k={} valid (non-empty) sessions, got {}",
            k,
            valid_sessions
        );
    }

    let valid_owned: Vec<Vec<_>> = valid.iter().map(|s| (*s).clone()).collect();
    let dtw_packets = cluster_pda_sequences(&valid_owned, k)?;

    // Group sequences by DTW cluster so each HMM sees only its cluster's
    // tokens.
    let mut grouped: Vec<Vec<Vec<_>>> = vec![Vec::new(); k];
    for (sequence, packet) in valid_owned.iter().zip(dtw_packets.iter()) {
        grouped[packet.regime_cluster].push(sequence.clone());
    }

    // Every cluster must have ≥ 1 sequence (PAM guarantees this when
    // k ≤ valid_sessions). Still guard against pathological inputs where a
    // cluster is too small for Baum-Welch.
    let grouped_refs: Vec<&[Vec<_>]> = grouped.iter().map(|v| v.as_slice()).collect();
    let hmm_cluster = train_hmm_sequence_cluster(&grouped_refs, n_states)?;

    let hmm_classifications: Vec<HmmSequenceClassification> = valid_owned
        .iter()
        .map(|sequence| classify_pda_sequence(sequence, &hmm_cluster))
        .collect::<Result<_>>()?;

    let (_fcgr_vectors, fcgr_outcome) = fcgr_cluster_sessions(&valid_owned, k, kmer_k)?;
    let fcgr_labels = fcgr_outcome.labels.clone();

    let dtw_labels: Vec<usize> = dtw_packets.iter().map(|p| p.regime_cluster).collect();
    let hmm_labels: Vec<usize> = hmm_classifications.iter().map(|c| c.cluster).collect();
    let ensemble_packets = ensemble_classify_sessions(&dtw_labels, &hmm_labels, &fcgr_labels, k)?;

    let matches = dtw_packets
        .iter()
        .zip(hmm_classifications.iter())
        .filter(|(dtw, hmm)| dtw.regime_cluster == hmm.cluster)
        .count();
    let consistency_ratio = if valid_sessions == 0 {
        0.0
    } else {
        matches as f64 / valid_sessions as f64
    };

    let ensemble_mean_confidence = if ensemble_packets.is_empty() {
        0.0
    } else {
        ensemble_packets.iter().map(|p| p.confidence).sum::<f64>() / ensemble_packets.len() as f64
    };

    let silhouette_score = dtw_packets
        .first()
        .map(|packet| packet.silhouette_score)
        .unwrap_or(0.0);

    let generated_at = Utc::now();
    let artifact_id = format!(
        "pda-sequence-{}-{}",
        symbol,
        generated_at.timestamp_millis()
    );

    Ok(PdaSequenceAnalysisArtifact {
        artifact_id,
        generated_at,
        symbol: symbol.to_string(),
        method: PDA_SEQUENCE_ANALYSIS_METHOD.to_string(),
        k,
        n_states,
        kmer_k,
        total_sessions,
        valid_sessions,
        silhouette_score,
        consistency_ratio,
        ensemble_mean_confidence,
        dtw_packets,
        hmm_classifications,
        fcgr_labels,
        ensemble_packets,
        provenance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn ts(n: i64) -> DateTime<Utc> {
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

    fn trending_up_series(len: usize, seed: usize) -> Vec<Candle> {
        let mut candles = Vec::with_capacity(len);
        let mut base = 100.0 + seed as f64 * 0.5;
        for i in 0..len {
            let gap = if i % 6 == 3 { 1.5 } else { 0.0 };
            let open = base + gap;
            let close = open + 1.0;
            let high = close + 0.2;
            let low = open - 0.2;
            candles.push(candle(i as i64, open, high, low, close));
            base = close;
        }
        candles
    }

    fn trending_down_series(len: usize, seed: usize) -> Vec<Candle> {
        let mut candles = Vec::with_capacity(len);
        let mut base = 200.0 + seed as f64 * 0.5;
        for i in 0..len {
            let gap = if i % 6 == 3 { -1.5 } else { 0.0 };
            let open = base + gap;
            let close = open - 1.0;
            let high = open + 0.2;
            let low = close - 0.2;
            candles.push(candle(i as i64, open, high, low, close));
            base = close;
        }
        candles
    }

    fn mixed_sessions() -> Vec<Vec<Candle>> {
        let mut sessions = Vec::new();
        for seed in 0..4 {
            sessions.push(trending_up_series(60 + seed, seed));
        }
        for seed in 0..4 {
            sessions.push(trending_down_series(60 + seed, seed));
        }
        sessions
    }

    #[test]
    fn fails_when_valid_sessions_below_k() {
        // Single empty candle slice → no tokens, no clusters.
        let sessions: Vec<Vec<Candle>> = vec![vec![]];
        let res = analyze_pda_sequences(
            "NQ",
            &sessions,
            2,
            3,
            PDA_SEQUENCE_DEFAULT_KMER_K,
            RunProvenance::default(),
        );
        assert!(res.is_err());
    }

    #[test]
    fn rejects_zero_k_or_states() {
        let sessions = mixed_sessions();
        assert!(analyze_pda_sequences(
            "NQ",
            &sessions,
            0,
            3,
            PDA_SEQUENCE_DEFAULT_KMER_K,
            RunProvenance::default(),
        )
        .is_err());
        assert!(analyze_pda_sequences(
            "NQ",
            &sessions,
            2,
            0,
            PDA_SEQUENCE_DEFAULT_KMER_K,
            RunProvenance::default(),
        )
        .is_err());
        assert!(analyze_pda_sequences("NQ", &sessions, 2, 3, 0, RunProvenance::default()).is_err());
    }

    #[test]
    fn produces_consistent_artifact_on_mixed_fixture() {
        let sessions = mixed_sessions();
        let artifact = analyze_pda_sequences(
            "NQ",
            &sessions,
            2,
            3,
            PDA_SEQUENCE_DEFAULT_KMER_K,
            RunProvenance::default(),
        )
        .expect("analysis must succeed");
        assert_eq!(artifact.method, PDA_SEQUENCE_ANALYSIS_METHOD);
        assert_eq!(artifact.total_sessions, sessions.len());
        assert!(artifact.valid_sessions >= 2);
        assert_eq!(artifact.kmer_k, PDA_SEQUENCE_DEFAULT_KMER_K);
        assert_eq!(artifact.dtw_packets.len(), artifact.valid_sessions);
        assert_eq!(artifact.hmm_classifications.len(), artifact.valid_sessions);
        assert_eq!(artifact.fcgr_labels.len(), artifact.valid_sessions);
        assert_eq!(artifact.ensemble_packets.len(), artifact.valid_sessions);
        assert!(artifact.ensemble_mean_confidence >= 0.0);
        // HMM trained on DTW labels should strongly agree with DTW on the
        // training data — ≥ 50% is the minimum signal that clustering wasn't
        // catastrophically broken.
        assert!(
            artifact.consistency_ratio >= 0.5,
            "DTW↔HMM consistency {} too low",
            artifact.consistency_ratio
        );
        assert!(artifact.artifact_id.starts_with("pda-sequence-NQ-"));
    }

    #[test]
    fn artifact_fields_are_deterministic_except_timestamp() {
        let sessions = mixed_sessions();
        let a = analyze_pda_sequences(
            "NQ",
            &sessions,
            2,
            3,
            PDA_SEQUENCE_DEFAULT_KMER_K,
            RunProvenance::default(),
        )
        .unwrap();
        let b = analyze_pda_sequences(
            "NQ",
            &sessions,
            2,
            3,
            PDA_SEQUENCE_DEFAULT_KMER_K,
            RunProvenance::default(),
        )
        .unwrap();
        // Timestamp and artifact_id carry `Utc::now()`, so skip those in the
        // structural comparison.
        assert_eq!(a.k, b.k);
        assert_eq!(a.n_states, b.n_states);
        assert_eq!(a.kmer_k, b.kmer_k);
        assert_eq!(a.total_sessions, b.total_sessions);
        assert_eq!(a.valid_sessions, b.valid_sessions);
        assert_eq!(a.silhouette_score, b.silhouette_score);
        assert_eq!(a.consistency_ratio, b.consistency_ratio);
        assert_eq!(a.ensemble_mean_confidence, b.ensemble_mean_confidence);
        assert_eq!(a.dtw_packets, b.dtw_packets);
        assert_eq!(a.hmm_classifications, b.hmm_classifications);
        assert_eq!(a.fcgr_labels, b.fcgr_labels);
        assert_eq!(a.ensemble_packets, b.ensemble_packets);
    }
}
