use std::collections::BTreeMap;

use crate::application::backtest::render_recommended_command;
use crate::config::{family_history_window, shell_quote};
use crate::state::{
    AgentContextBundle, AgentContextBundleMinimal, CommandRecommendations, DatasetComparability,
    FactorFamilyOutcome, FactorIterationPrompt, PreBayesEvidenceFilter, StageAgentContext,
    StageAgentContextMinimal,
};

pub struct BuildAgentContextBundleInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
    pub workflow_state: &'a crate::state::WorkflowState,
    pub decision_hint: &'a str,
    pub recommended_next_command: &'a str,
    pub recommended_commands: &'a CommandRecommendations,
    pub dataset_comparability: &'a DatasetComparability,
    pub factor_iteration_queue: &'a [FactorIterationPrompt],
    pub family_outcomes: &'a [FactorFamilyOutcome],
    pub pre_bayes_evidence_filter: Option<&'a PreBayesEvidenceFilter>,
    pub pre_bayes_entry_quality_bridge: Option<&'a crate::state::PreBayesEntryQualityBridge>,
    pub pda_sequence_summary: Option<&'a crate::pda_sequence::PdaSequenceArtifactSummary>,
    pub factor_mutation_evaluation: Option<&'a crate::state::FactorMutationEvaluation>,
    pub artifact_decision_summary: Option<&'a crate::state::ArtifactDecisionSummary>,
}

