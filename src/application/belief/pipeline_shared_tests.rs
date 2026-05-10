use super::*;
use crate::state::{
    FactorPipelineLabelSource, PreBayesEntryQualityBridge, PreBayesEntryQualityBridgeDiff,
    PreBayesEvidenceFilter, StructuralBranchTransitionPrior, StructuralNodeDurationPrior,
    StructuralPriorEvent, StructuralPriorLearningState,
};
use std::collections::BTreeMap;

#[test]
fn build_factor_pipeline_debug_report_marks_missing_selected_entry_quality_unavailable() {
    let latest_signal = ExpansionLatestSignal {
        timestamp: chrono::Utc::now(),
        direction: "bull".to_string(),
        value: 1.0,
        confidence: 0.6,
        explanation: "test".to_string(),
    };
    let factor_diagnostics = ExpansionProbabilitySupport {
        long_support: 0.6,
        short_support: 0.4,
        support_gap: 0.2,
        alignment_threshold: 0.1,
        uncertainty: 0.2,
        alignment_label: "bullish".to_string(),
        uncertainty_label: "low".to_string(),
        long_entry_bias: vec![0.7, 0.2, 0.1],
        short_entry_bias: vec![0.2, 0.3, 0.5],
        bullish_factors: vec![],
        bearish_factors: vec![],
        uncertainty_factors: vec![],
    };
    let trace = FactorPipelineLabelSource {
        label: "bull".to_string(),
        derivation: "test".to_string(),
        evidence: vec!["e1".to_string()],
    };
    let bbn_support = ExpansionBbnSupport {
        market_regime_label: "bull".to_string(),
        liquidity_context_label: "neutral".to_string(),
        evidence_policy: "test_policy".to_string(),
        pre_bayes_filter: PreBayesEvidenceFilter {
            gating_status: "observe_only".to_string(),
            ..PreBayesEvidenceFilter::default()
        },
        evidence_assignments: BTreeMap::new(),
        raw_market_regime_trace: trace.clone(),
        raw_liquidity_context_trace: trace.clone(),
        raw_multi_timeframe_resonance_trace: trace,
        entry_quality_base: BTreeMap::new(),
        entry_quality_long: BTreeMap::new(),
        entry_quality_short: BTreeMap::new(),
        trade_outcome_long: BTreeMap::new(),
        trade_outcome_short: BTreeMap::new(),
        selected_direction: "bull".to_string(),
        selected_win_probability: 0.55,
    };
    let report = build_factor_pipeline_debug_report(FactorPipelineDebugReportInput {
        symbol: "NQ".to_string(),
        data: "NQ".to_string(),
        objective: "test".to_string(),
        factor_name: "factor_x".to_string(),
        latest_signal,
        factor_diagnostics,
        bbn_support,
        entry_quality_bridge: PreBayesEntryQualityBridge::default(),
        bridge_diff: PreBayesEntryQualityBridgeDiff {
            selected_entry_quality: None,
            ..PreBayesEntryQualityBridgeDiff::default()
        },
        multi_timeframe_summary: vec![],
        raw_pre_bayes_labels: BTreeMap::new(),
        soft_evidence_divergence: vec![],
        bridge_gap_clear_threshold: 0.12,
        paired_market_quality_report: None,
    })
    .unwrap();
    assert_eq!(report.selected_entry_quality, "entry_quality_unavailable");
}

