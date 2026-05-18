use anyhow::Result;
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::state::{
    load_factor_autoresearch_attempts, load_factor_autoresearch_live_snapshot,
    load_factor_autoresearch_sessions, save_text_state, FactorAutoresearchAttempt,
    FactorAutoresearchLiveSnapshot, FactorAutoresearchSession,
    FACTOR_AUTORESEARCH_EXPERIMENTS_FILE, FACTOR_AUTORESEARCH_FINAL_FILE,
    FACTOR_AUTORESEARCH_LIVE_FILE, FACTOR_AUTORESEARCH_RETROSPECTIVE_FILE,
};

#[derive(Debug, Clone, Serialize, Default)]
pub struct FactorAutoresearchStatusSurface {
    pub symbol: String,
    pub state_dir: String,
    pub session_filter: Option<String>,
    pub effective_status: String,
    pub interrupted: bool,
    pub final_summary_exists: bool,
    pub live_snapshot: Option<FactorAutoresearchLiveSnapshot>,
    pub sessions: Vec<FactorAutoresearchSession>,
    pub attempts: Vec<FactorAutoresearchAttempt>,
    pub decision_counts: BTreeMap<String, usize>,
    pub failure_tag_counts: BTreeMap<String, usize>,
    pub cluster_scoreboard: Vec<FactorAutoresearchClusterScorecardEntry>,
    pub cluster_fail_streaks: BTreeMap<String, usize>,
    pub best_attempt: Option<FactorAutoresearchAttempt>,
    pub derived_warnings: Vec<FactorAutoresearchDerivedWarning>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct FactorAutoresearchClusterScorecardEntry {
    pub cluster: String,
    pub attempts: usize,
    pub avg_score_delta: f64,
    pub best_score_delta: f64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct FactorAutoresearchDerivedWarning {
    pub attempt_id: String,
    pub session_id: String,
    pub code: String,
    pub severity: String,
    pub summary: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct FactorAutoresearchExperimentRow {
    pub timestamp: String,
    pub session_id: String,
    pub attempt_id: String,
    pub attempt_status: String,
    pub base_factor: String,
    pub mutation_id: String,
    pub decision_status: String,
    pub score_before: f64,
    pub score_after: f64,
    pub score_delta: f64,
    pub aggregate_return_before: Option<f64>,
    pub aggregate_return_after: f64,
    pub top_factor: String,
    pub failure_reason: String,
    pub recommended_directions: String,
    pub hypothesis: String,
    pub failure_tags: String,
    pub branch_summary: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct FactorAutoresearchRetrospective {
    pub symbol: String,
    pub session_id: Option<String>,
    pub objective: Option<String>,
    pub base_factor: Option<String>,
    pub effective_status: String,
    pub interrupted: bool,
    pub headline: String,
    pub attempts_total: usize,
    pub kept_attempts: usize,
    pub discarded_attempts: usize,
    pub best_attempt_id: Option<String>,
    pub best_score_delta: Option<f64>,
    pub best_attempt: Option<FactorAutoresearchAttempt>,
    pub aha_attempts: Vec<String>,
    pub most_valuable_attempt: Option<String>,
    pub score_trajectory: Vec<(String, f64)>,
    pub research_arc: Vec<String>,
    pub top_failure_tags: Vec<(String, usize)>,
    pub cluster_scoreboard: Vec<FactorAutoresearchClusterScorecardEntry>,
    pub cross_cluster_findings: Vec<String>,
    pub null_result_replications: Vec<String>,
    pub self_corrections: Vec<String>,
    pub derived_warnings: Vec<FactorAutoresearchDerivedWarning>,
    pub behavior_observations: Vec<String>,
    pub what_was_not_explored: Vec<String>,
    pub oracle_limitations: Vec<String>,
    pub evaluation: Vec<String>,
    pub open_questions: Vec<String>,
    pub recommended_next_focus: Vec<String>,
}

pub fn resolve_factor_autoresearch_effective_status(
    live_snapshot: Option<&FactorAutoresearchLiveSnapshot>,
    final_summary_exists: bool,
    now: DateTime<Utc>,
) -> (String, bool) {
    let staleness_threshold = Duration::minutes(10);
    let snapshot_is_stale = live_snapshot
        .map(|snapshot| now.signed_duration_since(snapshot.updated_at) > staleness_threshold)
        .unwrap_or(false);
    let snapshot_says_running = live_snapshot
        .map(|snapshot| snapshot.status == "running")
        .unwrap_or(false);
    let snapshot_says_completed = live_snapshot
        .map(|snapshot| snapshot.status == "completed")
        .unwrap_or(false);

    if final_summary_exists || snapshot_says_completed {
        ("completed".to_string(), false)
    } else if snapshot_says_running && snapshot_is_stale {
        ("interrupted".to_string(), true)
    } else if snapshot_says_running {
        ("running".to_string(), false)
    } else {
        ("unknown".to_string(), false)
    }
}

pub fn build_factor_autoresearch_cluster_scoreboard(
    attempts: &[FactorAutoresearchAttempt],
) -> Vec<FactorAutoresearchClusterScorecardEntry> {
    let mut cluster_scoreboard = BTreeMap::<String, (usize, f64, f64)>::new();
    for attempt in attempts {
        let cluster = factor_autoresearch_cluster_label(attempt);
        let entry = cluster_scoreboard
            .entry(cluster)
            .or_insert((0, 0.0, f64::MIN));
        entry.0 += 1;
        entry.1 += attempt.decision.score_delta;
        entry.2 = entry.2.max(attempt.decision.score_delta);
    }

    cluster_scoreboard
        .into_iter()
        .map(|(cluster, (attempts, sum_delta, best_delta))| {
            FactorAutoresearchClusterScorecardEntry {
                cluster,
                attempts,
                avg_score_delta: if attempts == 0 {
                    0.0
                } else {
                    sum_delta / attempts as f64
                },
                best_score_delta: if best_delta == f64::MIN {
                    0.0
                } else {
                    best_delta
                },
            }
        })
        .collect()
}

pub fn build_factor_autoresearch_status_surface(
    state_dir: &str,
    symbol: &str,
    session_filter: Option<&str>,
    latest_only: bool,
    limit: Option<usize>,
) -> Result<Option<FactorAutoresearchStatusSurface>> {
    let mut sessions = load_factor_autoresearch_sessions(state_dir, symbol)?;
    sessions.sort_by_key(|session| session.updated_at);
    sessions.reverse();
    if let Some(session_id) = session_filter {
        sessions.retain(|session| session.session_id == session_id);
    }
    if latest_only {
        sessions.truncate(1);
    }
    if let Some(limit) = limit {
        sessions.truncate(limit);
    }

    let selected_session_ids = sessions
        .iter()
        .map(|session| session.session_id.clone())
        .collect::<BTreeSet<_>>();
    let mut attempts = load_factor_autoresearch_attempts(state_dir, symbol)?;
    attempts.retain(|attempt| selected_session_ids.contains(&attempt.session_id));
    attempts.sort_by_key(|attempt| attempt.timestamp);
    attempts.reverse();

    let live_snapshot = load_factor_autoresearch_live_snapshot(state_dir, symbol).ok();
    let live_snapshot_file_exists = Path::new(state_dir)
        .join(symbol)
        .join(FACTOR_AUTORESEARCH_LIVE_FILE)
        .is_file();
    let final_summary_exists = Path::new(state_dir)
        .join(symbol)
        .join(FACTOR_AUTORESEARCH_FINAL_FILE)
        .is_file();

    if sessions.is_empty()
        && attempts.is_empty()
        && !live_snapshot_file_exists
        && !final_summary_exists
    {
        return Ok(None);
    }

    let (effective_status, interrupted) = resolve_factor_autoresearch_effective_status(
        live_snapshot.as_ref(),
        final_summary_exists,
        Utc::now(),
    );

    let mut decision_counts = BTreeMap::<String, usize>::new();
    let mut failure_tag_counts = BTreeMap::<String, usize>::new();
    let mut best_attempt = None;
    let mut best_score_delta = f64::MIN;
    let mut cluster_fail_streaks = BTreeMap::<String, usize>::new();
    for attempt in &attempts {
        *decision_counts
            .entry(attempt.decision.status.clone())
            .or_default() += 1;
        for tag in &attempt.evaluation.failure_tags {
            *failure_tag_counts.entry(tag.clone()).or_default() += 1;
        }
        if attempt.decision.score_delta > best_score_delta {
            best_score_delta = attempt.decision.score_delta;
            best_attempt = Some(attempt.clone());
        }
        if attempt.decision.status == "discard" {
            *cluster_fail_streaks
                .entry(factor_autoresearch_cluster_label(attempt))
                .or_default() += 1;
        }
    }

    let cluster_scoreboard = build_factor_autoresearch_cluster_scoreboard(&attempts);
    let mut surface = FactorAutoresearchStatusSurface {
        symbol: symbol.to_string(),
        state_dir: state_dir.to_string(),
        session_filter: session_filter.map(str::to_string),
        effective_status,
        interrupted,
        final_summary_exists,
        live_snapshot,
        sessions,
        attempts: attempts.clone(),
        decision_counts,
        failure_tag_counts,
        cluster_scoreboard,
        cluster_fail_streaks,
        best_attempt,
        derived_warnings: Vec::new(),
    };
    surface.derived_warnings = build_factor_autoresearch_warning_surface(&surface);

    Ok(Some(surface))
}

pub fn build_factor_autoresearch_experiment_rows(
    attempts: &[FactorAutoresearchAttempt],
) -> Vec<FactorAutoresearchExperimentRow> {
    attempts
        .iter()
        .map(|attempt| {
            let top_factor = attempt
                .evaluation
                .metrics_after
                .top_factor_names
                .first()
                .cloned()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "unknown".to_string());
            let failure_reason = sanitize_tsv_text(&attempt.evaluation.reason);
            let recommended_directions =
                sanitize_tsv_text(&attempt.evaluation.recommended_mutation_directions.join("|"));
            let hypothesis = sanitize_tsv_text(&attempt.candidate_mutation_spec.hypothesis);
            let failure_tags = sanitize_tsv_text(&attempt.evaluation.failure_tags.join("|"));
            let branch_summary = sanitize_tsv_text(&attempt.branch_summary.join(" | "));
            FactorAutoresearchExperimentRow {
                timestamp: attempt
                    .timestamp
                    .to_rfc3339_opts(SecondsFormat::Millis, true),
                session_id: sanitize_tsv_text(&attempt.session_id),
                attempt_id: sanitize_tsv_text(&attempt.attempt_id),
                attempt_status: sanitize_tsv_text(&attempt.decision.status),
                base_factor: sanitize_tsv_text(&attempt.base_factor),
                mutation_id: sanitize_tsv_text(&attempt.candidate_mutation_spec.mutation_id),
                decision_status: sanitize_tsv_text(&attempt.decision.status),
                score_before: attempt.decision.baseline_score_before,
                score_after: attempt.decision.candidate_score,
                score_delta: attempt.decision.score_delta,
                aggregate_return_before: attempt
                    .evaluation
                    .metrics_before
                    .as_ref()
                    .map(|metrics| metrics.aggregate_return),
                aggregate_return_after: attempt.evaluation.metrics_after.aggregate_return,
                top_factor: sanitize_tsv_text(&top_factor),
                failure_reason: failure_reason.clone(),
                recommended_directions: recommended_directions.clone(),
                hypothesis: hypothesis.clone(),
                failure_tags,
                branch_summary: branch_summary.clone(),
                note: build_factor_autoresearch_note(
                    attempt,
                    &hypothesis,
                    &failure_reason,
                    &branch_summary,
                    &recommended_directions,
                ),
            }
        })
        .collect()
}

pub fn render_factor_autoresearch_experiments_tsv(
    rows: &[FactorAutoresearchExperimentRow],
) -> String {
    let mut lines = Vec::with_capacity(rows.len() + 1);
    lines.push(
        [
            "timestamp",
            "session_id",
            "attempt_id",
            "attempt_status",
            "base_factor",
            "mutation_id",
            "decision_status",
            "score_before",
            "score_after",
            "score_delta",
            "aggregate_return_before",
            "aggregate_return_after",
            "top_factor",
            "failure_reason",
            "recommended_directions",
            "hypothesis",
            "failure_tags",
            "branch_summary",
            "note",
        ]
        .join("\t"),
    );
    for row in rows {
        lines.push(
            vec![
                sanitize_tsv_text(&row.timestamp),
                sanitize_tsv_text(&row.session_id),
                sanitize_tsv_text(&row.attempt_id),
                sanitize_tsv_text(&row.attempt_status),
                sanitize_tsv_text(&row.base_factor),
                sanitize_tsv_text(&row.mutation_id),
                sanitize_tsv_text(&row.decision_status),
                row.score_before.to_string(),
                row.score_after.to_string(),
                row.score_delta.to_string(),
                row.aggregate_return_before
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                row.aggregate_return_after.to_string(),
                sanitize_tsv_text(&row.top_factor),
                sanitize_tsv_text(&row.failure_reason),
                sanitize_tsv_text(&row.recommended_directions),
                sanitize_tsv_text(&row.hypothesis),
                sanitize_tsv_text(&row.failure_tags),
                sanitize_tsv_text(&row.branch_summary),
                sanitize_tsv_text(&row.note),
            ]
            .join("\t"),
        );
    }
    format!("{}\n", lines.join("\n"))
}

pub fn sync_factor_autoresearch_experiments_tsv(state_dir: &str, symbol: &str) -> Result<()> {
    let mut attempts = load_factor_autoresearch_attempts(state_dir, symbol)?;
    attempts.sort_by_key(|attempt| attempt.timestamp);
    let rows = build_factor_autoresearch_experiment_rows(&attempts);
    let rendered = render_factor_autoresearch_experiments_tsv(&rows);
    save_text_state(
        state_dir,
        symbol,
        FACTOR_AUTORESEARCH_EXPERIMENTS_FILE,
        &rendered,
    )
}

pub fn build_factor_autoresearch_warning_surface(
    surface: &FactorAutoresearchStatusSurface,
) -> Vec<FactorAutoresearchDerivedWarning> {
    let mut warnings = Vec::new();
    let mut attempts_in_order = surface.attempts.clone();
    attempts_in_order.sort_by_key(|attempt| attempt.timestamp);

    for (index, attempt) in attempts_in_order.iter().enumerate() {
        let prior_attempts = &attempts_in_order[..index];

        if prior_attempts.len() >= 3 {
            let prior_deltas = prior_attempts
                .iter()
                .map(|entry| entry.decision.score_delta)
                .collect::<Vec<_>>();
            let prior_max = prior_deltas
                .iter()
                .copied()
                .fold(f64::MIN, |acc, value| acc.max(value));
            let prior_median = median_f64(&prior_deltas);
            let current_delta = attempt.decision.score_delta;
            if current_delta >= 0.12
                && current_delta > prior_median + 0.15
                && current_delta > prior_max + 0.05
            {
                warnings.push(FactorAutoresearchDerivedWarning {
                    attempt_id: attempt.attempt_id.clone(),
                    session_id: attempt.session_id.clone(),
                    code: "score_delta_jump".to_string(),
                    severity: "medium".to_string(),
                    summary: "score delta jumped materially above recent session baseline"
                        .to_string(),
                    evidence: vec![
                        format!("current_score_delta={:.6}", current_delta),
                        format!("prior_median_score_delta={:.6}", prior_median),
                        format!("prior_max_score_delta={:.6}", prior_max),
                    ],
                });
            }
        }

        if let Some(metrics_before) = &attempt.evaluation.metrics_before {
            let return_before = metrics_before.aggregate_return;
            let return_after = attempt.evaluation.metrics_after.aggregate_return;
            if attempt.decision.score_delta >= 0.05 && return_after <= return_before + 0.01 {
                warnings.push(FactorAutoresearchDerivedWarning {
                    attempt_id: attempt.attempt_id.clone(),
                    session_id: attempt.session_id.clone(),
                    code: "return_mismatch".to_string(),
                    severity: "high".to_string(),
                    summary: "score improved without a meaningful aggregate return improvement"
                        .to_string(),
                    evidence: vec![
                        format!("score_delta={:.6}", attempt.decision.score_delta),
                        format!("aggregate_return_before={:.6}", return_before),
                        format!("aggregate_return_after={:.6}", return_after),
                    ],
                });
            }
        }

        if attempt.decision.status == "keep" && !attempt.evaluation.failure_tags.is_empty() {
            warnings.push(FactorAutoresearchDerivedWarning {
                attempt_id: attempt.attempt_id.clone(),
                session_id: attempt.session_id.clone(),
                code: "keep_with_failure_tags".to_string(),
                severity: "medium".to_string(),
                summary: "attempt was kept despite non-empty failure tags".to_string(),
                evidence: vec![format!(
                    "failure_tags={}",
                    attempt.evaluation.failure_tags.join("|")
                )],
            });
        }
    }

    if attempts_in_order.len() >= 5 {
        for window_end in 2..attempts_in_order.len() {
            let window_start = window_end.saturating_sub(2);
            let recent_window = &attempts_in_order[window_start..=window_end];
            let baseline = &attempts_in_order[..window_start];
            if baseline.len() < 2 {
                continue;
            }
            let baseline_deltas = baseline
                .iter()
                .map(|attempt| attempt.decision.score_delta)
                .collect::<Vec<_>>();
            let baseline_median = median_f64(&baseline_deltas);
            let distinct_clusters = recent_window
                .iter()
                .map(factor_autoresearch_cluster_label)
                .collect::<BTreeSet<_>>();
            let all_jump = recent_window.iter().all(|attempt| {
                attempt.decision.score_delta >= 0.08
                    && attempt.decision.score_delta > baseline_median + 0.08
            });
            if all_jump && distinct_clusters.len() >= 2 {
                let anchor_attempt = recent_window.last().unwrap();
                let recent_avg_delta = recent_window
                    .iter()
                    .map(|attempt| attempt.decision.score_delta)
                    .sum::<f64>()
                    / recent_window.len() as f64;
                warnings.push(FactorAutoresearchDerivedWarning {
                    attempt_id: anchor_attempt.attempt_id.clone(),
                    session_id: anchor_attempt.session_id.clone(),
                    code: "shared_surface_jump".to_string(),
                    severity: "high".to_string(),
                    summary: "multiple nearby attempts across distinct clusters jumped together"
                        .to_string(),
                    evidence: vec![
                        format!("baseline_median_score_delta={:.6}", baseline_median),
                        format!("recent_avg_score_delta={:.6}", recent_avg_delta),
                        format!(
                            "window_attempt_ids={}",
                            recent_window
                                .iter()
                                .map(|attempt| attempt.attempt_id.clone())
                                .collect::<Vec<_>>()
                                .join("|")
                        ),
                        format!(
                            "window_clusters={}",
                            distinct_clusters.into_iter().collect::<Vec<_>>().join("|")
                        ),
                    ],
                });
            }
        }
    }

    if surface.attempts.len() >= 5 {
        if let Some(dominant_cluster) = surface
            .cluster_scoreboard
            .iter()
            .max_by_key(|entry| entry.attempts)
        {
            let cluster_share = dominant_cluster.attempts as f64 / surface.attempts.len() as f64;
            if cluster_share >= 0.7 {
                let anchor_attempt = attempts_in_order
                    .iter()
                    .rev()
                    .find(|attempt| {
                        factor_autoresearch_cluster_label(attempt) == dominant_cluster.cluster
                    })
                    .or_else(|| attempts_in_order.last());
                if let Some(anchor_attempt) = anchor_attempt {
                    warnings.push(FactorAutoresearchDerivedWarning {
                        attempt_id: anchor_attempt.attempt_id.clone(),
                        session_id: anchor_attempt.session_id.clone(),
                        code: "cluster_overconcentration".to_string(),
                        severity: "medium".to_string(),
                        summary: "one cluster dominates the attempt population".to_string(),
                        evidence: vec![
                            format!("cluster={}", dominant_cluster.cluster),
                            format!("cluster_attempts={}", dominant_cluster.attempts),
                            format!("cluster_share={:.6}", cluster_share),
                        ],
                    });
                }
            }
        }
    }

    normalize_factor_autoresearch_warnings(warnings, &attempts_in_order)
}

pub fn build_factor_autoresearch_retrospective(
    surface: &FactorAutoresearchStatusSurface,
) -> FactorAutoresearchRetrospective {
    let latest_session = if surface.sessions.len() == 1 {
        surface.sessions.first()
    } else {
        None
    };
    let mut attempts_in_order = surface.attempts.clone();
    attempts_in_order.sort_by_key(|attempt| attempt.timestamp);

    let mut top_failure_tags = surface
        .failure_tag_counts
        .iter()
        .map(|(tag, count)| (tag.clone(), *count))
        .collect::<Vec<_>>();
    top_failure_tags.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    top_failure_tags.truncate(10);

    let kept_attempts = surface.decision_counts.get("keep").copied().unwrap_or(0);
    let discarded_attempts = surface.decision_counts.get("discard").copied().unwrap_or(0);
    let headline = build_factor_autoresearch_headline(surface, kept_attempts, discarded_attempts);

    FactorAutoresearchRetrospective {
        symbol: surface.symbol.clone(),
        session_id: latest_session.map(|session| session.session_id.clone()),
        objective: latest_session.map(|session| session.objective.clone()),
        base_factor: latest_session.map(|session| session.base_factor.clone()),
        effective_status: surface.effective_status.clone(),
        interrupted: surface.interrupted,
        headline,
        attempts_total: surface.attempts.len(),
        kept_attempts,
        discarded_attempts,
        best_attempt_id: surface
            .best_attempt
            .as_ref()
            .map(|attempt| attempt.attempt_id.clone()),
        best_score_delta: surface
            .best_attempt
            .as_ref()
            .map(|attempt| attempt.decision.score_delta),
        best_attempt: surface.best_attempt.clone(),
        aha_attempts: build_factor_autoresearch_aha_attempts(surface),
        most_valuable_attempt: build_factor_autoresearch_most_valuable_attempt(surface),
        score_trajectory: attempts_in_order
            .iter()
            .map(|attempt| (attempt.attempt_id.clone(), attempt.decision.score_delta))
            .collect(),
        research_arc: build_factor_autoresearch_research_arc(surface, &attempts_in_order),
        top_failure_tags,
        cluster_scoreboard: surface.cluster_scoreboard.clone(),
        cross_cluster_findings: build_factor_autoresearch_cross_cluster_findings(surface),
        null_result_replications: build_factor_autoresearch_null_result_replications(
            &attempts_in_order,
        ),
        self_corrections: build_factor_autoresearch_self_corrections(&attempts_in_order),
        derived_warnings: surface.derived_warnings.clone(),
        behavior_observations: build_factor_autoresearch_behavior_observations(surface),
        what_was_not_explored: build_factor_autoresearch_what_was_not_explored(surface),
        oracle_limitations: build_factor_autoresearch_oracle_limitations(surface),
        evaluation: build_factor_autoresearch_evaluation(surface),
        open_questions: build_factor_autoresearch_open_questions(surface),
        recommended_next_focus: build_factor_autoresearch_recommended_next_focus(
            &attempts_in_order,
        ),
    }
}

pub fn render_factor_autoresearch_retrospective_markdown(
    retrospective: &FactorAutoresearchRetrospective,
) -> String {
    let is_multi_session_summary =
        retrospective.session_id.is_none() && retrospective.attempts_total > 0;
    let session_label_default = if retrospective.attempts_total > 0 {
        "multi-session-summary"
    } else {
        "unscoped"
    };
    let objective_label_default = if is_multi_session_summary {
        "multiple"
    } else {
        "unknown"
    };
    let base_factor_label_default = if is_multi_session_summary {
        "multiple"
    } else {
        "unknown"
    };
    let session_label = retrospective
        .session_id
        .as_deref()
        .unwrap_or(session_label_default);
    let objective_label = retrospective
        .objective
        .as_deref()
        .unwrap_or(objective_label_default);
    let base_factor_label = retrospective
        .base_factor
        .as_deref()
        .unwrap_or(base_factor_label_default);
    let mut lines = vec![
        "# Session Summary".to_string(),
        String::new(),
        format!(
            "- Symbol: `{}`",
            sanitize_markdown_text(&retrospective.symbol)
        ),
        format!("- Session: `{}`", sanitize_markdown_text(session_label)),
        format!("- Objective: `{}`", sanitize_markdown_text(objective_label)),
        format!(
            "- Base factor: `{}`",
            sanitize_markdown_text(base_factor_label)
        ),
        format!("- Effective status: `{}`", retrospective.effective_status),
        format!("- Interrupted: `{}`", retrospective.interrupted),
        format!("- Attempts total: `{}`", retrospective.attempts_total),
        format!("- Kept attempts: `{}`", retrospective.kept_attempts),
        format!(
            "- Discarded attempts: `{}`",
            retrospective.discarded_attempts
        ),
        String::new(),
        "## Status".to_string(),
        String::new(),
        format!(
            "This session is currently classified as **`{}`**.",
            retrospective.effective_status
        ),
        String::new(),
        "## Headline".to_string(),
        String::new(),
        retrospective.headline.clone(),
        String::new(),
        "## Research Arc".to_string(),
        String::new(),
    ];

    if retrospective.research_arc.is_empty() {
        lines.push("- No research arc could be derived.".to_string());
    } else {
        for item in &retrospective.research_arc {
            lines.push(format!("- {}", item));
        }
    }

    lines.extend([
        String::new(),
        "## Decision Mix".to_string(),
        String::new(),
        "| decision | count |".to_string(),
        "|---|---:|".to_string(),
        format!("| keep | {} |", retrospective.kept_attempts),
        format!("| discard | {} |", retrospective.discarded_attempts),
        String::new(),
        "## Score Trajectory".to_string(),
        String::new(),
    ]);

    if retrospective.score_trajectory.is_empty() {
        lines.push("- No attempts recorded.".to_string());
    } else {
        for (attempt_id, score_delta) in &retrospective.score_trajectory {
            lines.push(format!("- `{}` → `{:.6}`", attempt_id, score_delta));
        }
    }

    lines.push(String::new());
    lines.push("## Aha Attempts".to_string());
    lines.push(String::new());
    if retrospective.aha_attempts.is_empty() {
        lines.push("- No aha attempt stood out from the recorded state.".to_string());
    } else {
        for item in &retrospective.aha_attempts {
            lines.push(format!("- {}", item));
        }
    }

    lines.push(String::new());
    lines.push("## Best Attempt".to_string());
    lines.push(String::new());
    if let Some(best_attempt) = &retrospective.best_attempt {
        lines.push(format!(
            "- Attempt: `{}`",
            sanitize_markdown_text(&best_attempt.attempt_id)
        ));
        lines.push(format!("- Decision: `{}`", best_attempt.decision.status));
        lines.push(format!(
            "- Score delta: `{:.6}`",
            best_attempt.decision.score_delta
        ));
        lines.push(format!(
            "- Reason: `{}`",
            sanitize_markdown_text(&best_attempt.evaluation.reason)
        ));
        if !best_attempt
            .candidate_mutation_spec
            .hypothesis
            .trim()
            .is_empty()
        {
            lines.push(format!(
                "- Hypothesis: `{}`",
                sanitize_markdown_text(&best_attempt.candidate_mutation_spec.hypothesis)
            ));
        }
        if !best_attempt.branch_summary.is_empty() {
            lines.push(format!(
                "- Branch summary: `{}`",
                sanitize_markdown_text(&best_attempt.branch_summary.join(" | "))
            ));
        }
    } else {
        lines.push("- No best attempt available.".to_string());
    }

    lines.push(String::new());
    lines.push("## Most Valuable Attempt".to_string());
    lines.push(String::new());
    match retrospective.most_valuable_attempt.as_ref() {
        Some(item) => lines.push(format!("- {}", item)),
        None => lines.push("- No most valuable attempt could be derived.".to_string()),
    }

    lines.push(String::new());
    lines.push("## Failure Tag Concentration".to_string());
    lines.push(String::new());
    if retrospective.top_failure_tags.is_empty() {
        lines.push("- No failure tags recorded.".to_string());
    } else {
        for (tag, count) in &retrospective.top_failure_tags {
            lines.push(format!("- `{}` × `{}`", tag, count));
        }
    }

    lines.push(String::new());
    lines.push("## Cluster Scoreboard".to_string());
    lines.push(String::new());
    lines.push("| cluster | attempts | avg_score_delta | best_score_delta |".to_string());
    lines.push("|---|---:|---:|---:|".to_string());
    if retrospective.cluster_scoreboard.is_empty() {
        lines.push("| none | 0 | 0.0 | 0.0 |".to_string());
    } else {
        for entry in &retrospective.cluster_scoreboard {
            lines.push(format!(
                "| {} | {} | {:.6} | {:.6} |",
                entry.cluster, entry.attempts, entry.avg_score_delta, entry.best_score_delta
            ));
        }
    }

    lines.push(String::new());
    lines.push("## Cross-Cluster Findings".to_string());
    lines.push(String::new());
    if retrospective.cross_cluster_findings.is_empty() {
        lines.push(
            "- No cross-cluster finding could be derived from the recorded attempts.".to_string(),
        );
    } else {
        for item in &retrospective.cross_cluster_findings {
            lines.push(format!("- {}", item));
        }
    }

    lines.push(String::new());
    lines.push("## Null-Result Replications".to_string());
    lines.push(String::new());
    if retrospective.null_result_replications.is_empty() {
        lines.push(
            "- No null-result replication could be inferred from the recorded attempts."
                .to_string(),
        );
    } else {
        for item in &retrospective.null_result_replications {
            lines.push(format!("- {}", item));
        }
    }

    lines.push(String::new());
    lines.push("## Self-Corrections".to_string());
    lines.push(String::new());
    if retrospective.self_corrections.is_empty() {
        lines.push(
            "- No self-correction event could be inferred from the recorded attempts.".to_string(),
        );
    } else {
        for item in &retrospective.self_corrections {
            lines.push(format!("- {}", item));
        }
    }

    lines.push(String::new());
    lines.push("## Derived Warnings".to_string());
    lines.push(String::new());
    if retrospective.derived_warnings.is_empty() {
        lines.push("- No derived warnings triggered.".to_string());
    } else {
        for warning in &retrospective.derived_warnings {
            lines.push(format!(
                "- `{}` [{}] on `{}`: {}",
                warning.code, warning.severity, warning.attempt_id, warning.summary
            ));
            for evidence in &warning.evidence {
                lines.push(format!("  - {}", evidence));
            }
        }
    }

    lines.push(String::new());
    lines.push("## Behavior Observations".to_string());
    lines.push(String::new());
    if retrospective.behavior_observations.is_empty() {
        lines.push("- No behavior observations derived.".to_string());
    } else {
        for item in &retrospective.behavior_observations {
            lines.push(format!("- {}", item));
        }
    }

    lines.push(String::new());
    lines.push("## What Was Not Explored".to_string());
    lines.push(String::new());
    if retrospective.what_was_not_explored.is_empty() {
        lines.push(
            "- No obvious unexplored area was derived from the recorded attempts.".to_string(),
        );
    } else {
        for item in &retrospective.what_was_not_explored {
            lines.push(format!("- {}", item));
        }
    }

    lines.push(String::new());
    lines.push("## Oracle Limitations".to_string());
    lines.push(String::new());
    if retrospective.oracle_limitations.is_empty() {
        lines.push(
            "- No additional oracle limitations were derived from the current state.".to_string(),
        );
    } else {
        for item in &retrospective.oracle_limitations {
            lines.push(format!("- {}", item));
        }
    }

    lines.push(String::new());
    lines.push("## Evaluation".to_string());
    lines.push(String::new());
    if retrospective.evaluation.is_empty() {
        lines.push("- No evaluation summary was derived.".to_string());
    } else {
        for item in &retrospective.evaluation {
            lines.push(format!("- {}", item));
        }
    }

    lines.push(String::new());
    lines.push("## Open Questions".to_string());
    lines.push(String::new());
    if retrospective.open_questions.is_empty() {
        lines.push("- No open questions were derived.".to_string());
    } else {
        for item in &retrospective.open_questions {
            lines.push(format!("- {}", item));
        }
    }

    lines.push(String::new());
    lines.push("## Suggested Next Focus".to_string());
    lines.push(String::new());
    if retrospective.recommended_next_focus.is_empty() {
        lines.push("- No mutation directions recorded yet.".to_string());
    } else {
        for focus in &retrospective.recommended_next_focus {
            lines.push(format!("- {}", sanitize_markdown_text(focus)));
        }
    }

    format!("{}\n", lines.join("\n"))
}

pub fn sync_factor_autoresearch_retrospective(
    state_dir: &str,
    symbol: &str,
    session_filter: Option<&str>,
) -> Result<()> {
    let Some(surface) =
        build_factor_autoresearch_status_surface(state_dir, symbol, session_filter, false, None)?
    else {
        return Ok(());
    };
    let retrospective = build_factor_autoresearch_retrospective(&surface);
    let rendered = render_factor_autoresearch_retrospective_markdown(&retrospective);
    save_text_state(
        state_dir,
        symbol,
        FACTOR_AUTORESEARCH_RETROSPECTIVE_FILE,
        &rendered,
    )
}

fn factor_autoresearch_cluster_label(attempt: &FactorAutoresearchAttempt) -> String {
    attempt
        .candidate_mutation_spec
        .direction_hints
        .get("cluster_jump")
        .cloned()
        .unwrap_or_else(|| "none".to_string())
}

fn build_factor_autoresearch_note(
    attempt: &FactorAutoresearchAttempt,
    hypothesis: &str,
    failure_reason: &str,
    branch_summary: &str,
    recommended_directions: &str,
) -> String {
    let mut segments = Vec::new();
    if !hypothesis.is_empty() {
        segments.push(format!("hypothesis={hypothesis}"));
    }
    if !failure_reason.is_empty() {
        segments.push(format!("reason={failure_reason}"));
    }
    if !branch_summary.is_empty() {
        segments.push(format!("branch={branch_summary}"));
    }
    if !recommended_directions.is_empty() {
        segments.push(format!("next={recommended_directions}"));
    }
    if segments.is_empty() {
        segments.push(format!(
            "decision={} score_delta={}",
            attempt.decision.status, attempt.decision.score_delta
        ));
    }
    sanitize_tsv_text(&segments.join(" || "))
}

fn sanitize_tsv_text(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '\t' | '\n' | '\r' => ' ',
            _ => ch,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn sanitize_markdown_text(value: &str) -> String {
    sanitize_tsv_text(value).replace('`', "'").replace('|', "/")
}

fn build_factor_autoresearch_recommended_next_focus(
    attempts: &[FactorAutoresearchAttempt],
) -> Vec<String> {
    let mut direction_counts = BTreeMap::<String, usize>::new();
    for attempt in attempts {
        for direction in &attempt.evaluation.recommended_mutation_directions {
            let trimmed = direction.trim();
            if !trimmed.is_empty() {
                *direction_counts.entry(trimmed.to_string()).or_default() += 1;
            }
        }
    }

    let mut ranked = direction_counts.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked.truncate(5);
    ranked
        .into_iter()
        .map(|(direction, count)| {
            format!(
                "{} (seen {} times)",
                sanitize_markdown_text(&direction),
                count
            )
        })
        .collect()
}

fn build_factor_autoresearch_headline(
    surface: &FactorAutoresearchStatusSurface,
    kept_attempts: usize,
    discarded_attempts: usize,
) -> String {
    match surface.best_attempt.as_ref() {
        Some(best_attempt) => format!(
            "Best attempt `{}` moved score delta to `{:.6}` while the session finished as `{}` with `{}` keeps and `{}` discards.",
            best_attempt.attempt_id,
            best_attempt.decision.score_delta,
            surface.effective_status,
            kept_attempts,
            discarded_attempts
        ),
        None => format!(
            "No best attempt was available; the session is currently `{}` with `{}` recorded attempts.",
            surface.effective_status,
            surface.attempts.len()
        ),
    }
}

fn build_factor_autoresearch_research_arc(
    surface: &FactorAutoresearchStatusSurface,
    attempts: &[FactorAutoresearchAttempt],
) -> Vec<String> {
    if attempts.is_empty() {
        return Vec::new();
    }

    let first_attempt = attempts.first().unwrap();
    let last_attempt = attempts.last().unwrap();
    let mut arc = vec![format!(
        "Search opened with `{}` and ended with `{}` across `{}` recorded attempts.",
        first_attempt.attempt_id,
        last_attempt.attempt_id,
        attempts.len()
    )];

    if let Some(best_attempt) = surface.best_attempt.as_ref() {
        arc.push(format!(
            "Peak uplift came from `{}` at `{:.6}` score delta.",
            best_attempt.attempt_id, best_attempt.decision.score_delta
        ));
    }

    if let Some(top_cluster) = surface
        .cluster_scoreboard
        .iter()
        .max_by_key(|entry| entry.attempts)
    {
        arc.push(format!(
            "Cluster `{}` absorbed the most exploration with `{}` attempts.",
            top_cluster.cluster, top_cluster.attempts
        ));
    }

    if surface.interrupted {
        arc.push("The run appears interrupted rather than cleanly completed, so late-session interpretation should be treated cautiously.".to_string());
    }

    arc
}

fn build_factor_autoresearch_aha_attempts(
    surface: &FactorAutoresearchStatusSurface,
) -> Vec<String> {
    let mut items = Vec::new();
    if let Some(best_attempt) = surface.best_attempt.as_ref() {
        items.push(format!(
            "`{}` stands out as the sharpest uplift with `{:.6}` score delta and reason `{}`.",
            best_attempt.attempt_id,
            best_attempt.decision.score_delta,
            sanitize_markdown_text(&best_attempt.evaluation.reason)
        ));
    }
    for warning in surface.derived_warnings.iter().take(2) {
        items.push(format!(
            "`{}` also matters because warning `{}` was triggered there, forcing a higher-skepticism read of the result.",
            warning.attempt_id,
            warning.code
        ));
    }
    items.truncate(3);
    items
}

fn build_factor_autoresearch_most_valuable_attempt(
    surface: &FactorAutoresearchStatusSurface,
) -> Option<String> {
    surface.best_attempt.as_ref().map(|attempt| {
        format!(
            "`{}` is the most valuable recorded attempt because it set the session high-water mark at `{:.6}` while preserving the clearest evidence bundle in the state surface.",
            attempt.attempt_id,
            attempt.decision.score_delta
        )
    })
}

fn build_factor_autoresearch_cross_cluster_findings(
    surface: &FactorAutoresearchStatusSurface,
) -> Vec<String> {
    if surface.cluster_scoreboard.len() < 2 {
        return Vec::new();
    }

    let mut entries = surface.cluster_scoreboard.clone();
    entries.sort_by(|a, b| {
        b.best_score_delta
            .partial_cmp(&a.best_score_delta)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.attempts.cmp(&a.attempts))
    });

    let best = &entries[0];
    let second = &entries[1];
    let mut findings = vec![format!(
        "Cluster `{}` outperformed `{}` on best score delta (`{:.6}` vs `{:.6}`).",
        best.cluster, second.cluster, best.best_score_delta, second.best_score_delta
    )];

    if let Some(most_sampled) = surface
        .cluster_scoreboard
        .iter()
        .max_by_key(|entry| entry.attempts)
    {
        findings.push(format!(
            "Most sampling went into `{}` with `{}` attempts, so conclusions may still be biased toward that family.",
            most_sampled.cluster,
            most_sampled.attempts
        ));
    }

    if surface
        .derived_warnings
        .iter()
        .any(|warning| warning.code == "shared_surface_jump")
    {
        findings.push("Distinct clusters showed a coordinated nearby uplift, which is more consistent with a shared-surface move than a purely local discovery.".to_string());
    }

    findings
}

fn build_factor_autoresearch_null_result_replications(
    attempts: &[FactorAutoresearchAttempt],
) -> Vec<String> {
    let mut first_failure_by_tag = BTreeMap::<String, usize>::new();
    let mut emitted_tags = BTreeSet::<String>::new();
    let mut items = Vec::new();

    for (index, attempt) in attempts.iter().enumerate() {
        let Some(tag) = primary_failure_tag(attempt) else {
            continue;
        };
        if let Some(&original_index) = first_failure_by_tag.get(tag) {
            if emitted_tags.contains(tag) {
                continue;
            }
            let original = &attempts[original_index];
            let outcome = if attempt.decision.status == "discard" {
                "confirmed-generalization"
            } else if attempt.decision.status == "keep" || attempt.decision.score_delta > 0.05 {
                "contradicted"
            } else {
                "inconclusive"
            };
            let hypothesis = if attempt.candidate_mutation_spec.hypothesis.trim().is_empty() {
                "hypothesis unavailable".to_string()
            } else {
                sanitize_markdown_text(&attempt.candidate_mutation_spec.hypothesis)
            };
            let clusters = [
                factor_autoresearch_cluster_label(original),
                factor_autoresearch_cluster_label(attempt),
            ]
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join("|");
            items.push(format!(
                "`{}` re-tested original failure `{}` with hypothesis `{}`; outcome `{}` after failure tag `{}` recurred across clusters `{}`.",
                attempt.attempt_id,
                original.attempt_id,
                hypothesis,
                outcome,
                tag,
                clusters
            ));
            emitted_tags.insert(tag.to_string());
        } else {
            first_failure_by_tag.insert(tag.to_string(), index);
        }
    }

    items.truncate(5);
    items
}

fn build_factor_autoresearch_self_corrections(
    attempts: &[FactorAutoresearchAttempt],
) -> Vec<String> {
    let mut first_discard_by_cluster = BTreeMap::<String, usize>::new();
    let mut emitted_clusters = BTreeSet::<String>::new();
    let mut items = Vec::new();

    for (index, attempt) in attempts.iter().enumerate() {
        let cluster = factor_autoresearch_cluster_label(attempt);
        if attempt.decision.status == "discard" {
            first_discard_by_cluster.entry(cluster).or_insert(index);
            continue;
        }
        if emitted_clusters.contains(&cluster) {
            continue;
        }
        if let Some(&original_index) = first_discard_by_cluster.get(&cluster) {
            let original = &attempts[original_index];
            if attempt.decision.score_delta > original.decision.score_delta + 0.10
                || (attempt.decision.status == "keep" && original.decision.status == "discard")
            {
                items.push(format!(
                    "Cluster `{}` was initially rejected at `{}` (`{:.6}`), but `{}` revised that read to `{}` with `{:.6}`.",
                    cluster,
                    original.attempt_id,
                    original.decision.score_delta,
                    attempt.attempt_id,
                    attempt.decision.status,
                    attempt.decision.score_delta
                ));
                emitted_clusters.insert(cluster);
            }
        }
    }

    items.truncate(5);
    items
}

fn build_factor_autoresearch_behavior_observations(
    surface: &FactorAutoresearchStatusSurface,
) -> Vec<String> {
    let mut observations = Vec::new();
    observations.push(format!(
        "Decision mix settled at `{}` keeps vs `{}` discards.",
        surface.decision_counts.get("keep").copied().unwrap_or(0),
        surface.decision_counts.get("discard").copied().unwrap_or(0)
    ));

    if let Some((tag, count)) = surface
        .failure_tag_counts
        .iter()
        .max_by_key(|(_, count)| *count)
    {
        observations.push(format!(
            "Most common failure tag was `{}` with `{}` hits.",
            tag, count
        ));
    }

    if !surface.derived_warnings.is_empty() {
        observations.push(format!(
            "Derived integrity layer emitted `{}` warnings, suggesting the session should be read with extra skepticism around sudden improvements.",
            surface.derived_warnings.len()
        ));
    }

    observations
}

fn build_factor_autoresearch_what_was_not_explored(
    surface: &FactorAutoresearchStatusSurface,
) -> Vec<String> {
    let mut items = Vec::new();
    if surface.cluster_scoreboard.len() <= 1 && !surface.attempts.is_empty() {
        let cluster = surface
            .cluster_scoreboard
            .first()
            .map(|entry| entry.cluster.as_str())
            .unwrap_or("none");
        items.push(format!(
            "Exploration stayed concentrated in cluster `{}`; no broader cross-cluster comparison was recorded.",
            cluster
        ));
    }
    if surface.decision_counts.get("keep").copied().unwrap_or(0) == 0
        && !surface.attempts.is_empty()
    {
        items.push("No candidate was kept, so the search never validated a stronger baseline variant inside the recorded window.".to_string());
    }
    items
}

fn primary_failure_tag(attempt: &FactorAutoresearchAttempt) -> Option<&str> {
    attempt.evaluation.failure_tags.first().map(String::as_str)
}

fn normalize_factor_autoresearch_warnings(
    warnings: Vec<FactorAutoresearchDerivedWarning>,
    attempts: &[FactorAutoresearchAttempt],
) -> Vec<FactorAutoresearchDerivedWarning> {
    let attempt_order = attempts
        .iter()
        .enumerate()
        .map(|(index, attempt)| (attempt.attempt_id.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let mut deduped = BTreeMap::<(String, String), FactorAutoresearchDerivedWarning>::new();

    for warning in warnings {
        let key = (warning.attempt_id.clone(), warning.code.clone());
        if let Some(existing) = deduped.get_mut(&key) {
            let should_replace = warning_severity_rank(&warning.severity)
                < warning_severity_rank(&existing.severity)
                || warning.evidence.len() > existing.evidence.len();
            if should_replace {
                *existing = warning;
            }
        } else {
            deduped.insert(key, warning);
        }
    }

    let mut warnings = deduped.into_values().collect::<Vec<_>>();
    warnings.sort_by(|left, right| {
        warning_severity_rank(&left.severity)
            .cmp(&warning_severity_rank(&right.severity))
            .then_with(|| {
                attempt_order
                    .get(&left.attempt_id)
                    .copied()
                    .unwrap_or(usize::MAX)
                    .cmp(
                        &attempt_order
                            .get(&right.attempt_id)
                            .copied()
                            .unwrap_or(usize::MAX),
                    )
            })
            .then_with(|| left.code.cmp(&right.code))
    });
    warnings
}

fn warning_severity_rank(severity: &str) -> usize {
    match severity {
        "high" => 0,
        "medium" => 1,
        _ => 2,
    }
}

fn build_factor_autoresearch_oracle_limitations(
    surface: &FactorAutoresearchStatusSurface,
) -> Vec<String> {
    let mut items = Vec::new();
    let attempts_missing_return_baseline = surface
        .attempts
        .iter()
        .filter(|attempt| attempt.evaluation.metrics_before.is_none())
        .count();
    if attempts_missing_return_baseline > 0 {
        items.push(format!(
            "`{}` attempts lacked `metrics_before`, so return-mismatch checks could not run on the full session history.",
            attempts_missing_return_baseline
        ));
    }
    if surface.interrupted {
        items.push("Interrupted sessions may understate late-stage convergence because the effective status is inferred from staleness rather than a clean terminal signal.".to_string());
    }
    items
}

fn build_factor_autoresearch_evaluation(surface: &FactorAutoresearchStatusSurface) -> Vec<String> {
    let mut items = Vec::new();
    if let Some(best_attempt) = surface.best_attempt.as_ref() {
        items.push(format!(
            "Best recorded improvement was `{:.6}` on `{}`.",
            best_attempt.decision.score_delta, best_attempt.attempt_id
        ));
    }
    items.push(format!(
        "Integrity layer reported `{}` derived warnings across `{}` attempts.",
        surface.derived_warnings.len(),
        surface.attempts.len()
    ));
    items
}

fn build_factor_autoresearch_open_questions(
    surface: &FactorAutoresearchStatusSurface,
) -> Vec<String> {
    let mut items = Vec::new();
    if surface
        .derived_warnings
        .iter()
        .any(|warning| warning.code == "return_mismatch")
    {
        items.push("Do score gains continue to hold when aggregate return is treated as a harder acceptance constraint?".to_string());
    }
    if let Some((tag, _)) = surface
        .failure_tag_counts
        .iter()
        .max_by_key(|(_, count)| *count)
    {
        items.push(format!(
            "What mutation change would most directly reduce recurring failure tag `{}`?",
            tag
        ));
    }
    if surface.cluster_scoreboard.len() <= 1 && !surface.attempts.is_empty() {
        items.push("Would a second cluster family invalidate the current conclusions or reveal a shared-surface effect?".to_string());
    }
    items
}

fn median_f64(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{
        append_factor_autoresearch_attempt, FactorAutoresearchDecision, FactorMutationEvaluation,
        FactorMutationMetricSet, FactorMutationSpec,
    };
    use crate::state::{save_factor_autoresearch_sessions, FactorAutoresearchSession};

    #[test]
    fn effective_status_marks_stale_running_as_interrupted() {
        let snapshot = FactorAutoresearchLiveSnapshot {
            status: "running".to_string(),
            updated_at: Utc::now() - Duration::minutes(30),
            ..FactorAutoresearchLiveSnapshot::default()
        };

        let (status, interrupted) =
            resolve_factor_autoresearch_effective_status(Some(&snapshot), false, Utc::now());

        assert_eq!(status, "interrupted");
        assert!(interrupted);
    }

    #[test]
    fn cluster_scoreboard_aggregates_attempts() {
        let attempts = vec![
            FactorAutoresearchAttempt {
                candidate_mutation_spec: FactorMutationSpec {
                    direction_hints: BTreeMap::from([(
                        "cluster_jump".to_string(),
                        "mss_bos_cluster".to_string(),
                    )]),
                    ..FactorMutationSpec::default()
                },
                decision: FactorAutoresearchDecision {
                    status: "discard".to_string(),
                    score_delta: -0.1,
                    ..FactorAutoresearchDecision::default()
                },
                ..FactorAutoresearchAttempt::default()
            },
            FactorAutoresearchAttempt {
                candidate_mutation_spec: FactorMutationSpec {
                    direction_hints: BTreeMap::from([(
                        "cluster_jump".to_string(),
                        "mss_bos_cluster".to_string(),
                    )]),
                    ..FactorMutationSpec::default()
                },
                decision: FactorAutoresearchDecision {
                    status: "discard".to_string(),
                    score_delta: -0.05,
                    ..FactorAutoresearchDecision::default()
                },
                ..FactorAutoresearchAttempt::default()
            },
        ];

        let scoreboard = build_factor_autoresearch_cluster_scoreboard(&attempts);
        let entry = scoreboard
            .iter()
            .find(|entry| entry.cluster == "mss_bos_cluster")
            .unwrap();

        assert_eq!(entry.attempts, 2);
        assert!((entry.avg_score_delta - (-0.075)).abs() < 1e-9);
        assert!((entry.best_score_delta - (-0.05)).abs() < 1e-9);
    }

    #[test]
    fn sync_experiments_tsv_writes_header_and_rows() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let symbol = "NQ";
        let attempt = FactorAutoresearchAttempt {
            session_id: "session-1".to_string(),
            attempt_id: "session-1:attempt-001".to_string(),
            base_factor: "structure_ict".to_string(),
            candidate_mutation_spec: FactorMutationSpec {
                mutation_id: "mut-1".to_string(),
                hypothesis: "break\nout\tcleanup".to_string(),
                ..FactorMutationSpec::default()
            },
            evaluation: FactorMutationEvaluation {
                reason: "reason\twith\nnoise".to_string(),
                failure_tags: vec!["bridge_gap_too_small".to_string()],
                recommended_mutation_directions: vec!["tighten filters".to_string()],
                metrics_before: Some(FactorMutationMetricSet {
                    aggregate_return: 1.25,
                    ..FactorMutationMetricSet::default()
                }),
                metrics_after: FactorMutationMetricSet {
                    aggregate_return: 2.5,
                    top_factor_names: vec!["alpha_core".to_string()],
                    ..FactorMutationMetricSet::default()
                },
                ..FactorMutationEvaluation::default()
            },
            decision: FactorAutoresearchDecision {
                status: "keep".to_string(),
                baseline_score_before: 0.4,
                candidate_score: 0.55,
                score_delta: 0.15,
                ..FactorAutoresearchDecision::default()
            },
            branch_summary: vec!["summary\nline".to_string()],
            ..FactorAutoresearchAttempt::default()
        };

        append_factor_autoresearch_attempt(temp.path(), symbol, attempt)?;
        sync_factor_autoresearch_experiments_tsv(temp.path().to_str().unwrap(), symbol)?;

        let rendered = std::fs::read_to_string(
            temp.path()
                .join(symbol)
                .join(FACTOR_AUTORESEARCH_EXPERIMENTS_FILE),
        )?;

        assert!(rendered.contains("timestamp\tsession_id\tattempt_id"));
        assert!(rendered.contains("session-1:attempt-001"));
        assert!(rendered.contains("break out cleanup"));
        assert!(rendered.contains("reason with noise"));
        assert!(rendered.contains("summary line"));
        Ok(())
    }

    #[test]
    fn sync_retrospective_writes_markdown_sections() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let symbol = "NQ";
        let session = FactorAutoresearchSession {
            session_id: "session-1".to_string(),
            updated_at: Utc::now(),
            objective: "improve composite score".to_string(),
            base_factor: "structure_ict".to_string(),
            status: "completed".to_string(),
            ..FactorAutoresearchSession::default()
        };
        let attempt = FactorAutoresearchAttempt {
            session_id: session.session_id.clone(),
            attempt_id: "session-1:attempt-001".to_string(),
            base_factor: "structure_ict".to_string(),
            candidate_mutation_spec: FactorMutationSpec {
                mutation_id: "mut-1".to_string(),
                hypothesis: "test focus".to_string(),
                ..FactorMutationSpec::default()
            },
            evaluation: FactorMutationEvaluation {
                reason: "improved structure alignment".to_string(),
                failure_tags: vec!["bridge_gap_too_small".to_string()],
                recommended_mutation_directions: vec!["tighten confirmation".to_string()],
                metrics_after: FactorMutationMetricSet {
                    aggregate_return: 2.5,
                    top_factor_names: vec!["alpha_core".to_string()],
                    ..FactorMutationMetricSet::default()
                },
                ..FactorMutationEvaluation::default()
            },
            decision: FactorAutoresearchDecision {
                status: "keep".to_string(),
                baseline_score_before: 0.4,
                candidate_score: 0.55,
                score_delta: 0.15,
                ..FactorAutoresearchDecision::default()
            },
            branch_summary: vec!["summary line".to_string()],
            ..FactorAutoresearchAttempt::default()
        };

        save_factor_autoresearch_sessions(temp.path(), symbol, &[session])?;
        append_factor_autoresearch_attempt(temp.path(), symbol, attempt)?;
        sync_factor_autoresearch_retrospective(
            temp.path().to_str().unwrap(),
            symbol,
            Some("session-1"),
        )?;

        let rendered = std::fs::read_to_string(
            temp.path()
                .join(symbol)
                .join(FACTOR_AUTORESEARCH_RETROSPECTIVE_FILE),
        )?;

        assert!(rendered.contains("# Session Summary"));
        assert!(rendered.contains("## Best Attempt"));
        assert!(rendered.contains("## Cluster Scoreboard"));
        assert!(rendered.contains("## Derived Warnings"));
        assert!(rendered.contains("## Headline"));
        assert!(rendered.contains("## Research Arc"));
        assert!(rendered.contains("## Aha Attempts"));
        assert!(rendered.contains("## Most Valuable Attempt"));
        assert!(rendered.contains("## Cross-Cluster Findings"));
        assert!(rendered.contains("## Null-Result Replications"));
        assert!(rendered.contains("## Self-Corrections"));
        assert!(rendered.contains("## Behavior Observations"));
        assert!(rendered.contains("## What Was Not Explored"));
        assert!(rendered.contains("## Oracle Limitations"));
        assert!(rendered.contains("## Evaluation"));
        assert!(rendered.contains("## Open Questions"));
        assert!(rendered.contains("tighten confirmation"));
        Ok(())
    }

    #[test]
    fn warning_surface_flags_return_mismatch_and_keep_with_failure_tags() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let symbol = "NQ";
        let session = FactorAutoresearchSession {
            session_id: "session-1".to_string(),
            updated_at: Utc::now(),
            objective: "improve composite score".to_string(),
            base_factor: "structure_ict".to_string(),
            status: "completed".to_string(),
            ..FactorAutoresearchSession::default()
        };
        let attempt = FactorAutoresearchAttempt {
            session_id: session.session_id.clone(),
            attempt_id: "session-1:attempt-001".to_string(),
            base_factor: "structure_ict".to_string(),
            evaluation: FactorMutationEvaluation {
                reason: "score up but returns flat".to_string(),
                failure_tags: vec!["bridge_gap_too_small".to_string()],
                metrics_before: Some(FactorMutationMetricSet {
                    aggregate_return: 2.0,
                    ..FactorMutationMetricSet::default()
                }),
                metrics_after: FactorMutationMetricSet {
                    aggregate_return: 2.0,
                    ..FactorMutationMetricSet::default()
                },
                ..FactorMutationEvaluation::default()
            },
            decision: FactorAutoresearchDecision {
                status: "keep".to_string(),
                score_delta: 0.2,
                ..FactorAutoresearchDecision::default()
            },
            ..FactorAutoresearchAttempt::default()
        };

        save_factor_autoresearch_sessions(temp.path(), symbol, &[session])?;
        append_factor_autoresearch_attempt(temp.path(), symbol, attempt)?;

        let surface = build_factor_autoresearch_status_surface(
            temp.path().to_str().unwrap(),
            symbol,
            Some("session-1"),
            false,
            None,
        )?
        .unwrap();

        assert!(surface
            .derived_warnings
            .iter()
            .any(|warning| warning.code == "return_mismatch"));
        assert!(surface
            .derived_warnings
            .iter()
            .any(|warning| warning.code == "keep_with_failure_tags"));
        Ok(())
    }

    #[test]
    fn warning_surface_flags_shared_surface_jump_and_prioritizes_high_severity() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let symbol = "NQ";
        let base_time = Utc::now();
        let session = FactorAutoresearchSession {
            session_id: "session-1".to_string(),
            updated_at: base_time,
            objective: "improve composite score".to_string(),
            base_factor: "structure_ict".to_string(),
            status: "completed".to_string(),
            ..FactorAutoresearchSession::default()
        };
        let session_id = session.session_id.clone();
        let make_attempt =
            |attempt_id: &str, cluster: &str, minutes: i64, score_delta: f64, status: &str| {
                FactorAutoresearchAttempt {
                    session_id: session_id.clone(),
                    attempt_id: attempt_id.to_string(),
                    timestamp: base_time + Duration::minutes(minutes),
                    candidate_mutation_spec: FactorMutationSpec {
                        direction_hints: BTreeMap::from([(
                            "cluster_jump".to_string(),
                            cluster.to_string(),
                        )]),
                        ..FactorMutationSpec::default()
                    },
                    decision: FactorAutoresearchDecision {
                        status: status.to_string(),
                        score_delta,
                        ..FactorAutoresearchDecision::default()
                    },
                    ..FactorAutoresearchAttempt::default()
                }
            };

        save_factor_autoresearch_sessions(temp.path(), symbol, &[session])?;
        for attempt in [
            make_attempt("a1", "alpha", 0, 0.01, "discard"),
            make_attempt("a2", "beta", 1, 0.02, "discard"),
            make_attempt("a3", "alpha", 2, 0.18, "keep"),
            make_attempt("a4", "beta", 3, 0.19, "keep"),
            make_attempt("a5", "alpha", 4, 0.20, "keep"),
        ] {
            append_factor_autoresearch_attempt(temp.path(), symbol, attempt)?;
        }

        let surface = build_factor_autoresearch_status_surface(
            temp.path().to_str().unwrap(),
            symbol,
            Some("session-1"),
            false,
            None,
        )?
        .unwrap();

        assert!(surface
            .derived_warnings
            .iter()
            .any(|warning| warning.code == "shared_surface_jump"));
        assert_eq!(
            surface
                .derived_warnings
                .first()
                .map(|warning| warning.code.as_str()),
            Some("shared_surface_jump")
        );
        Ok(())
    }

    #[test]
    fn retrospective_derives_null_result_replications_and_self_corrections() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let symbol = "NQ";
        let base_time = Utc::now();
        let session = FactorAutoresearchSession {
            session_id: "session-1".to_string(),
            updated_at: base_time,
            objective: "improve composite score".to_string(),
            base_factor: "structure_ict".to_string(),
            status: "completed".to_string(),
            ..FactorAutoresearchSession::default()
        };
        let session_id = session.session_id.clone();
        let make_attempt = |attempt_id: &str,
                            cluster: &str,
                            minutes: i64,
                            score_delta: f64,
                            status: &str,
                            failure_tags: Vec<String>,
                            hypothesis: &str| {
            FactorAutoresearchAttempt {
                session_id: session_id.clone(),
                attempt_id: attempt_id.to_string(),
                timestamp: base_time + Duration::minutes(minutes),
                candidate_mutation_spec: FactorMutationSpec {
                    hypothesis: hypothesis.to_string(),
                    direction_hints: BTreeMap::from([(
                        "cluster_jump".to_string(),
                        cluster.to_string(),
                    )]),
                    ..FactorMutationSpec::default()
                },
                evaluation: FactorMutationEvaluation {
                    failure_tags,
                    ..FactorMutationEvaluation::default()
                },
                decision: FactorAutoresearchDecision {
                    status: status.to_string(),
                    score_delta,
                    ..FactorAutoresearchDecision::default()
                },
                ..FactorAutoresearchAttempt::default()
            }
        };

        save_factor_autoresearch_sessions(temp.path(), symbol, &[session])?;
        for attempt in [
            make_attempt(
                "a1",
                "alpha",
                0,
                -0.05,
                "discard",
                vec!["bridge_gap_too_small".to_string()],
                "alpha first pass",
            ),
            make_attempt(
                "a2",
                "beta",
                1,
                -0.04,
                "discard",
                vec!["bridge_gap_too_small".to_string()],
                "beta retest",
            ),
            make_attempt(
                "a3",
                "alpha",
                2,
                -0.02,
                "discard",
                vec!["timing_noise".to_string()],
                "alpha tighten pass",
            ),
            make_attempt(
                "a4",
                "alpha",
                3,
                0.22,
                "keep",
                Vec::new(),
                "alpha recovered",
            ),
        ] {
            append_factor_autoresearch_attempt(temp.path(), symbol, attempt)?;
        }

        let surface = build_factor_autoresearch_status_surface(
            temp.path().to_str().unwrap(),
            symbol,
            Some("session-1"),
            false,
            None,
        )?
        .unwrap();
        let retrospective = build_factor_autoresearch_retrospective(&surface);
        let rendered = render_factor_autoresearch_retrospective_markdown(&retrospective);

        assert!(retrospective
            .null_result_replications
            .iter()
            .any(|item| item.contains("confirmed-generalization")));
        assert!(retrospective
            .self_corrections
            .iter()
            .any(|item| item.contains("revised")));
        assert!(rendered.contains("## Null-Result Replications"));
        assert!(rendered.contains("## Self-Corrections"));
        Ok(())
    }

