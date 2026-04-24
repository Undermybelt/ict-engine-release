use anyhow::{anyhow, Result};

use crate::agent::PROMPT_PACK_VERSION;
use crate::application::decision_utils::normalize_trade_outcome_label;
use crate::bbn::learning::cpt_updater::TradeOutcome;
use crate::bbn::learning::CPTUpdater;
use crate::bbn::trading::update::trade_evidence_from_labels;
use crate::state::{FeedbackFactorUsage, FeedbackRecord, LearningState, ModelProbabilitySnapshot};
use crate::types::Direction;

pub struct BuildFeedbackRecordInput<'a> {
    pub symbol: &'a str,
    pub source: &'a str,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub factor_diagnostics: &'a crate::factor_lab::FactorDiagnostics,
    pub decision: &'a crate::planner::ProbabilisticDecisionSnapshot,
    pub pnl: f64,
    pub realized_outcome: String,
    pub regime_at_entry: crate::types::Regime,
}

pub fn trade_outcome_label_from_pnl(pnl: f64) -> String {
    if pnl > 1e-12 {
        "win".to_string()
    } else if pnl < -1e-12 {
        "loss".to_string()
    } else {
        "breakeven".to_string()
    }
}

pub fn build_feedback_record(input: BuildFeedbackRecordInput<'_>) -> FeedbackRecord {
    let BuildFeedbackRecordInput {
        symbol,
        source,
        timestamp,
        factor_diagnostics,
        decision,
        pnl,
        realized_outcome,
        regime_at_entry,
    } = input;
    let mut factors_used = Vec::new();
    for factor in factor_diagnostics
        .bullish_factors
        .iter()
        .chain(factor_diagnostics.bearish_factors.iter())
        .chain(factor_diagnostics.uncertainty_factors.iter())
    {
        factors_used.push(FeedbackFactorUsage {
            factor_name: factor.factor_name.clone(),
            category: factor.category.clone(),
            direction: factor.direction,
            value: factor.value,
            confidence: factor.confidence,
            weight: factor.weighted_score.abs(),
            long_support: if factor.direction == Direction::Bull {
                factor.weighted_score.max(0.0)
            } else {
                0.0
            },
            short_support: if factor.direction == Direction::Bear {
                factor.weighted_score.abs()
            } else {
                0.0
            },
            uncertainty_contribution: factor.uncertainty_contribution,
        });
    }

    FeedbackRecord {
        timestamp,
        symbol: symbol.to_string(),
        source: source.to_string(),
        run_id: None,
        trade_id: None,
        prompt_version: Some(PROMPT_PACK_VERSION.to_string()),
        factor_version: None,
        data_fingerprint: None,
        factors_used,
        model_probabilities_before_trade: ModelProbabilitySnapshot {
            selected_direction: decision.selected_direction,
            selected_probability: decision.selected_win_probability,
            long_score: decision.long_score,
            short_score: decision.short_score,
            win_prob_long: decision.win_prob_long,
            win_prob_short: decision.win_prob_short,
            uncertainty: factor_diagnostics.uncertainty,
        },
        realized_outcome,
        pnl,
        regime_at_entry,
    }
}

pub fn enrich_feedback_record(
    mut feedback: FeedbackRecord,
    run_id: &str,
    trade_id: impl Into<String>,
    learning_state: &LearningState,
    data_fingerprint: &str,
) -> FeedbackRecord {
    if feedback.run_id.is_none() {
        feedback.run_id = Some(run_id.to_string());
    }
    if feedback.trade_id.is_none() {
        feedback.trade_id = Some(trade_id.into());
    }
    if feedback.prompt_version.is_none() {
        feedback.prompt_version = Some(PROMPT_PACK_VERSION.to_string());
    }
    if feedback.factor_version.is_none() {
        feedback.factor_version =
            Some(crate::application::backtest::factor_version(learning_state));
    }
    if feedback.data_fingerprint.is_none() {
        feedback.data_fingerprint = Some(data_fingerprint.to_string());
    }
    feedback
}

pub fn apply_feedback_to_trade_outcome_network(
    network: &mut crate::bbn::BayesianNetwork,
    feedback: &[FeedbackRecord],
) -> Result<usize> {
    let mut updates = Vec::new();

    for record in feedback {
        let entry_quality = entry_quality_label_from_probability(
            record.model_probabilities_before_trade.selected_probability,
        );
        let factor_alignment = factor_alignment_label_from_feedback(record);
        let factor_uncertainty = factor_uncertainty_label_from_feedback(record);
        let evidence = trade_evidence_from_labels(
            network,
            &[
                ("entry_quality", entry_quality),
                ("factor_alignment", factor_alignment.as_str()),
                ("factor_uncertainty", factor_uncertainty.as_str()),
            ],
        )?;
        let outcome_label = normalize_trade_outcome_label(&record.realized_outcome);
        let realized_state_index = network
            .nodes
            .get("trade_outcome")
            .and_then(|node| node.state_index(&outcome_label))
            .ok_or_else(|| anyhow!("unknown trade outcome state '{}'", outcome_label))?;
        updates.push((
            evidence,
            TradeOutcome {
                node_id: "trade_outcome".to_string(),
                realized_state_index,
            },
        ));
    }

    if updates.is_empty() {
        return Ok(0);
    }

    CPTUpdater::default().batch_update(network, &updates)?;
    Ok(updates.len())
}

fn entry_quality_label_from_probability(probability: f64) -> &'static str {
    if probability >= 0.66 {
        "high"
    } else if probability <= 0.33 {
        "low"
    } else {
        "medium"
    }
}

pub fn factor_alignment_label_from_feedback(record: &FeedbackRecord) -> String {
    if record.factors_used.is_empty() {
        return match record.model_probabilities_before_trade.selected_direction {
            Direction::Bull => "bullish".to_string(),
            Direction::Bear => "bearish".to_string(),
            Direction::Neutral => "mixed".to_string(),
        };
    }

    let long_support = record
        .factors_used
        .iter()
        .map(|factor| factor.long_support)
        .sum::<f64>();
    let short_support = record
        .factors_used
        .iter()
        .map(|factor| factor.short_support)
        .sum::<f64>();

    if long_support > short_support + 0.05 {
        "bullish".to_string()
    } else if short_support > long_support + 0.05 {
        "bearish".to_string()
    } else {
        "mixed".to_string()
    }
}

pub fn factor_uncertainty_label_from_feedback(record: &FeedbackRecord) -> String {
    let uncertainty = if record.factors_used.is_empty() {
        record.model_probabilities_before_trade.uncertainty
    } else {
        record
            .factors_used
            .iter()
            .map(|factor| factor.uncertainty_contribution)
            .sum::<f64>()
    };
    if uncertainty >= 0.45 {
        "high".to_string()
    } else {
        "low".to_string()
    }
}
