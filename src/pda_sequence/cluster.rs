use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::dtw::{dtw_alignment, dtw_distance_matrix};
use super::kmedoids::pam_cluster;
use super::token::{pda_token_cost, PdaToken};

pub const PDA_DTW_CLUSTER_METHOD: &str = "dtw_kmedoids_v1";

/// Typed packet the rest of ict-engine can ingest. Carries enough lineage
/// (medoid + alignment + silhouette) that downstream surfaces
/// (RegimeSegmentationPacket, reflection_bundle, PreBayes evidence) can
/// explain "why is this run in cluster X".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PdaDtwClusterPacket {
    pub method: String,
    pub regime_cluster: usize,
    pub cluster_name: String,
    pub dtw_distance_to_medoid: f64,
    pub dtw_alignment_path: Vec<(usize, usize)>,
    pub medoid_pda_sequence: Vec<PdaToken>,
    pub cluster_size: usize,
    pub silhouette_score: f64,
}

/// Produce a packet per input sequence, after DTW + PAM clustering.
/// Deterministic given the same `(sequences, k)`.
///
/// Constraints per the NLP plan:
/// - No global k default — caller decides (plan-level "先定 k 再验证").
/// - `cluster_name` is a stable slug derived from the medoid sequence; a
///   human-readable label layer can sit on top later without breaking this
///   contract.
pub fn cluster_pda_sequences(
    sequences: &[Vec<PdaToken>],
    k: usize,
) -> Result<Vec<PdaDtwClusterPacket>> {
    if sequences.is_empty() {
        return Ok(Vec::new());
    }
    if sequences.iter().any(|seq| seq.is_empty()) {
        anyhow::bail!("all input sequences must contain at least one token");
    }

    let distance_matrix = dtw_distance_matrix(sequences);
    let outcome = pam_cluster(&distance_matrix, k)?;
    let silhouette = outcome.silhouette;

    let mut packets = Vec::with_capacity(sequences.len());
    for (idx, sequence) in sequences.iter().enumerate() {
        let cluster = outcome.labels[idx];
        let medoid_index = outcome.medoids[cluster];
        let medoid_sequence = sequences[medoid_index].clone();
        let alignment = dtw_alignment(sequence, &medoid_sequence, pda_token_cost)
            .expect("non-empty sequences guaranteed above");
        let cluster_size = outcome.labels.iter().filter(|c| **c == cluster).count();
        let cluster_name = derive_cluster_name(&medoid_sequence, cluster);

        packets.push(PdaDtwClusterPacket {
            method: PDA_DTW_CLUSTER_METHOD.to_string(),
            regime_cluster: cluster,
            cluster_name,
            dtw_distance_to_medoid: alignment.distance,
            dtw_alignment_path: alignment.path,
            medoid_pda_sequence: medoid_sequence,
            cluster_size,
            silhouette_score: silhouette,
        });
    }
    Ok(packets)
}

