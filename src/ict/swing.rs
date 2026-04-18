use crate::types::{Candle, Direction, SwingPoint};

/// Find swing highs (local maxima)
pub fn find_swing_highs(candles: &[Candle], lookback: usize) -> Vec<SwingPoint> {
    if candles.len() < lookback * 2 + 1 {
        return Vec::new();
    }

    let mut swings = Vec::new();

    for i in lookback..candles.len() - lookback {
        let current = candles[i].high;
        let mut is_swing = true;

        // Check left side
        for candle in candles.iter().take(i).skip(i - lookback) {
            if candle.high >= current {
                is_swing = false;
                break;
            }
        }

        // Check right side
        if is_swing {
            for candle in candles.iter().skip(i + 1).take(lookback) {
                if candle.high >= current {
                    is_swing = false;
                    break;
                }
            }
        }

        if is_swing {
            swings.push(SwingPoint {
                index: i,
                price: current,
                sp_type: Direction::Bear, // Swing High is bearish
            });
        }
    }

    swings
}

/// Find swing lows (local minima)
pub fn find_swing_lows(candles: &[Candle], lookback: usize) -> Vec<SwingPoint> {
    if candles.len() < lookback * 2 + 1 {
        return Vec::new();
    }

    let mut swings = Vec::new();

    for i in lookback..candles.len() - lookback {
        let current = candles[i].low;
        let mut is_swing = true;

        // Check left side
        for candle in candles.iter().take(i).skip(i - lookback) {
            if candle.low <= current {
                is_swing = false;
                break;
            }
        }

        // Check right side
        if is_swing {
            for candle in candles.iter().skip(i + 1).take(lookback) {
                if candle.low <= current {
                    is_swing = false;
                    break;
                }
            }
        }

        if is_swing {
            swings.push(SwingPoint {
                index: i,
                price: current,
                sp_type: Direction::Bull, // Swing Low is bullish
            });
        }
    }

    swings
}

/// Get all swing points (both highs and lows)
pub fn find_all_swing_points(candles: &[Candle], lookback: usize) -> Vec<SwingPoint> {
    let mut highs = find_swing_highs(candles, lookback);
    let mut lows = find_swing_lows(candles, lookback);

    highs.append(&mut lows);
    highs.sort_by_key(|sp| sp.index);
    highs
}

/// Find the most recent swing high before a given index
pub fn find_last_swing_high_before(swings: &[SwingPoint], index: usize) -> Option<&SwingPoint> {
    swings
        .iter()
        .filter(|sp| sp.index < index && sp.sp_type == Direction::Bear)
        .max_by_key(|sp| sp.index)
}

/// Find the most recent swing low before a given index
pub fn find_last_swing_low_before(swings: &[SwingPoint], index: usize) -> Option<&SwingPoint> {
    swings
        .iter()
        .filter(|sp| sp.index < index && sp.sp_type == Direction::Bull)
        .max_by_key(|sp| sp.index)
}
