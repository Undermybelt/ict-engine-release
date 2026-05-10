//! Mitigation Block detector.
//!
//! ICT defines a *mitigation block* as the price level of a failed
//! swing — the market revisits a prior swing high/low but does **not**
//! take it out, then reverses. The unmitigated portion of the swing
//! becomes the mitigation block.
//!
//! This detector emits the *event* of mitigation, not the swing-bar
//! origin: the consumer is the timeline assembler in P1, which cares
//! about "when did the failure happen" more than "which exact candle
//! is the level".
//!
//! Lifecycle:
//! 1. Find a swing high (or low) at `anchor_bar`.
//! 2. Within `retest_window` bars after the swing, find the first
//!    `retest_bar` whose wick reaches the swing level but whose close
//!    rejects (close < SH or close > SL).
//! 3. Wait `confirm_window` bars; if no close exceeds the level, the
//!    failure is confirmed at `confirm_bar = retest_bar + confirm_window`.
//!
//! Forward-leak: emit only when `confirm_bar < candles.len()`.

use crate::ict::swing::{find_swing_highs, find_swing_lows};
use crate::types::{Candle, Direction, MitigationBlock};

pub const DEFAULT_MITIGATION_SWING_LOOKBACK: usize = 3;
pub const DEFAULT_MITIGATION_RETEST_WINDOW: usize = 30;
pub const DEFAULT_MITIGATION_CONFIRM_WINDOW: usize = 5;
/// Tolerance (in price terms relative to the swing level) for what
/// counts as "wicked into" the level. 5 bps is the same neighbourhood
/// `pda_state.rs` uses for `TOUCH_EPSILON_BPS`.
pub const DEFAULT_MITIGATION_TOUCH_EPSILON_BPS: f64 = 0.0005;

pub fn detect_mitigation_blocks(
    candles: &[Candle],
    swing_lookback: usize,
    retest_window: usize,
    confirm_window: usize,
    touch_epsilon_bps: f64,
) -> Vec<MitigationBlock> {
    if candles.len() < swing_lookback * 2 + 1 {
        return Vec::new();
    }
    let highs = find_swing_highs(candles, swing_lookback);
    let lows = find_swing_lows(candles, swing_lookback);
    let mut out = Vec::new();

    for sh in &highs {
        if let Some(block) = scan_for_failure(
            candles,
            sh.index,
            sh.price,
            Direction::Bear,
            retest_window,
            confirm_window,
            touch_epsilon_bps,
        ) {
            out.push(block);
        }
    }
    for sl in &lows {
        if let Some(block) = scan_for_failure(
            candles,
            sl.index,
            sl.price,
            Direction::Bull,
            retest_window,
            confirm_window,
            touch_epsilon_bps,
        ) {
            out.push(block);
        }
    }
    out.sort_by_key(|m| m.confirm_bar);
    out
}

pub fn detect_mitigation_blocks_default(candles: &[Candle]) -> Vec<MitigationBlock> {
    detect_mitigation_blocks(
        candles,
        DEFAULT_MITIGATION_SWING_LOOKBACK,
        DEFAULT_MITIGATION_RETEST_WINDOW,
        DEFAULT_MITIGATION_CONFIRM_WINDOW,
        DEFAULT_MITIGATION_TOUCH_EPSILON_BPS,
    )
}

