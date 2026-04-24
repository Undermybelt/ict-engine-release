//! Raw-candle → `Vec<PdaToken>` emitter. Uses the existing `ict::` detectors
//! with MVP defaults so the `cluster_pda_sequences` pipeline has a real input
//! source. Pure function; fully deterministic.
//!
//! Constraints carried from the NLP plan:
//! - emitter is additive — it does not replace any existing detector or
//!   feature pipeline
//! - produced tokens are observations only; they do not feed trading
//!   decisions yet
//! - the default parameters below are the module's public surface for
//!   reproducibility; callers that need other thresholds should build their
//!   own `PdaToken` pipeline rather than mutate these constants

use crate::ict::{
    detect_cisd, detect_fvg, detect_liquidity_pools, detect_liquidity_sweep, detect_order_blocks,
    detect_rb, detect_structure_breaks, find_swing_highs, find_swing_lows,
};
use crate::indicators::compute_atr;
use crate::types::{Candle, LiquiditySweep};

use super::token::{PdaToken, PdaTokenKind};

pub const EMITTER_ATR_PERIOD: usize = 14;
pub const EMITTER_SWING_STRENGTH: usize = 3;
pub const EMITTER_LIQUIDITY_POOL_ATR_MULT: f64 = 0.5;
pub const EMITTER_LIQUIDITY_POOL_MIN_TOUCHES: usize = 2;
pub const EMITTER_LIQUIDITY_SWEEP_RETURN_BARS: usize = 5;
pub const EMITTER_CISD_MIN_STRENGTH: usize = 2;
pub const EMITTER_RB_MIN_RANGE_ATR: f64 = 1.0;
pub const EMITTER_RB_BODY_WICK_RATIO: f64 = 0.3;
pub const EMITTER_OVERLAP_WINDOW_BARS: usize = 5;
pub const EMITTER_NEAR_SWEEP_WINDOW_BARS: usize = 3;

pub fn emit_pda_sequence_from_candles(candles: &[Candle]) -> Vec<PdaToken> {
    if candles.is_empty() {
        return Vec::new();
    }

    let atr = compute_atr(candles, EMITTER_ATR_PERIOD);
    let swing_highs = find_swing_highs(candles, EMITTER_SWING_STRENGTH);
    let swing_lows = find_swing_lows(candles, EMITTER_SWING_STRENGTH);

    let fvgs = detect_fvg(candles);
    let obs = detect_order_blocks(candles);
    let pools = detect_liquidity_pools(
        candles,
        &atr,
        EMITTER_LIQUIDITY_POOL_ATR_MULT,
        EMITTER_LIQUIDITY_POOL_MIN_TOUCHES,
    );
    let sweeps = detect_liquidity_sweep(candles, &pools, EMITTER_LIQUIDITY_SWEEP_RETURN_BARS);
    let breaks = detect_structure_breaks(candles, &swing_highs, &swing_lows);
    let cisds = detect_cisd(candles, &obs, EMITTER_CISD_MIN_STRENGTH);
    let rbs = detect_rb(
        candles,
        &atr,
        EMITTER_RB_MIN_RANGE_ATR,
        EMITTER_RB_BODY_WICK_RATIO,
    );

    let mut tokens: Vec<PdaToken> = Vec::new();
    for fvg in &fvgs {
        tokens.push(make_token(
            PdaTokenKind::FairValueGap,
            fvg.start_bar,
            candles,
            &sweeps,
        ));
    }
    for ob in &obs {
        tokens.push(make_token(
            PdaTokenKind::OrderBlock,
            ob.bar_index,
            candles,
            &sweeps,
        ));
    }
    for sweep in &sweeps {
        tokens.push(make_token(
            PdaTokenKind::LiquiditySweep,
            sweep.sweep_bar,
            candles,
            &sweeps,
        ));
    }
    for sb in &breaks {
        tokens.push(make_token(
            PdaTokenKind::StructureBreak,
            sb.bar_index,
            candles,
            &sweeps,
        ));
    }
    for rb in &rbs {
        tokens.push(make_token(
            PdaTokenKind::RejectionBlock,
            rb.bar_index,
            candles,
            &sweeps,
        ));
    }
    for cisd in &cisds {
        tokens.push(make_token(
            PdaTokenKind::Cisd,
            cisd.confirm_bar,
            candles,
            &sweeps,
        ));
    }
    // PropulsionBlock has no dedicated detector in `ict::` yet — omitted in v1.

    // Stable sort by bar index keeps the per-kind emission order as a
    // deterministic tiebreak (FVG → OB → Sweep → SB → RB → CISD).
    tokens.sort_by_key(|token| token.bar_index);
    apply_same_kind_overlap(&mut tokens);
    tokens
}

