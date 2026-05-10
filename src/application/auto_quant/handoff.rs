use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::config::shell_quote;

pub(crate) fn apply_provider_profile_to_command(
    command: &str,
    provider_profile_selector: Option<&str>,
) -> String {
    let Some(profile) = provider_profile_selector.filter(|value| !value.trim().is_empty()) else {
        return command.to_string();
    };
    let trimmed = command.trim();
    if trimmed.is_empty() || trimmed.contains(" --profile ") {
        return command.to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("ask-user: ") {
        if let Some((prefix, deferred)) = rest.split_once("| then ") {
            let rewritten_deferred =
                apply_provider_profile_to_command(deferred.trim(), Some(profile));
            return format!("ask-user: {}| then {}", prefix, rewritten_deferred);
        }
        return command.to_string();
    }
    if trimmed.starts_with("ict-engine workflow-status ")
        || trimmed.starts_with("ict-engine provider-status ")
        || trimmed.starts_with("ict-engine factor-research ")
        || trimmed.starts_with("ict-engine factor-autoresearch ")
    {
        return format!("{} --profile {}", trimmed, shell_quote(profile));
    }
    command.to_string()
}

use super::readiness::{auto_quant_readiness_from_status_and_data, AutoQuantReadinessSurface};
use super::strategy_materials::{discover_strategy_materials, AutoQuantStrategyMaterialSummary};
use super::types::AutoQuantDependencyStatus;
use super::workspace_profile::apply_workspace_profile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoQuantWorkspaceConfig {
    pub repo_root: String,
    pub program_md: String,
    pub prepare_script: String,
    pub run_script: String,
    pub config_json: String,
    pub strategies_dir: String,
    pub data_dir: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expected_data_files: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy_seed_source_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoQuantIterationUnitContext {
    pub unit_label: String,
    pub primitive_sequence: Vec<String>,
    pub timeframe: String,
    pub direction: String,
    pub strategy_brief: String,
    pub evaluation_priority: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consumer_evidence_profile:
        Option<crate::application::auto_quant::pda_unit_batch::AutoQuantConsumerEvidenceProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoQuantResearchHandoffPayload {
    pub artifact_id: String,
    pub handoff_kind: String,
    pub symbol: String,
    pub state_dir: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_profile_selector: Option<String>,
    pub objective: String,
    pub backend: String,
    pub data_path: String,
    pub paired_data_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auxiliary_evidence_path: Option<String>,
    pub mutation_spec_path: Option<String>,
    pub iterations: Option<usize>,
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy_material_root: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_strategy_materials: Vec<AutoQuantStrategyMaterialSummary>,
    pub dependency_status: AutoQuantDependencyStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readiness: Option<AutoQuantReadinessSurface>,
    pub workspace: AutoQuantWorkspaceConfig,
    pub data_ready: bool,
    pub handoff_artifact_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iteration_unit: Option<AutoQuantIterationUnitContext>,
    pub suggested_commands: Vec<String>,
    pub suggested_next_steps: Vec<String>,
    pub agent_prompt: String,
    pub notes: Vec<String>,
}

pub struct AutoQuantFactorResearchCommandInput<'a> {
    pub symbol: &'a str,
    pub data: &'a str,
    pub objective: &'a str,
    pub provider_profile_selector: Option<&'a str>,
    pub paired_data: Option<&'a str>,
    pub auto_quant_profile: Option<&'a str>,
    pub auxiliary_evidence_path: Option<&'a str>,
    pub mutation_spec_path: Option<&'a str>,
    pub strategy_material_root: Option<&'a str>,
    pub state_dir: &'a str,
    pub output_format: &'a str,
}

pub struct AutoQuantFactorAutoresearchCommandInput<'a> {
    pub symbol: &'a str,
    pub data: &'a str,
    pub objective: &'a str,
    pub provider_profile_selector: Option<&'a str>,
    pub paired_data: Option<&'a str>,
    pub auto_quant_profile: Option<&'a str>,
    pub auxiliary_evidence_path: Option<&'a str>,
    pub mutation_spec_path: Option<&'a str>,
    pub strategy_material_root: Option<&'a str>,
    pub iterations: usize,
    pub session_id: Option<&'a str>,
    pub state_dir: &'a str,
}

pub struct BuildFactorResearchHandoffPayloadInput<'a> {
    pub symbol: &'a str,
    pub data: &'a str,
    pub objective: &'a str,
    pub provider_profile_selector: Option<&'a str>,
    pub paired_data: Option<&'a str>,
    pub auxiliary_evidence_path: Option<&'a str>,
    pub mutation_spec_path: Option<&'a str>,
    pub strategy_material_root: Option<&'a str>,
    pub state_dir: &'a str,
    pub dependency_status: AutoQuantDependencyStatus,
}

