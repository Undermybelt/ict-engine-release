use crate::types::{Candle, Direction, RejectionBlock};

/// Detect Rejection Blocks (Pinbars/hammers/shooting stars)
/// A rejection block has:
/// - Range > ATR * min_range_atr
/// - Body < wick * body_wick_ratio
pub fn detect_rb(
    candles: &[Candle],
    atr: &[f64],
    min_range_atr: f64,
    body_wick_ratio: f64,
) -> Vec<RejectionBlock> {
    let mut rbs = Vec::new();

    for (i, candle) in candles.iter().enumerate() {
        let atr_val = if atr.is_empty() {
            candle.range()
        } else {
            let atr_idx = i.saturating_sub(candles.len() - atr.len());
            atr[atr_idx.min(atr.len() - 1)]
        };

        // Check if range is significant
        if candle.range() < atr_val * min_range_atr {
            continue;
        }

        let body = candle.body();
        let upper_wick = candle.upper_wick();
        let lower_wick = candle.lower_wick();

        // Determine direction based on wick location
        if lower_wick > upper_wick && body < lower_wick * body_wick_ratio {
            // Bullish pinbar (long lower wick)
            rbs.push(RejectionBlock {
                bar_index: i,
                direction: Direction::Bull,
                body_ratio: body / candle.range(),
                range_atr: candle.range() / atr_val,
            });
        } else if upper_wick > lower_wick && body < upper_wick * body_wick_ratio {
            // Bearish pinbar (long upper wick)
            rbs.push(RejectionBlock {
                bar_index: i,
                direction: Direction::Bear,
                body_ratio: body / candle.range(),
                range_atr: candle.range() / atr_val,
            });
        }
    }

    rbs
}

/// Check if there's a recent rejection block
pub fn has_recent_rb(
    candles: &[Candle],
    atr: &[f64],
    lookback: usize,
    min_range_atr: f64,
    body_wick_ratio: f64,
) -> bool {
    let rbs = detect_rb(candles, atr, min_range_atr, body_wick_ratio);
    let threshold = candles.len().saturating_sub(lookback);
    rbs.iter().any(|rb| rb.bar_index >= threshold)
}

/// Detect pinbar specifically (strong rejection)
pub fn detect_pinbar(candles: &[Candle], atr: &[f64]) -> Vec<RejectionBlock> {
    detect_rb(candles, atr, 2.0, 0.33)
}

/// Check if there's a recent pinbar
pub fn has_recent_pinbar(candles: &[Candle], atr: &[f64], lookback: usize) -> bool {
    let pinbars = detect_pinbar(candles, atr);
    let threshold = candles.len().saturating_sub(lookback);
    pinbars.iter().any(|p| p.bar_index >= threshold)
}
