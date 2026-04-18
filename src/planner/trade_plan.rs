use super::kelly::Kelly;
use super::ote::OTE;
use super::risk::Risk;
use crate::bayesian::BayesianFusion;
use crate::indicators::atr::latest_atr;
use crate::types::{Candle, CascadeResult, Direction, Regime, RegimeProbs, Symbol, TradePlan};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ProbabilisticPlanConfig {
    pub min_decision_score: f64,
    pub min_win_probability: f64,
    pub max_kelly_fraction: f64,
}

impl Default for ProbabilisticPlanConfig {
    fn default() -> Self {
        Self {
            min_decision_score: 0.08,
            min_win_probability: 0.45,
            max_kelly_fraction: 0.20,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TradePlanInput<'a> {
    pub mtf: &'a [Candle],
    pub ltf: &'a [Candle],
    pub fvgs: &'a [crate::types::FairValueGap],
    pub obs: &'a [crate::types::OrderBlock],
    pub cascade: &'a CascadeResult,
    pub direction: Direction,
    pub atr_ltf: &'a [f64],
    pub posterior: f64,
    pub symbol: &'a str,
}

#[derive(Debug, Clone)]
pub struct ProbabilisticTradePlanInput<'a> {
    pub mtf: &'a [Candle],
    pub ltf: &'a [Candle],
    pub fvgs: &'a [crate::types::FairValueGap],
    pub obs: &'a [crate::types::OrderBlock],
    pub symbol: &'a str,
    pub regime_probs: RegimeProbs,
    pub cascade_bull: &'a CascadeResult,
    pub cascade_bear: &'a CascadeResult,
    pub bull_trade_outcome: &'a [f64],
    pub bear_trade_outcome: &'a [f64],
    pub config: &'a ProbabilisticPlanConfig,
}

#[derive(Debug, Clone)]
struct BuildTradePlanInput<'a> {
    mtf: &'a [Candle],
    ltf: &'a [Candle],
    fvgs: &'a [crate::types::FairValueGap],
    obs: &'a [crate::types::OrderBlock],
    cascade_bull: &'a CascadeResult,
    cascade_bear: &'a CascadeResult,
    direction: Direction,
    posterior: f64,
    win_probability: f64,
    symbol: &'a str,
    uncertainties: Vec<String>,
    max_kelly_fraction: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilisticDecisionSnapshot {
    pub long_score: f64,
    pub short_score: f64,
    pub win_prob_long: f64,
    pub win_prob_short: f64,
    pub ict_support_long: f64,
    pub ict_support_short: f64,
    pub selected_direction: Direction,
    pub selected_score: f64,
    pub selected_win_probability: f64,
    pub ict_role: String,
}

/// Generate trade plan
pub fn generate_trade_plan(input: TradePlanInput<'_>) -> Option<TradePlan> {
    build_trade_plan(BuildTradePlanInput {
        mtf: input.mtf,
        ltf: input.ltf,
        fvgs: input.fvgs,
        obs: input.obs,
        cascade_bull: input.cascade,
        cascade_bear: input.cascade,
        direction: input.direction,
        posterior: input.posterior,
        win_probability: input.posterior,
        symbol: input.symbol,
        uncertainties: Vec::new(),
        max_kelly_fraction: 1.0,
    })
}

pub fn generate_probabilistic_trade_plan(input: ProbabilisticTradePlanInput<'_>) -> TradePlan {
    let snapshot = probabilistic_decision_snapshot(
        &input.regime_probs,
        input.cascade_bull,
        input.cascade_bear,
        input.bull_trade_outcome,
        input.bear_trade_outcome,
    );
    let diagnostics = snapshot_diagnostics(&snapshot);

    let direction_allowed = match BayesianFusion::should_trade(
        snapshot.long_score,
        snapshot.short_score,
        input.config.min_decision_score,
    ) {
        Some(Direction::Bull) => snapshot.selected_direction == Direction::Bull,
        Some(Direction::Bear) => snapshot.selected_direction == Direction::Bear,
        Some(Direction::Neutral) | None => false,
    };

    if !direction_allowed {
        let mut uncertainties = diagnostics;
        uncertainties.push(format!(
            "decision_score_below_threshold={:.3}",
            input.config.min_decision_score
        ));
        return no_trade_with_uncertainties(
            input.symbol,
            input.cascade_bull,
            input.cascade_bear,
            input.regime_probs,
            uncertainties,
        );
    }

    if snapshot.selected_win_probability < input.config.min_win_probability {
        let mut uncertainties = diagnostics;
        uncertainties.push(format!(
            "win_probability_below_threshold={:.3}",
            input.config.min_win_probability
        ));
        return no_trade_with_uncertainties(
            input.symbol,
            input.cascade_bull,
            input.cascade_bear,
            input.regime_probs,
            uncertainties,
        );
    }

    build_trade_plan(BuildTradePlanInput {
        mtf: input.mtf,
        ltf: input.ltf,
        fvgs: input.fvgs,
        obs: input.obs,
        cascade_bull: input.cascade_bull,
        cascade_bear: input.cascade_bear,
        direction: snapshot.selected_direction,
        posterior: snapshot.selected_score,
        win_probability: snapshot.selected_win_probability,
        symbol: input.symbol,
        uncertainties: diagnostics,
        max_kelly_fraction: input.config.max_kelly_fraction,
    })
    .unwrap_or_else(|| {
        no_trade_with_uncertainties(
            input.symbol,
            input.cascade_bull,
            input.cascade_bear,
            input.regime_probs,
            vec!["No valid OTE zone from current FVG/OB structures".to_string()],
        )
    })
}

pub fn probabilistic_decision_snapshot(
    regime_probs: &RegimeProbs,
    cascade_bull: &CascadeResult,
    cascade_bear: &CascadeResult,
    bull_trade_outcome: &[f64],
    bear_trade_outcome: &[f64],
) -> ProbabilisticDecisionSnapshot {
    let win_prob_long = effective_win_probability(bull_trade_outcome);
    let win_prob_short = effective_win_probability(bear_trade_outcome);
    let long_score = BayesianFusion::fuse_trade_probability(regime_probs, win_prob_long);
    let short_score = BayesianFusion::fuse_trade_probability(regime_probs, win_prob_short);

    let (selected_direction, selected_score, selected_win_probability) =
        if long_score >= short_score {
            (Direction::Bull, long_score, win_prob_long)
        } else {
            (Direction::Bear, short_score, win_prob_short)
        };

    ProbabilisticDecisionSnapshot {
        long_score,
        short_score,
        win_prob_long,
        win_prob_short,
        ict_support_long: cascade_bull.final_posterior,
        ict_support_short: cascade_bear.final_posterior,
        selected_direction,
        selected_score,
        selected_win_probability,
        ict_role: "evidence_only_non_deterministic".to_string(),
    }
}

fn build_trade_plan(input: BuildTradePlanInput<'_>) -> Option<TradePlan> {
    let BuildTradePlanInput {
        mtf,
        ltf,
        fvgs,
        obs,
        cascade_bull,
        cascade_bear,
        direction,
        posterior,
        win_probability,
        symbol,
        uncertainties,
        max_kelly_fraction,
    } = input;
    let atr = latest_atr(ltf, 14);
    if atr <= 0.0 {
        return None;
    }

    // Find OTE zone
    let ote = OTE::find_ote(mtf, fvgs, obs, direction)?;
    let entry = OTE::optimal_entry(ote.0, ote.1);

    // Calculate stop loss
    let stop_loss = Risk::atr_stop_loss(entry, atr, 1.5, direction);

    // Calculate take profits
    let (tp1, tp2, tp3) = Risk::atr_take_profits(entry, atr, direction);

    // Calculate risk-reward
    let risk_reward = Risk::risk_reward(entry, stop_loss, tp1);

    // Kelly fraction
    let win_prob = win_probability.min(0.95);
    let kelly_fraction = Kelly::safe_fraction(win_prob, risk_reward).min(max_kelly_fraction);

    // Parse symbol
    let sym = match symbol.to_uppercase().as_str() {
        "NQ" => Symbol::NQ,
        "ES" => Symbol::ES,
        "YM" => Symbol::YM,
        "GC" => Symbol::GC,
        "CL" => Symbol::CL,
        _ => Symbol::NQ,
    };

    Some(TradePlan {
        symbol: sym,
        direction,
        entry,
        stop_loss,
        tp1,
        tp2,
        tp3,
        risk_reward,
        kelly_fraction,
        position_size: kelly_fraction * 100.0, // As percentage
        regime: Regime::ManipulationExpansion,
        posterior,
        win_probability,
        cascade_bull: cascade_bull.clone(),
        cascade_bear: cascade_bear.clone(),
        uncertainties,
    })
}

/// Generate no-trade plan
pub fn no_trade(
    symbol: &str,
    cascade_bull: &CascadeResult,
    cascade_bear: &CascadeResult,
    regime_probs: RegimeProbs,
) -> TradePlan {
    no_trade_with_uncertainties(
        symbol,
        cascade_bull,
        cascade_bear,
        regime_probs,
        vec!["No trade signal".to_string()],
    )
}

fn no_trade_with_uncertainties(
    symbol: &str,
    cascade_bull: &CascadeResult,
    cascade_bear: &CascadeResult,
    regime_probs: RegimeProbs,
    uncertainties: Vec<String>,
) -> TradePlan {
    let sym = match symbol.to_uppercase().as_str() {
        "NQ" => Symbol::NQ,
        "ES" => Symbol::ES,
        "YM" => Symbol::YM,
        "GC" => Symbol::GC,
        "CL" => Symbol::CL,
        _ => Symbol::NQ,
    };
    let mut uncertainties = uncertainties;
    if !uncertainties
        .iter()
        .any(|item| item == "ict_role=evidence_only_non_deterministic")
    {
        uncertainties.push("ict_role=evidence_only_non_deterministic".to_string());
    }

    TradePlan {
        symbol: sym,
        direction: Direction::Neutral,
        entry: 0.0,
        stop_loss: 0.0,
        tp1: 0.0,
        tp2: 0.0,
        tp3: 0.0,
        risk_reward: 0.0,
        kelly_fraction: 0.0,
        position_size: 0.0,
        regime: regime_probs.dominant(),
        posterior: 0.0,
        win_probability: 0.0,
        cascade_bull: cascade_bull.clone(),
        cascade_bear: cascade_bear.clone(),
        uncertainties,
    }
}

fn effective_win_probability(trade_outcome: &[f64]) -> f64 {
    match trade_outcome {
        [win, breakeven, ..] => (win + 0.5 * breakeven).clamp(0.0, 0.999),
        [win] => (*win).clamp(0.0, 0.999),
        _ => 0.0,
    }
}

fn snapshot_diagnostics(snapshot: &ProbabilisticDecisionSnapshot) -> Vec<String> {
    vec![
        format!("ict_role={}", snapshot.ict_role),
        format!("bull_score={:.3}", snapshot.long_score),
        format!("bear_score={:.3}", snapshot.short_score),
        format!("bull_ict_support={:.3}", snapshot.ict_support_long),
        format!("bear_ict_support={:.3}", snapshot.ict_support_short),
        format!("bull_win_probability={:.3}", snapshot.win_prob_long),
        format!("bear_win_probability={:.3}", snapshot.win_prob_short),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CascadeLayer, CascadeStep, FairValueGap};
    use chrono::Utc;

    fn sample_candles() -> Vec<Candle> {
        (0..30)
            .map(|i| Candle {
                timestamp: Utc::now(),
                open: 100.0 + i as f64,
                high: 101.0 + i as f64,
                low: 99.5 + i as f64,
                close: 100.5 + i as f64,
                volume: 1_000.0 + i as f64,
            })
            .collect()
    }

    fn cascade(direction: Direction, posterior: f64) -> CascadeResult {
        CascadeResult {
            direction,
            stopped_at: None,
            steps: vec![CascadeStep {
                layer: CascadeLayer::L1,
                satisfied: true,
                lr: 2.0,
                prior: 0.2,
                posterior,
                description: "test".to_string(),
            }],
            final_posterior: posterior,
        }
    }

    #[test]
    fn test_generate_probabilistic_trade_plan_selects_bull() {
        let candles = sample_candles();
        let fvgs = vec![FairValueGap {
            top: 120.0,
            bottom: 118.0,
            direction: Direction::Bull,
            start_bar: 10,
            filled: false,
        }];
        let plan = generate_probabilistic_trade_plan(ProbabilisticTradePlanInput {
            mtf: &candles,
            ltf: &candles,
            fvgs: &fvgs,
            obs: &[],
            symbol: "NQ",
            regime_probs: RegimeProbs {
                accumulation: 0.2,
                manipulation_expansion: 0.6,
                distribution: 0.2,
            },
            cascade_bull: &cascade(Direction::Bull, 0.8),
            cascade_bear: &cascade(Direction::Bear, 0.2),
            bull_trade_outcome: &[0.65, 0.20, 0.15],
            bear_trade_outcome: &[0.20, 0.20, 0.60],
            config: &ProbabilisticPlanConfig::default(),
        });

        assert_eq!(plan.direction, Direction::Bull);
        assert!(plan.posterior > 0.0);
        assert!(plan.win_probability > 0.0);
    }

    #[test]
    fn test_probabilistic_decision_snapshot_exposes_agent_scores() {
        let snapshot = probabilistic_decision_snapshot(
            &RegimeProbs {
                accumulation: 0.2,
                manipulation_expansion: 0.6,
                distribution: 0.2,
            },
            &cascade(Direction::Bull, 0.8),
            &cascade(Direction::Bear, 0.3),
            &[0.65, 0.20, 0.15],
            &[0.20, 0.20, 0.60],
        );

        assert!(snapshot.long_score > snapshot.short_score);
        assert!(snapshot.win_prob_long > snapshot.win_prob_short);
        assert_eq!(snapshot.selected_direction, Direction::Bull);
        assert_eq!(snapshot.ict_role, "evidence_only_non_deterministic");
    }

    #[test]
    fn test_generate_probabilistic_trade_plan_can_return_no_trade() {
        let candles = sample_candles();
        let plan = generate_probabilistic_trade_plan(ProbabilisticTradePlanInput {
            mtf: &candles,
            ltf: &candles,
            fvgs: &[],
            obs: &[],
            symbol: "NQ",
            regime_probs: RegimeProbs {
                accumulation: 0.4,
                manipulation_expansion: 0.2,
                distribution: 0.4,
            },
            cascade_bull: &cascade(Direction::Bull, 0.2),
            cascade_bear: &cascade(Direction::Bear, 0.2),
            bull_trade_outcome: &[0.30, 0.20, 0.50],
            bear_trade_outcome: &[0.30, 0.20, 0.50],
            config: &ProbabilisticPlanConfig::default(),
        });

        assert_eq!(plan.direction, Direction::Neutral);
        assert!(plan
            .uncertainties
            .iter()
            .any(|item| item.contains("threshold")));
    }
}