pub fn build_agent_context_bundle(input: BuildAgentContextBundleInput<'_>) -> AgentContextBundle {
    let BuildAgentContextBundleInput {
        symbol,
        state_dir,
        workflow_state,
        decision_hint,
        recommended_next_command,
        recommended_commands,
        dataset_comparability,
        factor_iteration_queue,
        family_outcomes,
        pre_bayes_evidence_filter,
        pre_bayes_entry_quality_bridge,
        pda_sequence_summary,
        factor_mutation_evaluation,
        artifact_decision_summary,
    } = input;
    let pre_bayes_gate_status = pre_bayes_evidence_filter
        .map(|filter| filter.gating_status.clone())
        .unwrap_or_default();
    let pre_bayes_uses_soft_evidence = pre_bayes_evidence_filter
        .map(|filter| filter.uses_soft_evidence)
        .unwrap_or(false);
    let pre_bayes_evidence_quality_score = pre_bayes_evidence_filter
        .map(|filter| filter.evidence_quality_score)
        .unwrap_or_default();
    let pre_bayes_conflict_flags = pre_bayes_evidence_filter
        .map(|filter| filter.conflict_flags.clone())
        .unwrap_or_default();
    let pre_bayes_filtered_assignments = pre_bayes_evidence_filter
        .map(|filter| filter.evidence_assignments.clone())
        .unwrap_or_default();
    let pre_bayes_soft_evidence = pre_bayes_evidence_filter
        .map(|filter| {
            BTreeMap::from([
                (
                    "market_regime".to_string(),
                    filter.soft_market_regime_distribution.clone(),
                ),
                (
                    "liquidity_context".to_string(),
                    filter.soft_liquidity_context_distribution.clone(),
                ),
                (
                    "factor_alignment".to_string(),
                    filter.soft_factor_alignment_distribution.clone(),
                ),
                (
                    "factor_uncertainty".to_string(),
                    filter.soft_factor_uncertainty_distribution.clone(),
                ),
                (
                    "multi_timeframe_resonance".to_string(),
                    filter.soft_multi_timeframe_resonance_distribution.clone(),
                ),
            ])
        })
        .unwrap_or_default();
    let pre_bayes_policy_version = pre_bayes_evidence_filter
        .map(|filter| filter.policy.version.clone())
        .unwrap_or_default();
    let pre_bayes_multi_timeframe_direction_bias = pre_bayes_evidence_filter
        .map(|filter| filter.filtered_multi_timeframe_direction_bias.clone())
        .unwrap_or_default();
    let pre_bayes_multi_timeframe_alignment_score = pre_bayes_evidence_filter
        .and_then(|filter| filter.filtered_multi_timeframe_alignment_score);
    let pre_bayes_multi_timeframe_entry_alignment_score = pre_bayes_evidence_filter
        .and_then(|filter| filter.filtered_multi_timeframe_entry_alignment_score);
    let pre_bayes_soft_evidence_diff = pre_bayes_evidence_filter
        .map(pre_bayes_soft_evidence_diff)
        .unwrap_or_default();
    let pre_bayes_entry_quality_bridge_diff_summary =
        pre_bayes_entry_quality_bridge.map(pre_bayes_entry_quality_bridge_diff);
    let pre_bayes_entry_quality_bridge_summary = pre_bayes_entry_quality_bridge
        .map(|bridge| {
            let bridge_diff = pre_bayes_entry_quality_bridge_diff(bridge);
            let mut summary = bridge.rationale.clone();
            summary.push(format!(
                "long_signal_probability={:.3} short_signal_probability={:.3}",
                bridge.long_signal_probability, bridge.short_signal_probability
            ));
            summary.push(format!(
                "selected_entry_quality={:?} selected_probability={:.3} probability_gap={:.3}",
                bridge_diff.selected_entry_quality,
                bridge_diff.selected_entry_quality_probability,
                bridge_diff.long_short_signal_probability_gap
            ));
            summary.push(format!(
                "multi_timeframe_direction_bias={} multi_timeframe_alignment_score={:.3} multi_timeframe_entry_alignment_score={:.3}",
                bridge_diff.multi_timeframe_direction_bias,
                bridge_diff.multi_timeframe_alignment_score.unwrap_or_default(),
                bridge_diff
                    .multi_timeframe_entry_alignment_score
                    .unwrap_or_default()
            ));
            summary
        })
        .unwrap_or_default();
    let factor_mutation_priority_markets = factor_mutation_evaluation
        .map(factor_mutation_priority_markets)
        .unwrap_or_default();
    let factor_mutation_priority_reasons = factor_mutation_evaluation
        .map(factor_mutation_priority_reasons)
        .unwrap_or_default();
    let factor_mutation_recommended_focus = factor_mutation_evaluation
        .map(|evaluation| {
            let mut focus = evaluation.recommended_mutation_directions.clone();
            focus.truncate(3);
            focus
        })
        .unwrap_or_default();
    let artifact_gate_status = artifact_decision_summary
        .map(|summary| summary.consumed_trend_status.clone())
        .unwrap_or_default();
    let artifact_gate_reason = artifact_decision_summary
        .map(|summary| summary.consumed_trend_reason.clone())
        .unwrap_or_default();
    let artifact_gate_targets = artifact_decision_summary
        .map(|summary| summary.consumed_target_kinds.clone())
        .unwrap_or_default();

    AgentContextBundle {
        workflow_state: workflow_state.clone(),
        decision_hint: decision_hint.to_string(),
        recommended_next_command: recommended_next_command.to_string(),
        recommended_next_command_meta: crate::state::recommended_next_command_meta(
            recommended_next_command,
        ),
        recommended_commands: recommended_commands.clone(),
        family_history_window: family_history_window(),
        comparable_to_last_run: dataset_comparability.comparable,
        pre_bayes_gate_status,
        pre_bayes_uses_soft_evidence,
        pre_bayes_evidence_quality_score,
        pre_bayes_conflict_flags,
        pre_bayes_filtered_assignments,
        pre_bayes_soft_evidence,
        pre_bayes_policy_version,
        pre_bayes_multi_timeframe_direction_bias,
        pre_bayes_multi_timeframe_alignment_score,
        pre_bayes_multi_timeframe_entry_alignment_score,
        pre_bayes_entry_quality_bridge_summary,
        pre_bayes_soft_evidence_diff,
        pre_bayes_entry_quality_bridge_diff: pre_bayes_entry_quality_bridge_diff_summary,
        factor_mutation_evaluation: factor_mutation_evaluation.cloned(),
        factor_mutation_priority_markets,
        factor_mutation_priority_reasons,
        factor_mutation_recommended_focus,
        factor_mutation_direction_hints: Vec::new(),
        factor_mutation_step_size_hints: Vec::new(),
        multi_timeframe_summary: Vec::new(),
        artifact_consumed_gate_status: artifact_gate_status,
        artifact_consumed_gate_reason: artifact_gate_reason,
        artifact_consumed_gate_targets: artifact_gate_targets,
        pda_sequence_summary: pda_sequence_summary.map(|summary| {
            format!(
                "pda_sequence method={} primary_cluster={} confidence={:.3} consistency={:.3}",
                summary.method,
                summary
                    .primary_cluster_label
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                summary.primary_cluster_confidence.unwrap_or_default(),
                summary.consistency_ratio,
            )
        }),
        pda_cluster_label: pda_sequence_summary
            .and_then(|summary| summary.primary_cluster_label.clone()),
        pda_cluster_confidence: pda_sequence_summary
            .and_then(|summary| summary.primary_cluster_confidence),
        top_factor_actions: factor_iteration_queue
            .iter()
            .take(3)
            .map(|item| {
                format!(
                    "{}:{}:{:.2}",
                    item.factor_name, item.iteration_action, item.composite_score
                )
            })
            .collect(),
        family_actions: family_outcomes
            .iter()
            .map(|item| {
                format!(
                    "{}:{}:{}",
                    item.family, item.promotion_decision.status, item.rollback_recommendation.scope
                )
            })
            .collect(),
        stage_views: build_stage_views(
            symbol,
            state_dir,
            recommended_commands,
            factor_iteration_queue,
            family_outcomes,
            pre_bayes_evidence_filter,
            artifact_decision_summary,
        ),
    }
}

