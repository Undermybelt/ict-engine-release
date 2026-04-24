use std::collections::BTreeSet;

use crate::state::{PreBayesEntryQualityBridge, PreBayesEvidencePolicy, PreBayesPolicyRecord};

pub fn pre_bayes_policy_lineage_summary(
    history: &[PreBayesPolicyRecord],
    latest_gate_status: &str,
) -> crate::state::PreBayesPolicyLineageSummary {
    let latest = history.last();
    let previous = history.iter().rev().nth(1);
    let changed_fields_union = history
        .iter()
        .flat_map(|record| record.diff_from_previous.changed_fields.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let rollback_candidate_version =
        if matches!(latest_gate_status, "observe_only" | "pass_neutralized") {
            previous.map(|record| record.policy.version.clone())
        } else {
            None
        };
    let rollback_reason = if rollback_candidate_version.is_some() {
        format!(
            "latest_gate_status={} suggests reverting to previous stable policy version",
            latest_gate_status
        )
    } else {
        "no_policy_rollback_suggested".to_string()
    };
    crate::state::PreBayesPolicyLineageSummary {
        latest_version: latest.map(|record| record.policy.version.clone()),
        previous_version: previous.map(|record| record.policy.version.clone()),
        total_versions: history.len(),
        changed_fields_union,
        rollback_candidate_version,
        rollback_reason,
    }
}

pub fn pre_bayes_report_summary(
    policy: Option<&PreBayesEvidencePolicy>,
    bridge: Option<&PreBayesEntryQualityBridge>,
) -> Vec<String> {
    let mut summary = Vec::new();
    if let Some(policy) = policy {
        summary.push(format!(
            "policy_version={} source={} hard_pass={:.2} neutralized_pass={:.2}",
            policy.version,
            policy.source,
            policy.hard_pass_quality_threshold,
            policy.neutralized_quality_threshold
        ));
    }
    if let Some(bridge) = bridge {
        let bridge_diff = crate::application::backtest::pre_bayes_entry_quality_bridge_diff(bridge);
        summary.extend(bridge_diff.rationale_summary.clone());
        summary.push(format!(
            "selected_entry_quality={:?} selected_probability={:.3} long_short_gap={:.3} mtf_direction={} mtf_alignment={:.3} mtf_entry_alignment={:.3}",
            bridge_diff.selected_entry_quality,
            bridge_diff.selected_entry_quality_probability,
            bridge_diff.long_short_signal_probability_gap,
            bridge_diff.multi_timeframe_direction_bias,
            bridge_diff.multi_timeframe_alignment_score.unwrap_or_default(),
            bridge_diff
                .multi_timeframe_entry_alignment_score
                .unwrap_or_default()
        ));
    }
    summary
}

pub fn combine_regime_labels(labels: &[&str]) -> String {
    let bull = labels.iter().filter(|label| **label == "bull").count();
    let bear = labels.iter().filter(|label| **label == "bear").count();

    if bull > bear && bull >= 2 {
        "bull".to_string()
    } else if bear > bull && bear >= 2 {
        "bear".to_string()
    } else {
        "range".to_string()
    }
}

pub fn combine_liquidity_labels(labels: &[&str]) -> String {
    let hostile = labels.iter().filter(|label| **label == "hostile").count();
    let favorable = labels.iter().filter(|label| **label == "favorable").count();

    if hostile >= 2 {
        "hostile".to_string()
    } else if favorable == labels.len() {
        "favorable".to_string()
    } else {
        "neutral".to_string()
    }
}
