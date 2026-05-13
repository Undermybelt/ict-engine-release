use super::{
    adoption::{build_auto_quant_adoption_review, persist_auto_quant_adoption_decision},
    agent_material::{
        dispatch_agent_material_batch, persist_agent_material_batch, rank_agent_material_dispatch,
        AgentMaterialBatchArtifact, AgentMaterialDispatchArtifact, AgentMaterialRankArtifact,
    },
    auto_quant_bootstrap, auto_quant_readiness, auto_quant_status, auto_quant_update,
    handoff::{
        auto_quant_prepare_command as auto_quant_prepare_script_command,
        auto_quant_workspace_config, build_factor_autoresearch_handoff_payload,
        build_factor_research_handoff_payload, AutoQuantFactorAutoresearchCommandInput,
        AutoQuantFactorResearchCommandInput, BuildFactorAutoresearchHandoffPayloadInput,
        BuildFactorResearchHandoffPayloadInput,
    },
    live::{
        consume_live_signals, ConsumeLiveSignalsInput, ConsumeLiveSignalsOutcome, RealRedisSource,
        StreamSource,
    },
    pda_unit_batch::{
        persist_auto_quant_pda_unit_batch, AutoQuantPdaUnitBatchArtifact,
        AutoQuantPdaUnitBatchBuildInput,
    },
    pda_unit_dispatch::{
        dispatch_pda_unit_batch, AutoQuantPdaUnitDispatchArtifact, AutoQuantPdaUnitDispatchInput,
    },
    persistence::persist_handoff_payload,
    real_trades::{ingest_real_trades, IngestRealTradesInput, IngestRealTradesOutcome},
    results::{
        apply_strategy_library_prior_init, cross_check_manifest_against_log,
        find_any_active_prior_init_apply, find_existing_apply_for_library,
        load_strategy_library_manifest, parse_run_ibkr_log, persist_imported_library,
        persist_prior_init_outcome, AutoQuantPriorInitInput, DEFAULT_DEFAULT_PARENT_CONFIG,
        DEFAULT_PRIOR_STRENGTH, DEFAULT_TEMPER, STRATEGY_LIBRARY_FILE,
    },
    seed_evidence::{
        persist_auto_quant_seed_material_evidence, AUTO_QUANT_SEED_MATERIAL_EVIDENCE_DEFAULT_LIMIT,
    },
    AutoQuantDependencyStatus,
};
use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Ledger artifact_kind written by `auto-quant-consume-live-signals`.
pub const ARTIFACT_KIND_LIVE_SIGNALS: &str = "auto_quant_live_signals_ingested";

/// Rule version recorded on every live-signals ledger entry. Bump on
/// any change to the wire schema, persistence layout, or guard
/// semantics.
pub const LIVE_SIGNALS_RULE_VERSION: &str = "auto-quant-live-signals-v1";

use crate::application::output_foundation::{
    print_redacted_json, redact_local_paths_in_human_text,
};
use crate::application::release_closure::workflow_next_step_view;
use crate::bbn::trading::persistence::load_or_init_trading_network;
use crate::state::{save_state, state_exists, BBN_STATE_FILE};

fn ensure_dependency_ready(
    state_dir: &str,
    repo_url: Option<&str>,
    tracked_branch: Option<&str>,
) -> Result<AutoQuantDependencyStatus> {
    let status = auto_quant_status(state_dir)?;
    if status.bootstrap_needed {
        auto_quant_bootstrap(state_dir, repo_url, tracked_branch)
    } else {
        Ok(status)
    }
}

#[derive(Debug, serde::Serialize)]
struct AutoQuantHandoffCompactSurface<'a> {
    summary_line: String,
    handoff_kind: &'a str,
    status: String,
    data_ready: bool,
    recommended_next_command: String,
    review_command: String,
    workflow_status_command: String,
    suggested_next_steps: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
struct AutoQuantReadinessCompactSurface<'a> {
    summary_line: String,
    status: &'a str,
    healthy: bool,
    dependency_healthy: bool,
    data_ready: bool,
    bootstrap_needed: bool,
    update_available: bool,
    recommended_next_command: &'a str,
    notes: &'a [String],
}

fn auto_quant_status_summary_line(
    readiness: &super::readiness::AutoQuantReadinessSurface,
) -> String {
    format!(
        "auto_quant_status {} healthy={} dependency_healthy={} data_ready={} bootstrap_needed={} update_available={}",
        readiness.status,
        readiness.healthy,
        readiness.dependency_healthy,
        readiness.data_ready,
        readiness.bootstrap_needed,
        readiness.update_available
    )
}

fn build_auto_quant_readiness_compact_surface(
    readiness: &super::readiness::AutoQuantReadinessSurface,
) -> AutoQuantReadinessCompactSurface<'_> {
    AutoQuantReadinessCompactSurface {
        summary_line: auto_quant_status_summary_line(readiness),
        status: &readiness.status,
        healthy: readiness.healthy,
        dependency_healthy: readiness.dependency_healthy,
        data_ready: readiness.data_ready,
        bootstrap_needed: readiness.bootstrap_needed,
        update_available: readiness.update_available,
        recommended_next_command: &readiness.recommended_next_command,
        notes: &readiness.notes,
    }
}

fn render_auto_quant_readiness_human_output(
    readiness: &super::readiness::AutoQuantReadinessSurface,
) -> String {
    let mut lines = vec![format!(
        "Auto-Quant status | {} | dependency_healthy={} | data_ready={}",
        readiness.status, readiness.dependency_healthy, readiness.data_ready
    )];
    match readiness.status.as_str() {
        "missing_dependency" => {
            lines.push("Next: bootstrap the managed Auto-Quant checkout".to_string())
        }
        "dependency_unhealthy" => {
            lines.push("Next: repair the managed Auto-Quant checkout before use".to_string())
        }
        "update_available" => lines
            .push("Next: update the managed Auto-Quant checkout to the tracked ref".to_string()),
        "dependency_ready_data_missing" => {
            lines.push("Next: prepare Auto-Quant market data before strategy execution".to_string())
        }
        "dependency_ready_seed_required" => lines.push(
            "Next: add 2-3 active non-underscore strategy files before external execution"
                .to_string(),
        ),
        "dependency_ready_data_ready" => {
            lines.push("Next: workspace is ready for managed external execution".to_string())
        }
        _ => {}
    }
    if readiness.recommended_next_command.starts_with("blocked:") {
        lines.push(format!(
            "Block: {}",
            readiness
                .recommended_next_command
                .trim_start_matches("blocked:")
                .trim()
        ));
    } else if !readiness.recommended_next_command.trim().is_empty() {
        lines.push(format!("Run: {}", readiness.recommended_next_command));
    }
    lines.push(format!(
        "Workspace: repo={} | data={} | strategies={}",
        readiness.workspace.repo_root,
        readiness.workspace.data_dir,
        readiness.workspace.strategies_dir
    ));
    if !readiness.notes.is_empty() {
        lines.push(format!("Notes: {}", readiness.notes.join(" | ")));
    }
    redact_local_paths_in_human_text(&lines.join("\n"))
}

fn auto_quant_handoff_recommended_next_command(
    payload: &super::handoff::AutoQuantResearchHandoffPayload,
) -> String {
    super::handoff::apply_provider_profile_to_command(
        payload
            .readiness
            .as_ref()
            .map(|readiness| readiness.recommended_next_command.clone())
            .unwrap_or_default()
            .as_str(),
        payload.provider_profile_selector.as_deref(),
    )
}

fn auto_quant_handoff_review_command(
    payload: &super::handoff::AutoQuantResearchHandoffPayload,
) -> String {
    format!(
        "ict-engine auto-quant-adoption-review --symbol {} --state-dir {} --artifact-id {}",
        payload.symbol, payload.state_dir, payload.artifact_id
    )
}

fn auto_quant_handoff_workflow_status_command(
    payload: &super::handoff::AutoQuantResearchHandoffPayload,
) -> String {
    format!(
        "{} --human",
        super::handoff::apply_provider_profile_to_command(
            &format!(
                "ict-engine workflow-status --symbol {} --state-dir {}",
                payload.symbol, payload.state_dir
            ),
            payload.provider_profile_selector.as_deref(),
        )
    )
}

