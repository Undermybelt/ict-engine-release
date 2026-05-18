use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IctFactorSchema {
    pub schema_version: String,
    pub lightweight: bool,
    pub agent_friendly: bool,
    pub token_friendly: bool,
    pub guidance_friendly: bool,
    pub historical_backtest_path: bool,
    pub live_timeliness_split: bool,
    pub prior_postmortem_split: bool,
    pub factors: Vec<IctFactorDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IctFactorDefinition {
    pub name: String,
    pub family: String,
    pub value_type: String,
    pub cadence: String,
    pub required_inputs: Vec<String>,
    pub prior_fields: Vec<String>,
    pub postmortem_fields: Vec<String>,
    pub bbn_targets: Vec<String>,
}

pub fn default_ict_factor_schema() -> IctFactorSchema {
    IctFactorSchema {
        schema_version: "1.0.0".to_string(),
        lightweight: true,
        agent_friendly: true,
        token_friendly: true,
        guidance_friendly: true,
        historical_backtest_path: true,
        live_timeliness_split: true,
        prior_postmortem_split: true,
        factors: vec![
            IctFactorDefinition {
                name: "fvg_state".to_string(),
                family: "pda_timing".to_string(),
                value_type: "enum".to_string(),
                cadence: "per_bar".to_string(),
                required_inputs: vec!["high".into(), "low".into(), "close".into()],
                prior_fields: vec!["state".into(), "direction".into(), "distance_atr".into()],
                postmortem_fields: vec!["mitigation_progress".into(), "time_to_resolution".into()],
                bbn_targets: vec!["imbalance_context".into(), "entry_zone_quality".into()],
            },
            IctFactorDefinition {
                name: "liquidity_sweep_state".to_string(),
                family: "liquidity".to_string(),
                value_type: "enum".to_string(),
                cadence: "event".to_string(),
                required_inputs: vec!["swing_points".into(), "atr".into(), "close".into()],
                prior_fields: vec![
                    "state".into(),
                    "sweep_direction".into(),
                    "overshoot_atr".into(),
                ],
                postmortem_fields: vec!["reclaim_speed".into(), "followthrough".into()],
                bbn_targets: vec!["liquidity_event".into(), "reversal_readiness".into()],
            },
            IctFactorDefinition {
                name: "session_context_state".to_string(),
                family: "timing".to_string(),
                value_type: "struct".to_string(),
                cadence: "per_bar".to_string(),
                required_inputs: vec!["timestamp".into(), "session_calendar".into()],
                prior_fields: vec!["session_name".into(), "window_active".into()],
                postmortem_fields: vec!["session_outcome".into()],
                bbn_targets: vec!["execution_window".into(), "session_context".into()],
            },
        ],
    }
}

pub fn write_ict_factor_schema(path: impl AsRef<Path>) -> Result<()> {
    let schema = default_ict_factor_schema();
    fs::write(path, serde_json::to_string_pretty(&schema)?)?;
    Ok(())
}
