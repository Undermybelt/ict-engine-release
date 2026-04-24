use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OuExecutionMetrics {
    pub half_life_bars: f64,
    pub reversion_speed_per_bar: f64,
    pub pullback_expectation_zscore: f64,
    pub overextension_distance: f64,
    pub expected_pullback_bps: f64,
}

pub fn build_ou_execution_metrics(
    normalized_distance_to_projected_trend_bps: f64,
    ou_half_life_bars: f64,
    ou_reversion_speed_per_bar: f64,
    ou_pullback_expectation_zscore: f64,
    ou_expected_pullback_bps: f64,
) -> OuExecutionMetrics {
    let overextension_distance =
        (normalized_distance_to_projected_trend_bps.abs() / 10_000.0).clamp(0.0, 1.0);
    OuExecutionMetrics {
        half_life_bars: ou_half_life_bars,
        reversion_speed_per_bar: ou_reversion_speed_per_bar,
        pullback_expectation_zscore: ou_pullback_expectation_zscore,
        overextension_distance,
        expected_pullback_bps: ou_expected_pullback_bps,
    }
}

const OU_MIN_SAMPLES: usize = 20;

// Fit a local OU process to (prices, timestamps) via AR(1) regression on the
// raw series: X_{t+1} = alpha + phi * X_t + eps. Because
// phi = exp(-theta * dt), a per-bar reversion speed is theta_per_bar = -ln(phi)
// (well-defined only when phi in (0, 1), i.e. the series is mean-reverting).
// Returns None when the series is too short, non-stationary, or degenerate —
// callers should treat that as "OU not applicable" rather than zeroed metrics.
pub fn estimate_ou_execution_metrics(
    prices: &[f64],
    timestamps: &[DateTime<Utc>],
) -> Option<OuExecutionMetrics> {
    if prices.len() != timestamps.len() || prices.len() < OU_MIN_SAMPLES {
        return None;
    }
    if prices.iter().any(|p| !p.is_finite()) {
        return None;
    }

    // Use the median bar interval so irregular sampling does not skew theta.
    let mut intervals: Vec<f64> = timestamps
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).num_milliseconds() as f64 / 1_000.0)
        .filter(|value| *value > 0.0)
        .collect();
    if intervals.is_empty() {
        return None;
    }
    intervals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_dt_seconds = intervals[intervals.len() / 2];
    if median_dt_seconds <= 0.0 {
        return None;
    }

    let n = prices.len();
    let mean_level = prices.iter().sum::<f64>() / n as f64;
    if !mean_level.is_finite() || mean_level.abs() < f64::EPSILON {
        return None;
    }
    let demeaned: Vec<f64> = prices.iter().map(|value| value - mean_level).collect();

    let mut lag_num = 0.0_f64;
    let mut lag_den = 0.0_f64;
    for i in 0..(n - 1) {
        lag_num += demeaned[i + 1] * demeaned[i];
        lag_den += demeaned[i] * demeaned[i];
    }
    if lag_den <= 0.0 {
        return None;
    }
    let phi = lag_num / lag_den;
    if phi <= 0.0 || phi >= 1.0 {
        return None;
    }

    let mut residual_sq = 0.0_f64;
    for i in 0..(n - 1) {
        let residual = demeaned[i + 1] - phi * demeaned[i];
        residual_sq += residual * residual;
    }
    let residual_dof = (n - 2).max(1) as f64;
    let residual_variance = residual_sq / residual_dof;
    if residual_variance <= 0.0 {
        return None;
    }
    let phi_sq_gap = (1.0_f64 - phi * phi).max(f64::EPSILON);
    let unconditional_sd = (residual_variance / phi_sq_gap).sqrt();
    if unconditional_sd <= 0.0 {
        return None;
    }

    let reversion_speed_per_bar = -phi.ln();
    let half_life_bars = if reversion_speed_per_bar > 0.0 {
        std::f64::consts::LN_2 / reversion_speed_per_bar
    } else {
        f64::INFINITY
    };
    // Refuse the fit if the window is shorter than ~2 half-lives: an OU
    // estimate over a span that hasn't cycled through reversion is dominated
    // by drift and will report arbitrarily slow reversion (e.g. pure trends).
    if !(half_life_bars.is_finite() && half_life_bars > 0.0 && half_life_bars * 2.0 <= n as f64) {
        return None;
    }

    let last = *prices.last()?;
    let deviation = last - mean_level;
    let pullback_expectation_zscore = deviation / unconditional_sd;
    // Saturate at |z| = 3: beyond that, overextension is effectively maxed.
    let overextension_distance = (pullback_expectation_zscore.abs() / 3.0).clamp(0.0, 1.0);
    let expected_pullback_bps = (reversion_speed_per_bar * deviation / last).abs() * 10_000.0;

    Some(OuExecutionMetrics {
        half_life_bars,
        reversion_speed_per_bar,
        pullback_expectation_zscore,
        overextension_distance,
        expected_pullback_bps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn stamps(n: usize, step_seconds: i64) -> Vec<DateTime<Utc>> {
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        (0..n)
            .map(|i| base + Duration::seconds(step_seconds * i as i64))
            .collect()
    }

    #[test]
    fn returns_none_when_inputs_are_short_or_mismatched() {
        assert!(estimate_ou_execution_metrics(&[1.0; 5], &stamps(5, 60)).is_none());
        let prices = vec![100.0; 30];
        let too_few_stamps = stamps(10, 60);
        assert!(estimate_ou_execution_metrics(&prices, &too_few_stamps).is_none());
    }

    #[test]
    fn returns_none_for_constant_series() {
        let prices = vec![100.0_f64; 64];
        let timestamps = stamps(64, 60);
        assert!(estimate_ou_execution_metrics(&prices, &timestamps).is_none());
    }

    #[test]
    fn fits_mean_reverting_series_and_reports_finite_metrics() {
        // Synthetic AR(1) around mu = 100 with phi = 0.8, driven by white-ish
        // LCG noise so the estimator can recover phi without fighting noise
        // autocorrelation. The estimator should land near phi ~ 0.8 →
        // theta ~ 0.223 → half-life ~ 3.1 bars.
        let mu = 100.0_f64;
        let phi_true = 0.8_f64;
        let mut rng_state: u64 = 0x9E3779B97F4A7C15;
        let mut next_unit = || {
            // Numerical Recipes LCG → uniform in [-1, 1]
            rng_state = rng_state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((rng_state >> 33) as f64) / (u32::MAX as f64) * 2.0 - 1.0
        };
        let mut prices = Vec::with_capacity(400);
        let mut value = mu;
        for _ in 0..400 {
            value = mu + phi_true * (value - mu) + next_unit() * 1.0;
            prices.push(value);
        }
        let timestamps = stamps(prices.len(), 60);
        let metrics =
            estimate_ou_execution_metrics(&prices, &timestamps).expect("fit should succeed");
        assert!(metrics.reversion_speed_per_bar > 0.0);
        assert!(metrics.half_life_bars.is_finite() && metrics.half_life_bars > 0.0);
        assert!(metrics.overextension_distance >= 0.0 && metrics.overextension_distance <= 1.0);
        assert!(metrics.expected_pullback_bps >= 0.0);
        assert!(
            metrics.half_life_bars > 1.5 && metrics.half_life_bars < 8.0,
            "half_life_bars={} out of band",
            metrics.half_life_bars
        );
    }

    #[test]
    fn returns_none_for_trending_series() {
        // Pure upward drift is not mean-reverting; phi comes out >= 1 and we
        // must refuse to fit rather than emit bogus OU parameters.
        let prices: Vec<f64> = (0..100).map(|i| 100.0 + i as f64 * 0.5).collect();
        let timestamps = stamps(prices.len(), 60);
        assert!(estimate_ou_execution_metrics(&prices, &timestamps).is_none());
    }
}
