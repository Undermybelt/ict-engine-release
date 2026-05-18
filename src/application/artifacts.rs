use anyhow::{bail, Result};
use serde::Serialize;
use std::collections::BTreeMap;

use crate::config::{compute_hash, env_bool, env_bool_with_source, env_f64, env_f64_with_source};
use crate::state::{
    artifact_state_path, load_artifact_ledger, load_execution_candidate_history,
    load_pending_update_history, load_state_or_default, AnalyzeRunRecord,
    ArtifactConsumedImpactSummary, ArtifactConsumedImpactTrendComparison,
    ArtifactConsumedImpactWindow, ArtifactDecisionSection, ArtifactDecisionSummary,
    ArtifactFactorTrendSummary, ArtifactFamilyTrendSummary, ArtifactHistorySummary,
    ArtifactLedgerEntry, ArtifactLineageSummary, ArtifactRuleBreakEffect,
    ArtifactRuleBreakFactorImpact, ArtifactRuleBreakFamilyImpact, DecisionThresholds,
    ExecutionCandidateArtifact, ExecutionCandidateArtifactDiff, ExecutionCandidateArtifactSummary,
    PendingUpdateArtifact, PendingUpdateArtifactDiff, PendingUpdateArtifactSummary,
    PreBayesEntryQualityBridge, PreBayesEvidenceFilter, WorkflowPhaseSnapshot, ANALYZE_RUNS_FILE,
    EXECUTION_CANDIDATE_FILE, PENDING_UPDATE_ARTIFACT_FILE,
};

#[derive(Debug, Serialize)]
pub struct ArtifactStatusEntryView {
    #[serde(flatten)]
    pub entry: ArtifactLedgerEntry,
    pub path_kind: String,
    pub path_exists: bool,
}

#[derive(Debug, Serialize)]
pub struct ArtifactStatusView {
    symbol: String,
    total_entries: usize,
    entries: Vec<ArtifactStatusEntryView>,
}

#[derive(Debug, Serialize)]
pub struct ArtifactStatusBucketView {
    symbol: String,
    total_entries: usize,
    buckets: BTreeMap<String, Vec<ArtifactStatusEntryView>>,
}

#[derive(Debug, Serialize)]
pub struct ArtifactDiffView {
    pub kind: String,
    pub left_artifact_id: String,
    pub right_artifact_id: String,
    pub changed_fields: Vec<String>,
    pub numeric_evidence: Vec<String>,
    pub embedded_pre_bayes_evidence: Vec<String>,
    pub summary: String,
    pub cross_rule_version_summary: Option<String>,
    pub lineage_artifact_ids: Vec<String>,
    pub lineage_numeric_evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ArtifactLineageEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
}

#[derive(Debug, Serialize)]
pub struct ArtifactLineageView {
    pub symbol: String,
    pub focus_artifact_id: Option<String>,
    pub nodes: Vec<ArtifactLedgerEntry>,
    pub edges: Vec<ArtifactLineageEdge>,
}

#[derive(Debug, Clone, Default)]
pub struct ConsumedAnalyzeContext {
    pub analyze_run_id: Option<String>,
    pub pre_bayes_evidence_filter: Option<PreBayesEvidenceFilter>,
    pub pre_bayes_entry_quality_bridge: Option<PreBayesEntryQualityBridge>,
    pub multi_timeframe_summary: Vec<String>,
    pub canonical_structural_regime_posterior:
        Option<crate::state::CanonicalStructuralRegimePosterior>,
}

pub fn pending_update_artifact_path(state_dir: &str, symbol: &str) -> Option<String> {
    let path = std::path::Path::new(state_dir)
        .join(symbol)
        .join(PENDING_UPDATE_ARTIFACT_FILE);
    path.exists().then(|| path.to_string_lossy().to_string())
}

pub fn source_analyze_run_id_from_artifacts(
    pending: Option<&PendingUpdateArtifact>,
    execution: Option<&ExecutionCandidateArtifact>,
) -> Option<String> {
    pending
        .and_then(|artifact| artifact.source_run_id.clone())
        .or_else(|| execution.and_then(|artifact| artifact.source_run_id.clone()))
}

pub fn consumed_analyze_context_for_update(
    state_dir: impl AsRef<std::path::Path>,
    symbol: &str,
    pending: Option<&PendingUpdateArtifact>,
    execution: Option<&ExecutionCandidateArtifact>,
) -> Result<ConsumedAnalyzeContext> {
    if let Some(pending) = pending {
        if pending.pre_bayes_evidence_filter.is_some()
            || pending.pre_bayes_entry_quality_bridge.is_some()
            || !pending.multi_timeframe_summary.is_empty()
        {
            return Ok(ConsumedAnalyzeContext {
                analyze_run_id: pending.source_run_id.clone(),
                pre_bayes_evidence_filter: pending.pre_bayes_evidence_filter.clone(),
                pre_bayes_entry_quality_bridge: pending.pre_bayes_entry_quality_bridge.clone(),
                multi_timeframe_summary: pending.multi_timeframe_summary.clone(),
                canonical_structural_regime_posterior: None,
            });
        }
    }
    if let Some(execution) = execution {
        if execution.pre_bayes_evidence_filter.is_some()
            || execution.pre_bayes_entry_quality_bridge.is_some()
            || !execution.multi_timeframe_summary.is_empty()
        {
            return Ok(ConsumedAnalyzeContext {
                analyze_run_id: execution.source_run_id.clone(),
                pre_bayes_evidence_filter: execution.pre_bayes_evidence_filter.clone(),
                pre_bayes_entry_quality_bridge: execution.pre_bayes_entry_quality_bridge.clone(),
                multi_timeframe_summary: execution.multi_timeframe_summary.clone(),
                canonical_structural_regime_posterior: None,
            });
        }
    }
    let Some(run_id) = source_analyze_run_id_from_artifacts(pending, execution) else {
        return Ok(ConsumedAnalyzeContext::default());
    };
    let analyze_runs: Vec<AnalyzeRunRecord> =
        load_state_or_default(state_dir, symbol, ANALYZE_RUNS_FILE)?;
    let Some(run) = analyze_runs.iter().rev().find(|run| run.run_id == run_id) else {
        return Ok(ConsumedAnalyzeContext {
            analyze_run_id: Some(run_id),
            ..ConsumedAnalyzeContext::default()
        });
    };
    Ok(ConsumedAnalyzeContext {
        analyze_run_id: Some(run.run_id.clone()),
        pre_bayes_evidence_filter: Some(run.pre_bayes_evidence_filter.clone()),
        pre_bayes_entry_quality_bridge: Some(run.pre_bayes_entry_quality_bridge.clone()),
        multi_timeframe_summary: run.multi_timeframe_summary.clone(),
        canonical_structural_regime_posterior: run.canonical_structural_regime_posterior.clone(),
    })
}

pub fn artifact_review_rules() -> crate::state::ArtifactReviewRules {
    crate::state::ArtifactReviewRules {
        pending_update: crate::state::PendingUpdateReviewRules {
            min_probability_improvement: env_f64(
                "ICT_ENGINE_PENDING_MIN_PROBABILITY_IMPROVEMENT",
                0.03,
            ),
            min_top_factor_score_improvement: env_f64(
                "ICT_ENGINE_PENDING_MIN_TOP_FACTOR_SCORE_IMPROVEMENT",
                0.05,
            ),
            min_avg_family_score_improvement: env_f64(
                "ICT_ENGINE_PENDING_MIN_AVG_FAMILY_SCORE_IMPROVEMENT",
                0.03,
            ),
            require_same_data: env_bool("ICT_ENGINE_PENDING_REQUIRE_SAME_DATA", true),
            require_same_factor_version: env_bool(
                "ICT_ENGINE_PENDING_REQUIRE_SAME_FACTOR_VERSION",
                true,
            ),
            require_same_prompt_version: env_bool(
                "ICT_ENGINE_PENDING_REQUIRE_SAME_PROMPT_VERSION",
                true,
            ),
        },
        execution_candidate: crate::state::ExecutionCandidateReviewRules {
            min_posterior_improvement: env_f64(
                "ICT_ENGINE_EXECUTION_MIN_POSTERIOR_IMPROVEMENT",
                0.03,
            ),
            min_win_probability_improvement: env_f64(
                "ICT_ENGINE_EXECUTION_MIN_WIN_PROBABILITY_IMPROVEMENT",
                0.03,
            ),
            require_same_data: env_bool("ICT_ENGINE_EXECUTION_REQUIRE_SAME_DATA", true),
            require_same_factor_version: env_bool(
                "ICT_ENGINE_EXECUTION_REQUIRE_SAME_FACTOR_VERSION",
                true,
            ),
        },
    }
}

pub fn artifact_review_rule_sources() -> crate::state::ArtifactReviewRuleSources {
    let mut pending_update = BTreeMap::new();
    let (_, source) = env_f64_with_source("ICT_ENGINE_PENDING_MIN_PROBABILITY_IMPROVEMENT", 0.03);
    pending_update.insert("min_probability_improvement".to_string(), source);
    let (_, source) =
        env_f64_with_source("ICT_ENGINE_PENDING_MIN_TOP_FACTOR_SCORE_IMPROVEMENT", 0.05);
    pending_update.insert("min_top_factor_score_improvement".to_string(), source);
    let (_, source) =
        env_f64_with_source("ICT_ENGINE_PENDING_MIN_AVG_FAMILY_SCORE_IMPROVEMENT", 0.03);
    pending_update.insert("min_avg_family_score_improvement".to_string(), source);
    let (_, source) = env_bool_with_source("ICT_ENGINE_PENDING_REQUIRE_SAME_DATA", true);
    pending_update.insert("require_same_data".to_string(), source);
    let (_, source) = env_bool_with_source("ICT_ENGINE_PENDING_REQUIRE_SAME_FACTOR_VERSION", true);
    pending_update.insert("require_same_factor_version".to_string(), source);
    let (_, source) = env_bool_with_source("ICT_ENGINE_PENDING_REQUIRE_SAME_PROMPT_VERSION", true);
    pending_update.insert("require_same_prompt_version".to_string(), source);

    let mut execution_candidate = BTreeMap::new();
    let (_, source) = env_f64_with_source("ICT_ENGINE_EXECUTION_MIN_POSTERIOR_IMPROVEMENT", 0.03);
    execution_candidate.insert("min_posterior_improvement".to_string(), source);
    let (_, source) =
        env_f64_with_source("ICT_ENGINE_EXECUTION_MIN_WIN_PROBABILITY_IMPROVEMENT", 0.03);
    execution_candidate.insert("min_win_probability_improvement".to_string(), source);
    let (_, source) = env_bool_with_source("ICT_ENGINE_EXECUTION_REQUIRE_SAME_DATA", true);
    execution_candidate.insert("require_same_data".to_string(), source);
    let (_, source) =
        env_bool_with_source("ICT_ENGINE_EXECUTION_REQUIRE_SAME_FACTOR_VERSION", true);
    execution_candidate.insert("require_same_factor_version".to_string(), source);

    crate::state::ArtifactReviewRuleSources {
        pending_update,
        execution_candidate,
    }
}

pub fn pending_update_review_rule_version(
    rules: &crate::state::PendingUpdateReviewRules,
) -> String {
    compute_hash(&[
        format!("{:.6}", rules.min_probability_improvement),
        format!("{:.6}", rules.min_top_factor_score_improvement),
        format!("{:.6}", rules.min_avg_family_score_improvement),
        rules.require_same_data.to_string(),
        rules.require_same_factor_version.to_string(),
        rules.require_same_prompt_version.to_string(),
    ])
}

pub fn execution_candidate_review_rule_version(
    rules: &crate::state::ExecutionCandidateReviewRules,
) -> String {
    compute_hash(&[
        format!("{:.6}", rules.min_posterior_improvement),
        format!("{:.6}", rules.min_win_probability_improvement),
        rules.require_same_data.to_string(),
        rules.require_same_factor_version.to_string(),
    ])
}

pub struct ArtifactStatusCommandInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
    pub artifact_id: Option<&'a str>,
    pub kind: Option<&'a str>,
    pub latest_only: bool,
    pub actionable_only: bool,
    pub rule_break_only: bool,
    pub sort_by: &'a str,
    pub descending: bool,
    pub limit: Option<usize>,
    pub recent_n: Option<usize>,
    pub consumed_only: bool,
    pub bucket_by_kind: bool,
    pub bucket_order_by: &'a str,
    pub bucket_limit: Option<usize>,
}

pub struct ArtifactDiffCommandInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
    pub left_artifact_id: &'a str,
    pub right_artifact_id: &'a str,
}

pub struct ArtifactLineageCommandInput<'a> {
    pub symbol: &'a str,
    pub ledger: &'a [ArtifactLedgerEntry],
    pub summaries: Vec<ArtifactLineageSummary>,
    pub artifact_id: Option<&'a str>,
    pub latest_only: bool,
    pub improving_only: bool,
    pub regressing_only: bool,
    pub rule_break_only: bool,
}

pub fn artifact_status_entry_view(entry: ArtifactLedgerEntry) -> ArtifactStatusEntryView {
    let path = entry.path.trim();
    let path_exists = !path.is_empty() && std::path::Path::new(path).exists();
    let path_kind = if path.is_empty() {
        "none"
    } else if path_exists {
        "file"
    } else if std::path::Path::new(path).is_absolute() {
        "missing_absolute_file"
    } else {
        "missing_relative_file"
    };
    ArtifactStatusEntryView {
        entry,
        path_kind: path_kind.to_string(),
        path_exists,
    }
}

