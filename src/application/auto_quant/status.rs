use std::path::PathBuf;

use anyhow::Result;

use super::config::{default_config, load_config};
use super::health::{required_files, verify_checkout};
use super::repo_manager::{current_commit, is_git_repo, upstream_commit};
use super::types::AutoQuantDependencyStatus;

pub fn auto_quant_status(state_dir: &str) -> Result<AutoQuantDependencyStatus> {
    let config = load_config(state_dir)?;
    let config_present = config.is_some();
    let config = config.unwrap_or_else(|| default_config(state_dir));
    let managed_dir = PathBuf::from(&config.managed_dir);
    let managed_repo_present = managed_dir.exists() && is_git_repo(&managed_dir);
    let bootstrap_needed = !managed_repo_present;
    let mut notes = Vec::new();
    let (healthy, verify_notes) = if managed_repo_present {
        verify_checkout(&managed_dir)
    } else {
        (false, vec!["auto_quant_not_bootstrapped".to_string()])
    };
    notes.extend(verify_notes);
    let current_commit = if managed_repo_present {
        current_commit(&managed_dir).ok()
    } else {
        None
    };
    let upstream_commit = if managed_repo_present {
        match upstream_commit(&managed_dir, &config.tracked_branch) {
            Ok(commit) => Some(commit),
            Err(err) => {
                notes.push(format!("upstream_check_failed={err}"));
                None
            }
        }
    } else {
        None
    };
    let pinned_ref = config.pinned_ref.clone().or_else(|| current_commit.clone());
    Ok(AutoQuantDependencyStatus {
        repo_url: config.repo_url,
        managed_dir: managed_dir.to_string_lossy().to_string(),
        tracked_branch: config.tracked_branch,
        pinned_ref,
        current_commit: current_commit.clone(),
        upstream_commit: upstream_commit.clone(),
        bootstrap_needed,
        config_present,
        managed_repo_present,
        healthy,
        update_available: matches!((&current_commit, &upstream_commit), (Some(current), Some(upstream)) if current != upstream),
        required_files: required_files(),
        notes,
        adapter_version: config.adapter_version,
        last_sync: config.last_sync,
    })
}
