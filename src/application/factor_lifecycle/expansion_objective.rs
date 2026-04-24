use std::collections::{BTreeMap, HashMap};

use anyhow::Result;

use crate::application::backtest::pre_bayes_entry_quality_bridge_diff;
use crate::application::belief::{
    build_expansion_factor_pipeline_report_with_registry, market_category_for_symbol,
    objective_market_credibility_shrink,
};
use crate::application::decision_utils::score_grade;
use crate::application::factor_lifecycle::expansion_factor_scores_for_market;
use crate::factor_lab::ResearchReport;
use crate::factors::FactorRegistry;
use crate::state::FactorIterationPrompt;
use crate::types::Candle;

pub fn apply_expansion_manipulation_objective(
    report: &mut ResearchReport,
    registry: &FactorRegistry,
    symbol: &str,
    candles: &[Candle],
    multi_timeframe_summary: &[String],
    objective_jump_weight: Option<f64>,
) -> Result<()> {
    let expansion_scores = expansion_factor_scores_for_market(registry, candles, 20, 1.5)?
        .into_iter()
        .map(|score| (score.factor_name.clone(), score))
        .collect::<BTreeMap<_, _>>();

    let mut objective_scorecards = report.backtest.scorecards.clone();
    for scorecard in &mut objective_scorecards {
        let Some(expansion_score) = expansion_scores.get(&scorecard.factor_name) else {
            continue;
        };
        let pipeline = build_expansion_factor_pipeline_report_with_registry(
            symbol,
            &scorecard.factor_name,
            candles,
            None,
            multi_timeframe_summary,
            registry,
        )?;
        let gate_status = pipeline.bbn_support.pre_bayes_filter.gating_status.as_str();
        let bridge_gap = pre_bayes_entry_quality_bridge_diff(&pipeline.entry_quality_bridge)
            .long_short_signal_probability_gap;
        let bridge_gap_score = (bridge_gap / 0.25).clamp(0.0, 1.0);
        let gate_adjustment = match gate_status {
            "pass_hard" => 0.10,
            "pass_neutralized" => 0.03,
            "observe_only" => -0.12,
            _ => 0.0,
        };
        let objective_jump_weight = objective_jump_weight.unwrap_or(1.0).clamp(0.75, 1.35);
        let shrink = objective_market_credibility_shrink(
            Some("expansion_manipulation"),
            market_category_for_symbol(symbol),
            pipeline.bbn_support.pre_bayes_filter.evidence_quality_score,
        );
        let objective_score = (expansion_score.balanced_accuracy * 0.45
            + expansion_score.directional_accuracy * 0.20
            + expansion_score.fit_score * 0.15
            + bridge_gap_score * 0.10
            + pipeline.bbn_support.selected_win_probability * 0.10
            + gate_adjustment)
            * objective_jump_weight.clamp(0.75, 1.35)
            * shrink.shrink_weight;
        let objective_score = objective_score.clamp(0.0, 1.0);

        scorecard.composite_score = objective_score;
        scorecard.score_breakdown = BTreeMap::from([
            (
                "expansion_balanced_accuracy".to_string(),
                expansion_score.balanced_accuracy,
            ),
            (
                "expansion_directional_accuracy".to_string(),
                expansion_score.directional_accuracy,
            ),
            ("expansion_fit_score".to_string(), expansion_score.fit_score),
            ("pre_bayes_bridge_gap_score".to_string(), bridge_gap_score),
            (
                "selected_win_probability".to_string(),
                pipeline.bbn_support.selected_win_probability,
            ),
            ("objective_jump_weight".to_string(), objective_jump_weight),
            (
                "objective_market_shrink_weight".to_string(),
                shrink.shrink_weight,
            ),
            (
                "objective_market_credibility_score".to_string(),
                shrink.credibility_score,
            ),
        ]);
        let mut weaknesses = Vec::new();
        if expansion_score.balanced_accuracy < 0.60 {
            weaknesses.push("expansion_separation_weak".to_string());
        }
        if gate_status == "observe_only" {
            weaknesses.push("pre_bayes_gate_observe_only".to_string());
        }
        if bridge_gap < 0.05 {
            weaknesses.push("bridge_gap_too_small".to_string());
        }
        if pipeline
            .bbn_support
            .pre_bayes_filter
            .filtered_multi_timeframe_resonance_label
            != "aligned"
        {
            weaknesses.push("multi_timeframe_resonance_not_aligned".to_string());
        }
        scorecard.weaknesses = weaknesses;
        scorecard.grade = score_grade(objective_score);
        scorecard.iteration_action = if objective_score >= 0.82 && gate_status == "pass_hard" {
            "keep".to_string()
        } else if objective_score >= 0.60 {
            "tune".to_string()
        } else if objective_score >= 0.45 {
            "observe".to_string()
        } else {
            "replace".to_string()
        };
        scorecard.replacement_candidate = scorecard.iteration_action == "replace";
        scorecard.agent_prompt = format!(
            "Expansion/manipulation objective for '{}'. balanced_accuracy={:.3} directional_accuracy={:.3} gate_status={} bridge_gap={:.3}. Keep bull/bear expansion separation and liquidity-sweep manipulation discrimination while improving pre-bayes gate acceptance.",
            scorecard.factor_name,
            expansion_score.balanced_accuracy,
            expansion_score.directional_accuracy,
            gate_status,
            bridge_gap
        );
    }
    report.objective_surfaces = objective_scorecards
        .iter()
        .map(|scorecard| {
            HashMap::from([
                ("factor_name".to_string(), scorecard.factor_name.clone()),
                (
                    "research_objective".to_string(),
                    "expansion_manipulation".to_string(),
                ),
                (
                    "objective_score".to_string(),
                    format!("{:.6}", scorecard.composite_score),
                ),
                (
                    "objective_jump_weight".to_string(),
                    format!(
                        "{:.6}",
                        scorecard
                            .score_breakdown
                            .get("objective_jump_weight")
                            .copied()
                            .unwrap_or(1.0)
                    ),
                ),
                (
                    "objective_market_shrink_weight".to_string(),
                    format!(
                        "{:.6}",
                        scorecard
                            .score_breakdown
                            .get("objective_market_shrink_weight")
                            .copied()
                            .unwrap_or(1.0)
                    ),
                ),
                (
                    "objective_market_credibility_score".to_string(),
                    format!(
                        "{:.6}",
                        scorecard
                            .score_breakdown
                            .get("objective_market_credibility_score")
                            .copied()
                            .unwrap_or(1.0)
                    ),
                ),
            ])
        })
        .collect();
    objective_scorecards.sort_by(|a, b| {
        b.composite_score
            .partial_cmp(&a.composite_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    report.backtest.scorecards = objective_scorecards.clone();
    report.backtest.iteration_queue = objective_scorecards
        .iter()
        .map(FactorIterationPrompt::from)
        .filter(|item| item.iteration_action != "keep" || item.replacement_candidate)
        .collect();
    report.best_factor = objective_scorecards
        .first()
        .map(|scorecard| scorecard.factor_name.clone());
    report.backtest.best_factor = report.best_factor.clone();
    report.factor_count = objective_scorecards.len();
    Ok(())
}
