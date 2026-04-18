use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::PolicyDecisionArtifact;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PolicyFeatureVector {
    pub factor_alignment: String,
    pub factor_uncertainty: String,
    pub gating_status: String,
    pub selected_entry_quality: String,
    pub recommended_command: String,
    pub evidence_quality_score: f64,
    pub selected_direction: String,
    pub risk_reward: f64,
    pub kelly_fraction: f64,
    pub setup_family: String,
    pub entry_style: String,
    pub risk_template: String,
    pub setup_quality: String,
    pub signal_bar_pattern: String,
    pub session_model: String,
    pub higher_tf_bias_match: bool,
    pub discount_premium_correct: bool,
    pub liquidity_swept: bool,
    pub signal_bar_present: bool,
    pub pda_signal_overlap: bool,
    pub timed_pda_active_nearby: bool,
    pub timed_pda_inversed_nearby: bool,
    pub timed_pda_stale_nearby: bool,
    pub pda_distance_bps: f64,
    pub pda_width_bps: f64,
    pub overlap_ratio: f64,
    pub displacement_strength: f64,
    pub sweep_depth_bps: f64,
    pub entry_price_offset_bps: f64,
    pub sl_distance_bps: f64,
    pub tp_rr_ratio: f64,

    // ── Flowtree-derived ICT features ──────────────────────────────
    // Phase 0: macro context
    /// ATR consumption ratio (current move / daily ATR). >0.8 = veto zone.
    #[serde(default)]
    pub atr_consumption_ratio: f64,
    /// Distance to HTF draw-on-liquidity target, normalised 0-1 (0 = at target).
    #[serde(default)]
    pub htf_dol_distance_ratio: f64,

    // Phase 1: HTF sweep & RB classification
    /// HTF EQX sweep detected (buy-side or sell-side liquidity taken).
    #[serde(default)]
    pub htf_eqx_swept: bool,
    /// HTF rejection-block type after sweep: "strong" / "chop" / "weak" / "none".
    #[serde(default)]
    pub htf_rb_type: String,

    // Phase 2: LTF event sequence tracking
    /// Consecutive bearish event-B count (看跌 FVG/CISD). ≥3 = three-strikes.
    #[serde(default)]
    pub event_b_consecutive_count: u8,
    /// Bullish event-A sequence completion: 0=none, 1=CISD, 2=+iFVG, 3=+MSS.
    #[serde(default)]
    pub event_a_sequence_stage: u8,
    /// LTF path classification: "classic_double_sweep" / "smt_washout" /
    /// "v_reversal" / "trend_continuation_fail" / "none".
    #[serde(default)]
    pub ltf_path_label: String,

    // Phase 2 detail: OTE & structure
    /// Close position relative to 0.705 OTE level: >0 = above, <0 = below.
    #[serde(default)]
    pub ote_0705_offset: f64,
    /// Recent BOS/CHoCH count within lookback window.
    #[serde(default)]
    pub structure_break_count: u8,
    /// Latest structure break type: "bos" / "choch" / "none".
    #[serde(default)]
    pub latest_break_type: String,

    // Phase 3: multi-TF fractal sync
    /// LTF MSS driving HTF CISD (fractal sync confirmed).
    #[serde(default)]
    pub fractal_sync_confirmed: bool,
    /// Kill-switch: high-level RB+CISD+FVG+MSS completion (0-4).
    #[serde(default)]
    pub killswitch_completion: u8,

    // ICT structure counts (from ICTStructureSummary)
    /// Open (unfilled) FVG count.
    #[serde(default)]
    pub fvgs_open: u8,
    /// Nearby untested order-block count.
    #[serde(default)]
    pub order_blocks_nearby: u8,
    /// LTF CISD confirmed.
    #[serde(default)]
    pub cisd_ltf_confirmed: bool,
    /// HTF CISD confirmed.
    #[serde(default)]
    pub cisd_htf_confirmed: bool,
    /// Rejection-block / pinbar detected on current bar.
    #[serde(default)]
    pub rb_pinbar_detected: bool,
    /// Bull-side PDA count.
    #[serde(default)]
    pub pda_bull_count: u8,
    /// Liquidity sweep count in recent window.
    #[serde(default)]
    pub liquidity_sweep_count: u8,

    // Regime transition signal (flowtree3)
    /// Red-alert state: order-flow broken, pending confirmation.
    #[serde(default)]
    pub red_alert_active: bool,
    /// Bull recovery event-A streak during red-alert (≥3 = alert cleared).
    #[serde(default)]
    pub recovery_event_a_streak: u8,
    /// PDA survival regime: "bear" / "chop" / "bull_continuation" / "unknown".
    #[serde(default)]
    pub pda_survival_regime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CatBoostTreeSplit {
    pub feature: String,
    pub split_type: String,
    pub threshold: Option<f64>,
    pub category_match: Option<String>,
    pub yes: usize,
    pub no: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CatBoostLeafOutput {
    pub leaf_id: String,
    pub action: String,
    pub qualification: String,
    #[serde(default)]
    pub gating_status: Option<String>,
    #[serde(default)]
    pub selected_direction: Option<String>,
    #[serde(default)]
    pub factor_alignment: Option<String>,
    #[serde(default)]
    pub selected_entry_quality: Option<String>,
    pub confidence_band: String,
    pub recommended_command: Option<String>,
    pub invalidation_triggers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CatBoostPolicyModelArtifact {
    pub artifact_version: String,
    pub model_family: String,
    pub target_kind: String,
    pub feature_schema_version: String,
    pub categorical_features: Vec<String>,
    pub numerical_features: Vec<String>,
    pub target_label_space: Vec<String>,
    pub trees: Vec<CatBoostTreeSplit>,
    pub leaf_outputs: Vec<CatBoostLeafOutput>,
    pub notes: Vec<String>,
}

pub trait PolicyEngine {
    fn engine_name(&self) -> &'static str;
    fn artifact_version(&self) -> &str;
    fn infer(&self, features: &PolicyFeatureVector) -> PolicyDecisionArtifact;
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CatBoostCompatiblePolicyEngine {
    pub model_artifact: CatBoostPolicyModelArtifact,
}

impl CatBoostCompatiblePolicyEngine {
    pub fn default_policy_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/application/orchestration/catboost_policy.sample.json")
    }

    pub fn load_default_or_placeholder() -> Self {
        Self::load_from_file(&Self::default_policy_path()).unwrap_or_else(|_| Self::placeholder())
    }

    pub fn placeholder() -> Self {
        Self {
            model_artifact: CatBoostPolicyModelArtifact {
                artifact_version: "catboost-policy-placeholder-v0".to_string(),
                model_family: "catboost".to_string(),
                target_kind: "post_bbn_policy_action".to_string(),
                feature_schema_version: "policy_features_v2_execution_setup".to_string(),
                categorical_features: vec![
                    "factor_alignment".to_string(),
                    "factor_uncertainty".to_string(),
                    "gating_status".to_string(),
                    "selected_entry_quality".to_string(),
                    "recommended_command".to_string(),
                    "selected_direction".to_string(),
                    "setup_family".to_string(),
                    "entry_style".to_string(),
                    "risk_template".to_string(),
                    "setup_quality".to_string(),
                    "signal_bar_pattern".to_string(),
                    "session_model".to_string(),
                    "htf_rb_type".to_string(),
                    "ltf_path_label".to_string(),
                    "latest_break_type".to_string(),
                    "pda_survival_regime".to_string(),
                ],
                numerical_features: vec![
                    "evidence_quality_score".to_string(),
                    "risk_reward".to_string(),
                    "kelly_fraction".to_string(),
                    "pda_distance_bps".to_string(),
                    "pda_width_bps".to_string(),
                    "overlap_ratio".to_string(),
                    "displacement_strength".to_string(),
                    "sweep_depth_bps".to_string(),
                    "entry_price_offset_bps".to_string(),
                    "sl_distance_bps".to_string(),
                    "tp_rr_ratio".to_string(),
                    "atr_consumption_ratio".to_string(),
                    "htf_dol_distance_ratio".to_string(),
                    "ote_0705_offset".to_string(),
                    "event_b_consecutive_count".to_string(),
                    "event_a_sequence_stage".to_string(),
                    "structure_break_count".to_string(),
                    "killswitch_completion".to_string(),
                    "fvgs_open".to_string(),
                    "order_blocks_nearby".to_string(),
                    "pda_bull_count".to_string(),
                    "liquidity_sweep_count".to_string(),
                    "recovery_event_a_streak".to_string(),
                ],
                target_label_space: vec![
                    "Observe".to_string(),
                    "Bull".to_string(),
                    "Bear".to_string(),
                ],
                trees: Vec::new(),
                leaf_outputs: Vec::new(),
                notes: vec![
                    "catboost_schema_only_no_runtime".to_string(),
                    "post_bbn_policy_layer".to_string(),
                ],
            },
        }
    }

    pub fn from_artifact(model_artifact: CatBoostPolicyModelArtifact) -> Self {
        Self { model_artifact }
    }

    pub fn load_from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let artifact: CatBoostPolicyModelArtifact = serde_json::from_str(&content)?;
        Ok(Self::from_artifact(artifact))
    }
}

impl PolicyEngine for CatBoostCompatiblePolicyEngine {
    fn engine_name(&self) -> &'static str {
        "catboost-compatible-placeholder"
    }

    fn artifact_version(&self) -> &str {
        &self.model_artifact.artifact_version
    }

    fn infer(&self, features: &PolicyFeatureVector) -> PolicyDecisionArtifact {
        let qualification = if features.gating_status == "observe_only" {
            "disqualified"
        } else {
            "qualified"
        };
        let confidence_band = if features.evidence_quality_score >= 0.75 {
            "high"
        } else if features.evidence_quality_score >= 0.45 {
            "medium"
        } else {
            "low"
        };
        let leaf_output = self.model_artifact.leaf_outputs.iter().find(|leaf| {
            leaf.qualification == qualification
                && leaf
                    .gating_status
                    .as_ref()
                    .map(|value| value == &features.gating_status)
                    .unwrap_or(true)
                && leaf
                    .selected_direction
                    .as_ref()
                    .map(|value| value == &features.selected_direction)
                    .unwrap_or(
                        leaf.action == features.selected_direction
                            || leaf.action.eq_ignore_ascii_case("observe"),
                    )
                && leaf
                    .factor_alignment
                    .as_ref()
                    .map(|value| value == &features.factor_alignment)
                    .unwrap_or(true)
                && leaf
                    .selected_entry_quality
                    .as_ref()
                    .map(|value| value == &features.selected_entry_quality)
                    .unwrap_or(true)
        });
        let action = leaf_output
            .map(|leaf| leaf.action.clone())
            .unwrap_or_else(|| {
                if qualification == "qualified" && features.selected_direction != "Neutral" {
                    features.selected_direction.clone()
                } else {
                    "Observe".to_string()
                }
            });
        PolicyDecisionArtifact {
            policy_version: self.artifact_version().to_string(),
            action,
            qualification: qualification.to_string(),
            recommended_command: leaf_output
                .and_then(|leaf| leaf.recommended_command.clone())
                .unwrap_or_else(|| features.recommended_command.clone()),
            confidence_band: leaf_output
                .map(|leaf| leaf.confidence_band.clone())
                .unwrap_or_else(|| confidence_band.to_string()),
            leaf_id: leaf_output
                .map(|leaf| leaf.leaf_id.clone())
                .unwrap_or_else(|| {
                    format!(
                        "gate:{}|entry:{}|direction:{}",
                        features.gating_status,
                        features.selected_entry_quality,
                        features.selected_direction
                    )
                }),
            split_trace: vec![
                format!("gating_status={}", features.gating_status),
                format!("selected_entry_quality={}", features.selected_entry_quality),
                format!("factor_alignment={}", features.factor_alignment),
                format!("factor_uncertainty={}", features.factor_uncertainty),
            ],
            invalidation_triggers: vec![
                format!("gate_changes_from={}", features.gating_status),
                format!(
                    "entry_quality_changes_from={}",
                    features.selected_entry_quality
                ),
            ],
            summary: format!(
                "engine={} action={} qualification={} command={} confidence={}",
                self.engine_name(),
                if qualification == "qualified" && features.selected_direction != "Neutral" {
                    features.selected_direction.clone()
                } else {
                    "Observe".to_string()
                },
                qualification,
                features.recommended_command,
                confidence_band
            ),
        }
    }
}