pub fn artifact_status_command(input: ArtifactStatusCommandInput<'_>) -> Result<()> {
    let ArtifactStatusCommandInput {
        symbol,
        state_dir,
        artifact_id,
        kind,
        latest_only,
        actionable_only,
        rule_break_only,
        sort_by,
        descending,
        limit,
        recent_n,
        consumed_only,
        bucket_by_kind,
        bucket_order_by,
        bucket_limit,
    } = input;
    let ledger = load_artifact_ledger(state_dir, symbol)?;
    let mut entries = ledger.clone();
    if let Some(artifact_id) = artifact_id {
        entries.retain(|entry| entry.artifact_id == artifact_id);
    }
    if let Some(kind) = kind {
        entries.retain(|entry| entry.artifact_kind == kind);
    }
    if actionable_only {
        entries.retain(|entry| entry.actionable);
    }
    if consumed_only {
        entries.retain(|entry| entry.consumed_by_update_run_id.is_some());
    }
    if rule_break_only {
        entries.retain(|entry| artifact_entry_is_rule_break(&ledger, entry));
    }
    if let Some(recent_n) = recent_n {
        entries.sort_by_key(artifact_generated_recency_key);
        entries.reverse();
        entries.truncate(recent_n);
    }
    if latest_only {
        entries = latest_artifact_entries_by_kind(&entries);
    }
    sort_artifact_entries(&mut entries, sort_by, descending)?;
    if let Some(limit) = limit {
        entries.truncate(limit);
    }
    if bucket_by_kind {
        let mut buckets = BTreeMap::<String, Vec<ArtifactLedgerEntry>>::new();
        for entry in entries {
            buckets
                .entry(entry.artifact_kind.clone())
                .or_default()
                .push(entry);
        }
        let mut bucket_items = buckets.into_iter().collect::<Vec<_>>();
        sort_artifact_buckets(&mut bucket_items, bucket_order_by, descending)?;
        let buckets = bucket_items
            .into_iter()
            .map(|(kind, mut values)| {
                if let Some(limit) = bucket_limit {
                    values.truncate(limit);
                }
                (
                    kind,
                    values
                        .into_iter()
                        .map(artifact_status_entry_view)
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        println!(
            "{}",
            serde_json::to_string_pretty(&ArtifactStatusBucketView {
                symbol: symbol.to_string(),
                total_entries: buckets.values().map(Vec::len).sum(),
                buckets,
            })?
        );
        return Ok(());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&ArtifactStatusView {
            symbol: symbol.to_string(),
            total_entries: entries.len(),
            entries: entries
                .into_iter()
                .map(artifact_status_entry_view)
                .collect::<Vec<_>>(),
        })?
    );
    Ok(())
}

pub fn build_artifact_history_summary(
    artifact_ledger: &[ArtifactLedgerEntry],
) -> ArtifactHistorySummary {
    let total_entries = artifact_ledger.len();
    let pending_update_entries = artifact_ledger
        .iter()
        .filter(|entry| entry.artifact_kind == "pending_update")
        .count();
    let execution_candidate_entries = artifact_ledger
        .iter()
        .filter(|entry| entry.artifact_kind == "execution_candidate")
        .count();
    let ensemble_vote_entries = artifact_ledger
        .iter()
        .filter(|entry| entry.artifact_kind == "ensemble_vote")
        .count();
    let promotable_entries = artifact_ledger
        .iter()
        .filter(|entry| entry.promote_candidate)
        .count();
    let actionable_entries = artifact_ledger
        .iter()
        .filter(|entry| entry.actionable)
        .count();
    let consumed_entries = artifact_ledger
        .iter()
        .filter(|entry| entry.consumed_by_update_run_id.is_some())
        .count();
    let average_quality_score = if total_entries == 0 {
        0.0
    } else {
        artifact_ledger
            .iter()
            .map(|entry| entry.quality_score as f64)
            .sum::<f64>()
            / total_entries as f64
    };
    let latest_consumed_artifact_id = artifact_ledger
        .iter()
        .rev()
        .find(|entry| entry.consumed_by_update_run_id.is_some())
        .map(|entry| entry.artifact_id.clone());
    let mut statuses_by_kind = BTreeMap::<String, BTreeMap<String, usize>>::new();
    for entry in artifact_ledger {
        let kind = statuses_by_kind
            .entry(entry.artifact_kind.clone())
            .or_default();
        *kind.entry(entry.status.clone()).or_default() += 1;
    }

    ArtifactHistorySummary {
        total_entries,
        pending_update_entries,
        execution_candidate_entries,
        ensemble_vote_entries,
        promotable_entries,
        actionable_entries,
        consumed_entries,
        average_quality_score,
        latest_consumed_artifact_id,
        statuses_by_kind,
    }
}

pub fn filter_artifact_lineage_summaries(
    summaries: Vec<ArtifactLineageSummary>,
    improving_only: bool,
    regressing_only: bool,
    rule_break_only: bool,
) -> Result<Vec<ArtifactLineageSummary>> {
    let filter_count = improving_only as u8 + regressing_only as u8 + rule_break_only as u8;
    if filter_count > 1 {
        bail!(
            "artifact-lineage accepts at most one of --improving-only/--regressing-only/--rule-break-only"
        );
    }
    Ok(summaries
        .into_iter()
        .filter(|summary| {
            (!improving_only || summary.conclusion == "improving")
                && (!regressing_only || summary.conclusion == "deteriorating")
                && (!rule_break_only || summary.review_rule_break_count > 0)
        })
        .collect())
}

pub fn artifact_lineage_focus_artifact_id(
    ledger: &[ArtifactLedgerEntry],
    artifact_id: Option<&str>,
    latest_only: bool,
) -> Option<String> {
    if let Some(artifact_id) = artifact_id {
        Some(artifact_id.to_string())
    } else if latest_only {
        ledger
            .iter()
            .max_by_key(|entry| artifact_generated_recency_key(entry))
            .map(|entry| entry.artifact_id.clone())
    } else {
        None
    }
}

pub fn artifact_lineage_view(
    symbol: &str,
    ledger: &[ArtifactLedgerEntry],
    focus_artifact_id: Option<String>,
) -> ArtifactLineageView {
    let nodes = if let Some(focus) = focus_artifact_id.as_deref() {
        let mut related = BTreeMap::<String, ArtifactLedgerEntry>::new();
        for entry in ledger {
            if entry.artifact_id == focus
                || entry.supersedes_artifact_id.as_deref() == Some(focus)
                || entry.artifact_id
                    == ledger
                        .iter()
                        .find(|candidate| candidate.artifact_id == focus)
                        .and_then(|candidate| candidate.supersedes_artifact_id.clone())
                        .unwrap_or_default()
            {
                related.insert(entry.artifact_id.clone(), entry.clone());
            }
        }
        related.into_values().collect()
    } else {
        ledger.to_vec()
    };

    let edges = nodes
        .iter()
        .flat_map(|entry| {
            let mut edges = Vec::new();
            if let Some(previous) = &entry.supersedes_artifact_id {
                edges.push(ArtifactLineageEdge {
                    from: previous.clone(),
                    to: entry.artifact_id.clone(),
                    relation: "supersedes".to_string(),
                });
            }
            if let Some(update_run_id) = &entry.consumed_by_update_run_id {
                edges.push(ArtifactLineageEdge {
                    from: entry.artifact_id.clone(),
                    to: update_run_id.clone(),
                    relation: "consumed_by_update".to_string(),
                });
            }
            edges
        })
        .collect::<Vec<_>>();

    ArtifactLineageView {
        symbol: symbol.to_string(),
        focus_artifact_id,
        nodes,
        edges,
    }
}

pub fn artifact_diff_command(input: ArtifactDiffCommandInput<'_>) -> Result<()> {
    let ArtifactDiffCommandInput {
        symbol,
        state_dir,
        left_artifact_id,
        right_artifact_id,
    } = input;
    let ledger = load_artifact_ledger(state_dir, symbol)?;
    let left_entry = ledger
        .iter()
        .find(|entry| entry.artifact_id == left_artifact_id)
        .ok_or_else(|| anyhow::anyhow!("unknown artifact id '{}'", left_artifact_id))?;
    let right_entry = ledger
        .iter()
        .find(|entry| entry.artifact_id == right_artifact_id)
        .ok_or_else(|| anyhow::anyhow!("unknown artifact id '{}'", right_artifact_id))?;
    if left_entry.artifact_kind != right_entry.artifact_kind {
        bail!(
            "artifact kinds differ: '{}' vs '{}'",
            left_entry.artifact_kind,
            right_entry.artifact_kind
        );
    }

    let view = match left_entry.artifact_kind.as_str() {
        "pending_update" => artifact_diff_view_for_pending_update(
            &ledger,
            state_dir,
            symbol,
            left_artifact_id,
            right_artifact_id,
        )?,
        "execution_candidate" => artifact_diff_view_for_execution_candidate(
            &ledger,
            state_dir,
            symbol,
            left_artifact_id,
            right_artifact_id,
        )?,
        other => bail!("artifact-diff not supported for artifact kind '{}'", other),
    };
    println!("{}", serde_json::to_string_pretty(&view)?);
    Ok(())
}

pub fn artifact_lineage_command(input: ArtifactLineageCommandInput<'_>) -> Result<()> {
    let ArtifactLineageCommandInput {
        symbol,
        ledger,
        summaries,
        artifact_id,
        latest_only,
        improving_only,
        regressing_only,
        rule_break_only,
    } = input;
    let focus_artifact_id = artifact_lineage_focus_artifact_id(ledger, artifact_id, latest_only);

    if focus_artifact_id.is_none() {
        let summaries = filter_artifact_lineage_summaries(
            summaries,
            improving_only,
            regressing_only,
            rule_break_only,
        )?;
        println!("{}", serde_json::to_string_pretty(&summaries)?);
        return Ok(());
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&artifact_lineage_view(symbol, ledger, focus_artifact_id))?
    );
    Ok(())
}

pub fn artifact_generated_recency_key(entry: &ArtifactLedgerEntry) -> (i64, usize, String) {
    (
        entry.generated_at.timestamp_millis(),
        entry.version,
        entry.artifact_id.clone(),
    )
}

pub fn artifact_consumed_recency_key(entry: &ArtifactLedgerEntry) -> (i64, i64, usize, String) {
    (
        entry
            .consumed_at
            .unwrap_or(entry.generated_at)
            .timestamp_millis(),
        entry.generated_at.timestamp_millis(),
        entry.version,
        entry.artifact_id.clone(),
    )
}

pub fn latest_artifact_entries_by_kind(
    entries: &[ArtifactLedgerEntry],
) -> Vec<ArtifactLedgerEntry> {
    let mut latest = BTreeMap::<String, ArtifactLedgerEntry>::new();
    for entry in entries {
        let should_replace = latest
            .get(&entry.artifact_kind)
            .map(|current| {
                artifact_generated_recency_key(entry) > artifact_generated_recency_key(current)
            })
            .unwrap_or(true);
        if should_replace {
            latest.insert(entry.artifact_kind.clone(), entry.clone());
        }
    }
    latest.into_values().collect()
}

pub fn artifact_lineage_path(
    ledger: &[ArtifactLedgerEntry],
    left_artifact_id: &str,
    right_artifact_id: &str,
) -> Vec<String> {
    let mut chain = Vec::new();
    let mut current = Some(right_artifact_id.to_string());
    while let Some(artifact_id) = current {
        chain.push(artifact_id.clone());
        if artifact_id == left_artifact_id {
            chain.reverse();
            return chain;
        }
        current = ledger
            .iter()
            .find(|entry| entry.artifact_id == artifact_id)
            .and_then(|entry| entry.supersedes_artifact_id.clone());
    }
    Vec::new()
}

pub fn artifact_lineage_numeric_evidence(
    ledger: &[ArtifactLedgerEntry],
    lineage_artifact_ids: &[String],
) -> Vec<String> {
    if lineage_artifact_ids.len() < 2 {
        return Vec::new();
    }
    let entries = lineage_artifact_ids
        .iter()
        .filter_map(|artifact_id| {
            ledger
                .iter()
                .find(|entry| &entry.artifact_id == artifact_id)
        })
        .collect::<Vec<_>>();
    let Some(first) = entries.first() else {
        return Vec::new();
    };
    let Some(last) = entries.last() else {
        return Vec::new();
    };
    vec![
        format!("lineage_steps={}", entries.len()),
        format!(
            "lineage_quality_delta={}",
            last.quality_score - first.quality_score
        ),
        format!(
            "lineage_consumed_entries={}",
            entries
                .iter()
                .filter(|entry| entry.consumed_by_update_run_id.is_some())
                .count()
        ),
    ]
}

pub fn pending_update_artifact_by_id(
    state_dir: &str,
    symbol: &str,
    artifact_id: &str,
) -> Result<PendingUpdateArtifact> {
    load_pending_update_history(state_dir, symbol)?
        .into_iter()
        .find(|artifact| artifact.artifact_id == artifact_id)
        .ok_or_else(|| anyhow::anyhow!("unknown pending_update artifact '{}'", artifact_id))
}

pub fn execution_candidate_artifact_by_id(
    state_dir: &str,
    symbol: &str,
    artifact_id: &str,
) -> Result<ExecutionCandidateArtifact> {
    load_execution_candidate_history(state_dir, symbol)?
        .into_iter()
        .find(|artifact| artifact.artifact_id == artifact_id)
        .ok_or_else(|| anyhow::anyhow!("unknown execution_candidate artifact '{}'", artifact_id))
}

pub fn pending_update_artifact_diff(
    previous: &PendingUpdateArtifact,
    current: &PendingUpdateArtifact,
) -> PendingUpdateArtifactDiff {
    let mut changed_fields = Vec::new();
    if previous.entry_quality != current.entry_quality {
        changed_fields.push("entry_quality".to_string());
    }
    if previous.factor_alignment != current.factor_alignment {
        changed_fields.push("factor_alignment".to_string());
    }
    if previous.factor_uncertainty != current.factor_uncertainty {
        changed_fields.push("factor_uncertainty".to_string());
    }
    if previous
        .template_feedback
        .model_probabilities_before_trade
        .selected_direction
        != current
            .template_feedback
            .model_probabilities_before_trade
            .selected_direction
    {
        changed_fields.push("selected_direction".to_string());
    }
    if previous.provenance.data_fingerprint != current.provenance.data_fingerprint {
        changed_fields.push("data_fingerprint".to_string());
    }
    if previous.provenance.factor_version != current.provenance.factor_version {
        changed_fields.push("factor_version".to_string());
    }
    let comparable_same_data =
        previous.provenance.data_fingerprint == current.provenance.data_fingerprint;
    let comparable_same_factor_version =
        previous.provenance.factor_version == current.provenance.factor_version;
    let comparable_same_prompt_version =
        previous.provenance.prompt_version == current.provenance.prompt_version;
    let selected_probability_delta =
        current.selected_win_probability - previous.selected_win_probability;
    let top_factor_score_delta = current.top_factor_score - previous.top_factor_score;
    let avg_family_score_delta = current.avg_family_score - previous.avg_family_score;
    let quality_delta =
        pending_update_quality_score(current) - pending_update_quality_score(previous);
    PendingUpdateArtifactDiff {
        previous_artifact_id: Some(previous.artifact_id.clone()),
        exact_duplicate: changed_fields.is_empty(),
        changed_fields,
        quality_delta,
        comparable_same_data,
        comparable_same_factor_version,
        comparable_same_prompt_version,
        selected_probability_delta,
        top_factor_score_delta,
        avg_family_score_delta,
    }
}

pub fn pending_update_quality_score(artifact: &PendingUpdateArtifact) -> i32 {
    let entry_quality = match artifact.entry_quality.as_str() {
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    };
    let alignment = match artifact.factor_alignment.as_str() {
        "bullish" | "bearish" => 2,
        "mixed" => 1,
        _ => 0,
    };
    let uncertainty = match artifact.factor_uncertainty.as_str() {
        "low" => 2,
        "high" => 0,
        _ => 1,
    };
    let probability = (artifact
        .template_feedback
        .model_probabilities_before_trade
        .selected_probability
        * 100.0)
        .round() as i32;
    entry_quality * 100 + alignment * 10 + uncertainty * 5 + probability
}

pub fn execution_candidate_artifact_diff(
    previous: &ExecutionCandidateArtifact,
    current: &ExecutionCandidateArtifact,
) -> ExecutionCandidateArtifactDiff {
    let mut changed_fields = Vec::new();
    if previous.selected_direction != current.selected_direction {
        changed_fields.push("selected_direction".to_string());
    }
    if previous.trade_direction != current.trade_direction {
        changed_fields.push("trade_direction".to_string());
    }
    if previous.actionable != current.actionable {
        changed_fields.push("actionable".to_string());
    }
    if previous.factor_alignment != current.factor_alignment {
        changed_fields.push("factor_alignment".to_string());
    }
    if previous.factor_uncertainty != current.factor_uncertainty {
        changed_fields.push("factor_uncertainty".to_string());
    }
    if previous.provenance.data_fingerprint != current.provenance.data_fingerprint {
        changed_fields.push("data_fingerprint".to_string());
    }
    if previous.provenance.factor_version != current.provenance.factor_version {
        changed_fields.push("factor_version".to_string());
    }
    ExecutionCandidateArtifactDiff {
        previous_artifact_id: Some(previous.artifact_id.clone()),
        posterior_delta: current.posterior - previous.posterior,
        win_probability_delta: current.win_probability - previous.win_probability,
        entry_delta: current.entry - previous.entry,
        exact_duplicate: changed_fields.is_empty(),
        changed_fields,
    }
}

fn max_probability_label(distribution: &BTreeMap<String, f64>) -> (Option<String>, f64) {
    distribution
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(label, value)| (Some(label.clone()), *value))
        .unwrap_or((None, 0.0))
}