pub struct BuildFactorAutoresearchHandoffPayloadInput<'a> {
    pub symbol: &'a str,
    pub data: &'a str,
    pub objective: &'a str,
    pub provider_profile_selector: Option<&'a str>,
    pub paired_data: Option<&'a str>,
    pub auxiliary_evidence_path: Option<&'a str>,
    pub mutation_spec_path: Option<&'a str>,
    pub strategy_material_root: Option<&'a str>,
    pub iterations: usize,
    pub session_id: Option<&'a str>,
    pub state_dir: &'a str,
    pub dependency_status: AutoQuantDependencyStatus,
}

pub fn auto_quant_workspace_config(managed_dir: &str) -> AutoQuantWorkspaceConfig {
    let repo_root = PathBuf::from(managed_dir);
    AutoQuantWorkspaceConfig {
        repo_root: repo_root.to_string_lossy().to_string(),
        program_md: repo_root.join("program.md").to_string_lossy().to_string(),
        prepare_script: repo_root.join("prepare.py").to_string_lossy().to_string(),
        run_script: repo_root.join("run.py").to_string_lossy().to_string(),
        config_json: repo_root.join("config.json").to_string_lossy().to_string(),
        strategies_dir: repo_root
            .join("user_data/strategies")
            .to_string_lossy()
            .to_string(),
        data_dir: repo_root
            .join("user_data/data")
            .to_string_lossy()
            .to_string(),
        profile_name: None,
        expected_data_files: Vec::new(),
        strategy_seed_source_dir: None,
    }
}

pub fn auto_quant_workspace_config_for_state(
    managed_dir: &str,
    state_dir: &str,
) -> AutoQuantWorkspaceConfig {
    let mut workspace = auto_quant_workspace_config(managed_dir);
    if let Err(err) = apply_workspace_profile(state_dir, &mut workspace) {
        workspace
            .expected_data_files
            .push(format!("profile_apply_error:{err:#}"));
    }
    workspace
}

pub fn auto_quant_prepare_command(workspace: &AutoQuantWorkspaceConfig) -> String {
    format!("uv run --with ta-lib {}", workspace.prepare_script)
}

pub fn auto_quant_prepare_cli_command(state_dir: &str) -> String {
    format!("ict-engine auto-quant-prepare --state-dir {state_dir}")
}

pub fn auto_quant_run_command(workspace: &AutoQuantWorkspaceConfig) -> String {
    format!("uv run --with ta-lib {}", workspace.run_script)
}

pub fn auto_quant_data_ready(workspace: &AutoQuantWorkspaceConfig) -> bool {
    let data_dir = Path::new(&workspace.data_dir);
    if !data_dir.exists() {
        return false;
    }
    if !workspace.expected_data_files.is_empty() {
        return workspace
            .expected_data_files
            .iter()
            .all(|filename| data_dir.join(filename).exists());
    }
    match std::fs::read_dir(data_dir) {
        Ok(entries) => {
            entries
                .filter_map(Result::ok)
                .filter(|entry| {
                    entry
                        .path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext.eq_ignore_ascii_case("feather"))
                        .unwrap_or(false)
                })
                .count()
                >= 15
        }
        Err(_) => false,
    }
}

pub fn auto_quant_active_strategy_count(workspace: &AutoQuantWorkspaceConfig) -> usize {
    let strategies_dir = Path::new(&workspace.strategies_dir);
    if !strategies_dir.exists() {
        return workspace
            .strategy_seed_source_dir
            .as_deref()
            .map(|path| {
                let mut fallback = workspace.clone();
                fallback.strategies_dir = path.to_string();
                fallback.strategy_seed_source_dir = None;
                auto_quant_active_strategy_count(&fallback)
            })
            .unwrap_or(0);
    }
    match std::fs::read_dir(strategies_dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .filter(|entry| {
                let path = entry.path();
                let is_python = path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("py"))
                    .unwrap_or(false);
                let is_active = entry
                    .file_name()
                    .to_str()
                    .map(|name| !name.starts_with('_'))
                    .unwrap_or(false);
                is_python && is_active
            })
            .count()
            .max(
                workspace
                    .strategy_seed_source_dir
                    .as_deref()
                    .map(|path| {
                        let mut fallback = workspace.clone();
                        fallback.strategies_dir = path.to_string();
                        fallback.strategy_seed_source_dir = None;
                        auto_quant_active_strategy_count(&fallback)
                    })
                    .unwrap_or(0),
            ),
        Err(_) => 0,
    }
}

