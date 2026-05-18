use crate::state::{
    DatasetComparability, DecisionThresholds, FactorFamilyDecision, FactorFamilyOutcome,
    FactorIterationPrompt, PersistedFactorRanking, PreBayesEvidenceFilter, ProbabilityDiff,
    PromotionDecision, RankingDiffItem, RollbackRecommendation,
};
use anyhow::{bail, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResearchObjectiveMode {
    Generic,
    ExpansionManipulation,
}

#[derive(Debug, Clone, Default)]
pub struct ArtifactConsumedDecisionGate {
    pub status: String,
    pub reason: String,
    pub target_kinds: Vec<String>,
}

pub fn normalize_entry_quality_label(entry_signal: &str) -> String {
    let normalized = entry_signal.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "high" | "strong_buy" | "strong_sell" | "a+" => "high".to_string(),
        "low" | "weak" | "invalid" | "no_trade" => "low".to_string(),
        "medium" | "buy" | "sell" | "valid" => "medium".to_string(),
        _ if normalized.contains("strong") || normalized.contains("high") => "high".to_string(),
        _ if normalized.contains("weak") || normalized.contains("low") => "low".to_string(),
        _ => "medium".to_string(),
    }
}

pub fn normalize_trade_outcome_label(outcome: &str) -> String {
    let normalized = outcome.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "win" | "profit" | "tp" | "take_profit" => "win".to_string(),
        "loss" | "lose" | "sl" | "stop" | "stop_loss" => "loss".to_string(),
        _ => "breakeven".to_string(),
    }
}

pub fn normalize_distribution(values: &mut [f64]) {
    let sum: f64 = values.iter().sum();
    if sum <= f64::EPSILON {
        let uniform = 1.0 / values.len() as f64;
        values.fill(uniform);
        return;
    }
    for value in values {
        *value /= sum;
    }
}

pub fn pre_bayes_gate_rank(status: &str) -> u8 {
    match status {
        "pass_hard" => 2,
        "pass_neutralized" => 1,
        "observe_only" => 0,
        _ => 0,
    }
}

pub fn pre_bayes_gate_is_hard_pass(status: &str) -> bool {
    status == "pass_hard"
}

pub fn pre_bayes_gate_regressed(previous: &str, current: &str) -> bool {
    pre_bayes_gate_rank(current) < pre_bayes_gate_rank(previous)
}

pub fn build_analyze_decision_hint(
    dataset_comparability: &DatasetComparability,
    factor_iteration_queue: &[FactorIterationPrompt],
    factor_diagnostics: &crate::factor_lab::FactorDiagnostics,
) -> String {
    if !dataset_comparability.comparable {
        return format!(
            "Observe only: current run not comparable to last analyze ({}).",
            dataset_comparability.reason
        );
    }
    if factor_diagnostics.uncertainty >= 0.45 {
        return "Wait: evidence uncertainty remains high; defer action until structure clears."
            .to_string();
    }
    if let Some(next) = factor_iteration_queue.first() {
        return format!(
            "Comparable run, but factor backlog remains: {} {} first.",
            next.iteration_action, next.factor_name
        );
    }
    "Comparable run and factor stack stable; no immediate factor action required.".to_string()
}

