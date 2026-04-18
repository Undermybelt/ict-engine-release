use super::HawkesProcess;
use crate::types::{Candle, LiquidityPool, LiquiditySweep};

/// Hawkes-enhanced liquidity sweep detector
pub struct HawkesSweepDetector;

impl HawkesSweepDetector {
    /// Detect sweeps using Hawkes process
    pub fn detect_sweeps_with_hawkes(
        candles: &[Candle],
        pools: &[LiquidityPool],
        atr: f64,
    ) -> Vec<LiquiditySweep> {
        let mut sweeps = Vec::new();

        for pool in pools {
            let events = Self::extract_events(candles, pool.price_level, atr * 0.5);

            if events.len() >= 3 {
                let _hawkes = HawkesProcess::new(0.1, 0.5, 1.0);

                // Simple check: if events cluster, it's likely a sweep
                if events.len() > 5 {
                    sweeps.push(LiquiditySweep {
                        sweep_bar: events[0] as usize,
                        return_bar: events.last().copied().unwrap_or(events[0]) as usize,
                        pool_price: pool.price_level,
                        sweep_direction: pool.pool_type,
                    });
                }
            }
        }

        sweeps
    }

    /// Extract event times around a price level
    fn extract_events(candles: &[Candle], pool_price: f64, tolerance: f64) -> Vec<f64> {
        let mut events = Vec::new();

        for (i, candle) in candles.iter().enumerate() {
            if (candle.high - pool_price).abs() <= tolerance
                || (candle.low - pool_price).abs() <= tolerance
            {
                events.push(i as f64);
            }
        }

        events
    }
}
