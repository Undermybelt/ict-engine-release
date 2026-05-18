//! Ensemble PDA classifier: combines DTW, HMM, and FCGR cluster labels via
//! majority voting. DeLUCS-style "multi-network majority voting" applied to
//! the three independent clustering signals already in `pda_sequence`.
//!
//! Returns `PdaClusteringPacket` per session with the agreed primary label,
//! per-voter raw labels, and a confidence score = max-votes / total-voters.
//! Label alignment uses a greedy best-overlap permutation (good enough for
//! small `k`; future Hungarian-based alignment can swap in without changing
//! the API).

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub const PDA_ENSEMBLE_METHOD: &str = "pda_ensemble_majority_v1";
pub const PDA_ENSEMBLE_VOTERS: usize = 3;

/// Per-session output of `ensemble_classify_sessions`. `votes` records the
/// aligned vote each voter cast (DTW, HMM, FCGR — in that order).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PdaClusteringPacket {
    pub method: String,
    pub primary_cluster: usize,
    pub confidence: f64,
    pub vote_distribution: Vec<usize>,
    pub votes: [usize; PDA_ENSEMBLE_VOTERS],
    pub voter_names: [String; PDA_ENSEMBLE_VOTERS],
}

/// Greedy best-overlap label permutation. For each label in `to_align`,
/// pick the reference label with the highest co-occurrence count and remap
/// it. Stable: ties are broken by smaller reference index.
pub fn align_labels_to_reference(reference: &[usize], to_align: &[usize], k: usize) -> Vec<usize> {
    if reference.len() != to_align.len() || k == 0 {
        return to_align.to_vec();
    }
    let mut mapping = vec![0usize; k];
    for (candidate, mapped_label) in mapping.iter_mut().enumerate().take(k) {
        let mut best_overlap: i64 = -1;
        let mut best_ref = candidate;
        for ref_label in 0..k {
            let overlap = reference
                .iter()
                .zip(to_align.iter())
                .filter(|(r, t)| **r == ref_label && **t == candidate)
                .count() as i64;
            if overlap > best_overlap || (overlap == best_overlap && ref_label < best_ref) {
                best_overlap = overlap;
                best_ref = ref_label;
            }
        }
        *mapped_label = best_ref;
    }
    to_align.iter().map(|c| mapping[*c]).collect()
}