fn bridge_selected_entry_quality(bridge: &PreBayesEntryQualityBridge) -> String {
    max_probability_label(&bridge.selected_entry_quality)
        .0
        .unwrap_or_default()
}

pub fn pending_update_summary(
    state_dir: &str,
    symbol: &str,
    artifact: &PendingUpdateArtifact,
) -> PendingUpdateArtifactSummary {
    PendingUpdateArtifactSummary {
        artifact_id: artifact.artifact_id.clone(),
        version: artifact.version,
        generated_at: artifact.generated_at,
        symbol: artifact.symbol.clone(),
        source_phase: artifact.source_phase.clone(),
        source_run_id: artifact.source_run_id.clone(),
        path: artifact_state_path(state_dir, symbol, PENDING_UPDATE_ARTIFACT_FILE),
        decision_hint: artifact.decision_hint.clone(),
        entry_quality: artifact.entry_quality.clone(),
        factor_alignment: artifact.factor_alignment.clone(),
        factor_uncertainty: artifact.factor_uncertainty.clone(),
        top_factor_name: artifact.top_factor_name.clone(),
        top_factor_action: artifact.top_factor_action.clone(),
        review_rule_version: artifact.review_rule_version.clone(),
        review_status: artifact.review_decision.status.clone(),
        review_reason: artifact.review_decision.reason.clone(),
        pre_bayes_gate_status: artifact
            .pre_bayes_evidence_filter
            .as_ref()
            .map(|filter| filter.gating_status.clone())
            .unwrap_or_default(),
        pre_bayes_bridge_selected_entry_quality: artifact
            .pre_bayes_entry_quality_bridge
            .as_ref()
            .map(bridge_selected_entry_quality)
            .unwrap_or_default(),
        multi_timeframe_summary: artifact.multi_timeframe_summary.clone(),
        quality_delta: artifact.diff_from_previous.quality_delta,
        selected_probability_delta: artifact.diff_from_previous.selected_probability_delta,
        top_factor_score_delta: artifact.diff_from_previous.top_factor_score_delta,
        avg_family_score_delta: artifact.diff_from_previous.avg_family_score_delta,
    }
}

pub fn execution_candidate_summary(
    state_dir: &str,
    symbol: &str,
    artifact: &ExecutionCandidateArtifact,
) -> ExecutionCandidateArtifactSummary {
    ExecutionCandidateArtifactSummary {
        artifact_id: artifact.artifact_id.clone(),
        version: artifact.version,
        generated_at: artifact.generated_at,
        symbol: artifact.symbol.clone(),
        source_phase: artifact.source_phase.clone(),
        source_run_id: artifact.source_run_id.clone(),
        path: artifact_state_path(state_dir, symbol, EXECUTION_CANDIDATE_FILE),
        trade_direction: format!("{:?}", artifact.trade_direction),
        actionable: artifact.actionable,
        candidate_status: artifact.candidate_status.clone(),
        decision_hint: artifact.decision_hint.clone(),
        top_factor_name: artifact.top_factor_name.clone(),
        top_factor_action: artifact.top_factor_action.clone(),
        review_rule_version: artifact.review_rule_version.clone(),
        review_status: artifact.review_decision.status.clone(),
        review_reason: artifact.review_decision.reason.clone(),
        pre_bayes_gate_status: artifact
            .pre_bayes_evidence_filter
            .as_ref()
            .map(|filter| filter.gating_status.clone())
            .unwrap_or_default(),
        pre_bayes_bridge_selected_entry_quality: artifact
            .pre_bayes_entry_quality_bridge
            .as_ref()
            .map(bridge_selected_entry_quality)
            .unwrap_or_default(),
        multi_timeframe_summary: artifact.multi_timeframe_summary.clone(),
        posterior_delta: artifact.diff_from_previous.posterior_delta,
        win_probability_delta: artifact.diff_from_previous.win_probability_delta,
    }
}

fn latest_factor_action(
    snapshot: &Option<WorkflowPhaseSnapshot>,
    factor_name: &str,
) -> Option<String> {
    snapshot.as_ref().and_then(|snapshot| {
        snapshot.factor_actions.iter().find_map(|item| {
            let mut parts = item.splitn(3, ':');
            let name = parts.next()?;
            let action = parts.next()?;
            (name == factor_name).then(|| action.to_string())
        })
    })
}

fn latest_family_state(snapshot: &Option<WorkflowPhaseSnapshot>, family: &str) -> Option<String> {
    snapshot.as_ref().and_then(|snapshot| {
        snapshot.family_states.iter().find_map(|item| {
            let mut parts = item.splitn(3, ':');
            let name = parts.next()?;
            let promotion = parts.next()?;
            let rollback = parts.next()?;
            (name == family).then(|| format!("{}:{}", promotion, rollback))
        })
    })
}

fn consumed_impact_window(
    label: &str,
    entries: &[&ArtifactLedgerEntry],
) -> ArtifactConsumedImpactWindow {
    let count = entries.len();
    let positive = entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.consumption_regrade_status.as_deref(),
                Some("validated_positive")
            )
        })
        .count();
    let negative = entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.consumption_regrade_status.as_deref(),
                Some("validated_negative")
            )
        })
        .count();
    let neutral = entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.consumption_regrade_status.as_deref(),
                Some("validated_neutral")
            )
        })
        .count();
    let cumulative_quality_delta = entries
        .windows(2)
        .map(|window| window[1].quality_score - window[0].quality_score)
        .sum();
    ArtifactConsumedImpactWindow {
        label: label.to_string(),
        count,
        positive,
        negative,
        neutral,
        average_quality_score: if count == 0 {
            0.0
        } else {
            entries
                .iter()
                .map(|entry| entry.quality_score as f64)
                .sum::<f64>()
                / count as f64
        },
        cumulative_quality_delta,
    }
}

fn consumed_impact_trend_comparison(
    window: usize,
    consumed_entries: &[&ArtifactLedgerEntry],
) -> Option<ArtifactConsumedImpactTrendComparison> {
    if consumed_entries.len() < window + 1 {
        return None;
    }
    let recent_slice = &consumed_entries[consumed_entries.len() - window..];
    let baseline_end = consumed_entries.len().saturating_sub(window);
    let baseline_start = baseline_end.saturating_sub(window);
    let baseline_slice = &consumed_entries[baseline_start..baseline_end];
    if baseline_slice.is_empty() {
        return None;
    }
    let recent = consumed_impact_window(&format!("recent_{}", window), recent_slice);
    let baseline = consumed_impact_window(
        &format!("previous_{}", baseline_slice.len()),
        baseline_slice,
    );
    let recent_positive_rate = recent.positive as f64 / recent.count.max(1) as f64;
    let baseline_positive_rate = baseline.positive as f64 / baseline.count.max(1) as f64;
    let average_quality_score_delta = recent.average_quality_score - baseline.average_quality_score;
    let cumulative_quality_delta_delta =
        recent.cumulative_quality_delta - baseline.cumulative_quality_delta;
    let positive_rate_delta = recent_positive_rate - baseline_positive_rate;
    let conclusion = if average_quality_score_delta > 5.0 || positive_rate_delta > 0.25 {
        "improving".to_string()
    } else if average_quality_score_delta < -5.0 || positive_rate_delta < -0.25 {
        "regressing".to_string()
    } else {
        "stable".to_string()
    };
    Some(ArtifactConsumedImpactTrendComparison {
        label: format!("recent_{}_vs_previous_{}", window, baseline_slice.len()),
        recent,
        baseline,
        average_quality_score_delta,
        cumulative_quality_delta_delta,
        positive_rate_delta,
        conclusion,
    })
}

fn consumed_validation_rank(status: &str) -> u8 {
    match status {
        "validated_regressing" => 3,
        "validated_improving" => 2,
        "validated_stable" => 1,
        _ => 0,
    }
}

fn consumed_validation_score(status: &str, reason: &str) -> f64 {
    let quality_delta = reason
        .split("avg_quality_score_delta=")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|value| value.trim_end_matches(',').parse::<f64>().ok())
        .unwrap_or(0.0);
    let positive_rate_delta = reason
        .split("positive_rate_delta=")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|value| value.trim_end_matches(',').parse::<f64>().ok())
        .unwrap_or(0.0);
    let magnitude = quality_delta.abs().max((positive_rate_delta * 100.0).abs());
    match status {
        "validated_regressing" => -magnitude,
        "validated_improving" => magnitude,
        _ => 0.0,
    }
}

