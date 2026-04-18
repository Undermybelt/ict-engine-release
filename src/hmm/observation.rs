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
        let recent_fvgs = fvgs
            .iter()
            .filter(|f| f.start_bar >= i.saturating_sub(10))
            .count();
        obs.push(recent_fvgs as f64);

        // 8. Sweep count (recent)
        let recent_sweeps = sweeps
            .iter()
            .filter(|s| s.sweep_bar >= i.saturating_sub(10))
            .count();
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

        observations.push(obs);
    }

    observations
}
