//! Multi-model HMM sequence clustering (Phase 2 of
//! `docs/plans/nlp-inspired-pda-sequence-clustering-plan.md`).
//!
//! Trains one HMM per cluster on pre-grouped `PdaToken` sequences (typically
//! grouped by the DTW/PAM output from `cluster::cluster_pda_sequences`), then
//! classifies a new sequence by picking the model with the highest forward
//! log-likelihood. Keeps the DTW path available as the "which samples go
//! together" signal while letting HMMs take over the "which family does this
//! new sequence look like" question.
//!
//! Constraints:
//! - reuses the existing `hmm::` machinery (`BaumWelch`, `ForwardBackward`);
//!   does not introduce a second HMM implementation
//! - classification returns a softmax-normalised posterior, never a hard
//!   rejection — callers decide thresholds
//! - no main.rs / PreBayes wiring yet; this is a companion surface

use serde::{Deserialize, Serialize};

use crate::hmm::{BaumWelch, ForwardBackward};
use crate::types::HMMParams;

use super::token::{PdaToken, PdaTokenKind};

pub const PDA_TOKEN_OBS_DIM: usize = 10;
pub const HMM_SEQUENCE_CLUSTER_METHOD: &str = "multi_hmm_sequence_v1";

/// Default Baum-Welch training bounds. Low iterations because each cluster's
/// training set is usually tiny relative to a full trading dataset and the
/// emission dimension is already well-separated by the one-hot kind prefix.
pub const HMM_TRAIN_MAX_ITER: usize = 48;
pub const HMM_TRAIN_TOLERANCE: f64 = 1e-4;

/// Encode one `PdaToken` as a fixed-length observation vector. The first 7
/// dimensions are a one-hot of `PdaTokenKind`; the last 3 are metadata
/// (`overlap`, `liquidity_swept` as 0/1, `volume_imbalance_ratio`).
pub fn encode_pda_token(token: &PdaToken) -> Vec<f64> {
    let mut obs = vec![0.0_f64; PDA_TOKEN_OBS_DIM];
    obs[kind_index(token.kind)] = 1.0;
    obs[7] = token.overlap;
    obs[8] = if token.liquidity_swept { 1.0 } else { 0.0 };
    obs[9] = token.volume_imbalance_ratio;
    obs
}

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

pub fn encode_sequence(sequence: &[PdaToken]) -> Vec<Vec<f64>> {
    sequence.iter().map(encode_pda_token).collect()
}

/// Trained multi-model HMM. `models[i]` is the Baum-Welch fit on cluster `i`.
#[derive(Debug, Clone)]
pub struct HmmSequenceCluster {
    pub models: Vec<HMMParams>,
    pub n_states: usize,
    pub obs_dim: usize,
}

/// Result of classifying a single sequence against a trained cluster.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HmmSequenceClassification {
    pub method: String,
    pub cluster: usize,
    pub log_likelihood: f64,
    pub posterior: Vec<f64>,
}

/// Train one HMM per input cluster using Baum-Welch on the concatenation of
/// that cluster's `PdaToken` observations. Errors if any cluster has no
/// sequences or if the concatenated observation stream is too short to
/// support Baum-Welch's forward/backward recursion.
pub fn train_hmm_sequence_cluster(
    sequences_per_cluster: &[&[Vec<PdaToken>]],
    n_states: usize,
) -> anyhow::Result<HmmSequenceCluster> {
    if sequences_per_cluster.is_empty() {
        anyhow::bail!("at least one cluster required");
    }
    if n_states == 0 {
        anyhow::bail!("n_states must be > 0");
    }

    let mut models = Vec::with_capacity(sequences_per_cluster.len());
    for (idx, cluster_sequences) in sequences_per_cluster.iter().enumerate() {
        if cluster_sequences.is_empty() {
            anyhow::bail!("cluster {idx} has no sequences");
        }
        let observations: Vec<Vec<f64>> = cluster_sequences
            .iter()
            .flat_map(|seq| seq.iter().map(encode_pda_token))
            .collect();
        if observations.len() < 2 {
            anyhow::bail!(
                "cluster {idx} has only {} observation(s); Baum-Welch requires ≥ 2",
                observations.len()
            );
        }
        let init = initial_params(n_states, &observations);
        let fitted = BaumWelch::fit(
            &observations,
            &init,
            HMM_TRAIN_MAX_ITER,
            HMM_TRAIN_TOLERANCE,
        );
        models.push(fitted);
    }
    Ok(HmmSequenceCluster {
        models,
        n_states,
        obs_dim: PDA_TOKEN_OBS_DIM,
    })
}

