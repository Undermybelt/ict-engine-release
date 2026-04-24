use anyhow::{anyhow, bail, Result};
use chrono::Utc;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

use crate::application::decision_utils::parse_research_objective;
use crate::application::factor_lifecycle::{
    build_factor_autoresearch_status_surface, build_hint_effectiveness_summary,
    compare_hint_effectiveness, factor_mutation_direction_hint_summary,
    factor_mutation_recommended_focus, factor_mutation_step_size_hint_summary,
    next_mutation_spec_template, sync_factor_autoresearch_experiments_tsv,
    sync_factor_autoresearch_retrospective, FactorMutationPerFactorHintSummary,
};
use crate::config::shell_quote;
use crate::state::{
    append_factor_autoresearch_attempt, load_factor_autoresearch_attempts,
    load_factor_autoresearch_sessions, load_state_or_default, migrate_ensemble_executor_scorecards,
    save_factor_autoresearch_final_summary, save_factor_autoresearch_live_snapshot,
    save_factor_autoresearch_sessions, FactorAutoresearchAttempt, FactorAutoresearchDecision,
    FactorAutoresearchLiveSnapshot, FactorAutoresearchSession, FactorAutoresearchSummary,
    FactorMutationEvaluation, FactorMutationRunRecord, FactorMutationSpec,
    FACTOR_MUTATION_RUNS_FILE,
};

