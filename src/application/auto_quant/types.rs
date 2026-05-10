use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const AUTO_QUANT_CONFIG_FILE: &str = "auto_quant_dependency.json";
pub const AUTO_QUANT_ADAPTER_VERSION: &str = "v1";
pub const AUTO_QUANT_REPO_URL_ENV_VAR: &str = "ICT_ENGINE_AUTO_QUANT_REPO_URL";
pub const AUTO_QUANT_BRANCH_ENV_VAR: &str = "ICT_ENGINE_AUTO_QUANT_BRANCH";
pub const AUTO_QUANT_DIR_ENV_VAR: &str = "ICT_ENGINE_AUTO_QUANT_DIR";
pub const DEFAULT_AUTO_QUANT_REPO_URL: &str = "https://github.com/TraderAlice/Auto-Quant.git";
pub const DEFAULT_AUTO_QUANT_BRANCH: &str = "master";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoQuantDependencyConfig {
    pub repo_url: String,
    pub managed_dir: String,
    pub tracked_branch: String,
    pub pinned_ref: Option<String>,
    pub adapter_version: String,
    pub last_sync: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoQuantDependencyStatus {
    pub repo_url: String,
    pub managed_dir: String,
    pub tracked_branch: String,
    pub pinned_ref: Option<String>,
    pub current_commit: Option<String>,
    pub upstream_commit: Option<String>,
    pub bootstrap_needed: bool,
    pub config_present: bool,
    pub managed_repo_present: bool,
    pub healthy: bool,
    pub update_available: bool,
    pub required_files: Vec<String>,
    pub notes: Vec<String>,
    pub adapter_version: String,
    pub last_sync: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoQuantUpdateReport {
    pub repo_url: String,
    pub managed_dir: String,
    pub tracked_branch: String,
    pub previous_commit: Option<String>,
    pub target_ref: String,
    pub current_commit: String,
    pub applied: bool,
    pub rolled_back: bool,
    pub healthy: bool,
    pub notes: Vec<String>,
    pub adapter_version: String,
    pub last_sync: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoQuantAdoptionDecisionArtifact {
    pub artifact_id: String,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub handoff_artifact_id: String,
    pub handoff_kind: String,
    pub decision: String,
    pub rationale: String,
    pub requested_by: String,
    pub state_dir: String,
}
