//! Assembles the unified PDA event timeline from a candle series.
//!
//! Runs all 13 detectors covered by `PdaEventKind`, normalises their
//! heterogeneous bar-index conventions to a single "emission bar"
//! (the last candle required to confirm the event), and returns a
//! `Vec<PdaEvent>` sorted by `bar_index`.
//!
//! Defaults are chosen to match the per-detector module defaults
//! (`DEFAULT_*` constants). Callers needing different thresholds can
//! call the per-detector functions directly and assemble their own
//! timeline via `events_from_*` helpers.

use crate::ict::{
    detect_breaker_blocks, detect_cisd, detect_fvg, detect_inverse_fvgs, detect_liquidity_pools,
    detect_liquidity_sweep, detect_liquidity_voids, detect_market_structure_shifts,
    detect_mitigation_blocks, detect_order_blocks, detect_propulsion_blocks, detect_rb,
    detect_structure_breaks, detect_volume_imbalances, find_swing_highs, find_swing_lows,
    DEFAULT_IFVG_CONFIRM_WINDOW, DEFAULT_LIQUIDITY_VOID_MIN_GAP_ATR,
    DEFAULT_MITIGATION_CONFIRM_WINDOW, DEFAULT_MITIGATION_RETEST_WINDOW,
    DEFAULT_MITIGATION_SWING_LOOKBACK, DEFAULT_MITIGATION_TOUCH_EPSILON_BPS,
    DEFAULT_MSS_SWING_LOOKBACK, DEFAULT_PROPULSION_BODY_RANGE_MIN,
    DEFAULT_PROPULSION_RANGE_ATR_MIN, DEFAULT_PROPULSION_VOLUME_WINDOW,
    DEFAULT_PROPULSION_VOLUME_Z_MIN, DEFAULT_VOLUME_IMBALANCE_WINDOW,
    DEFAULT_VOLUME_IMBALANCE_Z_MIN,
};
use crate::types::Candle;

use super::event::{PdaEvent, PdaEventKind};

pub const TIMELINE_DEFAULT_SWING_STRENGTH: usize = 3;
pub const TIMELINE_DEFAULT_LIQUIDITY_POOL_ATR_MULT: f64 = 0.5;
pub const TIMELINE_DEFAULT_LIQUIDITY_POOL_MIN_TOUCHES: usize = 2;
pub const TIMELINE_DEFAULT_LIQUIDITY_SWEEP_RETURN_BARS: usize = 5;
pub const TIMELINE_DEFAULT_CISD_MIN_STRENGTH: usize = 2;
pub const TIMELINE_DEFAULT_RB_MIN_RANGE_ATR: f64 = 1.0;
pub const TIMELINE_DEFAULT_RB_BODY_WICK_RATIO: f64 = 0.3;