fn render_auto_quant_handoff_human_output(
    payload: &super::handoff::AutoQuantResearchHandoffPayload,
) -> String {
    let status = payload
        .readiness
        .as_ref()
        .map(|readiness| readiness.status.clone())
        .unwrap_or_else(|| {
            if payload.data_ready {
                "dependency_ready_data_ready".to_string()
            } else {
                "dependency_ready_data_missing".to_string()
            }
        });
    let next_command = auto_quant_handoff_recommended_next_command(payload);
    let mut lines = vec![format!(
        "Auto-Quant handoff | status={} | objective={} | data_ready={}",
        status, payload.objective, payload.data_ready
    )];
    if let Some(step) = payload.suggested_next_steps.first() {
        lines.push(format!("Next: {}", step));
    }
    if !next_command.trim().is_empty() {
        lines.push(format!("Run: {}", next_command));
    }
    lines.push(format!(
        "Review: {}",
        auto_quant_handoff_review_command(payload)
    ));
    lines.push(format!(
        "Workflow: {}",
        auto_quant_handoff_workflow_status_command(payload)
    ));
    redact_local_paths_in_human_text(&lines.join("\n"))
}

fn build_auto_quant_handoff_compact_surface(
    payload: &super::handoff::AutoQuantResearchHandoffPayload,
) -> AutoQuantHandoffCompactSurface<'_> {
    AutoQuantHandoffCompactSurface {
        summary_line: format!(
            "auto_quant_handoff {} data_ready={} objective={}",
            payload.handoff_kind, payload.data_ready, payload.objective
        ),
        handoff_kind: &payload.handoff_kind,
        status: payload
            .readiness
            .as_ref()
            .map(|readiness| readiness.status.clone())
            .unwrap_or_else(|| "status_unavailable".to_string()),
        data_ready: payload.data_ready,
        recommended_next_command: auto_quant_handoff_recommended_next_command(payload),
        review_command: auto_quant_handoff_review_command(payload),
        workflow_status_command: auto_quant_handoff_workflow_status_command(payload),
        suggested_next_steps: payload.suggested_next_steps.clone(),
    }
}

fn build_auto_quant_handoff_output_payload(
    payload: &super::handoff::AutoQuantResearchHandoffPayload,
) -> serde_json::Value {
    let recommended_next_command = auto_quant_handoff_recommended_next_command(payload);
    let recommended_next_step = payload
        .readiness
        .as_ref()
        .map(|readiness| readiness.next_step.clone())
        .unwrap_or_else(|| workflow_next_step_view(&recommended_next_command, None));
    json!({
        "auto_quant_handoff_candidate": payload,
        "recommended_next_command": recommended_next_command,
        "recommended_next_step": recommended_next_step,
        "review_command": auto_quant_handoff_review_command(payload),
        "workflow_status_command": auto_quant_handoff_workflow_status_command(payload),
        "suggested_next_steps": payload.suggested_next_steps,
        "human_output": render_auto_quant_handoff_human_output(payload),
    })
}

fn emit_auto_quant_handoff_output(
    output_format: &str,
    payload: &super::handoff::AutoQuantResearchHandoffPayload,
) -> Result<()> {
    let structured = build_auto_quant_handoff_output_payload(payload);
    let compact = build_auto_quant_handoff_compact_surface(payload);
    match output_format.trim().to_ascii_lowercase().as_str() {
        "json" | "agent" => println!("{}", serde_json::to_string_pretty(&structured)?),
        "compact" => print_redacted_json(&compact)?,
        "human" => println!(
            "{}",
            redact_local_paths_in_human_text(
                structured["human_output"].as_str().unwrap_or_default()
            )
        ),
        other => bail!("unsupported auto-quant output format '{}'", other),
    }
    Ok(())
}

pub fn auto_quant_status_command(state_dir: &str, output_format: &str) -> Result<()> {
    let readiness = auto_quant_readiness(state_dir)?;
    match output_format.trim().to_ascii_lowercase().as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&readiness)?);
            Ok(())
        }
        "compact" => print_redacted_json(&build_auto_quant_readiness_compact_surface(&readiness)),
        "human" => {
            println!("{}", render_auto_quant_readiness_human_output(&readiness));
            Ok(())
        }
        other => bail!("unsupported auto-quant output format '{}'", other),
    }
}

pub fn auto_quant_bootstrap_command(
    state_dir: &str,
    repo_url: Option<&str>,
    tracked_branch: Option<&str>,
) -> Result<()> {
    let status = auto_quant_bootstrap(state_dir, repo_url, tracked_branch)?;
    println!("{}", serde_json::to_string_pretty(&status)?);
    Ok(())
}

pub fn auto_quant_update_command(
    state_dir: &str,
    repo_url: Option<&str>,
    tracked_branch: Option<&str>,
    target_ref: Option<&str>,
) -> Result<()> {
    let report = auto_quant_update(state_dir, repo_url, tracked_branch, target_ref)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

pub fn auto_quant_prepare_workspace_command(state_dir: &str) -> Result<()> {
    let readiness_before = auto_quant_readiness(state_dir)?;
    if readiness_before.bootstrap_needed {
        bail!(
            "auto-quant dependency is missing; bootstrap first with ict-engine auto-quant-bootstrap --state-dir {}",
            state_dir
        );
    }
    if !readiness_before.dependency_healthy {
        bail!(
            "auto-quant dependency is unhealthy; repair it first with ict-engine auto-quant-update --state-dir {}",
            state_dir
        );
    }
    let prepare_command = auto_quant_prepare_script_command(&readiness_before.workspace);
    let workspace_root = absolute_path(&readiness_before.workspace.repo_root)?;
    let prepare_script = absolute_path(&readiness_before.workspace.prepare_script)?;
    let output = if let Some(profile) = super::workspace_profile::materialize_workspace_profile(
        state_dir,
        &readiness_before.workspace,
    )? {
        let csv_path = workspace_root.join("profile_source.csv");
        let timeframes = std::iter::once(profile.base_timeframe.clone())
            .chain(profile.additional_timeframes.clone())
            .collect::<Vec<_>>()
            .join(",");
        Command::new("uv")
            .args([
                "run",
                "--with",
                "ta-lib",
                path_str(&prepare_script, "prepare script")?,
                "--csv",
                csv_path.to_str().unwrap_or("profile_source.csv"),
                "--pair",
                profile.pair.as_str(),
                "--timeframes",
                timeframes.as_str(),
                "--datadir",
                "user_data/data",
                "--column-map",
                "date:date,open:open,high:high,low:low,close:close,volume:volume",
                "--no-clean",
            ])
            .current_dir(&workspace_root)
            .output()
            .with_context(|| format!("failed to launch {}", prepare_command))?
    } else {
        Command::new("uv")
            .args([
                "run",
                "--with",
                "ta-lib",
                path_str(&prepare_script, "prepare script")?,
            ])
            .current_dir(&workspace_root)
            .output()
            .with_context(|| format!("failed to launch {}", prepare_command))?
    };
    if !output.status.success() {
        bail!(
            "auto-quant prepare failed with status {} while running {}.\nstdout:\n{}\nstderr:\n{}",
            output.status,
            prepare_command,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let readiness_after = auto_quant_readiness(state_dir)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "status": "prepared",
            "state_dir": state_dir,
            "prepare_command": "ict-engine auto-quant-prepare",
            "workspace_repo_root": readiness_before.workspace.repo_root,
            "dependency_status_before": readiness_before.status,
            "dependency_status_after": readiness_after.status,
            "data_ready": readiness_after.data_ready,
            "next_step": workflow_next_step_view(
                &format!("ict-engine auto-quant-status --state-dir {}", state_dir),
                None
            ),
        }))?
    );
    Ok(())
}

fn absolute_path(path: &str) -> Result<PathBuf> {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn path_str<'a>(path: &'a Path, label: &str) -> Result<&'a str> {
    path.to_str()
        .ok_or_else(|| anyhow!("{label} path is not valid UTF-8: {}", path.display()))
}

