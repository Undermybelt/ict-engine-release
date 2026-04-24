//! NLP-inspired PDA sequence clustering (Phase 1 skeleton of
//! `docs/plans/nlp-inspired-pda-sequence-clustering-plan.md`).
//!
//! This module is intentionally standalone:
//! - no main.rs wiring
//! - no PreBayes / BBN hookup
//! - no trading decisions derived from cluster labels
//!
//! The schema (`PdaDtwClusterPacket`) is fixed first so downstream surfaces
//! (RegimeSegmentationPacket, reflection_bundle, factor-research output)
//! can start consuming it as soon as the PDA detection layer learns to emit
//! `PdaToken` sequences.

pub mod analysis;
pub mod cluster;
pub mod dtw;
pub mod emitter;
pub mod ensemble_cluster;
pub mod fcgr;
pub mod hmm_cluster;
pub mod kmedoids;
pub mod persistence;
pub mod token;

pub use analysis::{
    analyze_pda_sequences, summarize_pda_sequence_artifact, PdaSequenceAnalysisArtifact,
    PdaSequenceArtifactSummary, PDA_SEQUENCE_ANALYSIS_METHOD, PDA_SEQUENCE_DEFAULT_KMER_K,
};
pub use cluster::{cluster_pda_sequences, PdaDtwClusterPacket, PDA_DTW_CLUSTER_METHOD};
pub use dtw::{dtw_alignment, dtw_distance, dtw_distance_matrix, DtwAlignment};
pub use emitter::{
    emit_pda_sequence_from_candles, EMITTER_ATR_PERIOD, EMITTER_CISD_MIN_STRENGTH,
    EMITTER_LIQUIDITY_POOL_ATR_MULT, EMITTER_LIQUIDITY_POOL_MIN_TOUCHES,
    EMITTER_LIQUIDITY_SWEEP_RETURN_BARS, EMITTER_NEAR_SWEEP_WINDOW_BARS,
    EMITTER_OVERLAP_WINDOW_BARS, EMITTER_RB_BODY_WICK_RATIO, EMITTER_RB_MIN_RANGE_ATR,
    EMITTER_SWING_STRENGTH,
};
pub use ensemble_cluster::{
    align_labels_to_reference, ensemble_classify_sessions, PdaClusteringPacket,
    PDA_ENSEMBLE_METHOD, PDA_ENSEMBLE_VOTERS,
};
pub use fcgr::{
    fcgr_cluster_sessions, fcgr_cosine_distance, fcgr_cosine_distance_matrix,
    pda_sequence_to_fcgr_vector, PDA_KIND_COUNT,
};
pub use hmm_cluster::{
    classify_pda_sequence, encode_pda_token, encode_sequence, train_hmm_sequence_cluster,
    HmmSequenceClassification, HmmSequenceCluster, HMM_SEQUENCE_CLUSTER_METHOD, HMM_TRAIN_MAX_ITER,
    HMM_TRAIN_TOLERANCE, PDA_TOKEN_OBS_DIM,
};
pub use kmedoids::{pam_cluster, PamOutcome};
pub use persistence::{
    classify_pda_sequence_ledger_status, load_pda_sequence_analysis, persist_pda_sequence_analysis,
    PDA_SEQUENCE_ARTIFACT_FILE, PDA_SEQUENCE_CONSISTENCY_ACTIONABLE,
    PDA_SEQUENCE_SILHOUETTE_ACTIONABLE,
};
pub use token::{pda_token_cost, PdaToken, PdaTokenKind};