fn consumed_validation_status_from_comparisons(
    comparisons: &[ArtifactConsumedImpactTrendComparison],
) -> (String, String) {
    let thresholds = DecisionThresholds::default();
    let primary = comparisons
        .iter()
        .filter(|comparison| comparison.recent.count >= thresholds.artifact_consumed_min_window)
        .max_by(|left, right| {
            left.recent.count.cmp(&right.recent.count).then_with(|| {
                left.average_quality_score_delta
                    .abs()
                    .partial_cmp(&right.average_quality_score_delta.abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });
    match primary {
        Some(primary)
            if primary.average_quality_score_delta
                >= thresholds.artifact_consumed_improvement_quality_delta
                || primary.positive_rate_delta
                    >= thresholds.artifact_consumed_improvement_positive_rate_delta =>
        {
            (
                "validated_improving".to_string(),
                format!(
                    "label={} avg_quality_score_delta={:.2} positive_rate_delta={:.3} min_window={} improvement_thresholds=({:.2},{:.3})",
                    primary.label,
                    primary.average_quality_score_delta,
                    primary.positive_rate_delta,
                    thresholds.artifact_consumed_min_window,
                    thresholds.artifact_consumed_improvement_quality_delta,
                    thresholds.artifact_consumed_improvement_positive_rate_delta
                ),
            )
        }
        Some(primary)
            if primary.average_quality_score_delta
                <= thresholds.artifact_consumed_regression_quality_delta
                || primary.positive_rate_delta
                    <= thresholds.artifact_consumed_regression_positive_rate_delta =>
        {
            (
                "validated_regressing".to_string(),
                format!(
                    "label={} avg_quality_score_delta={:.2} positive_rate_delta={:.3} min_window={} regression_thresholds=({:.2},{:.3})",
                    primary.label,
                    primary.average_quality_score_delta,
                    primary.positive_rate_delta,
                    thresholds.artifact_consumed_min_window,
                    thresholds.artifact_consumed_regression_quality_delta,
                    thresholds.artifact_consumed_regression_positive_rate_delta
                ),
            )
        }
        Some(primary) => (
            "validated_stable".to_string(),
            format!(
                "label={} avg_quality_score_delta={:.2} positive_rate_delta={:.3} thresholds_not_crossed",
                primary.label, primary.average_quality_score_delta, primary.positive_rate_delta
            ),
        ),
        None if comparisons.is_empty() => (
            "insufficient_consumed_history".to_string(),
            format!(
                "no_comparisons_available min_window={}",
                thresholds.artifact_consumed_min_window
            ),
        ),
        None => (
            "insufficient_consumed_history".to_string(),
            format!(
                "comparisons_below_min_window min_window={}",
                thresholds.artifact_consumed_min_window
            ),
        ),
    }
}

pub fn build_artifact_consumed_impact_summary(
    artifact_ledger: &[ArtifactLedgerEntry],
) -> ArtifactConsumedImpactSummary {
    let mut consumed_entries = artifact_ledger
        .iter()
        .filter(|entry| entry.consumed_by_update_run_id.is_some())
        .collect::<Vec<_>>();
    consumed_entries.sort_by_key(|entry| artifact_consumed_recency_key(entry));
    let mut previous_quality = None;
    let points = consumed_entries
        .iter()
        .map(|entry| {
            let delta = previous_quality
                .map(|value| entry.quality_score - value)
                .unwrap_or(0);
            previous_quality = Some(entry.quality_score);
            crate::state::ArtifactConsumedImpactPoint {
                artifact_id: entry.artifact_id.clone(),
                artifact_kind: entry.artifact_kind.clone(),
                consumed_at: entry.consumed_at,
                consumed_outcome: entry.consumed_outcome.clone(),
                quality_score: entry.quality_score,
                regrade_status: entry.consumption_regrade_status.clone(),
                quality_delta_from_previous_consumed: delta,
            }
        })
        .collect::<Vec<_>>();
    let by_kind = consumed_entries
        .iter()
        .fold(
            BTreeMap::<String, Vec<&ArtifactLedgerEntry>>::new(),
            |mut acc, entry| {
                acc.entry(entry.artifact_kind.clone())
                    .or_default()
                    .push(*entry);
                acc
            },
        )
        .into_iter()
        .map(|(kind, entries)| (kind, consumed_impact_window("all", &entries)))
        .collect::<BTreeMap<_, _>>();
    let recent_windows = [3usize, 5usize]
        .into_iter()
        .filter(|&window| consumed_entries.len() >= window)
        .map(|window| {
            consumed_impact_window(
                &format!("recent_{}", window),
                &consumed_entries[consumed_entries.len() - window..],
            )
        })
        .collect::<Vec<_>>();
    let trend_comparisons = [3usize, 5usize]
        .into_iter()
        .filter_map(|window| consumed_impact_trend_comparison(window, &consumed_entries))
        .collect::<Vec<_>>();
    let by_kind_trend_comparisons = consumed_entries
        .iter()
        .fold(
            BTreeMap::<String, Vec<&ArtifactLedgerEntry>>::new(),
            |mut acc, entry| {
                acc.entry(entry.artifact_kind.clone())
                    .or_default()
                    .push(*entry);
                acc
            },
        )
        .into_iter()
        .map(|(kind, entries)| {
            let comparisons = [3usize, 5usize]
                .into_iter()
                .filter_map(|window| consumed_impact_trend_comparison(window, &entries))
                .collect::<Vec<_>>();
            (kind, comparisons)
        })
        .collect::<BTreeMap<_, _>>();
    ArtifactConsumedImpactSummary {
        total_consumed: consumed_entries.len(),
        positive_consumed: consumed_entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.consumption_regrade_status.as_deref(),
                    Some("validated_positive")
                )
            })
            .count(),
        negative_consumed: consumed_entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.consumption_regrade_status.as_deref(),
                    Some("validated_negative")
                )
            })
            .count(),
        neutral_consumed: consumed_entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.consumption_regrade_status.as_deref(),
                    Some("validated_neutral")
                )
            })
            .count(),
        cumulative_quality_score: consumed_entries
            .iter()
            .map(|entry| entry.quality_score)
            .sum(),
        points,
        by_kind,
        recent_windows,
        trend_comparisons,
        by_kind_trend_comparisons,
    }
}

pub fn build_artifact_factor_trends(
    artifact_ledger: &[ArtifactLedgerEntry],
    research: &Option<WorkflowPhaseSnapshot>,
    backtest: &Option<WorkflowPhaseSnapshot>,
    update: &Option<WorkflowPhaseSnapshot>,
) -> Vec<ArtifactFactorTrendSummary> {
    let mut grouped = BTreeMap::<String, Vec<&ArtifactLedgerEntry>>::new();
    for entry in artifact_ledger {
        if let Some(factor_name) = &entry.top_factor_name {
            grouped.entry(factor_name.clone()).or_default().push(entry);
        }
    }
    let mut trends = grouped
        .into_iter()
        .map(|(factor_name, entries)| {
            let factor_name_for_reason = factor_name.clone();
            let entries_len = entries.len();
            let mut consumed_entries_sorted = entries
                .iter()
                .copied()
                .filter(|entry| entry.consumed_by_update_run_id.is_some())
                .collect::<Vec<_>>();
            consumed_entries_sorted.sort_by_key(|entry| artifact_consumed_recency_key(entry));
            let consumed_comparisons = [3usize, 5usize]
                .into_iter()
                .filter_map(|window| {
                    consumed_impact_trend_comparison(window, &consumed_entries_sorted)
                })
                .collect::<Vec<_>>();
            let (consumed_validation_status, consumed_validation_reason) =
                consumed_validation_status_from_comparisons(&consumed_comparisons);
            let consumed_validation_rank =
                i32::from(consumed_validation_rank(&consumed_validation_status));
            let consumed_validation_score = consumed_validation_score(
                &consumed_validation_status,
                &consumed_validation_reason,
            );
            let promotable_entries = entries
                .iter()
                .filter(|entry| entry.promote_candidate)
                .count();
            let consumed_entries = entries
                .iter()
                .filter(|entry| entry.consumed_by_update_run_id.is_some())
                .count();
            let average_quality_score = if entries_len == 0 {
                0.0
            } else {
                entries
                    .iter()
                    .map(|entry| entry.quality_score as f64)
                    .sum::<f64>()
                    / entries_len as f64
            };
            let latest_action = entries
                .last()
                .and_then(|entry| entry.top_factor_action.clone());
            let promotion_link_status = if entries.iter().any(|entry| entry.promote_candidate) {
                "promotion_supporting".to_string()
            } else {
                "none".to_string()
            };
            let rollback_link_status = if entries.iter().any(|entry| {
                matches!(
                    entry.consumption_regrade_status.as_deref(),
                    Some("validated_negative")
                )
            }) || consumed_validation_status == "validated_regressing" {
                "rollback_watch".to_string()
            } else {
                "none".to_string()
            };
            let decision_status = if rollback_link_status != "none" {
                "rollback_watch".to_string()
            } else if promotion_link_status != "none"
                || consumed_validation_status == "validated_improving"
            {
                "promotion_supporting".to_string()
            } else {
                "observe".to_string()
            };
            ArtifactFactorTrendSummary {
                factor_name,
                entries: entries_len,
                promotable_entries,
                consumed_entries,
                average_quality_score,
                latest_status: entries.last().map(|entry| entry.status.clone()),
                latest_action: latest_action.clone(),
                decision_status,
                decision_reason: format!(
                    "latest_action={:?} research_action={:?} backtest_action={:?} update_action={:?} consumed_validation_status={} consumed_validation_reason={}",
                    latest_action,
                    latest_factor_action(research, &factor_name_for_reason),
                    latest_factor_action(backtest, &factor_name_for_reason),
                    latest_factor_action(update, &factor_name_for_reason),
                    consumed_validation_status,
                    consumed_validation_reason
                ),
                promotion_link_status,
                rollback_link_status,
                consumed_validation_status,
                consumed_validation_reason,
                consumed_validation_rank,
                consumed_validation_score,
            }
        })
        .collect::<Vec<_>>();
    trends.sort_by(|a, b| {
        b.entries
            .cmp(&a.entries)
            .then_with(|| a.factor_name.cmp(&b.factor_name))
    });
    trends
}

pub fn build_artifact_family_trends(
    artifact_ledger: &[ArtifactLedgerEntry],
    research: &Option<WorkflowPhaseSnapshot>,
    backtest: &Option<WorkflowPhaseSnapshot>,
    update: &Option<WorkflowPhaseSnapshot>,
) -> Vec<ArtifactFamilyTrendSummary> {
    let mut grouped = BTreeMap::<String, Vec<(f64, &ArtifactLedgerEntry)>>::new();
    for entry in artifact_ledger {
        for (family, score) in &entry.family_scores {
            grouped
                .entry(family.clone())
                .or_default()
                .push((*score, entry));
        }
    }
    let mut trends = grouped
        .into_iter()
        .map(|(family, entries)| {
            let family_for_reason = family.clone();
            let entries_len = entries.len();
            let mut consumed_entries_sorted = entries
                .iter()
                .map(|(_, entry)| *entry)
                .filter(|entry| entry.consumed_by_update_run_id.is_some())
                .collect::<Vec<_>>();
            consumed_entries_sorted.sort_by_key(|entry| artifact_consumed_recency_key(entry));
            let consumed_comparisons = [3usize, 5usize]
                .into_iter()
                .filter_map(|window| {
                    consumed_impact_trend_comparison(window, &consumed_entries_sorted)
                })
                .collect::<Vec<_>>();
            let (consumed_validation_status, consumed_validation_reason) =
                consumed_validation_status_from_comparisons(&consumed_comparisons);
            let consumed_validation_rank =
                i32::from(consumed_validation_rank(&consumed_validation_status));
            let consumed_validation_score = consumed_validation_score(
                &consumed_validation_status,
                &consumed_validation_reason,
            );
            let promotable_entries = entries
                .iter()
                .filter(|(_, entry)| entry.promote_candidate)
                .count();
            let consumed_entries = entries
                .iter()
                .filter(|(_, entry)| entry.consumed_by_update_run_id.is_some())
                .count();
            let average_quality_score = if entries_len == 0 {
                0.0
            } else {
                entries
                    .iter()
                    .map(|(_, entry)| entry.quality_score as f64)
                    .sum::<f64>()
                    / entries_len as f64
            };
            let latest = entries.last().copied();
            let promotion_link_status = if entries.iter().any(|(_, entry)| entry.promote_candidate)
            {
                "promotion_supporting".to_string()
            } else {
                "none".to_string()
            };
            let rollback_link_status = if entries.iter().any(|(_, entry)| {
                matches!(
                    entry.consumption_regrade_status.as_deref(),
                    Some("validated_negative")
                )
            }) || consumed_validation_status == "validated_regressing" {
                "rollback_watch".to_string()
            } else {
                "none".to_string()
            };
            let decision_status = if rollback_link_status != "none" {
                "rollback_watch".to_string()
            } else if promotion_link_status != "none"
                || consumed_validation_status == "validated_improving"
            {
                "promotion_supporting".to_string()
            } else {
                "observe".to_string()
            };
            ArtifactFamilyTrendSummary {
                family,
                entries: entries_len,
                promotable_entries,
                consumed_entries,
                average_quality_score,
                latest_status: latest.map(|(_, entry)| entry.status.clone()),
                latest_score: latest.map(|(score, _)| score),
                decision_status,
                decision_reason: format!(
                    "research_state={:?} backtest_state={:?} update_state={:?} consumed_validation_status={} consumed_validation_reason={}",
                    latest_family_state(research, &family_for_reason),
                    latest_family_state(backtest, &family_for_reason),
                    latest_family_state(update, &family_for_reason),
                    consumed_validation_status,
                    consumed_validation_reason
                ),
                promotion_link_status,
                rollback_link_status,
                consumed_validation_status,
                consumed_validation_reason,
                consumed_validation_rank,
                consumed_validation_score,
            }
        })
        .collect::<Vec<_>>();
    trends.sort_by(|a, b| {
        b.entries
            .cmp(&a.entries)
            .then_with(|| a.family.cmp(&b.family))
    });
    trends
}

