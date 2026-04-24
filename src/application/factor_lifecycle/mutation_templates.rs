use std::collections::BTreeMap;

use crate::agent::{AgentPrompt, AgentPromptInput};
use crate::factors::FactorRegistry;
use crate::state::{FactorMutationEvaluation, FactorMutationSpec};

use super::{
    factor_mutation_priority_markets, factor_mutation_priority_reasons,
    factor_mutation_recommended_focus,
};

fn should_force_mutation_cluster_jump(evaluation: &FactorMutationEvaluation) -> bool {
    evaluation
        .failure_tags
        .iter()
        .any(|tag| tag == "best_factor_composite_regressed")
        && evaluation
            .failure_tags
            .iter()
            .any(|tag| tag == "no_superior_mutation_found")
}

pub fn forced_cluster_jump_template(
    current_spec: Option<&FactorMutationSpec>,
    evaluation: &FactorMutationEvaluation,
    evaluate_expansion_preview: bool,
) -> Option<FactorMutationSpec> {
    if !should_force_mutation_cluster_jump(evaluation) {
        return None;
    }
    let original_base_factor = current_spec
        .map(|spec| spec.base_factor.clone())
        .filter(|value| !value.is_empty())
        .or_else(|| evaluation.metrics_after.top_factor_names.first().cloned())
        .unwrap_or_else(|| "structure_ict".to_string());
    let market_bias = evaluation
        .metrics_after
        .top_factor_names
        .iter()
        .find(|name| name.as_str() == "structure_ict")
        .map(|_| "NQ_market_specific_fork_validation")
        .unwrap_or("label_refinement_or_market_specific_fork_validation");
    let cycle = current_spec
        .and_then(|spec| spec.direction_hints.get("cluster_jump_cycle"))
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let jump_target = if original_base_factor == "structure_ict" {
        match cycle % 4 {
            0 => "displacement_fvg_cluster",
            1 => "mss_bos_cluster",
            2 => "premium_discount_ote_cluster",
            _ => "smt_cluster",
        }
    } else {
        "market_specific_or_label_refinement_cluster"
    };
    let next_cycle_value = cycle + 1;
    let output_base_factor = if jump_target == "smt_cluster" {
        "cross_market_smt".to_string()
    } else {
        original_base_factor.clone()
    };
    let mut parameter_overrides = current_spec
        .map(|spec| spec.parameter_overrides.clone())
        .unwrap_or_default();
    match jump_target {
        "displacement_fvg_cluster" => {
            parameter_overrides.insert("post_sweep_displacement_weight".to_string(), 1.35);
            parameter_overrides.insert("sweep_weight".to_string(), 1.10);
            parameter_overrides.insert("unconfirmed_sweep_weight".to_string(), 0.45);
            parameter_overrides.insert("expansion_threshold".to_string(), 1.05);
        }
        "mss_bos_cluster" => {
            parameter_overrides.insert("lookback".to_string(), 10.0);
            parameter_overrides.insert("expansion_threshold".to_string(), 1.18);
            parameter_overrides.insert("sweep_return_bars".to_string(), 5.0);
            parameter_overrides.insert("opposing_sweep_penalty".to_string(), 1.25);
        }
        "premium_discount_ote_cluster" => {
            parameter_overrides.insert("lookback".to_string(), 14.0);
            parameter_overrides.insert("expansion_threshold".to_string(), 0.92);
            parameter_overrides.insert("sweep_recency_bars".to_string(), 8.0);
            parameter_overrides.insert("sweep_return_bars".to_string(), 6.0);
        }
        "smt_cluster" => {
            parameter_overrides.insert("lookback".to_string(), 24.0);
            parameter_overrides.insert("sweep_atr_multiplier".to_string(), 0.60);
            parameter_overrides.insert("sweep_weight".to_string(), 0.72);
            parameter_overrides.insert("opposing_sweep_penalty".to_string(), 1.05);
        }
        _ => {}
    }
    Some(FactorMutationSpec {
        mutation_id: format!("{}:jump", evaluation.mutation_id),
        base_factor: output_base_factor,
        hypothesis: format!(
            "Forced cluster jump after repeated near-local-optimum regression: stop narrow same-family tuning and pivot to {} with {}",
            jump_target, market_bias
        ),
        parameter_overrides,
        direction_hints: BTreeMap::from([
            ("cluster_jump".to_string(), jump_target.to_string()),
            (
                "cluster_jump_cycle".to_string(),
                next_cycle_value.to_string(),
            ),
            (
                "available_clusters".to_string(),
                "displacement_fvg_cluster|mss_bos_cluster|premium_discount_ote_cluster|smt_cluster"
                    .to_string(),
            ),
            (
                "next_cycle".to_string(),
                "label_refinement_or_market_specific_fork".to_string(),
            ),
            (
                "market_specific_fork".to_string(),
                if market_bias == "NQ_market_specific_fork_validation" {
                    "NQ".to_string()
                } else {
                    "generic".to_string()
                },
            ),
        ]),
        step_size_hints: BTreeMap::new(),
        enabled_overrides: current_spec
            .map(|spec| spec.enabled_overrides.clone())
            .unwrap_or_default(),
        evaluate_expansion_preview,
    })
}