pub fn auto_quant_adoption_review_command(
    symbol: &str,
    state_dir: &str,
    artifact_id: Option<&str>,
) -> Result<()> {
    let review = build_auto_quant_adoption_review(symbol, state_dir, artifact_id)?;
    println!("{}", serde_json::to_string_pretty(&review)?);
    Ok(())
}

pub fn auto_quant_adoption_decision_command(
    symbol: &str,
    state_dir: &str,
    artifact_id: Option<&str>,
    decision: &str,
    rationale: &str,
    requested_by: &str,
) -> Result<()> {
    let artifact = persist_auto_quant_adoption_decision(
        symbol,
        state_dir,
        artifact_id,
        decision,
        rationale,
        requested_by,
    )?;
    println!("{}", serde_json::to_string_pretty(&artifact)?);
    Ok(())
}

pub fn auto_quant_seed_evidence_command(
    symbol: &str,
    state_dir: &str,
    strategy_material_root: &str,
    limit: usize,
) -> Result<()> {
    let dependency_status = ensure_dependency_ready(state_dir, None, None)?;
    let workspace = auto_quant_workspace_config(&dependency_status.managed_dir);
    let artifact = persist_auto_quant_seed_material_evidence(
        symbol,
        state_dir,
        Some(strategy_material_root),
        &workspace,
        limit,
    )?
    .ok_or_else(|| {
        anyhow!(
            "no external strategy materials with usable evidence were found under '{}'",
            strategy_material_root
        )
    })?;
    println!("{}", serde_json::to_string_pretty(&artifact)?);
    Ok(())
}

pub struct AutoQuantPdaUnitBatchCommandInput<'a> {
    pub symbol: &'a str,
    pub objective: &'a str,
    pub factors: &'a str,
    pub combination_size: usize,
    pub directions: &'a str,
    pub timeframes: &'a str,
    pub timeframe_data: &'a [String],
    pub evidence_surfaces: &'a str,
    pub indicator_list: &'a str,
    pub evidence_notes: &'a [String],
    pub max_parallel: usize,
    pub state_dir: &'a str,
    pub repo_url: Option<&'a str>,
    pub tracked_branch: Option<&'a str>,
}

pub struct AutoQuantPdaUnitDispatchCommandInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
    pub batch_artifact_id: Option<&'a str>,
    pub group_indices: Option<&'a str>,
}

pub struct AutoQuantAgentMaterialBatchCommandInput<'a> {
    pub symbol: &'a str,
    pub material_paths: &'a [String],
    pub max_parallel: usize,
    pub state_dir: &'a str,
    pub repo_url: Option<&'a str>,
    pub tracked_branch: Option<&'a str>,
}

pub struct AutoQuantAgentMaterialDispatchCommandInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
    pub group_indices: Option<&'a str>,
}

pub struct AutoQuantAgentMaterialRankCommandInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
}

pub fn auto_quant_pda_unit_batch_command(
    input: AutoQuantPdaUnitBatchCommandInput<'_>,
) -> Result<()> {
    let dependency_status =
        ensure_dependency_ready(input.state_dir, input.repo_url, input.tracked_branch)?;
    let artifact = persist_auto_quant_pda_unit_batch(AutoQuantPdaUnitBatchBuildInput {
        symbol: input.symbol,
        objective: input.objective,
        factors: input.factors,
        combination_size: input.combination_size,
        directions: input.directions,
        timeframes: input.timeframes,
        timeframe_data_entries: input.timeframe_data,
        evidence_surfaces: input.evidence_surfaces,
        indicator_list: input.indicator_list,
        evidence_notes: input.evidence_notes,
        max_parallel: input.max_parallel,
        state_dir: input.state_dir,
        dependency_status,
    })?;
    print_pda_unit_batch_summary(&artifact)?;
    Ok(())
}

fn print_pda_unit_batch_summary(artifact: &AutoQuantPdaUnitBatchArtifact) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(artifact)?);
    Ok(())
}

pub fn auto_quant_pda_unit_dispatch_command(
    input: AutoQuantPdaUnitDispatchCommandInput<'_>,
) -> Result<()> {
    let artifact = dispatch_pda_unit_batch(AutoQuantPdaUnitDispatchInput {
        symbol: input.symbol,
        state_dir: input.state_dir,
        batch_artifact_id: input.batch_artifact_id,
        group_indices: input.group_indices,
    })?;
    print_pda_unit_dispatch_summary(&artifact)?;
    Ok(())
}

fn print_pda_unit_dispatch_summary(artifact: &AutoQuantPdaUnitDispatchArtifact) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(artifact)?);
    Ok(())
}

pub fn auto_quant_agent_material_batch_command(
    input: AutoQuantAgentMaterialBatchCommandInput<'_>,
) -> Result<()> {
    let dependency_status =
        ensure_dependency_ready(input.state_dir, input.repo_url, input.tracked_branch)?;
    let artifact = persist_agent_material_batch(
        input.symbol,
        input.state_dir,
        &dependency_status.managed_dir,
        Some(&dependency_status.repo_url),
        input.max_parallel,
        input.material_paths,
    )?;
    print_agent_material_batch_summary(&artifact)
}

fn print_agent_material_batch_summary(artifact: &AgentMaterialBatchArtifact) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(artifact)?);
    Ok(())
}

pub fn auto_quant_agent_material_dispatch_command(
    input: AutoQuantAgentMaterialDispatchCommandInput<'_>,
) -> Result<()> {
    let artifact =
        dispatch_agent_material_batch(input.state_dir, input.symbol, input.group_indices)?;
    print_agent_material_dispatch_summary(&artifact)
}

fn print_agent_material_dispatch_summary(artifact: &AgentMaterialDispatchArtifact) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(artifact)?);
    Ok(())
}

pub fn auto_quant_agent_material_rank_command(
    input: AutoQuantAgentMaterialRankCommandInput<'_>,
) -> Result<()> {
    let artifact = rank_agent_material_dispatch(input.state_dir, input.symbol)?;
    print_agent_material_rank_summary(&artifact)
}

fn print_agent_material_rank_summary(artifact: &AgentMaterialRankArtifact) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(artifact)?);
    Ok(())
}

fn maybe_persist_seed_material_evidence(
    symbol: &str,
    state_dir: &str,
    strategy_material_root: Option<&str>,
    dependency_status: &AutoQuantDependencyStatus,
) -> Result<Option<String>> {
    let workspace = auto_quant_workspace_config(&dependency_status.managed_dir);
    let artifact = persist_auto_quant_seed_material_evidence(
        symbol,
        state_dir,
        strategy_material_root,
        &workspace,
        AUTO_QUANT_SEED_MATERIAL_EVIDENCE_DEFAULT_LIMIT,
    )?;
    Ok(artifact.map(|item| item.artifact_id))
}

pub fn auto_quant_factor_research_command(
    input: AutoQuantFactorResearchCommandInput<'_>,
) -> Result<()> {
    let AutoQuantFactorResearchCommandInput {
        symbol,
        data,
        objective,
        provider_profile_selector,
        paired_data,
        auto_quant_profile,
        auxiliary_evidence_path,
        mutation_spec_path,
        strategy_material_root,
        state_dir,
        output_format,
    } = input;
    super::workspace_profile::persist_workspace_profile_selection(
        state_dir,
        auto_quant_profile,
        symbol,
        data,
    )?;
    let dependency_status = ensure_dependency_ready(state_dir, None, None)?;
    let seed_evidence_artifact_id = maybe_persist_seed_material_evidence(
        symbol,
        state_dir,
        strategy_material_root,
        &dependency_status,
    )
    .map_err(|err| {
        eprintln!(
            "warning: failed to persist auto-quant seed material evidence for {}: {err:#}",
            symbol
        );
        err
    })
    .ok()
    .flatten();
    let mut payload =
        build_factor_research_handoff_payload(BuildFactorResearchHandoffPayloadInput {
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
        });
    if let Some(artifact_id) = seed_evidence_artifact_id {
        payload.notes.push(format!(
            "auto_quant_seed_material_evidence_artifact_id={}",
            artifact_id
        ));
    }
    let handoff_path = persist_handoff_payload(state_dir, &payload)?;
    payload.handoff_artifact_path = handoff_path;
    emit_auto_quant_handoff_output(output_format, &payload)
}