pub fn artifact_consumed_trend_signal(
    consumed_impact_summary: &ArtifactConsumedImpactSummary,
) -> (String, String, Vec<String>) {
    if consumed_impact_summary.total_consumed == 0 {
        return (
            "no_consumed_validation".to_string(),
            "no_consumed_artifacts".to_string(),
            Vec::new(),
        );
    }
    let primary = consumed_impact_summary
        .trend_comparisons
        .iter()
        .max_by(|left, right| {
            left.recent.count.cmp(&right.recent.count).then_with(|| {
                left.average_quality_score_delta
                    .abs()
                    .partial_cmp(&right.average_quality_score_delta.abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });
    let consumed_target_kinds = consumed_impact_summary
        .by_kind_trend_comparisons
        .iter()
        .filter_map(|(kind, comparisons)| {
            let (status, _) = consumed_validation_status_from_comparisons(comparisons);
            matches!(
                status.as_str(),
                "validated_improving" | "validated_regressing"
            )
            .then(|| kind.clone())
        })
        .collect::<Vec<_>>();
    let (status, reason) = consumed_validation_status_from_comparisons(
        primary
            .map(|comparison| vec![comparison.clone()])
            .unwrap_or_default()
            .as_slice(),
    );
    match primary {
        Some(_) => (
            status,
            format!("{} target_kinds={:?}", reason, consumed_target_kinds),
            consumed_target_kinds,
        ),
        None => (
            "insufficient_consumed_history".to_string(),
            format!(
                "consumed_total={} requires_more_consumed_windows",
                consumed_impact_summary.total_consumed
            ),
            Vec::new(),
        ),
    }
}

pub fn artifact_action_summary(
    factor_trends: &[ArtifactFactorTrendSummary],
    family_trends: &[ArtifactFamilyTrendSummary],
    consumed_impact_summary: &ArtifactConsumedImpactSummary,
) -> Vec<String> {
    let mut summary = Vec::new();
    summary.extend(
        factor_trends
            .iter()
            .filter(|trend| trend.decision_status != "observe")
            .take(3)
            .map(|trend| {
                format!(
                    "factor:{} status={} reason={}",
                    trend.factor_name, trend.decision_status, trend.decision_reason
                )
            }),
    );
    summary.extend(
        family_trends
            .iter()
            .filter(|trend| trend.decision_status != "observe")
            .take(3)
            .map(|trend| {
                format!(
                    "family:{} status={} reason={}",
                    trend.family, trend.decision_status, trend.decision_reason
                )
            }),
    );
    let (consumed_trend_status, consumed_trend_reason, _) =
        artifact_consumed_trend_signal(consumed_impact_summary);
    if matches!(
        consumed_trend_status.as_str(),
        "validated_improving" | "validated_regressing"
    ) {
        summary.push(format!(
            "consumed:{} reason={}",
            consumed_trend_status, consumed_trend_reason
        ));
    }
    summary
}

pub fn artifact_decision_summary_from_trends(
    actionable_artifacts: &[ArtifactLedgerEntry],
    latest_promotable_artifact: Option<&ArtifactLedgerEntry>,
    lineage_summaries: &[ArtifactLineageSummary],
    factor_trends: &[ArtifactFactorTrendSummary],
    family_trends: &[ArtifactFamilyTrendSummary],
    consumed_impact_summary: &ArtifactConsumedImpactSummary,
) -> ArtifactDecisionSummary {
    let highlighted_actions =
        artifact_action_summary(factor_trends, family_trends, consumed_impact_summary);
    let highlighted_factor_targets = factor_trends
        .iter()
        .filter(|trend| trend.decision_status != "observe")
        .map(|trend| trend.factor_name.clone())
        .collect::<Vec<_>>();
    let highlighted_family_targets = family_trends
        .iter()
        .filter(|trend| trend.decision_status != "observe")
        .map(|trend| trend.family.clone())
        .collect::<Vec<_>>();
    let (consumed_trend_status, consumed_trend_reason, consumed_target_kinds) =
        artifact_consumed_trend_signal(consumed_impact_summary);
    let mut promotion_strength =
        if latest_promotable_artifact.is_some() && actionable_artifacts.len() >= 2 {
            "high".to_string()
        } else if latest_promotable_artifact.is_some() {
            "medium".to_string()
        } else {
            "low".to_string()
        };
    let mut rollback_strength = if factor_trends
        .iter()
        .any(|trend| trend.rollback_link_status == "rollback_watch")
        || family_trends
            .iter()
            .any(|trend| trend.rollback_link_status == "rollback_watch")
    {
        "high".to_string()
    } else {
        "low".to_string()
    };
    match consumed_trend_status.as_str() {
        "validated_improving" if latest_promotable_artifact.is_some() => {
            promotion_strength = "high".to_string();
        }
        "validated_regressing" => {
            promotion_strength = "low".to_string();
            rollback_strength = "high".to_string();
        }
        _ => {}
    }
    ArtifactDecisionSummary {
        actionable_artifact_count: actionable_artifacts.len(),
        latest_promotable_artifact_id: latest_promotable_artifact
            .map(|entry| entry.artifact_id.clone()),
        artifact_rule_break_count: lineage_summaries
            .iter()
            .map(|summary| summary.review_rule_break_count)
            .sum(),
        summary: format!(
            "actionable_artifacts={} latest_promotable={:?} rule_breaks={} consumed_trend={} consumed_targets={:?}",
            actionable_artifacts.len(),
            latest_promotable_artifact.map(|entry| entry.artifact_id.clone()),
            lineage_summaries
                .iter()
                .map(|summary| summary.review_rule_break_count)
                .sum::<usize>(),
            consumed_trend_status.clone(),
            consumed_target_kinds.clone()
        ),
        highlighted_actions,
        highlighted_factor_targets,
        highlighted_family_targets,
        promotion_strength,
        rollback_strength,
        consumed_trend_status,
        consumed_trend_reason,
        consumed_target_kinds,
    }
}

pub fn artifact_decision_summary_from_snapshot(
    snapshot: &crate::state::WorkflowSnapshot,
    artifact_action_summary: &[String],
) -> ArtifactDecisionSummary {
    let (consumed_trend_status, consumed_trend_reason, consumed_target_kinds) =
        artifact_consumed_trend_signal(&snapshot.artifact_consumed_impact_summary);
    let mut promotion_strength = if snapshot.latest_promotable_artifact.is_some()
        && snapshot.actionable_artifacts.len() >= 2
    {
        "high".to_string()
    } else if snapshot.latest_promotable_artifact.is_some() {
        "medium".to_string()
    } else {
        "low".to_string()
    };
    let mut rollback_strength = if snapshot
        .artifact_factor_trends
        .iter()
        .any(|trend| trend.rollback_link_status == "rollback_watch")
        || snapshot
            .artifact_family_trends
            .iter()
            .any(|trend| trend.rollback_link_status == "rollback_watch")
    {
        "high".to_string()
    } else {
        "low".to_string()
    };
    match consumed_trend_status.as_str() {
        "validated_improving" if snapshot.latest_promotable_artifact.is_some() => {
            promotion_strength = "high".to_string();
        }
        "validated_regressing" => {
            promotion_strength = "low".to_string();
            rollback_strength = "high".to_string();
        }
        _ => {}
    }
    ArtifactDecisionSummary {
        actionable_artifact_count: snapshot.actionable_artifacts.len(),
        latest_promotable_artifact_id: snapshot
            .latest_promotable_artifact
            .as_ref()
            .map(|entry| entry.artifact_id.clone()),
        artifact_rule_break_count: snapshot
            .artifact_lineage_summaries
            .iter()
            .map(|summary| summary.review_rule_break_count)
            .sum(),
        highlighted_actions: artifact_action_summary.to_vec(),
        highlighted_factor_targets: snapshot
            .artifact_factor_trends
            .iter()
            .filter(|trend| trend.decision_status != "observe")
            .map(|trend| trend.factor_name.clone())
            .collect(),
        highlighted_family_targets: snapshot
            .artifact_family_trends
            .iter()
            .filter(|trend| trend.decision_status != "observe")
            .map(|trend| trend.family.clone())
            .collect(),
        promotion_strength,
        rollback_strength,
        consumed_trend_status: consumed_trend_status.clone(),
        consumed_trend_reason: consumed_trend_reason.clone(),
        consumed_target_kinds: consumed_target_kinds.clone(),
        summary: format!(
            "actionable_artifacts={} latest_promotable={:?} rule_breaks={} consumed_trend={}",
            snapshot.actionable_artifacts.len(),
            snapshot
                .latest_promotable_artifact
                .as_ref()
                .map(|entry| entry.artifact_id.clone()),
            snapshot
                .artifact_lineage_summaries
                .iter()
                .map(|summary| summary.review_rule_break_count)
                .sum::<usize>(),
            consumed_trend_status
        ),
    }
}

pub fn artifact_trend_summaries_for_symbol(
    state_dir: &str,
    symbol: &str,
) -> Result<(
    Vec<ArtifactFactorTrendSummary>,
    Vec<ArtifactFamilyTrendSummary>,
)> {
    let ledger = load_artifact_ledger(state_dir, symbol)?;
    Ok(artifact_trend_summaries_from_ledger(&ledger))
}

pub fn artifact_trend_summaries_from_ledger(
    artifact_ledger: &[ArtifactLedgerEntry],
) -> (
    Vec<ArtifactFactorTrendSummary>,
    Vec<ArtifactFamilyTrendSummary>,
) {
    (
        build_artifact_factor_trends(artifact_ledger, &None, &None, &None),
        build_artifact_family_trends(artifact_ledger, &None, &None, &None),
    )
}

pub fn artifact_decision_summary_for_symbol(
    state_dir: &str,
    symbol: &str,
) -> Result<ArtifactDecisionSummary> {
    let ledger = load_artifact_ledger(state_dir, symbol)?;
    Ok(artifact_decision_summary_from_ledger(&ledger))
}

pub fn artifact_decision_summary_from_ledger(
    artifact_ledger: &[ArtifactLedgerEntry],
) -> ArtifactDecisionSummary {
    let actionable_artifacts = artifact_ledger
        .iter()
        .filter(|entry| entry.actionable && entry.consumed_by_update_run_id.is_none())
        .cloned()
        .collect::<Vec<_>>();
    let latest_promotable_artifact = artifact_ledger
        .iter()
        .filter(|entry| entry.promote_candidate && entry.consumed_by_update_run_id.is_none())
        .max_by_key(|entry| artifact_generated_recency_key(entry))
        .cloned();
    let lineage = build_artifact_lineage_summaries(artifact_ledger);
    let (factor_trends, family_trends) = artifact_trend_summaries_from_ledger(artifact_ledger);
    let consumed_impact_summary = build_artifact_consumed_impact_summary(artifact_ledger);
    artifact_decision_summary_from_trends(
        &actionable_artifacts,
        latest_promotable_artifact.as_ref(),
        &lineage,
        &factor_trends,
        &family_trends,
        &consumed_impact_summary,
    )
}

pub fn artifact_rule_break_effects_for_symbol(
    state_dir: &str,
    symbol: &str,
) -> Result<Vec<ArtifactRuleBreakEffect>> {
    let ledger = load_artifact_ledger(state_dir, symbol)?;
    Ok(build_artifact_rule_break_effects(&ledger))
}

pub fn artifact_consumed_impact_summary_for_symbol(
    state_dir: &str,
    symbol: &str,
) -> Result<ArtifactConsumedImpactSummary> {
    let ledger = load_artifact_ledger(state_dir, symbol)?;
    Ok(build_artifact_consumed_impact_summary(&ledger))
}

pub fn apply_artifact_consumption_preview(
    artifact_ledger: &mut [ArtifactLedgerEntry],
    artifact_id: &str,
    update_run_id: &str,
    realized_outcome: &str,
    pnl: f64,
    consumed_at: chrono::DateTime<chrono::Utc>,
) {
    for entry in artifact_ledger {
        if entry.artifact_id != artifact_id {
            continue;
        }
        entry.consumed_by_update_run_id = Some(update_run_id.to_string());
        entry.consumed_at = Some(consumed_at);
        entry.consumed_outcome = Some(realized_outcome.to_string());
        entry.regraded_at = Some(consumed_at);
        let (regrade_status, regrade_reason, quality_adjustment) = match realized_outcome {
            "win" if pnl > 0.0 => ("validated_positive", "consumed_with_positive_pnl", 20),
            "win" => ("validated_positive", "consumed_with_win_outcome", 10),
            "loss" if pnl < 0.0 => ("validated_negative", "consumed_with_negative_pnl", -20),
            "loss" => ("validated_negative", "consumed_with_loss_outcome", -10),
            _ => ("validated_neutral", "consumed_with_breakeven_outcome", 0),
        };
        entry.consumption_regrade_status = Some(regrade_status.to_string());
        entry.consumption_regrade_reason = Some(regrade_reason.to_string());
        entry.quality_score += quality_adjustment;
        entry.actionable = false;
        entry.promote_candidate = false;
    }
}

pub fn top_consumed_trend_comparisons(
    consumed_impact_summary: &ArtifactConsumedImpactSummary,
) -> Vec<ArtifactConsumedImpactTrendComparison> {
    let mut comparisons = consumed_impact_summary.trend_comparisons.clone();
    comparisons.sort_by(|left, right| {
        (right.conclusion != "stable")
            .cmp(&(left.conclusion != "stable"))
            .then_with(|| right.recent.count.cmp(&left.recent.count))
            .then_with(|| {
                right
                    .average_quality_score_delta
                    .abs()
                    .partial_cmp(&left.average_quality_score_delta.abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    comparisons.truncate(3);
    comparisons
}

pub fn artifact_decision_section_from_parts(
    summary: &ArtifactDecisionSummary,
    action_summary: &[String],
    factor_trends: &[ArtifactFactorTrendSummary],
    family_trends: &[ArtifactFamilyTrendSummary],
    rule_break_effects: &[ArtifactRuleBreakEffect],
    consumed_impact_summary: &ArtifactConsumedImpactSummary,
) -> ArtifactDecisionSection {
    ArtifactDecisionSection {
        summary: summary.clone(),
        action_summary: action_summary.to_vec(),
        top_factor_trends: factor_trends.iter().take(3).cloned().collect(),
        top_family_trends: family_trends.iter().take(3).cloned().collect(),
        top_rule_break_effects: rule_break_effects.iter().take(3).cloned().collect(),
        top_consumed_trend_comparisons: top_consumed_trend_comparisons(consumed_impact_summary),
    }
}

pub fn artifact_decision_section_from_snapshot(
    snapshot: &crate::state::WorkflowSnapshot,
) -> ArtifactDecisionSection {
    ArtifactDecisionSection {
        summary: snapshot.artifact_decision_summary.clone(),
        action_summary: snapshot
            .artifact_decision_summary
            .highlighted_actions
            .clone(),
        top_factor_trends: snapshot
            .artifact_factor_trends
            .iter()
            .take(3)
            .cloned()
            .collect(),
        top_family_trends: snapshot
            .artifact_family_trends
            .iter()
            .take(3)
            .cloned()
            .collect(),
        top_rule_break_effects: snapshot
            .artifact_rule_break_effects
            .iter()
            .take(3)
            .cloned()
            .collect(),
        top_consumed_trend_comparisons: top_consumed_trend_comparisons(
            &snapshot.artifact_consumed_impact_summary,
        ),
    }
}

pub fn artifact_embedded_pre_bayes_evidence(
    left_filter: Option<&PreBayesEvidenceFilter>,
    right_filter: Option<&PreBayesEvidenceFilter>,
    left_bridge: Option<&PreBayesEntryQualityBridge>,
    right_bridge: Option<&PreBayesEntryQualityBridge>,
    left_multi_timeframe_summary: &[String],
    right_multi_timeframe_summary: &[String],
) -> Vec<String> {
    let mut evidence = Vec::new();
    match (left_filter, right_filter) {
        (Some(left), Some(right)) => {
            if left.policy.version != right.policy.version {
                evidence.push(format!(
                    "pre_bayes_policy_version:{}->{}",
                    left.policy.version, right.policy.version
                ));
            }
            if left.gating_status != right.gating_status {
                evidence.push(format!(
                    "pre_bayes_gate_status:{}->{}",
                    left.gating_status, right.gating_status
                ));
            }
            if (left.evidence_quality_score - right.evidence_quality_score).abs() > f64::EPSILON {
                evidence.push(format!(
                    "pre_bayes_quality_delta={:.4}",
                    right.evidence_quality_score - left.evidence_quality_score
                ));
            }
            if left.filtered_multi_timeframe_resonance_label
                != right.filtered_multi_timeframe_resonance_label
            {
                evidence.push(format!(
                    "pre_bayes_resonance:{}->{}",
                    left.filtered_multi_timeframe_resonance_label,
                    right.filtered_multi_timeframe_resonance_label
                ));
            }
        }
        (Some(left), None) => evidence.push(format!(
            "pre_bayes_embedded_left_only gate_status={} policy_version={}",
            left.gating_status, left.policy.version
        )),
        (None, Some(right)) => evidence.push(format!(
            "pre_bayes_embedded_right_only gate_status={} policy_version={}",
            right.gating_status, right.policy.version
        )),
        (None, None) => {}
    }
    match (left_bridge, right_bridge) {
        (Some(left), Some(right)) => {
            let (left_selected_entry_quality, left_selected_entry_quality_probability) =
                max_probability_label(&left.selected_entry_quality);
            let (right_selected_entry_quality, right_selected_entry_quality_probability) =
                max_probability_label(&right.selected_entry_quality);
            if left_selected_entry_quality != right_selected_entry_quality {
                evidence.push(format!(
                    "pre_bayes_bridge_selected_entry_quality:{:?}->{:?}",
                    left_selected_entry_quality, right_selected_entry_quality
                ));
            }
            let left_gap = (left.long_signal_probability - left.short_signal_probability).abs();
            let right_gap = (right.long_signal_probability - right.short_signal_probability).abs();
            if (left_gap - right_gap).abs() > f64::EPSILON {
                evidence.push(format!(
                    "pre_bayes_bridge_probability_gap_delta={:.4}",
                    right_gap - left_gap
                ));
            }
            if (left_selected_entry_quality_probability - right_selected_entry_quality_probability)
                .abs()
                > f64::EPSILON
            {
                evidence.push(format!(
                    "pre_bayes_bridge_selected_entry_quality_probability_delta={:.4}",
                    right_selected_entry_quality_probability
                        - left_selected_entry_quality_probability
                ));
            }
        }
        (Some(_), None) => evidence.push("pre_bayes_bridge_left_only".to_string()),
        (None, Some(_)) => evidence.push("pre_bayes_bridge_right_only".to_string()),
        (None, None) => {}
    }
    if left_multi_timeframe_summary != right_multi_timeframe_summary {
        evidence.push(format!(
            "embedded_multi_timeframe_summary_changed left={:?} right={:?}",
            left_multi_timeframe_summary, right_multi_timeframe_summary
        ));
    }
    evidence
}

fn pending_update_embedded_filter<'a>(
    artifact_id: &str,
    artifacts: &'a [PendingUpdateArtifact],
) -> Option<&'a PreBayesEvidenceFilter> {
    artifacts
        .iter()
        .find(|artifact| artifact.artifact_id == artifact_id)
        .and_then(|artifact| artifact.pre_bayes_evidence_filter.as_ref())
}

fn pending_update_embedded_bridge<'a>(
    artifact_id: &str,
    artifacts: &'a [PendingUpdateArtifact],
) -> Option<&'a PreBayesEntryQualityBridge> {
    artifacts
        .iter()
        .find(|artifact| artifact.artifact_id == artifact_id)
        .and_then(|artifact| artifact.pre_bayes_entry_quality_bridge.as_ref())
}

fn pending_update_embedded_mtf<'a>(
    artifact_id: &str,
    artifacts: &'a [PendingUpdateArtifact],
) -> &'a [String] {
    artifacts
        .iter()
        .find(|artifact| artifact.artifact_id == artifact_id)
        .map(|artifact| artifact.multi_timeframe_summary.as_slice())
        .unwrap_or(&[])
}

fn execution_candidate_embedded_filter<'a>(
    artifact_id: &str,
    artifacts: &'a [ExecutionCandidateArtifact],
) -> Option<&'a PreBayesEvidenceFilter> {
    artifacts
        .iter()
        .find(|artifact| artifact.artifact_id == artifact_id)
        .and_then(|artifact| artifact.pre_bayes_evidence_filter.as_ref())
}