/// Combine DTW, HMM, and FCGR cluster labels per session into ensemble
/// packets. `k` is the shared number of clusters used by every voter; all
/// three label slices must have the same length as the number of sessions.
pub fn ensemble_classify_sessions(
    dtw_labels: &[usize],
    hmm_labels: &[usize],
    fcgr_labels: &[usize],
    k: usize,
) -> Result<Vec<PdaClusteringPacket>> {
    if k == 0 {
        anyhow::bail!("k must be > 0");
    }
    let n = dtw_labels.len();
    if hmm_labels.len() != n || fcgr_labels.len() != n {
        anyhow::bail!(
            "all voter label slices must have the same length (dtw={}, hmm={}, fcgr={})",
            n,
            hmm_labels.len(),
            fcgr_labels.len()
        );
    }
    let aligned_hmm = align_labels_to_reference(dtw_labels, hmm_labels, k);
    let aligned_fcgr = align_labels_to_reference(dtw_labels, fcgr_labels, k);

    let voter_names = [
        "dtw_kmedoids".to_string(),
        "hmm_sequence".to_string(),
        "fcgr_kmedoids".to_string(),
    ];

    let mut packets = Vec::with_capacity(n);
    for i in 0..n {
        let votes = [dtw_labels[i], aligned_hmm[i], aligned_fcgr[i]];
        let mut distribution = vec![0usize; k];
        for vote in &votes {
            if *vote < k {
                distribution[*vote] += 1;
            }
        }
        let (primary_cluster, max_votes) = distribution
            .iter()
            .copied()
            .enumerate()
            .max_by(|(idx_a, va), (idx_b, vb)| va.cmp(vb).then(idx_b.cmp(idx_a)))
            .unwrap_or((0, 0));
        let confidence = if PDA_ENSEMBLE_VOTERS == 0 {
            0.0
        } else {
            max_votes as f64 / PDA_ENSEMBLE_VOTERS as f64
        };
        packets.push(PdaClusteringPacket {
            method: PDA_ENSEMBLE_METHOD.to_string(),
            primary_cluster,
            confidence,
            vote_distribution: distribution,
            votes,
            voter_names: voter_names.clone(),
        });
    }
    Ok(packets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_k() {
        let res = ensemble_classify_sessions(&[0, 1], &[0, 1], &[0, 1], 0);
        assert!(res.is_err());
    }

    #[test]
    fn rejects_label_length_mismatch() {
        let res = ensemble_classify_sessions(&[0, 1], &[0], &[0, 1], 2);
        assert!(res.is_err());
    }

    #[test]
    fn unanimous_votes_yield_confidence_one() {
        let packets =
            ensemble_classify_sessions(&[0, 0, 1, 1], &[0, 0, 1, 1], &[0, 0, 1, 1], 2).unwrap();
        for packet in &packets {
            assert_eq!(packet.confidence, 1.0);
            assert_eq!(packet.method, PDA_ENSEMBLE_METHOD);
        }
        assert_eq!(packets[0].primary_cluster, 0);
        assert_eq!(packets[3].primary_cluster, 1);
    }

    #[test]
    fn aligns_inverted_label_permutation() {
        // HMM/FCGR use opposite labels to DTW — alignment should fix that
        // before voting so all three end up unanimous.
        let dtw = vec![0, 0, 1, 1];
        let hmm = vec![1, 1, 0, 0];
        let fcgr = vec![1, 1, 0, 0];
        let packets = ensemble_classify_sessions(&dtw, &hmm, &fcgr, 2).unwrap();
        for packet in &packets {
            assert_eq!(
                packet.confidence, 1.0,
                "after greedy alignment, all three voters should agree"
            );
        }
        assert_eq!(packets[0].primary_cluster, 0);
        assert_eq!(packets[3].primary_cluster, 1);
    }

    #[test]
    fn split_vote_falls_back_to_majority_with_low_confidence() {
        // DTW makes a 2/2 split between the first and last halves of the
        // dataset. HMM and FCGR both collapse all sessions onto a single
        // raw label — the greedy aligner cannot fabricate a split that
        // is not in the underlying data, so samples 2 and 3 must end up
        // with a 2-vs-1 majority and confidence 2/3.
        let dtw = vec![0, 0, 1, 1];
        let hmm = vec![0, 0, 0, 0];
        let fcgr = vec![1, 1, 1, 1];
        let packets = ensemble_classify_sessions(&dtw, &hmm, &fcgr, 2).unwrap();
        let any_split = packets.iter().any(|p| p.confidence < 1.0);
        assert!(
            any_split,
            "voter disagreement should yield confidence < 1.0 on at least one packet"
        );
        for packet in &packets[2..4] {
            assert!(
                (packet.confidence - 2.0 / 3.0).abs() < 1e-9,
                "samples DTW assigns to cluster 1 should fall back to 2/3 confidence (packet={packet:?})"
            );
            let max_votes = packet.vote_distribution.iter().copied().max().unwrap_or(0);
            assert_eq!(
                max_votes, 2,
                "majority should be 2 voters out of 3 on a true split"
            );
        }
    }

    #[test]
    fn align_labels_handles_identical_sequences() {
        let aligned = align_labels_to_reference(&[0, 1, 0, 1], &[0, 1, 0, 1], 2);
        assert_eq!(aligned, vec![0, 1, 0, 1]);
    }

    #[test]
    fn align_labels_inverts_swapped_sequences() {
        let aligned = align_labels_to_reference(&[0, 1, 0, 1], &[1, 0, 1, 0], 2);
        assert_eq!(aligned, vec![0, 1, 0, 1]);
    }
}