pub fn build_agent_context_bundle_minimal(
    bundle: &AgentContextBundle,
) -> AgentContextBundleMinimal {
    AgentContextBundleMinimal {
        workflow_phase: bundle.workflow_state.phase.clone(),
        recommended_next_command: bundle.recommended_next_command.clone(),
        recommended_next_command_meta: bundle.recommended_next_command_meta.clone(),
        family_history_window: bundle.family_history_window,
        comparable_to_last_run: bundle.comparable_to_last_run,
        pre_bayes_gate_status: bundle.pre_bayes_gate_status.clone(),
        pre_bayes_uses_soft_evidence: bundle.pre_bayes_uses_soft_evidence,
        pre_bayes_policy_version: bundle.pre_bayes_policy_version.clone(),
        pre_bayes_soft_evidence_divergence_count: bundle
            .pre_bayes_soft_evidence_diff
            .iter()
            .filter(|item| item.diverges_from_filtered_state)
            .count(),
        pre_bayes_bridge_selected_entry_quality: bundle
            .pre_bayes_entry_quality_bridge_diff
            .as_ref()
            .and_then(|diff| diff.selected_entry_quality.clone())
            .unwrap_or_default(),
        factor_mutation_acceptance_status: bundle
            .factor_mutation_evaluation
            .as_ref()
            .map(|evaluation| {
                if evaluation.accepted {
                    "accepted".to_string()
                } else {
                    "rejected".to_string()
                }
            })
            .unwrap_or_default(),
        factor_mutation_failure_tags: bundle
            .factor_mutation_evaluation
            .as_ref()
            .map(|evaluation| evaluation.failure_tags.clone())
            .unwrap_or_default(),
        factor_mutation_priority_markets: bundle.factor_mutation_priority_markets.clone(),
        factor_mutation_priority_reasons: bundle.factor_mutation_priority_reasons.clone(),
        factor_mutation_direction_hints: bundle.factor_mutation_direction_hints.clone(),
        factor_mutation_step_size_hints: bundle.factor_mutation_step_size_hints.clone(),
        multi_timeframe_summary: bundle.multi_timeframe_summary.clone(),
        artifact_consumed_gate_status: bundle.artifact_consumed_gate_status.clone(),
        pda_cluster_label: bundle.pda_cluster_label.clone(),
        top_factor_actions: bundle.top_factor_actions.clone(),
        stage_views: bundle
            .stage_views
            .iter()
            .map(|view| StageAgentContextMinimal {
                stage: view.stage.clone(),
                recommended_command: view.recommended_command.clone(),
                gate_status: view.gate_status.clone(),
            })
            .collect(),
    }
}