fn make_token(
    kind: PdaTokenKind,
    bar_index: usize,
    candles: &[Candle],
    sweeps: &[LiquiditySweep],
) -> PdaToken {
    PdaToken::new(kind, bar_index)
        .with_liquidity_swept(is_near_sweep(bar_index, sweeps))
        .with_volume_imbalance(candle_body_ratio(candles, bar_index))
}

fn is_near_sweep(bar_index: usize, sweeps: &[LiquiditySweep]) -> bool {
    sweeps.iter().any(|sweep| {
        let delta = sweep.sweep_bar.abs_diff(bar_index);
        delta <= EMITTER_NEAR_SWEEP_WINDOW_BARS
    })
}

fn candle_body_ratio(candles: &[Candle], bar_index: usize) -> f64 {
    let Some(candle) = candles.get(bar_index) else {
        return 0.0;
    };
    let range = (candle.high - candle.low).max(f64::EPSILON);
    ((candle.close - candle.open) / range).clamp(-1.0, 1.0)
}

fn apply_same_kind_overlap(tokens: &mut [PdaToken]) {
    for i in 0..tokens.len() {
        let this_kind = tokens[i].kind;
        let this_bar = tokens[i].bar_index;
        let prior_bar = tokens[..i]
            .iter()
            .rev()
            .find(|token| token.kind == this_kind)
            .map(|token| token.bar_index);
        if let Some(prior) = prior_bar {
            let delta = this_bar.saturating_sub(prior);
            if delta < EMITTER_OVERLAP_WINDOW_BARS {
                let ratio = 1.0 - (delta as f64 / EMITTER_OVERLAP_WINDOW_BARS as f64);
                tokens[i].overlap = ratio.clamp(0.0, 1.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pda_sequence::cluster::cluster_pda_sequences;
    use crate::types::Candle;
    use chrono::{Duration, TimeZone, Utc};

    fn ts(n: i64) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap() + Duration::minutes(n)
    }

    fn candle(idx: i64, open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            timestamp: ts(idx),
            open,
            high,
            low,
            close,
            volume: 1_000.0,
        }
    }

    fn trending_up_series(len: usize) -> Vec<Candle> {
        // Bullish drift with gaps: wide green bars with a gap up every few bars.
        let mut candles = Vec::with_capacity(len);
        let mut base = 100.0;
        for i in 0..len {
            let gap = if i % 6 == 3 { 1.5 } else { 0.0 };
            let open = base + gap;
            let close = open + 1.0;
            let high = close + 0.2;
            let low = open - 0.2;
            candles.push(candle(i as i64, open, high, low, close));
            base = close;
        }
        candles
    }

    fn trending_down_series(len: usize) -> Vec<Candle> {
        let mut candles = Vec::with_capacity(len);
        let mut base = 200.0;
        for i in 0..len {
            let gap = if i % 6 == 3 { -1.5 } else { 0.0 };
            let open = base + gap;
            let close = open - 1.0;
            let high = open + 0.2;
            let low = close - 0.2;
            candles.push(candle(i as i64, open, high, low, close));
            base = close;
        }
        candles
    }

    #[test]
    fn empty_input_yields_empty_output() {
        let tokens = emit_pda_sequence_from_candles(&[]);
        assert!(tokens.is_empty());
    }

    #[test]
    fn tokens_are_sorted_by_bar_index() {
        let candles = trending_up_series(60);
        let tokens = emit_pda_sequence_from_candles(&candles);
        for window in tokens.windows(2) {
            assert!(
                window[0].bar_index <= window[1].bar_index,
                "tokens must be sorted by bar_index, got {} after {}",
                window[1].bar_index,
                window[0].bar_index
            );
        }
    }

    #[test]
    fn emitter_is_deterministic() {
        let candles = trending_up_series(60);
        let a = emit_pda_sequence_from_candles(&candles);
        let b = emit_pda_sequence_from_candles(&candles);
        assert_eq!(a, b);
    }

    #[test]
    fn emits_at_least_one_token_for_nontrivial_series() {
        let candles = trending_up_series(60);
        let tokens = emit_pda_sequence_from_candles(&candles);
        assert!(
            !tokens.is_empty(),
            "trending fixture should produce at least one detectable PDA event"
        );
    }

    #[test]
    fn overlap_is_populated_for_close_same_kind_events() {
        // Force a scenario: two FVGs within the overlap window.
        // The trending fixture tends to emit multiple FVGs.
        let candles = trending_up_series(80);
        let tokens = emit_pda_sequence_from_candles(&candles);
        let fvg_overlaps: Vec<f64> = tokens
            .iter()
            .filter(|token| token.kind == PdaTokenKind::FairValueGap)
            .map(|token| token.overlap)
            .collect();
        if fvg_overlaps.len() >= 2 {
            assert!(
                fvg_overlaps.iter().any(|v| *v > 0.0),
                "at least one close-spaced FVG should carry overlap > 0"
            );
        }
    }

    #[test]
    fn volume_imbalance_reflects_candle_polarity() {
        let candles = trending_up_series(60);
        let tokens = emit_pda_sequence_from_candles(&candles);
        // In a pure bullish drift, most detected events should sit on green candles.
        let positive = tokens
            .iter()
            .filter(|token| token.volume_imbalance_ratio > 0.0)
            .count();
        let negative = tokens
            .iter()
            .filter(|token| token.volume_imbalance_ratio < 0.0)
            .count();
        if tokens.len() >= 3 {
            assert!(
                positive >= negative,
                "bullish drift should lean positive volume_imbalance_ratio (pos={}, neg={})",
                positive,
                negative
            );
        }
    }

    #[test]
    fn end_to_end_pipeline_produces_cluster_packets() {
        let up_sequences: Vec<Vec<PdaToken>> = (0..3)
            .map(|offset| {
                let candles = trending_up_series(60 + offset * 4);
                emit_pda_sequence_from_candles(&candles)
            })
            .filter(|seq| !seq.is_empty())
            .collect();
        let down_sequences: Vec<Vec<PdaToken>> = (0..3)
            .map(|offset| {
                let candles = trending_down_series(60 + offset * 4);
                emit_pda_sequence_from_candles(&candles)
            })
            .filter(|seq| !seq.is_empty())
            .collect();

        let mut all = up_sequences;
        all.extend(down_sequences);
        if all.len() < 2 {
            // Fixture generated no detectable events — the emitter chain is
            // still well-formed, but the fixture is too gentle to exercise
            // clustering. Fail loudly so a future regression does not
            // silently reduce coverage.
            panic!("fixture produced too few sequences for clustering test");
        }
        let k = all.len().min(2);
        let packets = cluster_pda_sequences(&all, k).expect("cluster must succeed on valid input");
        assert_eq!(packets.len(), all.len());
        for packet in &packets {
            assert!(packet.dtw_distance_to_medoid.is_finite());
            assert!(!packet.medoid_pda_sequence.is_empty());
        }
    }
}
