use crate::types::{
    Candle, Direction, FairValueGap, LiquidityPool, PdaConceptKind, PdaInvalidationRule,
    PdaInverseMode, PdaLifecycleState, PdaStateTransition, PriceLevelBand, TimedPdaState,
};

const DEFAULT_VALIDITY_BARS: usize = 64;
const TOUCH_EPSILON_BPS: f64 = 0.0005;

fn band_mid(band: &PriceLevelBand) -> f64 {
    (band.top + band.bottom) / 2.0
}

fn band_height(band: &PriceLevelBand) -> f64 {
    (band.top - band.bottom).abs().max(f64::EPSILON)
}

fn overlaps_candle(candle: &Candle, band: &PriceLevelBand) -> bool {
    candle.high >= band.bottom && candle.low <= band.top
}

fn mitigation_progress(candle: &Candle, band: &PriceLevelBand) -> f64 {
    let overlap_top = candle.high.min(band.top);
    let overlap_bottom = candle.low.max(band.bottom);
    let overlap = (overlap_top - overlap_bottom).max(0.0);
    (overlap / band_height(band)).clamp(0.0, 1.0)
}

fn close_through(candle: &Candle, direction: Direction, band: &PriceLevelBand) -> bool {
    match direction {
        Direction::Bull => candle.close < band.bottom,
        Direction::Bear => candle.close > band.top,
        Direction::Neutral => false,
    }
}

fn full_fill(candle: &Candle, direction: Direction, band: &PriceLevelBand) -> bool {
    match direction {
        Direction::Bull => candle.low <= band.bottom,
        Direction::Bear => candle.high >= band.top,
        Direction::Neutral => false,
    }
}

fn touch_band(candle: &Candle, band: &PriceLevelBand) -> bool {
    let eps = band_mid(band).abs() * TOUCH_EPSILON_BPS;
    let inside_band = candle.high >= band.bottom && candle.low <= band.top;
    let near_top = (candle.low - band.top).abs() <= eps;
    let near_bottom = (candle.high - band.bottom).abs() <= eps;
    inside_band || near_top || near_bottom
}

fn push_transition(
    state: &mut TimedPdaState,
    next: PdaLifecycleState,
    at_bar: usize,
    note: impl Into<String>,
) {
    if state.state == next {
        return;
    }
    state.state = next;
    state.last_updated_bar = at_bar;
    state.transitions.push(PdaStateTransition {
        state: next,
        at_bar,
        note: note.into(),
    });
}

#[derive(Debug, Clone, Copy)]
struct PdaRuleSpec {
    invalidation_rule: PdaInvalidationRule,
    inverse_mode: PdaInverseMode,
    validity_bars: usize,
}

fn pda_rule_spec(concept: PdaConceptKind) -> PdaRuleSpec {
    match concept {
        PdaConceptKind::FairValueGap => PdaRuleSpec {
            invalidation_rule: PdaInvalidationRule::FullFill,
            inverse_mode: PdaInverseMode::FlipNeedsConfirmation,
            validity_bars: DEFAULT_VALIDITY_BARS,
        },
        PdaConceptKind::InversionFairValueGap => PdaRuleSpec {
            invalidation_rule: PdaInvalidationRule::CloseThrough,
            inverse_mode: PdaInverseMode::FlipSameBand,
            validity_bars: DEFAULT_VALIDITY_BARS / 2,
        },
        PdaConceptKind::BalancedPriceRange => PdaRuleSpec {
            invalidation_rule: PdaInvalidationRule::BodyAcceptance,
            inverse_mode: PdaInverseMode::FlipNeedsConfirmation,
            validity_bars: DEFAULT_VALIDITY_BARS,
        },
        PdaConceptKind::LiquidityPool | PdaConceptKind::EqualHighsLows => PdaRuleSpec {
            invalidation_rule: PdaInvalidationRule::WickThrough,
            inverse_mode: PdaInverseMode::FlipNeedsConfirmation,
            validity_bars: DEFAULT_VALIDITY_BARS,
        },
        PdaConceptKind::OptimalTradeEntry => PdaRuleSpec {
            invalidation_rule: PdaInvalidationRule::CloseThrough,
            inverse_mode: PdaInverseMode::None,
            validity_bars: DEFAULT_VALIDITY_BARS / 2,
        },
        PdaConceptKind::Ndog | PdaConceptKind::Nwog | PdaConceptKind::OpenRangeGap => PdaRuleSpec {
            invalidation_rule: PdaInvalidationRule::CloseThrough,
            inverse_mode: PdaInverseMode::FlipSameBand,
            validity_bars: DEFAULT_VALIDITY_BARS,
        },
        PdaConceptKind::SwingFailurePattern => PdaRuleSpec {
            invalidation_rule: PdaInvalidationRule::StructureBreak,
            inverse_mode: PdaInverseMode::FlipSameBand,
            validity_bars: DEFAULT_VALIDITY_BARS / 2,
        },
    }
}

