use anyhow::{anyhow, Result};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use crate::application::backtest::{
    pre_bayes_entry_quality_bridge_diff, pre_bayes_soft_evidence_diff,
};
use crate::config::shell_quote;
use crate::state::{
    load_artifact_ledger, load_factor_autoresearch_attempts, load_factor_autoresearch_sessions,
    load_state_or_default, load_workflow_snapshot, recommended_next_command_meta, AnalyzeRunRecord,
    ArtifactLedgerEntry, BacktestRunRecord, FactorAutoresearchAttempt, FactorAutoresearchSession,
    FactorMutationRunRecord, RecommendedNextCommandKind, ResearchRunRecord, ANALYZE_RUNS_FILE,
    BACKTEST_RUNS_FILE, FACTOR_MUTATION_RUNS_FILE, RESEARCH_RUNS_FILE,
};

#[derive(Debug, Serialize)]
pub struct ResearchVerdictReport {
    symbol: String,
    state_dir: String,
    best_known_baseline: String,
    proven_bad_regions: Vec<String>,
    current_bottleneck: String,
    recommended_next_experiment: String,
    stop_or_continue: String,
    comparison_contaminated: bool,
    contamination_reasons: Vec<String>,
    isolated_comparison_recommended: bool,
    session_objectives: Vec<String>,
    session_base_factors: Vec<String>,
    research_objectives: Vec<String>,
    research_data_paths: Vec<String>,
    paired_data_paths: Vec<String>,
    mutation_source_commands: Vec<String>,
    artifact_source_phases: Vec<String>,
    actionable_artifact_count: usize,
    top_cluster_scoreboard: Vec<String>,
    evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct EvidenceQualityBreakdownReport {
    symbol: String,
    state_dir: String,
    generated_from_run_id: Option<String>,
    policy_version: String,
    gating_status: String,
    final_evidence_quality_score: f64,
    uses_soft_evidence: bool,
    hard_pass_gap: f64,
    neutralized_gap: f64,
    base_score: f64,
    support_gap_contribution: f64,
    uncertainty_penalty: f64,
    directional_conflict_penalty: f64,
    mixed_alignment_penalty: f64,
    mtf_direction_conflict_penalty: f64,
    mtf_alignment_penalty: f64,
    mtf_alignment_bonus: f64,
    mtf_entry_penalty: f64,
    liquidity_penalty_or_bonus: f64,
    bridge_gap: Option<f64>,
    bridge_selected_entry_quality: Option<String>,
    bridge_selected_entry_quality_probability: f64,
    bridge_multi_timeframe_direction_bias: String,
    raw_multi_timeframe_alignment_score: Option<f64>,
    raw_multi_timeframe_entry_alignment_score: Option<f64>,
    filtered_multi_timeframe_alignment_score: Option<f64>,
    filtered_multi_timeframe_entry_alignment_score: Option<f64>,
    soft_evidence_divergence_count: usize,
    soft_evidence_summary: Vec<String>,
    rationale: Vec<String>,
}

pub fn workflow_next_step_view(command: &str, blocked_reason: Option<&str>) -> Value {
    let meta = recommended_next_command_meta(command);
    match meta.kind {
        RecommendedNextCommandKind::Unavailable | RecommendedNextCommandKind::Unknown => {
            serde_json::json!({
                "action_type": "none",
                "user_input_required": false,
                "blocked_reason": blocked_reason,
                "prompt": null,
                "deferred_command": null,
            })
        }
        RecommendedNextCommandKind::AskUser => serde_json::json!({
            "action_type": "ask_user_choose_historical_data",
            "user_input_required": true,
            "blocked_reason": blocked_reason.unwrap_or("user_selected_historical_data_missing"),
            "prompt": meta.prompt,
            "deferred_command": meta.executable_command,
        }),
        _ => serde_json::json!({
            "action_type": "run_command",
            "user_input_required": false,
            "blocked_reason": blocked_reason,
            "prompt": null,
            "deferred_command": meta.executable_command.unwrap_or_else(|| command.trim().to_string()),
        }),
    }
}

pub fn research_verdict_command(symbol: &str, state_dir: &str) -> Result<()> {
    let sessions = load_factor_autoresearch_sessions(state_dir, symbol)?;
    let attempts = load_factor_autoresearch_attempts(state_dir, symbol)?;
    let research_runs: Vec<ResearchRunRecord> =
        load_state_or_default(state_dir, symbol, RESEARCH_RUNS_FILE)?;
    let backtest_runs: Vec<BacktestRunRecord> =
        load_state_or_default(state_dir, symbol, BACKTEST_RUNS_FILE)?;
    let mutation_runs: Vec<FactorMutationRunRecord> =
        load_state_or_default(state_dir, symbol, FACTOR_MUTATION_RUNS_FILE)?;
    let artifact_ledger = load_artifact_ledger(state_dir, symbol)?;

    let report = build_research_verdict_report(
        symbol,
        state_dir,
        &sessions,
        &attempts,
        &research_runs,
        &backtest_runs,
        &mutation_runs,
        &artifact_ledger,
    );
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn build_research_verdict_report(
    symbol: &str,
    state_dir: &str,
    sessions: &[FactorAutoresearchSession],
    attempts: &[FactorAutoresearchAttempt],
    research_runs: &[ResearchRunRecord],
    backtest_runs: &[BacktestRunRecord],
    mutation_runs: &[FactorMutationRunRecord],
    artifact_ledger: &[ArtifactLedgerEntry],
) -> ResearchVerdictReport {
    let mut contamination_reasons = Vec::new();
    let session_objectives = sessions
        .iter()
        .map(|session| session.objective.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let session_base_factors = sessions
        .iter()
        .map(|session| session.base_factor.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let research_objectives = research_runs
        .iter()
        .map(|run| run.research_objective.clone())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let research_data_paths = research_runs
        .iter()
        .map(|run| run.data_path.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let paired_data_paths = research_runs
        .iter()
        .filter_map(|run| run.paired_data_path.clone())
        .chain(
            backtest_runs
                .iter()
                .filter_map(|run| run.paired_data_path.clone()),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mutation_source_commands = mutation_runs
        .iter()
        .map(|run| run.source_command.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let artifact_source_phases = artifact_ledger
        .iter()
        .map(|item| item.source_phase.clone())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if sessions.len() > 1 {
        if session_objectives.len() > 1 {
            contamination_reasons.push(
                "multiple_autoresearch_sessions_share_one_state_dir_with_different_objectives"
                    .to_string(),
            );
        }
        if session_base_factors.len() > 1 {
            contamination_reasons.push(
                "multiple_autoresearch_sessions_share_one_state_dir_with_different_base_factors"
                    .to_string(),
            );
        }
    }
    if attempts.len() > 1 {
        let deltas = attempts
            .iter()
            .map(|attempt| attempt.decision.score_delta)
            .collect::<Vec<_>>();
        let monotonic_up = deltas.windows(2).all(|w| w[1] >= w[0]);
        let monotonic_down = deltas.windows(2).all(|w| w[1] <= w[0]);
        if monotonic_up || monotonic_down {
            contamination_reasons
                .push("attempt_score_deltas_are_monotonic_within_shared_state".to_string());
        }
    }
    if research_objectives.len() > 1 {
        contamination_reasons
            .push("research_runs_mix_multiple_objectives_in_one_state_dir".to_string());
    }
    if research_data_paths.len() > 1 {
        contamination_reasons.push("research_runs_mix_multiple_primary_data_paths".to_string());
    }
    if paired_data_paths.len() > 1 {
        contamination_reasons.push("comparison_runs_mix_multiple_paired_data_paths".to_string());
    }
    if mutation_runs.len() > 3 && mutation_source_commands.len() > 1 {
        contamination_reasons
            .push("factor_mutation_runs_mix_multiple_sources_in_one_state_dir".to_string());
    }
    if artifact_source_phases.len() > 3 {
        contamination_reasons
            .push("artifact_ledger_contains_many_source_phases_in_one_state_dir".to_string());
    }
    let comparison_contaminated = !contamination_reasons.is_empty();
    let isolated_comparison_recommended = comparison_contaminated;

    let best_research = research_runs.iter().max_by(|a, b| {
        a.aggregate_return
            .partial_cmp(&b.aggregate_return)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let best_backtest = backtest_runs.iter().max_by(|a, b| {
        a.total_return
            .partial_cmp(&b.total_return)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let cluster_scoreboard = mutation_runs.iter().fold(
        BTreeMap::<String, (usize, f64, f64)>::new(),
        |mut acc, run| {
            let cluster = run
                .mutation_spec
                .direction_hints
                .get("cluster_jump")
                .cloned()
                .unwrap_or_else(|| "none".to_string());
            let entry = acc.entry(cluster).or_insert((0, 0.0, f64::MIN));
            entry.0 += 1;
            entry.1 += run.evaluation.score_delta;
            entry.2 = entry.2.max(run.evaluation.score_delta);
            acc
        },
    );
    let best_cluster = cluster_scoreboard.iter().max_by(|a, b| {
        a.1 .2
            .partial_cmp(&b.1 .2)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let top_cluster_scoreboard = cluster_scoreboard
        .iter()
        .map(|(cluster, (attempts_count, total_delta, best_delta))| {
            format!(
                "{} attempts={} avg_score_delta={:.3} best_score_delta={:.3}",
                cluster,
                attempts_count,
                total_delta / *attempts_count as f64,
                best_delta
            )
        })
        .collect::<Vec<_>>();

    let no_research_activity = research_runs.is_empty()
        && sessions.is_empty()
        && attempts.is_empty()
        && mutation_runs.is_empty()
        && backtest_runs.is_empty();

    let best_known_baseline = if let Some(run) = best_research {
        format!(
            "research best_factor={} aggregate_return={:.3}",
            run.best_factor
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            run.aggregate_return
        )
    } else if let Some(run) = best_backtest {
        format!(
            "backtest return={:.3} trades={}",
            run.total_return, run.trade_count
        )
    } else {
        "no_persisted_research_baseline".to_string()
    };

    let mut proven_bad_regions = attempts
        .iter()
        .flat_map(|attempt| attempt.evaluation.failure_tags.clone())
        .chain(
            mutation_runs
                .iter()
                .flat_map(|run| run.evaluation.failure_tags.clone()),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    proven_bad_regions.sort();

    let current_bottleneck = if no_research_activity {
        "no_research_runs".to_string()
    } else if proven_bad_regions
        .iter()
        .any(|tag| tag.contains("bridge_gap"))
    {
        "bridge_gap".to_string()
    } else if proven_bad_regions
        .iter()
        .any(|tag| tag.contains("pre_bayes"))
    {
        "pre_bayes_gate".to_string()
    } else if proven_bad_regions
        .iter()
        .any(|tag| tag.contains("pair_quality") || tag.contains("cross_market"))
    {
        "paired_data_quality".to_string()
    } else if comparison_contaminated {
        "comparison_contamination".to_string()
    } else if best_cluster.is_some() {
        "cluster_search_follow_up".to_string()
    } else {
        "needs_more_evidence".to_string()
    };

    let recommended_next_experiment = match current_bottleneck.as_str() {
        "no_research_runs" => format!(
            "ict-engine factor-research --symbol {} --data <cleaned-candles.json> --state-dir {}",
            shell_quote(symbol),
            shell_quote(state_dir)
        ),
        "bridge_gap" => format!(
            "ict-engine evidence-quality-breakdown --symbol {} --state-dir {}",
            shell_quote(symbol),
            shell_quote(state_dir)
        ),
        "pre_bayes_gate" => format!(
            "ict-engine workflow-status --symbol {} --state-dir {} --phase pre-bayes-policy",
            shell_quote(symbol),
            shell_quote(state_dir)
        ),
        "paired_data_quality" => format!(
            "ict-engine factor-pipeline-debug --symbol {} --data <cleaned-15m.json> --factor cross_market_smt --objective expansion_manipulation",
            shell_quote(symbol)
        ),
        "comparison_contamination" => {
            "rerun experiments in an isolated fresh state_dir before comparing results".to_string()
        }
        "cluster_search_follow_up" => {
            let cluster = best_cluster.map(|item| item.0.as_str()).unwrap_or("none");
            format!(
                "continue cluster search around cluster_jump={} with isolated state_dir",
                cluster
            )
        }
        _ => format!(
            "ict-engine factor-autoresearch-status --symbol {} --state-dir {} --latest-only",
            shell_quote(symbol),
            shell_quote(state_dir)
        ),
    };

    let stop_or_continue = if no_research_activity {
        "bootstrap_required".to_string()
    } else if comparison_contaminated {
        "pivot".to_string()
    } else if best_research
        .map(|run| run.aggregate_return >= 0.0)
        .unwrap_or(false)
        && !proven_bad_regions.is_empty()
    {
        "stop_as_local_optimum".to_string()
    } else if artifact_ledger.iter().any(|item| item.actionable) || best_cluster.is_some() {
        "continue".to_string()
    } else {
        "needs_structural_change".to_string()
    };

    let mut evidence = Vec::new();
    evidence.push(format!("autoresearch_sessions={}", sessions.len()));
    evidence.push(format!("autoresearch_attempts={}", attempts.len()));
    evidence.push(format!("research_runs={}", research_runs.len()));
    evidence.push(format!("backtest_runs={}", backtest_runs.len()));
    evidence.push(format!("factor_mutation_runs={}", mutation_runs.len()));
    evidence.push(format!("artifact_rows={}", artifact_ledger.len()));
    if let Some((cluster, (attempts_count, avg_delta, best_delta))) = best_cluster {
        evidence.push(format!(
            "best_cluster={} attempts={} avg_score_delta={:.3} best_score_delta={:.3}",
            cluster,
            attempts_count,
            avg_delta / *attempts_count as f64,
            best_delta
        ));
    }

    ResearchVerdictReport {
        symbol: symbol.to_string(),
        state_dir: state_dir.to_string(),
        best_known_baseline,
        proven_bad_regions,
        current_bottleneck,
        recommended_next_experiment,
        stop_or_continue,
        comparison_contaminated,
        contamination_reasons,
        isolated_comparison_recommended,
        session_objectives,
        session_base_factors,
        research_objectives,
        research_data_paths,
        paired_data_paths,
        mutation_source_commands,
        artifact_source_phases,
        actionable_artifact_count: artifact_ledger
            .iter()
            .filter(|item| item.actionable)
            .count(),
        top_cluster_scoreboard,
        evidence,
    }
}

pub fn evidence_quality_breakdown_command(
    symbol: &str,
    state_dir: &str,
    _refresh: bool,
) -> Result<()> {
    let snapshot = load_workflow_snapshot(state_dir, symbol)?;
    snapshot
        .latest_analyze
        .as_ref()
        .ok_or_else(|| anyhow!("no latest analyze phase found for '{}'", symbol))?;
    let latest_analyze_runs: Vec<AnalyzeRunRecord> =
        load_state_or_default(state_dir, symbol, ANALYZE_RUNS_FILE)?;
    let analyze_run = latest_analyze_runs
        .last()
        .ok_or_else(|| anyhow!("no latest analyze run found for '{}'", symbol))?;
    let report = build_evidence_quality_breakdown_report(symbol, state_dir, analyze_run);
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

pub fn build_evidence_quality_breakdown_report(
    symbol: &str,
    state_dir: &str,
    analyze_run: &AnalyzeRunRecord,
) -> EvidenceQualityBreakdownReport {
    let filter = &analyze_run.pre_bayes_evidence_filter;
    let bridge = &analyze_run.pre_bayes_entry_quality_bridge;
    let policy = &filter.policy;
    let bridge_diff = pre_bayes_entry_quality_bridge_diff(bridge);
    let soft_evidence_diff = pre_bayes_soft_evidence_diff(filter);
    let support_gap = policy.min_directional_support_gap.clamp(0.0, 0.5);
    let base_score = 0.55;
    let support_gap_contribution = support_gap * 0.50;
    let uncertainty_penalty = if filter.filtered_factor_uncertainty == "high" {
        policy.high_uncertainty_threshold * 0.35
    } else {
        0.0
    };
    let directional_conflict_penalty = if filter
        .conflict_flags
        .iter()
        .any(|flag| flag == "directional_conflict")
    {
        policy.directional_conflict_penalty
    } else {
        0.0
    };
    let mixed_alignment_penalty = if filter.filtered_factor_alignment == "mixed" {
        policy.mixed_alignment_penalty
    } else {
        0.0
    };
    let mtf_direction_conflict_penalty = if filter
        .conflict_flags
        .iter()
        .any(|flag| flag == "multi_timeframe_direction_conflict")
    {
        policy.multi_timeframe_direction_conflict_penalty
    } else {
        0.0
    };
    let mtf_alignment_penalty = if filter
        .conflict_flags
        .iter()
        .any(|flag| flag == "multi_timeframe_alignment_weak")
    {
        policy.multi_timeframe_alignment_penalty
    } else {
        0.0
    };
    let mtf_alignment_bonus = if mtf_alignment_penalty == 0.0
        && filter
            .raw_multi_timeframe_alignment_score
            .map(|score| score >= policy.min_multi_timeframe_alignment_score)
            .unwrap_or(false)
    {
        policy.multi_timeframe_alignment_bonus
    } else {
        0.0
    };
    let mtf_entry_penalty = if filter
        .conflict_flags
        .iter()
        .any(|flag| flag == "multi_timeframe_entry_alignment_weak")
    {
        policy.multi_timeframe_entry_penalty
    } else {
        0.0
    };
    let liquidity_penalty_or_bonus = if filter.filtered_liquidity_context_label == "hostile" {
        -policy.hostile_liquidity_penalty
    } else if filter.filtered_liquidity_context_label == "favorable" {
        policy.favorable_liquidity_bonus
    } else {
        0.0
    };
    let hard_pass_gap = filter.evidence_quality_score - policy.hard_pass_quality_threshold;
    let neutralized_gap = filter.evidence_quality_score - policy.neutralized_quality_threshold;
    let bridge_gap = Some(bridge_diff.long_short_signal_probability_gap);
    let mut rationale = vec![
        format!("raw_market_regime_label={}", filter.raw_market_regime_label),
        format!(
            "filtered_market_regime_label={}",
            filter.filtered_market_regime_label
        ),
        format!(
            "raw_liquidity_context_label={}",
            filter.raw_liquidity_context_label
        ),
        format!(
            "filtered_liquidity_context_label={}",
            filter.filtered_liquidity_context_label
        ),
        format!("raw_factor_alignment_label={}", filter.raw_factor_alignment),
        format!(
            "filtered_factor_alignment_label={}",
            filter.filtered_factor_alignment
        ),
        format!(
            "raw_multi_timeframe_direction_bias={}",
            filter.raw_multi_timeframe_direction_bias
        ),
        format!(
            "filtered_multi_timeframe_direction_bias={}",
            filter.filtered_multi_timeframe_direction_bias
        ),
        format!("conflict_flags={}", filter.conflict_flags.join(",")),
    ];
    rationale.extend(filter.rationale.iter().take(5).cloned());
    EvidenceQualityBreakdownReport {
        symbol: symbol.to_string(),
        state_dir: state_dir.to_string(),
        generated_from_run_id: Some(analyze_run.run_id.clone()),
        policy_version: policy.version.clone(),
        gating_status: filter.gating_status.clone(),
        final_evidence_quality_score: filter.evidence_quality_score,
        uses_soft_evidence: filter.uses_soft_evidence,
        hard_pass_gap,
        neutralized_gap,
        base_score,
        support_gap_contribution,
        uncertainty_penalty,
        directional_conflict_penalty,
        mixed_alignment_penalty,
        mtf_direction_conflict_penalty,
        mtf_alignment_penalty,
        mtf_alignment_bonus,
        mtf_entry_penalty,
        liquidity_penalty_or_bonus,
        bridge_gap,
        bridge_selected_entry_quality: bridge_diff.selected_entry_quality,
        bridge_selected_entry_quality_probability: bridge_diff.selected_entry_quality_probability,
        bridge_multi_timeframe_direction_bias: bridge_diff.multi_timeframe_direction_bias,
        raw_multi_timeframe_alignment_score: filter.raw_multi_timeframe_alignment_score,
        raw_multi_timeframe_entry_alignment_score: filter.raw_multi_timeframe_entry_alignment_score,
        filtered_multi_timeframe_alignment_score: filter.filtered_multi_timeframe_alignment_score,
        filtered_multi_timeframe_entry_alignment_score: filter
            .filtered_multi_timeframe_entry_alignment_score,
        soft_evidence_divergence_count: soft_evidence_diff
            .iter()
            .filter(|item| item.diverges_from_filtered_state)
            .count(),
        soft_evidence_summary: soft_evidence_diff
            .into_iter()
            .map(|item| {
                format!(
                    "{} filtered={} dominant={:?} p={:.3} diverges={}",
                    item.node,
                    item.filtered_state,
                    item.dominant_soft_state,
                    item.dominant_soft_probability,
                    item.diverges_from_filtered_state
                )
            })
            .collect(),
        rationale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::BTreeMap;

    #[test]
    fn research_verdict_requires_bootstrap_when_no_research_runs_exist() {
        let report = build_research_verdict_report("DEMO", "state", &[], &[], &[], &[], &[], &[]);
        let verdict = serde_json::to_value(&report).expect("serialize verdict");
        assert_eq!(verdict["stop_or_continue"], "bootstrap_required");
        assert_eq!(verdict["current_bottleneck"], "no_research_runs");
        assert!(verdict["recommended_next_experiment"]
            .as_str()
            .unwrap()
            .contains("ict-engine factor-research"));
    }

    #[test]
    fn research_verdict_flags_mixed_data_paths_and_artifact_sources_as_contamination() {
        let sessions = vec![
            FactorAutoresearchSession {
                objective: "expansion_manipulation".to_string(),
                base_factor: "structure_ict".to_string(),
                ..FactorAutoresearchSession::default()
            },
            FactorAutoresearchSession {
                objective: "cross_market".to_string(),
                base_factor: "cross_market_smt".to_string(),
                ..FactorAutoresearchSession::default()
            },
        ];
        let research_runs = vec![
            ResearchRunRecord {
                data_path: "a.json".to_string(),
                paired_data_path: Some("es.json".to_string()),
                research_objective: "expansion_manipulation".to_string(),
                ..ResearchRunRecord::default()
            },
            ResearchRunRecord {
                data_path: "b.json".to_string(),
                paired_data_path: Some("ym.json".to_string()),
                research_objective: "cross_market".to_string(),
                ..ResearchRunRecord::default()
            },
        ];
        let mutation_runs = vec![
            FactorMutationRunRecord {
                source_command: "factor-research".to_string(),
                ..FactorMutationRunRecord::default()
            },
            FactorMutationRunRecord {
                source_command: "factor-autoresearch".to_string(),
                ..FactorMutationRunRecord::default()
            },
            FactorMutationRunRecord {
                source_command: "factor-autoresearch".to_string(),
                ..FactorMutationRunRecord::default()
            },
            FactorMutationRunRecord {
                source_command: "cluster-jump".to_string(),
                ..FactorMutationRunRecord::default()
            },
        ];
        let artifact_ledger = vec![
            ArtifactLedgerEntry {
                source_phase: "analyze".to_string(),
                actionable: true,
                ..ArtifactLedgerEntry::default()
            },
            ArtifactLedgerEntry {
                source_phase: "research".to_string(),
                actionable: true,
                ..ArtifactLedgerEntry::default()
            },
            ArtifactLedgerEntry {
                source_phase: "backtest".to_string(),
                actionable: false,
                ..ArtifactLedgerEntry::default()
            },
            ArtifactLedgerEntry {
                source_phase: "update".to_string(),
                actionable: false,
                ..ArtifactLedgerEntry::default()
            },
        ];

        let report = build_research_verdict_report(
            "DEMO",
            "state",
            &sessions,
            &[],
            &research_runs,
            &[],
            &mutation_runs,
            &artifact_ledger,
        );
        let verdict = serde_json::to_value(&report).expect("serialize verdict");
        assert_eq!(verdict["comparison_contaminated"], true);
        assert_eq!(verdict["isolated_comparison_recommended"], true);
        assert!(verdict["contamination_reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "research_runs_mix_multiple_objectives_in_one_state_dir"));
        assert!(verdict["contamination_reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "research_runs_mix_multiple_primary_data_paths"));
        assert_eq!(verdict["actionable_artifact_count"], 2);
    }

    #[test]
    fn evidence_quality_breakdown_surfaces_policy_bridge_and_soft_evidence_fields() {
        let mut analyze_run = AnalyzeRunRecord {
            run_id: "analyze:demo:1".to_string(),
            timestamp: Utc::now(),
            ..AnalyzeRunRecord::default()
        };
        analyze_run.pre_bayes_evidence_filter.policy.version = "policy-v3".to_string();
        analyze_run.pre_bayes_evidence_filter.gating_status = "pass_neutralized".to_string();
        analyze_run.pre_bayes_evidence_filter.evidence_quality_score = 0.72;
        analyze_run.pre_bayes_evidence_filter.uses_soft_evidence = true;
        analyze_run
            .pre_bayes_evidence_filter
            .raw_market_regime_label = "bull".to_string();
        analyze_run
            .pre_bayes_evidence_filter
            .filtered_market_regime_label = "range".to_string();
        analyze_run
            .pre_bayes_evidence_filter
            .raw_liquidity_context_label = "favorable".to_string();
        analyze_run
            .pre_bayes_evidence_filter
            .filtered_liquidity_context_label = "neutral".to_string();
        analyze_run.pre_bayes_evidence_filter.raw_factor_alignment = "bullish".to_string();
        analyze_run
            .pre_bayes_evidence_filter
            .filtered_factor_alignment = "mixed".to_string();
        analyze_run
            .pre_bayes_evidence_filter
            .raw_multi_timeframe_direction_bias = "bull".to_string();
        analyze_run
            .pre_bayes_evidence_filter
            .filtered_multi_timeframe_direction_bias = "neutral".to_string();
        analyze_run
            .pre_bayes_evidence_filter
            .raw_multi_timeframe_alignment_score = Some(0.81);
        analyze_run
            .pre_bayes_evidence_filter
            .raw_multi_timeframe_entry_alignment_score = Some(0.43);
        analyze_run
            .pre_bayes_evidence_filter
            .filtered_multi_timeframe_alignment_score = Some(0.81);
        analyze_run
            .pre_bayes_evidence_filter
            .filtered_multi_timeframe_entry_alignment_score = Some(0.43);
        analyze_run
            .pre_bayes_evidence_filter
            .filtered_factor_uncertainty = "high".to_string();
        analyze_run.pre_bayes_evidence_filter.conflict_flags = vec![
            "directional_conflict".to_string(),
            "multi_timeframe_direction_conflict".to_string(),
        ];
        analyze_run.pre_bayes_evidence_filter.rationale =
            vec!["soft evidence forced observe-only review".to_string()];
        analyze_run
            .pre_bayes_evidence_filter
            .soft_market_regime_distribution =
            BTreeMap::from([("range".to_string(), 0.7), ("bull".to_string(), 0.3)]);
        analyze_run
            .pre_bayes_evidence_filter
            .soft_liquidity_context_distribution =
            BTreeMap::from([("neutral".to_string(), 0.8), ("favorable".to_string(), 0.2)]);
        analyze_run
            .pre_bayes_evidence_filter
            .soft_factor_alignment_distribution =
            BTreeMap::from([("bullish".to_string(), 0.8), ("mixed".to_string(), 0.2)]);
        analyze_run
            .pre_bayes_evidence_filter
            .soft_factor_uncertainty_distribution =
            BTreeMap::from([("high".to_string(), 0.9), ("low".to_string(), 0.1)]);
        analyze_run
            .pre_bayes_evidence_filter
            .soft_multi_timeframe_resonance_distribution = BTreeMap::from([
            ("dislocated".to_string(), 0.65),
            ("aligned".to_string(), 0.35),
        ]);
        analyze_run
            .pre_bayes_entry_quality_bridge
            .long_signal_probability = 0.62;
        analyze_run
            .pre_bayes_entry_quality_bridge
            .short_signal_probability = 0.31;
        analyze_run
            .pre_bayes_entry_quality_bridge
            .selected_entry_quality =
            BTreeMap::from([("medium".to_string(), 0.7), ("high".to_string(), 0.3)]);
        analyze_run
            .pre_bayes_entry_quality_bridge
            .multi_timeframe_direction_bias = "bull".to_string();
        analyze_run
            .pre_bayes_entry_quality_bridge
            .multi_timeframe_alignment_score = Some(0.81);
        analyze_run
            .pre_bayes_entry_quality_bridge
            .multi_timeframe_entry_alignment_score = Some(0.43);

        let report = build_evidence_quality_breakdown_report("DEMO", "state", &analyze_run);
        let value = serde_json::to_value(report).expect("serialize report");
        assert_eq!(value["policy_version"], "policy-v3");
        assert_eq!(value["uses_soft_evidence"], true);
        assert_eq!(value["bridge_selected_entry_quality"], "medium");
        assert_eq!(value["bridge_multi_timeframe_direction_bias"], "bull");
        assert_eq!(value["soft_evidence_divergence_count"], 2);
        assert!(value["soft_evidence_summary"].as_array().unwrap().len() >= 5);
    }
}