#[test]
fn adapt_factor_pipeline_debug_report_prefers_explicit_paired_market_quality_report() {
    let trace = FactorPipelineLabelSource {
        label: "bull".to_string(),
        derivation: "test".to_string(),
        evidence: vec!["e1".to_string()],
    };
    let pipeline = ExpansionFactorPipelineReport {
        factor_name: "cross_market_smt".to_string(),
        parameters: BTreeMap::new(),
        latest_signal: ExpansionLatestSignal {
            timestamp: chrono::Utc::now(),
            direction: "bull".to_string(),
            value: 1.0,
            confidence: 0.5,
            explanation: "status=invalid_due_to_pair_quality;quality_tier=low;reason=from_explanation;aligned_length=2;primary_length=3;paired_length=4;overlap_ratio=0.5;safe_lookback=1".to_string(),
        },
        probability_support: ExpansionProbabilitySupport {
            long_support: 0.6,
            short_support: 0.4,
            support_gap: 0.2,
            alignment_threshold: 0.1,
            uncertainty: 0.2,
            alignment_label: "bullish".to_string(),
            uncertainty_label: "low".to_string(),
            long_entry_bias: vec![0.7, 0.2, 0.1],
            short_entry_bias: vec![0.2, 0.3, 0.5],
            bullish_factors: vec![],
            bearish_factors: vec![],
            uncertainty_factors: vec![],
        },
        paired_market_quality_report: Some(crate::factor_lab::PairedMarketQualityReport {
            paired_market_quality: "poor".to_string(),
            aligned_length: 2,
            primary_length: 3,
            paired_length: 4,
            overlap_ratio: 0.5,
            safe_lookback: 1,
            status: "invalid_due_to_pair_quality".to_string(),
            reason: "from_pipeline".to_string(),
        }),
        entry_quality_bridge: PreBayesEntryQualityBridge::default(),
        bbn_support: ExpansionBbnSupport {
            market_regime_label: "bull".to_string(),
            liquidity_context_label: "neutral".to_string(),
            evidence_policy: "test_policy".to_string(),
            pre_bayes_filter: PreBayesEvidenceFilter {
                gating_status: "observe_only".to_string(),
                ..PreBayesEvidenceFilter::default()
            },
            evidence_assignments: BTreeMap::new(),
            raw_market_regime_trace: trace.clone(),
            raw_liquidity_context_trace: trace.clone(),
            raw_multi_timeframe_resonance_trace: trace,
            entry_quality_base: BTreeMap::new(),
            entry_quality_long: BTreeMap::new(),
            entry_quality_short: BTreeMap::new(),
            trade_outcome_long: BTreeMap::new(),
            trade_outcome_short: BTreeMap::new(),
            selected_direction: "bull".to_string(),
            selected_win_probability: 0.55,
        },
        pipeline_summary: "summary".to_string(),
        recommended_actions: vec![],
        frame_physics_trace: vec![],
    };
    let explicit = crate::factor_lab::PairedMarketQualityReport {
        paired_market_quality: "medium".to_string(),
        aligned_length: 80,
        primary_length: 100,
        paired_length: 82,
        overlap_ratio: 0.80,
        safe_lookback: 24,
        status: "valid".to_string(),
        reason: "limited_pair_overlap".to_string(),
    };
    let report = adapt_factor_pipeline_debug_report(AdaptFactorPipelineDebugReportInput {
        symbol: "NQ",
        data: "test-data",
        objective: "test",
        pipeline: &pipeline,
        multi_timeframe_summary: &[],
        raw_pre_bayes_labels: BTreeMap::new(),
        soft_evidence_divergence: vec![],
        bridge_gap_clear_threshold: 0.12,
        paired_market_quality_report: Some(explicit.clone()),
    })
    .unwrap();
    assert_eq!(report.paired_market_quality_report, Some(explicit));
}