fn auto_quant_strategy_template_path(workspace: &AutoQuantWorkspaceConfig) -> String {
    PathBuf::from(&workspace.strategies_dir)
        .join("_template.py.example")
        .to_string_lossy()
        .to_string()
}

fn strategy_material_full_path(root: &str, material_path: &str) -> String {
    PathBuf::from(root)
        .join(material_path)
        .to_string_lossy()
        .to_string()
}

fn format_strategy_material_summary(material: &AutoQuantStrategyMaterialSummary) -> String {
    let mut parts = vec![format!("{} [{}]", material.name, material.strategy_path)];
    if let Some(csv_path) = &material.evidence_csv_path {
        parts.push(format!("csv={csv_path}"));
    }
    if material.trade_rows > 0 {
        parts.push(format!("trades={}", material.trade_rows));
    }
    if let Some(total_net_pnl) = material.total_net_pnl {
        parts.push(format!("net_pnl={total_net_pnl:.2}"));
    }
    if material.tp_count > 0 || material.sl_count > 0 || material.be_count > 0 {
        parts.push(format!(
            "tp/sl/be={}/{}/{}",
            material.tp_count, material.sl_count, material.be_count
        ));
    }
    if let Some(average_score) = material.average_score {
        parts.push(format!("avg_score={average_score:.2}"));
    }
    parts.join(", ")
}

pub fn base_suggested_commands(
    workspace: &AutoQuantWorkspaceConfig,
    state_dir: &str,
    data_ready: bool,
    active_strategy_count: usize,
    auxiliary_evidence_path: Option<&str>,
    strategy_material_root: Option<&str>,
    external_strategy_materials: &[AutoQuantStrategyMaterialSummary],
) -> Vec<String> {
    let mut commands = vec![format!("cat {}", workspace.program_md)];
    if let Some(path) = auxiliary_evidence_path.filter(|value| !value.trim().is_empty()) {
        commands.push(format!("cat {}", shell_quote(path)));
    }
    if active_strategy_count == 0 {
        commands.push(format!(
            "cat {}",
            auto_quant_strategy_template_path(workspace)
        ));
        if let Some(root) = strategy_material_root.filter(|value| !value.trim().is_empty()) {
            for material in external_strategy_materials.iter().take(2) {
                let strategy_path = strategy_material_full_path(root, &material.strategy_path);
                commands.push(format!("sed -n '1,160p' {}", shell_quote(&strategy_path)));
                if let Some(csv_path) = &material.evidence_csv_path {
                    let csv_path = strategy_material_full_path(root, csv_path);
                    commands.push(format!("head -n 20 {}", shell_quote(&csv_path)));
                }
            }
        }
    }
    if !data_ready {
        commands.push(auto_quant_prepare_cli_command(state_dir));
    } else {
        commands.push(auto_quant_run_command(workspace));
    }
    commands
}

pub fn suggested_next_steps_for_handoff(
    handoff_kind: &str,
    data_ready: bool,
    active_strategy_count: usize,
    has_external_strategy_materials: bool,
) -> Vec<String> {
    let seed_step = if has_external_strategy_materials {
        "read Auto-Quant program.md, the strategy template, and the attached external strategy material summaries, then create 2-3 active non-underscore strategy files across different paradigms before any run.py execution"
            .to_string()
    } else {
        "read Auto-Quant program.md plus the strategy template, then create 2-3 active non-underscore strategy files across different paradigms before any run.py execution"
            .to_string()
    };
    match (handoff_kind, data_ready, active_strategy_count == 0) {
        ("factor_autoresearch", false, _) => vec![
            "prepare Auto-Quant market data before attempting the autoresearch loop".to_string(),
            "re-run factor-autoresearch with backend=auto-quant after data becomes ready".to_string(),
        ],
        (_, false, _) => vec![
            "prepare Auto-Quant market data before attempting the research loop".to_string(),
            "re-run factor-research with backend=auto-quant after data becomes ready".to_string(),
        ],
        ("factor_autoresearch", true, true) => vec![
            seed_step.clone(),
            "after seeding, run the Auto-Quant loop, keep or discard only from measured backtest results, and export candidate plus retrospective checkpoints back to ict-engine".to_string(),
        ],
        (_, true, true) => vec![
            seed_step,
            "after seeding, run Auto-Quant backtests, keep the best measured candidate, and export the candidate package back to ict-engine".to_string(),
        ],
        ("factor_autoresearch", true, false) => vec![
            "resume or start the Auto-Quant autonomous loop with factor retention and explicit keep/discard review".to_string(),
            "export candidate/retrospective summary back to ict-engine after each iteration checkpoint".to_string(),
        ],
        (_, true, false) => vec![
            "open Auto-Quant program.md and stage a research loop for the requested objective".to_string(),
            "run Auto-Quant backtest loop and export a stable candidate package for ict-engine".to_string(),
        ],
    }
}

