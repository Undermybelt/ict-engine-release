use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use chrono::Utc;

use super::config::{ensure_bootstrapped_config, save_config};
use super::health::verify_checkout;
use super::repo_manager::{current_commit, git_output, is_git_repo};
use super::status::auto_quant_status;
use super::types::{AutoQuantDependencyStatus, AutoQuantUpdateReport};

pub fn auto_quant_bootstrap(
    state_dir: &str,
    repo_url: Option<&str>,
    tracked_branch: Option<&str>,
) -> Result<AutoQuantDependencyStatus> {
    let mut config = ensure_bootstrapped_config(state_dir, repo_url, tracked_branch);
    let managed_dir = PathBuf::from(&config.managed_dir);
    if managed_dir.exists() {
        if !is_git_repo(&managed_dir) {
            bail!(
                "managed auto-quant directory exists but is not a git repo: '{}'",
                managed_dir.display()
            );
        }
    } else {
        if let Some(parent) = managed_dir.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("creating auto-quant managed parent '{}'", parent.display())
            })?;
        }
        git_output(
            None,
            &[
                "clone",
                "--branch",
                &config.tracked_branch,
                &config.repo_url,
                &config.managed_dir,
            ],
        )?;
    }
    let pinned_ref = current_commit(&managed_dir)?;
    config.pinned_ref = Some(pinned_ref);
    config.last_sync = Some(Utc::now());
    save_config(state_dir, &config)?;
    auto_quant_status(state_dir)
}

pub fn auto_quant_update(
    state_dir: &str,
    repo_url: Option<&str>,
    tracked_branch: Option<&str>,
    target_ref: Option<&str>,
) -> Result<AutoQuantUpdateReport> {
    let mut config = ensure_bootstrapped_config(state_dir, repo_url, tracked_branch);
    let managed_dir = PathBuf::from(&config.managed_dir);
    if !managed_dir.exists() || !is_git_repo(&managed_dir) {
        auto_quant_bootstrap(state_dir, repo_url, tracked_branch)?;
    }
    let managed_dir = PathBuf::from(&config.managed_dir);
    let previous_commit = current_commit(&managed_dir).ok();
    let target_ref = target_ref
        .map(str::to_string)
        .unwrap_or_else(|| config.tracked_branch.clone());
    let mut notes = Vec::new();
    if target_ref == config.tracked_branch {
        git_output(
            Some(&managed_dir),
            &[
                "fetch",
                "origin",
                &config.tracked_branch,
                "--tags",
                "--prune",
            ],
        )?;
    } else {
        git_output(
            Some(&managed_dir),
            &["fetch", "origin", "--tags", "--prune"],
        )?;
    }
    let update_result = if target_ref == config.tracked_branch {
        git_output(Some(&managed_dir), &["checkout", "--detach", "FETCH_HEAD"])
    } else {
        git_output(Some(&managed_dir), &["checkout", "--detach", &target_ref])
    };
    if let Err(err) = update_result {
        notes.push(format!("update_failed={err}"));
        return Err(err);
    }
    let current_commit = current_commit(&managed_dir)?;
    let (healthy, verify_notes) = verify_checkout(&managed_dir);
    notes.extend(verify_notes);
    let mut rolled_back = false;
    if !healthy {
        if let Some(previous_commit) = &previous_commit {
            git_output(
                Some(&managed_dir),
                &["checkout", "--detach", previous_commit],
            )?;
            rolled_back = true;
            notes.push("rolled_back_to_previous_commit".to_string());
        } else {
            bail!(
                "auto-quant update produced an unhealthy checkout and no rollback target existed"
            );
        }
    } else {
        config.pinned_ref = Some(current_commit.clone());
        config.last_sync = Some(Utc::now());
        save_config(state_dir, &config)?;
    }
    Ok(AutoQuantUpdateReport {
        repo_url: config.repo_url,
        managed_dir: config.managed_dir,
        tracked_branch: config.tracked_branch,
        previous_commit,
        target_ref,
        current_commit,
        applied: !rolled_back,
        rolled_back,
        healthy,
        notes,
        adapter_version: config.adapter_version,
        last_sync: config.last_sync.unwrap_or_else(Utc::now),
    })
}