    #[test]
    fn retrospective_markdown_sanitizes_free_text_fields() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let symbol = "NQ";
        let session = FactorAutoresearchSession {
            session_id: "session-1".to_string(),
            updated_at: Utc::now(),
            objective: "objective with `pipe|break`".to_string(),
            base_factor: "base|factor".to_string(),
            status: "completed".to_string(),
            ..FactorAutoresearchSession::default()
        };
        let attempt = FactorAutoresearchAttempt {
            session_id: session.session_id.clone(),
            attempt_id: "session-1:attempt-001".to_string(),
            base_factor: "structure_ict".to_string(),
            candidate_mutation_spec: FactorMutationSpec {
                mutation_id: "mut-1".to_string(),
                hypothesis: "test\n`focus|edge`".to_string(),
                ..FactorMutationSpec::default()
            },
            evaluation: FactorMutationEvaluation {
                reason: "improved\n`structure|alignment`".to_string(),
                recommended_mutation_directions: vec!["tighten\n`confirmation|gate`".to_string()],
                ..FactorMutationEvaluation::default()
            },
            decision: FactorAutoresearchDecision {
                status: "keep".to_string(),
                score_delta: 0.15,
                ..FactorAutoresearchDecision::default()
            },
            branch_summary: vec!["branch\n`a|b`".to_string()],
            ..FactorAutoresearchAttempt::default()
        };