#[test]
fn canonical_belief_snapshot_with_structural_prior_state_uses_duration_prior_for_regime_confidence()
{
    let filter = PreBayesEvidenceFilter {
        filtered_market_regime_label: "bull".to_string(),
        soft_market_regime_distribution: BTreeMap::from([
            ("bull".to_string(), 0.72),
            ("range".to_string(), 0.18),
            ("transition".to_string(), 0.10),
        ]),
        uses_soft_evidence: true,
        evidence_quality_score: 1.0,
        ..PreBayesEvidenceFilter::default()
    };
    let mut structural_prior_state = StructuralPriorLearningState::default();
    structural_prior_state.node_duration_priors.insert(
        "NQ:belief_regime_node:trend".to_string(),
        StructuralNodeDurationPrior {
            observations: 6,
            streak_count: 3,
            weighted_streak_mass: 2.4,
            weighted_success_mass: 2.4,
            weighted_failure_mass: 0.0,
            total_streak_length: 6,
            avg_streak_length: 2.0,
            max_streak_length: 3,
            last_streak_length: 3,
            persistence_prior: 0.90,
            duration_outcome_support: 0.7727272727,
            temporal_posterior_support: 0.8618181818,
            last_recommended_at: Some("2026-04-30T03:00:00Z".to_string()),
            ..StructuralNodeDurationPrior::default()
        },
    );
    structural_prior_state.node_temporal_posteriors.insert(
        "NQ:belief_regime_node:trend".to_string(),
        crate::state::StructuralNodeTemporalPosteriorState {
            node_id: "NQ:belief_regime_node:trend".to_string(),
            observations: 9,
            streak_count: 4,
            weighted_streak_mass: 2.4,
            duration_outcome_support: 0.7727272727,
            temporal_posterior_support: 0.8618181818,
            posterior_blend_weight: 0.4,
            summary_line:
                "duration_mass=2.400 duration_support=0.773 duration_temporal=0.862 blend=0.400"
                    .to_string(),
            last_recommended_at: Some("2026-04-30T03:00:00Z".to_string()),
            ..crate::state::StructuralNodeTemporalPosteriorState::default()
        },
    );

    let report = build_canonical_belief_snapshot_with_pda_and_structural_prior_state(
        "NQ",
        Some("NQ"),
        &filter,
        None,
        None,
        None,
        Some(&structural_prior_state),
    )
    .unwrap();

    assert_eq!(
        report.regime_posterior.active_regime.as_deref(),
        Some("trend"),
        "probs={:?} report={report:?}",
        report.regime_posterior.probabilities,
    );
    assert!(report.regime_posterior.confidence.unwrap_or_default() > 0.72);
    assert!(report.regime_posterior.evidence.iter().any(|line| line
        .contains("duration_persistence_prior")
        && line.contains("observations=9")
        && line.contains("streaks=4")
        && line.contains("weighted_streak_mass=2.400")
        && line.contains("duration_outcome_support=0.773")
        && line.contains("duration_temporal_posterior_support=0.862")));
    assert!(report
        .regime_posterior
        .evidence
        .iter()
        .any(|line| line.contains("duration_posterior_blend_weight=0.400")));
    assert!(report
        .regime_posterior
        .evidence
        .iter()
        .any(|line| line.contains("node_temporal_summary=duration_mass=2.400")));
    let market_regime = report
        .belief_posteriors
        .iter()
        .find(|item| item.node_id == "market_regime")
        .expect("market_regime posterior");
    assert_eq!(market_regime.top_state, "trend");
    assert_eq!(
        market_regime.top_probability,
        report.regime_posterior.probabilities["trend"]
    );
}