/// Pick the HMM with the highest forward log-likelihood and report the
/// softmax-normalised posterior over all cluster models.
pub fn classify_pda_sequence(
    sequence: &[PdaToken],
    cluster: &HmmSequenceCluster,
) -> anyhow::Result<HmmSequenceClassification> {
    if sequence.is_empty() {
        anyhow::bail!("cannot classify an empty sequence");
    }
    if cluster.models.is_empty() {
        anyhow::bail!("HmmSequenceCluster contains no models");
    }
    let observations = encode_sequence(sequence);
    let log_likelihoods: Vec<f64> = cluster
        .models
        .iter()
        .map(|model| ForwardBackward::forward(&observations, model).1)
        .collect();
    let (best, &best_ll) = log_likelihoods
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .expect("non-empty log_likelihoods");
    let max_ll = best_ll;
    let exps: Vec<f64> = log_likelihoods
        .iter()
        .map(|ll| (ll - max_ll).exp())
        .collect();
    let total: f64 = exps.iter().sum();
    let total_safe = if total > 0.0 { total } else { 1.0 };
    let posterior: Vec<f64> = exps.iter().map(|e| e / total_safe).collect();
    Ok(HmmSequenceClassification {
        method: HMM_SEQUENCE_CLUSTER_METHOD.to_string(),
        cluster: best,
        log_likelihood: best_ll,
        posterior,
    })
}

