//! Smoke tests for the spectral execution layer.
//!
//! Locks the fixtures used by Sprint 2 acceptance — rhythmic series must show
//! a concentrated dominant mode and low entropy; noisy series must show the
//! inverse. These fixtures double as the regression surface for the math
//! layer: if the FFT / softshrink implementation regresses, these expectations
//! will catch it before the overlay changes behaviour.

use std::f64::consts::PI;

use ict_engine::domain::execution::{
    estimate_spectral_execution_metrics, SPECTRAL_DEFAULT_LAMBDA_RATIO,
};

fn rhythmic_prices(n: usize, cycles: f64) -> Vec<f64> {
    (0..n)
        .map(|i| 100.0 + (2.0 * PI * cycles * i as f64 / n as f64).sin() * 1.5)
        .collect()
}

fn deterministic_noise(n: usize, seed: u64) -> Vec<f64> {
    let mut state = seed;
    (0..n)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            100.0 + (((state >> 33) as f64) / (u32::MAX as f64) * 2.0 - 1.0) * 0.8
        })
        .collect()
}

#[test]
fn rhythmic_series_reports_concentrated_dominant_mode() {
    let prices = rhythmic_prices(256, 8.0);
    let metrics = estimate_spectral_execution_metrics(&prices, SPECTRAL_DEFAULT_LAMBDA_RATIO)
        .expect("rhythmic series should yield spectral metrics");

    assert!(
        metrics.dominant_cycle_energy > 0.90,
        "dominant_cycle_energy={}",
        metrics.dominant_cycle_energy
    );
    assert!(
        metrics.spectral_entropy < 0.20,
        "spectral_entropy={}",
        metrics.spectral_entropy
    );
    assert!(
        (metrics.dominant_cycle_period_bars - 32.0).abs() < 1.0,
        "period={}",
        metrics.dominant_cycle_period_bars
    );
    assert!(
        metrics.cycle_phase_alignment.abs() <= 1.0,
        "phase_alignment={}",
        metrics.cycle_phase_alignment
    );
}

#[test]
fn noisy_series_reports_high_entropy_and_low_dominance() {
    let prices = deterministic_noise(256, 0xA5A5A5A5A5A5A5A5);
    let metrics = estimate_spectral_execution_metrics(&prices, SPECTRAL_DEFAULT_LAMBDA_RATIO)
        .expect("noisy series should still yield spectral metrics");

    assert!(
        metrics.spectral_entropy > 0.70,
        "spectral_entropy={}",
        metrics.spectral_entropy
    );
    assert!(
        metrics.dominant_cycle_energy < 0.25,
        "dominant_cycle_energy={}",
        metrics.dominant_cycle_energy
    );
}

#[test]
fn short_window_returns_none() {
    let prices = rhythmic_prices(16, 2.0);
    assert!(estimate_spectral_execution_metrics(&prices, SPECTRAL_DEFAULT_LAMBDA_RATIO).is_none());
}

#[test]
fn constant_series_returns_none() {
    let prices = vec![100.0_f64; 128];
    assert!(estimate_spectral_execution_metrics(&prices, SPECTRAL_DEFAULT_LAMBDA_RATIO).is_none());
}

#[test]
fn softshrink_prunes_some_energy_for_mixed_signal() {
    // Strong 32-bar cycle + weak 4-bar cycle. With lambda_ratio=0.05, weak
    // high-frequency bins should be pruned; high_freq_noise_ratio must be
    // positive but less than 1.0.
    let n = 256;
    let prices: Vec<f64> = (0..n)
        .map(|i| {
            100.0
                + (2.0 * PI * i as f64 / 32.0).sin() * 1.0
                + (2.0 * PI * i as f64 / 4.0).sin() * 0.06
        })
        .collect();
    let metrics = estimate_spectral_execution_metrics(&prices, SPECTRAL_DEFAULT_LAMBDA_RATIO)
        .expect("mixed signal yields metrics");
    assert!(
        metrics.high_freq_noise_ratio > 0.0 && metrics.high_freq_noise_ratio < 1.0,
        "high_freq_noise_ratio={}",
        metrics.high_freq_noise_ratio
    );
    assert!(metrics.softshrink_lambda > 0.0);
}