pub fn append_pda_sequence_hint(
    base_hint: &str,
    pda_sequence_summary: Option<&crate::pda_sequence::PdaSequenceArtifactSummary>,
    pre_bayes_evidence_filter: &PreBayesEvidenceFilter,
) -> String {
    let suffix = match pda_sequence_summary {
        Some(summary)
            if pre_bayes_conflict(filter_has(
                pre_bayes_evidence_filter,
                "pda_sequence_sparse_sessions",
            )) =>
        {
            format!(
                "|pda_sequence=sparse_sessions:{}:{:.3}:{:.3}",
                summary
                    .primary_cluster_label
                    .as_deref()
                    .unwrap_or("unknown"),
                summary.primary_cluster_confidence.unwrap_or_default(),
                summary.consistency_ratio,
            )
        }
        Some(summary)
            if pre_bayes_conflict(filter_has(
                pre_bayes_evidence_filter,
                "pda_sequence_low_consistency",
            )) =>
        {
            format!(
                "|pda_sequence=low_consistency:{}:{:.3}:{:.3}",
                summary
                    .primary_cluster_label
                    .as_deref()
                    .unwrap_or("unknown"),
                summary.primary_cluster_confidence.unwrap_or_default(),
                summary.consistency_ratio,
            )
        }
        Some(summary)
            if pre_bayes_conflict(filter_has(
                pre_bayes_evidence_filter,
                "pda_sequence_low_confidence",
            )) =>
        {
            format!(
                "|pda_sequence=low_confidence:{}:{:.3}:{:.3}",
                summary
                    .primary_cluster_label
                    .as_deref()
                    .unwrap_or("unknown"),
                summary.primary_cluster_confidence.unwrap_or_default(),
                summary.consistency_ratio,
            )
        }
        Some(summary)
            if pre_bayes_conflict(filter_has(
                pre_bayes_evidence_filter,
                "pda_regime_family_disagreement",
            )) =>
        {
            format!(
                "|pda_sequence=regime_disagreement:{}:{}:{:.3}",
                summary
                    .primary_cluster_label
                    .as_deref()
                    .unwrap_or("unknown"),
                summary
                    .primary_cluster_family
                    .as_deref()
                    .unwrap_or("unknown"),
                summary.primary_cluster_confidence.unwrap_or_default(),
            )
        }
        Some(summary)
            if pre_bayes_evidence_filter
                .conflict_flags
                .iter()
                .any(|flag| flag == "pda_sequence_cluster_weak") =>
        {
            format!(
                "|pda_sequence=weak_cluster:{}:{:.3}:{:.3}",
                summary
                    .primary_cluster_label
                    .as_deref()
                    .unwrap_or("unknown"),
                summary.primary_cluster_confidence.unwrap_or_default(),
                summary.consistency_ratio,
            )
        }
        Some(summary)
            if summary.primary_cluster_confidence.unwrap_or_default() >= 0.80
                && summary.consistency_ratio >= 0.70
                && summary.ensemble_mean_confidence >= 0.70 =>
        {
            format!(
                "|pda_sequence=reinforcing_cluster:{}:{:.3}:{:.3}",
                summary
                    .primary_cluster_label
                    .as_deref()
                    .unwrap_or("unknown"),
                summary.primary_cluster_confidence.unwrap_or_default(),
                summary.consistency_ratio,
            )
        }
        Some(summary) => format!(
            "|pda_sequence=informational_cluster:{}:{:.3}:{:.3}",
            summary
                .primary_cluster_label
                .as_deref()
                .unwrap_or("unknown"),
            summary.primary_cluster_confidence.unwrap_or_default(),
            summary.consistency_ratio,
        ),
        None => "|pda_sequence=unavailable".to_string(),
    };
    format!("{base_hint}{suffix}")
}

fn filter_has(filter: &PreBayesEvidenceFilter, flag: &str) -> bool {
    filter.conflict_flags.iter().any(|item| item == flag)
}

fn pre_bayes_conflict(value: bool) -> bool {
    value
}

