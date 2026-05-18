use crate::agent::PROMPT_PACK_VERSION;
use crate::config::{compute_hash, family_history_window};
use crate::state::{DatasetComparability, LearningState, RunProvenance};
use crate::types::Candle;

pub fn dataset_comparability(
    previous_run_id: Option<String>,
    previous: Option<&RunProvenance>,
    current: &RunProvenance,
) -> DatasetComparability {
    match previous {
        None => DatasetComparability {
            comparable: false,
            previous_run_id,
            reason: "no_previous_run".to_string(),
            comparison_class: "no_previous_run".to_string(),
            same_data: false,
            same_config: false,
            same_prompt_version: false,
            same_factor_version: false,
        },
        Some(previous) if previous.data_fingerprint == current.data_fingerprint => {
            let same_config = previous.config_hash == current.config_hash;
            DatasetComparability {
                comparable: true,
                previous_run_id,
                reason: if same_config {
                    "same_data_same_config".to_string()
                } else {
                    "same_data_different_config".to_string()
                },
                comparison_class: if same_config {
                    "same_data_same_config".to_string()
                } else {
                    "same_data_different_config".to_string()
                },
                same_data: true,
                same_config,
                same_prompt_version: previous.prompt_version == current.prompt_version,
                same_factor_version: previous.factor_version == current.factor_version,
            }
        }
        Some(previous) => DatasetComparability {
            comparable: false,
            previous_run_id,
            reason: "different_data_fingerprint".to_string(),
            comparison_class: "different_data_fingerprint".to_string(),
            same_data: false,
            same_config: false,
            same_prompt_version: previous.prompt_version == current.prompt_version,
            same_factor_version: previous.factor_version == current.factor_version,
        },
    }
}

pub fn decision_thresholds() -> crate::state::DecisionThresholds {
    crate::state::DecisionThresholds::default()
}

pub fn data_fingerprint(
    candles: &[Candle],
    paired_candles: Option<&[Candle]>,
    source_tag: &str,
) -> String {
    let mut parts = vec![
        source_tag.to_string(),
        candles.len().to_string(),
        candles
            .first()
            .map(|candle| candle.timestamp.to_rfc3339())
            .unwrap_or_default(),
        candles
            .last()
            .map(|candle| candle.timestamp.to_rfc3339())
            .unwrap_or_default(),
        candles
            .first()
            .map(|candle| format!("{:.6}", candle.close))
            .unwrap_or_default(),
        candles
            .last()
            .map(|candle| format!("{:.6}", candle.close))
            .unwrap_or_default(),
    ];

    if let Some(paired) = paired_candles {
        parts.push(format!("paired:{}", paired.len()));
        parts.push(
            paired
                .first()
                .map(|candle| candle.timestamp.to_rfc3339())
                .unwrap_or_default(),
        );
        parts.push(
            paired
                .last()
                .map(|candle| candle.timestamp.to_rfc3339())
                .unwrap_or_default(),
        );
    }

    compute_hash(&parts)
}

pub fn factor_version(learning_state: &LearningState) -> String {
    let parts = learning_state
        .factor_profiles
        .iter()
        .map(|(name, profile)| {
            format!(
                "{}:{}:{:.6}:{:.6}:{:?}:{:?}",
                name,
                profile.enabled,
                profile.base_weight,
                profile.posterior_reliability,
                profile.parameters,
                profile.regime_stats
            )
        })
        .collect::<Vec<_>>();
    compute_hash(&parts)
}

pub fn run_provenance(
    learning_state: &LearningState,
    config_hash_source: &[impl AsRef<str>],
    data_fingerprint: String,
) -> RunProvenance {
    let mut config_parts = config_hash_source
        .iter()
        .map(|part| part.as_ref().to_string())
        .collect::<Vec<_>>();
    config_parts.push(format!("family_history_window={}", family_history_window()));
    RunProvenance {
        prompt_version: PROMPT_PACK_VERSION.to_string(),
        factor_version: factor_version(learning_state),
        config_hash: compute_hash(&config_parts),
        data_fingerprint,
    }
}
