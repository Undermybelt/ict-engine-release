pub mod consumer_bundle_adapter;
pub mod multi_timeframe_training;
pub mod native_frame_aggregation;
pub mod native_frame_analysis;
pub mod persistence;
pub mod recovery;

pub use multi_timeframe_training::build_multi_timeframe_training_observations;
pub use native_frame_aggregation::{
    native_frame_weight, weighted_majority_label, weighted_regime_probs,
};
pub use native_frame_analysis::{native_frame_computations, NativeFrameComputation};
pub use persistence::{
    build_mece_recovery_artifact, load_or_init_hmm_params_with_numeric_artifact,
    persist_mece_recovery_artifact, HmmNumericTrainerArtifact, HMM_NUMERIC_TRAINER_ARTIFACT_FILE,
    HMM_STATE_FILE, MECE_RECOVERY_ARTIFACT_FILE,
};
pub use recovery::{search_factors_for_mece_recovery, MeceRecoveryReport};