pub fn auto_quant_factor_autoresearch_command(
    input: AutoQuantFactorAutoresearchCommandInput<'_>,
) -> Result<()> {
    let AutoQuantFactorAutoresearchCommandInput {
        symbol,
        data,
        objective,
        provider_profile_selector,
        paired_data,
        auto_quant_profile,
        auxiliary_evidence_path,
        mutation_spec_path,
        strategy_material_root,
        iterations,
        session_id,
        state_dir,
    } = input;
    super::workspace_profile::persist_workspace_profile_selection(
        state_dir,
        auto_quant_profile,
        symbol,
        data,
    )?;
    let dependency_status = ensure_dependency_ready(state_dir, None, None)?;
    let seed_evidence_artifact_id = maybe_persist_seed_material_evidence(
        symbol,
        state_dir,
        strategy_material_root,
        &dependency_status,
    )
    .map_err(|err| {
        eprintln!(
            "warning: failed to persist auto-quant seed material evidence for {}: {err:#}",
            symbol
        );
        err
    })
    .ok()
    .flatten();
    let mut payload =
        build_factor_autoresearch_handoff_payload(BuildFactorAutoresearchHandoffPayloadInput {
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
        });
    if let Some(artifact_id) = seed_evidence_artifact_id {
        payload.notes.push(format!(
            "auto_quant_seed_material_evidence_artifact_id={artifact_id}"
        ));
    }
    let handoff_path = persist_handoff_payload(state_dir, &payload)?;
    payload.handoff_artifact_path = handoff_path;
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

/// Inputs for `auto_quant_prior_init_command`. Pulled out so the
/// CLI surface can default sensibly without burning a large positional
/// signature.
pub struct AutoQuantPriorInitCommandInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
    /// Optional path to a `strategy_library.json`. If `None`, the
    /// command falls back to the canonical state copy
    /// (`<state_dir>/<symbol>/auto_quant_strategy_library.json`)
    /// produced by a prior `auto-quant-results-import`.
    pub library_path: Option<&'a str>,
    /// If `Some`, restrict prior init to these strategy names.
    pub strategy_filter: Option<&'a [String]>,
    /// `temper ∈ [0, 1]`. Defaults to `DEFAULT_TEMPER`.
    pub temper: Option<f64>,
    /// Defaults to `DEFAULT_PRIOR_STRENGTH`.
    pub prior_strength: Option<f64>,
    /// Length-3 parent config `[entry_quality, factor_alignment,
    /// factor_uncertainty]`. Defaults to `[0, 0, 0]`.
    pub parent_config: Option<Vec<usize>>,
    /// If `true`, compute and emit the diff but do not persist the
    /// mutated trading network.
    pub dry_run: bool,
    /// Override the ledger-enforced single-apply guard. By default the
    /// command refuses a non-dry-run apply when an
    /// `auto_quant_prior_init_applied` entry with `status="applied"`
    /// already exists for the same `library_artifact_id`, because a
    /// second tempered pseudo-count layer would silently double the
    /// effective evidence weight on the trade_outcome row. Set to
    /// `true` only after consciously rolling back the BBN snapshot
    /// (e.g. by deleting `bbn_network.json`).
    pub force: bool,
}