pub fn build_pda_timeline(candles: &[Candle], atr: &[f64]) -> Vec<PdaEvent> {
    if candles.is_empty() {
        return Vec::new();
    }

    let swing_highs = find_swing_highs(candles, TIMELINE_DEFAULT_SWING_STRENGTH);
    let swing_lows = find_swing_lows(candles, TIMELINE_DEFAULT_SWING_STRENGTH);

    let mut events: Vec<PdaEvent> = Vec::new();

    // 1. Fair Value Gaps
    for fvg in detect_fvg(candles) {
        events.push(
            PdaEvent::new(PdaEventKind::FairValueGap, fvg.start_bar, fvg.direction)
                .with_level((fvg.top + fvg.bottom) / 2.0),
        );
    }

    // 2. Inverse Fair Value Gaps
    for ifvg in detect_inverse_fvgs(candles, DEFAULT_IFVG_CONFIRM_WINDOW) {
        events.push(
            PdaEvent::new(
                PdaEventKind::InverseFairValueGap,
                ifvg.confirm_bar,
                ifvg.direction,
            )
            .with_level((ifvg.top + ifvg.bottom) / 2.0),
        );
    }

    // 3. Order Blocks
    let order_blocks = detect_order_blocks(candles);
    for ob in &order_blocks {
        events.push(
            PdaEvent::new(PdaEventKind::OrderBlock, ob.bar_index, ob.ob_type)
                .with_level((ob.high + ob.low) / 2.0),
        );
    }

    // 4. Breaker Blocks
    for breaker in detect_breaker_blocks(candles) {
        events.push(
            PdaEvent::new(
                PdaEventKind::BreakerBlock,
                breaker.retest_bar,
                breaker.direction,
            )
            .with_level((breaker.high + breaker.low) / 2.0),
        );
    }

    // 5. Mitigation Blocks
    for mb in detect_mitigation_blocks(
        candles,
        DEFAULT_MITIGATION_SWING_LOOKBACK,
        DEFAULT_MITIGATION_RETEST_WINDOW,
        DEFAULT_MITIGATION_CONFIRM_WINDOW,
        DEFAULT_MITIGATION_TOUCH_EPSILON_BPS,
    ) {
        events.push(
            PdaEvent::new(PdaEventKind::MitigationBlock, mb.confirm_bar, mb.direction)
                .with_level(mb.level),
        );
    }

    // 6. Propulsion Blocks
    for pb in detect_propulsion_blocks(
        candles,
        atr,
        DEFAULT_PROPULSION_BODY_RANGE_MIN,
        DEFAULT_PROPULSION_RANGE_ATR_MIN,
        DEFAULT_PROPULSION_VOLUME_WINDOW,
        DEFAULT_PROPULSION_VOLUME_Z_MIN,
    ) {
        let level = candles
            .get(pb.bar_index)
            .map(|c| c.close)
            .unwrap_or_default();
        events.push(
            PdaEvent::new(PdaEventKind::PropulsionBlock, pb.bar_index, pb.direction)
                .with_level(level),
        );
    }

    // 7. Rejection Blocks
    for rb in detect_rb(
        candles,
        atr,
        TIMELINE_DEFAULT_RB_MIN_RANGE_ATR,
        TIMELINE_DEFAULT_RB_BODY_WICK_RATIO,
    ) {
        let level = candles
            .get(rb.bar_index)
            .map(|c| c.close)
            .unwrap_or_default();
        events.push(
            PdaEvent::new(PdaEventKind::RejectionBlock, rb.bar_index, rb.direction)
                .with_level(level),
        );
    }

    // 8. Liquidity Sweeps (built off the on-the-fly liquidity pools)
    let pools = detect_liquidity_pools(
        candles,
        atr,
        TIMELINE_DEFAULT_LIQUIDITY_POOL_ATR_MULT,
        TIMELINE_DEFAULT_LIQUIDITY_POOL_MIN_TOUCHES,
    );
    let sweeps = detect_liquidity_sweep(
        candles,
        &pools,
        TIMELINE_DEFAULT_LIQUIDITY_SWEEP_RETURN_BARS,
    );
    for sweep in &sweeps {
        events.push(
            PdaEvent::new(
                PdaEventKind::LiquiditySweep,
                sweep.return_bar,
                sweep.sweep_direction,
            )
            .with_level(sweep.pool_price),
        );
    }

    // 9. Liquidity Voids
    for v in detect_liquidity_voids(candles, atr, DEFAULT_LIQUIDITY_VOID_MIN_GAP_ATR) {
        events.push(
            PdaEvent::new(PdaEventKind::LiquidityVoid, v.start_bar, v.direction)
                .with_level((v.top + v.bottom) / 2.0),
        );
    }

    // 10. Structure Breaks (BOS / CHoCH on wick)
    for sb in detect_structure_breaks(candles, &swing_highs, &swing_lows) {
        events.push(
            PdaEvent::new(PdaEventKind::StructureBreak, sb.bar_index, sb.direction)
                .with_level(sb.level),
        );
    }

    // 11. Market Structure Shifts (CHoCH on close)
    for mss in detect_market_structure_shifts(candles, DEFAULT_MSS_SWING_LOOKBACK) {
        events.push(
            PdaEvent::new(
                PdaEventKind::MarketStructureShift,
                mss.bar_index,
                mss.direction,
            )
            .with_level(mss.broken_swing_price),
        );
    }

    // 12. CISD
    for cisd in detect_cisd(candles, &order_blocks, TIMELINE_DEFAULT_CISD_MIN_STRENGTH) {
        let level = candles
            .get(cisd.confirm_bar)
            .map(|c| c.close)
            .unwrap_or_default();
        events.push(
            PdaEvent::new(PdaEventKind::Cisd, cisd.confirm_bar, cisd.direction).with_level(level),
        );
    }

    // 13. Volume Imbalances
    for vi in detect_volume_imbalances(
        candles,
        DEFAULT_VOLUME_IMBALANCE_WINDOW,
        DEFAULT_VOLUME_IMBALANCE_Z_MIN,
    ) {
        let level = candles
            .get(vi.bar_index)
            .map(|c| c.close)
            .unwrap_or_default();
        events.push(
            PdaEvent::new(PdaEventKind::VolumeImbalance, vi.bar_index, vi.direction)
                .with_level(level),
        );
    }

    // Stable sort by emission bar; secondary key (kind discriminant)
    // pinned via the emission order above for reproducibility.
    events.sort_by(|a, b| {
        a.bar_index
            .cmp(&b.bar_index)
            .then_with(|| event_kind_order(a.kind).cmp(&event_kind_order(b.kind)))
    });

    // Populate wall-clock timestamps from the candle slice so cross-
    // timeframe and session-aware setup matchers can compare event
    // times across timelines that don't share a bar_index basis.
    for ev in events.iter_mut() {
        if let Some(candle) = candles.get(ev.bar_index) {
            ev.timestamp = Some(candle.timestamp);
        }
    }
    events
}

fn event_kind_order(kind: PdaEventKind) -> u8 {
    match kind {
        PdaEventKind::FairValueGap => 0,
        PdaEventKind::InverseFairValueGap => 1,
        PdaEventKind::OrderBlock => 2,
        PdaEventKind::BreakerBlock => 3,
        PdaEventKind::MitigationBlock => 4,
        PdaEventKind::PropulsionBlock => 5,
        PdaEventKind::RejectionBlock => 6,
        PdaEventKind::LiquiditySweep => 7,
        PdaEventKind::LiquidityVoid => 8,
        PdaEventKind::StructureBreak => 9,
        PdaEventKind::MarketStructureShift => 10,
        PdaEventKind::Cisd => 11,
        PdaEventKind::VolumeImbalance => 12,
    }
}