pub fn factor_mutation_direction_hint_summary(
    evaluation: &FactorMutationEvaluation,
) -> Vec<String> {
    let template = next_mutation_spec_template(None, evaluation, false);
    if template.direction_hints.is_empty() {
        return Vec::new();
    }
    template
        .direction_hints
        .into_iter()
        .map(|(parameter, hint)| format!("{}:{}", parameter, hint))
        .collect()
}

pub fn factor_mutation_step_size_hint_summary(
    evaluation: &FactorMutationEvaluation,
) -> Vec<String> {
    let template = next_mutation_spec_template(None, evaluation, false);
    if template.step_size_hints.is_empty() {
        return Vec::new();
    }
    template
        .step_size_hints
        .into_iter()
        .map(|(parameter, step)| format!("{}:{:.4}", parameter, step))
        .collect()
}

pub fn next_mutation_spec_template(
    current_spec: Option<&FactorMutationSpec>,
    evaluation: &FactorMutationEvaluation,
    evaluate_expansion_preview: bool,
) -> FactorMutationSpec {
    if let Some(forced) =
        forced_cluster_jump_template(current_spec, evaluation, evaluate_expansion_preview)
    {
        return forced;
    }
    next_mutation_spec_template_with_preferences(
        current_spec,
        evaluation,
        evaluate_expansion_preview,
        None,
        None,
    )
}

pub fn next_mutation_spec_template_with_preferences(
    current_spec: Option<&FactorMutationSpec>,
    evaluation: &FactorMutationEvaluation,
    evaluate_expansion_preview: bool,
    preferred_direction_hints: Option<&BTreeMap<String, String>>,
    preferred_step_size_hints: Option<&BTreeMap<String, f64>>,
) -> FactorMutationSpec {
    if let Some(forced) =
        forced_cluster_jump_template(current_spec, evaluation, evaluate_expansion_preview)
    {
        return forced;
    }
    let priority_reasons = factor_mutation_priority_reasons(evaluation);
    let base_factor = current_spec
        .map(|spec| spec.base_factor.clone())
        .filter(|value| !value.is_empty())
        .or_else(|| evaluation.metrics_after.top_factor_names.first().cloned())
        .unwrap_or_default();
    let base_parameter_overrides = current_spec
        .map(|spec| spec.parameter_overrides.clone())
        .unwrap_or_default();
    let base_enabled_overrides = current_spec
        .map(|spec| spec.enabled_overrides.clone())
        .unwrap_or_default();
    let mut direction_hints = reason_aware_direction_hints(&base_factor, &priority_reasons);
    let mut step_size_hints = reason_aware_step_size_hints(&base_factor, &priority_reasons);
    if let Some(preferred_direction_hints) = preferred_direction_hints {
        for (parameter, hint) in preferred_direction_hints {
            if direction_hints.contains_key(parameter) {
                direction_hints.insert(parameter.clone(), hint.clone());
            }
        }
    }
    if let Some(preferred_step_size_hints) = preferred_step_size_hints {
        for (parameter, step) in preferred_step_size_hints {
            if step_size_hints.contains_key(parameter) {
                step_size_hints.insert(parameter.clone(), *step);
            }
        }
    }
    let reason_aware_parameter_overrides = reason_aware_parameter_overrides(
        &base_factor,
        &priority_reasons,
        &base_parameter_overrides,
        &direction_hints,
        &step_size_hints,
    );
    let reason_aware_enabled_overrides =
        reason_aware_enabled_overrides(&priority_reasons, &base_enabled_overrides);
    FactorMutationSpec {
        mutation_id: format!("{}:next", evaluation.mutation_id),
        base_factor,
        hypothesis: if priority_reasons.is_empty() {
            "Run one atomic mutation that improves PreBayes/bridge quality without widening soft-evidence conflicts"
                .to_string()
        } else {
            format!(
                "Run one atomic mutation targeting: {} with direction_hints={}",
                priority_reasons.join(","),
                format_direction_hints(&direction_hints)
            )
        },
        parameter_overrides: reason_aware_parameter_overrides,
        direction_hints,
        step_size_hints,
        enabled_overrides: reason_aware_enabled_overrides,
        evaluate_expansion_preview,
    }
}

