use anyhow::Result;
use serde::Serialize;
use serde_json::{json, Value};

use crate::analyze_report_shell::AnalyzeReport;
use crate::application::belief::{BeliefPolicyLineageSurface, BeliefShadowPolicySurface};
use crate::application::orchestration::ExecutionTriage;
use crate::application::output_foundation::{
    format_executor_summary_lines, print_redacted_json, redact_local_paths_in_human_text,
    redact_local_paths_in_value,
};
use crate::application::reporting::{
    build_agent_guidance_report, build_compact_analyze_report, build_human_analyze_report,
    humanize_decision_hint, humanize_next_step_line, AgentGuidanceReport, CompactAnalyzeReport,
    HumanAnalyzeReport,
};
use crate::config::shell_quote;
use crate::pda_sequence::{
    load_pda_sequence_analysis, summarize_pda_sequence_artifact, PdaSequenceArtifactSummary,
};

use crate::types::Direction;

const ANALYZE_JSON_LEDGER_TAIL_DEFAULT: usize = 5;

#[derive(Debug, Serialize)]
pub struct AnalyzeMarketFamilySummary {
    pub market_family: Option<String>,
    pub market_behavior_profile: Option<String>,
    pub selected_market_subgraph: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeOutputEnvelope<R, E>
where
    R: Serialize,
    E: Serialize,
{
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_triage: Option<ExecutionTriage>,
    pub report: R,
    pub compact_report: CompactAnalyzeReport,
    pub agent_report: AgentGuidanceReport,
    pub human_report: String,
    pub market_family_summary: AnalyzeMarketFamilySummary,
    pub belief_shadow_policy: BeliefShadowPolicySurface,
    pub belief_policy_lineage: BeliefPolicyLineageSurface,
    pub ensemble_vote: E,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pda_sequence_summary: Option<PdaSequenceArtifactSummary>,
    pub executor_scorecard_summary: Vec<String>,
    pub executor_scorecard_source: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeLiveOutputEnvelope<R>
where
    R: Serialize,
{
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_triage: Option<ExecutionTriage>,
    pub report: R,
    pub source_snapshot: Option<serde_json::Value>,
    pub freshness_gate: Option<serde_json::Value>,
    pub compact_report: CompactAnalyzeReport,
    pub agent_report: AgentGuidanceReport,
    pub human_report: String,
    pub belief_shadow_policy: BeliefShadowPolicySurface,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pda_sequence_summary: Option<PdaSequenceArtifactSummary>,
}

#[derive(Debug, Clone, Copy)]
pub struct AnalyzeOutputDispatchInput<'a> {
    pub output_format: &'a str,
    pub inline_ledger: bool,
}

impl<'a> AnalyzeOutputDispatchInput<'a> {
    pub fn new(output_format: &'a str) -> Self {
        Self {
            output_format,
            inline_ledger: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AnalyzeLiveOutputDispatchInput<'a> {
    pub output_format: &'a str,
    pub include_pda_sequence_summary: bool,
    pub redact_paths: bool,
}

impl Default for AnalyzeLiveOutputDispatchInput<'_> {
    fn default() -> Self {
        Self {
            output_format: "json",
            include_pda_sequence_summary: true,
            redact_paths: true,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AnalyzeLiveOutputEmitInput {
    pub include_pda_sequence_summary: bool,
    pub redact_paths: bool,
}

impl Default for AnalyzeLiveOutputEmitInput {
    fn default() -> Self {
        Self {
            include_pda_sequence_summary: true,
            redact_paths: true,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AnalyzeLiveReportingBundleInput {
    pub include_pda_sequence_summary: bool,
}

impl Default for AnalyzeLiveReportingBundleInput {
    fn default() -> Self {
        Self {
            include_pda_sequence_summary: true,
        }
    }
}

#[derive(Debug)]
pub struct AnalyzeLiveReportingBundle {
    pub source_snapshot: Option<serde_json::Value>,
    pub freshness_gate: Option<serde_json::Value>,
    pub compact_report: CompactAnalyzeReport,
    pub agent_report: AgentGuidanceReport,
    pub human_report: HumanAnalyzeReport,
    pub belief_shadow_policy: BeliefShadowPolicySurface,
    pub pda_sequence_summary: Option<PdaSequenceArtifactSummary>,
}

#[derive(Debug, Clone)]
pub struct AnalyzeHumanInput<'a> {
    pub symbol: &'a str,
    pub selected_direction: Direction,
    pub entry_quality: &'a str,
    pub gate_status: &'a str,
    pub evidence_quality_score: f64,
    pub decision_hint: &'a str,
    pub factor_iteration_queue: &'a [crate::state::FactorIterationPrompt],
    pub recommended_next_command: &'a str,
    pub price_action_narrative: &'a str,
    pub technical_price_narrative: &'a str,
    pub smt_correlation_narrative: &'a str,
    pub regime_label: &'a str,
    pub liquidity_label: &'a str,
    pub regime_selected_direction: Direction,
    pub trade_plan_narrative: &'a str,
    pub market_family: Option<&'a str>,
    pub market_subgraph: &'a str,
    pub objective_jump_weight: Option<f64>,
    pub regime_companion_suffix: Option<&'a str>,
}

pub struct AnalyzeReportingBundleInput<'a> {
    pub input: AnalyzeHumanInput<'a>,
    pub artifact_action_summary: &'a [String],
    pub multi_timeframe_summary: &'a [String],
    pub decision_hint: &'a str,
    pub selected_direction: Direction,
    pub entry_quality_state: &'a str,
    pub gate_status: &'a str,
    pub recommended_next_command: &'a str,
}

pub struct AnalyzeOutputEnvelopeInput<'a, R, E>
where
    R: Serialize,
    E: Serialize,
{
    pub report: R,
    pub compact_report: CompactAnalyzeReport,
    pub agent_report: AgentGuidanceReport,
    pub human_report: &'a HumanAnalyzeReport,
    pub market_family_summary: AnalyzeMarketFamilySummary,
    pub belief_shadow_policy: BeliefShadowPolicySurface,
    pub belief_policy_lineage: BeliefPolicyLineageSurface,
    pub ensemble_vote: E,
    pub pda_sequence_summary: Option<PdaSequenceArtifactSummary>,
    pub executor_scorecard_source: String,
}

pub struct AnalyzeLiveOutputEnvelopeInput<'a, R, S, F>
where
    R: Serialize,
    S: Serialize,
    F: Serialize,
{
    pub report: R,
    pub source_snapshot: Option<S>,
    pub freshness_gate: Option<F>,
    pub compact_report: CompactAnalyzeReport,
    pub agent_report: AgentGuidanceReport,
    pub human_report: &'a HumanAnalyzeReport,
    pub belief_shadow_policy: BeliefShadowPolicySurface,
    pub pda_sequence_summary: Option<PdaSequenceArtifactSummary>,
}

pub struct AnalyzeLiveOutputValueInput<'a, R, S, F>
where
    R: Serialize,
    S: Serialize,
    F: Serialize,
{
    pub report: R,
    pub source_snapshot: Option<S>,
    pub freshness_gate: Option<F>,
    pub compact_report: CompactAnalyzeReport,
    pub agent_report: AgentGuidanceReport,
    pub human_report: &'a HumanAnalyzeReport,
    pub belief_shadow_policy: BeliefShadowPolicySurface,
    pub pda_sequence_summary: Option<PdaSequenceArtifactSummary>,
    pub redact_paths: bool,
}

pub struct AnalyzeOutputValueInput<'a, R, E>
where
    R: Serialize,
    E: Serialize,
{
    pub report: R,
    pub compact_report: CompactAnalyzeReport,
    pub agent_report: AgentGuidanceReport,
    pub human_report: &'a HumanAnalyzeReport,
    pub market_family_summary: AnalyzeMarketFamilySummary,
    pub belief_shadow_policy: BeliefShadowPolicySurface,
    pub belief_policy_lineage: BeliefPolicyLineageSurface,
    pub ensemble_vote: E,
    pub pda_sequence_summary: Option<PdaSequenceArtifactSummary>,
    pub executor_scorecard_source: String,
    pub inline_ledger: bool,
    pub redact_paths: bool,
}

pub struct EmitAnalyzeOutputEnvelopeInput<'a, R, E>
where
    R: Serialize,
    E: Serialize,
{
    pub report: &'a R,
    pub output_format: &'a str,
    pub inline_ledger: bool,
    pub compact_report: &'a CompactAnalyzeReport,
    pub agent_report: &'a AgentGuidanceReport,
    pub human_report: &'a HumanAnalyzeReport,
    pub market_family_summary: AnalyzeMarketFamilySummary,
    pub belief_shadow_policy: BeliefShadowPolicySurface,
    pub belief_policy_lineage: BeliefPolicyLineageSurface,
    pub ensemble_vote: &'a E,
    pub pda_sequence_summary: Option<PdaSequenceArtifactSummary>,
    pub executor_scorecard_source: String,
}

pub struct EmitAnalyzeLiveOutputEnvelopeInput<'a, R, S, F>
where
    R: Serialize,
    S: Serialize,
    F: Serialize,
{
    pub report: &'a R,
    pub source_snapshot: Option<S>,
    pub freshness_gate: Option<F>,
    pub compact_report: CompactAnalyzeReport,
    pub agent_report: AgentGuidanceReport,
    pub human_report: &'a HumanAnalyzeReport,
    pub belief_shadow_policy: BeliefShadowPolicySurface,
    pub pda_sequence_summary: Option<PdaSequenceArtifactSummary>,
}

pub fn build_analyze_compact_evidence(
    multi_timeframe_summary: &[String],
    objective_jump_weight: Option<f64>,
) -> Vec<String> {
    let objective_jump_weight =
        objective_jump_weight.map(|weight| format!("objective_jump_weight={weight:.3}"));
    objective_jump_weight
        .iter()
        .chain(multi_timeframe_summary.iter())
        .cloned()
        .collect::<Vec<_>>()
}

pub fn build_analyze_reporting_bundle(
    input: AnalyzeReportingBundleInput<'_>,
) -> (
    CompactAnalyzeReport,
    AgentGuidanceReport,
    HumanAnalyzeReport,
) {
    let AnalyzeReportingBundleInput {
        input,
        artifact_action_summary,
        multi_timeframe_summary,
        decision_hint,
        selected_direction,
        entry_quality_state,
        gate_status,
        recommended_next_command,
    } = input;
    let compact_evidence =
        build_analyze_compact_evidence(multi_timeframe_summary, input.objective_jump_weight);
    let compact_report = build_compact_analyze_report(
        decision_hint.to_string(),
        Some(format!("{:?}", selected_direction)),
        Some(entry_quality_state.to_string()),
        Some(gate_status.to_string()),
        Some(recommended_next_command.to_string()),
        &compact_evidence,
        artifact_action_summary,
        std::slice::from_ref(&recommended_next_command.to_string()),
    );
    let agent_report = build_agent_guidance_report(
        Some(format!("{:?}", selected_direction)),
        Some(entry_quality_state.to_string()),
        Some(gate_status.to_string()),
        Some(recommended_next_command.to_string()),
        Some(decision_hint.to_string()),
        multi_timeframe_summary,
        artifact_action_summary,
        std::slice::from_ref(&recommended_next_command.to_string()),
    );
    let human_report = build_human_analyze_surface(input);
    (compact_report, agent_report, human_report)
}

pub fn build_analyze_policy_outputs(
    report: &AnalyzeReport,
) -> Result<(BeliefShadowPolicySurface, BeliefPolicyLineageSurface)> {
    let policy_history =
        crate::state::load_pre_bayes_policy_history(&report.meta.state_dir, &report.symbol)?;
    let policy_record = policy_history.last().cloned();
    let shadow = crate::application::belief::build_belief_shadow_policy_surface(
        &report.supporting.canonical_belief_report,
        policy_record.as_ref(),
    );
    let lineage = crate::application::belief::build_belief_policy_lineage_surface(
        &policy_history,
        report
            .supporting
            .pre_bayes_evidence_filter
            .gating_status
            .as_str(),
    );
    Ok((shadow, lineage))
}

pub fn emit_analyze_output(report: &AnalyzeReport, output_format: &str) -> Result<()> {
    dispatch_analyze_output(report, AnalyzeOutputDispatchInput::new(output_format))
}

pub fn dispatch_analyze_output(
    report: &AnalyzeReport,
    input: AnalyzeOutputDispatchInput<'_>,
) -> Result<()> {
    let (mut compact_report, mut agent_report, mut human_report) =
        build_analyze_reporting_bundle(AnalyzeReportingBundleInput {
            input: AnalyzeHumanInput {
                symbol: &report.symbol,
                selected_direction: report.supporting.decision.selected_direction,
                entry_quality: &report.supporting.entry_quality.selected_state,
                gate_status: &report.supporting.pre_bayes_evidence_filter.gating_status,
                evidence_quality_score: report
                    .supporting
                    .pre_bayes_evidence_filter
                    .evidence_quality_score,
                decision_hint: &report.supporting.decision_hint,
                factor_iteration_queue: &report.supporting.factor_iteration_queue,
                recommended_next_command: &report.supporting.recommended_next_command,
                price_action_narrative: &report.analysis.price_action.narrative,
                technical_price_narrative: &report.analysis.technical_price.narrative,
                smt_correlation_narrative: &report.analysis.smt_correlation.narrative,
                regime_label: &report.analysis.regime_bayesian.regime_label,
                liquidity_label: &report.analysis.regime_bayesian.liquidity_label,
                regime_selected_direction: report.analysis.regime_bayesian.selected_direction,
                trade_plan_narrative: &report.analysis.trade_plan.narrative,
                market_family: report
                    .supporting
                    .canonical_belief_report
                    .market_family
                    .as_deref(),
                market_subgraph: report
                    .supporting
                    .canonical_belief_report
                    .selected_market_subgraph
                    .as_deref()
                    .unwrap_or("unknown"),
                objective_jump_weight: report.supporting.objective_jump_weight,
                regime_companion_suffix: None,
            },
            artifact_action_summary: &report.supporting.artifact_action_summary,
            multi_timeframe_summary: &report.supporting.multi_timeframe_summary,
            decision_hint: &report.supporting.decision_hint,
            selected_direction: report.supporting.decision.selected_direction,
            entry_quality_state: &report.supporting.entry_quality.selected_state,
            gate_status: &report.supporting.pre_bayes_evidence_filter.gating_status,
            recommended_next_command: &report.supporting.recommended_next_command,
        });
    if let Some(triage) = report.supporting.execution_triage.as_ref() {
        human_report.execution_triage_line = Some(triage.consumer_reason.clone());
        compact_report.execution_triage = Some(triage.clone());
        agent_report.execution_triage = Some(triage.clone());
    }
    let (belief_shadow_policy, belief_policy_lineage) = build_analyze_policy_outputs(report)?;
    let ensemble_vote = crate::application::orchestration::build_stub_ensemble_vote_from_input(
        &crate::application::orchestration::AnalyzeEnsembleVoteInput {
            symbol: report.symbol.clone(),
            state_dir: None,
            recommended_next_command: report.supporting.recommended_next_command.clone(),
            hard_blocked: false,
            hard_block_reason: None,
            hard_block_command: None,
            provenance: report.supporting.provenance.clone(),
            dataset_comparability: report.supporting.dataset_comparability.clone(),
            pre_bayes_filter: Some(report.supporting.pre_bayes_evidence_filter.clone()),
            belief: report.supporting.canonical_belief_report.clone(),
            ict_structure: None,
        },
    );
    let persisted_scorecards =
        crate::state::load_ensemble_executor_scorecards(&report.meta.state_dir, &report.symbol)
            .unwrap_or_default();
    let (_, scorecard_source) =
        crate::application::orchestration::executor_scorecard_surface(&persisted_scorecards, &[]);
    let pda_sequence_summary = load_pda_sequence_analysis(&report.meta.state_dir, &report.symbol)
        .ok()
        .map(|artifact| summarize_pda_sequence_artifact(&artifact));

    emit_analyze_output_envelope(EmitAnalyzeOutputEnvelopeInput {
        report,
        output_format: input.output_format,
        inline_ledger: input.inline_ledger,
        compact_report: &compact_report,
        agent_report: &agent_report,
        human_report: &human_report,
        market_family_summary: AnalyzeMarketFamilySummary {
            market_family: report
                .supporting
                .canonical_belief_report
                .market_family
                .clone(),
            market_behavior_profile: report
                .supporting
                .canonical_belief_report
                .market_behavior_profile
                .clone(),
            selected_market_subgraph: report
                .supporting
                .canonical_belief_report
                .selected_market_subgraph
                .clone(),
        },
        belief_shadow_policy,
        belief_policy_lineage,
        ensemble_vote: &ensemble_vote,
        pda_sequence_summary,
        executor_scorecard_source: scorecard_source.to_string(),
    })
}

pub fn build_analyze_live_reporting_bundle(
    report: &AnalyzeReport,
    input: AnalyzeLiveReportingBundleInput,
) -> Result<AnalyzeLiveReportingBundle> {
    let source_snapshot = report.meta.data_source.as_ref().map(|source| {
        crate::application::data_sources::build_source_snapshot(source, report.timestamp)
    });
    let freshness_gate = report.meta.data_source.as_ref().map(|source| {
        crate::application::decision_freshness::build_decision_freshness_gate(
            300,
            report
                .timestamp
                .signed_duration_since(source.fetched_at)
                .num_seconds(),
        )
    });
    let regime_companion_suffix = regime_companion_human_suffix(&report.analysis.regime_bayesian);
    let (mut compact_report, mut agent_report, mut human_report) =
        build_analyze_reporting_bundle(AnalyzeReportingBundleInput {
            input: AnalyzeHumanInput {
                symbol: &report.symbol,
                selected_direction: report.supporting.decision.selected_direction,
                entry_quality: &report.supporting.entry_quality.selected_state,
                gate_status: &report.supporting.pre_bayes_evidence_filter.gating_status,
                evidence_quality_score: report
                    .supporting
                    .pre_bayes_evidence_filter
                    .evidence_quality_score,
                decision_hint: &report.supporting.decision_hint,
                factor_iteration_queue: &report.supporting.factor_iteration_queue,
                recommended_next_command: &report.supporting.recommended_next_command,
                price_action_narrative: &report.analysis.price_action.narrative,
                technical_price_narrative: &report.analysis.technical_price.narrative,
                smt_correlation_narrative: &report.analysis.smt_correlation.narrative,
                regime_label: &report.analysis.regime_bayesian.regime_label,
                liquidity_label: &report.analysis.regime_bayesian.liquidity_label,
                regime_selected_direction: report.analysis.regime_bayesian.selected_direction,
                trade_plan_narrative: &report.analysis.trade_plan.narrative,
                market_family: report
                    .supporting
                    .canonical_belief_report
                    .market_family
                    .as_deref(),
                market_subgraph: report
                    .supporting
                    .canonical_belief_report
                    .selected_market_subgraph
                    .as_deref()
                    .unwrap_or("unknown"),
                objective_jump_weight: report.supporting.objective_jump_weight,
                regime_companion_suffix: (!regime_companion_suffix.is_empty())
                    .then_some(regime_companion_suffix.as_str()),
            },
            artifact_action_summary: &report.supporting.artifact_action_summary,
            multi_timeframe_summary: &report.supporting.multi_timeframe_summary,
            decision_hint: &report.supporting.decision_hint,
            selected_direction: report.supporting.decision.selected_direction,
            entry_quality_state: &report.supporting.entry_quality.selected_state,
            gate_status: &report.supporting.pre_bayes_evidence_filter.gating_status,
            recommended_next_command: &report.supporting.recommended_next_command,
        });
    if let Some(triage) = report.supporting.execution_triage.as_ref() {
        human_report.execution_triage_line = Some(triage.consumer_reason.clone());
        compact_report.execution_triage = Some(triage.clone());
        agent_report.execution_triage = Some(triage.clone());
    }
    let policy_record =
        crate::state::load_pre_bayes_policy_history(&report.meta.state_dir, &report.symbol)?
            .into_iter()
            .last();
    let belief_shadow_policy = crate::application::belief::build_belief_shadow_policy_surface(
        &report.supporting.canonical_belief_report,
        policy_record.as_ref(),
    );
    let pda_sequence_summary = input
        .include_pda_sequence_summary
        .then(|| load_pda_sequence_analysis(&report.meta.state_dir, &report.symbol).ok())
        .flatten()
        .map(|artifact| summarize_pda_sequence_artifact(&artifact));

    Ok(AnalyzeLiveReportingBundle {
        source_snapshot: source_snapshot.and_then(|value| serde_json::to_value(value).ok()),
        freshness_gate: freshness_gate.and_then(|value| serde_json::to_value(value).ok()),
        compact_report,
        agent_report,
        human_report,
        belief_shadow_policy,
        pda_sequence_summary,
    })
}

pub fn emit_analyze_live_output(report: &AnalyzeReport) -> Result<()> {
    emit_analyze_live_output_with_input(report, AnalyzeLiveOutputEmitInput::default())
}

pub fn emit_analyze_live_output_with_input(
    report: &AnalyzeReport,
    input: AnalyzeLiveOutputEmitInput,
) -> Result<()> {
    let bundle = build_analyze_live_reporting_bundle(
        report,
        AnalyzeLiveReportingBundleInput {
            include_pda_sequence_summary: input.include_pda_sequence_summary,
        },
    )?;
    let output = build_analyze_live_output_value(AnalyzeLiveOutputValueInput {
        report,
        source_snapshot: bundle.source_snapshot,
        freshness_gate: bundle.freshness_gate,
        compact_report: bundle.compact_report,
        agent_report: bundle.agent_report,
        human_report: &bundle.human_report,
        belief_shadow_policy: bundle.belief_shadow_policy,
        pda_sequence_summary: bundle.pda_sequence_summary,
        redact_paths: input.redact_paths,
    })?;
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

pub fn dispatch_analyze_live_output(
    report: &AnalyzeReport,
    input: AnalyzeLiveOutputDispatchInput<'_>,
) -> Result<()> {
    let bundle = build_analyze_live_reporting_bundle(
        report,
        AnalyzeLiveReportingBundleInput {
            include_pda_sequence_summary: input.include_pda_sequence_summary,
        },
    )?;
    match input.output_format.trim().to_ascii_lowercase().as_str() {
        "json" => {
            let output = build_analyze_live_output_value(AnalyzeLiveOutputValueInput {
                report,
                source_snapshot: bundle.source_snapshot,
                freshness_gate: bundle.freshness_gate,
                compact_report: bundle.compact_report,
                agent_report: bundle.agent_report,
                human_report: &bundle.human_report,
                belief_shadow_policy: bundle.belief_shadow_policy,
                pda_sequence_summary: bundle.pda_sequence_summary,
                redact_paths: input.redact_paths,
            })?;
            println!("{}", serde_json::to_string_pretty(&output)?);
            Ok(())
        }
        "compact" => print_redacted_json(&bundle.compact_report),
        "agent" => print_redacted_json(&bundle.agent_report),
        "human" => {
            let rendered = if input.redact_paths {
                redact_local_paths_in_human_text(&bundle.human_report.render())
            } else {
                bundle.human_report.render()
            };
            println!("{rendered}");
            Ok(())
        }
        other => anyhow::bail!("unsupported output format '{}'", other),
    }
}

fn human_direction_bias_label(direction: Direction) -> &'static str {
    match direction {
        Direction::Bull => "Bull bias",
        Direction::Bear => "Bear bias",
        Direction::Neutral => "Neutral bias",
    }
}

fn build_workflow_snapshot_pointer_command(symbol: &str, state_dir: &str, field: &str) -> String {
    let symbol = shell_quote(symbol);
    let state_dir = shell_quote(state_dir);
    match field {
        "actionable_artifacts" => {
            format!(
                "ict-engine workflow-status --symbol {symbol} --state-dir {state_dir} --artifacts"
            )
        }
        _ => {
            format!(
                "ict-engine workflow-status --symbol {symbol} --state-dir {state_dir} --output-format json"
            )
        }
    }
}

fn trim_workflow_snapshot_ledger_field(
    snapshot: &mut serde_json::Map<String, Value>,
    field: &str,
    symbol: &str,
    state_dir: &str,
) {
    let Some(Value::Array(items)) = snapshot.get_mut(field) else {
        return;
    };

    let total_count = items.len();
    let retained_count = total_count.min(ANALYZE_JSON_LEDGER_TAIL_DEFAULT);
    if total_count > retained_count {
        let start = total_count - retained_count;
        let tail = items.split_off(start);
        *items = tail;
    }

    snapshot.insert(
        format!("{field}_inline_meta"),
        json!({
            "inline_mode": if total_count > retained_count { "tail" } else { "full" },
            "tail_limit": ANALYZE_JSON_LEDGER_TAIL_DEFAULT,
            "retained_count": retained_count,
            "total_count": total_count,
            "omitted_count": total_count.saturating_sub(retained_count),
            "pointer_command": build_workflow_snapshot_pointer_command(symbol, state_dir, field),
        }),
    );
}

fn trim_analyze_output_workflow_snapshot_ledgers(output: &mut Value) {
    let symbol = output
        .pointer("/report/symbol")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let state_dir = output
        .pointer("/report/meta/state_dir")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let Some(snapshot) = output
        .pointer_mut("/report/supporting/workflow_snapshot")
        .and_then(Value::as_object_mut)
    else {
        return;
    };

    trim_workflow_snapshot_ledger_field(snapshot, "actionable_artifacts", &symbol, &state_dir);
    trim_workflow_snapshot_ledger_field(
        snapshot,
        "artifact_lineage_summaries",
        &symbol,
        &state_dir,
    );
}

fn human_action_line(queue: &[crate::state::FactorIterationPrompt]) -> String {
    let action = queue
        .iter()
        .find(|item| item.iteration_action != "keep" || item.replacement_candidate)
        .map(|item| {
            format!(
                "{} {}",
                item.iteration_action.to_uppercase(),
                item.factor_name
            )
        })
        .unwrap_or_else(|| "OBSERVE no_factor_change".to_string());
    format!("Action: {action}")
}

fn regime_companion_human_suffix(
    section: &crate::analyze_sections::RegimeBayesianSection,
) -> String {
    let mut fragments = Vec::new();
    if let Some(label) = &section.hybrid_regime_label {
        fragments.push(format!("hybrid_regime={label}"));
    }
    if let Some(hazard) = section.hybrid_transition_hazard {
        fragments.push(format!("hybrid_transition_hazard={hazard:.3}"));
    }
    if let Some(model) = &section.hybrid_duration_model {
        fragments.push(format!("hybrid_duration_model={model}"));
    }
    if let Some(remaining) = section.hybrid_remaining_expected_bars {
        fragments.push(format!("hybrid_remaining_expected_bars={remaining:.2}"));
    }
    if let Some(family) = &section.pda_cluster_family {
        fragments.push(format!("pda_family={family}"));
    }
    if let Some(aligned) = section.pda_hybrid_alignment {
        fragments.push(format!("pda_hybrid_alignment={aligned}"));
    }
    if fragments.is_empty() {
        String::new()
    } else {
        format!("; {}", fragments.join(" "))
    }
}

pub fn build_human_analyze_surface(input: AnalyzeHumanInput<'_>) -> HumanAnalyzeReport {
    let regime_companion_suffix = input.regime_companion_suffix.unwrap_or("");
    let market_family_prefix = input
        .market_family
        .map(|family| format!("market_family={family} "))
        .unwrap_or_default();
    let regime_bayes_analysis = match input.objective_jump_weight {
        Some(weight) => format!(
            "{market_family_prefix}regime={} liquidity={} direction={:?} subgraph={} objective_jump_weight={weight:.3}{}",
            input.regime_label,
            input.liquidity_label,
            input.regime_selected_direction,
            input.market_subgraph,
            regime_companion_suffix
        ),
        None => format!(
            "{market_family_prefix}regime={} liquidity={} direction={:?} subgraph={}{}",
            input.regime_label,
            input.liquidity_label,
            input.regime_selected_direction,
            input.market_subgraph,
            regime_companion_suffix
        ),
    };

    build_human_analyze_report(
        Some(format!(
            "{} | {} | entry={} | gate={} | quality={:.3}",
            input.symbol,
            human_direction_bias_label(input.selected_direction),
            input.entry_quality,
            input.gate_status,
            input.evidence_quality_score
        )),
        Some(format!(
            "Decision: {}",
            humanize_decision_hint(input.decision_hint)
        )),
        Some(human_action_line(input.factor_iteration_queue)),
        Some(format!(
            "Next: {}",
            humanize_next_step_line(input.recommended_next_command)
        )),
        input.price_action_narrative.to_string(),
        input.technical_price_narrative.to_string(),
        input.smt_correlation_narrative.to_string(),
        regime_bayes_analysis,
        input.trade_plan_narrative,
    )
}

pub fn build_analyze_output_envelope<R, E>(
    input: AnalyzeOutputEnvelopeInput<'_, R, E>,
) -> AnalyzeOutputEnvelope<R, E>
where
    R: Serialize,
    E: Serialize,
{
    let AnalyzeOutputEnvelopeInput {
        report,
        compact_report,
        agent_report,
        human_report,
        market_family_summary,
        belief_shadow_policy,
        belief_policy_lineage,
        ensemble_vote,
        pda_sequence_summary,
        executor_scorecard_source,
    } = input;
    let ensemble_value = serde_json::to_value(&ensemble_vote).unwrap_or_default();
    let executor_summaries = ensemble_value
        .get("executor_summaries")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let policy_runtime_summary = ensemble_value
        .get("policy_runtime_sources")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|value| !value.is_empty());
    let report_value = serde_json::to_value(&report).unwrap_or_default();
    let ranker_validation_summary = report_value
        .get("supporting")
        .and_then(|value| value.get("workflow_snapshot"))
        .and_then(|value| value.get("policy_training_status"))
        .and_then(|value| value.get("structural_path_ranking_validation_summary"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.trim().is_empty());
    let ranker_runtime_summary = report_value
        .get("supporting")
        .and_then(|value| value.get("workflow_snapshot"))
        .and_then(|value| value.get("policy_training_status"))
        .and_then(|value| value.get("structural_path_ranking_runtime_summary"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.trim().is_empty());

    AnalyzeOutputEnvelope {
        execution_triage: None,
        report,
        compact_report,
        agent_report,
        human_report: human_report.render(),
        market_family_summary,
        belief_shadow_policy,
        belief_policy_lineage,
        ensemble_vote,
        pda_sequence_summary,
        executor_scorecard_summary: {
            let mut lines = format_executor_summary_lines(&executor_summaries);
            if let Some(summary) = policy_runtime_summary {
                lines.push(format!("policy_runtime={summary}"));
            }
            if let Some(summary) = ranker_runtime_summary {
                lines.push(format!("ranker_runtime={summary}"));
            }
            if let Some(summary) = ranker_validation_summary {
                lines.push(format!("ranker_validation={summary}"));
            }
            lines
        },
        executor_scorecard_source,
    }
}

pub fn build_analyze_live_output_envelope<R, S, F>(
    input: AnalyzeLiveOutputEnvelopeInput<'_, R, S, F>,
) -> AnalyzeLiveOutputEnvelope<R>
where
    R: Serialize,
    S: Serialize,
    F: Serialize,
{
    let AnalyzeLiveOutputEnvelopeInput {
        report,
        source_snapshot,
        freshness_gate,
        compact_report,
        agent_report,
        human_report,
        belief_shadow_policy,
        pda_sequence_summary,
    } = input;
    AnalyzeLiveOutputEnvelope {
        execution_triage: None,
        report,
        source_snapshot: source_snapshot.and_then(|value| serde_json::to_value(value).ok()),
        freshness_gate: freshness_gate.and_then(|value| serde_json::to_value(value).ok()),
        compact_report,
        agent_report,
        human_report: human_report.render(),
        belief_shadow_policy,
        pda_sequence_summary,
    }
}

pub fn build_analyze_live_output_value<R, S, F>(
    input: AnalyzeLiveOutputValueInput<'_, R, S, F>,
) -> Result<serde_json::Value>
where
    R: Serialize,
    S: Serialize,
    F: Serialize,
{
    let AnalyzeLiveOutputValueInput {
        report,
        source_snapshot,
        freshness_gate,
        compact_report,
        agent_report,
        human_report,
        belief_shadow_policy,
        pda_sequence_summary,
        redact_paths,
    } = input;
    let mut output = serde_json::to_value(build_analyze_live_output_envelope(
        AnalyzeLiveOutputEnvelopeInput {
            report,
            source_snapshot,
            freshness_gate,
            compact_report,
            agent_report,
            human_report,
            belief_shadow_policy,
            pda_sequence_summary,
        },
    ))?;
    if redact_paths {
        redact_local_paths_in_value(&mut output);
        if let Some(value) = output.get_mut("human_report") {
            *value = Value::String(redact_local_paths_in_human_text(&human_report.render()));
        }
    }
    Ok(output)
}

pub fn build_analyze_output_value<R, E>(input: AnalyzeOutputValueInput<'_, R, E>) -> Result<Value>
where
    R: Serialize,
    E: Serialize,
{
    let AnalyzeOutputValueInput {
        report,
        compact_report,
        agent_report,
        human_report,
        market_family_summary,
        belief_shadow_policy,
        belief_policy_lineage,
        ensemble_vote,
        pda_sequence_summary,
        executor_scorecard_source,
        inline_ledger,
        redact_paths,
    } = input;
    let mut output =
        serde_json::to_value(build_analyze_output_envelope(AnalyzeOutputEnvelopeInput {
            report,
            compact_report,
            agent_report,
            human_report,
            market_family_summary,
            belief_shadow_policy,
            belief_policy_lineage,
            ensemble_vote,
            pda_sequence_summary,
            executor_scorecard_source,
        }))?;
    if !inline_ledger {
        trim_analyze_output_workflow_snapshot_ledgers(&mut output);
    }
    if redact_paths {
        redact_local_paths_in_value(&mut output);
        if let Some(value) = output.get_mut("human_report") {
            *value = Value::String(redact_local_paths_in_human_text(&human_report.render()));
        }
    }
    Ok(output)
}

pub fn emit_analyze_output_envelope<R, E>(
    input: EmitAnalyzeOutputEnvelopeInput<'_, R, E>,
) -> Result<()>
where
    R: Serialize,
    E: Serialize,
{
    let EmitAnalyzeOutputEnvelopeInput {
        report,
        output_format,
        inline_ledger,
        compact_report,
        agent_report,
        human_report,
        market_family_summary,
        belief_shadow_policy,
        belief_policy_lineage,
        ensemble_vote,
        pda_sequence_summary,
        executor_scorecard_source,
    } = input;
    match output_format.trim().to_ascii_lowercase().as_str() {
        "json" => {
            let output = build_analyze_output_value(AnalyzeOutputValueInput {
                report,
                compact_report: compact_report.clone(),
                agent_report: agent_report.clone(),
                human_report,
                market_family_summary,
                belief_shadow_policy,
                belief_policy_lineage,
                ensemble_vote,
                pda_sequence_summary,
                executor_scorecard_source,
                inline_ledger,
                redact_paths: true,
            })?;
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        "compact" => print_redacted_json(compact_report)?,
        "agent" => print_redacted_json(agent_report)?,
        "human" => println!(
            "{}",
            redact_local_paths_in_human_text(&human_report.render())
        ),
        other => anyhow::bail!("unsupported output format '{}'", other),
    }
    Ok(())
}

pub fn emit_analyze_live_output_envelope<R, S, F>(
    input: EmitAnalyzeLiveOutputEnvelopeInput<'_, R, S, F>,
) -> Result<()>
where
    R: Serialize,
    S: Serialize,
    F: Serialize,
{
    let EmitAnalyzeLiveOutputEnvelopeInput {
        report,
        source_snapshot,
        freshness_gate,
        compact_report,
        agent_report,
        human_report,
        belief_shadow_policy,
        pda_sequence_summary,
    } = input;
    let output = build_analyze_live_output_value(AnalyzeLiveOutputValueInput {
        report,
        source_snapshot,
        freshness_gate,
        compact_report,
        agent_report,
        human_report,
        belief_shadow_policy,
        pda_sequence_summary,
        redact_paths: true,
    })?;
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::reporting::{
        build_agent_guidance_report, build_compact_analyze_report,
    };

    #[derive(Debug, Clone, Serialize)]
    struct StubReport {
        symbol: String,
        path: Option<String>,
    }

    #[derive(Debug, Clone, Serialize)]
    struct StubEnsembleVote {
        executor_summaries: Vec<String>,
        final_action: String,
        policy_runtime_sources: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize)]
    struct StubSnapshot {
        status: String,
    }

    #[test]
    fn dispatch_analyze_output_input_preserves_output_format() {
        let input = AnalyzeOutputDispatchInput {
            output_format: "agent",
            inline_ledger: true,
        };
        assert_eq!(input.output_format, "agent");
        assert!(input.inline_ledger);
    }

    #[test]
    fn analyze_live_output_dispatch_input_default_constructs() {
        let input = AnalyzeLiveOutputDispatchInput::default();
        assert_eq!(input.output_format, "json");
        assert!(input.include_pda_sequence_summary);
        assert!(input.redact_paths);
    }

    #[test]
    fn build_analyze_output_envelope_collects_executor_summary() {
        let report = StubReport {
            symbol: "NQ".to_string(),
            path: None,
        };
        let report_value = serde_json::json!({
            "symbol": report.symbol,
            "path": report.path,
            "supporting": {
                "workflow_snapshot": {
                    "policy_training_status": {
                        "structural_path_ranking_runtime_summary": "Ranker runtime: runtime enabled=false ready=false source=none status=disabled mode=none matches=0",
                        "structural_path_ranking_validation_summary": "Ranker validation: calibration=true quality_ready=true raw_scored_mature=30/30 production_validation=30/30 observation_validation=0/30 ready=true"
                    }
                }
            }
        });
        let compact_report = build_compact_analyze_report(
            "observe_only",
            Some("Bull".to_string()),
            Some("medium".to_string()),
            Some("pass_neutralized".to_string()),
            Some("ict-engine analyze".to_string()),
            &[],
            &[],
            &[],
        );
        let agent_report = build_agent_guidance_report(
            Some("Bull".to_string()),
            Some("medium".to_string()),
            Some("pass_neutralized".to_string()),
            Some("ict-engine analyze".to_string()),
            Some("observe_only".to_string()),
            &[],
            &[],
            &[],
        );
        let human_report = build_human_analyze_surface(AnalyzeHumanInput {
            symbol: "NQ",
            selected_direction: Direction::Bull,
            entry_quality: "medium",
            gate_status: "pass_neutralized",
            evidence_quality_score: 0.5,
            decision_hint: "observe_only",
            factor_iteration_queue: &[],
            recommended_next_command: "ict-engine analyze",
            price_action_narrative: "price",
            technical_price_narrative: "tech",
            smt_correlation_narrative: "smt",
            regime_label: "trend",
            liquidity_label: "sweep",
            regime_selected_direction: Direction::Bull,
            trade_plan_narrative: "plan",
            market_family: Some("futures_index"),
            market_subgraph: "index_beta",
            objective_jump_weight: Some(0.25),
            regime_companion_suffix: None,
        });
        let vote = StubEnsembleVote {
            executor_summaries: vec![
                "executor=catboost_file action=observe confidence=0.55 weight=0.55".to_string(),
            ],
            final_action: "observe".to_string(),
            policy_runtime_sources: vec![
                "catboost-compatible-placeholder:placeholder".to_string(),
                "catboost-compatible-placeholder:artifact".to_string(),
            ],
        };

        let output = build_analyze_output_envelope(AnalyzeOutputEnvelopeInput {
            report: report_value,
            compact_report,
            agent_report,
            human_report: &human_report,
            market_family_summary: AnalyzeMarketFamilySummary {
                market_family: Some("futures_index".to_string()),
                market_behavior_profile: Some("index_beta_regime_sensitive".to_string()),
                selected_market_subgraph: Some("index_beta".to_string()),
            },
            belief_shadow_policy: BeliefShadowPolicySurface::default(),
            belief_policy_lineage: BeliefPolicyLineageSurface::default(),
            ensemble_vote: vote,
            pda_sequence_summary: None,
            executor_scorecard_source: "persisted".to_string(),
        });

        assert_eq!(output.executor_scorecard_summary.len(), 4);
        assert_eq!(output.executor_scorecard_source, "persisted");
        assert!(output
            .executor_scorecard_summary
            .iter()
            .any(|line| line.contains("policy_runtime=")));
        assert!(output
            .executor_scorecard_summary
            .iter()
            .any(|line| line.contains("ranker_runtime=")));
        assert!(output
            .executor_scorecard_summary
            .iter()
            .any(|line| line.contains("ranker_validation=")));
        assert!(output.human_report.contains("Plan:"));
    }

    #[test]
    fn build_analyze_output_envelope_includes_pda_sequence_summary_when_present() {
        let report = StubReport {
            symbol: "NQ".to_string(),
            path: None,
        };
        let output = build_analyze_output_envelope(AnalyzeOutputEnvelopeInput {
            report,
            compact_report: build_compact_analyze_report(
                "observe_only",
                None,
                None,
                None,
                None,
                &[],
                &[],
                &[],
            ),
            agent_report: build_agent_guidance_report(None, None, None, None, None, &[], &[], &[]),
            human_report: &build_human_analyze_surface(AnalyzeHumanInput {
                symbol: "NQ",
                selected_direction: Direction::Bull,
                entry_quality: "medium",
                gate_status: "pass_neutralized",
                evidence_quality_score: 0.5,
                decision_hint: "observe_only",
                factor_iteration_queue: &[],
                recommended_next_command: "ict-engine analyze",
                price_action_narrative: "price",
                technical_price_narrative: "tech",
                smt_correlation_narrative: "smt",
                regime_label: "trend",
                liquidity_label: "sweep",
                regime_selected_direction: Direction::Bull,
                trade_plan_narrative: "plan",
                market_family: Some("futures_index"),
                market_subgraph: "index_beta",
                objective_jump_weight: Some(0.25),
                regime_companion_suffix: None,
            }),
            market_family_summary: AnalyzeMarketFamilySummary {
                market_family: Some("futures_index".to_string()),
                market_behavior_profile: Some("index_beta_regime_sensitive".to_string()),
                selected_market_subgraph: Some("index_beta".to_string()),
            },
            belief_shadow_policy: BeliefShadowPolicySurface::default(),
            belief_policy_lineage: BeliefPolicyLineageSurface::default(),
            ensemble_vote: StubEnsembleVote {
                executor_summaries: Vec::new(),
                final_action: "observe".to_string(),
                policy_runtime_sources: Vec::new(),
            },
            pda_sequence_summary: Some(PdaSequenceArtifactSummary {
                method: "pda_sequence_analysis_v2".to_string(),
                primary_cluster: Some(1),
                primary_cluster_label: Some("cluster_1".to_string()),
                primary_cluster_family: Some("trend".to_string()),
                primary_cluster_confidence: Some(0.88),
                consistency_ratio: 0.75,
                ensemble_mean_confidence: 0.83,
                valid_sessions: 8,
                kmer_k: 2,
            }),
            executor_scorecard_source: "persisted".to_string(),
        });
        assert_eq!(
            output
                .pda_sequence_summary
                .as_ref()
                .and_then(|summary| summary.primary_cluster_label.as_deref()),
            Some("cluster_1")
        );
    }

    #[test]
    fn analyze_human_surface_does_not_leak_ask_user_wire_protocol() {
        let report = build_human_analyze_surface(AnalyzeHumanInput {
            symbol: "NQ",
            selected_direction: Direction::Bull,
            entry_quality: "medium",
            gate_status: "observe_only",
            evidence_quality_score: 0.5,
            decision_hint: "observe_only",
            factor_iteration_queue: &[],
            recommended_next_command: "ask-user: Before using historical data for NQ again, ask the user which dataset to use. recorded_paths=/tmp/a.json | blocked until user_selected_historical_data | then ict-engine factor-research --symbol NQ --data /tmp/a.json --state-dir state",
            price_action_narrative: "price",
            technical_price_narrative: "tech",
            smt_correlation_narrative: "smt",
            regime_label: "trend",
            liquidity_label: "sweep",
            regime_selected_direction: Direction::Bull,
            trade_plan_narrative: "plan",
            market_family: None,
            market_subgraph: "unknown",
            objective_jump_weight: None,
            regime_companion_suffix: None,
        });
        let rendered = report.render();
        assert!(
            !rendered.contains("ask-user:"),
            "human output must not leak wire protocol:\n{rendered}"
        );
        assert!(
            rendered.contains("Ask the user"),
            "human output should surface user-readable ask; got:\n{rendered}"
        );
    }

    #[test]
    fn build_analyze_live_output_envelope_serializes_optional_surfaces() {
        let report = StubReport {
            symbol: "NQ".to_string(),
            path: None,
        };
        let human_report = build_human_analyze_surface(AnalyzeHumanInput {
            symbol: "NQ",
            selected_direction: Direction::Bull,
            entry_quality: "medium",
            gate_status: "pass_neutralized",
            evidence_quality_score: 0.5,
            decision_hint: "observe_only",
            factor_iteration_queue: &[],
            recommended_next_command: "ict-engine analyze",
            price_action_narrative: "price",
            technical_price_narrative: "tech",
            smt_correlation_narrative: "smt",
            regime_label: "trend",
            liquidity_label: "sweep",
            regime_selected_direction: Direction::Bull,
            trade_plan_narrative: "plan",
            market_family: None,
            market_subgraph: "unknown",
            objective_jump_weight: None,
            regime_companion_suffix: None,
        });
        let output = build_analyze_live_output_envelope(AnalyzeLiveOutputEnvelopeInput {
            report,
            source_snapshot: Some(StubSnapshot {
                status: "fresh".to_string(),
            }),
            freshness_gate: Some(StubSnapshot {
                status: "ok".to_string(),
            }),
            compact_report: build_compact_analyze_report(
                "observe_only",
                None,
                None,
                None,
                None,
                &[],
                &[],
                &[],
            ),
            agent_report: build_agent_guidance_report(None, None, None, None, None, &[], &[], &[]),
            human_report: &human_report,
            belief_shadow_policy: BeliefShadowPolicySurface::default(),
            pda_sequence_summary: None,
        });

        assert_eq!(output.source_snapshot.unwrap()["status"], "fresh");
        assert_eq!(output.freshness_gate.unwrap()["status"], "ok");
        assert!(output.human_report.contains("Plan:"));
    }

    #[test]
    fn build_analyze_live_output_value_respects_redaction_flag() {
        let report = StubReport {
            symbol: "NQ".to_string(),
            path: Some("/tmp/ict-live-state/report.json".to_string()),
        };
        let human_report = build_human_analyze_surface(AnalyzeHumanInput {
            symbol: "NQ",
            selected_direction: Direction::Bull,
            entry_quality: "medium",
            gate_status: "pass_neutralized",
            evidence_quality_score: 0.5,
            decision_hint: "observe_only",
            factor_iteration_queue: &[],
            recommended_next_command: "ict-engine analyze",
            price_action_narrative: "price",
            technical_price_narrative: "tech",
            smt_correlation_narrative: "smt",
            regime_label: "trend",
            liquidity_label: "sweep",
            regime_selected_direction: Direction::Bull,
            trade_plan_narrative: "plan",
            market_family: None,
            market_subgraph: "unknown",
            objective_jump_weight: None,
            regime_companion_suffix: None,
        });
        let raw_output = build_analyze_live_output_value(AnalyzeLiveOutputValueInput {
            report: report.clone(),
            source_snapshot: Some(StubSnapshot {
                status: "/tmp/ict-live-state/fresh.json".to_string(),
            }),
            freshness_gate: None::<StubSnapshot>,
            compact_report: build_compact_analyze_report(
                "observe_only",
                None,
                None,
                None,
                None,
                &[],
                &[],
                &[],
            ),
            agent_report: build_agent_guidance_report(None, None, None, None, None, &[], &[], &[]),
            human_report: &human_report,
            belief_shadow_policy: BeliefShadowPolicySurface::default(),
            pda_sequence_summary: None,
            redact_paths: false,
        })
        .unwrap();
        let redacted_output = build_analyze_live_output_value(AnalyzeLiveOutputValueInput {
            report,
            source_snapshot: Some(StubSnapshot {
                status: "/tmp/ict-live-state/fresh.json".to_string(),
            }),
            freshness_gate: None::<StubSnapshot>,
            compact_report: build_compact_analyze_report(
                "observe_only",
                None,
                None,
                None,
                None,
                &[],
                &[],
                &[],
            ),
            agent_report: build_agent_guidance_report(None, None, None, None, None, &[], &[], &[]),
            human_report: &human_report,
            belief_shadow_policy: BeliefShadowPolicySurface::default(),
            pda_sequence_summary: None,
            redact_paths: true,
        })
        .unwrap();

        assert_eq!(
            raw_output["report"]["path"],
            "/tmp/ict-live-state/report.json"
        );
        assert_eq!(
            raw_output["source_snapshot"]["status"],
            "/tmp/ict-live-state/fresh.json"
        );
        assert_eq!(redacted_output["report"]["path"], "<local-path>");
        assert_eq!(redacted_output["source_snapshot"]["status"], "<local-path>");
    }

    #[test]
    fn build_analyze_output_value_trims_workflow_snapshot_ledgers_by_default() {
        let output = build_analyze_output_value(AnalyzeOutputValueInput {
            report: json!({
                "symbol": "NQ",
                "meta": {
                    "state_dir": "state"
                },
                "supporting": {
                    "workflow_snapshot": {
                        "actionable_artifacts": (0..7)
                            .map(|index| json!({ "id": format!("artifact-{index}") }))
                            .collect::<Vec<_>>(),
                        "artifact_lineage_summaries": (0..9)
                            .map(|index| json!({ "id": format!("lineage-{index}") }))
                            .collect::<Vec<_>>()
                    }
                }
            }),
            compact_report: build_compact_analyze_report(
                "observe_only",
                None,
                None,
                None,
                None,
                &[],
                &[],
                &[],
            ),
            agent_report: build_agent_guidance_report(None, None, None, None, None, &[], &[], &[]),
            human_report: &build_human_analyze_surface(AnalyzeHumanInput {
                symbol: "NQ",
                selected_direction: Direction::Bull,
                entry_quality: "medium",
                gate_status: "pass_neutralized",
                evidence_quality_score: 0.5,
                decision_hint: "observe_only",
                factor_iteration_queue: &[],
                recommended_next_command: "ict-engine analyze",
                price_action_narrative: "price",
                technical_price_narrative: "tech",
                smt_correlation_narrative: "smt",
                regime_label: "trend",
                liquidity_label: "sweep",
                regime_selected_direction: Direction::Bull,
                trade_plan_narrative: "plan",
                market_family: None,
                market_subgraph: "unknown",
                objective_jump_weight: None,
                regime_companion_suffix: None,
            }),
            market_family_summary: AnalyzeMarketFamilySummary {
                market_family: None,
                market_behavior_profile: None,
                selected_market_subgraph: None,
            },
            belief_shadow_policy: BeliefShadowPolicySurface::default(),
            belief_policy_lineage: BeliefPolicyLineageSurface::default(),
            ensemble_vote: StubEnsembleVote {
                executor_summaries: Vec::new(),
                final_action: "observe".to_string(),
                policy_runtime_sources: Vec::new(),
            },
            pda_sequence_summary: None,
            executor_scorecard_source: "persisted".to_string(),
            inline_ledger: false,
            redact_paths: false,
        })
        .unwrap();

        assert_eq!(
            output["report"]["supporting"]["workflow_snapshot"]["actionable_artifacts"]
                .as_array()
                .unwrap()
                .len(),
            ANALYZE_JSON_LEDGER_TAIL_DEFAULT
        );
        assert_eq!(
            output["report"]["supporting"]["workflow_snapshot"]["artifact_lineage_summaries"]
                .as_array()
                .unwrap()
                .len(),
            ANALYZE_JSON_LEDGER_TAIL_DEFAULT
        );
        assert_eq!(
            output["report"]["supporting"]["workflow_snapshot"]["actionable_artifacts_inline_meta"]
                ["total_count"],
            7
        );
        assert_eq!(
            output["report"]["supporting"]["workflow_snapshot"]
                ["artifact_lineage_summaries_inline_meta"]["omitted_count"],
            4
        );
        assert_eq!(
            output["report"]["supporting"]["workflow_snapshot"]
                ["artifact_lineage_summaries_inline_meta"]["pointer_command"],
            "ict-engine workflow-status --symbol NQ --state-dir state --output-format json"
        );
    }

    #[test]
    fn build_analyze_output_value_keeps_full_ledgers_when_requested() {
        let output = build_analyze_output_value(AnalyzeOutputValueInput {
            report: json!({
                "symbol": "NQ",
                "meta": {
                    "state_dir": "state"
                },
                "supporting": {
                    "workflow_snapshot": {
                        "actionable_artifacts": (0..7)
                            .map(|index| json!({ "id": format!("artifact-{index}") }))
                            .collect::<Vec<_>>(),
                        "artifact_lineage_summaries": (0..9)
                            .map(|index| json!({ "id": format!("lineage-{index}") }))
                            .collect::<Vec<_>>()
                    }
                }
            }),
            compact_report: build_compact_analyze_report(
                "observe_only",
                None,
                None,
                None,
                None,
                &[],
                &[],
                &[],
            ),
            agent_report: build_agent_guidance_report(None, None, None, None, None, &[], &[], &[]),
            human_report: &build_human_analyze_surface(AnalyzeHumanInput {
                symbol: "NQ",
                selected_direction: Direction::Bull,
                entry_quality: "medium",
                gate_status: "pass_neutralized",
                evidence_quality_score: 0.5,
                decision_hint: "observe_only",
                factor_iteration_queue: &[],
                recommended_next_command: "ict-engine analyze",
                price_action_narrative: "price",
                technical_price_narrative: "tech",
                smt_correlation_narrative: "smt",
                regime_label: "trend",
                liquidity_label: "sweep",
                regime_selected_direction: Direction::Bull,
                trade_plan_narrative: "plan",
                market_family: None,
                market_subgraph: "unknown",
                objective_jump_weight: None,
                regime_companion_suffix: None,
            }),
            market_family_summary: AnalyzeMarketFamilySummary {
                market_family: None,
                market_behavior_profile: None,
                selected_market_subgraph: None,
            },
            belief_shadow_policy: BeliefShadowPolicySurface::default(),
            belief_policy_lineage: BeliefPolicyLineageSurface::default(),
            ensemble_vote: StubEnsembleVote {
                executor_summaries: Vec::new(),
                final_action: "observe".to_string(),
                policy_runtime_sources: Vec::new(),
            },
            pda_sequence_summary: None,
            executor_scorecard_source: "persisted".to_string(),
            inline_ledger: true,
            redact_paths: false,
        })
        .unwrap();

        assert_eq!(
            output["report"]["supporting"]["workflow_snapshot"]["actionable_artifacts"]
                .as_array()
                .unwrap()
                .len(),
            7
        );
        assert!(output["report"]["supporting"]["workflow_snapshot"]
            ["actionable_artifacts_inline_meta"]
            .is_null());
    }
}
