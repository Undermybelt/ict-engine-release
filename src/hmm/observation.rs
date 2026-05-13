use crate::types::{Candle, FairValueGap, LiquiditySweep, OBS_DIM};

#[derive(Debug, Clone)]
pub struct ObservationInput<'a> {
    pub candles: &'a [Candle],
    pub ltf_candles: &'a [Candle],
    pub implied_vol: &'a [f64],
    pub smoothed_prices: &'a [(f64, f64, f64)],
    pub atr: &'a [f64],
    pub rsi: &'a [f64],
    pub adx: &'a [f64],
    pub fvgs: &'a [FairValueGap],
    pub sweeps: &'a [LiquiditySweep],
}

/// Build observation vectors for HMM
pub fn build_observations(input: ObservationInput<'_>) -> Vec<Vec<f64>> {
    let ObservationInput {
        candles,
        ltf_candles: _ltf_candles,
        implied_vol,
        smoothed_prices,
        atr,
        rsi,
        adx,
        fvgs,
        sweeps,
    } = input;
    let mut observations = Vec::new();
    let min_len = candles.len().min(atr.len()).min(rsi.len()).min(adx.len());
    let mut fvg_bars = fvgs.iter().map(|fvg| fvg.start_bar).collect::<Vec<_>>();
    let mut sweep_bars = sweeps
        .iter()
        .map(|sweep| sweep.sweep_bar)
        .collect::<Vec<_>>();
    fvg_bars.sort_unstable();
    sweep_bars.sort_unstable();
    let mut fvg_window_start = 0usize;
    let mut fvg_window_end = 0usize;
    let mut sweep_window_start = 0usize;
    let mut sweep_window_end = 0usize;

    for i in 0..min_len {
        let mut obs = Vec::with_capacity(OBS_DIM);

        // 1. Normalized return
        let ret = if i > 0 {
            (candles[i].close - candles[i - 1].close) / candles[i - 1].close
        } else {
            0.0
        };
        obs.push(ret);

        // 2. ATR ratio (current ATR / average ATR)
        let atr_ratio = if i >= 20 {
            let avg_atr: f64 = atr[i.saturating_sub(20)..=i].iter().sum::<f64>() / 20.0;
            atr[i] / avg_atr.max(1e-10)
        } else {
            1.0
        };
        obs.push(atr_ratio);

        // 3. RSI (normalized to 0-1)
        obs.push(rsi[i] / 100.0);

        // 4. ADX (normalized)
        obs.push(adx[i].min(100.0) / 100.0);

        // 5. Kalman velocity (trend)
        if i < smoothed_prices.len() {
            obs.push(smoothed_prices[i].1); // velocity
        } else {
            obs.push(0.0);
        }

        // 6. Implied volatility
        if i < implied_vol.len() {
            obs.push(implied_vol[i]);
        } else {
            obs.push(0.0);
        }

        // 7. FVG count (recent)
        let recent_start = i.saturating_sub(10);
        while fvg_window_end < fvg_bars.len() && fvg_bars[fvg_window_end] <= i {
            fvg_window_end += 1;
        }
        while fvg_window_start < fvg_window_end && fvg_bars[fvg_window_start] < recent_start {
            fvg_window_start += 1;
        }
        let recent_fvgs = fvg_window_end.saturating_sub(fvg_window_start);
        obs.push(recent_fvgs as f64);

        // 8. Sweep count (recent)
        while sweep_window_end < sweep_bars.len() && sweep_bars[sweep_window_end] <= i {
            sweep_window_end += 1;
        }
        while sweep_window_start < sweep_window_end && sweep_bars[sweep_window_start] < recent_start
        {
            sweep_window_start += 1;
        }
        let recent_sweeps = sweep_window_end.saturating_sub(sweep_window_start);
        obs.push(recent_sweeps as f64);

        // 9. Price position in range (0 = low, 1 = high)
        let range = candles[i].high - candles[i].low;
        let pos = if range > 0.0 {
            (candles[i].close - candles[i].low) / range
        } else {
            0.5
        };
        obs.push(pos);

        // 10. Volume ratio
        let avg_vol = if i >= 20 {
            candles[i.saturating_sub(20)..=i]
                .iter()
                .map(|c| c.volume)
                .sum::<f64>()
                / 20.0
        } else {
            candles[i].volume
        };
        let vol_ratio = candles[i].volume / avg_vol.max(1e-10);
        obs.push(vol_ratio);

        // 11. Body ratio
        let body_ratio = candles[i].body() / candles[i].range().max(1e-10);
        obs.push(body_ratio);

        // 12. Momentum (price change over 5 bars)
        if i >= 5 {
            let mom = (candles[i].close - candles[i - 5].close) / candles[i - 5].close;
            obs.push(mom);
        } else {
            obs.push(0.0);
        }

        // Keep observation width aligned with the configured HMM emission width.
        while obs.len() < OBS_DIM {
            obs.push(0.0);
        }

        observations.push(obs);
    }

    observations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Direction;
    use chrono::{Duration, TimeZone, Utc};

    fn candle(idx: i64, open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            timestamp: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap() + Duration::minutes(idx),
            open,
            high,
            low,
            close,
            volume: 1_000.0 + idx as f64,
        }
    }

    #[test]
    fn build_observations_pads_vectors_to_obs_dim() {
        let candles = (0..30)
            .map(|idx| {
                let base = 100.0 + idx as f64 * 0.2;
                candle(idx as i64, base, base + 0.4, base - 0.3, base + 0.1)
            })
            .collect::<Vec<_>>();
        let observations = build_observations(ObservationInput {
            candles: &candles,
            ltf_candles: &candles,
            implied_vol: &vec![0.2; candles.len()],
            smoothed_prices: &vec![(0.0, 0.1, 0.0); candles.len()],
            atr: &vec![1.0; candles.len()],
            rsi: &vec![55.0; candles.len()],
            adx: &vec![25.0; candles.len()],
            fvgs: &[],
            sweeps: &[],
        });

        assert!(!observations.is_empty());
        assert!(observations.iter().all(|row| row.len() == OBS_DIM));
    }

    #[test]
    fn build_observations_counts_only_completed_recent_events() {
        let candles = (0..30)
            .map(|idx| {
                let base = 100.0 + idx as f64;
                candle(idx as i64, base, base + 0.4, base - 0.3, base + 0.1)
            })
            .collect::<Vec<_>>();
        let fvgs = vec![
            FairValueGap {
                top: 101.0,
                bottom: 100.0,
                direction: Direction::Bull,
                start_bar: 5,
                filled: false,
            },
            FairValueGap {
                top: 106.0,
                bottom: 105.0,
                direction: Direction::Bull,
                start_bar: 10,
                filled: false,
            },
            FairValueGap {
                top: 121.0,
                bottom: 120.0,
                direction: Direction::Bull,
                start_bar: 25,
                filled: false,
            },
        ];
        let sweeps = vec![
            LiquiditySweep {
                sweep_bar: 8,
                return_bar: 9,
                pool_price: 108.0,
                sweep_direction: Direction::Bear,
            },
            LiquiditySweep {
                sweep_bar: 22,
                return_bar: 23,
                pool_price: 122.0,
                sweep_direction: Direction::Bear,
            },
        ];

        let observations = build_observations(ObservationInput {
            candles: &candles,
            ltf_candles: &candles,
            implied_vol: &vec![0.2; candles.len()],
            smoothed_prices: &vec![(0.0, 0.1, 0.0); candles.len()],
            atr: &vec![1.0; candles.len()],
            rsi: &vec![55.0; candles.len()],
            adx: &vec![25.0; candles.len()],
            fvgs: &fvgs,
            sweeps: &sweeps,
        });

        assert_eq!(observations[0][6], 0.0);
        assert_eq!(observations[0][7], 0.0);
        assert_eq!(observations[10][6], 2.0);
        assert_eq!(observations[10][7], 1.0);
        assert_eq!(observations[25][6], 1.0);
        assert_eq!(observations[25][7], 1.0);
    }
}