fn reason_aware_parameter_overrides(
    base_factor: &str,
    priority_reasons: &[String],
    parameter_overrides: &BTreeMap<String, f64>,
    direction_hints: &BTreeMap<String, String>,
    step_size_hints: &BTreeMap<String, f64>,
) -> BTreeMap<String, f64> {
    let registry = FactorRegistry::default();
    let factor_definition = registry.get(base_factor);
    let mut selected = BTreeMap::new();
    for reason in priority_reasons {
        let grouped_keys = factor_definition
            .map(|definition| definition.mutation_parameter_group(reason))
            .unwrap_or_default();
        let keywords: &[&str] = match reason.as_str() {
            "balanced_accuracy_regressed"
            | "bull_bear_separation_regressed"
            | "bull_bear_separation_weak"
            | "worst_market_separation_weak" => {
                &["window", "lookback", "threshold", "fast", "slow", "period"]
            }
            "bridge_gap_regressed"
            | "bridge_gap_too_small"
            | "worst_market_bridge_gap_too_small" => {
                &["threshold", "sensitivity", "weight", "bias", "level"]
            }
            "pre_bayes_gate_regressed"
            | "pre_bayes_gate_observe_only"
            | "pre_bayes_gate_neutralized" => {
                &["threshold", "uncertainty", "confidence", "bias", "weight"]
            }
            _ => &[],
        };
        for (key, value) in parameter_overrides {
            if grouped_keys.iter().any(|grouped| grouped == key)
                || keywords.iter().any(|keyword| key.contains(keyword))
            {
                selected.insert(key.clone(), *value);
            }
        }
        if parameter_overrides.is_empty() {
            for key in grouped_keys {
                if let Some(default_value) = factor_definition
                    .and_then(|definition| definition.parameters.get(&key).copied())
                {
                    let adjusted = apply_direction_hint(
                        default_value,
                        direction_hints.get(&key).map(String::as_str),
                        step_size_hints.get(&key).copied(),
                    );
                    selected.insert(key, adjusted);
                }
            }
        }
    }
    if selected.is_empty() {
        if parameter_overrides.is_empty() {
            return BTreeMap::new();
        }
        parameter_overrides
            .iter()
            .take(2)
            .map(|(key, value)| (key.clone(), *value))
            .collect()
    } else {
        selected
    }
}

fn reason_aware_direction_hints(
    base_factor: &str,
    priority_reasons: &[String],
) -> BTreeMap<String, String> {
    let registry = FactorRegistry::default();
    let Some(definition) = registry.get(base_factor) else {
        return BTreeMap::new();
    };
    let mut hints = BTreeMap::new();
    for reason in priority_reasons {
        for (parameter, hint) in definition.mutation_direction_hint(reason) {
            hints.entry(parameter).or_insert(hint);
        }
    }
    hints
}

fn reason_aware_step_size_hints(
    base_factor: &str,
    priority_reasons: &[String],
) -> BTreeMap<String, f64> {
    let registry = FactorRegistry::default();
    let Some(definition) = registry.get(base_factor) else {
        return BTreeMap::new();
    };
    let mut hints = BTreeMap::new();
    for reason in priority_reasons {
        for (parameter, step) in definition.mutation_step_size_hint(reason) {
            hints.entry(parameter).or_insert(step);
        }
    }
    hints
}

