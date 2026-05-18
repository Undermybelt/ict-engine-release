use crate::types::{Candle, Direction, OrderBlock};

/// Detect Order Blocks
/// Bullish OB: Last bearish candle before a bullish expansion
/// Bearish OB: Last bullish candle before a bearish expansion
pub fn detect_order_blocks(candles: &[Candle]) -> Vec<OrderBlock> {
    if candles.len() < 3 {
        return Vec::new();
    }

    let mut obs = Vec::new();

    for i in 2..candles.len() {
        let prev2 = &candles[i - 2];
        let prev1 = &candles[i - 1];
        let curr = &candles[i];

        // Bullish OB: prev1 is bearish, curr is bullish with significant move
        if prev1.is_bearish() && curr.is_bullish() && curr.close > prev2.high {
            obs.push(OrderBlock {
                high: prev1.high,
                low: prev1.low,
                ob_type: Direction::Bull,
                bar_index: i - 1,
                tested: false,
            });
        }

        // Bearish OB: prev1 is bullish, curr is bearish with significant move
        if prev1.is_bullish() && curr.is_bearish() && curr.close < prev2.low {
            obs.push(OrderBlock {
                high: prev1.high,
                low: prev1.low,
                ob_type: Direction::Bear,
                bar_index: i - 1,
                tested: false,
            });
        }
    }

    obs
}

/// Check if an Order Block has been tested
pub fn check_ob_tested(candles: &[Candle], ob: &OrderBlock) -> bool {
    for candle in candles.iter().skip(ob.bar_index + 1) {
        if ob.ob_type == Direction::Bull {
            // Bullish OB is tested when price returns to it
            if candle.low <= ob.high && candle.high >= ob.low {
                return true;
            }
        } else {
            // Bearish OB is tested when price returns to it
            if candle.high >= ob.low && candle.low <= ob.high {
                return true;
            }
        }
    }

    false
}

/// Find untested Order Blocks
pub fn find_untested_obs(candles: &[Candle]) -> Vec<OrderBlock> {
    let mut obs = detect_order_blocks(candles);

    for ob in &mut obs {
        ob.tested = check_ob_tested(candles, ob);
    }

    obs.retain(|o| !o.tested);
    obs
}

/// Count nearby untested Order Blocks
pub fn count_nearby_obs(candles: &[Candle], lookback: usize) -> usize {
    let obs = find_untested_obs(candles);
    let threshold = candles.len().saturating_sub(lookback);
    obs.iter().filter(|o| o.bar_index >= threshold).count()
}

/// Find the nearest Order Block to a given price
pub fn find_nearest_ob(candles: &[Candle], price: f64, direction: Direction) -> Option<OrderBlock> {
    let obs = find_untested_obs(candles);

    obs.iter()
        .filter(|o| o.ob_type == direction)
        .min_by(|a, b| {
            let dist_a = ((a.high + a.low) / 2.0 - price).abs();
            let dist_b = ((b.high + b.low) / 2.0 - price).abs();
            dist_a
                .partial_cmp(&dist_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned()
}
