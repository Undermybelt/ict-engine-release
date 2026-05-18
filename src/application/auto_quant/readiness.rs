use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

use crate::application::release_closure::workflow_next_step_view;
use crate::state::{recommended_next_command_meta, RecommendedNextCommandMeta};

use super::handoff::{
    auto_quant_active_strategy_count, auto_quant_data_ready, auto_quant_prepare_cli_command,
    auto_quant_run_command, auto_quant_workspace_config_for_state, AutoQuantWorkspaceConfig,
};
use super::status::auto_quant_status;
use super::types::AutoQuantDependencyStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoQuantReadinessSurface {
    pub status: String,
    pub healthy: bool,
    pub bootstrap_needed: bool,
    pub dependency_healthy: bool,
    pub data_ready: bool,
    pub update_available: bool,
    pub managed_dir: String,
    pub workspace: AutoQuantWorkspaceConfig,
    pub dependency_status: AutoQuantDependencyStatus,
    pub recommended_next_command: String,
    pub recommended_next_command_meta: RecommendedNextCommandMeta,
    pub next_step: Value,
    pub notes: Vec<String>,
}

pub fn auto_quant_readiness(state_dir: &str) -> Result<AutoQuantReadinessSurface> {
    let dependency_status = auto_quant_status(state_dir)?;
    Ok(auto_quant_readiness_from_status_with_state_dir(
        &dependency_status,
        state_dir,
    ))
}

pub fn auto_quant_readiness_from_status(
    dependency_status: &AutoQuantDependencyStatus,
) -> AutoQuantReadinessSurface {
    let state_dir = Path::new(&dependency_status.managed_dir)
        .parent()
        .and_then(|path| {
            if path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value == ".deps")
            {
                path.parent()
            } else {
                Some(path)
            }
        })
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|| "state".to_string());
    auto_quant_readiness_from_status_with_state_dir(dependency_status, &state_dir)
}

pub fn auto_quant_readiness_from_status_with_state_dir(
    dependency_status: &AutoQuantDependencyStatus,
    state_dir: &str,
) -> AutoQuantReadinessSurface {
    let workspace =
        auto_quant_workspace_config_for_state(&dependency_status.managed_dir, state_dir);
    let data_ready = auto_quant_data_ready(&workspace);
    auto_quant_readiness_from_status_and_data(dependency_status, state_dir, workspace, data_ready)
}

pub fn auto_quant_readiness_from_status_and_data(
    dependency_status: &AutoQuantDependencyStatus,
    state_dir: &str,
    workspace: AutoQuantWorkspaceConfig,
    data_ready: bool,
) -> AutoQuantReadinessSurface {
    let active_strategy_count = auto_quant_active_strategy_count(&workspace);
    let run_command = auto_quant_run_command(&workspace);
    let (status, command, blocked_reason) = if dependency_status.bootstrap_needed {
        (
            "missing_dependency",
            format!("ict-engine auto-quant-bootstrap --state-dir {state_dir}"),
            Some("auto_quant_bootstrap_required"),
        )
    } else if !dependency_status.healthy {
        (
            "dependency_unhealthy",
            format!("ict-engine auto-quant-update --state-dir {state_dir}"),
            Some("auto_quant_dependency_unhealthy"),
        )
    } else if dependency_status.update_available {
        (
            "update_available",
            format!("ict-engine auto-quant-update --state-dir {state_dir}"),
            Some("auto_quant_update_available"),
        )
    } else if !data_ready {
        (
            "dependency_ready_data_missing",
            auto_quant_prepare_cli_command(state_dir),
            Some("auto_quant_prepare_required"),
        )
    } else if active_strategy_count == 0 {
        (
            "dependency_ready_seed_required",
            format!(
                "blocked: create 2-3 active non-underscore strategy files under {} before {}",
                workspace.strategies_dir, run_command
            ),
            Some("auto_quant_seed_strategies_required"),
        )
    } else {
        ("dependency_ready_data_ready", run_command.clone(), None)
    };
    let command = command.to_string();
    let mut notes = dependency_status.notes.clone();
    if let Some(profile) = &workspace.profile_name {
        notes.push(format!("auto_quant_profile={profile}"));
    }
    if data_ready && active_strategy_count == 0 {
        notes.push("auto_quant_seed_strategies_required".to_string());
    }
    AutoQuantReadinessSurface {
        status: status.to_string(),
        healthy: dependency_status.healthy && data_ready,
        bootstrap_needed: dependency_status.bootstrap_needed,
        dependency_healthy: dependency_status.healthy,
        data_ready,
        update_available: dependency_status.update_available,
        managed_dir: dependency_status.managed_dir.clone(),
        workspace,
        dependency_status: dependency_status.clone(),
        recommended_next_command_meta: recommended_next_command_meta(&command),
        next_step: workflow_next_step_view(&command, blocked_reason),
        recommended_next_command: command,
        notes,
    }
}
