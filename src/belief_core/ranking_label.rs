use anyhow::Result;
use csv::StringRecord;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::belief_core::beta_dirichlet_update::{beta_posterior_lower_bound, beta_posterior_mean};

const STRUCTURAL_PATH_RANKING_RUNTIME_DIR: &str = "policy_training";
pub const STRUCTURAL_PATH_RANKING_IPS_WEIGHT_CLIP: f64 = 5.0;
pub const STRUCTURAL_PATH_RANKING_EXECUTION_GATE_MIN_PATH_PROB: f64 = 0.5;

pub const STRUCTURAL_PATH_RANKING_RUNTIME_SELECTION_FILE: &str =
    "structural_path_ranking_runtime_selection.json";
pub const STRUCTURAL_PATH_RANKING_RUNTIME_SELECTION_PROTOCOL_VERSION: &str =
    "structural-path-ranking-runtime-selection-v1";
pub const STRUCTURAL_PATH_RANKING_RUNTIME_MODE_CANDIDATE_SET_ONLY: &str = "candidate_set_only";
pub const STRUCTURAL_PATH_RANKING_RUNTIME_MODE_PREFER_HISTORY: &str = "prefer_history";
pub const STRUCTURAL_PATH_RANKER_DIRECT_MODEL_FAMILY_WEIGHTED_SUM_V1: &str =
    "weighted_feature_sum_v1";
pub const STRUCTURAL_PATH_RANKER_DIRECT_MODEL_FAMILY_LINEAR_SCORE_V1: &str =
    "linear_feature_score_v1";
