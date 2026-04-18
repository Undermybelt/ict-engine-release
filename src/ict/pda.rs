use crate::types::{Candle, Direction};

#[derive(Debug, Clone)]
pub struct PDASequence {
    pub direction: Direction,
    pub start_index: usize,
    pub end_index: usize,
    pub points: Vec<usize>, // Indices of swing points in the sequence
}

/// Detect bullish PDA (Premium Dollar Average)
/// Pattern: consecutive higher lows with closes in upper half of candles
pub fn detect_bullish_pda(candles: &[Candle], min_points: usize) -> Vec<PDASequence> {
    let swings = super::swing::find_swing_lows(candles, 2);
    let mut sequences = Vec::new();
    let mut current_seq: Vec<usize> = Vec::new();

    for i in 0..swings.len() {
        let sp = &swings[i];

        // Check if close is in upper half
        let candle = &candles[sp.index];
        let upper_half = candle.close >= (candle.high + candle.low) / 2.0;

        if !upper_half {
            // Reset sequence
            if current_seq.len() >= min_points {
                sequences.push(PDASequence {
                    direction: Direction::Bull,
                    start_index: swings[current_seq[0]].index,
                    end_index: swings[current_seq[current_seq.len() - 1]].index,
                    points: current_seq.clone(),
                });
            }
            current_seq.clear();
            continue;
        }

        // Check for higher low
        if let Some(&last_idx) = current_seq.last() {
            if swings[i].price > swings[last_idx].price {
                current_seq.push(i);
            } else {
                // Lower low breaks the sequence
                if current_seq.len() >= min_points {
                    sequences.push(PDASequence {
                        direction: Direction::Bull,
                        start_index: swings[current_seq[0]].index,
                        end_index: swings[current_seq[current_seq.len() - 1]].index,
                        points: current_seq.clone(),
                    });
                }
                current_seq.clear();
                current_seq.push(i);
            }
        } else {
            current_seq.push(i);
        }
    }

    // Don't forget the last sequence
    if current_seq.len() >= min_points {
        sequences.push(PDASequence {
            direction: Direction::Bull,
            start_index: swings[current_seq[0]].index,
            end_index: swings[current_seq[current_seq.len() - 1]].index,
            points: current_seq,
        });
    }

    sequences
}

/// Detect bearish PDA
/// Pattern: consecutive lower highs with closes in lower half of candles
pub fn detect_bearish_pda(candles: &[Candle], min_points: usize) -> Vec<PDASequence> {
    let swings = super::swing::find_swing_highs(candles, 2);
    let mut sequences = Vec::new();
    let mut current_seq: Vec<usize> = Vec::new();

    for i in 0..swings.len() {
        let sp = &swings[i];

        // Check if close is in lower half
        let candle = &candles[sp.index];
        let lower_half = candle.close <= (candle.high + candle.low) / 2.0;

        if !lower_half {
            // Reset sequence
            if current_seq.len() >= min_points {
                sequences.push(PDASequence {
                    direction: Direction::Bear,
                    start_index: swings[current_seq[0]].index,
                    end_index: swings[current_seq[current_seq.len() - 1]].index,
                    points: current_seq.clone(),
                });
            }
            current_seq.clear();
            continue;
        }

        // Check for lower high
        if let Some(&last_idx) = current_seq.last() {
            if swings[i].price < swings[last_idx].price {
                current_seq.push(i);
            } else {
                // Higher high breaks the sequence
                if current_seq.len() >= min_points {
                    sequences.push(PDASequence {
                        direction: Direction::Bear,
                        start_index: swings[current_seq[0]].index,
                        end_index: swings[current_seq[current_seq.len() - 1]].index,
                        points: current_seq.clone(),
                    });
                }
                current_seq.clear();
                current_seq.push(i);
            }
        } else {
            current_seq.push(i);
        }
    }

    // Don't forget the last sequence
    if current_seq.len() >= min_points {
        sequences.push(PDASequence {
            direction: Direction::Bear,
            start_index: swings[current_seq[0]].index,
            end_index: swings[current_seq[current_seq.len() - 1]].index,
            points: current_seq,
        });
    }

    sequences
}

/// Count active PDA sequences
pub fn count_active_pda(candles: &[Candle], min_points: usize) -> (usize, usize) {
    let bullish = detect_bullish_pda(candles, min_points);
    let bearish = detect_bearish_pda(candles, min_points);

    let bull_count = bullish
        .iter()
        .filter(|p| p.end_index >= candles.len() - 5)
        .count();
    let bear_count = bearish
        .iter()
        .filter(|p| p.end_index >= candles.len() - 5)
        .count();

    (bull_count, bear_count)
}