pub fn build_stage_views(
    symbol: &str,
    state_dir: &str,
    recommended_commands: &CommandRecommendations,
    factor_iteration_queue: &[FactorIterationPrompt],
    family_outcomes: &[FactorFamilyOutcome],
    pre_bayes_evidence_filter: Option<&PreBayesEvidenceFilter>,
    artifact_decision_summary: Option<&crate::state::ArtifactDecisionSummary>,
) -> Vec<StageAgentContext> {
    let pre_bayes_gate_status = pre_bayes_evidence_filter
        .map(|filter| filter.gating_status.clone())
        .unwrap_or_default();
    let pre_bayes_gate_reason = pre_bayes_evidence_filter
        .map(|filter| filter.rationale.join(";"))
        .unwrap_or_default();
    let pda_sequence_cluster_weak = pre_bayes_evidence_filter
        .map(|filter| {
            filter
                .conflict_flags
                .iter()
                .any(|flag| flag == "pda_sequence_cluster_weak")
        })
        .unwrap_or(false);
    let artifact_gate_status = artifact_decision_summary
        .map(|summary| summary.consumed_trend_status.clone())
        .unwrap_or_default();
    let artifact_gate_reason = artifact_decision_summary
        .map(|summary| summary.consumed_trend_reason.clone())
        .unwrap_or_default();
    let mut views = vec![
        StageAgentContext {
            stage: "analyze".to_string(),
            blocking_items: 0,
            recommended_command: render_recommended_command(&recommended_commands.analyze),
            actions: if recommended_commands.analyze.ready {
                vec!["observe current market state".to_string()]
            } else {
                vec![format!(
                    "blocked_by:{}",
                    recommended_commands.analyze.missing_inputs.join(",")
                )]
            },
            gate_status: pre_bayes_gate_status.clone(),
            gate_reason: pre_bayes_gate_reason.clone(),
        },
        StageAgentContext {
            stage: "research".to_string(),
            blocking_items: family_outcomes
                .iter()
                .filter(|family| !family.promotion_decision.approved)
                .count(),
            recommended_command: render_recommended_command(&recommended_commands.research),
            actions: if recommended_commands.research.ready {
                family_outcomes
                    .iter()
                    .filter(|family| !family.promotion_decision.approved)
                    .map(|family| {
                        format!(
                            "family:{} promotion={}",
                            family.family, family.promotion_decision.status
                        )
                    })
                    .collect()
            } else {
                vec![format!(
                    "blocked_by:{}",
                    recommended_commands.research.missing_inputs.join(",")
                )]
            },
            gate_status: artifact_gate_status.clone(),
            gate_reason: artifact_gate_reason.clone(),
        },
        StageAgentContext {
            stage: "backtest".to_string(),
            blocking_items: factor_iteration_queue
                .iter()
                .filter(|item| item.iteration_action == "replace")
                .count(),
            recommended_command: render_recommended_command(&recommended_commands.backtest),
            actions: if recommended_commands.backtest.ready {
                factor_iteration_queue
                    .iter()
                    .take(3)
                    .map(|item| format!("{}:{}", item.factor_name, item.iteration_action))
                    .collect()
            } else {
                vec![format!(
                    "blocked_by:{}",
                    recommended_commands.backtest.missing_inputs.join(",")
                )]
            },
            gate_status: artifact_gate_status.clone(),
            gate_reason: artifact_gate_reason.clone(),
        },
        StageAgentContext {
            stage: "update".to_string(),
            blocking_items: family_outcomes
                .iter()
                .filter(|family| family.rollback_recommendation.should_rollback)
                .count(),
            recommended_command: render_recommended_command(&recommended_commands.update),
            actions: if recommended_commands.update.ready {
                family_outcomes
                    .iter()
                    .filter(|family| family.rollback_recommendation.should_rollback)
                    .map(|family| format!("family:{} rollback", family.family))
                    .collect()
            } else {
                vec![format!(
                    "blocked_by:{}",
                    recommended_commands.update.missing_inputs.join(",")
                )]
            },
            gate_status: artifact_gate_status.clone(),
            gate_reason: artifact_gate_reason.clone(),
        },
    ];
    if pda_sequence_cluster_weak {
        views.push(StageAgentContext {
            stage: "pda_sequence_review".to_string(),
            blocking_items: 1,
            recommended_command: render_recommended_command(&recommended_commands.analyze),
            actions: vec![pre_bayes_evidence_filter
                .map(pda_sequence_review_rationale)
                .unwrap_or_else(|| {
                    "PDA sequence reinforcement requires manual review before it can influence gating"
                        .to_string()
                })],
            gate_status: pre_bayes_gate_status.clone(),
            gate_reason: pre_bayes_gate_reason.clone(),
        });
    }
    if !artifact_gate_status.is_empty() {
        views.push(StageAgentContext {
            stage: "artifact_consumption".to_string(),
            blocking_items: usize::from(artifact_gate_status == "validated_regressing"),
            recommended_command: format!(
                "ict-engine workflow-status --symbol {} --state-dir {} --phase artifact-consumed-gate",
                shell_quote(symbol),
                shell_quote(state_dir)
            ),
            actions: if artifact_gate_reason.is_empty() {
                Vec::new()
            } else {
                vec![artifact_gate_reason.clone()]
            },
            gate_status: artifact_gate_status,
            gate_reason: artifact_gate_reason,
        });
    }
    views
}