fn scan_for_failure(
    candles: &[Candle],
    anchor_bar: usize,
    level: f64,
    failure_direction: Direction,
    retest_window: usize,
    confirm_window: usize,
    touch_epsilon_bps: f64,
) -> Option<MitigationBlock> {
    let touch_epsilon = level.abs() * touch_epsilon_bps;
    let retest_end = (anchor_bar + retest_window).min(candles.len() - 1);
    if retest_end <= anchor_bar {
        return None;
    }
    for bar in (anchor_bar + 1)..=retest_end {
        let candle = &candles[bar];
        let touched = match failure_direction {
            // Bear failure (swing-high failed): wick reached SH but
            // close stayed below.
            Direction::Bear => candle.high >= level - touch_epsilon && candle.close < level,
            // Bull failure (swing-low failed): wick reached SL but
            // close stayed above.
            Direction::Bull => candle.low <= level + touch_epsilon && candle.close > level,
            Direction::Neutral => false,
        };
        if !touched {
            continue;
        }
        let confirm_end = bar + confirm_window;
        if confirm_end >= candles.len() {
            return None;
        }
        let breached = (bar + 1..=confirm_end).any(|j| {
            let c = &candles[j];
            match failure_direction {
                Direction::Bear => c.close > level,
                Direction::Bull => c.close < level,
                Direction::Neutral => false,
            }
        });
        if breached {
            // The retest didn't actually fail — keep scanning later
            // bars within the retest window for a fresh attempt.
            continue;
        }
        return Some(MitigationBlock {
            level,
            direction: failure_direction,
            anchor_bar,
            retest_bar: bar,
            confirm_bar: confirm_end,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn baseline(len: usize, base: f64, drift: f64) -> Vec<Candle> {
        (0..len)
            .map(|i| {
                let mid = base + drift * i as f64;
                candle(i as i64, mid, mid + 0.2, mid - 0.2, mid + 0.05)
            })
            .collect()
    }

    #[test]
    fn empty_yields_empty() {
        assert!(detect_mitigation_blocks_default(&[]).is_empty());
    }

    #[test]
    fn no_failed_swing_yields_no_mitigation() {
        // Pure uptrend — every swing low is held.
        let candles = baseline(80, 100.0, 0.05);
        let out = detect_mitigation_blocks_default(&candles);
        assert!(out.is_empty(), "monotone uptrend has no failed swings");
    }

    #[test]
    fn swing_high_retested_then_failed_emits_bear_mitigation() {
        // Build a fixture with a clear swing high at ~104, retested
        // at ~104 without taking it out, then prices fall.
        let mut candles = vec![
            candle(0, 100.0, 100.2, 99.8, 100.1),
            candle(1, 100.1, 100.4, 100.0, 100.3),
            candle(2, 100.3, 100.6, 100.2, 100.5),
            candle(3, 100.5, 100.8, 100.4, 100.7),
            candle(4, 100.7, 104.0, 100.7, 103.8), // swing high candidate
            candle(5, 103.8, 103.9, 103.5, 103.7), // post-swing (high < bar 4 high)
            candle(6, 103.7, 103.9, 102.5, 102.7),
            candle(7, 102.7, 103.0, 101.8, 102.0),
            candle(8, 102.0, 102.5, 101.0, 101.2),
            candle(9, 101.2, 101.8, 100.8, 101.0),
            candle(10, 101.0, 102.0, 100.5, 100.8),
        ];
        // Retest: bar 11 wicks up to 104 but closes below
        candles.push(candle(11, 100.8, 104.0, 100.8, 103.5));
        // Confirmation: 5 bars all close < 104
        for i in 12..=16 {
            candles.push(candle(i as i64, 103.5, 103.7, 102.0, 103.0));
        }
        // Buffer
        for i in 17..30 {
            candles.push(candle(i as i64, 103.0, 103.2, 102.5, 102.8));
        }

        let out = detect_mitigation_blocks_default(&candles);
        let bear: Vec<&MitigationBlock> = out
            .iter()
            .filter(|m| m.direction == Direction::Bear)
            .collect();
        assert!(
            !bear.is_empty(),
            "expected a bearish mitigation at the failed retest"
        );
        let mb = bear[0];
        assert_eq!(mb.retest_bar, 11);
        assert_eq!(mb.confirm_bar, 11 + DEFAULT_MITIGATION_CONFIRM_WINDOW);
        assert!((mb.level - 104.0).abs() < 1e-6);
    }

    #[test]
    fn retest_that_breaches_does_not_emit() {
        // Same as above but the confirmation window contains a
        // close > 104 — this is BOS, not a failed mitigation.
        let mut candles = vec![
            candle(0, 100.0, 100.2, 99.8, 100.1),
            candle(1, 100.1, 100.4, 100.0, 100.3),
            candle(2, 100.3, 100.6, 100.2, 100.5),
            candle(3, 100.5, 100.8, 100.4, 100.7),
            candle(4, 100.7, 104.0, 100.7, 103.8),
            candle(5, 103.8, 103.9, 103.5, 103.7),
            candle(6, 103.7, 103.9, 102.5, 102.7),
            candle(7, 102.7, 103.0, 101.8, 102.0),
            candle(8, 102.0, 102.5, 101.0, 101.2),
            candle(9, 101.2, 101.8, 100.8, 101.0),
            candle(10, 101.0, 102.0, 100.5, 100.8),
            candle(11, 100.8, 104.0, 100.8, 103.5), // retest-touch
            candle(12, 103.5, 105.0, 103.4, 104.8), // BOS — close > 104
        ];
        for i in 13..30 {
            candles.push(candle(i as i64, 105.0, 105.2, 104.5, 104.8));
        }
        let out = detect_mitigation_blocks_default(&candles);
        let bear: Vec<&MitigationBlock> = out
            .iter()
            .filter(|m| m.direction == Direction::Bear && m.retest_bar == 11)
            .collect();
        assert!(
            bear.is_empty(),
            "BOS within confirm_window must reject mitigation"
        );
    }
}