pub const STRUCTURAL_PATH_RANKER_SERVICE_FAMILY_ROW_SCORING_V1: &str = "row_scoring_service_v1";
pub const STRUCTURAL_PATH_RANKER_EXPLICIT_FAMILY_CORELS: &str = "corels";
pub const STRUCTURAL_PATH_RANKER_EXPLICIT_FAMILY_GOSDT: &str = "gosdt";
pub const STRUCTURAL_PATH_RANKER_EXPLICIT_FAMILY_GA_MASK_TREE: &str = "ga_mask_tree";

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathRankingRuntimeSelection {
    pub protocol_version: String,
    pub enabled: bool,
    pub reuse_mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_at: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathRankerRuntimeSurface {
    pub enabled: bool,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reuse_mode: Option<String>,
    #[serde(default)]
    pub artifact_match_count: usize,
    #[serde(default)]
    pub candidate_set_match_count: usize,
    #[serde(default)]
    pub history_match_count: usize,
    #[serde(default)]
    pub applied_path_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_model_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_source_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_model_artifact_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_generator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathRankerRuntimeRow {
    pub candidate_set_id: String,
    pub path_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_path_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibrated_path_prob: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_prob_lower_bound: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_gate_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_model_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_source_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_model_artifact_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_generator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathRankingTrainerManifest {
    pub protocol_version: String,
    pub dataset_role: String,
    pub group_id_column: String,
    pub label_column: String,
    pub weight_column: String,
    pub maturity_column: String,
    pub raw_score_column: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feature_columns: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calibration_columns: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub guardrail_columns: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathRankingTargetExportSummary {
    pub symbol: String,
    pub rows: usize,
    pub candidate_set_id: String,
    pub candidate_set_size: usize,
    pub pending_reward_states: BTreeMap<String, usize>,
    #[serde(default)]
    pub mature_rows: usize,
    pub rows_with_raw_path_score: usize,
    pub rows_with_calibrated_path_prob: usize,
    pub rows_with_path_prob_lower_bound: usize,
    pub rows_with_propensity_estimate: usize,
    #[serde(default)]
    pub rows_with_execution_gate_status: usize,
    #[serde(default)]
    pub rows_with_training_weight: usize,
    pub csv_path: String,
    pub jsonl_path: String,
    #[serde(default)]
    pub history_csv_path: String,
    #[serde(default)]
    pub history_jsonl_path: String,
    #[serde(default)]
    pub history_rows: usize,
    #[serde(default)]
    pub history_mature_rows: usize,
    #[serde(default)]
    pub history_rows_with_raw_path_score: usize,
    #[serde(default)]
    pub history_rows_with_calibrated_path_prob: usize,
    #[serde(default)]
    pub history_rows_with_path_prob_lower_bound: usize,
    #[serde(default)]
    pub history_rows_with_propensity_estimate: usize,
    #[serde(default)]
    pub history_rows_with_training_weight: usize,
    pub summary_path: String,
    #[serde(default)]
    pub trainer_manifest: StructuralPathRankingTrainerManifest,
    pub summary_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathProbabilityCalibrationReport {
    pub status: String,
    pub observed_rows: usize,
    pub calibrated_rows: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bins: Vec<StructuralPathProbabilityCalibrationBin>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    pub summary_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathProbabilityCalibrationBin {
    pub regime_calibration_bucket: String,
    pub observations: usize,
    pub successes: usize,
    pub raw_path_score_min: f64,
    pub raw_path_score_max: f64,
    pub calibrated_path_prob: f64,
    pub path_prob_lower_bound: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathProbabilityCalibrationEvaluationReport {
    pub status: String,
    pub eligible_rows: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brier_score: Option<f64>,
    #[serde(default)]
    pub propensity_weighted_rows: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub propensity_weighted_brier_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_calibration_error: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_calibration_error: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bins: Vec<StructuralPathProbabilityCalibrationEvaluationBin>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    pub summary_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathProbabilityCalibrationEvaluationBin {
    pub regime_calibration_bucket: String,
    pub observations: usize,
    pub mean_calibrated_path_prob: f64,
    pub empirical_success_rate: f64,
    pub absolute_error: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralPathRankingTargetArtifact {
    pub protocol_version: String,
    pub symbol: String,
    pub candidate_set_id: String,
    pub candidate_set_size: usize,
    pub generated_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<StructuralPathRankingTargetRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralPathRankingTargetRow {
    pub rank: usize,
    pub candidate_set_id: String,
    pub candidate_set_size: usize,
    pub path_id: String,
    pub scenario_id: String,
    pub path_label: String,
    pub direction: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_path_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibrated_path_prob: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_prob_lower_bound: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_gate_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_gate_min_path_prob: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_gate_reason: Option<String>,
    pub pending_reward_state: String,
    #[serde(default)]
    pub maturity_mask: bool,
    #[serde(default)]
    pub maturity_weight: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibrated_label: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub propensity_estimate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ips_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub training_weight: Option<f64>,
    pub regime_calibration_bucket: String,
    pub behavior_policy_probability: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_propensity: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_policy_probability_confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_policy_probability_lower_bound: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_policy_reward_prior: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_policy_reward_lower_bound: Option<f64>,
    pub experience_prior: f64,
    pub current_posterior: f64,
    pub structural_baseline_score: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_model_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_source_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_model_artifact_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_generator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathRankingExternalScoreInput {
    pub candidate_set_id: String,
    pub path_id: String,
    pub raw_path_score: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_model_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_source_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_model_artifact_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_generator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathRankerRuntimeArtifactMetadata {
    #[serde(default)]
    pub protocol_version: String,
    #[serde(default)]
    pub dataset_role: String,
    #[serde(default)]
    pub model_family: String,
    #[serde(default)]
    pub artifact_uri: String,
    #[serde(default)]
    pub score_column: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathRankerDirectModelArtifact {
    #[serde(default)]
    pub protocol_version: String,
    #[serde(default)]
    pub model_family: String,
    #[serde(default)]
    pub feature_schema_version: String,
    #[serde(default)]
    pub output_transform: String,
    #[serde(default)]
    pub intercept: f64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub numerical_feature_weights: BTreeMap<String, f64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub categorical_feature_weights: BTreeMap<String, BTreeMap<String, f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lower_bound_margin: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_gate_min_path_prob: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathRankerValidationMetrics {
    #[serde(default)]
    pub raw_scored_mature_rows: usize,
    #[serde(default)]
    pub raw_scored_mature_min_rows: usize,
    #[serde(default)]
    pub production_validation_rows: usize,
    #[serde(default)]
    pub production_validation_min_rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathRankerCalibrationMetrics {
    #[serde(default)]
    pub eligible_rows: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brier_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub propensity_weighted_brier_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_calibration_error: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_calibration_error: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathRankerRuleCondition {
    #[serde(default)]
    pub feature: String,
    #[serde(default)]
    pub operator: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numeric_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub string_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathRankerRule {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<StructuralPathRankerRuleCondition>,
    #[serde(default)]
    pub score: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_prob_lower_bound: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_gate_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathRankerTreeNode {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numeric_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub string_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<Box<StructuralPathRankerTreeNode>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right: Option<Box<StructuralPathRankerTreeNode>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_prob_lower_bound: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_gate_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct StructuralPathRankerExplicitArtifact {
    #[serde(default)]
    pub protocol_version: String,
    #[serde(default)]
    pub dataset_role: String,
    #[serde(default)]
    pub model_family: String,
    #[serde(default)]
    pub artifact_uri: String,
    #[serde(default)]
    pub score_column: String,
    #[serde(default)]
    pub trained_rows: usize,
    #[serde(default)]
    pub history_rows: usize,
    #[serde(
        default,
        alias = "feature_columns",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub selected_features: Vec<String>,
    #[serde(default)]
    pub validation_metrics: StructuralPathRankerValidationMetrics,
    #[serde(default)]
    pub calibration_metrics: StructuralPathRankerCalibrationMetrics,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rule_list: Vec<StructuralPathRankerRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree_json: Option<StructuralPathRankerTreeNode>,
}

#[derive(Debug, Clone, Serialize)]
struct StructuralPathRankerServiceRequest<'a> {
    protocol_version: &'static str,
    symbol: &'a str,
    candidate_set_id: &'a str,
    score_column: &'a str,
    rows: &'a [StructuralPathRankingTargetRow],
}

pub fn structural_path_ranking_runtime_selection_path(state_dir: &str, symbol: &str) -> String {
    Path::new(state_dir)
        .join(symbol)
        .join(STRUCTURAL_PATH_RANKING_RUNTIME_DIR)
        .join(STRUCTURAL_PATH_RANKING_RUNTIME_SELECTION_FILE)
        .to_string_lossy()
        .to_string()
}

pub fn load_structural_path_ranking_runtime_selection(
    state_dir: &str,
    symbol: &str,
) -> Option<StructuralPathRankingRuntimeSelection> {
    let path = structural_path_ranking_runtime_selection_path(state_dir, symbol);
    let raw = fs::read_to_string(path).ok()?;
    let selection = serde_json::from_str::<StructuralPathRankingRuntimeSelection>(&raw).ok()?;
    if selection.protocol_version.trim()
        != STRUCTURAL_PATH_RANKING_RUNTIME_SELECTION_PROTOCOL_VERSION
    {
        return None;
    }
    if !matches!(
        selection.reuse_mode.as_str(),
        STRUCTURAL_PATH_RANKING_RUNTIME_MODE_CANDIDATE_SET_ONLY
            | STRUCTURAL_PATH_RANKING_RUNTIME_MODE_PREFER_HISTORY
    ) {
        return None;
    }
    Some(selection)
}

pub fn structural_path_ranker_supports_direct_model_family(model_family: &str) -> bool {
    matches!(
        model_family.trim(),
        STRUCTURAL_PATH_RANKER_DIRECT_MODEL_FAMILY_WEIGHTED_SUM_V1
            | STRUCTURAL_PATH_RANKER_DIRECT_MODEL_FAMILY_LINEAR_SCORE_V1
    )
}

pub fn structural_path_ranker_supports_service_family(model_family: &str) -> bool {
    matches!(
        model_family.trim(),
        STRUCTURAL_PATH_RANKER_SERVICE_FAMILY_ROW_SCORING_V1
    )
}

pub fn structural_path_ranker_supports_explicit_family(model_family: &str) -> bool {
    matches!(
        model_family.trim(),
        STRUCTURAL_PATH_RANKER_EXPLICIT_FAMILY_CORELS
            | STRUCTURAL_PATH_RANKER_EXPLICIT_FAMILY_GOSDT
            | STRUCTURAL_PATH_RANKER_EXPLICIT_FAMILY_GA_MASK_TREE
    )
}

fn structural_path_ranker_artifact_json_path(state_dir: &str, symbol: &str) -> PathBuf {
    Path::new(state_dir)
        .join(symbol)
        .join(STRUCTURAL_PATH_RANKING_RUNTIME_DIR)
        .join("structural_path_ranking_trainer_artifact.json")
}

pub fn load_structural_path_ranker_runtime_artifact_metadata(
    state_dir: &str,
    symbol: &str,
) -> Option<StructuralPathRankerRuntimeArtifactMetadata> {
    let path = structural_path_ranker_artifact_json_path(state_dir, symbol);
    let raw = fs::read_to_string(path).ok()?;
    let artifact =
        serde_json::from_str::<StructuralPathRankerRuntimeArtifactMetadata>(&raw).ok()?;
    if artifact.artifact_uri.trim().is_empty() || artifact.score_column.trim().is_empty() {
        return None;
    }
    Some(artifact)
}

pub fn load_structural_path_ranker_runtime_artifact_ref(
    state_dir: &str,
    symbol: &str,
) -> Option<(String, String)> {
    let artifact = load_structural_path_ranker_runtime_artifact_metadata(state_dir, symbol)?;
    Some((artifact.artifact_uri, artifact.score_column))
}

fn structural_path_ranker_artifact_uri_path(
    state_dir: &str,
    symbol: &str,
    artifact_uri: &str,
) -> Option<PathBuf> {
    let artifact_uri = artifact_uri.trim();
    if artifact_uri.is_empty() {
        return None;
    }
    if let Some(path) = artifact_uri.strip_prefix("file://") {
        return Some(PathBuf::from(path));
    }
    if artifact_uri.contains("://") {
        return None;
    }
    let path = Path::new(artifact_uri);
    if path.is_absolute() {
        Some(path.to_path_buf())
    } else {
        Some(
            Path::new(state_dir)
                .join(symbol)
                .join(STRUCTURAL_PATH_RANKING_RUNTIME_DIR)
                .join(path),
        )
    }
}

fn load_structural_path_ranker_runtime_artifact_raw(
    state_dir: &str,
    symbol: &str,
    artifact_uri: &str,
) -> Result<Option<(String, String)>> {
    if structural_path_ranker_runtime_source_kind(artifact_uri) == "remote" {
        let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
        let raw = client
            .get(artifact_uri)
            .send()?
            .error_for_status()?
            .text()?;
        return Ok(Some((artifact_uri.to_string(), raw)));
    }
    let artifact_path =
        structural_path_ranker_artifact_uri_path(state_dir, symbol, artifact_uri)
            .ok_or_else(|| anyhow::anyhow!("artifact uri is not a supported local path"))?;
    if !artifact_path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&artifact_path)?;
    Ok(Some((artifact_path.to_string_lossy().to_string(), raw)))
}

fn structural_path_ranker_runtime_source_kind(artifact_uri: &str) -> &'static str {
    let artifact_uri = artifact_uri.trim();
    if artifact_uri.starts_with("http://") || artifact_uri.starts_with("https://") {
        "remote"
    } else {
        "local"
    }
}

fn structural_path_ranker_runtime_source_extension(source_hint: &str) -> String {
    let trimmed = source_hint.trim();
    let path_like = trimmed
        .split('?')
        .next()
        .unwrap_or(trimmed)
        .split('#')
        .next()
        .unwrap_or(trimmed);
    Path::new(path_like)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn non_empty_json_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn structural_path_ranker_runtime_row_from_value(
    value: &Value,
    score_column: &str,
) -> Option<StructuralPathRankerRuntimeRow> {
    let candidate_set_id = value.get("candidate_set_id")?.as_str()?.trim().to_string();
    let path_id = value.get("path_id")?.as_str()?.trim().to_string();
    let raw_path_score = value.get(score_column).and_then(Value::as_f64);
    let calibrated_path_prob = value.get("calibrated_path_prob").and_then(Value::as_f64);
    let path_prob_lower_bound = value.get("path_prob_lower_bound").and_then(Value::as_f64);
    let execution_gate_status = non_empty_json_string(value, "execution_gate_status");
    Some(StructuralPathRankerRuntimeRow {
        candidate_set_id,
        path_id,
        raw_path_score,
        calibrated_path_prob,
        path_prob_lower_bound,
        execution_gate_status,
        score_model_family: non_empty_json_string(value, "score_model_family"),
        score_source_kind: non_empty_json_string(value, "score_source_kind"),
        score_model_artifact_uri: non_empty_json_string(value, "score_model_artifact_uri"),
        score_generator: non_empty_json_string(value, "score_generator"),
    })
}

fn structural_path_ranker_runtime_row_from_csv_record(
    headers: &StringRecord,
    record: &StringRecord,
    score_column: &str,
) -> Option<StructuralPathRankerRuntimeRow> {
    let value_for = |name: &str| -> Option<&str> {
        let index = headers.iter().position(|header| header == name)?;
        record.get(index)
    };
    let candidate_set_id = value_for("candidate_set_id")?.trim().to_string();
    let path_id = value_for("path_id")?.trim().to_string();
    let parse_f64 = |name: &str| -> Option<f64> { value_for(name)?.trim().parse::<f64>().ok() };
    let optional_string = |name: &str| -> Option<String> {
        value_for(name)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
    };
    Some(StructuralPathRankerRuntimeRow {
        candidate_set_id,
        path_id,
        raw_path_score: parse_f64(score_column),
        calibrated_path_prob: parse_f64("calibrated_path_prob"),
        path_prob_lower_bound: parse_f64("path_prob_lower_bound"),
        execution_gate_status: optional_string("execution_gate_status"),
        score_model_family: optional_string("score_model_family"),
        score_source_kind: optional_string("score_source_kind"),
        score_model_artifact_uri: optional_string("score_model_artifact_uri"),
        score_generator: optional_string("score_generator"),
    })
}

fn parse_structural_path_ranker_runtime_rows_from_raw(
    source_hint: &str,
    raw: &str,
    score_column: &str,
) -> Result<Vec<StructuralPathRankerRuntimeRow>> {
    match structural_path_ranker_runtime_source_extension(source_hint).as_str() {
        "jsonl" => Ok(raw
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter_map(|value| structural_path_ranker_runtime_row_from_value(&value, score_column))
            .collect()),
        "json" => {
            let value = serde_json::from_str::<Value>(raw)?;
            let rows = match value {
                Value::Array(items) => items
                    .iter()
                    .filter_map(|item| {
                        structural_path_ranker_runtime_row_from_value(item, score_column)
                    })
                    .collect(),
                Value::Object(ref object)
                    if object.get("rows").and_then(Value::as_array).is_some() =>
                {
                    object
                        .get("rows")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(|item| {
                            structural_path_ranker_runtime_row_from_value(item, score_column)
                        })
                        .collect()
                }
                Value::Object(_) => {
                    structural_path_ranker_runtime_row_from_value(&value, score_column)
                        .into_iter()
                        .collect()
                }
                _ => Vec::new(),
            };
            Ok(rows)
        }
        _ => {
            let mut reader = csv::Reader::from_reader(raw.as_bytes());
            let headers = reader.headers()?.clone();
            Ok(reader
                .records()
                .filter_map(|record| record.ok())
                .filter_map(|record| {
                    structural_path_ranker_runtime_row_from_csv_record(
                        &headers,
                        &record,
                        score_column,
                    )
                })
                .collect())
        }
    }
}

pub fn load_structural_path_ranker_runtime_artifact_rows(
    state_dir: &str,
    symbol: &str,
    artifact_uri: &str,
    score_column: &str,
) -> Result<Vec<StructuralPathRankerRuntimeRow>> {
    let Some((source_hint, raw)) =
        load_structural_path_ranker_runtime_artifact_raw(state_dir, symbol, artifact_uri)?
    else {
        return Ok(Vec::new());
    };
    parse_structural_path_ranker_runtime_rows_from_raw(&source_hint, &raw, score_column)
}

fn structural_path_ranker_row_numeric_feature(
    row: &StructuralPathRankingTargetRow,
    feature_name: &str,
) -> Option<f64> {
    match feature_name.trim() {
        "rank" => Some(row.rank as f64),
        "candidate_set_size" => Some(row.candidate_set_size as f64),
        "behavior_policy_probability" => Some(row.behavior_policy_probability),
        "execution_propensity" => row.execution_propensity,
        "target_policy_probability_confidence" => row.target_policy_probability_confidence,
        "target_policy_probability_lower_bound" => row.target_policy_probability_lower_bound,
        "target_policy_reward_prior" => row.target_policy_reward_prior,
        "target_policy_reward_lower_bound" => row.target_policy_reward_lower_bound,
        "experience_prior" => Some(row.experience_prior),
        "current_posterior" => Some(row.current_posterior),
        "structural_baseline_score" => Some(row.structural_baseline_score),
        "maturity_weight" => Some(row.maturity_weight),
        "propensity_estimate" => row.propensity_estimate,
        "ips_weight" => row.ips_weight,
        "training_weight" => row.training_weight,
        "calibrated_label" => row.calibrated_label,
        "raw_path_score" => row.raw_path_score,
        "calibrated_path_prob" => row.calibrated_path_prob,
        "path_prob_lower_bound" => row.path_prob_lower_bound,
        _ => None,
    }
}

fn structural_path_ranker_row_categorical_feature<'a>(
    row: &'a StructuralPathRankingTargetRow,
    feature_name: &str,
) -> Option<&'a str> {
    match feature_name.trim() {
        "direction" => Some(row.direction.as_str()),
        "regime_calibration_bucket" => Some(row.regime_calibration_bucket.as_str()),
        "path_id" => Some(row.path_id.as_str()),
        "scenario_id" => Some(row.scenario_id.as_str()),
        "pending_reward_state" => Some(row.pending_reward_state.as_str()),
        "execution_gate_status" => row.execution_gate_status.as_deref(),
        _ => None,
    }
}

fn structural_path_ranker_direct_model_probability(
    model: &StructuralPathRankerDirectModelArtifact,
    row: &StructuralPathRankingTargetRow,
) -> f64 {
    let mut score = model.intercept;
    for (feature_name, weight) in &model.numerical_feature_weights {
        if let Some(value) = structural_path_ranker_row_numeric_feature(row, feature_name) {
            score += value * *weight;
        }
    }
    for (feature_name, weights) in &model.categorical_feature_weights {
        if let Some(value) = structural_path_ranker_row_categorical_feature(row, feature_name) {
            score += weights
                .get(value)
                .copied()
                .or_else(|| weights.get("*").copied())
                .unwrap_or_default();
        }
    }
    match model.output_transform.trim() {
        "" | "sigmoid" => (1.0 / (1.0 + (-score).exp())).clamp(0.0, 1.0),
        "identity" | "clamp_01" | "identity_clamped" => score.clamp(0.0, 1.0),
        _ => score.clamp(0.0, 1.0),
    }
}

fn structural_path_ranker_condition_matches(
    condition: &StructuralPathRankerRuleCondition,
    row: &StructuralPathRankingTargetRow,
) -> bool {
    let operator = condition.operator.trim().to_ascii_lowercase();
    if let Some(value) = structural_path_ranker_row_numeric_feature(row, &condition.feature) {
        let rhs = condition.numeric_value.unwrap_or_default();
        return match operator.as_str() {
            "gt" => value > rhs,
            "ge" | ">=" => value >= rhs,
            "lt" => value < rhs,
            "le" | "<=" => value <= rhs,
            "eq" | "==" => (value - rhs).abs() <= 1e-9,
            "neq" | "!=" => (value - rhs).abs() > 1e-9,
            _ => false,
        };
    }
    if let Some(value) = structural_path_ranker_row_categorical_feature(row, &condition.feature) {
        let rhs = condition.string_value.as_deref().unwrap_or_default();
        return match operator.as_str() {
            "eq" | "==" => value == rhs,
            "neq" | "!=" => value != rhs,
            _ => false,
        };
    }
    false
}

fn structural_path_ranker_rule_matches(
    rule: &StructuralPathRankerRule,
    row: &StructuralPathRankingTargetRow,
) -> bool {
    rule.conditions
        .iter()
        .all(|condition| structural_path_ranker_condition_matches(condition, row))
}

fn structural_path_ranker_tree_leaf_row(
    row: &StructuralPathRankingTargetRow,
    score: f64,
    path_prob_lower_bound: Option<f64>,
    execution_gate_status: Option<String>,
    model_family: Option<String>,
    source_kind: Option<String>,
) -> StructuralPathRankerRuntimeRow {
    let score = score.clamp(0.0, 1.0);
    StructuralPathRankerRuntimeRow {
        candidate_set_id: row.candidate_set_id.clone(),
        path_id: row.path_id.clone(),
        raw_path_score: Some(score),
        calibrated_path_prob: Some(score),
        path_prob_lower_bound,
        execution_gate_status: Some(execution_gate_status.unwrap_or_else(|| {
            if path_prob_lower_bound.unwrap_or(score)
                >= STRUCTURAL_PATH_RANKING_EXECUTION_GATE_MIN_PATH_PROB
            {
                "pass".to_string()
            } else {
                "observe".to_string()
            }
        })),
        score_model_family: model_family,
        score_source_kind: source_kind,
        score_model_artifact_uri: None,
        score_generator: None,
    }
}

fn structural_path_ranker_tree_evaluate(
    node: &StructuralPathRankerTreeNode,
    row: &StructuralPathRankingTargetRow,
) -> Option<StructuralPathRankerRuntimeRow> {
    if let Some(score) = node.score {
        return Some(structural_path_ranker_tree_leaf_row(
            row,
            score,
            node.path_prob_lower_bound,
            node.execution_gate_status.clone(),
            None,
            Some("explicit_artifact".to_string()),
        ));
    }
    let condition = StructuralPathRankerRuleCondition {
        feature: node.feature.clone().unwrap_or_default(),
        operator: node.operator.clone().unwrap_or_default(),
        numeric_value: node.numeric_value,
        string_value: node.string_value.clone(),
    };
    let next = if structural_path_ranker_condition_matches(&condition, row) {
        node.left.as_deref()
    } else {
        node.right.as_deref()
    }?;
    structural_path_ranker_tree_evaluate(next, row)
}

fn score_structural_path_ranker_runtime_rows_with_explicit_artifact(
    artifact: &StructuralPathRankerExplicitArtifact,
    candidate_rows: &[StructuralPathRankingTargetRow],
) -> Vec<StructuralPathRankerRuntimeRow> {
    candidate_rows
        .iter()
        .filter_map(|row| {
            let mut runtime_row = if !artifact.rule_list.is_empty() {
                let rule = artifact
                    .rule_list
                    .iter()
                    .find(|rule| structural_path_ranker_rule_matches(rule, row))?;
                structural_path_ranker_tree_leaf_row(
                    row,
                    rule.score,
                    rule.path_prob_lower_bound,
                    rule.execution_gate_status.clone(),
                    None,
                    None,
                )
            } else {
                artifact
                    .tree_json
                    .as_ref()
                    .and_then(|tree| structural_path_ranker_tree_evaluate(tree, row))?
            };
            runtime_row.score_model_family = Some(artifact.model_family.clone());
            runtime_row.score_source_kind = Some("explicit_artifact".to_string());
            Some(runtime_row)
        })
        .collect()
}

fn load_structural_path_ranker_explicit_artifact(
    state_dir: &str,
    symbol: &str,
) -> Result<Option<StructuralPathRankerExplicitArtifact>> {
    let path = structural_path_ranker_artifact_json_path(state_dir, symbol);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path)?;
    let artifact = serde_json::from_str::<StructuralPathRankerExplicitArtifact>(&raw)?;
    if !structural_path_ranker_supports_explicit_family(&artifact.model_family) {
        return Ok(None);
    }
    Ok(Some(artifact))
}

pub fn load_structural_path_ranker_direct_model_artifact(
    state_dir: &str,
    symbol: &str,
    artifact_uri: &str,
    model_family: &str,
) -> Result<Option<StructuralPathRankerDirectModelArtifact>> {
    if !structural_path_ranker_supports_direct_model_family(model_family) {
        return Ok(None);
    }
    let Some((_, raw)) =
        load_structural_path_ranker_runtime_artifact_raw(state_dir, symbol, artifact_uri)?
    else {
        return Ok(None);
    };
    let artifact = serde_json::from_str::<StructuralPathRankerDirectModelArtifact>(&raw)?;
    let declared_family = artifact.model_family.trim();
    if !declared_family.is_empty()
        && declared_family != model_family.trim()
        && !structural_path_ranker_supports_direct_model_family(declared_family)
    {
        return Err(anyhow::anyhow!(
            "runtime direct model family mismatch: declared='{}' registered='{}'",
            declared_family,
            model_family
        ));
    }
    Ok(Some(artifact))
}

pub fn score_structural_path_ranker_runtime_rows_with_direct_model(
    state_dir: &str,
    symbol: &str,
    artifact_uri: &str,
    model_family: &str,
    candidate_rows: &[StructuralPathRankingTargetRow],
) -> Result<Vec<StructuralPathRankerRuntimeRow>> {
    let Some(model) = load_structural_path_ranker_direct_model_artifact(
        state_dir,
        symbol,
        artifact_uri,
        model_family,
    )?
    else {
        return Ok(Vec::new());
    };
    let lower_bound_margin = model.lower_bound_margin.unwrap_or_default().clamp(0.0, 1.0);
    let gate_threshold = model
        .execution_gate_min_path_prob
        .unwrap_or(STRUCTURAL_PATH_RANKING_EXECUTION_GATE_MIN_PATH_PROB)
        .clamp(0.0, 1.0);
    Ok(candidate_rows
        .iter()
        .map(|row| {
            let probability = structural_path_ranker_direct_model_probability(&model, row);
            let path_prob_lower_bound = if lower_bound_margin > f64::EPSILON {
                Some((probability - lower_bound_margin).clamp(0.0, 1.0))
            } else {
                None
            };
            let gate_signal = path_prob_lower_bound.unwrap_or(probability);
            StructuralPathRankerRuntimeRow {
                candidate_set_id: row.candidate_set_id.clone(),
                path_id: row.path_id.clone(),
                raw_path_score: Some(probability),
                calibrated_path_prob: Some(probability),
                path_prob_lower_bound,
                execution_gate_status: Some(
                    if gate_signal >= gate_threshold {
                        "pass"
                    } else {
                        "observe"
                    }
                    .to_string(),
                ),
                score_model_family: Some(model.model_family.clone()),
                score_source_kind: Some("direct_model".to_string()),
                score_model_artifact_uri: Some(artifact_uri.to_string()),
                score_generator: Some("ict_engine_direct_model".to_string()),
            }
        })
        .collect())
}

pub fn score_structural_path_ranker_runtime_rows_with_service(
    symbol: &str,
    artifact_uri: &str,
    score_column: &str,
    model_family: &str,
    candidate_rows: &[StructuralPathRankingTargetRow],
) -> Result<Vec<StructuralPathRankerRuntimeRow>> {
    if !structural_path_ranker_supports_service_family(model_family) {
        return Ok(Vec::new());
    }
    let candidate_set_id = candidate_rows
        .first()
        .map(|row| row.candidate_set_id.as_str())
        .unwrap_or_default();
    let request = StructuralPathRankerServiceRequest {
        protocol_version: "structural-path-ranking-service-request-v1",
        symbol,
        candidate_set_id,
        score_column,
        rows: candidate_rows,
    };
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let raw = client
        .post(artifact_uri)
        .json(&request)
        .send()?
        .error_for_status()?
        .text()?;
    parse_structural_path_ranker_runtime_rows_from_raw("service_response.json", &raw, score_column)
}

pub fn score_structural_path_ranker_runtime_rows_with_explicit_family(
    state_dir: &str,
    symbol: &str,
    model_family: &str,
    candidate_rows: &[StructuralPathRankingTargetRow],
) -> Result<Vec<StructuralPathRankerRuntimeRow>> {
    if !structural_path_ranker_supports_explicit_family(model_family) {
        return Ok(Vec::new());
    }
    let Some(artifact) = load_structural_path_ranker_explicit_artifact(state_dir, symbol)? else {
        return Ok(Vec::new());
    };
    Ok(score_structural_path_ranker_runtime_rows_with_explicit_artifact(&artifact, candidate_rows))
}

pub fn structural_path_ranking_target_row_history_key(
    row: &StructuralPathRankingTargetRow,
) -> String {
    let label = row
        .calibrated_label
        .map(|value| format!("{value:.6}"))
        .unwrap_or_else(|| "none".to_string());
    format!(
        "{}|{}|{}|{}|{:.6}|{:.6}|{:.6}|{}",
        row.candidate_set_id,
        row.path_id,
        row.pending_reward_state,
        label,
        row.current_posterior,
        row.experience_prior,
        row.structural_baseline_score,
        row.rank
    )
}

pub fn structural_path_ranking_target_row_score_key(
    row: &StructuralPathRankingTargetRow,
) -> String {
    format!("{}|{}", row.candidate_set_id, row.path_id)
}

pub fn load_structural_path_ranking_target_rows(
    jsonl_path: &Path,
) -> Result<Vec<StructuralPathRankingTargetRow>> {
    if !jsonl_path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(jsonl_path)?;
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str::<StructuralPathRankingTargetRow>)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn render_structural_path_ranking_target_rows_jsonl(
    rows: &[StructuralPathRankingTargetRow],
) -> Result<String> {
    let mut out = String::new();
    for row in rows {
        out.push_str(&serde_json::to_string(row)?);
        out.push('\n');
    }
    Ok(out)
}

pub fn render_structural_path_ranking_target_jsonl(
    artifact: &StructuralPathRankingTargetArtifact,
) -> Result<String> {
    render_structural_path_ranking_target_rows_jsonl(&artifact.rows)
}

pub fn render_structural_path_ranking_target_rows_csv(
    protocol_version: &str,
    symbol: &str,
    generated_at: &str,
    rows: &[StructuralPathRankingTargetRow],
) -> String {
    let mut out = String::from(
        "protocol_version,symbol,generated_at,candidate_set_id,candidate_set_size,rank,path_id,scenario_id,path_label,direction,raw_path_score,calibrated_path_prob,path_prob_lower_bound,execution_gate_status,execution_gate_min_path_prob,execution_gate_reason,pending_reward_state,maturity_mask,maturity_weight,calibrated_label,propensity_estimate,ips_weight,training_weight,regime_calibration_bucket,behavior_policy_probability,execution_propensity,target_policy_probability_confidence,target_policy_probability_lower_bound,target_policy_reward_prior,target_policy_reward_lower_bound,experience_prior,current_posterior,structural_baseline_score,score_model_family,score_source_kind,score_model_artifact_uri,score_generator\n",
    );
    for row in rows {
        let fields = [
            csv_escape(protocol_version),
            csv_escape(symbol),
            csv_escape(generated_at),
            csv_escape(&row.candidate_set_id),
            row.candidate_set_size.to_string(),
            row.rank.to_string(),
            csv_escape(&row.path_id),
            csv_escape(&row.scenario_id),
            csv_escape(&row.path_label),
            csv_escape(&row.direction),
            csv_optional_f64(row.raw_path_score),
            csv_optional_f64(row.calibrated_path_prob),
            csv_optional_f64(row.path_prob_lower_bound),
            csv_optional_string(row.execution_gate_status.as_deref()),
            csv_optional_f64(row.execution_gate_min_path_prob),
            csv_optional_string(row.execution_gate_reason.as_deref()),
            csv_escape(&row.pending_reward_state),
            row.maturity_mask.to_string(),
            csv_f64(row.maturity_weight),
            csv_optional_f64(row.calibrated_label),
            csv_optional_f64(row.propensity_estimate),
            csv_optional_f64(row.ips_weight),
            csv_optional_f64(row.training_weight),
            csv_escape(&row.regime_calibration_bucket),
            csv_f64(row.behavior_policy_probability),
            csv_optional_f64(row.execution_propensity),
            csv_optional_f64(row.target_policy_probability_confidence),
            csv_optional_f64(row.target_policy_probability_lower_bound),
            csv_optional_f64(row.target_policy_reward_prior),
            csv_optional_f64(row.target_policy_reward_lower_bound),
            csv_f64(row.experience_prior),
            csv_f64(row.current_posterior),
            csv_f64(row.structural_baseline_score),
            csv_optional_string(row.score_model_family.as_deref()),
            csv_optional_string(row.score_source_kind.as_deref()),
            csv_optional_string(row.score_model_artifact_uri.as_deref()),
            csv_optional_string(row.score_generator.as_deref()),
        ];
        out.push_str(&fields.join(","));
        out.push('\n');
    }
    out
}

pub fn render_structural_path_ranking_target_csv(
    artifact: &StructuralPathRankingTargetArtifact,
) -> String {
    render_structural_path_ranking_target_rows_csv(
        &artifact.protocol_version,
        &artifact.symbol,
        &artifact.generated_at,
        &artifact.rows,
    )
}

fn csv_f64(value: f64) -> String {
    format!("{value:.6}")
}

fn csv_optional_f64(value: Option<f64>) -> String {
    value.map(csv_f64).unwrap_or_default()
}

fn csv_optional_string(value: Option<&str>) -> String {
    value.map(csv_escape).unwrap_or_default()
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

pub fn structural_path_ranking_reward_label(pending_reward_state: &str) -> Option<f64> {
    match pending_reward_state {
        "matured_success" => Some(1.0),
        "matured_failure" | "matured_invalidated" => Some(0.0),
        _ => None,
    }
}

pub fn structural_path_ranking_beta_mean(success_mass: f64, failure_mass: f64) -> f64 {
    beta_posterior_mean(success_mass, failure_mass)
}

pub fn structural_path_ranking_beta_lower_bound(success_mass: f64, failure_mass: f64) -> f64 {
    beta_posterior_lower_bound(success_mass, failure_mass, 1.64)
}

pub fn structural_path_ranking_ips_weight(propensity_estimate: Option<f64>) -> Option<f64> {
    let propensity = propensity_estimate?.clamp(0.0, 1.0);
    if propensity <= f64::EPSILON {
        None
    } else {
        Some((1.0 / propensity).min(STRUCTURAL_PATH_RANKING_IPS_WEIGHT_CLIP))
    }
}

pub fn structural_path_ranking_propensity_estimate(
    execution_propensity: Option<f64>,
    behavior_policy_probability: f64,
) -> Option<f64> {
    execution_propensity.map(|propensity| {
        (propensity.clamp(0.0, 1.0) * behavior_policy_probability.clamp(0.0, 1.0)).clamp(0.0, 1.0)
    })
}

pub fn structural_path_ranking_training_weight(
    calibrated_label: Option<f64>,
    maturity_weight: f64,
    ips_weight: Option<f64>,
) -> Option<f64> {
    calibrated_label?;
    let maturity_weight = maturity_weight.clamp(0.0, 1.0);
    if maturity_weight <= f64::EPSILON {
        return None;
    }
    Some(maturity_weight * ips_weight?)
}

pub fn structural_path_ranking_propensity_evaluation_weight(
    row: &StructuralPathRankingTargetRow,
) -> Option<f64> {
    let ips_weight = row
        .ips_weight
        .or_else(|| structural_path_ranking_ips_weight(row.propensity_estimate))?;
    let maturity_weight = if row.maturity_weight > f64::EPSILON {
        row.maturity_weight.clamp(0.0, 1.0)
    } else if row.maturity_mask
        || structural_path_ranking_reward_label(&row.pending_reward_state).is_some()
    {
        1.0
    } else {
        0.0
    };
    if maturity_weight <= f64::EPSILON {
        return None;
    }
    Some(ips_weight.clamp(0.0, STRUCTURAL_PATH_RANKING_IPS_WEIGHT_CLIP) * maturity_weight)
}

pub fn apply_structural_path_probability_bins(
    rows: &mut [StructuralPathRankingTargetRow],
    bins: &[StructuralPathProbabilityCalibrationBin],
) -> usize {
    let mut calibrated_rows = 0;
    for row in rows {
        let Some(raw_score) = row.raw_path_score else {
            continue;
        };
        let raw_score = raw_score.clamp(0.0, 1.0);
        let Some(bin) = bins.iter().find(|bin| {
            bin.regime_calibration_bucket == row.regime_calibration_bucket
                && raw_score >= bin.raw_path_score_min
                && raw_score <= bin.raw_path_score_max
        }) else {
            continue;
        };
        row.calibrated_path_prob = Some(bin.calibrated_path_prob);
        row.path_prob_lower_bound = Some(bin.path_prob_lower_bound);
        calibrated_rows += 1;
    }
    calibrated_rows
}

pub fn apply_structural_path_ranking_execution_gates(
    artifact: &mut StructuralPathRankingTargetArtifact,
) {
    for row in &mut artifact.rows {
        let Some(lower_bound) = row.path_prob_lower_bound else {
            continue;
        };
        let lower_bound = lower_bound.clamp(0.0, 1.0);
        let min_path_prob = STRUCTURAL_PATH_RANKING_EXECUTION_GATE_MIN_PATH_PROB;
        let status = if lower_bound >= min_path_prob {
            "pass"
        } else {
            "observe"
        };
        row.execution_gate_status = Some(status.to_string());
        row.execution_gate_min_path_prob = Some(min_path_prob);
        row.execution_gate_reason = Some(format!(
            "path_prob_lower_bound={lower_bound:.3} min_path_prob={min_path_prob:.3}"
        ));
    }
}

pub fn apply_structural_path_probability_calibration(
    artifact: &mut StructuralPathRankingTargetArtifact,
) -> StructuralPathProbabilityCalibrationReport {
    let mut by_bucket = BTreeMap::<String, Vec<(f64, f64)>>::new();
    for row in &artifact.rows {
        let Some(raw_score) = row.raw_path_score else {
            continue;
        };
        let Some(reward) = structural_path_ranking_reward_label(&row.pending_reward_state) else {
            continue;
        };
        by_bucket
            .entry(row.regime_calibration_bucket.clone())
            .or_default()
            .push((raw_score.clamp(0.0, 1.0), reward));
    }

    let observed_rows = by_bucket.values().map(Vec::len).sum::<usize>();
    let mut bins = Vec::new();
    let mut warnings = Vec::new();
    for (bucket, observations) in by_bucket {
        if observations.len() < 2 {
            warnings.push(format!(
                "calibration_bucket_insufficient_observations:{bucket}:{}",
                observations.len()
            ));
            continue;
        }
        let successes = observations
            .iter()
            .filter(|(_, reward)| *reward > 0.5)
            .count();
        let calibrated_path_prob = structural_path_ranking_beta_mean(
            successes as f64,
            (observations.len() - successes) as f64,
        );
        let path_prob_lower_bound = structural_path_ranking_beta_lower_bound(
            successes as f64,
            (observations.len() - successes) as f64,
        );
        let raw_path_score_min = observations
            .iter()
            .map(|(score, _)| *score)
            .fold(f64::INFINITY, f64::min);
        let raw_path_score_max = observations
            .iter()
            .map(|(score, _)| *score)
            .fold(f64::NEG_INFINITY, f64::max);
        bins.push(StructuralPathProbabilityCalibrationBin {
            regime_calibration_bucket: bucket,
            observations: observations.len(),
            successes,
            raw_path_score_min,
            raw_path_score_max,
            calibrated_path_prob,
            path_prob_lower_bound,
        });
    }

    let calibrated_rows = apply_structural_path_probability_bins(&mut artifact.rows, &bins);
    let status = if calibrated_rows > 0 {
        "calibrated"
    } else if observed_rows > 0 {
        "insufficient_calibration_data"
    } else {
        "no_calibration_observations"
    };
    if calibrated_rows == 0 {
        warnings.push("structural_path_probability_calibration_not_fitted".to_string());
    }
    apply_structural_path_ranking_execution_gates(artifact);
    StructuralPathProbabilityCalibrationReport {
        status: status.to_string(),
        observed_rows,
        calibrated_rows,
        bins,
        warnings,
        summary_line: format!(
            "structural_path_probability_calibration status={status} observed_rows={observed_rows} calibrated_rows={calibrated_rows}"
        ),
    }
}

pub fn evaluate_structural_path_probability_calibration_rows(
    rows: &[StructuralPathRankingTargetRow],
) -> StructuralPathProbabilityCalibrationEvaluationReport {
    let mut by_bucket = BTreeMap::<String, Vec<(f64, f64)>>::new();
    let mut squared_error_sum = 0.0;
    let mut propensity_weighted_squared_error_sum = 0.0;
    let mut propensity_weight_sum = 0.0;
    let mut propensity_weighted_rows = 0;
    for row in rows {
        if row.raw_path_score.is_none() {
            continue;
        }
        let Some(calibrated_prob) = row.calibrated_path_prob else {
            continue;
        };
        let Some(reward) = structural_path_ranking_reward_label(&row.pending_reward_state) else {
            continue;
        };
        let calibrated_prob = calibrated_prob.clamp(0.0, 1.0);
        let squared_error = (calibrated_prob - reward).powi(2);
        squared_error_sum += squared_error;
        if let Some(weight) = structural_path_ranking_propensity_evaluation_weight(row) {
            propensity_weighted_squared_error_sum += weight * squared_error;
            propensity_weight_sum += weight;
            propensity_weighted_rows += 1;
        }
        by_bucket
            .entry(row.regime_calibration_bucket.clone())
            .or_default()
            .push((calibrated_prob, reward));
    }

    let eligible_rows = by_bucket.values().map(Vec::len).sum::<usize>();
    let mut warnings = Vec::new();
    if eligible_rows < 2 {
        warnings.push(
            "structural_path_probability_calibration_evaluation_insufficient_rows".to_string(),
        );
        return StructuralPathProbabilityCalibrationEvaluationReport {
            status: "insufficient_calibration_evaluation_rows".to_string(),
            eligible_rows,
            warnings,
            summary_line: format!(
                "structural_path_probability_calibration_evaluation status=insufficient_calibration_evaluation_rows eligible_rows={eligible_rows}"
            ),
            ..StructuralPathProbabilityCalibrationEvaluationReport::default()
        };
    }

    let mut expected_calibration_error = 0.0;
    let mut max_calibration_error: f64 = 0.0;
    let mut bins = Vec::new();
    for (bucket, observations) in by_bucket {
        let observation_count = observations.len();
        let mean_calibrated_path_prob = observations
            .iter()
            .map(|(probability, _)| *probability)
            .sum::<f64>()
            / observation_count as f64;
        let empirical_success_rate =
            observations.iter().map(|(_, reward)| *reward).sum::<f64>() / observation_count as f64;
        let absolute_error = (mean_calibrated_path_prob - empirical_success_rate).abs();
        expected_calibration_error +=
            (observation_count as f64 / eligible_rows as f64) * absolute_error;
        max_calibration_error = max_calibration_error.max(absolute_error);
        bins.push(StructuralPathProbabilityCalibrationEvaluationBin {
            regime_calibration_bucket: bucket,
            observations: observation_count,
            mean_calibrated_path_prob,
            empirical_success_rate,
            absolute_error,
        });
    }

    let brier_score = squared_error_sum / eligible_rows as f64;
    let propensity_weighted_brier_score = if propensity_weight_sum > f64::EPSILON {
        Some(propensity_weighted_squared_error_sum / propensity_weight_sum)
    } else {
        warnings.push(
            "structural_path_probability_calibration_evaluation_propensity_missing".to_string(),
        );
        None
    };
    let propensity_weighted_brier_summary = propensity_weighted_brier_score
        .map(|score| format!(" propensity_weighted_brier_score={score:.6}"))
        .unwrap_or_default();
    StructuralPathProbabilityCalibrationEvaluationReport {
        status: "evaluated".to_string(),
        eligible_rows,
        brier_score: Some(brier_score),
        propensity_weighted_rows,
        propensity_weighted_brier_score,
        expected_calibration_error: Some(expected_calibration_error),
        max_calibration_error: Some(max_calibration_error),
        bins,
        warnings,
        summary_line: format!(
            "structural_path_probability_calibration_evaluation status=evaluated eligible_rows={eligible_rows} brier_score={brier_score:.6} expected_calibration_error={expected_calibration_error:.6} propensity_weighted_rows={propensity_weighted_rows}{propensity_weighted_brier_summary}"
        ),
    }
}

pub fn structural_path_ranking_trainer_manifest() -> StructuralPathRankingTrainerManifest {
    StructuralPathRankingTrainerManifest {
        protocol_version: "structural-path-ranking-trainer-manifest-v1".to_string(),
        dataset_role: "external_path_ranker_training_dataset".to_string(),
        group_id_column: "candidate_set_id".to_string(),
        label_column: "calibrated_label".to_string(),
        weight_column: "training_weight".to_string(),
        maturity_column: "maturity_mask".to_string(),
        raw_score_column: "raw_path_score".to_string(),
        feature_columns: vec![
            "rank".to_string(),
            "direction".to_string(),
            "regime_calibration_bucket".to_string(),
            "behavior_policy_probability".to_string(),
            "execution_propensity".to_string(),
            "target_policy_probability_confidence".to_string(),
            "target_policy_probability_lower_bound".to_string(),
            "target_policy_reward_prior".to_string(),
            "target_policy_reward_lower_bound".to_string(),
            "experience_prior".to_string(),
            "current_posterior".to_string(),
            "structural_baseline_score".to_string(),
        ],
        calibration_columns: vec![
            "calibrated_path_prob".to_string(),
            "path_prob_lower_bound".to_string(),
            "execution_gate_status".to_string(),
        ],
        guardrail_columns: vec![
            "candidate_set_size".to_string(),
            "path_id".to_string(),
            "scenario_id".to_string(),
            "pending_reward_state".to_string(),
            "maturity_weight".to_string(),
            "propensity_estimate".to_string(),
            "ips_weight".to_string(),
        ],
        notes: vec![
            "Trainer runs outside the Rust belief engine; this manifest only describes exported columns."
                .to_string(),
            "Rows without calibrated_label or training_weight are censored/unusable for supervised ranker loss."
                .to_string(),
        ],
    }
}

pub struct StructuralPathRankingTargetExportSummaryInput<'a> {
    pub state_dir: &'a str,
    pub symbol: &'a str,
    pub artifact: &'a StructuralPathRankingTargetArtifact,
    pub csv_name: &'a str,
    pub jsonl_name: &'a str,
    pub history_csv_name: &'a str,
    pub history_jsonl_name: &'a str,
    pub history_rows: &'a [StructuralPathRankingTargetRow],
    pub summary_name: &'a str,
}

pub fn structural_path_ranking_target_export_summary(
    input: StructuralPathRankingTargetExportSummaryInput<'_>,
) -> StructuralPathRankingTargetExportSummary {
    let StructuralPathRankingTargetExportSummaryInput {
        state_dir,
        symbol,
        artifact,
        csv_name,
        jsonl_name,
        history_csv_name,
        history_jsonl_name,
        history_rows,
        summary_name,
    } = input;
    let mut pending_reward_states = BTreeMap::new();
    for row in &artifact.rows {
        *pending_reward_states
            .entry(row.pending_reward_state.clone())
            .or_insert(0) += 1;
    }
    let rows = artifact.rows.len();
    let mature_rows = artifact.rows.iter().filter(|row| row.maturity_mask).count();
    let rows_with_raw_path_score = artifact
        .rows
        .iter()
        .filter(|row| row.raw_path_score.is_some())
        .count();
    let rows_with_calibrated_path_prob = artifact
        .rows
        .iter()
        .filter(|row| row.calibrated_path_prob.is_some())
        .count();
    let rows_with_path_prob_lower_bound = artifact
        .rows
        .iter()
        .filter(|row| row.path_prob_lower_bound.is_some())
        .count();
    let rows_with_propensity_estimate = artifact
        .rows
        .iter()
        .filter(|row| row.propensity_estimate.is_some())
        .count();
    let rows_with_execution_gate_status = artifact
        .rows
        .iter()
        .filter(|row| row.execution_gate_status.is_some())
        .count();
    let rows_with_training_weight = artifact
        .rows
        .iter()
        .filter(|row| row.training_weight.is_some())
        .count();
    let history_mature_rows = history_rows.iter().filter(|row| row.maturity_mask).count();
    let history_rows_with_raw_path_score = history_rows
        .iter()
        .filter(|row| row.raw_path_score.is_some())
        .count();
    let history_rows_with_calibrated_path_prob = history_rows
        .iter()
        .filter(|row| row.calibrated_path_prob.is_some())
        .count();
    let history_rows_with_path_prob_lower_bound = history_rows
        .iter()
        .filter(|row| row.path_prob_lower_bound.is_some())
        .count();
    let history_rows_with_propensity_estimate = history_rows
        .iter()
        .filter(|row| row.propensity_estimate.is_some())
        .count();
    let history_rows_with_training_weight = history_rows
        .iter()
        .filter(|row| row.training_weight.is_some())
        .count();
    let summary_line = format!(
        "structural_path_ranking_target rows={} history_rows={} candidate_set_size={} mature_rows={} history_mature_rows={} propensity_rows={} calibrated_rows={} execution_gate_rows={} training_weight_rows={}",
        rows,
        history_rows.len(),
        artifact.candidate_set_size,
        mature_rows,
        history_mature_rows,
        rows_with_propensity_estimate,
        rows_with_calibrated_path_prob,
        rows_with_execution_gate_status,
        rows_with_training_weight
    );
    StructuralPathRankingTargetExportSummary {
        symbol: artifact.symbol.clone(),
        rows,
        candidate_set_id: artifact.candidate_set_id.clone(),
        candidate_set_size: artifact.candidate_set_size,
        pending_reward_states,
        mature_rows,
        rows_with_raw_path_score,
        rows_with_calibrated_path_prob,
        rows_with_path_prob_lower_bound,
        rows_with_propensity_estimate,
        rows_with_execution_gate_status,
        rows_with_training_weight,
        csv_path: Path::new(state_dir)
            .join(symbol)
            .join(csv_name)
            .to_string_lossy()
            .to_string(),
        jsonl_path: Path::new(state_dir)
            .join(symbol)
            .join(jsonl_name)
            .to_string_lossy()
            .to_string(),
        history_csv_path: Path::new(state_dir)
            .join(symbol)
            .join(history_csv_name)
            .to_string_lossy()
            .to_string(),
        history_jsonl_path: Path::new(state_dir)
            .join(symbol)
            .join(history_jsonl_name)
            .to_string_lossy()
            .to_string(),
        history_rows: history_rows.len(),
        history_mature_rows,
        history_rows_with_raw_path_score,
        history_rows_with_calibrated_path_prob,
        history_rows_with_path_prob_lower_bound,
        history_rows_with_propensity_estimate,
        history_rows_with_training_weight,
        summary_path: Path::new(state_dir)
            .join(symbol)
            .join(summary_name)
            .to_string_lossy()
            .to_string(),
        trainer_manifest: structural_path_ranking_trainer_manifest(),
        summary_line,
    }
}

pub fn clear_structural_path_ranking_target_row_outputs(row: &mut StructuralPathRankingTargetRow) {
    row.calibrated_path_prob = None;
    row.path_prob_lower_bound = None;
    row.execution_gate_status = None;
    row.execution_gate_min_path_prob = None;
    row.execution_gate_reason = None;
}

pub fn upsert_structural_path_ranking_target_history(
    history_jsonl_path: &Path,
    rows: &[StructuralPathRankingTargetRow],
) -> Result<Vec<StructuralPathRankingTargetRow>> {
    let mut history = load_structural_path_ranking_target_rows(history_jsonl_path)?;
    let mut index = history
        .iter()
        .enumerate()
        .map(|(position, row)| {
            (
                structural_path_ranking_target_row_history_key(row),
                position,
            )
        })
        .collect::<BTreeMap<_, _>>();
    for row in rows {
        let key = structural_path_ranking_target_row_history_key(row);
        if let Some(position) = index.get(&key).copied() {
            history[position] = row.clone();
        } else {
            index.insert(key, history.len());
            history.push(row.clone());
        }
    }
    Ok(history)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_target_row(
        candidate_set_id: &str,
        path_id: &str,
        pending_reward_state: &str,
        current_posterior: f64,
        raw_path_score: Option<f64>,
    ) -> StructuralPathRankingTargetRow {
        StructuralPathRankingTargetRow {
            rank: 1,
            candidate_set_id: candidate_set_id.to_string(),
            candidate_set_size: 1,
            path_id: path_id.to_string(),
            scenario_id: format!("scenario:{path_id}"),
            path_label: path_id.to_string(),
            direction: "Observe".to_string(),
            raw_path_score,
            calibrated_path_prob: raw_path_score,
            path_prob_lower_bound: raw_path_score.map(|score| (score - 0.1).clamp(0.0, 1.0)),
            execution_gate_status: None,
            execution_gate_min_path_prob: None,
            execution_gate_reason: None,
            pending_reward_state: pending_reward_state.to_string(),
            maturity_mask: matches!(
                pending_reward_state,
                "matured_success" | "matured_failure" | "matured_invalidated"
            ),
            maturity_weight: if matches!(
                pending_reward_state,
                "matured_success" | "matured_failure" | "matured_invalidated"
            ) {
                1.0
            } else {
                0.0
            },
            calibrated_label: structural_path_ranking_reward_label(pending_reward_state),
            propensity_estimate: Some(0.5),
            ips_weight: Some(2.0),
            training_weight: structural_path_ranking_reward_label(pending_reward_state)
                .map(|_| 2.0),
            regime_calibration_bucket: "NQ:trend".to_string(),
            behavior_policy_probability: 0.5,
            execution_propensity: Some(0.5),
            target_policy_probability_confidence: Some(0.55),
            target_policy_probability_lower_bound: Some(0.30),
            target_policy_reward_prior: Some(0.58),
            target_policy_reward_lower_bound: Some(0.28),
            experience_prior: 0.5,
            current_posterior,
            structural_baseline_score: 0.5,
            score_model_family: None,
            score_source_kind: None,
            score_model_artifact_uri: None,
            score_generator: None,
        }
    }

    #[test]
    fn target_history_keeps_distinct_mature_observations_for_same_candidate_path() {
        let temp = tempfile::tempdir().unwrap();
        let history_path = temp.path().join("history.jsonl");
        let path_id = "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary";
        let first = test_target_row(
            "structural-candidates:NQ:stable",
            path_id,
            "matured_success",
            0.41,
            Some(0.8),
        );
        let second = test_target_row(
            "structural-candidates:NQ:stable",
            path_id,
            "matured_failure",
            0.62,
            Some(0.2),
        );

        let after_first = upsert_structural_path_ranking_target_history(
            &history_path,
            std::slice::from_ref(&first),
        )
        .unwrap();
        fs::write(
            &history_path,
            render_structural_path_ranking_target_rows_jsonl(&after_first).unwrap(),
        )
        .unwrap();
        let after_second = upsert_structural_path_ranking_target_history(
            &history_path,
            std::slice::from_ref(&second),
        )
        .unwrap();

        assert_eq!(after_first.len(), 1);
        assert_eq!(after_second.len(), 2);
        assert_eq!(
            structural_path_ranking_target_row_score_key(&first),
            structural_path_ranking_target_row_score_key(&second)
        );
        assert_ne!(
            structural_path_ranking_target_row_history_key(&first),
            structural_path_ranking_target_row_history_key(&second)
        );
    }
}