pub fn initial_timed_pda_state(
    concept: PdaConceptKind,
    direction: Direction,
    band: PriceLevelBand,
    anchor_bar: usize,
    invalidation_rule: PdaInvalidationRule,
    inverse_mode: PdaInverseMode,
    validity_bars: usize,
) -> TimedPdaState {
    TimedPdaState {
        concept,
        direction,
        band,
        anchor_bar,
        last_updated_bar: anchor_bar,
        state: PdaLifecycleState::Active,
        invalidation_rule,
        inverse_mode,
        validity_bars,
        touch_count: 0,
        mitigation_progress: 0.0,
        inverse_confirmed: false,
        transitions: vec![PdaStateTransition {
            state: PdaLifecycleState::Active,
            at_bar: anchor_bar,
            note: "initialized".to_string(),
        }],
    }
}

pub fn update_timed_pda_state(state: &mut TimedPdaState, candles: &[Candle]) {
    if candles.is_empty() || state.last_updated_bar >= candles.len().saturating_sub(1) {
        return;
    }

    for (idx, candle) in candles.iter().enumerate().skip(state.last_updated_bar + 1) {
        if idx.saturating_sub(state.anchor_bar) > state.validity_bars {
            push_transition(
                state,
                PdaLifecycleState::Expired,
                idx,
                "validity window elapsed",
            );
            break;
        }

        if touch_band(candle, &state.band) {
            state.touch_count += 1;
            if matches!(state.state, PdaLifecycleState::Active) {
                push_transition(
                    state,
                    PdaLifecycleState::Touched,
                    idx,
                    "price interacted with band",
                );
            }
        }

        let progress = mitigation_progress(candle, &state.band);
        if progress > state.mitigation_progress {
            state.mitigation_progress = progress;
        }
        if progress >= 0.5
            && matches!(
                state.state,
                PdaLifecycleState::Active | PdaLifecycleState::Touched
            )
        {
            push_transition(
                state,
                PdaLifecycleState::Mitigated,
                idx,
                format!("mitigation_progress={progress:.3}"),
            );
        }

        let invalidated = match state.invalidation_rule {
            PdaInvalidationRule::WickThrough => overlaps_candle(candle, &state.band),
            PdaInvalidationRule::CloseThrough => {
                close_through(candle, state.direction, &state.band)
            }
            PdaInvalidationRule::BodyAcceptance => {
                close_through(candle, state.direction, &state.band)
            }
            PdaInvalidationRule::FullFill => full_fill(candle, state.direction, &state.band),
            PdaInvalidationRule::StructureBreak => {
                close_through(candle, state.direction, &state.band)
            }
            PdaInvalidationRule::TimeExpiry => {
                idx.saturating_sub(state.anchor_bar) > state.validity_bars
            }
        };

        if invalidated {
            push_transition(
                state,
                PdaLifecycleState::Invalidated,
                idx,
                "invalidation rule triggered",
            );
            match state.inverse_mode {
                PdaInverseMode::None => {}
                PdaInverseMode::FlipSameBand => {
                    state.direction = match state.direction {
                        Direction::Bull => Direction::Bear,
                        Direction::Bear => Direction::Bull,
                        Direction::Neutral => Direction::Neutral,
                    };
                    state.inverse_confirmed = true;
                    push_transition(
                        state,
                        PdaLifecycleState::Inversed,
                        idx,
                        "auto inverse on invalidation",
                    );
                }
                PdaInverseMode::FlipNeedsConfirmation => {
                    state.direction = match state.direction {
                        Direction::Bull => Direction::Bear,
                        Direction::Bear => Direction::Bull,
                        Direction::Neutral => Direction::Neutral,
                    };
                    state.inverse_confirmed = false;
                    push_transition(
                        state,
                        PdaLifecycleState::Inversed,
                        idx,
                        "inverse pending confirmation",
                    );
                }
            }
            break;
        }

        state.last_updated_bar = idx;
    }
}

