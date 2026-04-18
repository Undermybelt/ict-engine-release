use crate::types::{Candle, Direction, StructureBreak, StructureType, SwingPoint};

/// Detect Break of Structure (BOS) and Change of Character (CHoCH)
pub fn detect_structure_breaks(
    candles: &[Candle],
    swing_highs: &[SwingPoint],
    swing_lows: &[SwingPoint],
) -> Vec<StructureBreak> {
    let mut breaks = Vec::new();
    let mut last_trend = Direction::Neutral;

    for (i, candle) in candles.iter().enumerate() {
        // Check for bullish break (breaking above last swing high)
        if let Some(last_high) = swing_highs
            .iter()
            .filter(|sp| sp.index < i)
            .max_by_key(|sp| sp.index)
        {
            if candle.high > last_high.price {
                let break_type = if last_trend == Direction::Bear {
                    StructureType::CHoCH // Change of character
                } else {
                    StructureType::BOS // Break of structure
                };

                breaks.push(StructureBreak {
                    bar_index: i,
                    break_type,
                    direction: Direction::Bull,
                    level: last_high.price,
                });

                last_trend = Direction::Bull;
            }
        }

        // Check for bearish break (breaking below last swing low)
        if let Some(last_low) = swing_lows
            .iter()
            .filter(|sp| sp.index < i)
            .max_by_key(|sp| sp.index)
        {
            if candle.low < last_low.price {
                let break_type = if last_trend == Direction::Bull {
                    StructureType::CHoCH // Change of character
                } else {
                    StructureType::BOS // Break of structure
                };

                breaks.push(StructureBreak {
                    bar_index: i,
                    break_type,
                    direction: Direction::Bear,
                    level: last_low.price,
                });

                last_trend = Direction::Bear;
            }
        }
    }

    breaks
}

/// Get the latest structure break
pub fn latest_structure_break(breaks: &[StructureBreak]) -> Option<&StructureBreak> {
    breaks.iter().max_by_key(|b| b.bar_index)
}

/// Count recent structure breaks
pub fn count_recent_breaks(
    breaks: &[StructureBreak],
    lookback: usize,
    total_candles: usize,
) -> usize {
    let threshold = total_candles.saturating_sub(lookback);
    breaks.iter().filter(|b| b.bar_index >= threshold).count()
}

/// Detect trend based on structure breaks
pub fn detect_trend_from_breaks(breaks: &[StructureBreak]) -> Direction {
    if let Some(latest) = latest_structure_break(breaks) {
        latest.direction
    } else {
        Direction::Neutral
    }
}