fn execution_candidate_embedded_bridge<'a>(
    artifact_id: &str,
    artifacts: &'a [ExecutionCandidateArtifact],
) -> Option<&'a PreBayesEntryQualityBridge> {
    artifacts
        .iter()
        .find(|artifact| artifact.artifact_id == artifact_id)
        .and_then(|artifact| artifact.pre_bayes_entry_quality_bridge.as_ref())
}

fn execution_candidate_embedded_mtf<'a>(
    artifact_id: &str,
    artifacts: &'a [ExecutionCandidateArtifact],
) -> &'a [String] {
    artifacts
        .iter()
        .find(|artifact| artifact.artifact_id == artifact_id)
        .map(|artifact| artifact.multi_timeframe_summary.as_slice())
        .unwrap_or(&[])
}

pub fn embedded_pre_bayes_evidence_for_entry<'a>(
    entry: &'a ArtifactLedgerEntry,
    pending_updates: &'a [PendingUpdateArtifact],
    execution_candidates: &'a [ExecutionCandidateArtifact],
) -> (
    Option<&'a PreBayesEvidenceFilter>,
    Option<&'a PreBayesEntryQualityBridge>,
    &'a [String],
) {
    match entry.artifact_kind.as_str() {
        "pending_update" => (
            pending_update_embedded_filter(&entry.artifact_id, pending_updates),
            pending_update_embedded_bridge(&entry.artifact_id, pending_updates),
            pending_update_embedded_mtf(&entry.artifact_id, pending_updates),
        ),
        "execution_candidate" => (
            execution_candidate_embedded_filter(&entry.artifact_id, execution_candidates),
            execution_candidate_embedded_bridge(&entry.artifact_id, execution_candidates),
            execution_candidate_embedded_mtf(&entry.artifact_id, execution_candidates),
        ),
        _ => (None, None, &[]),
    }
}

pub fn build_artifact_lineage_summaries_with_embedded_snapshots(
    artifact_ledger: &[ArtifactLedgerEntry],
    pending_updates: &[PendingUpdateArtifact],
    execution_candidates: &[ExecutionCandidateArtifact],
) -> Vec<ArtifactLineageSummary> {
    let mut children = BTreeMap::<String, Vec<&ArtifactLedgerEntry>>::new();
    for entry in artifact_ledger {
        if let Some(parent) = &entry.supersedes_artifact_id {
            children.entry(parent.clone()).or_default().push(entry);
        }
    }
    artifact_ledger
        .iter()
        .filter(|entry| entry.supersedes_artifact_id.is_none())
        .map(|root| {
            let mut chain = vec![root];
            let mut current = root;
            while let Some(next) = children
                .get(&current.artifact_id)
                .and_then(|items| items.iter().max_by_key(|item| item.version).copied())
            {
                chain.push(next);
                current = next;
            }
            let first = chain.first().copied().unwrap_or(root);
            let last = chain.last().copied().unwrap_or(root);
            let distinct_review_rule_versions = chain
                .iter()
                .filter_map(|entry| {
                    if entry.review_rule_version.is_empty() {
                        None
                    } else {
                        Some(entry.review_rule_version.clone())
                    }
                })
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let review_rule_break_count = chain
                .windows(2)
                .filter(|window| window[0].review_rule_version != window[1].review_rule_version)
                .count();
            let embedded_pre_bayes_change_count = chain
                .windows(2)
                .filter(|window| {
                    let (left_filter, left_bridge, left_mtf) =
                        embedded_pre_bayes_evidence_for_entry(
                            window[0],
                            pending_updates,
                            execution_candidates,
                        );
                    let (right_filter, right_bridge, right_mtf) =
                        embedded_pre_bayes_evidence_for_entry(
                            window[1],
                            pending_updates,
                            execution_candidates,
                        );
                    !artifact_embedded_pre_bayes_evidence(
                        left_filter,
                        right_filter,
                        left_bridge,
                        right_bridge,
                        left_mtf,
                        right_mtf,
                    )
                    .is_empty()
                })
                .count();
            let (latest_filter, latest_bridge, _) =
                embedded_pre_bayes_evidence_for_entry(last, pending_updates, execution_candidates);
            ArtifactLineageSummary {
                artifact_kind: root.artifact_kind.clone(),
                root_artifact_id: first.artifact_id.clone(),
                latest_artifact_id: last.artifact_id.clone(),
                artifact_count: chain.len(),
                quality_delta: last.quality_score - first.quality_score,
                consumed_count: chain
                    .iter()
                    .filter(|entry| entry.consumed_by_update_run_id.is_some())
                    .count(),
                conclusion: if last.quality_score - first.quality_score > 10 {
                    "improving".to_string()
                } else if first.quality_score - last.quality_score > 10 {
                    "deteriorating".to_string()
                } else {
                    "stable".to_string()
                },
                distinct_review_rule_versions,
                review_rule_break_count,
                embedded_pre_bayes_change_count,
                latest_pre_bayes_gate_status: latest_filter
                    .map(|filter| filter.gating_status.clone())
                    .unwrap_or_default(),
                latest_pre_bayes_bridge_selected_entry_quality: latest_bridge
                    .map(bridge_selected_entry_quality)
                    .unwrap_or_default(),
                latest_pre_bayes_multi_timeframe_direction_bias: latest_filter
                    .map(|filter| filter.filtered_multi_timeframe_direction_bias.clone())
                    .unwrap_or_default(),
            }
        })
        .collect()
}

pub fn build_artifact_lineage_summaries(
    artifact_ledger: &[ArtifactLedgerEntry],
) -> Vec<ArtifactLineageSummary> {
    build_artifact_lineage_summaries_with_embedded_snapshots(artifact_ledger, &[], &[])
}

pub fn artifact_lineage_root_id(
    artifact_ledger: &[ArtifactLedgerEntry],
    artifact_id: &str,
) -> String {
    let mut current = artifact_id.to_string();
    while let Some(parent) = artifact_ledger
        .iter()
        .find(|entry| entry.artifact_id == current)
        .and_then(|entry| entry.supersedes_artifact_id.clone())
    {
        current = parent;
    }
    current
}

pub fn build_artifact_rule_break_effects(
    artifact_ledger: &[ArtifactLedgerEntry],
) -> Vec<ArtifactRuleBreakEffect> {
    let mut effects = Vec::new();
    let mut grouped = BTreeMap::<String, Vec<&ArtifactLedgerEntry>>::new();
    for entry in artifact_ledger {
        let root = artifact_lineage_root_id(artifact_ledger, &entry.artifact_id);
        grouped.entry(root).or_default().push(entry);
    }
    for (root_id, mut entries) in grouped {
        entries.sort_by_key(|entry| entry.version);
        for window in entries.windows(2) {
            let left = window[0];
            let right = window[1];
            if left.review_rule_version != right.review_rule_version {
                effects.push(ArtifactRuleBreakEffect {
                    artifact_kind: right.artifact_kind.clone(),
                    lineage_root_artifact_id: root_id.clone(),
                    from_artifact_id: left.artifact_id.clone(),
                    to_artifact_id: right.artifact_id.clone(),
                    from_rule_version: left.review_rule_version.clone(),
                    to_rule_version: right.review_rule_version.clone(),
                    quality_delta: right.quality_score - left.quality_score,
                    consumed_delta: i32::from(right.consumed_by_update_run_id.is_some())
                        - i32::from(left.consumed_by_update_run_id.is_some()),
                    conclusion: if right.quality_score - left.quality_score > 10 {
                        "improving".to_string()
                    } else if left.quality_score - right.quality_score > 10 {
                        "deteriorating".to_string()
                    } else {
                        "stable".to_string()
                    },
                });
            }
        }
    }
    effects
}