pub fn timed_state_from_fvg(fvg: &FairValueGap, candles: &[Candle]) -> TimedPdaState {
    let spec = pda_rule_spec(PdaConceptKind::FairValueGap);
    let mut state = initial_timed_pda_state(
        PdaConceptKind::FairValueGap,
        fvg.direction,
        PriceLevelBand {
            top: fvg.top,
            bottom: fvg.bottom,
        },
        fvg.start_bar,
        spec.invalidation_rule,
        spec.inverse_mode,
        spec.validity_bars,
    );
    update_timed_pda_state(&mut state, candles);
    state
}

pub fn build_timed_states_from_fvgs(
    candles: &[Candle],
    fvgs: &[FairValueGap],
) -> Vec<TimedPdaState> {
    fvgs.iter()
        .map(|fvg| timed_state_from_fvg(fvg, candles))
        .collect()
}

pub fn timed_state_from_liquidity_pool(
    pool: &LiquidityPool,
    candles: &[Candle],
    anchor_bar: usize,
) -> TimedPdaState {
    let band = PriceLevelBand {
        top: pool.price_level,
        bottom: pool.price_level,
    };
    let spec = pda_rule_spec(PdaConceptKind::LiquidityPool);
    let mut state = initial_timed_pda_state(
        PdaConceptKind::LiquidityPool,
        pool.pool_type,
        band,
        anchor_bar,
        spec.invalidation_rule,
        spec.inverse_mode,
        spec.validity_bars,
    );
    update_timed_pda_state(&mut state, candles);
    state
}

pub fn timed_state_from_equal_hl(
    price_level: f64,
    direction: Direction,
    candles: &[Candle],
    anchor_bar: usize,
) -> TimedPdaState {
    let spec = pda_rule_spec(PdaConceptKind::EqualHighsLows);
    let mut state = initial_timed_pda_state(
        PdaConceptKind::EqualHighsLows,
        direction,
        PriceLevelBand {
            top: price_level,
            bottom: price_level,
        },
        anchor_bar,
        spec.invalidation_rule,
        spec.inverse_mode,
        spec.validity_bars,
    );
    update_timed_pda_state(&mut state, candles);
    state
}

pub fn timed_state_from_ote(
    low: f64,
    high: f64,
    direction: Direction,
    candles: &[Candle],
    anchor_bar: usize,
) -> TimedPdaState {
    let spec = pda_rule_spec(PdaConceptKind::OptimalTradeEntry);
    let mut state = initial_timed_pda_state(
        PdaConceptKind::OptimalTradeEntry,
        direction,
        PriceLevelBand {
            top: high,
            bottom: low,
        },
        anchor_bar,
        spec.invalidation_rule,
        spec.inverse_mode,
        spec.validity_bars,
    );
    update_timed_pda_state(&mut state, candles);
    state
}

