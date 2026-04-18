use anyhow::{anyhow, Result};

use crate::analyze::multi_timeframe_parse::parse_multi_timeframe_evidence;
use crate::bbn::trading::{
    topology::build_trading_network,
    update::{
        entry_quality_bias_from_signal, infer_entry_quality, infer_entry_quality_with_bias,
        infer_trade_outcome_with_entry_quality_bias, trade_evidence_from_pre_bayes_filter,
    },
};
use crate::config::{build_frame_features_for_market, build_pre_bayes_evidence_filter, env_bool};
use crate::factor_lab::{FactorContext, FactorEngine};
use crate::factors::FactorRegistry;
use crate::state::PreBayesEvidencePolicy;
use crate::types::{Candle, Direction, Regime};

pub use super::pipeline_shared::{
    adapt_factor_pipeline_debug_report, apply_factor_outcome_overlay,
    build_canonical_belief_report, build_canonical_belief_snapshot,
    build_factor_pipeline_debug_report, build_pre_bayes_entry_quality_bridge, combine_bias_vectors,
    effective_trade_outcome_win_probability, multi_timeframe_entry_quality_bias, probability_map,
    raw_liquidity_context_trace, raw_market_regime_trace, raw_multi_timeframe_resonance_trace,
    FactorPipelineDebugReport, PreBayesEntryQualityBridgeInput,
};
use super::pipeline_types::{
    ExpansionBbnSupport, ExpansionFactorPipelineReport, ExpansionLatestSignal,
    ExpansionProbabilitySupport,
};

pub fn infer_market_from_symbol(symbol: &str) -> String {
    symbol
        .split(['.', '_', '-'])
        .next()
        .unwrap_or(symbol)
        .to_ascii_uppercase()
}

pub fn build_expansion_factor_pipeline_report(
    symbol: &str,
    factor_name: &str,
    candles: &[Candle],
    multi_timeframe_summary: &[String],
) -> Result<ExpansionFactorPipelineReport> {
    let mut registry = FactorRegistry::default();
    let factor_names = registry
        .list()
        .into_iter()
        .map(|definition| definition.name.clone())
        .collect::<Vec<_>>();
    for name in factor_names {
        registry.set_enabled(&name, name == factor_name);
    }
    build_expansion_factor_pipeline_report_with_registry(
        symbol,
        factor_name,
        candles,
        None,
        multi_timeframe_summary,
        &registry,
    )
}

