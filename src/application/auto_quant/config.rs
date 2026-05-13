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
    let mut config = resolved_config(state_dir)
        .ok()
        .map(|(_, config)| config)
        .unwrap_or_else(|| default_config(state_dir));
    if let Some(repo_url) = repo_url {
        config.repo_url = repo_url.to_string();
    }
    if let Some(tracked_branch) = tracked_branch {
        config.tracked_branch = tracked_branch.to_string();
    }
    config.adapter_version = AUTO_QUANT_ADAPTER_VERSION.to_string();
    config
}

pub(super) fn resolved_config(state_dir: &str) -> Result<(bool, AutoQuantDependencyConfig)> {
    let loaded = load_config(state_dir)?;
    let config_present = loaded.is_some();
    let mut config = loaded.unwrap_or_else(|| default_config(state_dir));
    normalize_managed_dir_for_state(state_dir, &mut config, config_present);
    config.adapter_version = AUTO_QUANT_ADAPTER_VERSION.to_string();
    Ok((config_present, config))
}

fn normalize_managed_dir_for_state(
    state_dir: &str,
    config: &mut AutoQuantDependencyConfig,
    loaded_from_disk: bool,
) {
    if config.managed_dir.trim().is_empty() {
        config.managed_dir = resolve_managed_dir(state_dir).to_string_lossy().to_string();
    } else if loaded_from_disk && should_rebase_copied_managed_dir(state_dir, &config.managed_dir) {
        config.managed_dir = default_managed_dir(state_dir).to_string_lossy().to_string();
    }
}

fn should_rebase_copied_managed_dir(state_dir: &str, managed_dir: &str) -> bool {
    let managed_dir = managed_dir.trim();
    if managed_dir.is_empty() {
        return false;
    }

    let state_dir = Path::new(state_dir);
    let default_dir = default_managed_dir(state_dir.to_string_lossy().as_ref());
    let managed_path = PathBuf::from(managed_dir);

    managed_path.is_absolute()
        && managed_path != default_dir
        && !managed_path.starts_with(state_dir)
        && looks_like_state_local_managed_dir(&managed_path)
}

fn looks_like_state_local_managed_dir(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()) == Some("auto-quant")
        && path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            == Some(".deps")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copied_state_config_rebases_absolute_managed_dir_to_local_workspace() {
        let source = tempfile::tempdir().unwrap();
        let copied = tempfile::tempdir().unwrap();
        let source_workspace = default_managed_dir(source.path().to_str().unwrap());
        let copied_workspace = default_managed_dir(copied.path().to_str().unwrap());
        std::fs::create_dir_all(&source_workspace).unwrap();
        std::fs::create_dir_all(&copied_workspace).unwrap();

        let mut config = default_config(copied.path().to_str().unwrap());
        config.managed_dir = source_workspace.to_string_lossy().to_string();
        save_config(copied.path().to_str().unwrap(), &config).unwrap();

        let resolved = ensure_bootstrapped_config(copied.path().to_str().unwrap(), None, None);

        assert_eq!(
            resolved.managed_dir,
            copied_workspace.to_string_lossy().to_string()
        );
    }

    #[test]
    fn copied_state_config_rebases_even_before_local_workspace_exists() {
        let source = tempfile::tempdir().unwrap();
        let copied = tempfile::tempdir().unwrap();
        let source_workspace = default_managed_dir(source.path().to_str().unwrap());
        let copied_workspace = default_managed_dir(copied.path().to_str().unwrap());
        std::fs::create_dir_all(&source_workspace).unwrap();

        let mut config = default_config(copied.path().to_str().unwrap());
        config.managed_dir = source_workspace.to_string_lossy().to_string();
        save_config(copied.path().to_str().unwrap(), &config).unwrap();

        let resolved = ensure_bootstrapped_config(copied.path().to_str().unwrap(), None, None);

        assert_eq!(
            resolved.managed_dir,
            copied_workspace.to_string_lossy().to_string()
        );
    }

    #[test]
    fn copied_state_config_preserves_external_custom_managed_dir() {
        let copied = tempfile::tempdir().unwrap();
        let custom = tempfile::tempdir().unwrap();
        let custom_workspace = custom.path().join("Auto-Quant");
        std::fs::create_dir_all(&custom_workspace).unwrap();

        let mut config = default_config(copied.path().to_str().unwrap());
        config.managed_dir = custom_workspace.to_string_lossy().to_string();
        save_config(copied.path().to_str().unwrap(), &config).unwrap();

        let resolved = ensure_bootstrapped_config(copied.path().to_str().unwrap(), None, None);

        assert_eq!(
            resolved.managed_dir,
            custom_workspace.to_string_lossy().to_string()
        );
    }

    #[test]
    fn generated_default_config_does_not_rebase_external_deps_shaped_workspace() {
        let copied = tempfile::tempdir().unwrap();
        let custom = tempfile::tempdir().unwrap();
        let custom_workspace = custom.path().join(".deps").join("auto-quant");
        std::fs::create_dir_all(&custom_workspace).unwrap();

        let mut config = default_config(copied.path().to_str().unwrap());
        config.managed_dir = custom_workspace.to_string_lossy().to_string();

        normalize_managed_dir_for_state(copied.path().to_str().unwrap(), &mut config, false);

        assert_eq!(
            config.managed_dir,
            custom_workspace.to_string_lossy().to_string()
        );
    }
}