fn initial_params(n_states: usize, observations: &[Vec<f64>]) -> HMMParams {
    let obs_dim = observations
        .first()
        .map(|o| o.len())
        .unwrap_or(PDA_TOKEN_OBS_DIM);

    // Diagonal-biased transition keeps Baum-Welch from immediately collapsing
    // states while still allowing inter-state flow.
    let stay = 0.80;
    let leave = if n_states > 1 {
        (1.0 - stay) / (n_states - 1) as f64
    } else {
        0.0
    };
    let transition: Vec<Vec<f64>> = (0..n_states)
        .map(|i| {
            (0..n_states)
                .map(|j| if i == j { stay } else { leave })
                .collect()
        })
        .collect();

    let initial_probs = vec![1.0 / n_states as f64; n_states];

    // Seed each state's mean from a different slice of the observation
    // stream. Gives Baum-Welch enough spread to find distinct states without
    // using randomness.
    let emission_means: Vec<Vec<f64>> = (0..n_states)
        .map(|state| {
            let pick_idx = if observations.is_empty() {
                0
            } else {
                (state * observations.len()) / n_states.max(1)
            };
            observations
                .get(pick_idx)
                .cloned()
                .unwrap_or_else(|| vec![0.0; obs_dim])
        })
        .collect();

    let emission_stds = vec![vec![0.5; obs_dim]; n_states];

    HMMParams {
        n_states,
        transition,
        emission_means,
        emission_stds,
        initial_probs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pda_sequence::token::PdaTokenKind;

    fn tok(kind: PdaTokenKind, bar: usize, imbalance: f64) -> PdaToken {
        PdaToken::new(kind, bar).with_volume_imbalance(imbalance)
    }

    fn expansion_sequence(offset: usize) -> Vec<PdaToken> {
        vec![
            tok(PdaTokenKind::StructureBreak, 10 + offset, 0.6),
            tok(PdaTokenKind::FairValueGap, 11 + offset, 0.5),
            tok(PdaTokenKind::OrderBlock, 12 + offset, 0.4),
            tok(PdaTokenKind::PropulsionBlock, 13 + offset, 0.7),
        ]
    }

    fn reversion_sequence(offset: usize) -> Vec<PdaToken> {
        vec![
            tok(PdaTokenKind::LiquiditySweep, 20 + offset, -0.4),
            tok(PdaTokenKind::RejectionBlock, 21 + offset, -0.6),
            tok(PdaTokenKind::Cisd, 22 + offset, -0.3),
            tok(PdaTokenKind::OrderBlock, 23 + offset, -0.2),
        ]
    }

    #[test]
    fn encoding_is_fixed_dim_with_one_hot_kind() {
        let token = tok(PdaTokenKind::LiquiditySweep, 3, 0.2).with_overlap(0.5);
        let obs = encode_pda_token(&token);
        assert_eq!(obs.len(), PDA_TOKEN_OBS_DIM);
        assert_eq!(obs[2], 1.0, "LiquiditySweep must land at index 2");
        // All other one-hot positions should be 0.
        for (i, value) in obs.iter().enumerate().take(7) {
            if i == 2 {
                continue;
            }
            assert_eq!(*value, 0.0, "unexpected activation at kind index {i}");
        }
        assert_eq!(obs[7], 0.5);
        assert_eq!(obs[8], 0.0);
        assert!((obs[9] - 0.2).abs() < 1e-9);
    }

    #[test]
    fn rejects_empty_cluster_inputs() {
        let empty: Vec<Vec<PdaToken>> = Vec::new();
        let empty_refs: &[&[Vec<PdaToken>]] = &[empty.as_slice()];
        assert!(train_hmm_sequence_cluster(empty_refs, 2).is_err());
    }

    #[test]
    fn rejects_zero_states() {
        let expansions = vec![expansion_sequence(0), expansion_sequence(5)];
        let reversions = vec![reversion_sequence(0), reversion_sequence(5)];
        assert!(
            train_hmm_sequence_cluster(&[expansions.as_slice(), reversions.as_slice()], 0).is_err()
        );
    }

    #[test]
    fn classifier_separates_expansion_from_reversion() {
        let expansions = vec![
            expansion_sequence(0),
            expansion_sequence(5),
            expansion_sequence(10),
            expansion_sequence(15),
        ];
        let reversions = vec![
            reversion_sequence(0),
            reversion_sequence(5),
            reversion_sequence(10),
            reversion_sequence(15),
        ];
        let cluster =
            train_hmm_sequence_cluster(&[expansions.as_slice(), reversions.as_slice()], 3)
                .expect("training must succeed");

        let new_expansion = expansion_sequence(40);
        let new_reversion = reversion_sequence(40);

        let expansion_class = classify_pda_sequence(&new_expansion, &cluster).unwrap();
        let reversion_class = classify_pda_sequence(&new_reversion, &cluster).unwrap();

        assert_eq!(
            expansion_class.cluster, 0,
            "expansion-like must match cluster 0"
        );
        assert_eq!(
            reversion_class.cluster, 1,
            "reversion-like must match cluster 1"
        );
        assert_eq!(expansion_class.method, HMM_SEQUENCE_CLUSTER_METHOD);
    }

    #[test]
    fn posterior_is_a_simplex() {
        let expansions = vec![expansion_sequence(0), expansion_sequence(5)];
        let reversions = vec![reversion_sequence(0), reversion_sequence(5)];
        let cluster =
            train_hmm_sequence_cluster(&[expansions.as_slice(), reversions.as_slice()], 2).unwrap();
        let class = classify_pda_sequence(&expansion_sequence(20), &cluster).unwrap();
        assert_eq!(class.posterior.len(), 2);
        let sum: f64 = class.posterior.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-6,
            "posterior must sum to 1, got {sum}"
        );
        for value in &class.posterior {
            assert!(
                (0.0..=1.0).contains(value),
                "posterior entries must live in [0, 1]"
            );
        }
    }

    #[test]
    fn classifier_rejects_empty_sequence() {
        let expansions = vec![expansion_sequence(0), expansion_sequence(5)];
        let reversions = vec![reversion_sequence(0), reversion_sequence(5)];
        let cluster =
            train_hmm_sequence_cluster(&[expansions.as_slice(), reversions.as_slice()], 2).unwrap();
        assert!(classify_pda_sequence(&[], &cluster).is_err());
    }
}