fn derive_cluster_name(medoid: &[PdaToken], cluster_index: usize) -> String {
    let mut slug = String::new();
    for (i, token) in medoid.iter().enumerate() {
        if i > 0 {
            slug.push_str("->");
        }
        slug.push_str(token.kind.as_str());
    }
    format!("cluster_{}::{}", cluster_index, slug)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pda_sequence::token::PdaTokenKind;

    fn tok(kind: PdaTokenKind, bar: usize) -> PdaToken {
        PdaToken::new(kind, bar)
    }

    fn expansion_sequence(drift: usize) -> Vec<PdaToken> {
        // "Expansion" motif: SB -> FVG -> OB -> Propulsion
        vec![
            tok(PdaTokenKind::StructureBreak, 10 + drift),
            tok(PdaTokenKind::FairValueGap, 11 + drift),
            tok(PdaTokenKind::OrderBlock, 12 + drift),
            tok(PdaTokenKind::PropulsionBlock, 13 + drift),
        ]
    }

    fn reversion_sequence(drift: usize) -> Vec<PdaToken> {
        // "Reversion / liquidity play" motif: Sweep -> RB -> CISD -> OB
        vec![
            tok(PdaTokenKind::LiquiditySweep, 20 + drift),
            tok(PdaTokenKind::RejectionBlock, 21 + drift),
            tok(PdaTokenKind::Cisd, 22 + drift),
            tok(PdaTokenKind::OrderBlock, 23 + drift),
        ]
    }

    #[test]
    fn empty_input_returns_empty() {
        let packets = cluster_pda_sequences(&[], 2).unwrap();
        assert!(packets.is_empty());
    }

    #[test]
    fn rejects_empty_sequence() {
        let sequences = vec![expansion_sequence(0), vec![]];
        assert!(cluster_pda_sequences(&sequences, 2).is_err());
    }

    #[test]
    fn separates_expansion_from_reversion_motifs() {
        let sequences = vec![
            expansion_sequence(0),
            expansion_sequence(5),
            expansion_sequence(10),
            reversion_sequence(0),
            reversion_sequence(5),
            reversion_sequence(10),
        ];
        let packets = cluster_pda_sequences(&sequences, 2).unwrap();
        assert_eq!(packets.len(), sequences.len());

        // All "expansion" samples must share one cluster; "reversion" must share another.
        let expansion_cluster = packets[0].regime_cluster;
        let reversion_cluster = packets[3].regime_cluster;
        assert_ne!(expansion_cluster, reversion_cluster);
        for packet in &packets[0..3] {
            assert_eq!(packet.regime_cluster, expansion_cluster);
        }
        for packet in &packets[3..6] {
            assert_eq!(packet.regime_cluster, reversion_cluster);
        }

        // Silhouette should be strong on a clean 2-motif fixture.
        assert!(
            packets[0].silhouette_score > 0.5,
            "clean motif fixture should yield silhouette > 0.5 (got {})",
            packets[0].silhouette_score
        );

        // Every packet carries a non-empty alignment path and medoid.
        for packet in &packets {
            assert_eq!(packet.method, PDA_DTW_CLUSTER_METHOD);
            assert!(!packet.dtw_alignment_path.is_empty());
            assert!(!packet.medoid_pda_sequence.is_empty());
            assert!(packet.cluster_name.starts_with("cluster_"));
            assert!(packet.cluster_size >= 1);
        }
    }

    #[test]
    fn medoid_of_own_cluster_has_zero_distance_to_medoid() {
        // Use structurally distinct sequences so only the medoid itself is 0-distance.
        let a = vec![
            tok(PdaTokenKind::StructureBreak, 10).with_volume_imbalance(0.8),
            tok(PdaTokenKind::FairValueGap, 11).with_overlap(0.2),
            tok(PdaTokenKind::OrderBlock, 12).with_liquidity_swept(true),
            tok(PdaTokenKind::PropulsionBlock, 13),
        ];
        let b = vec![
            tok(PdaTokenKind::StructureBreak, 20).with_volume_imbalance(0.4),
            tok(PdaTokenKind::FairValueGap, 21).with_overlap(0.8),
            tok(PdaTokenKind::OrderBlock, 22).with_liquidity_swept(false),
            tok(PdaTokenKind::PropulsionBlock, 23),
        ];
        let c = reversion_sequence(0);
        let sequences = vec![a, b, c];
        let packets = cluster_pda_sequences(&sequences, 2).unwrap();
        // Every packet has a finite distance, and every cluster has at least one
        // zero-distance packet (the medoid itself).
        let mut clusters_with_medoid: std::collections::HashSet<usize> =
            std::collections::HashSet::new();
        for packet in &packets {
            assert!(
                packet.dtw_distance_to_medoid.is_finite() && packet.dtw_distance_to_medoid >= 0.0,
                "distance must be finite and non-negative"
            );
            if packet.dtw_distance_to_medoid.abs() < 1e-9 {
                clusters_with_medoid.insert(packet.regime_cluster);
            }
        }
        assert_eq!(
            clusters_with_medoid.len(),
            2,
            "each of 2 clusters must have at least one medoid (distance == 0)"
        );
    }

    #[test]
    fn deterministic_across_reruns() {
        let sequences = vec![
            expansion_sequence(0),
            reversion_sequence(0),
            expansion_sequence(5),
            reversion_sequence(5),
        ];
        let a = cluster_pda_sequences(&sequences, 2).unwrap();
        let b = cluster_pda_sequences(&sequences, 2).unwrap();
        assert_eq!(a, b);
    }
}