pub fn build_artifact_factor_rule_break_impacts(
    artifact_ledger: &[ArtifactLedgerEntry],
    effects: &[ArtifactRuleBreakEffect],
) -> Vec<ArtifactRuleBreakFactorImpact> {
    let mut grouped = BTreeMap::<String, Vec<&ArtifactRuleBreakEffect>>::new();
    for effect in effects {
        if let Some(name) = artifact_ledger
            .iter()
            .find(|entry| entry.artifact_id == effect.to_artifact_id)
            .and_then(|entry| entry.top_factor_name.clone())
        {
            grouped.entry(name).or_default().push(effect);
        }
    }
    let mut impacts = grouped
        .into_iter()
        .map(|(factor_name, effects)| ArtifactRuleBreakFactorImpact {
            factor_name,
            break_count: effects.len(),
            cumulative_quality_delta: effects.iter().map(|effect| effect.quality_delta).sum(),
            improving_breaks: effects
                .iter()
                .filter(|effect| effect.conclusion == "improving")
                .count(),
            deteriorating_breaks: effects
                .iter()
                .filter(|effect| effect.conclusion == "deteriorating")
                .count(),
            consumed_breaks: effects
                .iter()
                .filter(|effect| effect.consumed_delta > 0)
                .count(),
        })
        .collect::<Vec<_>>();
    impacts.sort_by(|a, b| {
        b.break_count
            .cmp(&a.break_count)
            .then_with(|| b.cumulative_quality_delta.cmp(&a.cumulative_quality_delta))
    });
    impacts
}

pub fn build_artifact_family_rule_break_impacts(
    artifact_ledger: &[ArtifactLedgerEntry],
    effects: &[ArtifactRuleBreakEffect],
) -> Vec<ArtifactRuleBreakFamilyImpact> {
    let mut grouped = BTreeMap::<String, Vec<&ArtifactRuleBreakEffect>>::new();
    for effect in effects {
        if let Some(scores) = artifact_ledger
            .iter()
            .find(|entry| entry.artifact_id == effect.to_artifact_id)
            .map(|entry| entry.family_scores.clone())
        {
            for family in scores.keys() {
                grouped.entry(family.clone()).or_default().push(effect);
            }
        }
    }
    let mut impacts = grouped
        .into_iter()
        .map(|(family, effects)| ArtifactRuleBreakFamilyImpact {
            family,
            break_count: effects.len(),
            cumulative_quality_delta: effects.iter().map(|effect| effect.quality_delta).sum(),
            improving_breaks: effects
                .iter()
                .filter(|effect| effect.conclusion == "improving")
                .count(),
            deteriorating_breaks: effects
                .iter()
                .filter(|effect| effect.conclusion == "deteriorating")
                .count(),
            consumed_breaks: effects
                .iter()
                .filter(|effect| effect.consumed_delta > 0)
                .count(),
        })
        .collect::<Vec<_>>();
    impacts.sort_by(|a, b| {
        b.break_count
            .cmp(&a.break_count)
            .then_with(|| b.cumulative_quality_delta.cmp(&a.cumulative_quality_delta))
    });
    impacts
}

pub fn artifact_diff_view_for_pending_update(
    ledger: &[ArtifactLedgerEntry],
    state_dir: &str,
    symbol: &str,
    left_artifact_id: &str,
    right_artifact_id: &str,
) -> Result<ArtifactDiffView> {
    let left = pending_update_artifact_by_id(state_dir, symbol, left_artifact_id)?;
    let right = pending_update_artifact_by_id(state_dir, symbol, right_artifact_id)?;
    let diff = pending_update_artifact_diff(&left, &right);
    let lineage_artifact_ids = artifact_lineage_path(ledger, left_artifact_id, right_artifact_id);
    Ok(ArtifactDiffView {
        kind: "pending_update".to_string(),
        left_artifact_id: left.artifact_id,
        right_artifact_id: right.artifact_id,
        changed_fields: diff.changed_fields,
        numeric_evidence: vec![
            format!(
                "selected_probability_delta={:.4}",
                diff.selected_probability_delta
            ),
            format!("top_factor_score_delta={:.4}", diff.top_factor_score_delta),
            format!("avg_family_score_delta={:.4}", diff.avg_family_score_delta),
            format!("quality_delta={}", diff.quality_delta),
        ],
        embedded_pre_bayes_evidence: artifact_embedded_pre_bayes_evidence(
            left.pre_bayes_evidence_filter.as_ref(),
            right.pre_bayes_evidence_filter.as_ref(),
            left.pre_bayes_entry_quality_bridge.as_ref(),
            right.pre_bayes_entry_quality_bridge.as_ref(),
            &left.multi_timeframe_summary,
            &right.multi_timeframe_summary,
        ),
        summary: format!(
            "same_data={} same_factor_version={} same_prompt_version={}",
            diff.comparable_same_data,
            diff.comparable_same_factor_version,
            diff.comparable_same_prompt_version
        ),
        cross_rule_version_summary: (left.review_rule_version != right.review_rule_version).then(
            || {
                format!(
                    "rule_version_changed:{}->{} quality_delta={} probability_delta={:.4}",
                    left.review_rule_version,
                    right.review_rule_version,
                    diff.quality_delta,
                    diff.selected_probability_delta
                )
            },
        ),
        lineage_artifact_ids: lineage_artifact_ids.clone(),
        lineage_numeric_evidence: artifact_lineage_numeric_evidence(ledger, &lineage_artifact_ids),
    })
}

pub fn artifact_diff_view_for_execution_candidate(
    ledger: &[ArtifactLedgerEntry],
    state_dir: &str,
    symbol: &str,
    left_artifact_id: &str,
    right_artifact_id: &str,
) -> Result<ArtifactDiffView> {
    let left = execution_candidate_artifact_by_id(state_dir, symbol, left_artifact_id)?;
    let right = execution_candidate_artifact_by_id(state_dir, symbol, right_artifact_id)?;
    let diff = execution_candidate_artifact_diff(&left, &right);
    let lineage_artifact_ids = artifact_lineage_path(ledger, left_artifact_id, right_artifact_id);
    Ok(ArtifactDiffView {
        kind: "execution_candidate".to_string(),
        left_artifact_id: left.artifact_id,
        right_artifact_id: right.artifact_id,
        changed_fields: diff.changed_fields,
        numeric_evidence: vec![
            format!("posterior_delta={:.4}", diff.posterior_delta),
            format!("win_probability_delta={:.4}", diff.win_probability_delta),
            format!("entry_delta={:.4}", diff.entry_delta),
        ],
        embedded_pre_bayes_evidence: artifact_embedded_pre_bayes_evidence(
            left.pre_bayes_evidence_filter.as_ref(),
            right.pre_bayes_evidence_filter.as_ref(),
            left.pre_bayes_entry_quality_bridge.as_ref(),
            right.pre_bayes_entry_quality_bridge.as_ref(),
            &left.multi_timeframe_summary,
            &right.multi_timeframe_summary,
        ),
        summary: format!("exact_duplicate={}", diff.exact_duplicate),
        cross_rule_version_summary: (left.review_rule_version != right.review_rule_version).then(
            || {
                format!(
                    "rule_version_changed:{}->{} posterior_delta={:.4} win_probability_delta={:.4}",
                    left.review_rule_version,
                    right.review_rule_version,
                    diff.posterior_delta,
                    diff.win_probability_delta
                )
            },
        ),
        lineage_artifact_ids: lineage_artifact_ids.clone(),
        lineage_numeric_evidence: artifact_lineage_numeric_evidence(ledger, &lineage_artifact_ids),
    })
}

pub fn artifact_entry_is_rule_break(
    artifact_ledger: &[ArtifactLedgerEntry],
    entry: &ArtifactLedgerEntry,
) -> bool {
    entry
        .supersedes_artifact_id
        .as_deref()
        .and_then(|parent_id| {
            artifact_ledger
                .iter()
                .find(|candidate| candidate.artifact_id == parent_id)
        })
        .map(|parent| parent.review_rule_version != entry.review_rule_version)
        .unwrap_or(false)
}

pub fn sort_artifact_entries(
    entries: &mut [ArtifactLedgerEntry],
    sort_by: &str,
    descending: bool,
) -> Result<()> {
    match sort_by.trim().to_ascii_lowercase().as_str() {
        "generated" => entries.sort_by_key(|entry| entry.generated_at),
        "quality" => entries.sort_by_key(|entry| entry.quality_score),
        "improvement" => entries.sort_by_key(artifact_improvement_score),
        "regression" => entries.sort_by_key(artifact_regression_score),
        "kind" => entries.sort_by(|a, b| a.artifact_kind.cmp(&b.artifact_kind)),
        "status" => entries.sort_by(|a, b| a.status.cmp(&b.status)),
        "version" => entries.sort_by_key(|entry| entry.version),
        other => bail!("unsupported artifact-status sort '{}'", other),
    }
    if descending {
        entries.reverse();
    }
    Ok(())
}

pub fn sort_artifact_buckets(
    buckets: &mut [(String, Vec<ArtifactLedgerEntry>)],
    bucket_order_by: &str,
    descending: bool,
) -> Result<()> {
    match bucket_order_by.trim().to_ascii_lowercase().as_str() {
        "kind" => buckets.sort_by(|a, b| a.0.cmp(&b.0)),
        "count" => buckets.sort_by_key(|(_, values)| values.len()),
        "quality" => buckets.sort_by_key(|(_, values)| {
            values
                .iter()
                .map(|entry| entry.quality_score)
                .max()
                .unwrap_or_default()
        }),
        other => bail!("unsupported artifact-status bucket-order-by '{}'", other),
    }
    if descending {
        buckets.reverse();
    }
    Ok(())
}

pub fn artifact_improvement_score(entry: &ArtifactLedgerEntry) -> i32 {
    entry.quality_score
        + if entry.promote_candidate { 50 } else { 0 }
        + if matches!(
            entry.consumption_regrade_status.as_deref(),
            Some("validated_positive")
        ) {
            25
        } else {
            0
        }
}