#[test]
fn canonical_belief_snapshot_with_structural_prior_state_uses_branch_transition_priors() {
    let filter = PreBayesEvidenceFilter {
        filtered_market_regime_label: "bull".to_string(),
        soft_market_regime_distribution: BTreeMap::from([
            ("bull".to_string(), 0.55),
            ("range".to_string(), 0.30),
            ("transition".to_string(), 0.15),
        ]),
        uses_soft_evidence: true,
        ..PreBayesEvidenceFilter::default()
    };
    let mut structural_prior_state = StructuralPriorLearningState::default();
    structural_prior_state
        .event_ledger
        .push(StructuralPriorEvent {
            source_label: "backtest".to_string(),
            symbol: "NQ".to_string(),
            recommendation_id: "rec-prev".to_string(),
            recommended_at: "2026-04-30T01:00:00Z".to_string(),
            node_id: "NQ:belief_regime_node:trend".to_string(),
            branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
            scenario_id: "scenario:NQ:belief_regime_node:trend:trend_follow_through".to_string(),
            path_id: "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
                .to_string(),
            followed_path: true,
            realized_outcome: Some("win".to_string()),
        });
    structural_prior_state.branch_transition_priors.insert(
        "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:trend:transition_confirmation".to_string(),
        StructuralBranchTransitionPrior {
            from_node_id: "NQ:belief_regime_node:trend".to_string(),
            to_node_id: "NQ:belief_regime_node:trend".to_string(),
            from_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
            to_branch_id: "NQ:belief_regime_node:trend:transition_confirmation".to_string(),
            observations: 3,
            weighted_observation_mass: 2.4,
            wins: 2,
            losses: 1,
            invalidated: 0,
            transition_prior: 0.8,
            transition_outcome_support: 0.56,
            temporal_posterior_support: 0.728,
            weighted_success_mass: 1.6,
            weighted_failure_mass: 1.25,
            last_recommended_at: Some("2026-04-30T02:00:00Z".to_string()),
        },
    );
    structural_prior_state.branch_temporal_posteriors.insert(
        "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:trend:transition_confirmation".to_string(),
        crate::state::StructuralBranchTemporalPosteriorState {
            transition_key: "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:trend:transition_confirmation".to_string(),
            from_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
            to_branch_id: "NQ:belief_regime_node:trend:transition_confirmation".to_string(),
            observations: 3,
            weighted_observation_mass: 2.4,
            transition_prior: 0.8,
            transition_outcome_support: 0.56,
            temporal_posterior_support: 0.728,
            posterior_multiplier: 1.3648,
            normalized_transition_posterior: 0.8,
            summary_line: "transition_mass=2.400 transition_support=0.560 transition_temporal=0.728 multiplier=1.365".to_string(),
            last_recommended_at: Some("2026-04-30T02:00:00Z".to_string()),
        },
    );

    let baseline_report = build_canonical_belief_snapshot_with_pda_and_structural_prior_state(
        "NQ",
        Some("NQ"),
        &filter,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    let report = build_canonical_belief_snapshot_with_pda_and_structural_prior_state(
        "NQ",
        Some("NQ"),
        &filter,
        None,
        None,
        None,
        Some(&structural_prior_state),
    )
    .unwrap();

    let baseline_total: f64 = baseline_report
        .regime_posterior
        .probabilities
        .values()
        .copied()
        .sum();
    let baseline_transition =
        baseline_report.regime_posterior.probabilities["transition"] / baseline_total;
    let adjusted_transition = report.regime_posterior.probabilities["transition"];
    assert!(
        adjusted_transition > baseline_transition,
        "baseline={baseline_transition:?} adjusted={adjusted_transition:?} baseline_report={baseline_report:?} report={report:?}"
    );
    assert!(report.regime_posterior.evidence.iter().any(|line| line
        .contains("branch_transition_prior")
        && line.contains("weighted_transition_mass=2.400")
        && line.contains("transition_outcome_support=0.560")
        && line.contains("transition_temporal_posterior_support=0.728")));
    assert!(report
        .regime_posterior
        .evidence
        .iter()
        .any(|line| line.contains("transition_posterior_multiplier=1.365")));
    assert!(report
        .regime_posterior
        .evidence
        .iter()
        .any(|line| line.contains("branch_temporal_summary=transition_mass=2.400")));
}

#[test]
fn canonical_belief_snapshot_with_structural_prior_state_uses_node_transition_posterior() {
    let filter = PreBayesEvidenceFilter {
        filtered_market_regime_label: "bull".to_string(),
        soft_market_regime_distribution: BTreeMap::from([
            ("bull".to_string(), 0.60),
            ("range".to_string(), 0.25),
            ("transition".to_string(), 0.15),
        ]),
        uses_soft_evidence: true,
        ..PreBayesEvidenceFilter::default()
    };
    let mut structural_prior_state = StructuralPriorLearningState::default();
    structural_prior_state
        .event_ledger
        .push(StructuralPriorEvent {
            source_label: "backtest".to_string(),
            symbol: "NQ".to_string(),
            recommendation_id: "rec-prev".to_string(),
            recommended_at: "2026-04-30T01:00:00Z".to_string(),
            node_id: "NQ:belief_regime_node:trend".to_string(),
            branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
            scenario_id: "scenario:NQ:belief_regime_node:trend:trend_follow_through".to_string(),
            path_id: "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
                .to_string(),
            followed_path: true,
            realized_outcome: Some("win".to_string()),
        });
    let transition_key = "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:transition:transition_confirmation".to_string();
    structural_prior_state.branch_transition_priors.insert(
        transition_key.clone(),
        StructuralBranchTransitionPrior {
            from_node_id: "NQ:belief_regime_node:trend".to_string(),
            to_node_id: "NQ:belief_regime_node:transition".to_string(),
            from_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
            to_branch_id: "NQ:belief_regime_node:transition:transition_confirmation".to_string(),
            observations: 3,
            weighted_observation_mass: 2.4,
            wins: 2,
            losses: 1,
            invalidated: 0,
            transition_prior: 0.7,
            transition_outcome_support: 0.56,
            temporal_posterior_support: 0.728,
            weighted_success_mass: 1.6,
            weighted_failure_mass: 1.25,
            last_recommended_at: Some("2026-04-30T02:00:00Z".to_string()),
        },
    );
    structural_prior_state.branch_temporal_posteriors.insert(
        transition_key.clone(),
        crate::state::StructuralBranchTemporalPosteriorState {
            transition_key,
            from_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
            to_branch_id: "NQ:belief_regime_node:transition:transition_confirmation".to_string(),
            observations: 3,
            weighted_observation_mass: 2.4,
            transition_prior: 0.7,
            transition_outcome_support: 0.56,
            temporal_posterior_support: 0.728,
            posterior_multiplier: 0.2,
            normalized_transition_posterior: 0.7,
            summary_line: "transition_mass=2.400 transition_support=0.560 transition_temporal=0.728 multiplier=0.200".to_string(),
            last_recommended_at: Some("2026-04-30T02:00:00Z".to_string()),
        },
    );

    let report = build_canonical_belief_snapshot_with_pda_and_structural_prior_state(
        "NQ",
        Some("NQ"),
        &filter,
        None,
        None,
        None,
        Some(&structural_prior_state),
    )
    .unwrap();

    assert_eq!(
        report.regime_posterior.active_regime.as_deref(),
        Some("transition"),
        "regime_probs={:?} report={report:?}",
        report.regime_posterior.probabilities
    );
    assert!((report.regime_posterior.probabilities["transition"] - 0.7).abs() < 1e-9);
    assert!(report
        .regime_posterior
        .evidence
        .iter()
        .any(|line| line.contains(
            "node_transition_posterior_from=NQ:belief_regime_node:trend:trend_follow_through"
        )));
}

#[test]
fn canonical_belief_snapshot_with_structural_prior_state_reconciles_gate_and_strategy_with_adjusted_regime(
) {
    let filter = PreBayesEvidenceFilter {
        filtered_market_regime_label: "range".to_string(),
        soft_market_regime_distribution: BTreeMap::from([
            ("bull".to_string(), 0.40),
            ("range".to_string(), 0.45),
            ("transition".to_string(), 0.15),
        ]),
        uses_soft_evidence: true,
        evidence_quality_score: 1.0,
        ..PreBayesEvidenceFilter::default()
    };
    let mut structural_prior_state = StructuralPriorLearningState::default();
    structural_prior_state.node_duration_priors.insert(
        "NQ:belief_regime_node:trend".to_string(),
        StructuralNodeDurationPrior {
            observations: 8,
            streak_count: 4,
            weighted_streak_mass: 3.1,
            weighted_success_mass: 3.1,
            weighted_failure_mass: 0.0,
            total_streak_length: 10,
            avg_streak_length: 2.5,
            max_streak_length: 4,
            last_streak_length: 4,
            persistence_prior: 0.95,
            duration_outcome_support: 0.8039215686,
            temporal_posterior_support: 0.9061764706,
            last_recommended_at: Some("2026-04-30T04:00:00Z".to_string()),
            ..StructuralNodeDurationPrior::default()
        },
    );

    let report = build_canonical_belief_snapshot_with_pda_and_structural_prior_state(
        "NQ",
        Some("NQ"),
        &filter,
        None,
        None,
        None,
        Some(&structural_prior_state),
    )
    .unwrap();

    assert_eq!(
        report.regime_posterior.active_regime.as_deref(),
        Some("trend"),
        "regime_probs={:?} report={report:?}",
        report.regime_posterior.probabilities
    );
    assert_eq!(report.gate_decision.selected_regime, "trend");
    assert_eq!(report.strategy_recommendation.direction, "bull");
    assert_eq!(
        report
            .strategy_recommendation
            .selected_market_subgraph
            .as_deref(),
        Some("futures_index_trend_subgraph")
    );
    assert_eq!(
        report.selected_market_subgraph.as_deref(),
        Some("futures_index_trend_subgraph")
    );
    assert_eq!(
        report
            .temporal_summary
            .as_ref()
            .map(|summary| summary.dominant_regime.as_str()),
        Some("trend")
    );
}

#[test]
fn adapt_factor_pipeline_debug_report_uses_pipeline_structured_report_before_explanation() {
    let trace = FactorPipelineLabelSource {
        label: "bull".to_string(),
        derivation: "test".to_string(),
        evidence: vec!["e1".to_string()],
    };
    let pipeline_report = crate::factor_lab::PairedMarketQualityReport {
        paired_market_quality: "medium".to_string(),
        aligned_length: 80,
        primary_length: 100,
        paired_length: 82,
        overlap_ratio: 0.80,
        safe_lookback: 24,
        status: "valid".to_string(),
        reason: "from_pipeline".to_string(),
    };
    let pipeline = ExpansionFactorPipelineReport {
        factor_name: "cross_market_smt".to_string(),
        parameters: BTreeMap::new(),
        latest_signal: ExpansionLatestSignal {
            timestamp: chrono::Utc::now(),
            direction: "bull".to_string(),
            value: 1.0,
            confidence: 0.5,
            explanation: "status=invalid_due_to_pair_quality;quality_tier=low;reason=from_explanation;aligned_length=2;primary_length=3;paired_length=4;overlap_ratio=0.5;safe_lookback=1".to_string(),
        },
        probability_support: ExpansionProbabilitySupport {
            long_support: 0.6,
            short_support: 0.4,
            support_gap: 0.2,
            alignment_threshold: 0.1,
            uncertainty: 0.2,
            alignment_label: "bullish".to_string(),
            uncertainty_label: "low".to_string(),
            long_entry_bias: vec![0.7, 0.2, 0.1],
            short_entry_bias: vec![0.2, 0.3, 0.5],
            bullish_factors: vec![],
            bearish_factors: vec![],
            uncertainty_factors: vec![],
        },
        paired_market_quality_report: Some(pipeline_report.clone()),
        entry_quality_bridge: PreBayesEntryQualityBridge::default(),
        bbn_support: ExpansionBbnSupport {
            market_regime_label: "bull".to_string(),
            liquidity_context_label: "neutral".to_string(),
            evidence_policy: "test_policy".to_string(),
            pre_bayes_filter: PreBayesEvidenceFilter {
                gating_status: "observe_only".to_string(),
                ..PreBayesEvidenceFilter::default()
            },
            evidence_assignments: BTreeMap::new(),
            raw_market_regime_trace: trace.clone(),
            raw_liquidity_context_trace: trace.clone(),
            raw_multi_timeframe_resonance_trace: trace,
            entry_quality_base: BTreeMap::new(),
            entry_quality_long: BTreeMap::new(),
            entry_quality_short: BTreeMap::new(),
            trade_outcome_long: BTreeMap::new(),
            trade_outcome_short: BTreeMap::new(),
            selected_direction: "bull".to_string(),
            selected_win_probability: 0.55,
        },
        pipeline_summary: "summary".to_string(),
        recommended_actions: vec![],
        frame_physics_trace: vec![],
    };
    let report = adapt_factor_pipeline_debug_report(AdaptFactorPipelineDebugReportInput {
        symbol: "NQ",
        data: "test-data",
        objective: "test",
        pipeline: &pipeline,
        multi_timeframe_summary: &[],
        raw_pre_bayes_labels: BTreeMap::new(),
        soft_evidence_divergence: vec![],
        bridge_gap_clear_threshold: 0.12,
        paired_market_quality_report: None,
    })
    .unwrap();
    assert_eq!(report.paired_market_quality_report, Some(pipeline_report));
}