pub fn derive_family_outcomes(
    family_decisions: &[FactorFamilyDecision],
    thresholds: &DecisionThresholds,
    comparability: &DatasetComparability,
    artifact_family_trends: Option<&[crate::state::ArtifactFamilyTrendSummary]>,
) -> Vec<FactorFamilyOutcome> {
    family_decisions
        .iter()
        .map(|family| {
            let replacement_candidates = family.replacement_candidates.clone();
            let artifact_family_trend = artifact_family_trends
                .and_then(|trends| trends.iter().find(|trend| trend.family == family.family));
            let artifact_regressing = artifact_family_trend
                .map(|trend| trend.consumed_validation_status == "validated_regressing")
                .unwrap_or(false);
            let artifact_improving = artifact_family_trend
                .map(|trend| trend.consumed_validation_status == "validated_improving")
                .unwrap_or(false);
            let artifact_reason = artifact_family_trend
                .map(|trend| trend.consumed_validation_reason.clone())
                .unwrap_or_default();
            let should_promote = comparability.comparable
                && family.avg_score >= thresholds.promotion_min_score
                && !artifact_regressing;
            let should_rollback = comparability.comparable
                && (artifact_regressing
                    || !replacement_candidates.is_empty()
                    || family.avg_score
                        <= thresholds.promotion_min_score + thresholds.rollback_score_delta.abs());
            FactorFamilyOutcome {
                family: family.family.clone(),
                promotion_decision: PromotionDecision {
                    approved: should_promote,
                    status: if should_promote {
                        "promote".to_string()
                    } else {
                        "hold".to_string()
                    },
                    reason: if artifact_regressing {
                        format!(
                            "family_artifact_consumption_validated_regression:{}",
                            artifact_reason
                        )
                    } else if should_promote && artifact_improving {
                        format!(
                            "family_score_above_promotion_threshold_with_artifact_validation:{}",
                            artifact_reason
                        )
                    } else if should_promote {
                        "family_score_above_promotion_threshold".to_string()
                    } else {
                        comparability.reason.clone()
                    },
                    target_factors: family
                        .actions
                        .iter()
                        .filter(|item| item.ends_with(":keep") || item.ends_with(":tune"))
                        .cloned()
                        .collect(),
                    target_families: vec![family.family.clone()],
                },
                rollback_recommendation: RollbackRecommendation {
                    should_rollback,
                    scope: if should_rollback {
                        if artifact_regressing && replacement_candidates.is_empty() {
                            "family_artifact".to_string()
                        } else {
                            "family".to_string()
                        }
                    } else {
                        "none".to_string()
                    },
                    reason: if should_rollback {
                        if artifact_regressing {
                            format!(
                                "family_artifact_consumption_validated_regression:{}",
                                artifact_reason
                            )
                        } else if !replacement_candidates.is_empty() {
                            "family_contains_replacement_candidates".to_string()
                        } else {
                            "family_score_below_safe_band".to_string()
                        }
                    } else {
                        "family_stable".to_string()
                    },
                    target_factors: replacement_candidates,
                    target_families: vec![family.family.clone()],
                },
            }
        })
        .collect()
}

fn credibility_regressing(rankings: &[PersistedFactorRanking]) -> Option<String> {
    rankings.iter().find_map(|ranking| {
        if ranking.conformal_coverage_1sigma < 0.55 {
            Some(format!(
                "conformal_coverage_low:{}:{:.3}",
                ranking.factor_name, ranking.conformal_coverage_1sigma
            ))
        } else if ranking.regime_break_penalty > 0.20 {
            Some(format!(
                "regime_break_penalty_high:{}:{:.3}",
                ranking.factor_name, ranking.regime_break_penalty
            ))
        } else {
            None
        }
    })
}

pub fn derive_promotion_decision(
    rankings: &[PersistedFactorRanking],
    score_deltas: &[RankingDiffItem],
    comparability: &DatasetComparability,
    thresholds: &DecisionThresholds,
    artifact_consumed_gate: Option<&ArtifactConsumedDecisionGate>,
) -> PromotionDecision {
    let improving = score_deltas
        .iter()
        .filter(|item| item.score_delta >= thresholds.promotion_min_score_delta)
        .map(|item| item.factor_name.clone())
        .collect::<Vec<_>>();
    let top_score = rankings
        .first()
        .map(|item| item.composite_score)
        .unwrap_or(0.0);
    let severe_regression = score_deltas
        .iter()
        .any(|item| item.score_delta <= thresholds.rollback_score_delta);
    let artifact_regressing = artifact_consumed_gate
        .map(|gate| gate.status == "validated_regressing")
        .unwrap_or(false);
    let artifact_improving = artifact_consumed_gate
        .map(|gate| gate.status == "validated_improving")
        .unwrap_or(false);
    let credibility_regression_reason = credibility_regressing(rankings);

    if !comparability.comparable {
        PromotionDecision {
            approved: false,
            status: "hold".to_string(),
            reason: comparability.reason.clone(),
            target_factors: improving,
            target_families: Vec::new(),
        }
    } else if artifact_regressing {
        PromotionDecision {
            approved: false,
            status: "hold".to_string(),
            reason: artifact_consumed_gate
                .map(|gate| {
                    format!(
                        "artifact_consumption_validated_regression:{} target_kinds={:?}",
                        gate.reason, gate.target_kinds
                    )
                })
                .unwrap_or_else(|| "artifact_consumption_validated_regression".to_string()),
            target_factors: improving,
            target_families: Vec::new(),
        }
    } else if let Some(reason) = credibility_regression_reason {
        PromotionDecision {
            approved: false,
            status: "hold".to_string(),
            reason,
            target_factors: improving,
            target_families: Vec::new(),
        }
    } else if !improving.is_empty()
        && !severe_regression
        && top_score >= thresholds.promotion_min_score
    {
        PromotionDecision {
            approved: true,
            status: "promote".to_string(),
            reason: if artifact_improving {
                artifact_consumed_gate
                    .map(|gate| {
                        format!(
                            "material_score_improvement_with_artifact_consumption_validation:{}",
                            gate.reason
                        )
                    })
                    .unwrap_or_else(|| {
                        "material_score_improvement_with_artifact_consumption_validation"
                            .to_string()
                    })
            } else {
                "material_score_improvement_without_major_regression".to_string()
            },
            target_factors: improving,
            target_families: Vec::new(),
        }
    } else {
        PromotionDecision {
            approved: false,
            status: "hold".to_string(),
            reason: if severe_regression {
                "score_regression_detected".to_string()
            } else {
                "insufficient_improvement".to_string()
            },
            target_factors: improving,
            target_families: Vec::new(),
        }
    }
}