fn build_auto_quant_agent_prompt(
    handoff_kind: &str,
    objective: &str,
    workspace: &AutoQuantWorkspaceConfig,
    active_strategy_count: usize,
    auxiliary_evidence_path: Option<&str>,
    strategy_material_root: Option<&str>,
    external_strategy_materials: &[AutoQuantStrategyMaterialSummary],
) -> String {
    let template_path = auto_quant_strategy_template_path(workspace);
    let external_materials_summary = if external_strategy_materials.is_empty() {
        String::new()
    } else {
        let root = strategy_material_root.unwrap_or("<external-strategy-material-root>");
        let materials = external_strategy_materials
            .iter()
            .take(3)
            .map(format_strategy_material_summary)
            .collect::<Vec<_>>()
            .join(" | ");
        format!(
            " Read-only external strategy materials from {} are attached as seed inspiration only; do not execute those scripts directly or carry their absolute-path runtime dependencies into the managed Auto-Quant workspace. Highest-evidence materials: {}.",
            root, materials
        )
    };
    let auxiliary_instruction = auxiliary_evidence_path
        .filter(|value| !value.trim().is_empty())
        .map(|path| {
            format!(
                " Auxiliary/options evidence is attached at {}; treat it as a static market overlay for options_hedging and dealer-positioning judgment rather than inventing a proxy from scratch.",
                path
            )
        })
        .unwrap_or_default();
    let seed_instruction = if active_strategy_count == 0 {
        format!(
            "If {} has no active non-underscore .py strategies, first read {}, create 2-3 seed strategies across different paradigms, prefer archived winners or minimal descendants when available, and only then run {}.{}{}",
            workspace.strategies_dir,
            template_path,
            auto_quant_run_command(workspace),
            external_materials_summary,
            auxiliary_instruction,
        )
    } else {
        format!(
            "Run {} on the current active strategy set, review measured results, and iterate only from backtest evidence.{}{}",
            auto_quant_run_command(workspace),
            external_materials_summary,
            auxiliary_instruction,
        )
    };
    match handoff_kind {
        "factor_autoresearch" => format!(
            "Auto-Quant is the autoresearch execution backend for this request. Keep ict-engine as the control plane, preserve existing ict-engine factors, work the '{}' objective, and read {} before acting. {} Never treat 'no strategies found' as completion. Keep, discard, fork, or kill only from measured results and return a candidate package plus retrospective signals to ict-engine.",
            objective, workspace.program_md, seed_instruction
        ),
        _ => format!(
            "Auto-Quant is the research execution backend for this request. Keep ict-engine as the control plane, preserve old factors, work the '{}' objective, and read {} before acting. {} Never treat 'no strategies found' as completion. Export the best measured candidate package back into ict-engine state.",
            objective, workspace.program_md, seed_instruction
        ),
    }
}