pub fn timed_state_from_named_band(
    concept: PdaConceptKind,
    band: PriceLevelBand,
    direction: Direction,
    candles: &[Candle],
    anchor_bar: usize,
) -> TimedPdaState {
    let spec = pda_rule_spec(concept);
    let mut state = initial_timed_pda_state(
        concept,
        direction,
        band,
        anchor_bar,
        spec.invalidation_rule,
        spec.inverse_mode,
        spec.validity_bars,
    );
    update_timed_pda_state(&mut state, candles);
    state
}

pub fn build_core_timed_pda_states(
    candles: &[Candle],
    fvgs: &[FairValueGap],
    pools: &[LiquidityPool],
    ote: Option<(f64, f64, Direction)>,
) -> Vec<TimedPdaState> {
    let mut states = build_timed_states_from_fvgs(candles, fvgs);
    let anchor_bar = candles.len().saturating_sub(1);

    states.extend(
        pools
            .iter()
            .map(|pool| timed_state_from_liquidity_pool(pool, candles, anchor_bar)),
    );

    let bull_eq = pools
        .iter()
        .filter(|pool| pool.pool_type == Direction::Bull)
        .max_by_key(|pool| pool.sp_count)
        .map(|pool| {
            timed_state_from_equal_hl(pool.price_level, Direction::Bull, candles, anchor_bar)
        });
    let bear_eq = pools
        .iter()
        .filter(|pool| pool.pool_type == Direction::Bear)
        .max_by_key(|pool| pool.sp_count)
        .map(|pool| {
            timed_state_from_equal_hl(pool.price_level, Direction::Bear, candles, anchor_bar)
        });
    if let Some(state) = bull_eq {
        states.push(state);
    }
    if let Some(state) = bear_eq {
        states.push(state);
    }

    if let Some((low, high, direction)) = ote {
        states.push(timed_state_from_ote(
            low, high, direction, candles, anchor_bar,
        ));
    }

    if let Some(first) = fvgs.first() {
        states.push(timed_state_from_named_band(
            PdaConceptKind::InversionFairValueGap,
            PriceLevelBand {
                top: first.top,
                bottom: first.bottom,
            },
            match first.direction {
                Direction::Bull => Direction::Bear,
                Direction::Bear => Direction::Bull,
                Direction::Neutral => Direction::Neutral,
            },
            candles,
            first.start_bar,
        ));
        states.push(timed_state_from_named_band(
            PdaConceptKind::BalancedPriceRange,
            PriceLevelBand {
                top: first.top,
                bottom: first.bottom,
            },
            first.direction,
            candles,
            first.start_bar,
        ));
        states.push(timed_state_from_named_band(
            PdaConceptKind::OpenRangeGap,
            PriceLevelBand {
                top: first.top,
                bottom: first.bottom,
            },
            first.direction,
            candles,
            first.start_bar,
        ));
        states.push(timed_state_from_named_band(
            PdaConceptKind::Ndog,
            PriceLevelBand {
                top: first.top,
                bottom: first.bottom,
            },
            first.direction,
            candles,
            first.start_bar,
        ));
        states.push(timed_state_from_named_band(
            PdaConceptKind::Nwog,
            PriceLevelBand {
                top: first.top,
                bottom: first.bottom,
            },
            first.direction,
            candles,
            first.start_bar,
        ));
        states.push(timed_state_from_named_band(
            PdaConceptKind::SwingFailurePattern,
            PriceLevelBand {
                top: first.top,
                bottom: first.bottom,
            },
            match first.direction {
                Direction::Bull => Direction::Bear,
                Direction::Bear => Direction::Bull,
                Direction::Neutral => Direction::Neutral,
            },
            candles,
            first.start_bar,
        ));
    }

    states
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Candle;
    use chrono::{TimeZone, Utc};

    fn candle(ts: i64, open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            timestamp: Utc.timestamp_opt(ts, 0).unwrap(),
            open,
            high,
            low,
            close,
            volume: 1.0,
        }
    }

    #[test]
    fn fvg_state_inverses_after_full_fill() {
        let candles = vec![
            candle(1, 100.0, 101.0, 99.5, 100.5),
            candle(2, 101.0, 102.0, 100.8, 101.8),
            candle(3, 103.0, 104.0, 103.2, 103.7),
            candle(4, 103.5, 103.8, 101.2, 101.4),
            candle(5, 101.1, 101.3, 99.0, 99.4),
        ];
        let fvg = FairValueGap {
            top: 103.2,
            bottom: 101.0,
            direction: Direction::Bull,
            start_bar: 1,
            filled: false,
        };
        let state = timed_state_from_fvg(&fvg, &candles);
        assert_eq!(state.state, PdaLifecycleState::Inversed);
        assert_eq!(state.direction, Direction::Bear);
        assert!(state
            .transitions
            .iter()
            .any(|t| t.state == PdaLifecycleState::Mitigated));
    }

    #[test]
    fn fvg_state_expires_when_not_revisited() {
        let candles = vec![
            candle(1, 100.0, 101.0, 99.5, 100.5),
            candle(2, 101.0, 102.0, 100.8, 101.8),
            candle(3, 110.0, 111.0, 109.5, 110.5),
            candle(4, 111.0, 112.0, 110.5, 111.5),
        ];
        let mut state = initial_timed_pda_state(
            PdaConceptKind::FairValueGap,
            Direction::Bull,
            PriceLevelBand {
                top: 103.2,
                bottom: 101.0,
            },
            0,
            PdaInvalidationRule::TimeExpiry,
            PdaInverseMode::None,
            1,
        );
        update_timed_pda_state(&mut state, &candles);
        assert_eq!(state.state, PdaLifecycleState::Expired);
    }

    #[test]
    fn pda_rule_spec_is_concept_specific() {
        let sfp = pda_rule_spec(PdaConceptKind::SwingFailurePattern);
        let ote = pda_rule_spec(PdaConceptKind::OptimalTradeEntry);
        let ndog = pda_rule_spec(PdaConceptKind::Ndog);
        assert_eq!(sfp.invalidation_rule, PdaInvalidationRule::StructureBreak);
        assert_eq!(ote.inverse_mode, PdaInverseMode::None);
        assert_eq!(ndog.inverse_mode, PdaInverseMode::FlipSameBand);
    }

    #[test]
    fn core_timed_states_include_pool_and_ote_variants() {
        let candles = vec![
            candle(1, 100.0, 101.0, 99.5, 100.5),
            candle(2, 101.0, 102.0, 100.8, 101.8),
            candle(3, 103.0, 104.0, 103.2, 103.7),
            candle(4, 103.5, 103.8, 101.2, 101.4),
            candle(5, 101.1, 101.3, 99.0, 99.4),
        ];
        let fvgs = vec![FairValueGap {
            top: 103.2,
            bottom: 101.0,
            direction: Direction::Bull,
            start_bar: 1,
            filled: false,
        }];
        let pools = vec![LiquidityPool {
            price_level: 101.0,
            sp_count: 3,
            pool_type: Direction::Bull,
        }];
        let states = build_core_timed_pda_states(
            &candles,
            &fvgs,
            &pools,
            Some((101.0, 103.2, Direction::Bull)),
        );
        assert!(states
            .iter()
            .any(|s| s.concept == PdaConceptKind::LiquidityPool));
        assert!(states
            .iter()
            .any(|s| s.concept == PdaConceptKind::OptimalTradeEntry));
        assert!(states
            .iter()
            .any(|s| s.concept == PdaConceptKind::InversionFairValueGap));
        assert!(states
            .iter()
            .any(|s| s.concept == PdaConceptKind::BalancedPriceRange));
        assert!(states
            .iter()
            .any(|s| s.concept == PdaConceptKind::OpenRangeGap));
        assert!(states.iter().any(|s| s.concept == PdaConceptKind::Ndog));
        assert!(states.iter().any(|s| s.concept == PdaConceptKind::Nwog));
        assert!(states
            .iter()
            .any(|s| s.concept == PdaConceptKind::SwingFailurePattern));
    }
}