fn apply_direction_hint(value: f64, hint: Option<&str>, step_size: Option<f64>) -> f64 {
    let step = step_size.unwrap_or(0.10);
    match hint.unwrap_or("") {
        "increase" | "widen" => value * (1.0 + step),
        "decrease" | "tighten" => value * (1.0 - step),
        _ => value,
    }
}

fn format_direction_hints(hints: &BTreeMap<String, String>) -> String {
    if hints.is_empty() {
        "none".to_string()
    } else {
        hints
            .iter()
            .map(|(parameter, hint)| format!("{}:{}", parameter, hint))
            .collect::<Vec<_>>()
            .join("|")
    }
}

fn reason_aware_enabled_overrides(
    priority_reasons: &[String],
    enabled_overrides: &BTreeMap<String, bool>,
) -> BTreeMap<String, bool> {
    if enabled_overrides.is_empty() {
        return BTreeMap::new();
    }
    let should_preserve = priority_reasons.iter().any(|reason| {
        matches!(
            reason.as_str(),
            "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
        )
    });
    if should_preserve {
        enabled_overrides
            .iter()
            .take(1)
            .map(|(key, value)| (key.clone(), *value))
            .collect()
    } else {
        BTreeMap::new()
    }
}

pub fn factor_mutation_focus_prompt(
    current_spec: Option<&FactorMutationSpec>,
    evaluation: &FactorMutationEvaluation,
    evaluate_expansion_preview: bool,
) -> AgentPrompt {
    let priority_markets = factor_mutation_priority_markets(evaluation);
    let priority_reasons = factor_mutation_priority_reasons(evaluation);
    let recommended_focus = factor_mutation_recommended_focus(evaluation);
    let next_spec_template =
        next_mutation_spec_template(current_spec, evaluation, evaluate_expansion_preview);
    AgentPrompt::new(AgentPromptInput {
        id: "factor-mutation-focus".to_string(),
        stage: "iteration".to_string(),
        priority: "high".to_string(),
        objective: "Prepare the next atomic factor mutation using the current priority markets, regression reasons, and recommended focus.".to_string(),
        system_prompt: "Choose exactly one small mutation that targets the highest-priority regression pattern. Do not broaden scope across multiple factors or bypass the PreBayes gate.".to_string(),
        user_prompt: format!(
            "mutation_id={} accepted={} priority_markets={} priority_reasons={} recommended_focus={} direction_hints={} step_size_hints={} next_mutation_spec_template={}",
            evaluation.mutation_id,
            evaluation.accepted,
            if priority_markets.is_empty() {
                "none".to_string()
            } else {
                priority_markets.join(",")
            },
            if priority_reasons.is_empty() {
                "none".to_string()
            } else {
                priority_reasons.join(",")
            },
            if recommended_focus.is_empty() {
                "none".to_string()
            } else {
                recommended_focus.join(" | ")
            },
            if next_spec_template.direction_hints.is_empty() {
                "none".to_string()
            } else {
                next_spec_template
                    .direction_hints
                    .iter()
                    .map(|(parameter, hint)| format!("{}:{}", parameter, hint))
                    .collect::<Vec<_>>()
                    .join("|")
            },
            if next_spec_template.step_size_hints.is_empty() {
                "none".to_string()
            } else {
                next_spec_template
                    .step_size_hints
                    .iter()
                    .map(|(parameter, step)| format!("{}:{:.4}", parameter, step))
                    .collect::<Vec<_>>()
                    .join("|")
            },
            serde_json::to_string(&next_spec_template).unwrap_or_else(|_| "{}".to_string())
        ),
        success_criteria: vec![
            "Pick one mutation that addresses the top regression reason in the top priority market".to_string(),
            "Prefer parameter tuning over enabling extra factors unless the current reason clearly points to missing directional separation".to_string(),
            "Do not accept any next mutation that regresses PreBayes gate quality or widens soft evidence conflicts".to_string(),
        ],
        suggested_files: vec!["src/main.rs".to_string(), "src/factors/registry.rs".to_string()],
    })
}