pub fn build_factor_research_handoff_payload(
    input: BuildFactorResearchHandoffPayloadInput<'_>,
) -> AutoQuantResearchHandoffPayload {
    let BuildFactorResearchHandoffPayloadInput {
        symbol,
        data,
        objective,
        provider_profile_selector,
        paired_data,
        auxiliary_evidence_path,
        mutation_spec_path,
        strategy_material_root,
        state_dir,
        dependency_status,
    } = input;
    let workspace =
        auto_quant_workspace_config_for_state(&dependency_status.managed_dir, state_dir);
    let data_ready = auto_quant_data_ready(&workspace);
    let active_strategy_count = auto_quant_active_strategy_count(&workspace);
    let external_strategy_materials = discover_strategy_materials(strategy_material_root, 3);
    let readiness = auto_quant_readiness_from_status_and_data(
        &dependency_status,
        state_dir,
        workspace.clone(),
        data_ready,
    );
    let mut payload = AutoQuantResearchHandoffPayload {
        artifact_id: format!(
            "auto-quant-handoff:factor_research:{}:{}",
            symbol,
            Utc::now().format("%Y%m%dT%H%M%S%.3fZ")
        ),
        handoff_kind: "factor_research".to_string(),
        symbol: symbol.to_string(),
        state_dir: state_dir.to_string(),
        provider_profile_selector: provider_profile_selector.map(str::to_string),
        objective: objective.to_string(),
        backend: "auto-quant".to_string(),
        data_path: data.to_string(),
        paired_data_path: paired_data.map(str::to_string),
        auxiliary_evidence_path: auxiliary_evidence_path.map(str::to_string),
        mutation_spec_path: mutation_spec_path.map(str::to_string),
        iterations: None,
        session_id: None,
        strategy_material_root: strategy_material_root.map(str::to_string),
        external_strategy_materials,
        dependency_status,
        readiness: Some(readiness),
        workspace,
        data_ready,
        handoff_artifact_path: String::new(),
        iteration_unit: None,
        suggested_commands: Vec::new(),
        suggested_next_steps: Vec::new(),
        agent_prompt: String::new(),
        notes: Vec::new(),
    };
    payload.suggested_commands = base_suggested_commands(
        &payload.workspace,
        &payload.state_dir,
        payload.data_ready,
        active_strategy_count,
        payload.auxiliary_evidence_path.as_deref(),
        payload.strategy_material_root.as_deref(),
        &payload.external_strategy_materials,
    );
    payload.suggested_next_steps = suggested_next_steps_for_handoff(
        &payload.handoff_kind,
        payload.data_ready,
        active_strategy_count,
        !payload.external_strategy_materials.is_empty(),
    );
    payload.agent_prompt = build_auto_quant_agent_prompt(
        &payload.handoff_kind,
        &payload.objective,
        &payload.workspace,
        active_strategy_count,
        payload.auxiliary_evidence_path.as_deref(),
        payload.strategy_material_root.as_deref(),
        &payload.external_strategy_materials,
    );
    if !payload.data_ready {
        payload
            .notes
            .push("auto_quant_prepare_required_before_run".to_string());
    }
    if active_strategy_count == 0 {
        payload
            .notes
            .push("auto_quant_seed_strategies_required".to_string());
    }
    payload.notes.push(format!(
        "auto_quant_active_strategy_count={active_strategy_count}"
    ));
    if let Some(path) = &payload.auxiliary_evidence_path {
        payload
            .notes
            .push(format!("auto_quant_auxiliary_evidence_path={path}"));
    }
    if let Some(root) = &payload.strategy_material_root {
        payload
            .notes
            .push(format!("auto_quant_strategy_material_root={root}"));
        payload.notes.push(format!(
            "auto_quant_external_strategy_material_count={}",
            payload.external_strategy_materials.len()
        ));
    }
    for material in payload.external_strategy_materials.iter().take(3) {
        payload.notes.push(format!(
            "auto_quant_external_strategy_material={}",
            format_strategy_material_summary(material)
        ));
    }
    payload.notes.push(format!(
        "requested_at={}",
        Utc::now().format("%Y%m%dT%H%M%S%.3fZ")
    ));
    payload
}

