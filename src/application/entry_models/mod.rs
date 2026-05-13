use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

use crate::state::PreBayesEvidenceFilter;
use crate::types::Candle;

pub mod breaker_rb;
pub mod cisd_rb;
pub mod training_export;

pub type EntryModelPacketStore = BTreeMap<String, Value>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsumerDefaultMode {
    InternalTrainingOnly,
    AdoptableOptIn,
    AdoptedByDefault,
}

impl ConsumerDefaultMode {
    pub fn adopted_by_default(self) -> bool {
        matches!(self, Self::AdoptedByDefault)
    }

    pub fn effect_label(self) -> &'static str {
        match self {
            Self::InternalTrainingOnly => "internal_training_only",
            Self::AdoptableOptIn => "consumer_opt_in_available",
            Self::AdoptedByDefault => "consumer_default_enabled",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntryModelTrainingRows {
    pub provider_id: String,
    pub matched_rows: usize,
    pub bbn_training_filename: String,
    pub bbn_csv: String,
    pub catboost_training_filename: String,
    pub catboost_csv: String,
    pub summary_filename: String,
    pub summary_json: String,
}

pub trait EntryModelProvider {
    fn provider_id(&self) -> &'static str;
    fn consumer_default_mode(&self) -> ConsumerDefaultMode;
    fn build_analyze_packet(
        &self,
        _symbol: &str,
        _timeframe: &str,
        _candles: &[Candle],
        _filter: &PreBayesEvidenceFilter,
    ) -> Option<Value> {
        None
    }
    fn training_rows(
        &self,
        state_dir: &str,
        symbol: &str,
    ) -> anyhow::Result<EntryModelTrainingRows>;
    fn status_surface(
        &self,
        state_dir: &str,
        symbol: &str,
    ) -> anyhow::Result<self::training_export::PolicyTrainingProviderStatusSurface>;
}

pub fn insert_entry_model_packet<T: Serialize>(
    store: &mut EntryModelPacketStore,
    model_id: &str,
    packet: &T,
) -> serde_json::Result<()> {
    store.insert(model_id.to_string(), serde_json::to_value(packet)?);
    Ok(())
}

pub fn decode_entry_model_packet<T: DeserializeOwned>(
    store: &EntryModelPacketStore,
    model_id: &str,
) -> Option<T> {
    store
        .get(model_id)
        .and_then(|value| serde_json::from_value::<T>(value.clone()).ok())
}

pub fn entry_model_providers() -> Vec<Box<dyn EntryModelProvider>> {
    vec![
        Box::new(self::training_export::CisdRbEntryModelProvider),
        Box::new(self::training_export::BreakerRbEntryModelProvider),
    ]
}

pub fn find_entry_model_provider(provider_id: &str) -> Option<Box<dyn EntryModelProvider>> {
    entry_model_providers()
        .into_iter()
        .find(|provider| provider.provider_id() == provider_id)
}

pub fn build_entry_model_packet_store_for_analyze(
    symbol: &str,
    timeframe: &str,
    candles: &[Candle],
    filter: &PreBayesEvidenceFilter,
) -> EntryModelPacketStore {
    let mut store = EntryModelPacketStore::default();
    for provider in entry_model_providers() {
        if let Some(packet) = provider.build_analyze_packet(symbol, timeframe, candles, filter) {
            store.insert(provider.provider_id().to_string(), packet);
        }
    }
    store
}

pub use breaker_rb::{
    bin_breaker_rb_for_bbn, build_breaker_rb_catboost_feature_row,
    build_breaker_rb_entry_model_packet, BreakerRbBbnEvidence, BreakerRbBestParams,
    BreakerRbCatBoostFeatureRow, BreakerRbEntryModelPacket, BREAKER_RB_DEFAULT_BEST_PARAMS,
    BREAKER_RB_SETUP_MODEL_ID,
};
pub use cisd_rb::{
    apply_cisd_rb_to_belief_packet, apply_cisd_rb_to_policy_features, bin_cisd_rb_for_bbn,
    build_cisd_rb_catboost_feature_row, build_cisd_rb_entry_model_packet,
    build_cisd_rb_hmm_features, CisdRbBbnEvidence, CisdRbBestParams, CisdRbCatBoostFeatureRow,
    CisdRbEntryModelPacket, CISD_RB_DEFAULT_BEST_PARAMS, CISD_RB_HMM_FEATURE_DIM,
    CISD_RB_SETUP_MODEL_ID,
};
pub use training_export::{
    apply_structural_path_ranking_external_scores_command,
    clear_structural_path_ranking_trainer_artifact_command,
    disable_structural_path_ranking_runtime_command,
    enable_structural_path_ranking_runtime_command, export_policy_training_tables,
    export_structural_path_ranking_target_command, policy_training_status,
    policy_training_status_command, register_structural_path_ranking_trainer_artifact_command,
    BreakerRbEntryModelProvider, CisdRbBbnTrainingRow, CisdRbCatBoostTrainingRow,
    CisdRbEntryModelProvider, CisdRbTrainingExportSummary, CisdRbTrainingStatusSurface,
    FactorCandidatePackTrainingStatusSurface, PolicyTrainingProviderStatusSurface,
    PolicyTrainingStatusSurface, RegimeConfidenceAssetTrainingStatusSurface,
    CISD_RB_BBN_TRAINING_FILE, CISD_RB_CATBOOST_TRAINING_FILE, CISD_RB_TRAINING_SUMMARY_FILE,
    POLICY_TRAINING_DIR,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_registered_cisd_rb_provider() {
        let provider =
            find_entry_model_provider(CISD_RB_SETUP_MODEL_ID).expect("registered provider");
        assert_eq!(provider.provider_id(), CISD_RB_SETUP_MODEL_ID);
        assert_eq!(
            provider.consumer_default_mode(),
            ConsumerDefaultMode::InternalTrainingOnly
        );
    }

    #[test]
    fn finds_registered_breaker_rb_provider() {
        let provider =
            find_entry_model_provider(BREAKER_RB_SETUP_MODEL_ID).expect("registered provider");
        assert_eq!(provider.provider_id(), BREAKER_RB_SETUP_MODEL_ID);
        assert_eq!(
            provider.consumer_default_mode(),
            ConsumerDefaultMode::InternalTrainingOnly
        );
    }
}
