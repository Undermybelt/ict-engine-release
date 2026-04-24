//! FCGR (Frequency Chaos Game Representation) for PDA sequences.
//!
//! Maps a variable-length `PdaToken` sequence to a fixed-length k-mer
//! frequency vector of dimension `kind_count^k`. Lets DNA-style sequence
//! analysis tools (k-NN, cosine clustering, neural-net embeddings) consume
//! PDA sequences without per-call padding.
//!
//! Companion to the DTW + HMM clusterers — same "no trading decisions
//! derived" constraint.

use anyhow::Result;

use super::kmedoids::{pam_cluster, PamOutcome};
use super::token::{PdaToken, PdaTokenKind};

pub const PDA_KIND_COUNT: usize = 7;

fn kind_index(kind: PdaTokenKind) -> usize {
    match kind {
        PdaTokenKind::FairValueGap => 0,
        PdaTokenKind::OrderBlock => 1,
        PdaTokenKind::LiquiditySweep => 2,
        PdaTokenKind::StructureBreak => 3,
        PdaTokenKind::RejectionBlock => 4,
        PdaTokenKind::PropulsionBlock => 5,
        PdaTokenKind::Cisd => 6,
    }
}

/// Build a normalised k-mer frequency vector for `tokens`. Dimension is
/// `PDA_KIND_COUNT.pow(k as u32)` (e.g. `k=3` → 343-dim). Returns the
/// zero vector when the sequence is shorter than `k` so callers always
/// get a well-formed FCGR slot for any session.
pub fn pda_sequence_to_fcgr_vector(tokens: &[PdaToken], k: usize) -> Vec<f64> {
    assert!(k >= 1, "k-mer length must be ≥ 1");
    let dim = PDA_KIND_COUNT.pow(k as u32);
    let mut counts = vec![0.0_f64; dim];
    if tokens.len() < k {
        return counts;
    }
    let mut total = 0.0;
    for window in tokens.windows(k) {
        let mut idx = 0usize;
        for token in window {
            idx = idx * PDA_KIND_COUNT + kind_index(token.kind);
        }
        counts[idx] += 1.0;
        total += 1.0;
    }
    if total > 0.0 {
        for value in &mut counts {
            *value /= total;
        }
    }
    counts
}

/// Cosine distance `1 - cos(a, b)` clamped to `[0, 2]`. Returns `1.0` when
/// either vector has zero norm (no information either way).
pub fn fcgr_cosine_distance(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 1.0;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 1.0;
    }
    let cosine = dot / (norm_a * norm_b);
    (1.0 - cosine.clamp(-1.0, 1.0)).clamp(0.0, 2.0)
}

/// Pairwise cosine distance matrix over a list of FCGR vectors.
pub fn fcgr_cosine_distance_matrix(vectors: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = vectors.len();
    let mut matrix = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let d = fcgr_cosine_distance(&vectors[i], &vectors[j]);
            matrix[i][j] = d;
            matrix[j][i] = d;
        }
    }
    matrix
}

