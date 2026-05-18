use crate::types::{Candle, Direction, FairValueGap};

/// Detect Fair Value Gaps (FVG)
/// Bullish FVG: candle[i+1].low > candle[i-1].high
/// Bearish FVG: candle[i+1].high < candle[i-1].low
pub fn detect_fvg(candles: &[Candle]) -> Vec<FairValueGap> {
    if candles.len() < 3 {
        return Vec::new();
    }

    let mut fvgs = Vec::new();

    for i in 1..candles.len() - 1 {
        let prev = &candles[i - 1];
        let _curr = &candles[i];
        let next = &candles[i + 1];

        // Bullish FVG
        if next.low > prev.high {
            fvgs.push(FairValueGap {
                top: next.low,
                bottom: prev.high,
                direction: Direction::Bull,
                start_bar: i,
                filled: false,
            });
        }

        // Bearish FVG
        if next.high < prev.low {
            fvgs.push(FairValueGap {
                top: prev.low,
                bottom: next.high,
                direction: Direction::Bear,
                start_bar: i,
                filled: false,
            });
        }
    }

    fvgs
}

/// Check if an FVG has been filled
pub fn check_fvg_filled(candles: &[Candle], fvg: &FairValueGap) -> bool {
    for candle in candles.iter().skip(fvg.start_bar + 2) {
        if fvg.direction == Direction::Bull {
            // Bullish FVG is filled when price drops below the gap
            if candle.low <= fvg.bottom {
                return true;
            }
        } else {
            // Bearish FVG is filled when price rises above the gap
            if candle.high >= fvg.top {
                return true;
            }
        }
    }

    false
}

/// Find unfilled FVGs
pub fn find_unfilled_fvgs(candles: &[Candle]) -> Vec<FairValueGap> {
    let mut fvgs = detect_fvg(candles);

    for fvg in &mut fvgs {
        fvg.filled = check_fvg_filled(candles, fvg);
    }

    fvgs.retain(|f| !f.filled);
    fvgs
}

/// Count open (unfilled) FVGs
pub fn count_open_fvgs(candles: &[Candle]) -> usize {
    find_unfilled_fvgs(candles).len()
}

/// Find the nearest FVG to a given price
pub fn find_nearest_fvg(
    candles: &[Candle],
    price: f64,
    direction: Direction,
) -> Option<FairValueGap> {
    let fvgs = detect_fvg(candles);

    fvgs.iter()
        .filter(|f| f.direction == direction && !f.filled)
        .min_by(|a, b| {
            let dist_a = ((a.top + a.bottom) / 2.0 - price).abs();
            let dist_b = ((b.top + b.bottom) / 2.0 - price).abs();
            dist_a
                .partial_cmp(&dist_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned()
}
