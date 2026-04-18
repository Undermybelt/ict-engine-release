use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::factor_lab::factor_definition::{FactorContext, FactorSeries, FactorSignal};
use crate::factors::regime_conditional::RegimeConditional;
use crate::factors::registry::FactorRegistry;
use crate::state::LearningState;
use crate::types::{Candle, Direction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorContribution {
    pub factor_name: String,
    pub category: String,
    pub direction: Direction,
    pub value: f64,
    pub confidence: f64,
    pub weighted_score: f64,
    pub uncertainty_contribution: f64,
    pub explanation: String,
}

impl Default for FactorContribution {
    fn default() -> Self {
        Self {
            factor_name: String::new(),
            category: String::new(),
            direction: Direction::Neutral,
            value: 0.0,
            confidence: 0.0,
            weighted_score: 0.0,
            uncertainty_contribution: 0.0,
            explanation: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorDiagnostics {
    pub long_support: f64,
    pub short_support: f64,
    pub uncertainty: f64,
    pub alignment_label: String,
    pub uncertainty_label: String,
    pub long_entry_bias: Vec<f64>,
    pub short_entry_bias: Vec<f64>,
    pub bullish_factors: Vec<FactorContribution>,
    pub bearish_factors: Vec<FactorContribution>,
    pub uncertainty_factors: Vec<FactorContribution>,
}

pub const ALIGNMENT_SUPPORT_GAP_THRESHOLD: f64 = 0.10;

impl FactorDiagnostics {
    pub fn directional_bias(&self, direction: Direction) -> f64 {
        match direction {
            Direction::Bull => (self.long_support - self.short_support).clamp(-1.0, 1.0),
            Direction::Bear => (self.short_support - self.long_support).clamp(-1.0, 1.0),
            Direction::Neutral => 0.0,
        }
    }

    pub fn entry_bias_for_direction(&self, direction: Direction) -> Vec<f64> {
        match direction {
            Direction::Bull => self.long_entry_bias.clone(),
            Direction::Bear => self.short_entry_bias.clone(),
            Direction::Neutral => normalize_bias(vec![0.33, 0.34, 0.33]),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorEngineOutput {
    pub factor_series: Vec<FactorSeries>,
    pub latest_signals: Vec<FactorSignal>,
    pub diagnostics: FactorDiagnostics,
}

pub struct FactorEngine {
    pub registry: FactorRegistry,
}

impl FactorEngine {
    pub fn new(registry: FactorRegistry) -> Self {
        Self { registry }
    }

    pub fn run<'a>(
        &self,
        candles: &[Candle],
        context: &FactorContext<'a>,
        learning_state: Option<&LearningState>,
    ) -> Result<FactorEngineOutput> {
        let mut factor_series = Vec::new();
        let mut latest_signals = Vec::new();

        for definition in self.registry.enabled_factors() {
            let series = definition.evaluate(candles, context)?;
            if let Some(signal) = series.latest_signal() {
                latest_signals.push(signal);
            }
            factor_series.push(series);
        }

        if latest_signals.is_empty() {
            return Ok(FactorEngineOutput::default());
        }

        let regime = context
            .regime
            .unwrap_or(crate::types::Regime::ManipulationExpansion);
        let total_weight: f64 = latest_signals
            .iter()
            .map(|signal| {
                learning_state
                    .and_then(|state| state.profile(&signal.factor_name))
                    .map(|profile| profile.base_weight.max(0.0))
                    .unwrap_or(1.0)
            })
            .sum();
        let normalizer = total_weight.max(f64::EPSILON);

        let signal_count = latest_signals.len();
        for signal in &mut latest_signals {
            let profile = learning_state.and_then(|state| state.profile(&signal.factor_name));
            signal.weight = profile
                .map(|profile| profile.base_weight.max(0.0) / normalizer)
                .unwrap_or(1.0 / signal_count as f64);
            signal.posterior_reliability = profile
                .map(|profile| profile.posterior_reliability)
                .unwrap_or(0.5);
            signal.regime_multiplier = RegimeConditional::multiplier_opt(profile, regime);
            signal.regime_adjusted_score = signal.value
                * signal.confidence
                * signal.weight
                * signal.posterior_reliability
                * signal.regime_multiplier;
        }

        let diagnostics = build_diagnostics(&latest_signals);

        Ok(FactorEngineOutput {
            factor_series,
            latest_signals,
            diagnostics,
        })
    }
}

fn build_diagnostics(latest_signals: &[FactorSignal]) -> FactorDiagnostics {
    let mut diagnostics = FactorDiagnostics::default();

    for signal in latest_signals {
        let directional_score = signal.regime_adjusted_score;
        let uncertainty_contribution = ((1.0 - signal.confidence) * signal.weight * 0.45)
            .clamp(0.0, 1.0)
            + if matches!(
                signal.category,
                crate::factor_lab::factor_definition::FactorCategory::CrossMarketSmt
                    | crate::factor_lab::factor_definition::FactorCategory::OptionsHedging
            ) && signal.confidence < 0.35
            {
                signal.weight * 0.20
            } else {
                0.0
            };

        let contribution = FactorContribution {
            factor_name: signal.factor_name.clone(),
            category: signal.category.as_str().to_string(),
            direction: signal.direction,
            value: signal.value,
            confidence: signal.confidence,
            weighted_score: directional_score,
            uncertainty_contribution,
            explanation: signal.explanation.clone(),
        };

        match signal.direction {
            Direction::Bull => {
                diagnostics.long_support += directional_score.max(0.0);
                diagnostics.bullish_factors.push(contribution);
            }
            Direction::Bear => {
                diagnostics.short_support += (-directional_score).max(0.0);
                diagnostics.bearish_factors.push(contribution);
            }
            Direction::Neutral => diagnostics.uncertainty_factors.push(contribution),
        }

        diagnostics.uncertainty += uncertainty_contribution;
    }

    let scale =
        (diagnostics.long_support + diagnostics.short_support + diagnostics.uncertainty).max(1.0);
    diagnostics.long_support = (diagnostics.long_support / scale).clamp(0.0, 1.0);
    diagnostics.short_support = (diagnostics.short_support / scale).clamp(0.0, 1.0);
    diagnostics.uncertainty = (diagnostics.uncertainty / scale).clamp(0.0, 1.0);

    diagnostics.bullish_factors.sort_by(|a, b| {
        b.weighted_score
            .partial_cmp(&a.weighted_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    diagnostics.bearish_factors.sort_by(|a, b| {
        b.weighted_score
            .abs()
            .partial_cmp(&a.weighted_score.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    diagnostics.uncertainty_factors.sort_by(|a, b| {
        b.uncertainty_contribution
            .partial_cmp(&a.uncertainty_contribution)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    diagnostics.long_entry_bias = directional_entry_bias(
        diagnostics.long_support,
        diagnostics.short_support,
        diagnostics.uncertainty,
    );
    diagnostics.short_entry_bias = directional_entry_bias(
        diagnostics.short_support,
        diagnostics.long_support,
        diagnostics.uncertainty,
    );
    diagnostics.alignment_label = if diagnostics.long_support
        >= diagnostics.short_support + ALIGNMENT_SUPPORT_GAP_THRESHOLD
    {
        "bullish".to_string()
    } else if diagnostics.short_support
        >= diagnostics.long_support + ALIGNMENT_SUPPORT_GAP_THRESHOLD
    {
        "bearish".to_string()
    } else {
        "mixed".to_string()
    };
    diagnostics.uncertainty_label = if diagnostics.uncertainty >= 0.45 {
        "high".to_string()
    } else {
        "low".to_string()
    };

    diagnostics
}

fn directional_entry_bias(favored: f64, opposing: f64, uncertainty: f64) -> Vec<f64> {
    let quality = (favored - opposing).max(0.0);
    normalize_bias(vec![
        (0.25 + quality * 0.95 - uncertainty * 0.20).max(0.05),
        (0.35 + (favored + opposing) * 0.20).max(0.05),
        (0.20 + opposing * 0.80 + uncertainty * 0.70 - quality * 0.15).max(0.05),
    ])
}

fn normalize_bias(mut values: Vec<f64>) -> Vec<f64> {
    let sum: f64 = values.iter().sum();
    if sum <= f64::EPSILON {
        let uniform = 1.0 / values.len() as f64;
        values.fill(uniform);
        return values;
    }

    for value in &mut values {
        *value /= sum;
    }
    values
}

pub type FactorResearchEngine = FactorEngine;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factor_lab::factor_definition::{FactorCategory, FactorSignal};
    use crate::factors::FactorRegistry;
    use chrono::Utc;

    fn sample_signal(direction: Direction, weighted_score: f64) -> FactorSignal {
        FactorSignal {
            factor_name: "boundary_case".to_string(),
            category: FactorCategory::StructureIct,
            roles: vec![crate::factor_lab::factor_definition::FactorRole::StateTransition],
            direction,
            value: weighted_score.max(0.0),
            confidence: 1.0,
            explanation: format!("weighted_score={weighted_score}"),
            paired_market_quality_report: None,
            weight: 1.0,
            posterior_reliability: 1.0,
            regime_multiplier: 1.0,
            regime_adjusted_score: weighted_score,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_build_diagnostics_treats_equal_threshold_gap_as_bullish() {
        let diagnostics = build_diagnostics(&[
            sample_signal(Direction::Bull, ALIGNMENT_SUPPORT_GAP_THRESHOLD),
            sample_signal(Direction::Neutral, 0.0),
        ]);

        assert_eq!(diagnostics.long_support, ALIGNMENT_SUPPORT_GAP_THRESHOLD);
        assert_eq!(diagnostics.short_support, 0.0);
        assert_eq!(diagnostics.alignment_label, "bullish");
    }

    #[test]
    fn test_run_keeps_neutral_regime_multiplier_without_learning_profile() {
        let registry = FactorRegistry::default();
        let engine = FactorEngine::new(registry);
        let candles = (0..80)
            .map(|i| Candle {
                timestamp: Utc::now() + chrono::Duration::minutes(i as i64),
                open: 100.0 + i as f64 * 0.1,
                high: 100.5 + i as f64 * 0.1,
                low: 99.5 + i as f64 * 0.1,
                close: 100.2 + i as f64 * 0.1,
                volume: 1.0,
            })
            .collect::<Vec<_>>();
        let context = FactorContext {
            regime: Some(crate::types::Regime::Distribution),
            ..FactorContext::default()
        };

        let output = engine
            .run(&candles, &context, None)
            .expect("factor engine output");

        assert!(!output.latest_signals.is_empty());
        assert!(output
            .latest_signals
            .iter()
            .all(|signal| (signal.regime_multiplier - 1.0).abs() <= f64::EPSILON));
    }
}
