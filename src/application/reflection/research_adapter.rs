use crate::application::execution::{
    apply_execution_artifact_to_reflection_bundle, build_execution_artifact,
};
use crate::application::orchestration::build_stub_ensemble_vote_from_research;
use crate::factor_lab::research::ResearchReport;

use super::{build_reflection_bundle, ReflectionBundle, ReflectionBundleInput};

pub fn build_research_reflection_bundle(
    symbol: &str,
    report: &ResearchReport,
    compare_summary: Option<&str>,
) -> ReflectionBundle {
    let next_candidates = if report.recommended_next_command.is_empty() {
        vec![format!(
            "research={}",
            report.recommended_commands.research.command
        )]
    } else {
        vec![report.recommended_next_command.clone()]
    };

    let mut bundle = build_reflection_bundle(ReflectionBundleInput {
        symbol: symbol.to_string(),
        timestamp: report.provenance.data_fingerprint.clone(),
        objective: report.research_objective.clone(),
        expected_regime: report
            .artifact_decision_summary
            .consumed_trend_status
            .clone(),
        expected_direction: report
            .best_factor
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        realized_outcome: "research_completed".to_string(),
        evidence: report.multi_timeframe_summary.clone(),
        next_candidates: next_candidates.clone(),
    });
    if let Some(compare_summary) = compare_summary {
        bundle.compare_summary = Some(compare_summary.to_string());
        bundle
            .prior
            .expected_key_evidence
            .push(compare_summary.to_string());
        bundle
            .postmortem
            .evidence_drift
            .push(compare_summary.to_string());
        bundle
            .postmortem
            .what_worked
            .push(compare_summary.to_string());
    }
    let ensemble_vote = build_stub_ensemble_vote_from_research(report);
    bundle.ensemble_vote_summary = Some(ensemble_vote.human_next_triage.clone());
    bundle.ensemble_vote_artifact_id = Some(format!(
        "ensemble-vote:{}",
        report.provenance.data_fingerprint
    ));
    if !ensemble_vote.disagreement_flags.is_empty() {
        bundle.ensemble_disagreement_summary = Some(ensemble_vote.disagreement_flags.join(","));
    }
    let execution_artifact = build_execution_artifact(
        symbol,
        0.0,
        if report.recommended_commands.research.ready {
            0.8
        } else {
            0.35
        },
        if report.promotion_decision.approved {
            0.75
        } else {
            0.35
        },
        0.5,
        report.best_factor.as_ref().map(|_| 0.55).unwrap_or(0.25),
        None,
        None,
        None,
        None,
        None,
        None, // physics_overlay
        &report.provenance,
    );
    apply_execution_artifact_to_reflection_bundle(&mut bundle, &execution_artifact);
    bundle.prediction_summary = Some(format!(
        "prediction_edge={:.3}; best_factor={}",
        execution_artifact.features.prediction_edge_share,
        report
            .best_factor
            .clone()
            .unwrap_or_else(|| "unknown".to_string())
    ));
    let setup_family = report
        .pre_bayes_evidence_filter
        .nearest_active_pda
        .as_deref()
        .map(|label| {
            if label.starts_with("FairValueGap") {
                "fair_value_gap"
            } else if label.starts_with("InversionFairValueGap") {
                "inverse_fvg"
            } else if label.starts_with("OptimalTradeEntry") {
                "ote_confluence"
            } else if label.starts_with("SwingFailurePattern") {
                "judas_swing"
            } else {
                "none"
            }
        })
        .unwrap_or("none");
    let session_model = report
        .multi_timeframe_summary
        .iter()
        .find_map(|item| item.strip_prefix("multi_timeframe_source="))
        .map(|value| {
            let lower = value.to_ascii_lowercase();
            if lower.contains("silver") {
                "silver_bullet"
            } else if lower.contains("judas") {
                "judas"
            } else if lower.contains("turtle") {
                "turtle_soup"
            } else {
                "standard"
            }
        })
        .unwrap_or("standard");
    bundle.execution_setup_summary = Some(
        format!(
            "execution_setup_layer=experimental; setup_family={setup_family}; session_model={session_model}; active_pda_count={}; timed_pda_source=pre_bayes_filter",
            report.pre_bayes_evidence_filter.active_pda_count
        ),
    );
    bundle.execution_setup_guardrail = Some(
        "execution setup tree is policy metadata only; do not treat PDA heuristics as Bayesian hard evidence"
            .to_string(),
    );
    bundle
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn research_adapter_builds_bundle() {
        let report = ResearchReport {
            research_objective: "generic".to_string(),
            best_factor: Some("trend_momentum".to_string()),
            pre_bayes_evidence_filter: crate::state::PreBayesEvidenceFilter {
                active_pda_count: 2,
                nearest_active_pda: Some(
                    "FairValueGap:Bull|top=4200.0|bottom=4188.0|width_bps=28.6|sweep_depth_bps=0.0"
                        .to_string(),
                ),
                ..crate::state::PreBayesEvidenceFilter::default()
            },
            multi_timeframe_summary: vec![
                "mtf=bullish".to_string(),
                "multi_timeframe_source=silver_bullet_auto".to_string(),
            ],
            ..ResearchReport::default()
        };
        let bundle = build_research_reflection_bundle(
            "NQ",
            &report,
            Some("Research compare: duration_sizing_direction=scaled_up"),
        );
        assert_eq!(bundle.prior.symbol, "NQ");
        assert_eq!(bundle.postmortem.realized_outcome, "research_completed");
        assert!(bundle.ensemble_vote_summary.is_some());
        assert_eq!(
            bundle.compare_summary.as_deref(),
            Some("Research compare: duration_sizing_direction=scaled_up")
        );
        assert!(bundle
            .prior
            .expected_key_evidence
            .iter()
            .any(|line| line.contains("Research compare:")));
        assert!(bundle.execution_setup_summary.is_some());
        assert!(bundle
            .execution_setup_summary
            .as_deref()
            .unwrap_or_default()
            .contains("session_model=silver_bullet"));
        assert!(bundle.execution_setup_guardrail.is_some());
    }
}