#[derive(Debug, Serialize)]
pub struct FactorMutationFailureCluster {
    tag: String,
    count: usize,
    latest_mutation_id: Option<String>,
    average_score_delta: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FactorMutationSourceSummary {
    source_command: String,
    total_runs: usize,
    accepted_runs: usize,
    latest_mutation_id: Option<String>,
    average_score_delta: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FactorMutationReasonSummary {
    reason: String,
    count: usize,
    markets: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FactorMutationMarketSummary {
    market: String,
    count: usize,
    reasons: Vec<String>,
}

fn factor_mutation_research_command(symbol: &str, data: &str, state_dir: &str) -> String {
    format!(
        "ict-engine factor-research --symbol {} --data {} --state-dir {}",
        shell_quote(symbol),
        shell_quote(data),
        shell_quote(state_dir)
    )
}

pub fn factor_mutation_status_command(
    symbol: &str,
    state_dir: &str,
    source_command: Option<&str>,
    latest_only: bool,
    accepted_only: bool,
    bucket_by_source: bool,
    limit: Option<usize>,
) -> Result<()> {
    let mut runs: Vec<FactorMutationRunRecord> =
        load_state_or_default(state_dir, symbol, FACTOR_MUTATION_RUNS_FILE)?;
    if let Some(source_command) = source_command {
        runs.retain(|run| run.source_command == source_command);
    }
    runs.sort_by_key(|run| run.timestamp);
    runs.reverse();
    if latest_only {
        runs.truncate(1);
    }
    if accepted_only {
        runs.retain(|run| run.evaluation.accepted);
    }
    if let Some(limit) = limit {
        runs.truncate(limit);
    }
    let all_runs: Vec<FactorMutationRunRecord> =
        load_state_or_default(state_dir, symbol, FACTOR_MUTATION_RUNS_FILE)?;
    let all_runs = all_runs
        .into_iter()
        .filter(|run| {
            source_command
                .map(|expected| run.source_command == expected)
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    let mut failure_tag_counts = BTreeMap::<String, usize>::new();
    let mut regression_reason_counts = BTreeMap::<String, usize>::new();
    let mut regression_reason_markets = BTreeMap::<String, BTreeSet<String>>::new();
    let mut market_reason_counts = BTreeMap::<String, usize>::new();
    let mut market_reasons = BTreeMap::<String, BTreeSet<String>>::new();
    let mut direction_hint_deltas = BTreeMap::<String, Vec<f64>>::new();
    let mut direction_hint_accepts = BTreeMap::<String, usize>::new();
    let mut step_hint_deltas = BTreeMap::<String, Vec<f64>>::new();
    let mut step_hint_accepts = BTreeMap::<String, usize>::new();
    let mut per_factor_direction_hint_deltas =
        BTreeMap::<String, BTreeMap<String, Vec<f64>>>::new();
    let mut per_factor_direction_hint_accepts = BTreeMap::<String, BTreeMap<String, usize>>::new();
    let mut per_factor_step_hint_deltas = BTreeMap::<String, BTreeMap<String, Vec<f64>>>::new();
    let mut per_factor_step_hint_accepts = BTreeMap::<String, BTreeMap<String, usize>>::new();
    let mut cluster_deltas = BTreeMap::<String, Vec<f64>>::new();
    let mut cluster_latest = BTreeMap::<String, String>::new();
    for run in &all_runs {
        for tag in &run.evaluation.failure_tags {
            *failure_tag_counts.entry(tag.clone()).or_default() += 1;
            cluster_deltas
                .entry(tag.clone())
                .or_default()
                .push(run.evaluation.score_delta);
            cluster_latest.insert(tag.clone(), run.evaluation.mutation_id.clone());
        }
        for (market, reasons) in &run.evaluation.metrics_after.regression_reasons_by_market {
            *market_reason_counts.entry(market.clone()).or_default() += 1;
            for reason in reasons {
                *regression_reason_counts.entry(reason.clone()).or_default() += 1;
                regression_reason_markets
                    .entry(reason.clone())
                    .or_default()
                    .insert(market.clone());
                market_reasons
                    .entry(market.clone())
                    .or_default()
                    .insert(reason.clone());
            }
        }
        for (parameter, hint) in &run.mutation_spec.direction_hints {
            let label = format!("{}:{}", parameter, hint);
            direction_hint_deltas
                .entry(label.clone())
                .or_default()
                .push(run.evaluation.score_delta);
            per_factor_direction_hint_deltas
                .entry(run.mutation_spec.base_factor.clone())
                .or_default()
                .entry(label.clone())
                .or_default()
                .push(run.evaluation.score_delta);
            if run.evaluation.accepted {
                *direction_hint_accepts.entry(label.clone()).or_default() += 1;
                *per_factor_direction_hint_accepts
                    .entry(run.mutation_spec.base_factor.clone())
                    .or_default()
                    .entry(label)
                    .or_default() += 1;
            }
        }
        for (parameter, step) in &run.mutation_spec.step_size_hints {
            let label = format!("{}:{:.4}", parameter, step);
            step_hint_deltas
                .entry(label.clone())
                .or_default()
                .push(run.evaluation.score_delta);
            per_factor_step_hint_deltas
                .entry(run.mutation_spec.base_factor.clone())
                .or_default()
                .entry(label.clone())
                .or_default()
                .push(run.evaluation.score_delta);
            if run.evaluation.accepted {
                *step_hint_accepts.entry(label.clone()).or_default() += 1;
                *per_factor_step_hint_accepts
                    .entry(run.mutation_spec.base_factor.clone())
                    .or_default()
                    .entry(label)
                    .or_default() += 1;
            }
        }
    }
    let mut failure_clusters = failure_tag_counts
        .iter()
        .map(|(tag, count)| FactorMutationFailureCluster {
            tag: tag.clone(),
            count: *count,
            latest_mutation_id: cluster_latest.get(tag).cloned(),
            average_score_delta: cluster_deltas
                .get(tag)
                .map(|values| values.iter().sum::<f64>() / values.len() as f64)
                .unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    failure_clusters.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.tag.cmp(&b.tag)));
    let mut runs_by_source = BTreeMap::<String, Vec<&FactorMutationRunRecord>>::new();
    for run in &all_runs {
        runs_by_source
            .entry(run.source_command.clone())
            .or_default()
            .push(run);
    }
    let mut source_summaries = runs_by_source
        .into_iter()
        .map(
            |(source_command, grouped_runs)| FactorMutationSourceSummary {
                source_command,
                total_runs: grouped_runs.len(),
                accepted_runs: grouped_runs
                    .iter()
                    .filter(|run| run.evaluation.accepted)
                    .count(),
                latest_mutation_id: grouped_runs
                    .iter()
                    .max_by_key(|run| run.timestamp)
                    .map(|run| run.evaluation.mutation_id.clone()),
                average_score_delta: if grouped_runs.is_empty() {
                    0.0
                } else {
                    grouped_runs
                        .iter()
                        .map(|run| run.evaluation.score_delta)
                        .sum::<f64>()
                        / grouped_runs.len() as f64
                },
            },
        )
        .collect::<Vec<_>>();
    source_summaries.sort_by(|a, b| {
        b.total_runs
            .cmp(&a.total_runs)
            .then_with(|| a.source_command.cmp(&b.source_command))
    });
    let mut regression_reason_summaries = regression_reason_counts
        .into_iter()
        .map(|(reason, count)| FactorMutationReasonSummary {
            markets: regression_reason_markets
                .remove(&reason)
                .map(|items| items.into_iter().collect::<Vec<_>>())
                .unwrap_or_default(),
            reason,
            count,
        })
        .collect::<Vec<_>>();
    regression_reason_summaries
        .sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.reason.cmp(&b.reason)));
    let mut market_regression_summaries = market_reason_counts
        .into_iter()
        .map(|(market, count)| FactorMutationMarketSummary {
            reasons: market_reasons
                .remove(&market)
                .map(|items| items.into_iter().collect::<Vec<_>>())
                .unwrap_or_default(),
            market,
            count,
        })
        .collect::<Vec<_>>();
    market_regression_summaries
        .sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.market.cmp(&b.market)));
    let mut direction_hint_effectiveness = direction_hint_deltas
        .into_iter()
        .map(|(hint, deltas)| {
            build_hint_effectiveness_summary(
                &hint,
                &deltas,
                direction_hint_accepts
                    .get(&hint)
                    .copied()
                    .unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>();
    direction_hint_effectiveness.sort_by(|a, b| compare_hint_effectiveness(b, a));
    let mut step_size_hint_effectiveness = step_hint_deltas
        .into_iter()
        .map(|(hint, deltas)| {
            build_hint_effectiveness_summary(
                &hint,
                &deltas,
                step_hint_accepts.get(&hint).copied().unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>();
    step_size_hint_effectiveness.sort_by(|a, b| compare_hint_effectiveness(b, a));
    let mut per_factor_hint_effectiveness = per_factor_direction_hint_deltas
        .into_iter()
        .map(|(base_factor, direction_entries)| {
            let mut direction_hint_effectiveness = direction_entries
                .into_iter()
                .map(|(hint, deltas)| {
                    let accepted_runs = per_factor_direction_hint_accepts
                        .get(&base_factor)
                        .and_then(|entries| entries.get(&hint))
                        .copied()
                        .unwrap_or_default();
                    build_hint_effectiveness_summary(&hint, &deltas, accepted_runs)
                })
                .collect::<Vec<_>>();
            direction_hint_effectiveness.sort_by(|a, b| compare_hint_effectiveness(b, a));
            let mut step_size_hint_effectiveness = per_factor_step_hint_deltas
                .get(&base_factor)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|(hint, deltas)| {
                    let accepted_runs = per_factor_step_hint_accepts
                        .get(&base_factor)
                        .and_then(|entries| entries.get(&hint))
                        .copied()
                        .unwrap_or_default();
                    build_hint_effectiveness_summary(&hint, &deltas, accepted_runs)
                })
                .collect::<Vec<_>>();
            step_size_hint_effectiveness.sort_by(|a, b| compare_hint_effectiveness(b, a));
            FactorMutationPerFactorHintSummary {
                base_factor,
                direction_hint_effectiveness,
                step_size_hint_effectiveness,
            }
        })
        .collect::<Vec<_>>();
    per_factor_hint_effectiveness.sort_by(|a, b| a.base_factor.cmp(&b.base_factor));
    let priority_markets = market_regression_summaries
        .iter()
        .take(3)
        .map(|summary| summary.market.clone())
        .collect::<Vec<_>>();
    let priority_regression_reasons = regression_reason_summaries
        .iter()
        .take(3)
        .map(|summary| summary.reason.clone())
        .collect::<Vec<_>>();
    let recommended_next_mutation_focus =
        if let Some(latest_run) = all_runs.iter().max_by_key(|run| run.timestamp) {
            factor_mutation_recommended_focus(&latest_run.evaluation)
        } else {
            Vec::new()
        };
    let latest_direction_hints =
        if let Some(latest_run) = all_runs.iter().max_by_key(|run| run.timestamp) {
            factor_mutation_direction_hint_summary(&latest_run.evaluation)
        } else {
            Vec::new()
        };
    let latest_step_size_hints =
        if let Some(latest_run) = all_runs.iter().max_by_key(|run| run.timestamp) {
            factor_mutation_step_size_hint_summary(&latest_run.evaluation)
        } else {
            Vec::new()
        };
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "symbol": symbol,
            "source_command_filter": source_command,
            "bucket_by_source": bucket_by_source,
            "total_runs": all_runs.len(),
            "accepted_runs": all_runs.iter().filter(|run| run.evaluation.accepted).count(),
            "latest_run": all_runs.iter().max_by_key(|run| run.timestamp).cloned(),
            "priority_markets": priority_markets,
            "priority_regression_reasons": priority_regression_reasons,
            "recommended_next_mutation_focus": recommended_next_mutation_focus,
            "latest_direction_hints": latest_direction_hints,
            "latest_step_size_hints": latest_step_size_hints,
            "direction_hint_effectiveness": direction_hint_effectiveness,
            "step_size_hint_effectiveness": step_size_hint_effectiveness,
            "per_factor_hint_effectiveness": per_factor_hint_effectiveness,
            "failure_tag_counts": failure_tag_counts,
            "failure_clusters": failure_clusters,
            "regression_reason_summaries": regression_reason_summaries,
            "market_regression_summaries": market_regression_summaries,
            "source_summaries": if bucket_by_source { source_summaries.clone() } else { Vec::<FactorMutationSourceSummary>::new() },
            "runs": runs,
            "recommended_commands": [
                format!(
                    "ict-engine factor-mutation-status --symbol {} --state-dir {} --limit 10{}{}",
                    shell_quote(symbol),
                    shell_quote(state_dir),
                    source_command.map(|value| format!(" --source-command {}", shell_quote(value))).unwrap_or_default(),
                    if bucket_by_source { " --bucket-by-source" } else { "" }
                ),
                format!(
                    "{} --mutation-spec <spec.json> --emit-mutation-evaluation",
                    factor_mutation_research_command(symbol, "<data.json>", state_dir)
                ),
                format!(
                    "ict-engine expansion-sop --root {} --output-dir <output> --interval 15m --lookback 20 --atr-multiplier 1.50 --mutation-spec <spec.json> --emit-mutation-evaluation",
                    shell_quote(
                        &crate::application::multi_timeframe_inputs::detected_tomac_root_or_placeholder()
                    )
                ),
            ]
        }))?
    );
    Ok(())
}

pub struct FactorAutoresearchCommandInput<'a> {
    pub symbol: &'a str,
    pub data: &'a str,
    pub objective: &'a str,
    pub mutation_spec_path: Option<&'a str>,
    pub iterations: usize,
    pub data_1m: Option<&'a str>,
    pub data_5m: Option<&'a str>,
    pub data_15m: Option<&'a str>,
    pub data_1h: Option<&'a str>,
    pub data_4h: Option<&'a str>,
    pub data_1d: Option<&'a str>,
    pub paired_data: Option<&'a str>,
    pub session_id: Option<&'a str>,
    pub resume_latest: bool,
    pub max_cluster_fail_streak: usize,
    pub state_dir: &'a str,
}

pub fn factor_autoresearch_branch_summary(evaluation: &FactorMutationEvaluation) -> Vec<String> {
    let mut summary = Vec::new();
    if !evaluation.reason.is_empty() {
        summary.push(format!("reason={}", evaluation.reason));
    }
    if !evaluation.failure_tags.is_empty() {
        summary.push(format!(
            "failure_tags={}",
            evaluation.failure_tags.join("|")
        ));
    }
    if !evaluation.recommended_mutation_directions.is_empty() {
        summary.push(format!(
            "next_focus={}",
            evaluation.recommended_mutation_directions.join(" | ")
        ));
    }
    summary
}

pub fn factor_autoresearch_decision(
    evaluation: &FactorMutationEvaluation,
) -> FactorAutoresearchDecision {
    FactorAutoresearchDecision {
        status: if evaluation.accepted {
            "keep".to_string()
        } else {
            "discard".to_string()
        },
        reason: evaluation.reason.clone(),
        promoted_to_baseline: evaluation.accepted,
        baseline_score_before: evaluation.score_before,
        candidate_score: evaluation.score_after,
        score_delta: evaluation.score_delta,
    }
}

pub fn factor_autoresearch_command<FLoad, FRun>(
    input: FactorAutoresearchCommandInput<'_>,
    load_mutation_spec: FLoad,
    run_research: FRun,
) -> Result<()>
where
    FLoad: Fn(&str) -> Result<FactorMutationSpec>,
    FRun: Fn(
        crate::application::decision_utils::ResearchObjectiveMode,
        &FactorMutationSpec,
    ) -> Result<crate::factor_lab::research::ResearchReport>,
{
    let objective = parse_research_objective(input.objective)?;
    let _ = migrate_ensemble_executor_scorecards(input.state_dir, input.symbol)?;
    let initial_spec = match (input.mutation_spec_path, input.resume_latest) {
        (Some(path), _) => load_mutation_spec(path)?,
        (None, true) => {
            let attempts = load_factor_autoresearch_attempts(input.state_dir, input.symbol)?;
            attempts
                .into_iter()
                .last()
                .map(|attempt| attempt.candidate_mutation_spec)
                .ok_or_else(|| {
                    anyhow!("--resume-latest requested but no prior autoresearch attempts found")
                })?
        }
        (None, false) => {
            bail!("factor-autoresearch requires --mutation-spec unless --resume-latest is set")
        }
    };
    let now = Utc::now();
    let session_id = input
        .session_id
        .map(str::to_string)
        .unwrap_or_else(|| format!("factor-autoresearch:{}", now.format("%Y%m%dT%H%M%S%.3fZ")));

    let mut sessions = load_factor_autoresearch_sessions(input.state_dir, input.symbol)?;
    let mut session = sessions
        .iter()
        .find(|session| session.session_id == session_id)
        .cloned()
        .unwrap_or_else(|| FactorAutoresearchSession {
            session_id: session_id.clone(),
            started_at: now,
            updated_at: now,
            symbol: input.symbol.to_string(),
            objective: input.objective.to_string(),
            source_command: "factor-autoresearch".to_string(),
            base_factor: initial_spec.base_factor.clone(),
            baseline_mutation_id: Some(initial_spec.mutation_id.clone())
                .filter(|id| !id.is_empty()),
            baseline_score: 0.0,
            attempts_total: 0,
            kept_attempts: 0,
            discarded_attempts: 0,
            last_attempt_id: None,
            status: "running".to_string(),
        });

    let prior_attempts = load_factor_autoresearch_attempts(input.state_dir, input.symbol)?;
    let prior_session_attempts = prior_attempts
        .into_iter()
        .filter(|attempt| attempt.session_id == session_id)
        .collect::<Vec<_>>();
    let mut current_spec = prior_session_attempts
        .iter()
        .rev()
        .find(|attempt| attempt.decision.promoted_to_baseline)
        .map(|attempt| attempt.candidate_mutation_spec.clone())
        .or_else(|| {
            prior_session_attempts
                .last()
                .map(|attempt| attempt.candidate_mutation_spec.clone())
        })
        .unwrap_or_else(|| initial_spec.clone());

    let mut latest_attempt = None;
    let mut cluster_fail_streaks = BTreeMap::<String, usize>::new();
    let mut live_snapshot = FactorAutoresearchLiveSnapshot {
        session_id: session_id.clone(),
        started_at: session.started_at,
        updated_at: now,
        symbol: input.symbol.to_string(),
        objective: input.objective.to_string(),
        current_iteration: 0,
        attempts_total: session.attempts_total,
        kept_attempts: session.kept_attempts,
        discarded_attempts: session.discarded_attempts,
        current_candidate_spec: Some(current_spec.clone()),
        latest_attempt_id: session.last_attempt_id.clone(),
        status: "running".to_string(),
    };
    save_factor_autoresearch_live_snapshot(input.state_dir, input.symbol, &live_snapshot)?;
    for iteration_index in 0..input.iterations {
        live_snapshot.current_iteration = iteration_index + 1;
        live_snapshot.updated_at = Utc::now();
        live_snapshot.current_candidate_spec = Some(current_spec.clone());
        live_snapshot.status = "running".to_string();
        save_factor_autoresearch_live_snapshot(input.state_dir, input.symbol, &live_snapshot)?;
        let report = run_research(objective, &current_spec)?;
        let evaluation = report
            .factor_mutation_evaluation
            .clone()
            .ok_or_else(|| anyhow!("factor-autoresearch requires factor_mutation_evaluation"))?;
        let decision = factor_autoresearch_decision(&evaluation);
        let timestamp = Utc::now();
        let attempt = FactorAutoresearchAttempt {
            session_id: session_id.clone(),
            attempt_id: format!("{}:attempt-{:03}", session_id, session.attempts_total + 1),
            timestamp,
            symbol: input.symbol.to_string(),
            source_command: "factor-autoresearch".to_string(),
            base_factor: current_spec.base_factor.clone(),
            baseline_mutation_id_before: session.baseline_mutation_id.clone(),
            candidate_mutation_spec: current_spec.clone(),
            evaluation: evaluation.clone(),
            decision: decision.clone(),
            branch_summary: factor_autoresearch_branch_summary(&evaluation),
        };
        append_factor_autoresearch_attempt(input.state_dir, input.symbol, attempt.clone())?;
        if let Err(err) = sync_factor_autoresearch_experiments_tsv(input.state_dir, input.symbol) {
            eprintln!(
                "warning: failed to sync derived artifact experiments.tsv for {}: {err:#}",
                input.symbol
            );
        }
        let cluster = attempt
            .candidate_mutation_spec
            .direction_hints
            .get("cluster_jump")
            .cloned()
            .unwrap_or_else(|| "none".to_string());
        session.attempts_total += 1;
        session.updated_at = timestamp;
        session.last_attempt_id = Some(attempt.attempt_id.clone());
        if decision.promoted_to_baseline {
            session.kept_attempts += 1;
            session.baseline_mutation_id =
                Some(attempt.candidate_mutation_spec.mutation_id.clone())
                    .filter(|id| !id.is_empty());
            session.baseline_score = decision.candidate_score;
            session.base_factor = attempt.candidate_mutation_spec.base_factor.clone();
            cluster_fail_streaks.insert(cluster.clone(), 0);
        } else {
            session.discarded_attempts += 1;
            *cluster_fail_streaks.entry(cluster.clone()).or_default() += 1;
        }
        current_spec = next_mutation_spec_template(
            Some(&attempt.candidate_mutation_spec),
            &evaluation,
            attempt.candidate_mutation_spec.evaluate_expansion_preview,
        );
        if cluster_fail_streaks.get(&cluster).copied().unwrap_or(0) >= input.max_cluster_fail_streak
        {
            if let Some(cycle) = current_spec
                .direction_hints
                .get("cluster_jump_cycle")
                .and_then(|value| value.parse::<usize>().ok())
            {
                current_spec
                    .direction_hints
                    .insert("cluster_jump_cycle".to_string(), (cycle + 1).to_string());
            }
        }
        latest_attempt = Some(attempt);
    }

    session.status = "completed".to_string();
    live_snapshot.updated_at = Utc::now();
    live_snapshot.attempts_total = session.attempts_total;
    live_snapshot.kept_attempts = session.kept_attempts;
    live_snapshot.discarded_attempts = session.discarded_attempts;
    live_snapshot.current_candidate_spec = Some(current_spec.clone());
    live_snapshot.latest_attempt_id = session.last_attempt_id.clone();
    live_snapshot.status = "completed".to_string();
    save_factor_autoresearch_live_snapshot(input.state_dir, input.symbol, &live_snapshot)?;
    if let Some(existing) = sessions
        .iter_mut()
        .find(|entry| entry.session_id == session_id)
    {
        *existing = session.clone();
    } else {
        sessions.push(session.clone());
    }
    save_factor_autoresearch_sessions(input.state_dir, input.symbol, &sessions)?;

    let summary = FactorAutoresearchSummary {
        session,
        latest_attempt,
        next_mutation_spec_template: Some(current_spec),
        live_snapshot: Some(live_snapshot),
    };
    save_factor_autoresearch_final_summary(input.state_dir, input.symbol, &summary)?;
    if let Err(err) = sync_factor_autoresearch_retrospective(
        input.state_dir,
        input.symbol,
        Some(&summary.session.session_id),
    ) {
        eprintln!(
            "warning: failed to sync derived artifact factor_autoresearch_retrospective.md for {} session {}: {err:#}",
            input.symbol,
            summary.session.session_id
        );
    }
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

pub fn factor_autoresearch_status_command(
    symbol: &str,
    state_dir: &str,
    session_id: Option<&str>,
    latest_only: bool,
    limit: Option<usize>,
) -> Result<()> {
    let Some(surface) = build_factor_autoresearch_status_surface(
        state_dir,
        symbol,
        session_id,
        latest_only,
        limit,
    )?
    else {
        let empty =
            crate::application::orchestration::workflow_status::factor_autoresearch_status_value_for_empty_state(symbol, state_dir);
        println!("{}", serde_json::to_string_pretty(&empty)?);
        return Ok(());
    };

    println!("{}", serde_json::to_string_pretty(&surface)?);
    Ok(())
}