pub fn artifact_regression_score(entry: &ArtifactLedgerEntry) -> i32 {
    let mut score = 0;
    if entry.status == "discard" {
        score += 50;
    }
    if matches!(
        entry.consumption_regrade_status.as_deref(),
        Some("validated_negative")
    ) {
        score += 25;
    }
    score - entry.quality_score
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn parent_entry() -> ArtifactLedgerEntry {
        ArtifactLedgerEntry {
            entry_id: "parent".to_string(),
            artifact_kind: "pending_update".to_string(),
            artifact_id: "parent".to_string(),
            version: 1,
            generated_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: None,
            path: "p".to_string(),
            status: "observe".to_string(),
            promote_candidate: false,
            actionable: false,
            decision_hint: "decision_hint_unavailable".to_string(),
            review_reason: "review_reason_unavailable".to_string(),
            review_rule_version: "rules-v1".to_string(),
            top_factor_name: None,
            top_factor_action: None,
            family_scores: BTreeMap::new(),
            supersedes_artifact_id: None,
            quality_score: 50,
            consumed_by_update_run_id: None,
            consumed_at: None,
            consumed_outcome: None,
            regraded_at: None,
            consumption_regrade_status: None,
            consumption_regrade_reason: None,
        }
    }

    #[test]
    fn artifact_entry_is_rule_break_requires_parent_version_change() {
        let parent = parent_entry();
        let same = ArtifactLedgerEntry {
            artifact_id: "same".to_string(),
            supersedes_artifact_id: Some("parent".to_string()),
            review_rule_version: "rules-v1".to_string(),
            ..parent.clone()
        };
        let changed = ArtifactLedgerEntry {
            artifact_id: "changed".to_string(),
            supersedes_artifact_id: Some("parent".to_string()),
            review_rule_version: "rules-v2".to_string(),
            ..parent.clone()
        };

        assert!(!artifact_entry_is_rule_break(
            &[parent.clone(), same.clone()],
            &same
        ));
        assert!(artifact_entry_is_rule_break(
            &[parent, changed.clone()],
            &changed
        ));
    }

    #[test]
    fn artifact_status_latest_only_contract_is_latest_per_kind() {
        let entries = vec![
            ArtifactLedgerEntry {
                entry_id: "entry-old-pending".into(),
                artifact_id: "pending-1".into(),
                artifact_kind: "pending_update".into(),
                generated_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
                version: 1,
                ..ArtifactLedgerEntry::default()
            },
            ArtifactLedgerEntry {
                entry_id: "entry-new-pending".into(),
                artifact_id: "pending-2".into(),
                artifact_kind: "pending_update".into(),
                generated_at: Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap(),
                version: 2,
                ..ArtifactLedgerEntry::default()
            },
            ArtifactLedgerEntry {
                entry_id: "entry-vote".into(),
                artifact_id: "vote-1".into(),
                artifact_kind: "ensemble_vote".into(),
                generated_at: Utc.with_ymd_and_hms(2026, 1, 15, 0, 0, 0).unwrap(),
                version: 1,
                ..ArtifactLedgerEntry::default()
            },
        ];

        let latest = latest_artifact_entries_by_kind(&entries);

        assert_eq!(
            latest.len(),
            2,
            "--latest-only contract: one entry per artifact_kind"
        );
        let ids: std::collections::BTreeSet<_> = latest
            .iter()
            .map(|entry| entry.artifact_id.clone())
            .collect();
        assert!(
            ids.contains("pending-2"),
            "newer pending_update must win within its kind"
        );
        assert!(
            !ids.contains("pending-1"),
            "older pending_update must be dropped within its kind"
        );
        assert!(
            ids.contains("vote-1"),
            "single-kind entries must be preserved"
        );
    }

    #[test]
    fn artifact_status_entry_view_marks_existing_and_missing_paths() {
        let temp = tempfile::tempdir().unwrap();
        let existing = temp.path().join("artifact.json");
        std::fs::write(&existing, "{}").unwrap();

        let existing_view = artifact_status_entry_view(ArtifactLedgerEntry {
            path: existing.to_string_lossy().to_string(),
            ..ArtifactLedgerEntry::default()
        });
        assert!(existing_view.path_exists);
        assert_eq!(existing_view.path_kind, "file");

        let missing_absolute_view = artifact_status_entry_view(ArtifactLedgerEntry {
            path: temp
                .path()
                .join("missing.json")
                .to_string_lossy()
                .to_string(),
            ..ArtifactLedgerEntry::default()
        });
        assert!(!missing_absolute_view.path_exists);
        assert_eq!(missing_absolute_view.path_kind, "missing_absolute_file");

        let missing_relative_view = artifact_status_entry_view(ArtifactLedgerEntry {
            path: "state/NQ/artifact.json".to_string(),
            ..ArtifactLedgerEntry::default()
        });
        assert!(!missing_relative_view.path_exists);
        assert_eq!(missing_relative_view.path_kind, "missing_relative_file");
    }

    #[test]
    fn artifact_lineage_path_walks_parent_chain() {
        let ledger = vec![
            ArtifactLedgerEntry {
                artifact_id: "pending-1".to_string(),
                supersedes_artifact_id: None,
                ..ArtifactLedgerEntry::default()
            },
            ArtifactLedgerEntry {
                artifact_id: "pending-2".to_string(),
                supersedes_artifact_id: Some("pending-1".to_string()),
                ..ArtifactLedgerEntry::default()
            },
        ];

        let path = artifact_lineage_path(&ledger, "pending-1", "pending-2");
        assert_eq!(path, vec!["pending-1".to_string(), "pending-2".to_string()]);
    }

    #[test]
    fn artifact_lineage_numeric_evidence_summarizes_chain() {
        let ledger = vec![
            ArtifactLedgerEntry {
                artifact_id: "pending-1".to_string(),
                quality_score: 40,
                ..ArtifactLedgerEntry::default()
            },
            ArtifactLedgerEntry {
                artifact_id: "pending-2".to_string(),
                quality_score: 80,
                consumed_by_update_run_id: Some("update:1".to_string()),
                ..ArtifactLedgerEntry::default()
            },
        ];

        let evidence = artifact_lineage_numeric_evidence(
            &ledger,
            &["pending-1".to_string(), "pending-2".to_string()],
        );
        assert!(evidence.contains(&"lineage_steps=2".to_string()));
        assert!(evidence.contains(&"lineage_quality_delta=40".to_string()));
        assert!(evidence.contains(&"lineage_consumed_entries=1".to_string()));
    }

    #[test]
    fn filter_artifact_lineage_summaries_respects_exclusive_filters() {
        let summaries = vec![
            ArtifactLineageSummary {
                latest_artifact_id: "a".to_string(),
                conclusion: "improving".to_string(),
                ..ArtifactLineageSummary::default()
            },
            ArtifactLineageSummary {
                latest_artifact_id: "b".to_string(),
                conclusion: "deteriorating".to_string(),
                review_rule_break_count: 1,
                ..ArtifactLineageSummary::default()
            },
        ];

        let improving =
            filter_artifact_lineage_summaries(summaries.clone(), true, false, false).unwrap();
        assert_eq!(improving.len(), 1);
        assert_eq!(improving[0].latest_artifact_id, "a");

        let rule_breaks =
            filter_artifact_lineage_summaries(summaries.clone(), false, false, true).unwrap();
        assert_eq!(rule_breaks.len(), 1);
        assert_eq!(rule_breaks[0].latest_artifact_id, "b");

        let err = filter_artifact_lineage_summaries(summaries, true, true, false).unwrap_err();
        assert!(err.to_string().contains("at most one"));
    }

    #[test]
    fn artifact_lineage_view_builds_supersedes_and_consumed_edges() {
        let ledger = vec![
            ArtifactLedgerEntry {
                artifact_id: "pending-1".to_string(),
                ..ArtifactLedgerEntry::default()
            },
            ArtifactLedgerEntry {
                artifact_id: "pending-2".to_string(),
                supersedes_artifact_id: Some("pending-1".to_string()),
                consumed_by_update_run_id: Some("update-1".to_string()),
                ..ArtifactLedgerEntry::default()
            },
        ];

        let view = artifact_lineage_view("NQ", &ledger, Some("pending-2".to_string()));
        assert_eq!(view.symbol, "NQ");
        assert_eq!(view.nodes.len(), 2);
        assert!(view
            .edges
            .iter()
            .any(|edge| edge.from == "pending-1" && edge.to == "pending-2"));
        assert!(view
            .edges
            .iter()
            .any(|edge| edge.from == "pending-2" && edge.to == "update-1"));
    }

    #[test]
    fn artifact_diff_view_includes_lineage_chain() {
        let temp = tempfile::tempdir().unwrap();
        crate::state::append_artifact_ledger_entry(
            temp.path(),
            "NQ",
            ArtifactLedgerEntry {
                entry_id: "ledger:pending-1".to_string(),
                artifact_kind: "pending_update".to_string(),
                artifact_id: "pending-1".to_string(),
                version: 1,
                generated_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                symbol: "NQ".to_string(),
                source_phase: "analyze".to_string(),
                source_run_id: Some("analyze:1".to_string()),
                path: "state/NQ/pending_update_feedback.json".to_string(),
                status: "observe".to_string(),
                promote_candidate: false,
                actionable: true,
                decision_hint: "hint-1".to_string(),
                review_reason: "observe".to_string(),
                review_rule_version: "r1".to_string(),
                top_factor_name: Some("trend_momentum".to_string()),
                top_factor_action: Some("tune".to_string()),
                family_scores: BTreeMap::from([("trend_momentum".to_string(), 0.45)]),
                supersedes_artifact_id: None,
                quality_score: 40,
                consumed_by_update_run_id: None,
                consumed_at: None,
                consumed_outcome: None,
                regraded_at: None,
                consumption_regrade_status: None,
                consumption_regrade_reason: None,
            },
        )
        .unwrap();
        crate::state::append_artifact_ledger_entry(
            temp.path(),
            "NQ",
            ArtifactLedgerEntry {
                entry_id: "ledger:pending-2".to_string(),
                artifact_kind: "pending_update".to_string(),
                artifact_id: "pending-2".to_string(),
                version: 2,
                generated_at: Utc.with_ymd_and_hms(2024, 1, 1, 1, 0, 0).unwrap(),
                symbol: "NQ".to_string(),
                source_phase: "analyze".to_string(),
                source_run_id: Some("analyze:2".to_string()),
                path: "state/NQ/pending_update_feedback.json".to_string(),
                status: "promote_latest".to_string(),
                promote_candidate: true,
                actionable: true,
                decision_hint: "hint-2".to_string(),
                review_reason: "promote".to_string(),
                review_rule_version: "r1".to_string(),
                top_factor_name: Some("trend_momentum".to_string()),
                top_factor_action: Some("keep".to_string()),
                family_scores: BTreeMap::from([("trend_momentum".to_string(), 0.72)]),
                supersedes_artifact_id: Some("pending-1".to_string()),
                quality_score: 80,
                consumed_by_update_run_id: None,
                consumed_at: None,
                consumed_outcome: None,
                regraded_at: None,
                consumption_regrade_status: None,
                consumption_regrade_reason: None,
            },
        )
        .unwrap();
        crate::state::append_pending_update_artifact_history(
            temp.path(),
            "NQ",
            PendingUpdateArtifact {
                artifact_id: "pending-1".to_string(),
                version: 1,
                entry_quality: "medium".to_string(),
                factor_alignment: "mixed".to_string(),
                factor_uncertainty: "low".to_string(),
                selected_win_probability: 0.50,
                top_factor_score: 0.45,
                avg_family_score: 0.45,
                pre_bayes_evidence_filter: Some(PreBayesEvidenceFilter {
                    gating_status: "observe_only".to_string(),
                    policy: crate::state::PreBayesEvidencePolicy {
                        version: "policy-a".to_string(),
                        ..crate::state::PreBayesEvidencePolicy::default()
                    },
                    filtered_multi_timeframe_resonance_label: "mixed".to_string(),
                    ..PreBayesEvidenceFilter::default()
                }),
                pre_bayes_entry_quality_bridge: Some(PreBayesEntryQualityBridge {
                    selected_entry_quality: BTreeMap::from([("medium".to_string(), 0.7)]),
                    ..PreBayesEntryQualityBridge::default()
                }),
                multi_timeframe_summary: vec!["higher_timeframe_direction_bias=bullish".to_string()],
                ..PendingUpdateArtifact::default()
            },
        )
        .unwrap();
        crate::state::append_pending_update_artifact_history(
            temp.path(),
            "NQ",
            PendingUpdateArtifact {
                artifact_id: "pending-2".to_string(),
                version: 2,
                entry_quality: "high".to_string(),
                factor_alignment: "bullish".to_string(),
                factor_uncertainty: "low".to_string(),
                selected_win_probability: 0.70,
                top_factor_score: 0.72,
                avg_family_score: 0.72,
                pre_bayes_evidence_filter: Some(PreBayesEvidenceFilter {
                    gating_status: "pass_hard".to_string(),
                    policy: crate::state::PreBayesEvidencePolicy {
                        version: "policy-b".to_string(),
                        ..crate::state::PreBayesEvidencePolicy::default()
                    },
                    filtered_multi_timeframe_resonance_label: "aligned".to_string(),
                    ..PreBayesEvidenceFilter::default()
                }),
                pre_bayes_entry_quality_bridge: Some(PreBayesEntryQualityBridge {
                    selected_entry_quality: BTreeMap::from([("high".to_string(), 0.8)]),
                    long_signal_probability: 0.7,
                    short_signal_probability: 0.3,
                    ..PreBayesEntryQualityBridge::default()
                }),
                multi_timeframe_summary: vec!["higher_timeframe_direction_bias=bearish".to_string()],
                ..PendingUpdateArtifact::default()
            },
        )
        .unwrap();

        let ledger = crate::state::load_artifact_ledger(temp.path(), "NQ").unwrap();
        let diff = artifact_diff_view_for_pending_update(
            &ledger,
            temp.path().to_str().unwrap(),
            "NQ",
            "pending-1",
            "pending-2",
        )
        .unwrap();

        assert_eq!(diff.lineage_artifact_ids, vec!["pending-1", "pending-2"]);
        assert!(!diff.lineage_numeric_evidence.is_empty());
        assert!(!diff.embedded_pre_bayes_evidence.is_empty());
        assert!(diff
            .embedded_pre_bayes_evidence
            .iter()
            .any(|item| item.contains("pre_bayes_policy_version:policy-a->policy-b")));
    }

    #[test]
    fn artifact_lineage_summary_counts_embedded_pre_bayes_changes() {
        let ledger = vec![
            ArtifactLedgerEntry {
                artifact_id: "pending-1".to_string(),
                artifact_kind: "pending_update".to_string(),
                version: 1,
                generated_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                ..ArtifactLedgerEntry::default()
            },
            ArtifactLedgerEntry {
                artifact_id: "pending-2".to_string(),
                artifact_kind: "pending_update".to_string(),
                version: 2,
                supersedes_artifact_id: Some("pending-1".to_string()),
                generated_at: Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
                ..ArtifactLedgerEntry::default()
            },
        ];
        let summaries = build_artifact_lineage_summaries_with_embedded_snapshots(
            &ledger,
            &[
                PendingUpdateArtifact {
                    artifact_id: "pending-1".to_string(),
                    pre_bayes_evidence_filter: Some(PreBayesEvidenceFilter {
                        gating_status: "observe_only".to_string(),
                        filtered_multi_timeframe_direction_bias: "bullish".to_string(),
                        policy: crate::state::PreBayesEvidencePolicy {
                            version: "policy-a".to_string(),
                            ..crate::state::PreBayesEvidencePolicy::default()
                        },
                        ..PreBayesEvidenceFilter::default()
                    }),
                    ..PendingUpdateArtifact::default()
                },
                PendingUpdateArtifact {
                    artifact_id: "pending-2".to_string(),
                    pre_bayes_evidence_filter: Some(PreBayesEvidenceFilter {
                        gating_status: "pass_hard".to_string(),
                        filtered_multi_timeframe_direction_bias: "bearish".to_string(),
                        policy: crate::state::PreBayesEvidencePolicy {
                            version: "policy-b".to_string(),
                            ..crate::state::PreBayesEvidencePolicy::default()
                        },
                        ..PreBayesEvidenceFilter::default()
                    }),
                    ..PendingUpdateArtifact::default()
                },
            ],
            &[],
        );

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].embedded_pre_bayes_change_count, 1);
        assert_eq!(summaries[0].latest_pre_bayes_gate_status, "pass_hard");
        assert_eq!(
            summaries[0].latest_pre_bayes_multi_timeframe_direction_bias,
            "bearish"
        );
    }
}