pub fn build_expansion_factor_pipeline_report_with_registry(
    symbol: &str,
    factor_name: &str,
    candles: &[Candle],
    paired_candles: Option<&[Candle]>,
    multi_timeframe_summary: &[String],
    registry: &FactorRegistry,
) -> Result<ExpansionFactorPipelineReport> {
    let definition = registry
        .get(factor_name)
        .cloned()
        .ok_or_else(|| anyhow!("unknown factor '{}'", factor_name))?;
    let factor_engine = FactorEngine::new(registry.clone());
    let output = factor_engine.run(
        candles,
        &FactorContext {
            paired_candles,
            regime: Some(Regime::ManipulationExpansion),
            ..FactorContext::default()
        },
        None,
    )?;
    let signal = output
        .latest_signals
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("factor '{}' did not produce a latest signal", factor_name))?;
    let market = infer_market_from_symbol(symbol);
    let frame = build_frame_features_for_market(candles, Some(&market))?;
    let market_regime_trace = raw_market_regime_trace(
        &frame.regime_label,
        &frame.regime_label,
        frame.sweep_count,
        frame.fvg_count,
        frame.normalized_distance_to_range_mid_bps,
        frame.normalized_distance_to_projected_trend_bps,
        frame.ou_half_life_bars,
        frame.ou_reversion_speed_per_bar,
        frame.ou_pullback_expectation_zscore,
    );
    let liquidity_context_trace = raw_liquidity_context_trace(
        &frame.liquidity_label,
        &frame.liquidity_label,
        frame.sweep_count,
        frame.fvg_count,
        frame.normalized_distance_to_range_mid_bps,
        frame.normalized_distance_to_projected_trend_bps,
        frame.ou_half_life_bars,
        frame.ou_reversion_speed_per_bar,
        frame.ou_pullback_expectation_zscore,
    );
    let network = build_trading_network()?;
    let pre_bayes_policy = pre_bayes_evidence_policy();
    let multi_timeframe_evidence = parse_multi_timeframe_evidence(multi_timeframe_summary);
    let pre_bayes_filter = build_pre_bayes_evidence_filter(
        &pre_bayes_policy,
        &frame.regime_label,
        &frame.liquidity_label,
        &output.diagnostics,
        &multi_timeframe_evidence,
        Some(&market),
    );
    let mut recommended_physics_actions = Vec::new();
    let distance_enabled = env_bool("ICT_ENGINE_FEATURE_DISTANCE_ONLY", false)
        || env_bool("ICT_ENGINE_FEATURE_DISTANCE_AND_OU", false);
    let ou_enabled = env_bool("ICT_ENGINE_FEATURE_OU_ONLY", false)
        || env_bool("ICT_ENGINE_FEATURE_DISTANCE_AND_OU", false);
    if distance_enabled {
        if frame.normalized_distance_to_range_mid_bps.abs() >= 800.0 {
            recommended_physics_actions.push(format!(
                "distance_feature: price is stretched {:.1}bps from range mid",
                frame.normalized_distance_to_range_mid_bps
            ));
        }
        if frame.normalized_distance_to_projected_trend_bps.abs() <= 150.0 {
            recommended_physics_actions
                .push("distance_feature: price remains close to projected trend path".to_string());
        }
    }
    if ou_enabled {
        if frame.ou_pullback_expectation_zscore.abs() >= 2.0 {
            recommended_physics_actions.push(format!(
                "ou_feature: pullback pressure zscore={:.2}",
                frame.ou_pullback_expectation_zscore
            ));
        }
        if frame.ou_half_life_bars >= 1000.0 {
            recommended_physics_actions
                .push("ou_feature: mean reversion is slow on the current path".to_string());
        }
    }
    let resonance_trace = raw_multi_timeframe_resonance_trace(
        &pre_bayes_policy,
        &pre_bayes_filter,
        &multi_timeframe_evidence,
        &frame.regime_label,
        &output.diagnostics.alignment_label,
    );
    let evidence_assignments = pre_bayes_filter.evidence_assignments.clone();
    let evidence = trade_evidence_from_pre_bayes_filter(&network, &pre_bayes_filter)?;
    let base_entry_quality = infer_entry_quality(&network, &evidence)?;
    let long_bias = combine_bias_vectors(
        &combine_bias_vectors(
            &entry_quality_bias_from_signal(output.diagnostics.long_support.max(signal.confidence)),
            &output.diagnostics.entry_bias_for_direction(Direction::Bull),
        ),
        &multi_timeframe_entry_quality_bias(&multi_timeframe_evidence, Direction::Bull),
    );
    let short_bias = combine_bias_vectors(
        &combine_bias_vectors(
            &entry_quality_bias_from_signal(
                output.diagnostics.short_support.max(signal.confidence),
            ),
            &output.diagnostics.entry_bias_for_direction(Direction::Bear),
        ),
        &multi_timeframe_entry_quality_bias(&multi_timeframe_evidence, Direction::Bear),
    );
    let long_entry_quality = infer_entry_quality_with_bias(&network, &evidence, &long_bias)?;
    let short_entry_quality = infer_entry_quality_with_bias(&network, &evidence, &short_bias)?;
    let long_trade_outcome = apply_factor_outcome_overlay(
        &infer_trade_outcome_with_entry_quality_bias(&network, &evidence, &long_bias)?,
        output.diagnostics.directional_bias(Direction::Bull),
        output.diagnostics.uncertainty,
    );
    let short_trade_outcome = apply_factor_outcome_overlay(
        &infer_trade_outcome_with_entry_quality_bias(&network, &evidence, &short_bias)?,
        output.diagnostics.directional_bias(Direction::Bear),
        output.diagnostics.uncertainty,
    );
    let trade_outcome_node = network
        .nodes
        .get("trade_outcome")
        .ok_or_else(|| anyhow!("missing node 'trade_outcome'"))?;
    let entry_quality_node = network
        .nodes
        .get("entry_quality")
        .ok_or_else(|| anyhow!("missing node 'entry_quality'"))?;
    let long_win_probability = effective_trade_outcome_win_probability(&long_trade_outcome);
    let short_win_probability = effective_trade_outcome_win_probability(&short_trade_outcome);
    let (selected_direction, selected_win_probability) =
        if long_win_probability >= short_win_probability {
            ("bull".to_string(), long_win_probability)
        } else {
            ("bear".to_string(), short_win_probability)
        };
    let entry_quality_bridge =
        build_pre_bayes_entry_quality_bridge(PreBayesEntryQualityBridgeInput {
            factor_diagnostics: output.diagnostics.clone(),
            decision: crate::planner::ProbabilisticDecisionSnapshot {
                long_score: output.diagnostics.long_support,
                short_score: output.diagnostics.short_support,
                win_prob_long: long_win_probability,
                win_prob_short: short_win_probability,
                ict_support_long: 0.0,
                ict_support_short: 0.0,
                selected_direction: if selected_direction == "bull" {
                    Direction::Bull
                } else {
                    Direction::Bear
                },
                selected_score: long_win_probability.max(short_win_probability),
                selected_win_probability,
                ict_role: "expansion_sop_factor_only".to_string(),
            },
            long_entry_bias: long_bias.clone(),
            short_entry_bias: short_bias.clone(),
            long_entry_quality: long_entry_quality.clone(),
            short_entry_quality: short_entry_quality.clone(),
            selected_entry_quality: if selected_direction == "bull" {
                long_entry_quality.clone()
            } else {
                short_entry_quality.clone()
            },
            entry_quality_states: entry_quality_node.states.clone(),
            multi_timeframe_evidence: multi_timeframe_evidence.clone(),
        });

    Ok(ExpansionFactorPipelineReport {
        factor_name: factor_name.to_string(),
        parameters: definition.parameters,
        latest_signal: ExpansionLatestSignal {
            timestamp: signal.timestamp,
            direction: format!("{:?}", signal.direction),
            value: signal.value,
            confidence: signal.confidence,
            explanation: signal.explanation.clone(),
        },
        probability_support: ExpansionProbabilitySupport {
            long_support: output.diagnostics.long_support,
            short_support: output.diagnostics.short_support,
            support_gap: (output.diagnostics.long_support - output.diagnostics.short_support).abs(),
            alignment_threshold: 0.10,
            uncertainty: output.diagnostics.uncertainty,
            alignment_label: output.diagnostics.alignment_label.clone(),
            uncertainty_label: output.diagnostics.uncertainty_label.clone(),
            long_entry_bias: output.diagnostics.entry_bias_for_direction(Direction::Bull),
            short_entry_bias: output.diagnostics.entry_bias_for_direction(Direction::Bear),
            bullish_factors: output.diagnostics.bullish_factors.clone(),
            bearish_factors: output.diagnostics.bearish_factors.clone(),
            uncertainty_factors: output.diagnostics.uncertainty_factors.clone(),
        },
        paired_market_quality_report: signal.paired_market_quality_report.clone(),
        entry_quality_bridge,
        bbn_support: ExpansionBbnSupport {
            market_regime_label: frame.regime_label,
            liquidity_context_label: frame.liquidity_label,
            evidence_policy: "expansion_sop_factor_signal_to_pre_bayes_soft_evidence_to_bbn"
                .to_string(),
            pre_bayes_filter: pre_bayes_filter.clone(),
            evidence_assignments,
            raw_market_regime_trace: market_regime_trace.clone(),
            raw_liquidity_context_trace: liquidity_context_trace.clone(),
            raw_multi_timeframe_resonance_trace: resonance_trace,
            entry_quality_base: probability_map(&entry_quality_node.states, &base_entry_quality),
            entry_quality_long: probability_map(&entry_quality_node.states, &long_entry_quality),
            entry_quality_short: probability_map(&entry_quality_node.states, &short_entry_quality),
            trade_outcome_long: probability_map(&trade_outcome_node.states, &long_trade_outcome),
            trade_outcome_short: probability_map(&trade_outcome_node.states, &short_trade_outcome),
            selected_direction,
            selected_win_probability,
        },
        pipeline_summary: format!(
            "factor_signal -> diagnostics(alignment={}, uncertainty={}) -> evidence(market_regime/liquidity_context/factor_alignment/factor_uncertainty) -> entry_quality -> trade_outcome",
            output.diagnostics.alignment_label,
            output.diagnostics.uncertainty_label
        ),
        recommended_actions: {
            let mut actions = vec![
                format!(
                    "Use {} as the primary expansion discrimination factor in the MVP probability stack",
                    factor_name
                ),
                "Treat factor_alignment and factor_uncertainty as the BBN bridge, not as hard triggers"
                    .to_string(),
                "Review whether market_regime/liquidity_context labels should be made more independent from the expansion heuristic".to_string(),
            ];
            actions.extend(recommended_physics_actions);
            actions
        },
        frame_physics_trace: vec![market_regime_trace.clone(), liquidity_context_trace.clone()],
    })
}