pub fn pre_bayes_soft_evidence_diff(
    filter: &PreBayesEvidenceFilter,
) -> Vec<crate::state::PreBayesSoftEvidenceNodeDiff> {
    [
        (
            "market_regime",
            &filter.filtered_market_regime_label,
            &filter.soft_market_regime_distribution,
        ),
        (
            "liquidity_context",
            &filter.filtered_liquidity_context_label,
            &filter.soft_liquidity_context_distribution,
        ),
        (
            "factor_alignment",
            &filter.filtered_factor_alignment,
            &filter.soft_factor_alignment_distribution,
        ),
        (
            "factor_uncertainty",
            &filter.filtered_factor_uncertainty,
            &filter.soft_factor_uncertainty_distribution,
        ),
        (
            "multi_timeframe_resonance",
            &filter.filtered_multi_timeframe_resonance_label,
            &filter.soft_multi_timeframe_resonance_distribution,
        ),
    ]
    .into_iter()
    .map(|(node, filtered_state, distribution)| {
        let dominant = distribution
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal));
        let entropy = distribution
            .values()
            .copied()
            .filter(|value| *value > f64::EPSILON)
            .map(|value| -value * value.ln())
            .sum::<f64>();
        crate::state::PreBayesSoftEvidenceNodeDiff {
            node: node.to_string(),
            filtered_state: filtered_state.to_string(),
            dominant_soft_state: dominant.map(|(state, _)| state.clone()),
            dominant_soft_probability: dominant.map(|(_, value)| *value).unwrap_or(0.0),
            entropy,
            diverges_from_filtered_state: dominant
                .map(|(state, _)| state != filtered_state)
                .unwrap_or(false),
        }
    })
    .collect()
}

pub fn pre_bayes_entry_quality_bridge_diff(
    bridge: &crate::state::PreBayesEntryQualityBridge,
) -> crate::state::PreBayesEntryQualityBridgeDiff {
    let (dominant_long_entry_quality, dominant_long_entry_quality_probability) =
        max_probability_label(&bridge.long_entry_quality);
    let (dominant_short_entry_quality, dominant_short_entry_quality_probability) =
        max_probability_label(&bridge.short_entry_quality);
    let (selected_entry_quality, selected_entry_quality_probability) =
        max_probability_label(&bridge.selected_entry_quality);
    crate::state::PreBayesEntryQualityBridgeDiff {
        dominant_long_entry_quality,
        dominant_long_entry_quality_probability,
        dominant_short_entry_quality,
        dominant_short_entry_quality_probability,
        selected_entry_quality,
        selected_entry_quality_probability,
        long_short_signal_probability_gap: (bridge.long_signal_probability
            - bridge.short_signal_probability)
            .abs(),
        multi_timeframe_direction_bias: bridge.multi_timeframe_direction_bias.clone(),
        multi_timeframe_alignment_score: bridge.multi_timeframe_alignment_score,
        multi_timeframe_entry_alignment_score: bridge.multi_timeframe_entry_alignment_score,
        rationale_summary: bridge.rationale.iter().take(5).cloned().collect(),
    }
}

fn factor_mutation_priority_markets(
    evaluation: &crate::state::FactorMutationEvaluation,
) -> Vec<String> {
    let mut items = evaluation.metrics_after.regressed_markets.clone();
    if items.is_empty() {
        items.extend(
            evaluation
                .metrics_after
                .regression_reasons_by_market
                .keys()
                .cloned(),
        );
    }
    items.truncate(3);
    items
}

fn factor_mutation_priority_reasons(
    evaluation: &crate::state::FactorMutationEvaluation,
) -> Vec<String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for reasons in evaluation
        .metrics_after
        .regression_reasons_by_market
        .values()
    {
        for reason in reasons {
            *counts.entry(reason.clone()).or_default() += 1;
        }
    }
    let mut ordered = counts.into_iter().collect::<Vec<_>>();
    ordered.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut items = ordered
        .into_iter()
        .map(|(reason, _)| reason)
        .collect::<Vec<_>>();
    if items.is_empty() {
        items = evaluation.failure_tags.clone();
    }
    items.truncate(3);
    items
}

fn max_probability_label(distribution: &BTreeMap<String, f64>) -> (Option<String>, f64) {
    distribution
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(label, value)| (Some(label.clone()), *value))
        .unwrap_or((None, 0.0))
}

fn filter_has(filter: &PreBayesEvidenceFilter, flag: &str) -> bool {
    filter.conflict_flags.iter().any(|item| item == flag)
}

fn pda_sequence_review_rationale(filter: &PreBayesEvidenceFilter) -> String {
    if filter_has(filter, "pda_sequence_sparse_sessions") {
        "PDA sequence reinforcement is unreliable because too few valid sessions were emitted"
            .to_string()
    } else if filter_has(filter, "pda_sequence_low_consistency") {
        "PDA sequence reinforcement is unreliable because DTW/HMM agreement is too low".to_string()
    } else if filter_has(filter, "pda_sequence_low_confidence") {
        "PDA sequence reinforcement is unreliable because the winning cluster confidence is too low"
            .to_string()
    } else {
        "PDA sequence reinforcement requires manual review before it can influence gating"
            .to_string()
    }
}
