use crate::types::{Candle, Direction, LiquidityPool, LiquiditySweep};

/// Detect liquidity pools (areas where multiple swing points cluster)
pub fn detect_liquidity_pools(
    candles: &[Candle],
    atr: &[f64],
    atr_multiplier: f64,
    min_touches: usize,
) -> Vec<LiquidityPool> {
    let swings = super::swing::find_all_swing_points(candles, 3);
    let mut pools = Vec::new();

    // Group swing points by price level
    let tolerance = if atr.is_empty() {
        10.0
    } else {
        atr.last().unwrap() * atr_multiplier
    };

    let mut used = vec![false; swings.len()];

    for i in 0..swings.len() {
        if used[i] {
            continue;
        }

        let mut cluster = vec![&swings[i]];
        used[i] = true;

        for j in i + 1..swings.len() {
            if used[j] {
                continue;
            }

            if (swings[j].price - swings[i].price).abs() <= tolerance {
                cluster.push(&swings[j]);
                used[j] = true;
            }
        }

        if cluster.len() >= min_touches {
            let avg_price: f64 =
                cluster.iter().map(|sp| sp.price).sum::<f64>() / cluster.len() as f64;
            let pool_type = if cluster.iter().any(|sp| sp.sp_type == Direction::Bear) {
                Direction::Bear // Resistance
            } else {
                Direction::Bull // Support
            };

            pools.push(LiquidityPool {
                price_level: avg_price,
                sp_count: cluster.len(),
                pool_type,
            });
        }
    }

    pools.sort_by(|a, b| a.price_level.partial_cmp(&b.price_level).unwrap());
    pools
}

/// Detect liquidity sweeps (price breaks through pool then returns)
pub fn detect_liquidity_sweep(
    candles: &[Candle],
    pools: &[LiquidityPool],
    return_bars: usize,
) -> Vec<LiquiditySweep> {
    if candles.len() <= return_bars {
        return Vec::new();
    }

    let mut sweeps = Vec::new();

    for pool in pools {
        for i in 0..candles.len() - return_bars {
            let candle = &candles[i];

            // Check for sweep above resistance
            if pool.pool_type == Direction::Bear && candle.high > pool.price_level {
                // Check if price returns below within return_bars
                let mut returned = false;
                let mut return_bar = i;

                for (j, candle) in candles
                    .iter()
                    .enumerate()
                    .skip(i + 1)
                    .take(return_bars.min(candles.len() - i - 1))
                {
                    if candle.close < pool.price_level {
                        returned = true;
                        return_bar = j;
                        break;
                    }
                }

                if returned {
                    sweeps.push(LiquiditySweep {
                        sweep_bar: i,
                        return_bar,
                        pool_price: pool.price_level,
                        sweep_direction: Direction::Bear, // Sweep above then return down
                    });
                }
            }

            // Check for sweep below support
            if pool.pool_type == Direction::Bull && candle.low < pool.price_level {
                // Check if price returns above within return_bars
                let mut returned = false;
                let mut return_bar = i;

                for (j, candle) in candles
                    .iter()
                    .enumerate()
                    .skip(i + 1)
                    .take(return_bars.min(candles.len() - i - 1))
                {
                    if candle.close > pool.price_level {
                        returned = true;
                        return_bar = j;
                        break;
                    }
                }

                if returned {
                    sweeps.push(LiquiditySweep {
                        sweep_bar: i,
                        return_bar,
                        pool_price: pool.price_level,
                        sweep_direction: Direction::Bull, // Sweep below then return up
                    });
                }
            }
        }
    }

    sweeps
}

/// Count recent liquidity sweeps
pub fn count_recent_sweeps(
    candles: &[Candle],
    sweeps: &[LiquiditySweep],
    lookback: usize,
) -> usize {
    let threshold = candles.len().saturating_sub(lookback);
    sweeps.iter().filter(|s| s.sweep_bar >= threshold).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn candle(ts: i64, open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            timestamp: Utc.timestamp_opt(ts, 0).single().expect("valid ts"),
            open,
            high,
            low,
            close,
            volume: 1.0,
        }
    }

    #[test]
    fn detect_liquidity_sweep_returns_empty_for_short_windows() {
        let candles = vec![
            candle(1, 100.0, 101.0, 99.0, 100.5),
            candle(2, 100.5, 101.5, 100.0, 101.0),
            candle(3, 101.0, 102.0, 100.5, 101.5),
        ];
        let pools = vec![LiquidityPool {
            price_level: 101.0,
            sp_count: 2,
            pool_type: Direction::Bear,
        }];

        let sweeps = detect_liquidity_sweep(&candles, &pools, 5);
        assert!(sweeps.is_empty());
    }
}