pub fn build_factor_autoresearch_handoff_payload(
    input: BuildFactorAutoresearchHandoffPayloadInput<'_>,
) -> AutoQuantResearchHandoffPayload {
    let BuildFactorAutoresearchHandoffPayloadInput {
        symbol,
        data,
        objective,
        provider_profile_selector,
        paired_data,
        auxiliary_evidence_path,
        mutation_spec_path,
        strategy_material_root,
        iterations,
        session_id,
        state_dir,
        dependency_status,
    } = input;
    let workspace =
        auto_quant_workspace_config_for_state(&dependency_status.managed_dir, state_dir);
    let data_ready = auto_quant_data_ready(&workspace);
    let active_strategy_count = auto_quant_active_strategy_count(&workspace);
    let external_strategy_materials = discover_strategy_materials(strategy_material_root, 3);
    let readiness = auto_quant_readiness_from_status_and_data(
        &dependency_status,
        state_dir,
        workspace.clone(),
        data_ready,
    );
    let mut payload = AutoQuantResearchHandoffPayload {
        artifact_id: format!(
            "auto-quant-handoff:factor_autoresearch:{}:{}",
            symbol,
            Utc::now().format("%Y%m%dT%H%M%S%.3fZ")
        ),
        handoff_kind: "factor_autoresearch".to_string(),
        symbol: symbol.to_string(),
        state_dir: state_dir.to_string(),
        provider_profile_selector: provider_profile_selector.map(str::to_string),
        objective: objective.to_string(),
        backend: "auto-quant".to_string(),
        data_path: data.to_string(),
        paired_data_path: paired_data.map(str::to_string),
        auxiliary_evidence_path: auxiliary_evidence_path.map(str::to_string),
        mutation_spec_path: mutation_spec_path.map(str::to_string),
        iterations: Some(iterations),
        session_id: session_id.map(str::to_string),
        strategy_material_root: strategy_material_root.map(str::to_string),
        external_strategy_materials,
        dependency_status,
        readiness: Some(readiness),
        workspace,
        data_ready,
        handoff_artifact_path: String::new(),
        iteration_unit: None,
        suggested_commands: Vec::new(),
        suggested_next_steps: Vec::new(),
        agent_prompt: String::new(),
        notes: Vec::new(),
    };
    payload.suggested_commands = base_suggested_commands(
        &payload.workspace,
        &payload.state_dir,
        payload.data_ready,
        active_strategy_count,
        payload.auxiliary_evidence_path.as_deref(),
        payload.strategy_material_root.as_deref(),
        &payload.external_strategy_materials,
    );
    payload.suggested_next_steps = suggested_next_steps_for_handoff(
        &payload.handoff_kind,
        payload.data_ready,
        active_strategy_count,
        !payload.external_strategy_materials.is_empty(),
    );
    payload.agent_prompt = build_auto_quant_agent_prompt(
        &payload.handoff_kind,
        &payload.objective,
        &payload.workspace,
        active_strategy_count,
        payload.auxiliary_evidence_path.as_deref(),
        payload.strategy_material_root.as_deref(),
        &payload.external_strategy_materials,
    );
    if !payload.data_ready {
        payload
            .notes
            .push("auto_quant_prepare_required_before_run".to_string());
    }
    if active_strategy_count == 0 {
        payload
            .notes
            .push("auto_quant_seed_strategies_required".to_string());
    }
    payload.notes.push(format!(
        "auto_quant_active_strategy_count={active_strategy_count}"
    ));
    if let Some(path) = &payload.auxiliary_evidence_path {
        payload
            .notes
            .push(format!("auto_quant_auxiliary_evidence_path={path}"));
    }
    if let Some(root) = &payload.strategy_material_root {
        payload
            .notes
            .push(format!("auto_quant_strategy_material_root={root}"));
        payload.notes.push(format!(
            "auto_quant_external_strategy_material_count={}",
            payload.external_strategy_materials.len()
        ));
    }
    for material in payload.external_strategy_materials.iter().take(3) {
        payload.notes.push(format!(
            "auto_quant_external_strategy_material={}",
            format_strategy_material_summary(material)
        ));
    }
    payload.notes.push(format!(
        "requested_at={}",
        Utc::now().format("%Y%m%dT%H%M%S%.3fZ")
    ));
    payload
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::auto_quant::types::AutoQuantDependencyStatus;

    fn healthy_dependency_status_for(managed_dir: &str) -> AutoQuantDependencyStatus {
        AutoQuantDependencyStatus {
            repo_url: "repo".to_string(),
            managed_dir: managed_dir.to_string(),
            tracked_branch: "master".to_string(),
            pinned_ref: None,
            current_commit: None,
            upstream_commit: None,
            bootstrap_needed: false,
            config_present: true,
            managed_repo_present: true,
            healthy: true,
            update_available: false,
            required_files: Vec::new(),
            notes: Vec::new(),
            adapter_version: "v1".to_string(),
            last_sync: None,
        }
    }

    #[test]
    fn research_handoff_attaches_read_only_strategy_material_summary() {
        let temp = tempfile::tempdir().unwrap();
        let managed_dir = temp.path().join("managed-auto-quant");
        let strategies_dir = managed_dir.join("user_data/strategies");
        let data_dir = managed_dir.join("user_data/data");
        std::fs::create_dir_all(&strategies_dir).unwrap();
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::write(managed_dir.join("program.md"), "program").unwrap();
        std::fs::write(managed_dir.join("prepare.py"), "print('prepare')").unwrap();
        std::fs::write(managed_dir.join("run.py"), "print('run')").unwrap();
        std::fs::write(
            strategies_dir.join("_template.py.example"),
            "class Template: pass",
        )
        .unwrap();
        for index in 0..15 {
            std::fs::write(data_dir.join(format!("prepared-{index}.feather")), "ready").unwrap();
        }

        let material_root = temp.path().join("Tomac Material Library");
        std::fs::create_dir_all(&material_root).unwrap();
        std::fs::write(
            material_root.join("trend_runner.py"),
            "class TrendRunner: pass\n",
        )
        .unwrap();
        std::fs::write(
            material_root.join("trend_runner_results.csv"),
            "Time,Net PnL,Result,Score\n2024-01-01,12.5,TP,4.5\n2024-01-02,-2.5,BE,3.5\n",
        )
        .unwrap();

        let payload =
            build_factor_research_handoff_payload(BuildFactorResearchHandoffPayloadInput {
                symbol: "NQ",
                data: "demo.json",
                objective: "expansion_manipulation",
                provider_profile_selector: None,
                paired_data: None,
                auxiliary_evidence_path: None,
                mutation_spec_path: None,
                strategy_material_root: Some(material_root.to_str().unwrap()),
                state_dir: temp.path().to_str().unwrap(),
                dependency_status: healthy_dependency_status_for(managed_dir.to_str().unwrap()),
            });

        assert_eq!(payload.external_strategy_materials.len(), 1);
        assert_eq!(
            payload.external_strategy_materials[0]
                .evidence_csv_path
                .as_deref(),
            Some("trend_runner_results.csv")
        );
        assert!(payload
            .agent_prompt
            .contains("do not execute those scripts directly"));
        assert!(payload
            .notes
            .iter()
            .any(|note| note.starts_with("auto_quant_external_strategy_material_count=1")));
        assert!(payload.suggested_commands.iter().any(|command| {
            command.starts_with("sed -n '1,160p' ")
                && command.contains("'")
                && command.contains("Tomac Material Library/trend_runner.py")
        }));
    }

    #[test]
    fn autoresearch_handoff_preserves_iterations_and_session_id() {
        let temp = tempfile::tempdir().unwrap();
        let managed_dir = temp.path().join("managed-auto-quant");
        let strategies_dir = managed_dir.join("user_data/strategies");
        let data_dir = managed_dir.join("user_data/data");
        std::fs::create_dir_all(&strategies_dir).unwrap();
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::write(managed_dir.join("program.md"), "program").unwrap();
        std::fs::write(managed_dir.join("prepare.py"), "print('prepare')").unwrap();
        std::fs::write(managed_dir.join("run.py"), "print('run')").unwrap();
        std::fs::write(
            strategies_dir.join("_template.py.example"),
            "class Template: pass",
        )
        .unwrap();
        for index in 0..15 {
            std::fs::write(data_dir.join(format!("prepared-{index}.feather")), "ready").unwrap();
        }

        let payload =
            build_factor_autoresearch_handoff_payload(BuildFactorAutoresearchHandoffPayloadInput {
                symbol: "NQ",
                data: "demo.json",
                objective: "expansion_manipulation",
                provider_profile_selector: None,
                paired_data: None,
                auxiliary_evidence_path: None,
                mutation_spec_path: Some("mutation.json"),
                strategy_material_root: None,
                iterations: 3,
                session_id: Some("session-123"),
                state_dir: temp.path().to_str().unwrap(),
                dependency_status: healthy_dependency_status_for(managed_dir.to_str().unwrap()),
            });

        assert_eq!(payload.handoff_kind, "factor_autoresearch");
        assert_eq!(payload.iterations, Some(3));
        assert_eq!(payload.session_id.as_deref(), Some("session-123"));
        assert_eq!(payload.mutation_spec_path.as_deref(), Some("mutation.json"));
        assert!(payload
            .agent_prompt
            .contains("Keep, discard, fork, or kill only from measured results"));
    }

    #[test]
    fn handoff_suggested_commands_use_repo_prepare_wrapper_for_missing_data() {
        let temp = tempfile::tempdir().unwrap();
        let managed_dir = temp.path().join("managed-auto-quant");
        let strategies_dir = managed_dir.join("user_data/strategies");
        std::fs::create_dir_all(&strategies_dir).unwrap();
        std::fs::write(managed_dir.join("program.md"), "program").unwrap();
        std::fs::write(managed_dir.join("prepare.py"), "print('prepare')").unwrap();
        std::fs::write(managed_dir.join("run.py"), "print('run')").unwrap();
        std::fs::write(
            strategies_dir.join("_template.py.example"),
            "class Template: pass",
        )
        .unwrap();

        let missing_data =
            build_factor_research_handoff_payload(BuildFactorResearchHandoffPayloadInput {
                symbol: "NQ",
                data: "demo.json",
                objective: "generic",
                provider_profile_selector: None,
                paired_data: None,
                auxiliary_evidence_path: None,
                mutation_spec_path: None,
                strategy_material_root: None,
                state_dir: temp.path().to_str().unwrap(),
                dependency_status: healthy_dependency_status_for(managed_dir.to_str().unwrap()),
            });
        assert!(missing_data
            .suggested_commands
            .iter()
            .any(|command| command.contains("ict-engine auto-quant-prepare --state-dir")));

        let data_dir = managed_dir.join("user_data/data");
        std::fs::create_dir_all(&data_dir).unwrap();
        for index in 0..15 {
            std::fs::write(data_dir.join(format!("prepared-{index}.feather")), "ready").unwrap();
        }
        std::fs::write(strategies_dir.join("SeedAlpha.py"), "class SeedAlpha: pass").unwrap();

        let ready = build_factor_research_handoff_payload(BuildFactorResearchHandoffPayloadInput {
            symbol: "NQ",
            data: "demo.json",
            objective: "generic",
            provider_profile_selector: None,
            paired_data: None,
            auxiliary_evidence_path: None,
            mutation_spec_path: None,
            strategy_material_root: None,
            state_dir: temp.path().to_str().unwrap(),
            dependency_status: healthy_dependency_status_for(managed_dir.to_str().unwrap()),
        });
        assert!(ready
            .suggested_commands
            .iter()
            .any(|command| command.contains("uv run --with ta-lib")));
    }

    #[test]
    fn handoff_payload_can_carry_iteration_unit_context() {
        let temp = tempfile::tempdir().unwrap();
        let managed_dir = temp.path().join("managed-auto-quant");
        let strategies_dir = managed_dir.join("user_data/strategies");
        std::fs::create_dir_all(&strategies_dir).unwrap();
        std::fs::write(managed_dir.join("program.md"), "program").unwrap();
        std::fs::write(managed_dir.join("prepare.py"), "print('prepare')").unwrap();
        std::fs::write(managed_dir.join("run.py"), "print('run')").unwrap();
        std::fs::write(
            strategies_dir.join("_template.py.example"),
            "class Template: pass",
        )
        .unwrap();

        let mut payload =
            build_factor_research_handoff_payload(BuildFactorResearchHandoffPayloadInput {
                symbol: "NQ",
                data: "demo.json",
                objective: "expansion_manipulation",
                provider_profile_selector: None,
                paired_data: None,
                auxiliary_evidence_path: None,
                mutation_spec_path: None,
                strategy_material_root: None,
                state_dir: temp.path().to_str().unwrap(),
                dependency_status: healthy_dependency_status_for(managed_dir.to_str().unwrap()),
            });
        payload.iteration_unit = Some(AutoQuantIterationUnitContext {
            unit_label: "NQ:15m:long:order_block".to_string(),
            primitive_sequence: vec!["order_block".to_string()],
            timeframe: "15m".to_string(),
            direction: "long".to_string(),
            strategy_brief: "Iterate one order_block long unit. Optimize win rate first."
                .to_string(),
            evaluation_priority: vec![
                "win_rate".to_string(),
                "sharpe".to_string(),
                "return".to_string(),
            ],
            consumer_evidence_profile: None,
        });

        let unit = payload.iteration_unit.as_ref().unwrap();
        assert_eq!(unit.primitive_sequence, vec!["order_block".to_string()]);
        assert_eq!(unit.evaluation_priority[0], "win_rate");
    }

    #[test]
    fn handoff_payload_carries_auxiliary_evidence_path_into_commands_and_prompt() {
        let temp = tempfile::tempdir().unwrap();
        let managed_dir = temp.path().join("managed-auto-quant");
        let strategies_dir = managed_dir.join("user_data/strategies");
        let data_dir = managed_dir.join("user_data/data");
        std::fs::create_dir_all(&strategies_dir).unwrap();
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::write(managed_dir.join("program.md"), "program").unwrap();
        std::fs::write(managed_dir.join("prepare.py"), "print('prepare')").unwrap();
        std::fs::write(managed_dir.join("run.py"), "print('run')").unwrap();
        std::fs::write(
            strategies_dir.join("_template.py.example"),
            "class Template: pass",
        )
        .unwrap();
        for index in 0..15 {
            std::fs::write(data_dir.join(format!("prepared-{index}.feather")), "ready").unwrap();
        }
        let auxiliary_path = temp.path().join("family-g-aux.json");
        std::fs::write(&auxiliary_path, "{}").unwrap();

        let payload =
            build_factor_research_handoff_payload(BuildFactorResearchHandoffPayloadInput {
                symbol: "NQ",
                data: "demo.json",
                objective: "generic",
                provider_profile_selector: None,
                paired_data: None,
                auxiliary_evidence_path: Some(auxiliary_path.to_str().unwrap()),
                mutation_spec_path: None,
                strategy_material_root: None,
                state_dir: temp.path().to_str().unwrap(),
                dependency_status: healthy_dependency_status_for(managed_dir.to_str().unwrap()),
            });

        assert_eq!(
            payload.auxiliary_evidence_path.as_deref(),
            Some(auxiliary_path.to_str().unwrap())
        );
        assert!(payload
            .suggested_commands
            .iter()
            .any(|command| command.contains("family-g-aux.json")));
        assert!(payload
            .agent_prompt
            .contains("Auxiliary/options evidence is attached"));
        assert!(payload
            .notes
            .iter()
            .any(|note| note.contains("auto_quant_auxiliary_evidence_path=")));
    }
}
