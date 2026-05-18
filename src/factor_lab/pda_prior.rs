use serde::{Deserialize, Serialize};

use crate::data::RegimeSegmentationOutput;
use crate::ict::{
    detect_cisd, detect_liquidity_pools, detect_liquidity_sweep, detect_order_blocks,
};
use crate::types::{Candle, Direction};

pub fn expansion_leg_from_alignment(
    multi_timeframe_summary: &[String],
    anchor_direction: Direction,
) -> ExpansionLeg {
    let source_mode = multi_timeframe_summary
        .iter()
        .find_map(|item| item.strip_prefix("multi_timeframe_source="))
        .unwrap_or_default();
    let alignment = multi_timeframe_summary
        .iter()
        .find_map(|item| item.strip_prefix("higher_timeframe_alignment_score="))
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.5);
    let entry_alignment = multi_timeframe_summary
        .iter()
        .find_map(|item| item.strip_prefix("lower_timeframe_entry_alignment_score="))
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.5);

    if source_mode.contains("auto") && alignment >= 0.6 && entry_alignment >= 0.6 {
        ExpansionLeg::Reacceleration
    } else if matches!(anchor_direction, Direction::Bull | Direction::Bear)
        && (alignment >= 0.5 || entry_alignment >= 0.5)
    {
        ExpansionLeg::Pullback
    } else {
        ExpansionLeg::Unknown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketRegimeEnvelope {
    Expansion,
    Pullback,
    ReversalAttempt,
    Consolidation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketRegimeClass {
    Continuation,
    CountertrendPullback,
    Reversal,
    Consolidation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpansionLeg {
    Impulse,
    Pullback,
    Reacceleration,
    Terminal,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FootprintNodeKind {
    LiquiditySweep,
    DiscountZone,
    PremiumZone,
    RejectionBlock,
    FairValueGap,
    OrderBlock,
    Cisd,
    MarketStructureShift,
    RangeLiquidityTarget,
    HigherTimeframePda,
    DeliveryFailure,
    FollowThrough,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FootprintPhase {
    LeftContext,
    Anchor,
    RightContext,
    Confirmation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReversalHypothesis {
    BullishExpansionContinuation,
    BearishExpansionContinuation,
    BullishReversal,
    BearishReversal,
    NoExpansion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FootprintChainRegime {
    BullExpansionSecondLeg,
    BullExpansionToBearExpansion,
    BearExpansionToBullExpansion,
    BearExpansionSecondLeg,
    FailedBullExpansion,
    FailedBearExpansion,
    RangeLiquidityReversion,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FootprintNode {
    pub kind: FootprintNodeKind,
    pub phase: FootprintPhase,
    pub direction: Direction,
    pub timeframe: String,
    pub bars_from_anchor: i32,
    pub price_reference: Option<f64>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FootprintChain {
    pub regime_envelope: MarketRegimeEnvelope,
    pub regime: FootprintChainRegime,
    pub hypothesis: ReversalHypothesis,
    pub anchor_direction: Direction,
    pub regime_stack: Vec<RegimeNodeDescriptor>,
    pub nodes: Vec<FootprintNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeNodeDescriptor {
    pub timeframe_minutes: i64,
    pub state: String,
    pub dominant_share: f64,
    pub leg_bias: ExpansionLeg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorFootprintAdjustment {
    pub base_prior: f64,
    pub regime_bias_delta: f64,
    pub chain_bias_delta: f64,
    pub confirmation_delta: f64,
    pub adjusted_prior: f64,
    pub rationale: Vec<String>,
}

pub fn regime_envelope_for_class(class: MarketRegimeClass) -> MarketRegimeEnvelope {
    match class {
        MarketRegimeClass::Continuation => MarketRegimeEnvelope::Expansion,
        MarketRegimeClass::CountertrendPullback => MarketRegimeEnvelope::Pullback,
        MarketRegimeClass::Reversal => MarketRegimeEnvelope::ReversalAttempt,
        MarketRegimeClass::Consolidation => MarketRegimeEnvelope::Consolidation,
    }
}

pub fn build_regime_stack(
    frames: &[RegimeSegmentationOutput],
    bull_leg: ExpansionLeg,
    bear_leg: ExpansionLeg,
) -> (Vec<RegimeNodeDescriptor>, Vec<RegimeNodeDescriptor>) {
    let mut bull = Vec::with_capacity(frames.len());
    let mut bear = Vec::with_capacity(frames.len());
    for frame in frames {
        let state = format!("{:?}", frame.latest_state);
        let dominant_share = frame
            .bullish_share
            .max(frame.bearish_share)
            .max(frame.consolidation_share);
        bull.push(RegimeNodeDescriptor {
            timeframe_minutes: frame.timeframe_minutes,
            state: state.clone(),
            dominant_share,
            leg_bias: bull_leg,
        });
        bear.push(RegimeNodeDescriptor {
            timeframe_minutes: frame.timeframe_minutes,
            state,
            dominant_share,
            leg_bias: bear_leg,
        });
    }
    (bull, bear)
}

pub fn classify_footprint_chain_regime(
    anchor_direction: Direction,
    left_regime: MarketRegimeClass,
    left_leg: ExpansionLeg,
    nodes: &[FootprintNode],
) -> FootprintChainRegime {
    let has_discount = nodes
        .iter()
        .any(|node| node.kind == FootprintNodeKind::DiscountZone);
    let has_premium = nodes
        .iter()
        .any(|node| node.kind == FootprintNodeKind::PremiumZone);
    let has_cisd = nodes
        .iter()
        .any(|node| node.kind == FootprintNodeKind::Cisd);
    let has_fvg = nodes
        .iter()
        .any(|node| node.kind == FootprintNodeKind::FairValueGap);
    let has_follow_through = nodes
        .iter()
        .any(|node| node.kind == FootprintNodeKind::FollowThrough);
    let has_failure = nodes
        .iter()
        .any(|node| node.kind == FootprintNodeKind::DeliveryFailure);

    match (anchor_direction, left_regime, left_leg) {
        (Direction::Bull, MarketRegimeClass::Continuation, ExpansionLeg::Reacceleration)
            if has_discount && has_cisd && has_fvg =>
        {
            FootprintChainRegime::BullExpansionSecondLeg
        }
        (Direction::Bear, MarketRegimeClass::Continuation, ExpansionLeg::Terminal)
            if has_premium && has_cisd && has_follow_through =>
        {
            FootprintChainRegime::BullExpansionToBearExpansion
        }
        (Direction::Bull, MarketRegimeClass::Reversal, _)
            if has_discount && has_cisd && has_fvg =>
        {
            FootprintChainRegime::BearExpansionToBullExpansion
        }
        (Direction::Bear, MarketRegimeClass::Continuation, ExpansionLeg::Reacceleration)
            if has_premium && has_cisd && has_fvg =>
        {
            FootprintChainRegime::BearExpansionSecondLeg
        }
        (Direction::Bull, _, _) if has_failure => FootprintChainRegime::FailedBullExpansion,
        (Direction::Bear, _, _) if has_failure => FootprintChainRegime::FailedBearExpansion,
        (_, MarketRegimeClass::Consolidation, _) => FootprintChainRegime::RangeLiquidityReversion,
        _ => FootprintChainRegime::Unknown,
    }
}

pub fn infer_reversal_hypothesis(regime: FootprintChainRegime) -> ReversalHypothesis {
    match regime {
        FootprintChainRegime::BullExpansionSecondLeg => {
            ReversalHypothesis::BullishExpansionContinuation
        }
        FootprintChainRegime::BullExpansionToBearExpansion => ReversalHypothesis::BearishReversal,
        FootprintChainRegime::BearExpansionToBullExpansion => ReversalHypothesis::BullishReversal,
        FootprintChainRegime::BearExpansionSecondLeg => {
            ReversalHypothesis::BearishExpansionContinuation
        }
        FootprintChainRegime::FailedBullExpansion | FootprintChainRegime::FailedBearExpansion => {
            ReversalHypothesis::NoExpansion
        }
        FootprintChainRegime::RangeLiquidityReversion | FootprintChainRegime::Unknown => {
            ReversalHypothesis::NoExpansion
        }
    }
}

pub fn build_footprint_chain(
    candles: &[Candle],
    anchor_direction: Direction,
    left_regime: MarketRegimeClass,
    left_leg: ExpansionLeg,
    regime_stack: Vec<RegimeNodeDescriptor>,
) -> FootprintChain {
    let order_blocks = detect_order_blocks(candles);
    let cisds = detect_cisd(candles, &order_blocks, 1);
    let pools = detect_liquidity_pools(candles, &vec![1.0; candles.len()], 1.5, 2);
    let sweeps = detect_liquidity_sweep(candles, &pools, 5);
    let last_index = candles.len().saturating_sub(1);
    let anchor_bar = sweeps
        .iter()
        .rev()
        .find(|sweep| sweep.sweep_direction == anchor_direction)
        .map(|sweep| sweep.return_bar)
        .unwrap_or(last_index);

    let mut nodes = Vec::new();
    if let Some(stack_head) = regime_stack.first() {
        nodes.push(FootprintNode {
            kind: FootprintNodeKind::HigherTimeframePda,
            phase: FootprintPhase::LeftContext,
            direction: anchor_direction,
            timeframe: format!("{}m", stack_head.timeframe_minutes),
            bars_from_anchor: 0,
            price_reference: None,
            note: format!(
                "regime_stack={} share={:.3}",
                stack_head.state, stack_head.dominant_share
            ),
        });
    }
    if let Some(sweep) = sweeps
        .iter()
        .rev()
        .find(|sweep| sweep.sweep_direction == anchor_direction)
    {
        nodes.push(FootprintNode {
            kind: FootprintNodeKind::LiquiditySweep,
            phase: FootprintPhase::Anchor,
            direction: sweep.sweep_direction,
            timeframe: "ltf".to_string(),
            bars_from_anchor: 0,
            price_reference: Some(sweep.pool_price),
            note: "anchor_liquidity_sweep".to_string(),
        });
    }

    if cisds.iter().any(|cisd| cisd.direction == anchor_direction) {
        if let Some(cisd) = cisds
            .iter()
            .rev()
            .find(|cisd| cisd.direction == anchor_direction)
        {
            nodes.push(FootprintNode {
                kind: FootprintNodeKind::Cisd,
                phase: if cisd.confirm_bar >= anchor_bar {
                    FootprintPhase::RightContext
                } else {
                    FootprintPhase::LeftContext
                },
                direction: cisd.direction,
                timeframe: "ltf".to_string(),
                bars_from_anchor: cisd.confirm_bar as i32 - anchor_bar as i32,
                price_reference: candles.get(cisd.confirm_bar).map(|candle| candle.close),
                note: format!("cisd_strength={}", cisd.strength),
            });
        }
    }

    if let Some(ob) = order_blocks.last() {
        nodes.push(FootprintNode {
            kind: FootprintNodeKind::OrderBlock,
            phase: if ob.bar_index >= anchor_bar {
                FootprintPhase::RightContext
            } else {
                FootprintPhase::LeftContext
            },
            direction: ob.ob_type,
            timeframe: "ltf".to_string(),
            bars_from_anchor: ob.bar_index as i32 - anchor_bar as i32,
            price_reference: Some((ob.high + ob.low) * 0.5),
            note: if ob.tested {
                "order_block_tested=true".to_string()
            } else {
                "order_block_tested=false".to_string()
            },
        });
    }

    if let Some(last_candle) = candles.last() {
        nodes.push(FootprintNode {
            kind: match anchor_direction {
                Direction::Bull => FootprintNodeKind::DiscountZone,
                Direction::Bear => FootprintNodeKind::PremiumZone,
                Direction::Neutral => FootprintNodeKind::RangeLiquidityTarget,
            },
            phase: FootprintPhase::LeftContext,
            direction: anchor_direction,
            timeframe: "htf".to_string(),
            bars_from_anchor: 0,
            price_reference: Some(last_candle.close),
            note: "range_location_proxy".to_string(),
        });
    }

    if let Some(last_cisd) = cisds
        .iter()
        .rev()
        .find(|cisd| cisd.direction == anchor_direction)
    {
        if last_cisd.confirm_bar + 3 <= last_index {
            let follow_close = candles[last_index].close;
            let confirm_close = candles[last_cisd.confirm_bar].close;
            let supportive = match anchor_direction {
                Direction::Bull => follow_close >= confirm_close,
                Direction::Bear => follow_close <= confirm_close,
                Direction::Neutral => false,
            };
            nodes.push(FootprintNode {
                kind: if supportive {
                    FootprintNodeKind::FollowThrough
                } else {
                    FootprintNodeKind::DeliveryFailure
                },
                phase: FootprintPhase::Confirmation,
                direction: anchor_direction,
                timeframe: "ltf".to_string(),
                bars_from_anchor: last_index as i32 - anchor_bar as i32,
                price_reference: Some(follow_close),
                note: "post_cisd_follow_through_check".to_string(),
            });
        }
    }

    let regime = classify_footprint_chain_regime(anchor_direction, left_regime, left_leg, &nodes);
    let hypothesis = infer_reversal_hypothesis(regime);
    FootprintChain {
        regime_envelope: regime_envelope_for_class(left_regime),
        regime,
        hypothesis,
        anchor_direction,
        regime_stack,
        nodes,
    }
}

pub fn compute_prior_footprint_adjustment(
    base_prior: f64,
    chain: &FootprintChain,
) -> PriorFootprintAdjustment {
    let mut rationale = Vec::new();
    let stack_alignment = chain
        .regime_stack
        .iter()
        .map(|node| node.dominant_share)
        .sum::<f64>()
        / chain.regime_stack.len().max(1) as f64;
    let regime_bias_delta = match chain.regime {
        FootprintChainRegime::BullExpansionSecondLeg
        | FootprintChainRegime::BearExpansionSecondLeg => 0.08 + stack_alignment * 0.08,
        FootprintChainRegime::BullExpansionToBearExpansion
        | FootprintChainRegime::BearExpansionToBullExpansion => 0.10 + stack_alignment * 0.10,
        FootprintChainRegime::RangeLiquidityReversion => 0.03 + stack_alignment * 0.04,
        FootprintChainRegime::FailedBullExpansion | FootprintChainRegime::FailedBearExpansion => {
            -0.10 - stack_alignment * 0.06
        }
        FootprintChainRegime::Unknown => 0.0,
    };
    if regime_bias_delta != 0.0 {
        rationale.push(format!("regime={:?}", chain.regime));
    }
    if !chain.regime_stack.is_empty() {
        rationale.push(format!("regime_stack_depth={}", chain.regime_stack.len()));
    }

    let chain_bias_delta = chain
        .nodes
        .iter()
        .map(|node| match node.kind {
            FootprintNodeKind::LiquiditySweep => 0.02,
            FootprintNodeKind::RejectionBlock => 0.02,
            FootprintNodeKind::Cisd => 0.04,
            FootprintNodeKind::FairValueGap => 0.03,
            FootprintNodeKind::OrderBlock => 0.015,
            FootprintNodeKind::MarketStructureShift => 0.03,
            FootprintNodeKind::DiscountZone | FootprintNodeKind::PremiumZone => 0.015,
            FootprintNodeKind::FollowThrough => 0.05,
            FootprintNodeKind::DeliveryFailure => -0.08,
            FootprintNodeKind::RangeLiquidityTarget => 0.01,
            FootprintNodeKind::HigherTimeframePda => 0.01,
        })
        .sum::<f64>()
        .clamp(-0.16, 0.16);
    rationale.push(format!("node_count={}", chain.nodes.len()));

    let confirmation_delta = if chain
        .nodes
        .iter()
        .any(|node| node.phase == FootprintPhase::Confirmation)
    {
        rationale.push("confirmation_node_present=true".to_string());
        0.04
    } else {
        0.0
    };

    let adjusted_prior =
        (base_prior + regime_bias_delta + chain_bias_delta + confirmation_delta).clamp(0.01, 0.99);

    PriorFootprintAdjustment {
        base_prior,
        regime_bias_delta,
        chain_bias_delta,
        confirmation_delta,
        adjusted_prior,
        rationale,
    }
}
