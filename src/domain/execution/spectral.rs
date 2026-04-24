use serde::{Deserialize, Serialize};

use crate::math::spectral::{
    dominant_energy_ratio, dominant_mode, dominant_phase_alignment, high_frequency_noise_ratio,
    normalized_spectral_entropy, rfft_one_sided, softshrink_bins,
};

/// Minimum sample count before the estimator is allowed to fit. Below this
/// the dominant bin is dominated by aliasing and entropy is saturated.
pub const SPECTRAL_MIN_SAMPLES: usize = 32;

/// Default softshrink threshold as a fraction of the largest bin magnitude.
/// AFNO uses a fixed lambda because the input is pre-normalized; price series
/// are not, so we scale to the spectrum's peak. 0.05 prunes bottom-5% bins
/// relative to the dominant mode without collapsing the signal.
pub const SPECTRAL_DEFAULT_LAMBDA_RATIO: f64 = 0.05;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpectralExecutionMetrics {
    pub dominant_cycle_energy: f64, // fraction of non-DC energy in top mode, [0, 1]
    pub dominant_cycle_period_bars: f64, // length of dominant cycle in bars
    pub cycle_phase_alignment: f64, // cos(phase at latest sample), [-1, 1]
    pub spectral_entropy: f64,      // normalized Shannon entropy, [0, 1]
    pub high_freq_noise_ratio: f64, // energy pruned by softshrink, [0, 1]
    pub softshrink_lambda: f64,     // threshold used (absolute magnitude)
    pub sample_count: usize,        // samples fed to the FFT
    pub padded_length: usize,       // zero-padded FFT length
}

fn next_power_of_two(n: usize) -> usize {
    if n <= 1 {
        return 1;
    }
    let mut p = 1;
    while p < n {
        p <<= 1;
    }
    p
}

// Fits the spectral execution layer. Returns None when the window is too
// short or the signal is degenerate (zero AC energy) — callers should treat
// that as "spectral layer not applicable" rather than zeroed metrics, mirroring
// the OU estimator's None policy so downstream gates don't see misleading
// defaults.
pub fn estimate_spectral_execution_metrics(
    prices: &[f64],
    lambda_ratio: f64,
) -> Option<SpectralExecutionMetrics> {
    if prices.len() < SPECTRAL_MIN_SAMPLES {
        return None;
    }
    if prices.iter().any(|value| !value.is_finite()) {
        return None;
    }

    let bins = rfft_one_sided(prices);
    if bins.is_empty() {
        return None;
    }
    let padded_length = next_power_of_two(prices.len());

    let total_energy: f64 = bins.iter().skip(1).map(|bin| bin.norm_sq()).sum();
    if total_energy <= 0.0 {
        return None;
    }

    let dominant = dominant_mode(&bins, padded_length)?;
    let peak_magnitude = dominant.energy.sqrt();
    let lambda = (peak_magnitude * lambda_ratio.max(0.0)).max(0.0);

    let dominant_energy = dominant_energy_ratio(&bins);
    let spectral_entropy = normalized_spectral_entropy(&bins);
    let cycle_phase_alignment = dominant_phase_alignment(dominant, prices.len(), padded_length);

    let mut pruned_bins = bins.clone();
    softshrink_bins(&mut pruned_bins, lambda);
    let high_freq_noise_ratio = high_frequency_noise_ratio(&bins, &pruned_bins);

    Some(SpectralExecutionMetrics {
        dominant_cycle_energy: dominant_energy.clamp(0.0, 1.0),
        dominant_cycle_period_bars: dominant.period_bars,
        cycle_phase_alignment,
        spectral_entropy,
        high_freq_noise_ratio,
        softshrink_lambda: lambda,
        sample_count: prices.len(),
        padded_length,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn rejects_short_windows() {
        let prices: Vec<f64> = (0..16).map(|i| i as f64).collect();
        assert!(
            estimate_spectral_execution_metrics(&prices, SPECTRAL_DEFAULT_LAMBDA_RATIO).is_none()
        );
    }

    #[test]
    fn rejects_non_finite_values() {
        let mut prices: Vec<f64> = (0..64).map(|i| (i as f64 * 0.1).sin()).collect();
        prices[10] = f64::NAN;
        assert!(
            estimate_spectral_execution_metrics(&prices, SPECTRAL_DEFAULT_LAMBDA_RATIO).is_none()
        );
    }

    #[test]
    fn rejects_constant_series() {
        let prices = vec![100.0_f64; 128];
        assert!(
            estimate_spectral_execution_metrics(&prices, SPECTRAL_DEFAULT_LAMBDA_RATIO).is_none()
        );
    }

    #[test]
    fn rhythmic_series_yields_high_dominant_energy_and_low_entropy() {
        let n = 256usize;
        let cycles = 8.0_f64;
        let prices: Vec<f64> = (0..n)
            .map(|i| 100.0 + (2.0 * PI * cycles * i as f64 / n as f64).sin() * 2.0)
            .collect();
        let metrics = estimate_spectral_execution_metrics(&prices, SPECTRAL_DEFAULT_LAMBDA_RATIO)
            .expect("spectral metrics should fit");
        assert!(
            metrics.dominant_cycle_energy > 0.9,
            "dominant_cycle_energy={}",
            metrics.dominant_cycle_energy
        );
        assert!(
            metrics.spectral_entropy < 0.2,
            "spectral_entropy={}",
            metrics.spectral_entropy
        );
        assert!(
            (metrics.dominant_cycle_period_bars - 32.0).abs() < 1.0,
            "period={}",
            metrics.dominant_cycle_period_bars
        );
        assert!(metrics.cycle_phase_alignment >= -1.0 && metrics.cycle_phase_alignment <= 1.0);
    }

    #[test]
    fn noisy_series_yields_high_entropy_and_low_dominance() {
        let n = 256usize;
        let mut state: u64 = 0xC0FFEE1234567890;
        let prices: Vec<f64> = (0..n)
            .map(|_| {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                100.0 + (((state >> 33) as f64) / (u32::MAX as f64) * 2.0 - 1.0) * 0.5
            })
            .collect();
        let metrics = estimate_spectral_execution_metrics(&prices, SPECTRAL_DEFAULT_LAMBDA_RATIO)
            .expect("spectral metrics should fit");
        assert!(
            metrics.spectral_entropy > 0.7,
            "spectral_entropy={}",
            metrics.spectral_entropy
        );
        assert!(
            metrics.dominant_cycle_energy < 0.3,
            "dominant_cycle_energy={}",
            metrics.dominant_cycle_energy
        );
    }
}
