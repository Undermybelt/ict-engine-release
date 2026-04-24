use anyhow::Result;
use serde::Serialize;

use crate::factor_lab::FactorContext;
use crate::factors::FactorRegistry;
use crate::ict::{check_bear_expansion_exists, check_bull_expansion_exists};
use crate::types::{Candle, Direction};

#[derive(Debug, Clone, Serialize)]
pub struct ExpansionFactorScore {
    pub factor_name: String,
    pub expansion_samples: usize,
    pub bull_expansion_samples: usize,
    pub bear_expansion_samples: usize,
    pub bull_hit_rate: f64,
    pub bear_hit_rate: f64,
    pub balanced_accuracy: f64,
    pub directional_accuracy: f64,
    pub confidence_weighted_accuracy: f64,
    pub mean_confidence: f64,
    pub neutral_predictions: usize,
    pub wrong_direction_predictions: usize,
    pub fit_score: f64,
}

pub fn expansion_factor_scores_for_market(
    registry: &FactorRegistry,
    candles: &[Candle],
    lookback: usize,
    atr_multiplier: f64,
) -> Result<Vec<ExpansionFactorScore>> {
    let context = FactorContext::default();
    let labels = expansion_direction_labels(candles, lookback, atr_multiplier);
    let bull_expansion_samples = labels
        .iter()
        .filter(|label| matches!(label, Some(Direction::Bull)))
        .count();
    let bear_expansion_samples = labels
        .iter()
        .filter(|label| matches!(label, Some(Direction::Bear)))
        .count();
    let expansion_samples = bull_expansion_samples + bear_expansion_samples;

    let mut scores = registry
        .enabled_factors()
        .into_iter()
        .map(|definition| {
            let series = definition.evaluate(candles, &context)?;
            let mut bull_hits = 0usize;
            let mut bear_hits = 0usize;
            let mut correct = 0usize;
            let mut neutral_predictions = 0usize;
            let mut wrong_direction_predictions = 0usize;
            let mut confidence_total = 0.0;
            let mut confidence_correct = 0.0;

            for (signal, label) in series.signals.iter().zip(labels.iter()) {
                let Some(label) = label else {
                    continue;
                };
                confidence_total += signal.confidence;
                match (signal.direction, label) {
                    (Direction::Bull, Direction::Bull) => {
                        bull_hits += 1;
                        correct += 1;
                        confidence_correct += signal.confidence;
                    }
                    (Direction::Bear, Direction::Bear) => {
                        bear_hits += 1;
                        correct += 1;
                        confidence_correct += signal.confidence;
                    }
                    (Direction::Neutral, _) => neutral_predictions += 1,
                    _ => wrong_direction_predictions += 1,
                }
            }

            let bull_hit_rate = if bull_expansion_samples == 0 {
                0.0
            } else {
                bull_hits as f64 / bull_expansion_samples as f64
            };
            let bear_hit_rate = if bear_expansion_samples == 0 {
                0.0
            } else {
                bear_hits as f64 / bear_expansion_samples as f64
            };
            let directional_accuracy = if expansion_samples == 0 {
                0.0
            } else {
                correct as f64 / expansion_samples as f64
            };
            let balanced_accuracy = if bull_expansion_samples > 0 && bear_expansion_samples > 0 {
                (bull_hit_rate + bear_hit_rate) / 2.0
            } else {
                directional_accuracy
            };
            let confidence_weighted_accuracy = if confidence_total <= f64::EPSILON {
                0.0
            } else {
                confidence_correct / confidence_total
            };
            let mean_confidence = if expansion_samples == 0 {
                0.0
            } else {
                confidence_total / expansion_samples as f64
            };
            let fit_score = balanced_accuracy * 0.70
                + directional_accuracy * 0.20
                + confidence_weighted_accuracy * 0.10;

            Ok(ExpansionFactorScore {
                factor_name: definition.name.clone(),
                expansion_samples,
                bull_expansion_samples,
                bear_expansion_samples,
                bull_hit_rate,
                bear_hit_rate,
                balanced_accuracy,
                directional_accuracy,
                confidence_weighted_accuracy,
                mean_confidence,
                neutral_predictions,
                wrong_direction_predictions,
                fit_score,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    scores.sort_by(|a, b| {
        b.fit_score
            .partial_cmp(&a.fit_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                b.balanced_accuracy
                    .partial_cmp(&a.balanced_accuracy)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                b.directional_accuracy
                    .partial_cmp(&a.directional_accuracy)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    Ok(scores)
}

fn expansion_direction_labels(
    candles: &[Candle],
    lookback: usize,
    atr_multiplier: f64,
) -> Vec<Option<Direction>> {
    candles
        .iter()
        .enumerate()
        .map(|(index, _)| expansion_direction_at(candles, index, lookback, atr_multiplier))
        .collect()
}

fn expansion_direction_at(
    candles: &[Candle],
    index: usize,
    lookback: usize,
    atr_multiplier: f64,
) -> Option<Direction> {
    let window_size = lookback.max(14) * 2;
    let start = index.saturating_sub(window_size);
    let window = &candles[start..=index];
    let effective_lookback = lookback.min(window.len());
    let bull = check_bull_expansion_exists(window, effective_lookback, atr_multiplier);
    let bear = check_bear_expansion_exists(window, effective_lookback, atr_multiplier);
    match (bull, bear) {
        (true, false) => Some(Direction::Bull),
        (false, true) => Some(Direction::Bear),
        _ => None,
    }
}