/// Validate that every emitted event has `bar_index < candles.len()`.
/// Useful for shrinking down false positives during fixture work; the
/// production assembly already maintains this invariant.
pub fn assert_timeline_bars_valid(events: &[PdaEvent], candles_len: usize) {
    for ev in events {
        debug_assert!(
            ev.bar_index < candles_len,
            "timeline event {:?} at bar {} ≥ candles_len {}",
            ev.kind,
            ev.bar_index,
            candles_len
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators::compute_atr;
    use chrono::{Duration, TimeZone, Utc};

    fn ts(n: i64) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap() + Duration::minutes(n)
    }

    fn candle(idx: i64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Candle {
        Candle {
            timestamp: ts(idx),
            open,
            high,
            low,
            close,
            volume,
        }
    }

    fn trending_up(len: usize) -> Vec<Candle> {
        // Modest bullish drift with periodic gaps to seed FVGs.
        (0..len)
            .map(|i| {
                let gap = if i % 6 == 3 { 1.5 } else { 0.0 };
                let open = 100.0 + 0.1 * i as f64 + gap;
                let close = open + 0.4;
                let high = close + 0.2;
                let low = open - 0.2;
                let volume = if i % 7 == 0 { 5_000.0 } else { 1_000.0 };
                candle(i as i64, open, high, low, close, volume)
            })
            .collect()
    }

    #[test]
    fn empty_input_yields_empty_timeline() {
        let events = build_pda_timeline(&[], &[]);
        assert!(events.is_empty());
    }

    #[test]
    fn timeline_is_sorted_by_bar_index() {
        let candles = trending_up(80);
        let atr = compute_atr(&candles, 14);
        let events = build_pda_timeline(&candles, &atr);
        for window in events.windows(2) {
            assert!(
                window[0].bar_index <= window[1].bar_index,
                "timeline must be sorted by bar_index"
            );
        }
    }

    #[test]
    fn timeline_is_deterministic() {
        let candles = trending_up(80);
        let atr = compute_atr(&candles, 14);
        let a = build_pda_timeline(&candles, &atr);
        let b = build_pda_timeline(&candles, &atr);
        assert_eq!(a, b);
    }

    #[test]
    fn timeline_emits_at_least_one_event_for_nontrivial_series() {
        let candles = trending_up(80);
        let atr = compute_atr(&candles, 14);
        let events = build_pda_timeline(&candles, &atr);
        assert!(
            !events.is_empty(),
            "trending fixture should produce at least one PDA event"
        );
    }

    #[test]
    fn every_event_has_a_level() {
        let candles = trending_up(80);
        let atr = compute_atr(&candles, 14);
        let events = build_pda_timeline(&candles, &atr);
        for ev in &events {
            assert!(
                ev.level.is_some(),
                "event {:?} at bar {} missing level",
                ev.kind,
                ev.bar_index
            );
        }
    }

    #[test]
    fn every_event_bar_is_within_series() {
        let candles = trending_up(80);
        let atr = compute_atr(&candles, 14);
        let events = build_pda_timeline(&candles, &atr);
        assert_timeline_bars_valid(&events, candles.len());
        for ev in &events {
            assert!(ev.bar_index < candles.len());
        }
    }

    #[test]
    fn confirm_delayed_events_emit_at_their_confirmation_bar() {
        // Construct a minimal fixture that should emit a single
        // bullish FVG at bar 1 (start_bar) plus a forward-leak-safe
        // emission bar > 1 if the FVG ever inverts.
        let candles = vec![
            candle(0, 100.0, 100.5, 99.5, 100.4, 1000.0),
            candle(1, 100.4, 101.0, 100.4, 100.9, 1000.0),
            candle(2, 102.0, 102.5, 102.0, 102.3, 1000.0),
            candle(3, 102.3, 102.5, 100.0, 100.2, 1000.0),
            candle(4, 100.2, 100.4, 99.0, 99.2, 1000.0),
        ];
        let atr = compute_atr(&candles, 3);
        let events = build_pda_timeline(&candles, &atr);
        // The FVG should appear at bar 1 and the iFVG at bar 4.
        let fvg = events.iter().find(|e| e.kind == PdaEventKind::FairValueGap);
        let ifvg = events
            .iter()
            .find(|e| e.kind == PdaEventKind::InverseFairValueGap);
        assert!(fvg.is_some(), "expected FVG in fixture");
        assert!(ifvg.is_some(), "expected iFVG in fixture");
        let fvg_bar = fvg.unwrap().bar_index;
        let ifvg_bar = ifvg.unwrap().bar_index;
        assert!(
            ifvg_bar > fvg_bar,
            "iFVG emission bar must follow its parent FVG"
        );
    }
}