/// Cluster sessions by their FCGR vectors (cosine distance + PAM).
/// Returns `(per-session FCGR vectors, PAM outcome)` so callers can keep
/// the embeddings for downstream feature pipelines.
pub fn fcgr_cluster_sessions(
    sessions: &[Vec<PdaToken>],
    n_clusters: usize,
    kmer_k: usize,
) -> Result<(Vec<Vec<f64>>, PamOutcome)> {
    if sessions.is_empty() {
        anyhow::bail!("at least one session required");
    }
    let vectors: Vec<Vec<f64>> = sessions
        .iter()
        .map(|tokens| pda_sequence_to_fcgr_vector(tokens, kmer_k))
        .collect();
    let matrix = fcgr_cosine_distance_matrix(&vectors);
    let outcome = pam_cluster(&matrix, n_clusters)?;
    Ok((vectors, outcome))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(kind: PdaTokenKind, bar: usize) -> PdaToken {
        PdaToken::new(kind, bar)
    }

    #[test]
    fn vector_dim_matches_k() {
        for k in 1..=3 {
            let v = pda_sequence_to_fcgr_vector(&[tok(PdaTokenKind::FairValueGap, 0)], k);
            assert_eq!(v.len(), PDA_KIND_COUNT.pow(k as u32));
        }
    }

    #[test]
    fn short_sequence_returns_zero_vector() {
        let v = pda_sequence_to_fcgr_vector(&[tok(PdaTokenKind::OrderBlock, 0)], 3);
        assert!(v.iter().all(|x| *x == 0.0));
    }

    #[test]
    fn frequencies_sum_to_one() {
        let tokens = vec![
            tok(PdaTokenKind::FairValueGap, 0),
            tok(PdaTokenKind::OrderBlock, 1),
            tok(PdaTokenKind::LiquiditySweep, 2),
            tok(PdaTokenKind::FairValueGap, 3),
            tok(PdaTokenKind::OrderBlock, 4),
        ];
        let v = pda_sequence_to_fcgr_vector(&tokens, 2);
        let sum: f64 = v.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-9,
            "frequencies must sum to 1, got {sum}"
        );
    }

    #[test]
    fn cosine_distance_zero_for_identical_vectors() {
        let tokens = vec![
            tok(PdaTokenKind::FairValueGap, 0),
            tok(PdaTokenKind::OrderBlock, 1),
            tok(PdaTokenKind::LiquiditySweep, 2),
        ];
        let v = pda_sequence_to_fcgr_vector(&tokens, 2);
        let d = fcgr_cosine_distance(&v, &v);
        assert!(d.abs() < 1e-9, "self-distance must be zero, got {d}");
    }

    #[test]
    fn cosine_distance_positive_for_disjoint_vocabularies() {
        let a = vec![
            tok(PdaTokenKind::FairValueGap, 0),
            tok(PdaTokenKind::OrderBlock, 1),
            tok(PdaTokenKind::FairValueGap, 2),
        ];
        let b = vec![
            tok(PdaTokenKind::LiquiditySweep, 0),
            tok(PdaTokenKind::Cisd, 1),
            tok(PdaTokenKind::LiquiditySweep, 2),
        ];
        let va = pda_sequence_to_fcgr_vector(&a, 2);
        let vb = pda_sequence_to_fcgr_vector(&b, 2);
        let d = fcgr_cosine_distance(&va, &vb);
        assert!(
            d > 0.5,
            "fully-disjoint k-mers should yield high distance, got {d}"
        );
    }

    #[test]
    fn cluster_separates_motif_groups() {
        let expansion: Vec<Vec<PdaToken>> = (0..3)
            .map(|s| {
                vec![
                    tok(PdaTokenKind::StructureBreak, 10 + s),
                    tok(PdaTokenKind::FairValueGap, 11 + s),
                    tok(PdaTokenKind::OrderBlock, 12 + s),
                    tok(PdaTokenKind::PropulsionBlock, 13 + s),
                ]
            })
            .collect();
        let reversion: Vec<Vec<PdaToken>> = (0..3)
            .map(|s| {
                vec![
                    tok(PdaTokenKind::LiquiditySweep, 20 + s),
                    tok(PdaTokenKind::RejectionBlock, 21 + s),
                    tok(PdaTokenKind::Cisd, 22 + s),
                    tok(PdaTokenKind::OrderBlock, 23 + s),
                ]
            })
            .collect();
        let mut sessions = expansion;
        sessions.extend(reversion);

        let (vectors, outcome) = fcgr_cluster_sessions(&sessions, 2, 2).unwrap();
        assert_eq!(vectors.len(), sessions.len());
        // Group 0..3 must share a label, 3..6 must share the other label.
        let label_a = outcome.labels[0];
        let label_b = outcome.labels[3];
        assert_ne!(label_a, label_b);
        for label in &outcome.labels[0..3] {
            assert_eq!(*label, label_a);
        }
        for label in &outcome.labels[3..6] {
            assert_eq!(*label, label_b);
        }
        assert!(
            outcome.silhouette > 0.4,
            "silhouette={}",
            outcome.silhouette
        );
    }

    #[test]
    fn rejects_empty_session_list() {
        let sessions: Vec<Vec<PdaToken>> = Vec::new();
        assert!(fcgr_cluster_sessions(&sessions, 2, 3).is_err());
    }
}
