use crate::types::{Candle, Direction, OrderBlock, CISD};

/// Detect Change in State of Delivery (CISD)
/// CISD occurs when price reverses direction and breaks through an Order Block
pub fn detect_cisd(candles: &[Candle], obs: &[OrderBlock], min_strength: usize) -> Vec<CISD> {
    if candles.len() < 3 {
        return Vec::new();
    }

    let mut cisds = Vec::new();

    for i in 2..candles.len() {
        let prev2 = &candles[i - 2];
        let prev1 = &candles[i - 1];
        let curr = &candles[i];

        // Bullish CISD: reversal from bearish to bullish
        if prev2.is_bearish() && prev1.is_bearish() && curr.is_bullish() {
            // Check if it breaks through a bearish OB
            for ob in obs.iter().filter(|o| o.ob_type == Direction::Bear) {
                if curr.high > ob.high {
                    let strength = calculate_cisd_strength(candles, i, Direction::Bull);
                    if strength >= min_strength {
                        cisds.push(CISD {
                            confirm_bar: i,
                            direction: Direction::Bull,
                            strength,
                        });
                        break;
                    }
                }
            }
        }

        // Bearish CISD: reversal from bullish to bearish
        if prev2.is_bullish() && prev1.is_bullish() && curr.is_bearish() {
            // Check if it breaks through a bullish OB
            for ob in obs.iter().filter(|o| o.ob_type == Direction::Bull) {
                if curr.low < ob.low {
                    let strength = calculate_cisd_strength(candles, i, Direction::Bear);
                    if strength >= min_strength {
                        cisds.push(CISD {
                            confirm_bar: i,
                            direction: Direction::Bear,
                            strength,
                        });
                        break;
                    }
                }
            }
        }
    }

    cisds
}

/// Calculate CISD strength (number of consecutive bars in the new direction)
fn calculate_cisd_strength(candles: &[Candle], start: usize, direction: Direction) -> usize {
    let mut strength = 1;

    for candle in candles.iter().skip(start + 1) {
        let continues = match direction {
            Direction::Bull => candle.is_bullish(),
            Direction::Bear => candle.is_bearish(),
            Direction::Neutral => false,
        };

        if continues {
            strength += 1;
        } else {
            break;
        }
    }

    strength
}

/// Check if there's a recent CISD confirmation
pub fn has_recent_cisd(
    candles: &[Candle],
    obs: &[OrderBlock],
    lookback: usize,
    min_strength: usize,
) -> bool {
    let cisds = detect_cisd(candles, obs, min_strength);
    let threshold = candles.len().saturating_sub(lookback);
    cisds.iter().any(|c| c.confirm_bar >= threshold)
}