pub fn derive_rollback_recommendation(
    rankings: &[PersistedFactorRanking],
    score_deltas: &[RankingDiffItem],
    probability_deltas: &[ProbabilityDiff],
    comparability: &DatasetComparability,
    thresholds: &DecisionThresholds,
    artifact_consumed_gate: Option<&ArtifactConsumedDecisionGate>,
) -> RollbackRecommendation {
    if !comparability.comparable {
        return RollbackRecommendation {
            should_rollback: false,
            scope: "none".to_string(),
            reason: comparability.reason.clone(),
            target_factors: Vec::new(),
            target_families: Vec::new(),
        };
    }

    let target_factors = score_deltas
        .iter()
        .filter(|item| item.score_delta <= thresholds.rollback_score_delta)
        .map(|item| item.factor_name.clone())
        .collect::<Vec<_>>();
    let harmful_prob_shift = probability_deltas.iter().any(|item| {
        (item.state.ends_with(":win") && item.delta <= -thresholds.rollback_probability_delta)
            || (item.state.ends_with(":loss")
                && item.delta >= thresholds.rollback_probability_delta)
    });
    let artifact_regressing = artifact_consumed_gate
        .map(|gate| gate.status == "validated_regressing")
        .unwrap_or(false);
    let credibility_regression_reason = credibility_regressing(rankings);

    if harmful_prob_shift
        || !target_factors.is_empty()
        || artifact_regressing
        || credibility_regression_reason.is_some()
    {
        RollbackRecommendation {
            should_rollback: true,
            scope: if artifact_regressing && target_factors.is_empty() {
                "artifact".to_string()
            } else if target_factors.len() <= 1 {
                "targeted".to_string()
            } else {
                "broad".to_string()
            },
            reason: if artifact_regressing {
                artifact_consumed_gate
                    .map(|gate| {
                        format!(
                            "artifact_consumption_validated_regression:{} target_kinds={:?}",
                            gate.reason, gate.target_kinds
                        )
                    })
                    .unwrap_or_else(|| "artifact_consumption_validated_regression".to_string())
            } else if let Some(reason) = credibility_regression_reason {
                reason
            } else if harmful_prob_shift {
                "outcome_calibration_regressed".to_string()
            } else {
                "factor_score_regression".to_string()
            },
            target_factors,
            target_families: Vec::new(),
        }
    } else {
        RollbackRecommendation {
            should_rollback: false,
            scope: "none".to_string(),
            reason: "no_material_regression".to_string(),
            target_factors,
            target_families: Vec::new(),
        }
    }
}

pub fn parse_research_objective(value: &str) -> Result<ResearchObjectiveMode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "generic" => Ok(ResearchObjectiveMode::Generic),
        "expansion_manipulation" | "expansion-manipulation" | "expansion" => {
            Ok(ResearchObjectiveMode::ExpansionManipulation)
        }
        other => bail!("unsupported research objective '{}'", other),
    }
}

pub fn research_objective_label(objective: ResearchObjectiveMode) -> &'static str {
    match objective {
        ResearchObjectiveMode::Generic => "generic",
        ResearchObjectiveMode::ExpansionManipulation => "expansion_manipulation",
    }
}

pub fn score_grade(score: f64) -> String {
    if score >= 0.85 {
        "A".to_string()
    } else if score >= 0.70 {
        "B".to_string()
    } else if score >= 0.55 {
        "C".to_string()
    } else if score >= 0.40 {
        "D".to_string()
    } else {
        "F".to_string()
    }
}
