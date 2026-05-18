use std::collections::BTreeMap;

use crate::application::decision_utils::ResearchObjectiveMode;

pub fn recommended_mutation_directions_from_failure_tags(
    failure_tags: &[String],
    regressed_markets: &[String],
    regression_reasons_by_market: &BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    let mut directions = Vec::new();
    if failure_tags
        .iter()
        .any(|tag| tag == "bull_bear_separation_weak")
        || failure_tags
            .iter()
            .any(|tag| tag == "bull_bear_separation_regressed")
    {
        directions.push(
            "Prioritize base factor parameter tuning that improves bull/bear expansion separation before enabling additional factors"
                .to_string(),
        );
    }
    if failure_tags.iter().any(|tag| tag == "bridge_gap_too_small")
        || failure_tags.iter().any(|tag| tag == "bridge_gap_regressed")
    {
        directions.push(
            "Tighten directionality-sensitive parameters to widen PreBayes bridge probability gap instead of broad family enablement"
                .to_string(),
        );
    }
    if failure_tags
        .iter()
        .any(|tag| tag == "soft_evidence_divergence_elevated")
    {
        directions.push(
            "Reduce mutations that create label conflict; prefer edits that keep soft evidence aligned with filtered assignments"
                .to_string(),
        );
    }
    if failure_tags
        .iter()
        .any(|tag| tag == "pre_bayes_gate_observe_only" || tag == "pre_bayes_gate_neutralized")
    {
        directions.push(
            "Avoid mutations that neutralize the PreBayes gate; prefer stronger alignment/uncertainty separation on the selected factor"
                .to_string(),
        );
    }
    if failure_tags
        .iter()
        .any(|tag| tag == "pre_bayes_gate_regressed")
    {
        directions.push(
            "Revert the gate-regressing parameter move and prefer slower, confirmation-heavy edits that preserve PreBayes pass_neutralized or better"
                .to_string(),
        );
    }
    if failure_tags
        .iter()
        .any(|tag| tag == "best_factor_composite_regressed")
    {
        directions.push(
            "Keep the base factor first in the objective ranking before chasing bridge improvements; do not sacrifice composite separation quality for a single latest-sample boost"
                .to_string(),
        );
    }
    if failure_tags
        .iter()
        .any(|tag| tag == "market_specific_regressions_detected")
    {
        directions.push(
            "Stop global blind tuning and pivot to market-specific label refinement or per-market factor forks for the regressed families"
                .to_string(),
        );
    }
    if failure_tags
        .iter()
        .any(|tag| tag == "no_superior_mutation_found")
    {
        directions.push(
            "Treat the current default as near-local-optimum until new evidence appears; shift the next cycle to label refinement or market-specific fork validation"
                .to_string(),
        );
    }
    if !regressed_markets.is_empty() {
        directions.push(format!(
            "Inspect regressed markets first: {}",
            regressed_markets.join(",")
        ));
    }
    let markets_with_bridge_regressions = regression_reasons_by_market
        .iter()
        .filter(|(_, reasons)| {
            reasons
                .iter()
                .any(|reason| reason == "bridge_gap_regressed")
        })
        .map(|(market, _)| market.clone())
        .collect::<Vec<_>>();
    if !markets_with_bridge_regressions.is_empty() {
        directions.push(format!(
            "Target bridge-sensitive parameter edits for markets with bridge regressions: {}",
            markets_with_bridge_regressions.join(",")
        ));
    }
    let markets_with_gate_regressions = regression_reasons_by_market
        .iter()
        .filter(|(_, reasons)| {
            reasons
                .iter()
                .any(|reason| reason == "pre_bayes_gate_regressed")
        })
        .map(|(market, _)| market.clone())
        .collect::<Vec<_>>();
    if !markets_with_gate_regressions.is_empty() {
        directions.push(format!(
            "Prioritize mutations that restore PreBayes pass gating for markets: {}",
            markets_with_gate_regressions.join(",")
        ));
    }
    let markets_with_separation_regressions = regression_reasons_by_market
        .iter()
        .filter(|(_, reasons)| {
            reasons
                .iter()
                .any(|reason| reason == "balanced_accuracy_regressed")
        })
        .map(|(market, _)| market.clone())
        .collect::<Vec<_>>();
    if !markets_with_separation_regressions.is_empty() {
        directions.push(format!(
            "Re-tune expansion separation parameters for markets: {}",
            markets_with_separation_regressions.join(",")
        ));
    }
    directions.dedup();
    directions
}

pub fn no_superior_mutation_found(
    score_delta: f64,
    failure_tags: &[String],
    objective: ResearchObjectiveMode,
) -> bool {
    objective == ResearchObjectiveMode::ExpansionManipulation
        && score_delta <= 0.0
        && !failure_tags
            .iter()
            .any(|tag| tag == "pre_bayes_gate_regressed")
}
