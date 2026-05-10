use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::types::{
    AutoQuantDependencyConfig, AUTO_QUANT_ADAPTER_VERSION, AUTO_QUANT_BRANCH_ENV_VAR,
    AUTO_QUANT_CONFIG_FILE, AUTO_QUANT_DIR_ENV_VAR, AUTO_QUANT_REPO_URL_ENV_VAR,
    DEFAULT_AUTO_QUANT_BRANCH, DEFAULT_AUTO_QUANT_REPO_URL,
};

pub fn config_path(state_dir: &str) -> PathBuf {
    Path::new(state_dir).join(AUTO_QUANT_CONFIG_FILE)
}

fn default_managed_dir(state_dir: &str) -> PathBuf {
    Path::new(state_dir).join(".deps").join("auto-quant")
}

fn resolve_repo_url() -> String {
    env::var(AUTO_QUANT_REPO_URL_ENV_VAR)
        .unwrap_or_else(|_| DEFAULT_AUTO_QUANT_REPO_URL.to_string())
}

fn resolve_tracked_branch() -> String {
    env::var(AUTO_QUANT_BRANCH_ENV_VAR).unwrap_or_else(|_| DEFAULT_AUTO_QUANT_BRANCH.to_string())
}

fn resolve_managed_dir(state_dir: &str) -> PathBuf {
    env::var(AUTO_QUANT_DIR_ENV_VAR)
        .map(|value| PathBuf::from(value).expand_home())
        .unwrap_or_else(|_| default_managed_dir(state_dir))
}

trait ExpandHome {
    fn expand_home(self) -> PathBuf;
}

impl ExpandHome for PathBuf {
    fn expand_home(self) -> PathBuf {
        let text = self.to_string_lossy();
        if let Some(stripped) = text.strip_prefix("~/") {
            if let Ok(home) = env::var("HOME") {
                return Path::new(&home).join(stripped);
            }
        }
        self
    }
}

pub fn default_config(state_dir: &str) -> AutoQuantDependencyConfig {
    AutoQuantDependencyConfig {
        repo_url: resolve_repo_url(),
        managed_dir: resolve_managed_dir(state_dir).to_string_lossy().to_string(),
        tracked_branch: resolve_tracked_branch(),
        pinned_ref: None,
        adapter_version: AUTO_QUANT_ADAPTER_VERSION.to_string(),
        last_sync: None,
    }
}

pub fn load_config(state_dir: &str) -> Result<Option<AutoQuantDependencyConfig>> {
    let path = config_path(state_dir);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("reading auto-quant config '{}'", path.display()))?;
    let config = serde_json::from_str(&content)
        .with_context(|| format!("parsing auto-quant config '{}'", path.display()))?;
    Ok(Some(config))
}

pub fn save_config(state_dir: &str, config: &AutoQuantDependencyConfig) -> Result<()> {
    let path = config_path(state_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating auto-quant config parent '{}'", parent.display()))?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(config)?)
        .with_context(|| format!("writing auto-quant config '{}'", path.display()))?;
    Ok(())
}

pub fn ensure_bootstrapped_config(
    state_dir: &str,
    repo_url: Option<&str>,
    tracked_branch: Option<&str>,
) -> AutoQuantDependencyConfig {
    let mut config = load_config(state_dir)
        .ok()
        .flatten()
        .unwrap_or_else(|| default_config(state_dir));
    if let Some(repo_url) = repo_url {
        config.repo_url = repo_url.to_string();
    }
    if let Some(tracked_branch) = tracked_branch {
        config.tracked_branch = tracked_branch.to_string();
    }
    if config.managed_dir.trim().is_empty() {
        config.managed_dir = resolve_managed_dir(state_dir).to_string_lossy().to_string();
    }
    config.adapter_version = AUTO_QUANT_ADAPTER_VERSION.to_string();
    config
}
