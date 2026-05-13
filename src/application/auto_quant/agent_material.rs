use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
use csv::Writer;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use crate::data::loader::load_candles;
use crate::state::{
    append_artifact_ledger_entry, artifact_state_path, load_artifact_ledger, save_state,
    ArtifactLedgerEntry,
};

use super::pda_unit_batch::AutoQuantConsumerEvidenceProfile;

pub const AUTO_QUANT_AGENT_MATERIAL_BATCH_RULE_VERSION: &str = "auto-quant-agent-material-batch-v1";
pub const AUTO_QUANT_AGENT_MATERIAL_DISPATCH_RULE_VERSION: &str =
    "auto-quant-agent-material-dispatch-v1";
pub const AUTO_QUANT_AGENT_MATERIAL_RANK_RULE_VERSION: &str = "auto-quant-agent-material-rank-v1";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMaterialPackage {
    pub package_id: String,
    pub generated_at: Option<DateTime<Utc>>,
    pub title: String,
    pub symbol: String,
    pub timeframe: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timerange: Option<String>,
    pub direction: String,
    pub data_path: String,
    pub strategy_source_path: String,
    #[serde(default)]
    pub strategy_class_name: Option<String>,
    pub strategy_brief: String,
    #[serde(default)]
    pub evaluation_priority: Vec<String>,
    #[serde(default)]
    pub consumer_evidence_profile: AutoQuantConsumerEvidenceProfile,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMaterialBatchJob {
    pub job_id: String,
    pub isolated_state_dir: String,
    pub material_path: String,
    pub package: AgentMaterialPackage,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMaterialDispatchGroup {
    pub group_index: usize,
    pub job_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMaterialBatchArtifact {
    pub artifact_id: String,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub shared_workspace_root: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_repo_url: Option<String>,
    pub max_parallel: usize,
    pub jobs: Vec<AgentMaterialBatchJob>,
    pub dispatch_groups: Vec<AgentMaterialDispatchGroup>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMaterialRankArtifact {
    pub artifact_id: String,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub source_dispatch_artifact_id: String,
    pub ranking: Vec<AgentMaterialRankRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMaterialRankRow {
    pub unit_label: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regime_profit_branch_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_regime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_regime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_sub_regime_or_profit_factor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profit_factor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_provenance: Option<String>,
    pub win_rate_pct: Option<f64>,
    pub sharpe: Option<f64>,
    pub total_profit_pct: Option<f64>,
    pub trade_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMaterialDispatchArtifact {
    pub artifact_id: String,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub source_batch_artifact_id: String,
    pub selected_group_indices: Vec<usize>,
    pub groups: Vec<AgentMaterialDispatchGroupResult>,
    pub totals: AgentMaterialDispatchTotals,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMaterialDispatchGroupResult {
    pub group_index: usize,
    pub job_results: Vec<AgentMaterialDispatchJobResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMaterialDispatchTotals {
    pub total_jobs: usize,
    pub completed_jobs: usize,
    pub blocked_jobs: usize,
    pub failed_jobs: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMaterialDispatchJobResult {
    pub job_id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regime_profit_branch_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_regime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_regime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_sub_regime_or_profit_factor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profit_factor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_provenance: Option<String>,
    pub status: String,
    pub reason: String,
    pub workspace_root: String,
    pub stdout_log_path: String,
    pub stderr_log_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aggregate_metrics: Option<AgentMaterialAggregateMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMaterialAggregateMetrics {
    pub sharpe: f64,
    pub sortino: f64,
    pub calmar: f64,
    pub total_profit_pct: f64,
    pub max_drawdown_pct: f64,
    pub trade_count: usize,
    pub win_rate_pct: f64,
    pub profit_factor: f64,
}

pub fn load_agent_material_package<P: AsRef<Path>>(path: P) -> Result<AgentMaterialPackage> {
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("reading agent material '{}'", path.as_ref().display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("parsing agent material '{}'", path.as_ref().display()))
}

pub fn persist_agent_material_batch(
    symbol: &str,
    state_dir: &str,
    shared_workspace_root: &str,
    source_repo_url: Option<&str>,
    max_parallel: usize,
    material_paths: &[String],
) -> Result<AgentMaterialBatchArtifact> {
    let generated_at = Utc::now();
    let shared_workspace_root = absolute_existing_or_current_dir_path(shared_workspace_root)
        .with_context(|| {
            format!("resolving shared Auto-Quant workspace '{shared_workspace_root}'")
        })?
        .to_string_lossy()
        .to_string();
    let mut jobs = Vec::new();
    for path in material_paths {
        let material_path = absolute_existing_or_current_dir_path(path)
            .with_context(|| format!("resolving agent material path '{path}'"))?;
        let mut package = load_agent_material_package(&material_path)?;
        package.data_path = absolute_existing_or_current_dir_path(&package.data_path)
            .with_context(|| format!("resolving agent material data path '{}'", package.data_path))?
            .to_string_lossy()
            .to_string();
        package.strategy_source_path =
            absolute_existing_or_current_dir_path(&package.strategy_source_path)
                .with_context(|| {
                    format!(
                        "resolving agent material strategy path '{}'",
                        package.strategy_source_path
                    )
                })?
                .to_string_lossy()
                .to_string();
        let label = if package.title.trim().is_empty() {
            format!(
                "{}:{}:{}",
                package.symbol, package.timeframe, package.direction
            )
        } else {
            package.title.clone()
        };
        let job_id = label.replace([':', '/', ' '], "_");
        jobs.push(AgentMaterialBatchJob {
            isolated_state_dir: PathBuf::from(state_dir)
                .join("agent_material_units")
                .join(&job_id)
                .to_string_lossy()
                .to_string(),
            material_path: material_path.to_string_lossy().to_string(),
            package,
            job_id,
        });
    }
    let dispatch_groups = jobs
        .chunks(max_parallel.max(1))
        .enumerate()
        .map(|(index, chunk)| AgentMaterialDispatchGroup {
            group_index: index,
            job_ids: chunk.iter().map(|job| job.job_id.clone()).collect(),
        })
        .collect::<Vec<_>>();
    let artifact = AgentMaterialBatchArtifact {
        artifact_id: format!(
            "auto-quant-agent-material-batch:{}:{}",
            symbol,
            generated_at.format("%Y%m%dT%H%M%S%.3fZ")
        ),
        generated_at,
        symbol: symbol.to_string(),
        shared_workspace_root,
        source_repo_url: source_repo_url.map(str::to_string),
        max_parallel,
        jobs,
        dispatch_groups,
        notes: vec![
            "public_protocol_generic_only".to_string(),
            "ontology_must_live_inside_agent_materials_not_cli".to_string(),
        ],
    };
    let filename = format!(
        "auto_quant_agent_material_batch.{}.json",
        generated_at.format("%Y%m%dT%H%M%S%.3fZ")
    );
    save_state(state_dir, symbol, &filename, &artifact)?;
    append_artifact_ledger_entry(
        state_dir,
        symbol,
        ArtifactLedgerEntry {
            entry_id: format!("ledger:{}", artifact.artifact_id),
            artifact_kind: "auto_quant_agent_material_batch".to_string(),
            artifact_id: artifact.artifact_id.clone(),
            version: 1,
            generated_at,
            symbol: symbol.to_string(),
            source_phase: "auto_quant_agent_material_batch".to_string(),
            source_run_id: None,
            path: artifact_state_path(state_dir, symbol, &filename),
            status: "batch_ready_for_dispatch".to_string(),
            promote_candidate: false,
            actionable: true,
            decision_hint: "agent_material_dispatch".to_string(),
            review_reason: format!(
                "jobs={} max_parallel={}",
                artifact.jobs.len(),
                artifact.max_parallel
            ),
            review_rule_version: AUTO_QUANT_AGENT_MATERIAL_BATCH_RULE_VERSION.to_string(),
            top_factor_name: None,
            top_factor_action: Some("dispatch".to_string()),
            family_scores: BTreeMap::new(),
            supersedes_artifact_id: None,
            quality_score: artifact.jobs.len().min(i32::MAX as usize) as i32,
            consumed_by_update_run_id: None,
            consumed_at: None,
            consumed_outcome: None,
            regraded_at: None,
            consumption_regrade_status: None,
            consumption_regrade_reason: None,
        },
    )?;
    Ok(artifact)
}

pub fn load_latest_agent_material_batch(
    state_dir: &str,
    symbol: &str,
) -> Result<Option<AgentMaterialBatchArtifact>> {
    let ledger = load_artifact_ledger(state_dir, symbol)?;
    let target = ledger
        .iter()
        .rev()
        .find(|entry| entry.artifact_kind == "auto_quant_agent_material_batch");
    let Some(target) = target else {
        return Ok(None);
    };
    let content = std::fs::read_to_string(&target.path)
        .with_context(|| format!("reading agent material batch '{}'", target.path))?;
    let artifact = serde_json::from_str(&content)
        .with_context(|| format!("parsing agent material batch '{}'", target.path))?;
    Ok(Some(artifact))
}

fn load_agent_material_batch_by_artifact_id(
    state_dir: &str,
    symbol: &str,
    artifact_id: &str,
) -> Result<Option<AgentMaterialBatchArtifact>> {
    let ledger = load_artifact_ledger(state_dir, symbol)?;
    let target = ledger.iter().rev().find(|entry| {
        entry.artifact_kind == "auto_quant_agent_material_batch" && entry.artifact_id == artifact_id
    });
    let Some(target) = target else {
        return Ok(None);
    };
    let content = std::fs::read_to_string(&target.path)
        .with_context(|| format!("reading agent material batch '{}'", target.path))?;
    let artifact = serde_json::from_str(&content)
        .with_context(|| format!("parsing agent material batch '{}'", target.path))?;
    Ok(Some(artifact))
}

pub fn dispatch_agent_material_batch(
    state_dir: &str,
    symbol: &str,
    group_indices_csv: Option<&str>,
) -> Result<AgentMaterialDispatchArtifact> {
    let batch = load_latest_agent_material_batch(state_dir, symbol)?.ok_or_else(|| {
        anyhow!(
            "no auto_quant_agent_material_batch artifact found for {}",
            symbol
        )
    })?;
    let selected_group_indices =
        parse_group_indices(group_indices_csv, batch.dispatch_groups.len())?;
    let mut groups = Vec::new();
    for group_index in &selected_group_indices {
        let group = batch
            .dispatch_groups
            .iter()
            .find(|item| item.group_index == *group_index)
            .ok_or_else(|| anyhow!("unknown dispatch group index {}", group_index))?;
        let jobs = group
            .job_ids
            .iter()
            .filter_map(|job_id| batch.jobs.iter().find(|job| &job.job_id == job_id))
            .cloned()
            .collect::<Vec<_>>();
        let shared_workspace_root = batch.shared_workspace_root.clone();
        let handles = jobs
            .into_iter()
            .map(|job| {
                let shared_workspace_root = shared_workspace_root.clone();
                let source_repo_url = batch.source_repo_url.clone();
                thread::spawn(move || {
                    dispatch_one_material_job(
                        job,
                        &shared_workspace_root,
                        source_repo_url.as_deref(),
                    )
                })
            })
            .collect::<Vec<_>>();
        let mut job_results = Vec::new();
        for handle in handles {
            job_results.push(
                handle
                    .join()
                    .map_err(|_| anyhow!("agent material dispatch thread panicked"))??,
            );
        }
        groups.push(AgentMaterialDispatchGroupResult {
            group_index: *group_index,
            job_results,
        });
    }
    let totals = summarize_dispatch_totals(&groups);
    let artifact = AgentMaterialDispatchArtifact {
        artifact_id: format!(
            "auto-quant-agent-material-dispatch:{}:{}",
            symbol,
            Utc::now().format("%Y%m%dT%H%M%S%.3fZ")
        ),
        generated_at: Utc::now(),
        symbol: symbol.to_string(),
        source_batch_artifact_id: batch.artifact_id,
        selected_group_indices,
        groups,
        totals,
    };
    persist_dispatch_artifact(state_dir, &artifact)?;
    Ok(artifact)
}

pub fn rank_agent_material_dispatch(
    state_dir: &str,
    symbol: &str,
) -> Result<AgentMaterialRankArtifact> {
    let ledger = load_artifact_ledger(state_dir, symbol)?;
    let target = ledger
        .iter()
        .rev()
        .find(|entry| entry.artifact_kind == "auto_quant_agent_material_dispatch")
        .ok_or_else(|| {
            anyhow!(
                "no auto_quant_agent_material_dispatch artifact found for {}",
                symbol
            )
        })?;
    let content = fs::read_to_string(&target.path)
        .with_context(|| format!("reading agent material dispatch '{}'", target.path))?;
    let dispatch: AgentMaterialDispatchArtifact = serde_json::from_str(&content)
        .with_context(|| format!("parsing agent material dispatch '{}'", target.path))?;
    let source_batch = load_agent_material_batch_by_artifact_id(
        state_dir,
        symbol,
        &dispatch.source_batch_artifact_id,
    )?
    .or_else(|| {
        load_latest_agent_material_batch(state_dir, symbol)
            .ok()
            .flatten()
    });
    let source_jobs_by_id = source_batch
        .as_ref()
        .map(|batch| {
            batch
                .jobs
                .iter()
                .map(|job| (job.job_id.clone(), job))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();

    let mut ranking = dispatch
        .groups
        .iter()
        .flat_map(|group| group.job_results.iter())
        .map(|row| {
            let source_job = source_jobs_by_id.get(&row.job_id).copied();
            let fallback_branch = source_job.map(|job| branch_path_fields(&job.package));
            AgentMaterialRankRow {
                unit_label: row.title.clone(),
                status: row.status.clone(),
                branch_path: row.branch_path.clone().or_else(|| {
                    fallback_branch
                        .as_ref()
                        .and_then(|branch| branch.regime_profit_branch_path.clone())
                }),
                package_id: row
                    .package_id
                    .clone()
                    .or_else(|| source_job.map(|job| job.package.package_id.clone())),
                regime_profit_branch_path: row.regime_profit_branch_path.clone().or_else(|| {
                    fallback_branch
                        .as_ref()
                        .and_then(|branch| branch.regime_profit_branch_path.clone())
                }),
                main_regime: row.main_regime.clone().or_else(|| {
                    fallback_branch
                        .as_ref()
                        .and_then(|branch| branch.main_regime.clone())
                }),
                sub_regime: row.sub_regime.clone().or_else(|| {
                    fallback_branch
                        .as_ref()
                        .and_then(|branch| branch.sub_regime.clone())
                }),
                sub_sub_regime_or_profit_factor: row
                    .sub_sub_regime_or_profit_factor
                    .clone()
                    .or_else(|| {
                        fallback_branch
                            .as_ref()
                            .and_then(|branch| branch.sub_sub_regime_or_profit_factor.clone())
                    }),
                profit_factor: row.profit_factor.clone().or_else(|| {
                    fallback_branch
                        .as_ref()
                        .and_then(|branch| branch.profit_factor.clone())
                }),
                provider_provenance: row.provider_provenance.clone().or_else(|| {
                    fallback_branch
                        .as_ref()
                        .and_then(|branch| branch.provider_provenance.clone())
                }),
                win_rate_pct: row.aggregate_metrics.as_ref().map(|m| m.win_rate_pct),
                sharpe: row.aggregate_metrics.as_ref().map(|m| m.sharpe),
                total_profit_pct: row.aggregate_metrics.as_ref().map(|m| m.total_profit_pct),
                trade_count: row.aggregate_metrics.as_ref().map(|m| m.trade_count),
            }
        })
        .collect::<Vec<_>>();
    ranking.sort_by(|left, right| {
        let lk = (
            left.status != "completed",
            -(left.win_rate_pct.unwrap_or(-1.0) * 10_000.0) as i64,
            -(left.sharpe.unwrap_or(-999.0) * 10_000.0) as i64,
            -(left.total_profit_pct.unwrap_or(-999.0) * 10_000.0) as i64,
        );
        let rk = (
            right.status != "completed",
            -(right.win_rate_pct.unwrap_or(-1.0) * 10_000.0) as i64,
            -(right.sharpe.unwrap_or(-999.0) * 10_000.0) as i64,
            -(right.total_profit_pct.unwrap_or(-999.0) * 10_000.0) as i64,
        );
        lk.cmp(&rk)
    });
    let artifact = AgentMaterialRankArtifact {
        artifact_id: format!(
            "auto-quant-agent-material-rank:{}:{}",
            symbol,
            Utc::now().format("%Y%m%dT%H%M%S%.3fZ")
        ),
        generated_at: Utc::now(),
        symbol: symbol.to_string(),
        source_dispatch_artifact_id: dispatch.artifact_id,
        ranking,
    };
    persist_rank_artifact(state_dir, &artifact)?;
    Ok(artifact)
}

fn parse_group_indices(raw: Option<&str>, group_count: usize) -> Result<Vec<usize>> {
    if let Some(raw) = raw {
        let mut out = Vec::new();
        for item in raw.split(',') {
            let trimmed = item.trim();
            if trimmed.is_empty() {
                continue;
            }
            let idx = trimmed
                .parse::<usize>()
                .with_context(|| format!("invalid group index '{}'", trimmed))?;
            if idx >= group_count {
                bail!("group index {} out of range 0..{}", idx, group_count);
            }
            if !out.contains(&idx) {
                out.push(idx);
            }
        }
        if out.is_empty() {
            bail!("at least one valid group index is required");
        }
        Ok(out)
    } else {
        Ok((0..group_count).collect())
    }
}

fn dispatch_one_material_job(
    job: AgentMaterialBatchJob,
    shared_workspace_root: &str,
    source_repo_url: Option<&str>,
) -> Result<AgentMaterialDispatchJobResult> {
    let Some(reason) = blocking_reason_for_profile(&job.package.consumer_evidence_profile) else {
        return run_dispatch_material_job(job, shared_workspace_root, source_repo_url);
    };
    let branch = branch_path_fields(&job.package);
    Ok(AgentMaterialDispatchJobResult {
        job_id: job.job_id,
        title: job.package.title,
        branch_path: branch.regime_profit_branch_path.clone(),
        package_id: Some(job.package.package_id),
        regime_profit_branch_path: branch.regime_profit_branch_path,
        main_regime: branch.main_regime,
        sub_regime: branch.sub_regime,
        sub_sub_regime_or_profit_factor: branch.sub_sub_regime_or_profit_factor,
        profit_factor: branch.profit_factor,
        provider_provenance: branch.provider_provenance,
        status: "blocked".to_string(),
        reason,
        workspace_root: String::new(),
        stdout_log_path: String::new(),
        stderr_log_path: String::new(),
        aggregate_metrics: None,
    })
}

fn blocking_reason_for_profile(profile: &AutoQuantConsumerEvidenceProfile) -> Option<String> {
    let blocked = profile.required_surfaces.iter().any(|surface| {
        matches!(
            surface.as_str(),
            "greeks" | "open_interest" | "implied_volatility" | "options_chain" | "cross_market"
        )
    });
    blocked.then(|| {
        "dispatch_blocked_missing_external_evidence_provider: requested surfaces require external provider/runtime inputs not yet wired into generic dispatch"
            .to_string()
    })
}

#[derive(Debug, Clone, Default)]
struct BranchPathFields {
    regime_profit_branch_path: Option<String>,
    main_regime: Option<String>,
    sub_regime: Option<String>,
    sub_sub_regime_or_profit_factor: Option<String>,
    profit_factor: Option<String>,
    provider_provenance: Option<String>,
}

fn branch_path_fields(package: &AgentMaterialPackage) -> BranchPathFields {
    let provider_provenance = package
        .notes
        .iter()
        .filter_map(|note| note.trim().strip_prefix("source_provider="))
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(str::to_string);
    let Some(branch_path) = package
        .notes
        .iter()
        .filter_map(|note| note.trim().strip_prefix("branch_path="))
        .map(str::trim)
        .find(|value| !value.is_empty())
    else {
        return BranchPathFields {
            provider_provenance,
            ..Default::default()
        };
    };
    let segments = branch_path
        .split(" -> ")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    BranchPathFields {
        regime_profit_branch_path: Some(branch_path.to_string()),
        main_regime: segments.first().map(|value| (*value).to_string()),
        sub_regime: segments.get(1).map(|value| (*value).to_string()),
        sub_sub_regime_or_profit_factor: segments.get(2).map(|value| (*value).to_string()),
        profit_factor: segments.get(3).map(|value| (*value).to_string()),
        provider_provenance,
    }
}

fn run_dispatch_material_job(
    job: AgentMaterialBatchJob,
    shared_workspace_root: &str,
    source_repo_url: Option<&str>,
) -> Result<AgentMaterialDispatchJobResult> {
    let package_id = job.package.package_id.clone();
    let branch = branch_path_fields(&job.package);
    let workspace_root = PathBuf::from(&job.isolated_state_dir).join("aq_workspace");
    let runtime_python = resolve_runtime_python(shared_workspace_root, source_repo_url)?;
    materialize_material_workspace(
        &workspace_root,
        runtime_python.as_deref(),
        shared_workspace_root,
        &job.package,
    )?;
    let stdout_log_path = workspace_root.join("run_tomac.stdout.log");
    let stderr_log_path = workspace_root.join("run_tomac.stderr.log");
    let output = run_workspace_python_script(
        runtime_python.as_deref(),
        &workspace_root,
        "run_tomac.py",
        &[],
    )?;
    fs::write(&stdout_log_path, &output.stdout)?;
    fs::write(&stderr_log_path, &output.stderr)?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        return Ok(AgentMaterialDispatchJobResult {
            job_id: job.job_id,
            title: job.package.title,
            branch_path: branch.regime_profit_branch_path.clone(),
            package_id: Some(package_id),
            regime_profit_branch_path: branch.regime_profit_branch_path,
            main_regime: branch.main_regime,
            sub_regime: branch.sub_regime,
            sub_sub_regime_or_profit_factor: branch.sub_sub_regime_or_profit_factor,
            profit_factor: branch.profit_factor,
            provider_provenance: branch.provider_provenance,
            status: "failed".to_string(),
            reason: format!(
                "run_tomac_exit_nonzero: {}",
                stderr.lines().next().unwrap_or("unknown error")
            ),
            workspace_root: workspace_root.to_string_lossy().to_string(),
            stdout_log_path: stdout_log_path.to_string_lossy().to_string(),
            stderr_log_path: stderr_log_path.to_string_lossy().to_string(),
            aggregate_metrics: parse_run_tomac_aggregate_metrics(&stdout),
        });
    }
    Ok(AgentMaterialDispatchJobResult {
        job_id: job.job_id,
        title: job.package.title,
        branch_path: branch.regime_profit_branch_path.clone(),
        package_id: Some(package_id),
        regime_profit_branch_path: branch.regime_profit_branch_path,
        main_regime: branch.main_regime,
        sub_regime: branch.sub_regime,
        sub_sub_regime_or_profit_factor: branch.sub_sub_regime_or_profit_factor,
        profit_factor: branch.profit_factor,
        provider_provenance: branch.provider_provenance,
        status: "completed".to_string(),
        reason: "external_auto_quant_run_completed".to_string(),
        workspace_root: workspace_root.to_string_lossy().to_string(),
        stdout_log_path: stdout_log_path.to_string_lossy().to_string(),
        stderr_log_path: stderr_log_path.to_string_lossy().to_string(),
        aggregate_metrics: parse_run_tomac_aggregate_metrics(&stdout),
    })
}

fn resolve_runtime_python(
    shared_workspace_root: &str,
    source_repo_url: Option<&str>,
) -> Result<Option<PathBuf>> {
    let shared_workspace_root = absolute_existing_or_current_dir_path(shared_workspace_root)
        .with_context(|| {
            format!("resolving shared Auto-Quant workspace '{shared_workspace_root}'")
        })?;
    let shared_python = shared_workspace_root.join(".venv/bin/python");
    if shared_python.exists() {
        return Ok(Some(shared_python));
    }
    if let Some(repo_url) = source_repo_url {
        if repo_url.starts_with('/') {
            let candidate = PathBuf::from(repo_url).join(".venv/bin/python");
            if candidate.exists() {
                return Ok(Some(candidate));
            }
        }
    }
    let output = Command::new("uv")
        .arg("sync")
        .arg("--frozen")
        .current_dir(&shared_workspace_root)
        .output()
        .with_context(|| {
            format!(
                "running uv sync --frozen in '{}'",
                shared_workspace_root.display()
            )
        })?;
    if !output.status.success() {
        bail!(
            "failed to provision shared Auto-Quant runtime in '{}': {}",
            shared_workspace_root.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    if shared_python.exists() {
        Ok(Some(shared_python))
    } else {
        Ok(None)
    }
}

fn absolute_existing_or_current_dir_path(path: &str) -> Result<PathBuf> {
    let path = PathBuf::from(path);
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(fs::canonicalize(&absolute).unwrap_or(absolute))
}

fn materialize_material_workspace(
    workspace_root: &Path,
    runtime_python: Option<&Path>,
    shared_workspace_root: &str,
    package: &AgentMaterialPackage,
) -> Result<()> {
    fs::create_dir_all(workspace_root.join("user_data/strategies_external"))?;
    fs::create_dir_all(workspace_root.join("user_data/data"))?;
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let shared_root = PathBuf::from(shared_workspace_root);
    for filename in ["pyproject.toml", "uv.lock"] {
        fs::copy(shared_root.join(filename), workspace_root.join(filename))?;
    }
    fs::copy(
        repo_root.join("scripts/auto_quant_external/run_tomac.py"),
        workspace_root.join("run_tomac.py"),
    )?;
    fs::copy(
        repo_root.join("scripts/auto_quant_external/prepare_external.py"),
        workspace_root.join("prepare_external.py"),
    )?;
    fs::copy(
        repo_root.join("scripts/auto_quant_external/config.tomac.json"),
        workspace_root.join("config.tomac.json"),
    )?;
    write_material_config(&workspace_root.join("config.tomac.json"), package)?;
    write_material_candles_csv(&package.data_path, &workspace_root.join("unit.csv"))?;
    copy_material_strategy(
        Path::new(&package.strategy_source_path),
        &workspace_root
            .join("user_data/strategies_external")
            .join(material_strategy_filename(package)),
    )?;
    let pair = format!("{}/USD", package.symbol);
    let prepare = run_workspace_python_script(
        runtime_python,
        workspace_root,
        "prepare_external.py",
        &[
            "--csv",
            "unit.csv",
            "--pair",
            &pair,
            "--timeframes",
            &package.timeframe,
            "--datadir",
            "user_data/data",
            "--column-map",
            "date:date,open:open,high:high,low:low,close:close,volume:volume",
            "--no-clean",
        ],
    )?;
    if !prepare.status.success() {
        bail!(
            "prepare_external failed for '{}': {}",
            package.title,
            String::from_utf8_lossy(&prepare.stderr)
        );
    }
    if package.direction == "short" {
        materialize_futures_ohlcv_alias(workspace_root, &package.symbol, &package.timeframe)?;
    }
    Ok(())
}

fn write_material_config(path: &Path, package: &AgentMaterialPackage) -> Result<()> {
    let mut config: serde_json::Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    config["timeframe"] = serde_json::Value::String(package.timeframe.clone());
    if let Some(timerange) = effective_material_timerange(package)? {
        config["timerange"] = serde_json::Value::String(timerange.to_string());
    }
    config["exchange"]["pair_whitelist"] = serde_json::json!([format!("{}/USD", package.symbol)]);
    if package.direction == "short" {
        config["trading_mode"] = serde_json::Value::String("futures".to_string());
        config["margin_mode"] = serde_json::Value::String("isolated".to_string());
        config["entry_pricing"]["use_order_book"] = serde_json::Value::Bool(true);
        config["exit_pricing"]["use_order_book"] = serde_json::Value::Bool(true);
        config["exchange"]["_ft_has_params"]["uses_leverage_tiers"] =
            serde_json::Value::Bool(false);
    } else {
        config["trading_mode"] = serde_json::Value::String("spot".to_string());
        config["entry_pricing"]["use_order_book"] = serde_json::Value::Bool(false);
        config["exit_pricing"]["use_order_book"] = serde_json::Value::Bool(false);
        if let Some(root) = config.as_object_mut() {
            root.remove("margin_mode");
        }
        if let Some(exchange) = config["exchange"].as_object_mut() {
            exchange.remove("_ft_has_params");
        }
    }
    fs::write(path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}

fn effective_material_timerange(package: &AgentMaterialPackage) -> Result<Option<String>> {
    if let Some(timerange) = package
        .timerange
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(Some(timerange.to_string()));
    }
    derive_material_timerange_from_data_path(&package.data_path)
}

fn derive_material_timerange_from_data_path(data_path: &str) -> Result<Option<String>> {
    let candles = load_candles(data_path)
        .with_context(|| format!("deriving agent material timerange from '{}'", data_path))?;
    let Some(first) = candles.first() else {
        return Ok(None);
    };
    let (mut min_ts, mut max_ts) = (first.timestamp, first.timestamp);
    for candle in candles.iter().skip(1) {
        min_ts = min_ts.min(candle.timestamp);
        max_ts = max_ts.max(candle.timestamp);
    }
    Ok(Some(format!(
        "{}-{}",
        min_ts.format("%Y%m%d"),
        max_ts.format("%Y%m%d")
    )))
}

fn write_material_candles_csv(input_path: &str, csv_path: &Path) -> Result<()> {
    let candles = load_candles(input_path)?;
    let mut writer = Writer::from_path(csv_path)?;
    writer.write_record(["date", "open", "high", "low", "close", "volume"])?;
    for candle in candles {
        writer.write_record([
            candle.timestamp.to_rfc3339(),
            candle.open.to_string(),
            candle.high.to_string(),
            candle.low.to_string(),
            candle.close.to_string(),
            candle.volume.to_string(),
        ])?;
    }
    writer.flush()?;
    Ok(())
}

fn copy_material_strategy(source: &Path, destination: &Path) -> Result<()> {
    fs::copy(source, destination).with_context(|| {
        format!(
            "copying agent material strategy '{}' to '{}'",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}

fn material_strategy_filename(package: &AgentMaterialPackage) -> String {
    if let Some(class_name) = &package.strategy_class_name {
        format!("{class_name}.py")
    } else {
        Path::new(&package.strategy_source_path)
            .file_name()
            .and_then(|value| value.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| "AgentMaterialStrategy.py".to_string())
    }
}

fn run_workspace_python_script(
    python_override: Option<&Path>,
    workspace_root: &Path,
    script_name: &str,
    args: &[&str],
) -> Result<std::process::Output> {
    let mut command = if let Some(python) = python_override {
        let python = if python.is_absolute() {
            python.to_path_buf()
        } else {
            std::env::current_dir()
                .with_context(|| format!("resolving python runtime '{}'", python.display()))?
                .join(python)
        };
        let mut cmd = Command::new(python);
        cmd.arg(script_name);
        cmd
    } else {
        let mut cmd = Command::new("uv");
        cmd.arg("run").arg(script_name);
        cmd
    };
    command.args(args);
    command.current_dir(workspace_root);
    command
        .output()
        .with_context(|| format!("running {}", script_name))
}

fn materialize_futures_ohlcv_alias(
    workspace_root: &Path,
    symbol: &str,
    timeframe: &str,
) -> Result<()> {
    let pair_filename = format!("{}_USD", symbol);
    let source = workspace_root
        .join("user_data/data")
        .join(format!("{pair_filename}-{timeframe}.feather"));
    let target_dir = workspace_root.join("user_data/data/futures");
    fs::create_dir_all(&target_dir)?;
    let target = target_dir.join(format!("{pair_filename}-{timeframe}-futures.feather"));
    fs::copy(&source, &target)?;
    Ok(())
}

fn parse_run_tomac_aggregate_metrics(stdout: &str) -> Option<AgentMaterialAggregateMetrics> {
    let mut metrics = AgentMaterialAggregateMetrics::default();
    let mut seen_any = false;
    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("sharpe:") {
            metrics.sharpe = value.trim().parse().ok()?;
            seen_any = true;
        } else if let Some(value) = trimmed.strip_prefix("sortino:") {
            metrics.sortino = value.trim().parse().ok()?;
        } else if let Some(value) = trimmed.strip_prefix("calmar:") {
            metrics.calmar = value.trim().parse().ok()?;
        } else if let Some(value) = trimmed.strip_prefix("total_profit_pct:") {
            metrics.total_profit_pct = value.trim().parse().ok()?;
        } else if let Some(value) = trimmed.strip_prefix("max_drawdown_pct:") {
            metrics.max_drawdown_pct = value.trim().parse().ok()?;
        } else if let Some(value) = trimmed.strip_prefix("trade_count:") {
            metrics.trade_count = value.trim().parse().ok()?;
        } else if let Some(value) = trimmed.strip_prefix("win_rate_pct:") {
            metrics.win_rate_pct = value.trim().parse().ok()?;
        } else if let Some(value) = trimmed.strip_prefix("profit_factor:") {
            metrics.profit_factor = value.trim().parse().ok()?;
        }
    }
    seen_any.then_some(metrics)
}

fn summarize_dispatch_totals(
    groups: &[AgentMaterialDispatchGroupResult],
) -> AgentMaterialDispatchTotals {
    let mut totals = AgentMaterialDispatchTotals::default();
    for group in groups {
        for result in &group.job_results {
            totals.total_jobs += 1;
            match result.status.as_str() {
                "completed" => totals.completed_jobs += 1,
                "blocked" => totals.blocked_jobs += 1,
                _ => totals.failed_jobs += 1,
            }
        }
    }
    totals
}

fn persist_dispatch_artifact(
    state_dir: &str,
    artifact: &AgentMaterialDispatchArtifact,
) -> Result<()> {
    let filename = format!(
        "auto_quant_agent_material_dispatch.{}.json",
        artifact.generated_at.format("%Y%m%dT%H%M%S%.3fZ")
    );
    save_state(state_dir, &artifact.symbol, &filename, artifact)?;
    append_artifact_ledger_entry(
        state_dir,
        &artifact.symbol,
        ArtifactLedgerEntry {
            entry_id: format!("ledger:{}", artifact.artifact_id),
            artifact_kind: "auto_quant_agent_material_dispatch".to_string(),
            artifact_id: artifact.artifact_id.clone(),
            version: 1,
            generated_at: artifact.generated_at,
            symbol: artifact.symbol.clone(),
            source_phase: "auto_quant_agent_material_dispatch".to_string(),
            source_run_id: Some(artifact.source_batch_artifact_id.clone()),
            path: artifact_state_path(state_dir, &artifact.symbol, &filename),
            status: "dispatch_completed".to_string(),
            promote_candidate: false,
            actionable: true,
            decision_hint: "agent_material_results_ready".to_string(),
            review_reason: format!(
                "completed={} blocked={} failed={}",
                artifact.totals.completed_jobs,
                artifact.totals.blocked_jobs,
                artifact.totals.failed_jobs
            ),
            review_rule_version: AUTO_QUANT_AGENT_MATERIAL_DISPATCH_RULE_VERSION.to_string(),
            top_factor_name: None,
            top_factor_action: Some("review".to_string()),
            family_scores: BTreeMap::new(),
            supersedes_artifact_id: None,
            quality_score: artifact.totals.completed_jobs.min(i32::MAX as usize) as i32,
            consumed_by_update_run_id: None,
            consumed_at: None,
            consumed_outcome: None,
            regraded_at: None,
            consumption_regrade_status: None,
            consumption_regrade_reason: None,
        },
    )?;
    Ok(())
}

fn persist_rank_artifact(state_dir: &str, artifact: &AgentMaterialRankArtifact) -> Result<()> {
    let filename = format!(
        "auto_quant_agent_material_rank.{}.json",
        artifact.generated_at.format("%Y%m%dT%H%M%S%.3fZ")
    );
    save_state(state_dir, &artifact.symbol, &filename, artifact)?;
    append_artifact_ledger_entry(
        state_dir,
        &artifact.symbol,
        ArtifactLedgerEntry {
            entry_id: format!("ledger:{}", artifact.artifact_id),
            artifact_kind: "auto_quant_agent_material_rank".to_string(),
            artifact_id: artifact.artifact_id.clone(),
            version: 1,
            generated_at: artifact.generated_at,
            symbol: artifact.symbol.clone(),
            source_phase: "auto_quant_agent_material_rank".to_string(),
            source_run_id: Some(artifact.source_dispatch_artifact_id.clone()),
            path: artifact_state_path(state_dir, &artifact.symbol, &filename),
            status: "rank_ready".to_string(),
            promote_candidate: false,
            actionable: true,
            decision_hint: "agent_material_top_candidates_available".to_string(),
            review_reason: format!("rows={}", artifact.ranking.len()),
            review_rule_version: AUTO_QUANT_AGENT_MATERIAL_RANK_RULE_VERSION.to_string(),
            top_factor_name: artifact.ranking.first().map(|row| row.unit_label.clone()),
            top_factor_action: Some("rank".to_string()),
            family_scores: BTreeMap::new(),
            supersedes_artifact_id: None,
            quality_score: artifact.ranking.len().min(i32::MAX as usize) as i32,
            consumed_by_update_run_id: None,
            consumed_at: None,
            consumed_outcome: None,
            regraded_at: None,
            consumption_regrade_status: None,
            consumption_regrade_reason: None,
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_package(strategy_path: &str) -> AgentMaterialPackage {
        AgentMaterialPackage {
            package_id: "mat-1".to_string(),
            title: "NQ 1h continuation material".to_string(),
            symbol: "NQ".to_string(),
            timeframe: "1h".to_string(),
            direction: "long".to_string(),
            data_path: "/tmp/nq-1h.json".to_string(),
            strategy_source_path: strategy_path.to_string(),
            strategy_class_name: Some("NQ1hLongOrderBlock".to_string()),
            strategy_brief: "Agent-produced continuation material".to_string(),
            evaluation_priority: vec![
                "win_rate".to_string(),
                "sharpe".to_string(),
                "return".to_string(),
            ],
            consumer_evidence_profile: AutoQuantConsumerEvidenceProfile::default(),
            notes: vec!["note".to_string()],
            ..Default::default()
        }
    }

    #[test]
    fn load_agent_material_package_round_trips_json() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("material.json");
        let package = sample_package("/tmp/strategy.py");
        fs::write(&path, serde_json::to_string(&package).unwrap()).unwrap();

        let loaded = load_agent_material_package(&path).unwrap();
        assert_eq!(loaded.title, package.title);
        assert_eq!(loaded.strategy_source_path, "/tmp/strategy.py");
    }

    #[test]
    fn persist_agent_material_batch_writes_artifact_and_groups() {
        let dir = TempDir::new().unwrap();
        let material_a = dir.path().join("a.json");
        let material_b = dir.path().join("b.json");
        let strategy = dir.path().join("strategy.py");
        fs::write(&strategy, "class Dummy: pass\n").unwrap();
        fs::write(
            &material_a,
            serde_json::to_string(&sample_package(strategy.to_str().unwrap())).unwrap(),
        )
        .unwrap();
        let mut b = sample_package(strategy.to_str().unwrap());
        b.package_id = "mat-2".to_string();
        b.title = "Another material".to_string();
        fs::write(&material_b, serde_json::to_string(&b).unwrap()).unwrap();

        let artifact = persist_agent_material_batch(
            "NQ",
            dir.path().to_str().unwrap(),
            "/tmp/shared-aq",
            Some("/tmp/Auto-Quant"),
            1,
            &[
                material_a.to_string_lossy().to_string(),
                material_b.to_string_lossy().to_string(),
            ],
        )
        .unwrap();

        assert_eq!(artifact.jobs.len(), 2);
        assert_eq!(artifact.dispatch_groups.len(), 2);
        assert_eq!(artifact.source_repo_url.as_deref(), Some("/tmp/Auto-Quant"));
    }

    #[test]
    fn persist_agent_material_batch_normalizes_workspace_and_material_paths() {
        let dir = TempDir::new_in(".").unwrap();
        let dir_path = fs::canonicalize(dir.path()).unwrap();
        let shared = dir_path.join("shared-aq");
        let data = dir_path.join("candles.csv");
        let strategy = dir_path.join("strategy.py");
        let material = dir_path.join("material.json");
        fs::create_dir_all(&shared).unwrap();
        fs::write(&data, "timestamp,open,high,low,close,volume\n").unwrap();
        fs::write(&strategy, "class Dummy: pass\n").unwrap();
        let mut package = sample_package(strategy.to_str().unwrap());
        package.data_path = data
            .strip_prefix(std::env::current_dir().unwrap())
            .unwrap()
            .to_string_lossy()
            .to_string();
        package.strategy_source_path = strategy
            .strip_prefix(std::env::current_dir().unwrap())
            .unwrap()
            .to_string_lossy()
            .to_string();
        fs::write(&material, serde_json::to_string(&package).unwrap()).unwrap();
        let relative_shared = shared
            .strip_prefix(std::env::current_dir().unwrap())
            .unwrap()
            .to_string_lossy()
            .to_string();
        let relative_material = material
            .strip_prefix(std::env::current_dir().unwrap())
            .unwrap()
            .to_string_lossy()
            .to_string();

        let artifact = persist_agent_material_batch(
            "NQ",
            dir_path.to_str().unwrap(),
            &relative_shared,
            None,
            1,
            &[relative_material],
        )
        .unwrap();

        assert!(Path::new(&artifact.shared_workspace_root).is_absolute());
        assert!(Path::new(&artifact.jobs[0].material_path).is_absolute());
        assert!(Path::new(&artifact.jobs[0].package.data_path).is_absolute());
        assert!(Path::new(&artifact.jobs[0].package.strategy_source_path).is_absolute());
    }

    #[test]
    fn write_material_config_applies_package_timerange_when_present() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.tomac.json");
        fs::write(
            &config_path,
            serde_json::json!({
                "timeframe": "1h",
                "timerange": "20230101-20251231",
                "trading_mode": "spot",
                "entry_pricing": {"use_order_book": false},
                "exit_pricing": {"use_order_book": false},
                "exchange": {
                    "pair_whitelist": ["NQ/USD"],
                    "_ft_has_params": {"uses_leverage_tiers": false}
                }
            })
            .to_string(),
        )
        .unwrap();
        let mut package = sample_package("/tmp/strategy.py");
        package.timerange = Some("20220405-20220614".to_string());

        write_material_config(&config_path, &package).unwrap();

        let config: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(config_path).unwrap()).unwrap();
        assert_eq!(config["timerange"], "20220405-20220614");
        assert_eq!(config["exchange"]["pair_whitelist"][0], "NQ/USD");
    }

    #[test]
    fn write_material_config_derives_timerange_from_package_data_when_missing() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.tomac.json");
        let data_path = dir.path().join("material.csv");
        fs::write(
            &config_path,
            serde_json::json!({
                "timeframe": "1h",
                "timerange": "20230101-20251231",
                "trading_mode": "spot",
                "entry_pricing": {"use_order_book": false},
                "exit_pricing": {"use_order_book": false},
                "exchange": {
                    "pair_whitelist": ["NQ/USD"],
                    "_ft_has_params": {"uses_leverage_tiers": false}
                }
            })
            .to_string(),
        )
        .unwrap();
        fs::write(
            &data_path,
            "timestamp,open,high,low,close,volume\n\
             2026-01-01T00:00:00Z,1,2,1,2,10\n\
             2026-05-12T09:00:00Z,2,3,2,3,12\n",
        )
        .unwrap();
        let mut package = sample_package("/tmp/strategy.py");
        package.data_path = data_path.to_string_lossy().to_string();

        write_material_config(&config_path, &package).unwrap();

        let config: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(config_path).unwrap()).unwrap();
        assert_eq!(config["timerange"], "20260101-20260512");
        assert_eq!(config["exchange"]["pair_whitelist"][0], "NQ/USD");
    }

    #[test]
    fn run_workspace_python_script_resolves_relative_runtime_before_changing_cwd() {
        let dir = TempDir::new_in(".").unwrap();
        let dir_path = fs::canonicalize(dir.path()).unwrap();
        let runtime_dir = dir_path.join("runtime");
        let workspace = dir_path.join("workspace");
        fs::create_dir_all(&runtime_dir).unwrap();
        fs::create_dir_all(&workspace).unwrap();
        let python = runtime_dir.join("python");
        fs::write(
            &python,
            "#!/bin/sh\nprintf 'runtime=%s script=%s cwd=%s\\n' \"$0\" \"$1\" \"$(pwd)\"\n",
        )
        .unwrap();
        let mut perms = fs::metadata(&python).unwrap().permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(0o755);
            fs::set_permissions(&python, perms).unwrap();
        }
        let relative_python = python
            .strip_prefix(std::env::current_dir().unwrap())
            .unwrap();

        let output = run_workspace_python_script(
            Some(relative_python),
            &workspace,
            "prepare_external.py",
            &[],
        )
        .unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains(&format!("runtime={}", python.display())));
        assert!(stdout.contains("script=prepare_external.py"));
        assert!(stdout.contains(&format!("cwd={}", workspace.display())));
    }

    #[test]
    fn rank_agent_material_dispatch_orders_by_priority() {
        let dir = TempDir::new().unwrap();
        let dispatch = AgentMaterialDispatchArtifact {
            artifact_id: "dispatch-1".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_batch_artifact_id: "batch-1".to_string(),
            selected_group_indices: vec![0],
            groups: vec![AgentMaterialDispatchGroupResult {
                group_index: 0,
                job_results: vec![
                    AgentMaterialDispatchJobResult {
                        title: "B".to_string(),
                        status: "completed".to_string(),
                        aggregate_metrics: Some(AgentMaterialAggregateMetrics {
                            win_rate_pct: 41.0,
                            sharpe: 0.2,
                            total_profit_pct: 5.0,
                            trade_count: 10,
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    AgentMaterialDispatchJobResult {
                        title: "A".to_string(),
                        status: "completed".to_string(),
                        aggregate_metrics: Some(AgentMaterialAggregateMetrics {
                            win_rate_pct: 48.0,
                            sharpe: 0.7,
                            total_profit_pct: 12.0,
                            trade_count: 20,
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                ],
            }],
            totals: AgentMaterialDispatchTotals {
                total_jobs: 2,
                completed_jobs: 2,
                ..Default::default()
            },
        };
        let filename = "auto_quant_agent_material_dispatch.test.json";
        save_state(dir.path(), "NQ", filename, &dispatch).unwrap();
        append_artifact_ledger_entry(
            dir.path(),
            "NQ",
            ArtifactLedgerEntry {
                entry_id: "ledger:dispatch-1".to_string(),
                artifact_kind: "auto_quant_agent_material_dispatch".to_string(),
                artifact_id: "dispatch-1".to_string(),
                version: 1,
                generated_at: dispatch.generated_at,
                symbol: "NQ".to_string(),
                source_phase: "auto_quant_agent_material_dispatch".to_string(),
                source_run_id: None,
                path: artifact_state_path(dir.path(), "NQ", filename),
                status: "dispatch_completed".to_string(),
                promote_candidate: false,
                actionable: true,
                decision_hint: String::new(),
                review_reason: String::new(),
                review_rule_version: String::new(),
                top_factor_name: None,
                top_factor_action: None,
                family_scores: BTreeMap::new(),
                supersedes_artifact_id: None,
                quality_score: 0,
                consumed_by_update_run_id: None,
                consumed_at: None,
                consumed_outcome: None,
                regraded_at: None,
                consumption_regrade_status: None,
                consumption_regrade_reason: None,
            },
        )
        .unwrap();

        let rank = rank_agent_material_dispatch(dir.path().to_str().unwrap(), "NQ").unwrap();
        assert_eq!(rank.ranking[0].unit_label, "A");
        assert_eq!(rank.ranking[1].unit_label, "B");
    }

    #[test]
    fn rank_agent_material_dispatch_preserves_branch_path_and_provider_provenance() {
        let dir = TempDir::new().unwrap();
        let branch_path =
            "TrendExpansion -> MomentumPersistence -> macd_zero_line_reclaim -> macd_zero_line_reclaim_long_v1";
        let dispatch = AgentMaterialDispatchArtifact {
            artifact_id: "dispatch-branch".to_string(),
            generated_at: Utc::now(),
            symbol: "MACDMOM".to_string(),
            source_batch_artifact_id: "batch-branch".to_string(),
            selected_group_indices: vec![0],
            groups: vec![AgentMaterialDispatchGroupResult {
                group_index: 0,
                job_results: vec![AgentMaterialDispatchJobResult {
                    job_id: "macd-zero-line-yf".to_string(),
                    title: "MACD zero-line reclaim - yfinance/YF SPY 1h".to_string(),
                    status: "completed".to_string(),
                    reason: "external_auto_quant_run_completed".to_string(),
                    branch_path: Some(branch_path.to_string()),
                    main_regime: Some("TrendExpansion".to_string()),
                    sub_regime: Some("MomentumPersistence".to_string()),
                    sub_sub_regime_or_profit_factor: Some("macd_zero_line_reclaim".to_string()),
                    profit_factor: Some("macd_zero_line_reclaim_long_v1".to_string()),
                    provider_provenance: Some("yfinance/YF SPY 1h".to_string()),
                    aggregate_metrics: Some(AgentMaterialAggregateMetrics {
                        win_rate_pct: 50.0,
                        sharpe: 0.1778,
                        total_profit_pct: 4.66,
                        trade_count: 10,
                        ..Default::default()
                    }),
                    ..Default::default()
                }],
            }],
            totals: AgentMaterialDispatchTotals {
                total_jobs: 1,
                completed_jobs: 1,
                ..Default::default()
            },
        };
        let filename = "auto_quant_agent_material_dispatch.branch.json";
        save_state(dir.path(), "MACDMOM", filename, &dispatch).unwrap();
        append_artifact_ledger_entry(
            dir.path(),
            "MACDMOM",
            ArtifactLedgerEntry {
                entry_id: "ledger:dispatch-branch".to_string(),
                artifact_kind: "auto_quant_agent_material_dispatch".to_string(),
                artifact_id: "dispatch-branch".to_string(),
                version: 1,
                generated_at: dispatch.generated_at,
                symbol: "MACDMOM".to_string(),
                source_phase: "auto_quant_agent_material_dispatch".to_string(),
                source_run_id: None,
                path: artifact_state_path(dir.path(), "MACDMOM", filename),
                status: "dispatch_completed".to_string(),
                promote_candidate: false,
                actionable: true,
                decision_hint: String::new(),
                review_reason: String::new(),
                review_rule_version: String::new(),
                top_factor_name: None,
                top_factor_action: None,
                family_scores: BTreeMap::new(),
                supersedes_artifact_id: None,
                quality_score: 0,
                consumed_by_update_run_id: None,
                consumed_at: None,
                consumed_outcome: None,
                regraded_at: None,
                consumption_regrade_status: None,
                consumption_regrade_reason: None,
            },
        )
        .unwrap();

        let rank = rank_agent_material_dispatch(dir.path().to_str().unwrap(), "MACDMOM").unwrap();
        let row = &rank.ranking[0];
        assert_eq!(row.branch_path.as_deref(), Some(branch_path));
        assert_eq!(row.main_regime.as_deref(), Some("TrendExpansion"));
        assert_eq!(row.sub_regime.as_deref(), Some("MomentumPersistence"));
        assert_eq!(
            row.sub_sub_regime_or_profit_factor.as_deref(),
            Some("macd_zero_line_reclaim")
        );
        assert_eq!(
            row.profit_factor.as_deref(),
            Some("macd_zero_line_reclaim_long_v1")
        );
        assert_eq!(
            row.provider_provenance.as_deref(),
            Some("yfinance/YF SPY 1h")
        );
    }

    #[test]
    fn dispatch_agent_material_preserves_branch_path_when_provider_blocked() {
        let dir = TempDir::new().unwrap();
        let branch_path =
            "TrendExpansion -> PullbackContinuation -> fair_value_gap_retest -> fvg_retest_continuation_long_v1";
        let job = AgentMaterialBatchJob {
            job_id: "ob-fvg-yf".to_string(),
            isolated_state_dir: dir.path().join("job").to_string_lossy().to_string(),
            material_path: "ob-fvg-yf.material.json".to_string(),
            package: AgentMaterialPackage {
                package_id: "ob-fvg-yf-v1".to_string(),
                title: "OB/FVG fvg_retest_continuation_long_v1 - yfinance/YF SPY 1h".to_string(),
                symbol: "SPY".to_string(),
                timeframe: "1h".to_string(),
                direction: "long".to_string(),
                consumer_evidence_profile: AutoQuantConsumerEvidenceProfile {
                    required_surfaces: vec!["greeks".to_string()],
                    ..Default::default()
                },
                notes: vec![
                    "source_provider=yfinance/YF SPY 1h".to_string(),
                    format!("branch_path={branch_path}"),
                ],
                ..Default::default()
            },
        };

        let result = dispatch_one_material_job(job, dir.path().to_str().unwrap(), None).unwrap();

        assert_eq!(result.status, "blocked");
        assert_eq!(result.package_id.as_deref(), Some("ob-fvg-yf-v1"));
        assert_eq!(result.branch_path.as_deref(), Some(branch_path));
        assert_eq!(
            result.regime_profit_branch_path.as_deref(),
            Some(branch_path)
        );
        assert_eq!(result.main_regime.as_deref(), Some("TrendExpansion"));
        assert_eq!(result.sub_regime.as_deref(), Some("PullbackContinuation"));
        assert_eq!(
            result.sub_sub_regime_or_profit_factor.as_deref(),
            Some("fair_value_gap_retest")
        );
        assert_eq!(
            result.profit_factor.as_deref(),
            Some("fvg_retest_continuation_long_v1")
        );
        assert_eq!(
            result.provider_provenance.as_deref(),
            Some("yfinance/YF SPY 1h")
        );
    }

    #[test]
    fn rank_agent_material_dispatch_recovers_branch_fields_from_source_batch() {
        let dir = TempDir::new().unwrap();
        let branch_path =
            "TrendExpansion -> MomentumPersistence -> macd_signal_pullback -> macd_signal_pullback_continuation_v1";
        let mut package = sample_package("/tmp/strategy.py");
        package.package_id = "macd-signal-yf".to_string();
        package.title = "MACD signal pullback continuation - yfinance/YF SPY 1h".to_string();
        package.notes = vec![
            "source_provider=yfinance/YF SPY 1h".to_string(),
            format!("branch_path={branch_path}"),
        ];
        let batch = AgentMaterialBatchArtifact {
            artifact_id: "batch-branch-source".to_string(),
            generated_at: Utc::now(),
            symbol: "MACDMOM".to_string(),
            shared_workspace_root: "/tmp/aq".to_string(),
            source_repo_url: None,
            max_parallel: 1,
            jobs: vec![AgentMaterialBatchJob {
                job_id: "macd-signal-yf".to_string(),
                isolated_state_dir: "/tmp/unit".to_string(),
                material_path: "/tmp/material.json".to_string(),
                package,
            }],
            dispatch_groups: vec![],
            notes: vec![],
        };
        let batch_filename = "auto_quant_agent_material_batch.branch-source.json";
        save_state(dir.path(), "MACDMOM", batch_filename, &batch).unwrap();
        append_artifact_ledger_entry(
            dir.path(),
            "MACDMOM",
            ArtifactLedgerEntry {
                entry_id: "ledger:batch-branch-source".to_string(),
                artifact_kind: "auto_quant_agent_material_batch".to_string(),
                artifact_id: "batch-branch-source".to_string(),
                version: 1,
                generated_at: batch.generated_at,
                symbol: "MACDMOM".to_string(),
                source_phase: "auto_quant_agent_material_batch".to_string(),
                source_run_id: None,
                path: artifact_state_path(dir.path(), "MACDMOM", batch_filename),
                status: "batch_ready_for_dispatch".to_string(),
                promote_candidate: false,
                actionable: true,
                decision_hint: String::new(),
                review_reason: String::new(),
                review_rule_version: String::new(),
                top_factor_name: None,
                top_factor_action: None,
                family_scores: BTreeMap::new(),
                supersedes_artifact_id: None,
                quality_score: 0,
                consumed_by_update_run_id: None,
                consumed_at: None,
                consumed_outcome: None,
                regraded_at: None,
                consumption_regrade_status: None,
                consumption_regrade_reason: None,
            },
        )
        .unwrap();
        let dispatch = AgentMaterialDispatchArtifact {
            artifact_id: "dispatch-old-schema".to_string(),
            generated_at: Utc::now(),
            symbol: "MACDMOM".to_string(),
            source_batch_artifact_id: "batch-branch-source".to_string(),
            selected_group_indices: vec![0],
            groups: vec![AgentMaterialDispatchGroupResult {
                group_index: 0,
                job_results: vec![AgentMaterialDispatchJobResult {
                    job_id: "macd-signal-yf".to_string(),
                    title: "MACD signal pullback continuation - yfinance/YF SPY 1h".to_string(),
                    status: "completed".to_string(),
                    reason: "external_auto_quant_run_completed".to_string(),
                    aggregate_metrics: Some(AgentMaterialAggregateMetrics {
                        win_rate_pct: 64.2857,
                        sharpe: 0.1266,
                        total_profit_pct: 2.22,
                        trade_count: 14,
                        ..Default::default()
                    }),
                    ..Default::default()
                }],
            }],
            totals: AgentMaterialDispatchTotals {
                total_jobs: 1,
                completed_jobs: 1,
                ..Default::default()
            },
        };
        let dispatch_filename = "auto_quant_agent_material_dispatch.old-schema.json";
        save_state(dir.path(), "MACDMOM", dispatch_filename, &dispatch).unwrap();
        append_artifact_ledger_entry(
            dir.path(),
            "MACDMOM",
            ArtifactLedgerEntry {
                entry_id: "ledger:dispatch-old-schema".to_string(),
                artifact_kind: "auto_quant_agent_material_dispatch".to_string(),
                artifact_id: "dispatch-old-schema".to_string(),
                version: 1,
                generated_at: dispatch.generated_at,
                symbol: "MACDMOM".to_string(),
                source_phase: "auto_quant_agent_material_dispatch".to_string(),
                source_run_id: None,
                path: artifact_state_path(dir.path(), "MACDMOM", dispatch_filename),
                status: "dispatch_completed".to_string(),
                promote_candidate: false,
                actionable: true,
                decision_hint: String::new(),
                review_reason: String::new(),
                review_rule_version: String::new(),
                top_factor_name: None,
                top_factor_action: None,
                family_scores: BTreeMap::new(),
                supersedes_artifact_id: None,
                quality_score: 0,
                consumed_by_update_run_id: None,
                consumed_at: None,
                consumed_outcome: None,
                regraded_at: None,
                consumption_regrade_status: None,
                consumption_regrade_reason: None,
            },
        )
        .unwrap();

        let rank = rank_agent_material_dispatch(dir.path().to_str().unwrap(), "MACDMOM").unwrap();
        let row = &rank.ranking[0];
        assert_eq!(row.branch_path.as_deref(), Some(branch_path));
        assert_eq!(row.main_regime.as_deref(), Some("TrendExpansion"));
        assert_eq!(
            row.provider_provenance.as_deref(),
            Some("yfinance/YF SPY 1h")
        );
    }
}