        save_factor_autoresearch_sessions(temp.path(), symbol, &[session])?;
        append_factor_autoresearch_attempt(temp.path(), symbol, attempt)?;
        sync_factor_autoresearch_retrospective(
            temp.path().to_str().unwrap(),
            symbol,
            Some("session-1"),
        )?;

        let rendered = std::fs::read_to_string(
            temp.path()
                .join(symbol)
                .join(FACTOR_AUTORESEARCH_RETROSPECTIVE_FILE),
        )?;

        assert!(rendered.contains("objective with 'pipe/break'"));
        assert!(rendered.contains("base/factor"));
        assert!(rendered.contains("improved 'structure/alignment'"));
        assert!(rendered.contains("test 'focus/edge'"));
        assert!(rendered.contains("branch 'a/b'"));
        assert!(rendered.contains("tighten 'confirmation/gate' (seen 1 times)"));
        Ok(())
    }

    #[test]
    fn retrospective_multi_session_surface_uses_multi_session_header() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let symbol = "NQ";
        let now = Utc::now();
        let sessions = vec![
            FactorAutoresearchSession {
                session_id: "session-1".to_string(),
                updated_at: now,
                objective: "objective-1".to_string(),
                base_factor: "factor-1".to_string(),
                status: "completed".to_string(),
                ..FactorAutoresearchSession::default()
            },
            FactorAutoresearchSession {
                session_id: "session-2".to_string(),
                updated_at: now + Duration::minutes(1),
                objective: "objective-2".to_string(),
                base_factor: "factor-2".to_string(),
                status: "completed".to_string(),
                ..FactorAutoresearchSession::default()
            },
        ];
        let attempts = vec![
            FactorAutoresearchAttempt {
                session_id: "session-1".to_string(),
                attempt_id: "session-1:attempt-001".to_string(),
                timestamp: now,
                decision: FactorAutoresearchDecision {
                    status: "discard".to_string(),
                    score_delta: -0.02,
                    ..FactorAutoresearchDecision::default()
                },
                ..FactorAutoresearchAttempt::default()
            },
            FactorAutoresearchAttempt {
                session_id: "session-2".to_string(),
                attempt_id: "session-2:attempt-001".to_string(),
                timestamp: now + Duration::minutes(1),
                decision: FactorAutoresearchDecision {
                    status: "keep".to_string(),
                    score_delta: 0.11,
                    ..FactorAutoresearchDecision::default()
                },
                ..FactorAutoresearchAttempt::default()
            },
        ];

        save_factor_autoresearch_sessions(temp.path(), symbol, &sessions)?;
        for attempt in attempts {
            append_factor_autoresearch_attempt(temp.path(), symbol, attempt)?;
        }

        let surface = build_factor_autoresearch_status_surface(
            temp.path().to_str().unwrap(),
            symbol,
            None,
            false,
            None,
        )?
        .unwrap();
        let retrospective = build_factor_autoresearch_retrospective(&surface);
        let rendered = render_factor_autoresearch_retrospective_markdown(&retrospective);

        assert!(retrospective.session_id.is_none());
        assert!(retrospective.objective.is_none());
        assert!(retrospective.base_factor.is_none());
        assert!(rendered.contains("- Session: `multi-session-summary`"));
        assert!(rendered.contains("- Objective: `multiple`"));
        assert!(rendered.contains("- Base factor: `multiple`"));
        Ok(())
    }
}