/// Validate a `strategy_library.json` produced by Auto-Quant's
/// `export_strategy_library.py`, persist a canonical copy in the
/// symbol's state directory, and emit an
/// `auto_quant_strategy_library_validated` ledger entry.
///
/// When `log_path` is `Some`, additionally cross-check the manifest
/// against the canonical `run_ibkr.log` blocks: drift between the
/// two is surfaced in the summary but does **not** fail the import
/// (export + log are produced from independent code paths in
/// Auto-Quant; a divergence is a finding to raise, not a blocker).
pub fn auto_quant_results_import_command(
    symbol: &str,
    state_dir: &str,
    library_path: &str,
    log_path: Option<&str>,
) -> Result<()> {
    let manifest = load_strategy_library_manifest(library_path)
        .with_context(|| format!("loading strategy library from '{}'", library_path))?;
    let persisted = persist_imported_library(state_dir, symbol, &manifest, library_path)?;

    let cross_check = match log_path {
        Some(path) => {
            let blocks = parse_run_ibkr_log(path)
                .with_context(|| format!("parsing run_ibkr log '{}'", path))?;
            Some(cross_check_manifest_against_log(&manifest, &blocks))
        }
        None => None,
    };

    let summary = json!({
        "command": "auto-quant-results-import",
        "symbol": symbol,
        "library_source": library_path,
        "library_state_path": persisted.state_path,
        "library_artifact_id": persisted.artifact_id,
        "manifest_version": manifest.manifest_version,
        "auto_quant_pinned_ref": manifest.auto_quant_pinned_ref,
        "n_total_strategies": persisted.n_total_strategies,
        "n_ok": persisted.n_ok,
        "n_error": persisted.n_error,
        "n_not_run": persisted.n_not_run,
        "n_meta_invalid": manifest.validation_errors.len(),
        "log_cross_check": cross_check,
    });
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

/// Apply tempered Beta-Binomial pseudo-counts derived from a
/// previously-imported Auto-Quant strategy library to the trading
/// network's `trade_outcome` CPT row identified by `parent_config`.
pub fn auto_quant_prior_init_command(input: AutoQuantPriorInitCommandInput<'_>) -> Result<()> {
    let AutoQuantPriorInitCommandInput {
        symbol,
        state_dir,
        library_path,
        strategy_filter,
        temper,
        prior_strength,
        parent_config,
        dry_run,
        force,
    } = input;

    let library_state_path =
        crate::state::artifact_state_path(state_dir, symbol, STRATEGY_LIBRARY_FILE);
    let resolved_path = match library_path {
        Some(p) => p.to_string(),
        None => {
            if !state_exists(state_dir, symbol, STRATEGY_LIBRARY_FILE) {
                bail!(
                    "no library found at '{}' and no --library override given; \
                     run `ict-engine auto-quant-results-import` first",
                    library_state_path
                );
            }
            library_state_path.clone()
        }
    };
    let manifest = load_strategy_library_manifest(&resolved_path)
        .with_context(|| format!("loading strategy library from '{}'", resolved_path))?;

    // Resolve lineage early so the apply-guard can inspect the ledger
    // before any expensive math runs.
    let library_artifact_id =
        resolve_library_artifact_id(state_dir, symbol).unwrap_or_else(|| resolved_path.clone());

    let existing_apply = find_existing_apply_for_library(state_dir, symbol, &library_artifact_id)?;
    let cross_library_apply = find_any_active_prior_init_apply(state_dir, symbol)?;
    if !dry_run && !force {
        if let Some(prior_apply) = existing_apply.as_ref() {
            bail!(
                "library '{lib}' has already been applied via '{prior}'. \
                 Re-applying would silently double the tempered pseudo-counts. \
                 Roll back the BBN snapshot (`rm <state_dir>/<symbol>/bbn_network.json`) \
                 and pass --force to override, or re-run --dry-run to inspect the diff.",
                lib = library_artifact_id,
                prior = prior_apply,
            );
        }
        if let Some((prior_apply, prior_lib)) = cross_library_apply.as_ref() {
            // The same-library check is exhaustive against `library_artifact_id`,
            // so if we get here the prior apply belongs to a *different* library
            // (typically v1, where v2 was just imported and auto-superseded v1).
            // Without this guard the v2 mutation would stack on top of v1's still-live
            // CPT effect and silently double the evidence weight.
            let prior_lib_str = prior_lib.as_deref().unwrap_or("(unknown library)");
            bail!(
                "BBN already carries an Auto-Quant prior init from library '{prior_lib}' \
                 (apply '{prior_apply}'); current request targets library '{lib}'. \
                 Re-applying without rollback would stack two pseudo-count layers on the same \
                 trade_outcome row. Roll back the BBN snapshot \
                 (`rm <state_dir>/<symbol>/bbn_network.json` and re-run import + prior-init) \
                 or pass --force to deliberately stack.",
                prior_lib = prior_lib_str,
                prior_apply = prior_apply,
                lib = library_artifact_id,
            );
        }
    }

    let temper = temper.unwrap_or(DEFAULT_TEMPER);
    let prior_strength = prior_strength.unwrap_or(DEFAULT_PRIOR_STRENGTH);
    let parent_config = parent_config.unwrap_or_else(|| DEFAULT_DEFAULT_PARENT_CONFIG.to_vec());

    let mut network = load_or_init_trading_network(symbol, state_dir)?;
    let outcome = apply_strategy_library_prior_init(
        &mut network,
        AutoQuantPriorInitInput {
            manifest: &manifest,
            strategy_filter,
            parent_config: parent_config.clone(),
            temper,
            prior_strength,
        },
    )?;

    if !dry_run && !outcome.strategies_applied.is_empty() {
        save_state(state_dir, symbol, BBN_STATE_FILE, &network)
            .context("persisting updated trading network after prior init")?;
    }

    let persisted = persist_prior_init_outcome(
        state_dir,
        symbol,
        &outcome,
        &library_artifact_id,
        &resolved_path,
        dry_run,
    )?;

    let cross_library_apply_json = cross_library_apply.as_ref().map(|(apply_id, lib_id)| {
        json!({
            "apply_artifact_id": apply_id,
            "library_artifact_id": lib_id,
        })
    });

    let summary = json!({
        "command": "auto-quant-prior-init",
        "symbol": symbol,
        "library_path": resolved_path,
        "library_artifact_id": library_artifact_id,
        "existing_apply_artifact_id": existing_apply,
        "cross_library_apply": cross_library_apply_json,
        "force": force,
        "prior_init_artifact_id": persisted.artifact_id,
        "prior_init_state_path": persisted.state_path,
        "prior_init_history_path": persisted.history_path,
        "dry_run": dry_run,
        "temper": temper,
        "prior_strength": prior_strength,
        "parent_config": outcome.parent_config,
        "initial_probs": outcome.initial_probs,
        "final_probs": outcome.final_probs,
        "bbn_entropy_reduction": outcome.bbn_entropy_reduction,
        "bbn_log_loss_delta": outcome.bbn_log_loss_delta,
        "bbn_contradiction_lift": outcome.bbn_contradiction_lift,
        "evidence_value_gate_passed": outcome.evidence_value_gate_passed,
        "strategies_applied": outcome.strategies_applied,
        "strategies_skipped": outcome.strategies_skipped,
    });
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

/// Look up the most recently appended `auto_quant_strategy_library_validated`
/// entry's artifact_id so we can pin lineage in the prior_init ledger entry.
fn resolve_library_artifact_id(state_dir: &str, symbol: &str) -> Option<String> {
    let ledger: Vec<crate::state::ArtifactLedgerEntry> =
        crate::state::load_state_or_default(state_dir, symbol, crate::state::ARTIFACT_LEDGER_FILE)
            .ok()?;
    ledger
        .into_iter()
        .rev()
        .find(|entry| entry.artifact_kind == super::results::ARTIFACT_KIND_LIBRARY)
        .map(|entry| entry.artifact_id)
}

/// Operator-facing input for `auto-quant-consume-live-signals`.
#[derive(Debug, Clone)]
pub struct AutoQuantConsumeLiveSignalsInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
    pub redis_url: &'a str,
    pub max_iterations: Option<u32>,
    pub block_ms: u64,
    pub initial_id: &'a str,
}

/// Drive the live-signals consumer until shutdown (or `max_iterations`
/// is reached), persist the JSONL log + cursor, then write a
/// `auto_quant_live_signals_ingested` ledger entry summarising the
/// session. The session is **not** required to have processed any
/// envelopes — a zero-envelope session still emits a ledger entry
/// with `status = "no_op"`, which makes "did the consumer connect
/// and run?" auditable independently of "did it see any data?".
pub fn auto_quant_consume_live_signals_command(
    input: AutoQuantConsumeLiveSignalsInput<'_>,
) -> Result<()> {
    let mut source = RealRedisSource::connect(input.redis_url).with_context(|| {
        format!(
            "connecting to redis at '{}' (sanitised: {})",
            sanitise_redis_url(input.redis_url),
            sanitise_redis_url(input.redis_url),
        )
    })?;

    auto_quant_consume_live_signals_with_source(input, &mut source)
}

/// Same as [`auto_quant_consume_live_signals_command`] but takes an
/// arbitrary [`StreamSource`] so tests can drive the full path
/// (parse → JSONL → cursor → ledger) without a real Redis.
pub fn auto_quant_consume_live_signals_with_source<S: StreamSource>(
    input: AutoQuantConsumeLiveSignalsInput<'_>,
    source: &mut S,
) -> Result<()> {
    let consumer_input = ConsumeLiveSignalsInput {
        symbol: input.symbol.to_string(),
        state_dir: std::path::PathBuf::from(input.state_dir),
        redis_url: input.redis_url.to_string(),
        max_iterations: input.max_iterations,
        block_ms: input.block_ms,
        initial_id: input.initial_id.to_string(),
    };
    let outcome = consume_live_signals(&consumer_input, source)
        .with_context(|| format!("consuming live signals for symbol '{}'", input.symbol))?;

    let persisted_artifact = persist_live_signals_session(input.state_dir, input.symbol, &outcome)
        .with_context(|| {
            format!(
                "persisting live-signals ledger entry for symbol '{}'",
                input.symbol
            )
        })?;

    let summary = json!({
        "command": "auto-quant-consume-live-signals",
        "symbol": input.symbol,
        "stream_key": outcome.stream_key,
        "redis_url_sanitised": sanitise_redis_url(input.redis_url),
        "envelopes_applied": outcome.envelopes_applied,
        "envelopes_dropped": outcome.envelopes_dropped,
        "iterations": outcome.iterations,
        "started_at": outcome.started_at,
        "ended_at": outcome.ended_at,
        "cursor_start_id": outcome.cursor_start_id,
        "cursor_end_id": outcome.cursor_end_id,
        "ledger_artifact_id": persisted_artifact.artifact_id,
        "ledger_status": persisted_artifact.status,
    });
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

/// What the live-signals ledger writer returns to the caller.
#[derive(Debug, Clone)]
struct PersistedLiveSignalsArtifact {
    artifact_id: String,
    status: &'static str,
}

fn persist_live_signals_session(
    state_dir: &str,
    symbol: &str,
    outcome: &ConsumeLiveSignalsOutcome,
) -> Result<PersistedLiveSignalsArtifact> {
    let timestamp = Utc::now();
    let artifact_id = format!(
        "auto_quant_live_signals_{}_{}",
        symbol,
        timestamp.format("%Y%m%dT%H%M%S%.9fZ")
    );

    let status = if outcome.envelopes_applied > 0 {
        "applied"
    } else {
        "no_op"
    };

    let jsonl_path = super::live::persistence::jsonl_path(std::path::Path::new(state_dir), symbol)
        .to_string_lossy()
        .into_owned();

    let review_reason = format!(
        "consumed {} envelope(s), dropped {}, iter {}, cursor {} -> {} on stream {}",
        outcome.envelopes_applied,
        outcome.envelopes_dropped,
        outcome.iterations,
        outcome.cursor_start_id,
        outcome.cursor_end_id,
        outcome.stream_key,
    );

    crate::state::append_artifact_ledger_entry(
        state_dir,
        symbol,
        crate::state::ArtifactLedgerEntry {
            entry_id: format!("ledger:{}", artifact_id),
            artifact_kind: ARTIFACT_KIND_LIVE_SIGNALS.to_string(),
            artifact_id: artifact_id.clone(),
            version: 1,
            generated_at: timestamp,
            symbol: symbol.to_string(),
            source_phase: "auto_quant_live_signals".to_string(),
            source_run_id: None,
            path: jsonl_path,
            status: status.to_string(),
            promote_candidate: false,
            actionable: false,
            decision_hint: format!("ingested {} envelope(s)", outcome.envelopes_applied),
            review_reason,
            review_rule_version: LIVE_SIGNALS_RULE_VERSION.to_string(),
            quality_score: outcome.envelopes_applied.min(i32::MAX as u32) as i32,
            ..Default::default()
        },
    )?;

    Ok(PersistedLiveSignalsArtifact {
        artifact_id,
        status,
    })
}

/// Operator-facing input for `auto-quant-ingest-real-trades`.
#[derive(Debug, Clone)]
pub struct AutoQuantIngestRealTradesInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
    pub trades_path: &'a str,
    pub source: &'a str,
    pub dry_run: bool,
    pub force: bool,
}

/// Ingest a JSONL artifact of realised trade outcomes produced by
/// `auto_quant_export_real_trades.py`. Each row turns into a
/// `FeedbackRecord` consumed by
/// `apply_feedback_to_trade_outcome_network`. The same JSONL cannot
/// be applied twice without `--force`; the guard keys on a
/// content-hash recorded in the ledger.
pub fn auto_quant_ingest_real_trades_command(
    input: AutoQuantIngestRealTradesInput<'_>,
) -> Result<()> {
    let outcome = ingest_real_trades(IngestRealTradesInput {
        symbol: input.symbol,
        state_dir: input.state_dir,
        trades_path: input.trades_path,
        source: input.source,
        dry_run: input.dry_run,
        force: input.force,
    })
    .with_context(|| {
        format!(
            "ingesting real trades for symbol '{}' from '{}'",
            input.symbol, input.trades_path
        )
    })?;

    print_real_trades_summary(&input, &outcome)?;
    Ok(())
}

fn print_real_trades_summary(
    input: &AutoQuantIngestRealTradesInput<'_>,
    outcome: &IngestRealTradesOutcome,
) -> Result<()> {
    let summary = json!({
        "command": "auto-quant-ingest-real-trades",
        "symbol": input.symbol,
        "trades_path": input.trades_path,
        "source": input.source,
        "dry_run": input.dry_run,
        "force": input.force,
        "ledger_artifact_id": outcome.artifact_id,
        "ledger_status": outcome.status,
        "trades_total": outcome.trades_total,
        "trades_applied": outcome.trades_applied,
        "trades_invalid": outcome.trades_invalid,
        "feedback_records_inserted": outcome.feedback_records_inserted,
        "content_hash": outcome.content_hash,
        "previous_artifact_id": outcome.previous_artifact_id,
    });
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

/// Strip the password (if any) and trailing query-string from a Redis
/// URL so we never persist credentials in the ledger or stdout
/// summary. Returns `<scheme>://<host>[:<port>]`.
fn sanitise_redis_url(raw: &str) -> String {
    // redis://[:password@]host:port[/db]
    let scheme_end = raw.find("://").map(|i| i + 3).unwrap_or(0);
    let scheme = &raw[..scheme_end];
    let rest = &raw[scheme_end..];

    let after_creds = match rest.rfind('@') {
        Some(idx) => &rest[idx + 1..],
        None => rest,
    };
    // Drop anything after the first '/' or '?'.
    let host_port = after_creds.split(['/', '?']).next().unwrap_or(after_creds);
    format!("{scheme}{host_port}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::auto_quant::handoff::{
        build_factor_research_handoff_payload, BuildFactorResearchHandoffPayloadInput,
    };
    use crate::application::auto_quant::results::{
        StrategyLibraryEntry, StrategyLibraryManifest, StrategyLibraryMetadata,
        StrategyLibraryValidationMetrics,
    };
    use crate::application::auto_quant::types::AutoQuantDependencyStatus;

    fn write_manifest_to(temp: &std::path::Path, manifest: &StrategyLibraryManifest) -> String {
        let path = temp.join("strategy_library.json");
        std::fs::write(&path, serde_json::to_string_pretty(manifest).unwrap()).unwrap();
        path.to_string_lossy().into_owned()
    }

    fn ok_strategy(name: &str, trade_count: u32, win_rate_pct: f64) -> StrategyLibraryEntry {
        StrategyLibraryEntry {
            name: name.to_string(),
            file_path: format!("user_data/strategies_ibkr/{name}.py"),
            metadata: StrategyLibraryMetadata {
                strategy: name.to_string(),
                mutation_id: format!("mut-{name}"),
                ..Default::default()
            },
            status: "ok".to_string(),
            validation_metrics: Some(StrategyLibraryValidationMetrics {
                trade_count,
                win_rate_pct,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn healthy_dependency_status(managed_dir: String) -> AutoQuantDependencyStatus {
        AutoQuantDependencyStatus {
            repo_url: "repo".to_string(),
            managed_dir,
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
    fn auto_quant_handoff_human_output_is_short_text_not_json_dump() {
        let temp = tempfile::tempdir().unwrap();
        let payload =
            build_factor_research_handoff_payload(BuildFactorResearchHandoffPayloadInput {
                symbol: "DEMO",
                data: "examples/demo/demo-15m.json",
                objective: "expansion_manipulation",
                provider_profile_selector: None,
                paired_data: None,
                auxiliary_evidence_path: None,
                mutation_spec_path: None,
                strategy_material_root: None,
                state_dir: temp.path().to_str().unwrap(),
                dependency_status: healthy_dependency_status(
                    temp.path()
                        .join(".deps/auto-quant")
                        .to_string_lossy()
                        .into_owned(),
                ),
            });

        let output = build_auto_quant_handoff_output_payload(&payload);
        let human = output["human_output"].as_str().unwrap();

        assert!(human.contains("Auto-Quant handoff"));
        assert!(human.contains("Run:"));
        assert!(!human.trim_start().starts_with('{'));
        assert_eq!(
            output["recommended_next_step"]["action_type"],
            "run_command"
        );
    }

    #[test]
    fn auto_quant_handoff_output_keeps_provider_profile_on_workflow_status_followup() {
        let temp = tempfile::tempdir().unwrap();
        let payload =
            build_factor_research_handoff_payload(BuildFactorResearchHandoffPayloadInput {
                symbol: "DEMO",
                data: "examples/demo/demo-15m.json",
                objective: "expansion_manipulation",
                provider_profile_selector: Some("thrill3r-nq-closed-loop-v1"),
                paired_data: None,
                auxiliary_evidence_path: None,
                mutation_spec_path: None,
                strategy_material_root: None,
                state_dir: temp.path().to_str().unwrap(),
                dependency_status: healthy_dependency_status(
                    temp.path()
                        .join(".deps/auto-quant")
                        .to_string_lossy()
                        .into_owned(),
                ),
            });

        let output = build_auto_quant_handoff_output_payload(&payload);
        let expected_state_dir = temp.path().to_string_lossy().to_string();
        assert_eq!(
            output["workflow_status_command"].as_str(),
            Some(format!(
                "ict-engine workflow-status --symbol DEMO --state-dir {} --profile thrill3r-nq-closed-loop-v1 --human",
                expected_state_dir
            ))
            .as_deref()
        );
    }

    #[test]
    fn auto_quant_readiness_human_output_is_short_text_not_json_dump() {
        let temp = tempfile::tempdir().unwrap();
        let readiness = auto_quant_readiness(temp.path().to_str().unwrap()).unwrap();

        let human = render_auto_quant_readiness_human_output(&readiness);

        assert!(human.contains("Auto-Quant status | missing_dependency"));
        assert!(human.contains("Run: ict-engine auto-quant-bootstrap"));
        assert!(!human.trim_start().starts_with('{'));
    }

    #[test]
    fn auto_quant_readiness_compact_surface_keeps_summary_line() {
        let temp = tempfile::tempdir().unwrap();
        let readiness = auto_quant_readiness(temp.path().to_str().unwrap()).unwrap();

        let compact = build_auto_quant_readiness_compact_surface(&readiness);

        assert!(compact
            .summary_line
            .contains("auto_quant_status missing_dependency"));
        assert_eq!(compact.status, "missing_dependency");
        assert!(!compact.dependency_healthy);
    }

    #[test]
    fn auto_quant_prepare_resolves_relative_workspace_paths_before_chdir() {
        let current_dir = std::env::current_dir().unwrap();
        let resolved = absolute_path("state/.deps/auto-quant/prepare.py").unwrap();

        assert!(resolved.is_absolute());
        assert_eq!(
            resolved,
            current_dir.join("state/.deps/auto-quant/prepare.py")
        );
    }

    #[test]
    fn auto_quant_prepare_keeps_absolute_workspace_paths_unchanged() {
        let temp = tempfile::tempdir().unwrap();
        let script = temp.path().join("prepare.py");
        let resolved = absolute_path(script.to_str().unwrap()).unwrap();

        assert_eq!(resolved, script);
    }

    #[test]
    fn import_then_prior_init_dry_run_does_not_mutate_network_state() {
        let temp = tempfile::tempdir().unwrap();
        let state_dir = temp.path().to_str().unwrap();
        let manifest = StrategyLibraryManifest {
            manifest_version: "1.0".to_string(),
            strategies: vec![ok_strategy("S1", 100, 65.0)],
            ..Default::default()
        };
        let path = write_manifest_to(temp.path(), &manifest);

        auto_quant_results_import_command("NQ", state_dir, &path, None).unwrap();
        assert!(state_exists(state_dir, "NQ", STRATEGY_LIBRARY_FILE));
        assert!(!state_exists(state_dir, "NQ", BBN_STATE_FILE));

        auto_quant_prior_init_command(AutoQuantPriorInitCommandInput {
            symbol: "NQ",
            state_dir,
            library_path: None,
            strategy_filter: None,
            temper: Some(0.5),
            prior_strength: Some(4.0),
            parent_config: None,
            dry_run: true,
            force: false,
        })
        .unwrap();
        assert!(!state_exists(state_dir, "NQ", BBN_STATE_FILE));
    }

    #[test]
    fn prior_init_persists_network_when_not_dry_run() {
        let temp = tempfile::tempdir().unwrap();
        let state_dir = temp.path().to_str().unwrap();
        let manifest = StrategyLibraryManifest {
            manifest_version: "1.0".to_string(),
            strategies: vec![ok_strategy("S1", 100, 75.0)],
            ..Default::default()
        };
        let path = write_manifest_to(temp.path(), &manifest);

        auto_quant_results_import_command("NQ", state_dir, &path, None).unwrap();
        auto_quant_prior_init_command(AutoQuantPriorInitCommandInput {
            symbol: "NQ",
            state_dir,
            library_path: None,
            strategy_filter: None,
            temper: Some(0.5),
            prior_strength: Some(4.0),
            parent_config: None,
            dry_run: false,
            force: false,
        })
        .unwrap();
        assert!(state_exists(state_dir, "NQ", BBN_STATE_FILE));
    }

    #[test]
    fn prior_init_errors_without_library_when_no_state_present() {
        let temp = tempfile::tempdir().unwrap();
        let state_dir = temp.path().to_str().unwrap();
        let err = auto_quant_prior_init_command(AutoQuantPriorInitCommandInput {
            symbol: "NQ",
            state_dir,
            library_path: None,
            strategy_filter: None,
            temper: None,
            prior_strength: None,
            parent_config: None,
            dry_run: true,
            force: false,
        })
        .unwrap_err();
        assert!(err.to_string().contains("auto-quant-results-import"));
    }

    #[test]
    fn second_apply_against_same_library_is_blocked_without_force() {
        let temp = tempfile::tempdir().unwrap();
        let state_dir = temp.path().to_str().unwrap();
        let manifest = StrategyLibraryManifest {
            manifest_version: "1.0".to_string(),
            strategies: vec![ok_strategy("S1", 100, 75.0)],
            ..Default::default()
        };
        let path = write_manifest_to(temp.path(), &manifest);
        auto_quant_results_import_command("NQ", state_dir, &path, None).unwrap();

        // First apply succeeds and writes the BBN.
        auto_quant_prior_init_command(AutoQuantPriorInitCommandInput {
            symbol: "NQ",
            state_dir,
            library_path: None,
            strategy_filter: None,
            temper: Some(0.5),
            prior_strength: Some(4.0),
            parent_config: None,
            dry_run: false,
            force: false,
        })
        .unwrap();
        assert!(state_exists(state_dir, "NQ", BBN_STATE_FILE));

        // Second apply against the same library is refused.
        let err = auto_quant_prior_init_command(AutoQuantPriorInitCommandInput {
            symbol: "NQ",
            state_dir,
            library_path: None,
            strategy_filter: None,
            temper: Some(0.5),
            prior_strength: Some(4.0),
            parent_config: None,
            dry_run: false,
            force: false,
        })
        .unwrap_err();
        assert!(
            err.to_string().contains("already been applied"),
            "unexpected error: {}",
            err
        );

        // Dry-run is always allowed even after an apply.
        auto_quant_prior_init_command(AutoQuantPriorInitCommandInput {
            symbol: "NQ",
            state_dir,
            library_path: None,
            strategy_filter: None,
            temper: Some(0.5),
            prior_strength: Some(4.0),
            parent_config: None,
            dry_run: true,
            force: false,
        })
        .unwrap();

        // --force overrides the guard.
        auto_quant_prior_init_command(AutoQuantPriorInitCommandInput {
            symbol: "NQ",
            state_dir,
            library_path: None,
            strategy_filter: None,
            temper: Some(0.5),
            prior_strength: Some(4.0),
            parent_config: None,
            dry_run: false,
            force: true,
        })
        .unwrap();
    }

    #[test]
    fn second_apply_against_different_library_is_blocked_without_force() {
        let temp = tempfile::tempdir().unwrap();
        let state_dir = temp.path().to_str().unwrap();
        let manifest = StrategyLibraryManifest {
            manifest_version: "1.0".to_string(),
            strategies: vec![ok_strategy("S1", 100, 75.0)],
            ..Default::default()
        };
        let v1_path = write_manifest_to(temp.path(), &manifest);
        // Import v1 + apply v1 + import v2 (auto-supersedes v1).
        auto_quant_results_import_command("NQ", state_dir, &v1_path, None).unwrap();
        auto_quant_prior_init_command(AutoQuantPriorInitCommandInput {
            symbol: "NQ",
            state_dir,
            library_path: None,
            strategy_filter: None,
            temper: Some(0.5),
            prior_strength: Some(4.0),
            parent_config: None,
            dry_run: false,
            force: false,
        })
        .unwrap();
        // Re-export the same content as v2 by re-importing the same path.
        auto_quant_results_import_command("NQ", state_dir, &v1_path, None).unwrap();

        // Apply v2 (the latest ready_for_prior_init) → must bail with the
        // cross-library message. Without the guard the v2 pseudo-counts
        // would stack on top of v1's still-live CPT mutation.
        let err = auto_quant_prior_init_command(AutoQuantPriorInitCommandInput {
            symbol: "NQ",
            state_dir,
            library_path: None,
            strategy_filter: None,
            temper: Some(0.5),
            prior_strength: Some(4.0),
            parent_config: None,
            dry_run: false,
            force: false,
        })
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("BBN already carries an Auto-Quant prior init"),
            "unexpected error: {err}"
        );

        // --dry-run is still allowed (read-only review path).
        auto_quant_prior_init_command(AutoQuantPriorInitCommandInput {
            symbol: "NQ",
            state_dir,
            library_path: None,
            strategy_filter: None,
            temper: Some(0.5),
            prior_strength: Some(4.0),
            parent_config: None,
            dry_run: true,
            force: false,
        })
        .unwrap();

        // --force lets the operator deliberately stack.
        auto_quant_prior_init_command(AutoQuantPriorInitCommandInput {
            symbol: "NQ",
            state_dir,
            library_path: None,
            strategy_filter: None,
            temper: Some(0.5),
            prior_strength: Some(4.0),
            parent_config: None,
            dry_run: false,
            force: true,
        })
        .unwrap();
    }

    #[test]
    fn import_with_log_runs_cross_check() {
        let temp = tempfile::tempdir().unwrap();
        let state_dir = temp.path().to_str().unwrap();
        let manifest = StrategyLibraryManifest {
            manifest_version: "1.0".to_string(),
            strategies: vec![ok_strategy("GhostStrat", 100, 70.0)],
            ..Default::default()
        };
        let manifest_path = write_manifest_to(temp.path(), &manifest);
        // Empty log → manifest_only contains the GhostStrat entry. We
        // assert by re-loading the persisted summary's downstream
        // ledger state: no need to capture stdout here. The cross-check
        // is run in-process via the public command, which must not
        // bail despite the drift.
        let log_path = temp.path().join("empty_run.log");
        std::fs::write(&log_path, "preamble line only, no --- blocks\n").unwrap();
        auto_quant_results_import_command(
            "NQ",
            state_dir,
            &manifest_path,
            Some(log_path.to_str().unwrap()),
        )
        .unwrap();
        // Library was still imported despite the cross-check drift.
        assert!(state_exists(state_dir, "NQ", STRATEGY_LIBRARY_FILE));
    }

    // -------------------------------------------------------------------
    // auto-quant-consume-live-signals tests (Phase 2)

    use super::super::live::wire::{
        LiveFactorContribution, LiveFactorSignalEnvelope, SCHEMA_VERSION,
    };
    use super::super::live::{StreamEntry, StreamSource};
    use std::collections::VecDeque;

    /// Test stream source that returns queued batches then empty.
    #[derive(Default)]
    struct FakeSource {
        batches: VecDeque<Vec<StreamEntry>>,
    }

    impl StreamSource for FakeSource {
        fn xread_block(
            &mut self,
            _stream_key: &str,
            _last_id: &str,
            _block_ms: u64,
        ) -> Result<Vec<StreamEntry>> {
            Ok(self.batches.pop_front().unwrap_or_default())
        }
    }

    fn make_live_envelope_json(run_id: &str) -> String {
        let env = LiveFactorSignalEnvelope {
            schema_version: SCHEMA_VERSION.into(),
            symbol: "NQ".into(),
            timestamp_ms: 1_745_678_901_234,
            auto_quant_run_id: run_id.into(),
            strategy_name: "Strat".into(),
            strategy_mutation_id: "mut".into(),
            bar_close_ts_ms: 1_745_678_900_000,
            contributions: vec![LiveFactorContribution {
                factor_name: "f1".into(),
                category: "c".into(),
                direction: "Bull".into(),
                value: 0.1,
                confidence: 0.5,
                weighted_score: 0.05,
                uncertainty_contribution: 0.02,
                explanation: "".into(),
            }],
        };
        env.to_json().unwrap()
    }

    #[test]
    fn sanitise_redis_url_strips_password() {
        let raw = "redis://:secret@localhost:6379/0";
        assert_eq!(super::sanitise_redis_url(raw), "redis://localhost:6379");
    }

    #[test]
    fn sanitise_redis_url_strips_query_string() {
        let raw = "redis://localhost:6379?ssl=true";
        assert_eq!(super::sanitise_redis_url(raw), "redis://localhost:6379");
    }

    #[test]
    fn sanitise_redis_url_handles_url_without_creds() {
        let raw = "redis://example.com:6379";
        assert_eq!(super::sanitise_redis_url(raw), "redis://example.com:6379");
    }

    #[test]
    fn sanitise_redis_url_handles_userinfo_with_user() {
        let raw = "redis://default:hidden@host:6380/2";
        assert_eq!(super::sanitise_redis_url(raw), "redis://host:6380");
    }

    #[test]
    fn live_signals_no_op_ledger_when_no_envelopes() {
        let temp = tempfile::tempdir().unwrap();
        let state_dir = temp.path().to_str().unwrap();
        let mut src = FakeSource::default();
        src.batches.push_back(vec![]); // BLOCK timeout, zero entries.

        auto_quant_consume_live_signals_with_source(
            AutoQuantConsumeLiveSignalsInput {
                symbol: "NQ",
                state_dir,
                redis_url: "redis://localhost:6379",
                max_iterations: Some(1),
                block_ms: 0,
                initial_id: "$",
            },
            &mut src,
        )
        .unwrap();

        let ledger: Vec<crate::state::ArtifactLedgerEntry> = crate::state::load_state_or_default(
            state_dir,
            "NQ",
            crate::state::ARTIFACT_LEDGER_FILE,
        )
        .unwrap();
        let entry = ledger
            .iter()
            .find(|e| e.artifact_kind == ARTIFACT_KIND_LIVE_SIGNALS)
            .expect("live-signals ledger entry");
        assert_eq!(entry.status, "no_op");
        assert_eq!(entry.review_rule_version, LIVE_SIGNALS_RULE_VERSION);
        assert_eq!(entry.quality_score, 0);
    }

    #[test]
    fn live_signals_applied_ledger_when_envelope_consumed() {
        let temp = tempfile::tempdir().unwrap();
        let state_dir = temp.path().to_str().unwrap();
        let mut src = FakeSource::default();
        src.batches.push_back(vec![StreamEntry {
            id: "1745678901234-0".into(),
            payload: make_live_envelope_json("run-1"),
        }]);

        auto_quant_consume_live_signals_with_source(
            AutoQuantConsumeLiveSignalsInput {
                symbol: "NQ",
                state_dir,
                redis_url: "redis://localhost:6379",
                max_iterations: Some(1),
                block_ms: 0,
                initial_id: "$",
            },
            &mut src,
        )
        .unwrap();

        let ledger: Vec<crate::state::ArtifactLedgerEntry> = crate::state::load_state_or_default(
            state_dir,
            "NQ",
            crate::state::ARTIFACT_LEDGER_FILE,
        )
        .unwrap();
        let entry = ledger
            .iter()
            .find(|e| e.artifact_kind == ARTIFACT_KIND_LIVE_SIGNALS)
            .expect("live-signals ledger entry");
        assert_eq!(entry.status, "applied");
        assert_eq!(entry.quality_score, 1);
        assert!(entry
            .path
            .ends_with("auto_quant_live_factor_contributions.jsonl"));
        assert!(entry.review_reason.contains("auto_quant:factor_signals:nq"));
        // JSONL + cursor were written by the underlying consumer.
        assert!(super::super::live::persistence::jsonl_path(temp.path(), "NQ").exists());
        assert!(super::super::live::persistence::cursor_path(temp.path(), "NQ").exists());
    }

    #[test]
    fn live_signals_invalid_envelope_drops_and_records_no_op() {
        let temp = tempfile::tempdir().unwrap();
        let state_dir = temp.path().to_str().unwrap();
        let mut src = FakeSource::default();
        src.batches.push_back(vec![StreamEntry {
            id: "1-0".into(),
            payload: r#"{"schema_version":"9.9","symbol":"NQ","timestamp_ms":0,"auto_quant_run_id":"x","strategy_name":"y","bar_close_ts_ms":0,"contributions":[{"factor_name":"f","category":"c","direction":"Bull","value":0.0,"confidence":0.0,"weighted_score":0.0,"uncertainty_contribution":0.0}]}"#.into(),
        }]);

        auto_quant_consume_live_signals_with_source(
            AutoQuantConsumeLiveSignalsInput {
                symbol: "NQ",
                state_dir,
                redis_url: "redis://localhost:6379",
                max_iterations: Some(1),
                block_ms: 0,
                initial_id: "$",
            },
            &mut src,
        )
        .unwrap();

        let ledger: Vec<crate::state::ArtifactLedgerEntry> = crate::state::load_state_or_default(
            state_dir,
            "NQ",
            crate::state::ARTIFACT_LEDGER_FILE,
        )
        .unwrap();
        let entry = ledger
            .iter()
            .find(|e| e.artifact_kind == ARTIFACT_KIND_LIVE_SIGNALS)
            .expect("live-signals ledger entry");
        assert_eq!(entry.status, "no_op");
        assert!(entry.review_reason.contains("dropped 1"));
        // No JSONL, no cursor (envelope was rejected before either write).
        assert!(!super::super::live::persistence::jsonl_path(temp.path(), "NQ").exists());
        assert!(!super::super::live::persistence::cursor_path(temp.path(), "NQ").exists());
    }
}