pub fn pre_bayes_evidence_policy() -> PreBayesEvidencePolicy {
    let min_directional_support_gap =
        crate::config::env_f64("ICT_ENGINE_PREBAYES_MIN_SUPPORT_GAP", 0.08);
    let high_uncertainty_threshold =
        crate::config::env_f64("ICT_ENGINE_PREBAYES_HIGH_UNCERTAINTY_THRESHOLD", 0.45);
    let min_multi_timeframe_alignment_score =
        crate::config::env_f64("ICT_ENGINE_PREBAYES_MIN_MTF_ALIGNMENT_SCORE", 0.55);
    let min_multi_timeframe_entry_alignment_score =
        crate::config::env_f64("ICT_ENGINE_PREBAYES_MIN_MTF_ENTRY_ALIGNMENT_SCORE", 0.50);
    let hard_pass_quality_threshold =
        crate::config::env_f64("ICT_ENGINE_PREBAYES_HARD_PASS_QUALITY_THRESHOLD", 0.75);
    let neutralized_quality_threshold =
        crate::config::env_f64("ICT_ENGINE_PREBAYES_NEUTRALIZED_QUALITY_THRESHOLD", 0.40);
    let directional_conflict_penalty =
        crate::config::env_f64("ICT_ENGINE_PREBAYES_DIRECTIONAL_CONFLICT_PENALTY", 0.20);
    let mixed_alignment_penalty =
        crate::config::env_f64("ICT_ENGINE_PREBAYES_MIXED_ALIGNMENT_PENALTY", 0.10);
    let multi_timeframe_direction_conflict_penalty =
        crate::config::env_f64("ICT_ENGINE_PREBAYES_MTF_DIRECTION_CONFLICT_PENALTY", 0.18);
    let multi_timeframe_alignment_penalty =
        crate::config::env_f64("ICT_ENGINE_PREBAYES_MTF_ALIGNMENT_PENALTY", 0.10);
    let multi_timeframe_entry_penalty =
        crate::config::env_f64("ICT_ENGINE_PREBAYES_MTF_ENTRY_PENALTY", 0.08);
    let multi_timeframe_alignment_bonus =
        crate::config::env_f64("ICT_ENGINE_PREBAYES_MTF_ALIGNMENT_BONUS", 0.05);
    let hostile_liquidity_penalty =
        crate::config::env_f64("ICT_ENGINE_PREBAYES_HOSTILE_LIQUIDITY_PENALTY", 0.10);
    let favorable_liquidity_bonus =
        crate::config::env_f64("ICT_ENGINE_PREBAYES_FAVORABLE_LIQUIDITY_BONUS", 0.05);
    let hostile_liquidity_forces_high_uncertainty = crate::config::env_bool(
        "ICT_ENGINE_PREBAYES_HOSTILE_LIQUIDITY_FORCES_HIGH_UNCERTAINTY",
        true,
    );
    let market_overrides = std::collections::BTreeMap::from([
        (
            "NQ".to_string(),
            crate::state::PreBayesMarketPolicyOverride {
                hostile_liquidity_penalty: Some(0.10),
                favorable_liquidity_bonus: Some(0.04),
                hostile_liquidity_forces_high_uncertainty: Some(true),
            },
        ),
        (
            "ES".to_string(),
            crate::state::PreBayesMarketPolicyOverride {
                hostile_liquidity_penalty: Some(0.06),
                favorable_liquidity_bonus: Some(0.06),
                hostile_liquidity_forces_high_uncertainty: Some(false),
            },
        ),
        (
            "YM".to_string(),
            crate::state::PreBayesMarketPolicyOverride {
                hostile_liquidity_penalty: Some(0.07),
                favorable_liquidity_bonus: Some(0.05),
                hostile_liquidity_forces_high_uncertainty: Some(false),
            },
        ),
        (
            "GC".to_string(),
            crate::state::PreBayesMarketPolicyOverride {
                hostile_liquidity_penalty: Some(0.09),
                favorable_liquidity_bonus: Some(0.07),
                hostile_liquidity_forces_high_uncertainty: Some(false),
            },
        ),
        (
            "CL".to_string(),
            crate::state::PreBayesMarketPolicyOverride {
                hostile_liquidity_penalty: Some(0.14),
                favorable_liquidity_bonus: Some(0.03),
                hostile_liquidity_forces_high_uncertainty: Some(true),
            },
        ),
    ]);
    let source = if [
        "ICT_ENGINE_PREBAYES_MIN_SUPPORT_GAP",
        "ICT_ENGINE_PREBAYES_HIGH_UNCERTAINTY_THRESHOLD",
        "ICT_ENGINE_PREBAYES_MIN_MTF_ALIGNMENT_SCORE",
        "ICT_ENGINE_PREBAYES_MIN_MTF_ENTRY_ALIGNMENT_SCORE",
        "ICT_ENGINE_PREBAYES_HARD_PASS_QUALITY_THRESHOLD",
        "ICT_ENGINE_PREBAYES_NEUTRALIZED_QUALITY_THRESHOLD",
        "ICT_ENGINE_PREBAYES_DIRECTIONAL_CONFLICT_PENALTY",
        "ICT_ENGINE_PREBAYES_MIXED_ALIGNMENT_PENALTY",
        "ICT_ENGINE_PREBAYES_MTF_DIRECTION_CONFLICT_PENALTY",
        "ICT_ENGINE_PREBAYES_MTF_ALIGNMENT_PENALTY",
        "ICT_ENGINE_PREBAYES_MTF_ENTRY_PENALTY",
        "ICT_ENGINE_PREBAYES_MTF_ALIGNMENT_BONUS",
        "ICT_ENGINE_PREBAYES_HOSTILE_LIQUIDITY_PENALTY",
        "ICT_ENGINE_PREBAYES_FAVORABLE_LIQUIDITY_BONUS",
        "ICT_ENGINE_PREBAYES_HOSTILE_LIQUIDITY_FORCES_HIGH_UNCERTAINTY",
    ]
    .iter()
    .any(|name| std::env::var(name).is_ok())
    {
        "env_or_default".to_string()
    } else {
        "default".to_string()
    };
    let mut version_inputs = vec![
        format!("{:.6}", min_directional_support_gap),
        format!("{:.6}", high_uncertainty_threshold),
        format!("{:.6}", min_multi_timeframe_alignment_score),
        format!("{:.6}", min_multi_timeframe_entry_alignment_score),
        format!("{:.6}", hard_pass_quality_threshold),
        format!("{:.6}", neutralized_quality_threshold),
        format!("{:.6}", directional_conflict_penalty),
        format!("{:.6}", mixed_alignment_penalty),
        format!("{:.6}", multi_timeframe_direction_conflict_penalty),
        format!("{:.6}", multi_timeframe_alignment_penalty),
        format!("{:.6}", multi_timeframe_entry_penalty),
        format!("{:.6}", multi_timeframe_alignment_bonus),
        format!("{:.6}", hostile_liquidity_penalty),
        format!("{:.6}", favorable_liquidity_bonus),
        hostile_liquidity_forces_high_uncertainty.to_string(),
    ];
    for (market, override_policy) in &market_overrides {
        version_inputs.push(format!("market={market}"));
        version_inputs.push(format!(
            "hostile_liquidity_penalty={}",
            override_policy
                .hostile_liquidity_penalty
                .map(|value| format!("{value:.6}"))
                .unwrap_or_else(|| "none".to_string())
        ));
        version_inputs.push(format!(
            "favorable_liquidity_bonus={}",
            override_policy
                .favorable_liquidity_bonus
                .map(|value| format!("{value:.6}"))
                .unwrap_or_else(|| "none".to_string())
        ));
        version_inputs.push(format!(
            "hostile_liquidity_forces_high_uncertainty={}",
            override_policy
                .hostile_liquidity_forces_high_uncertainty
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_string())
        ));
    }
    let version = crate::config::compute_hash(&version_inputs);
    crate::state::PreBayesEvidencePolicy {
        version,
        source,
        min_directional_support_gap,
        high_uncertainty_threshold,
        min_multi_timeframe_alignment_score,
        min_multi_timeframe_entry_alignment_score,
        hard_pass_quality_threshold,
        neutralized_quality_threshold,
        directional_conflict_penalty,
        mixed_alignment_penalty,
        multi_timeframe_direction_conflict_penalty,
        multi_timeframe_alignment_penalty,
        multi_timeframe_entry_penalty,
        multi_timeframe_alignment_bonus,
        hostile_liquidity_penalty,
        favorable_liquidity_bonus,
        hostile_liquidity_forces_high_uncertainty,
        market_overrides,
    }
}
