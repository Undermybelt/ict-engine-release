use serde::{Deserialize, Serialize};

use crate::application::orchestration::PipelineState;
use crate::domain::execution::{
    SpectralExecutionMetrics, DOMINANT_ENERGY_FLOOR, EXECUTION_GATE_OBSERVE,
    SPECTRAL_ENTROPY_CHAOS_CAP,
};

/// Snapshot of the spectral overlay's execution-relevant decisions. Lives on
/// `PipelineState` so downstream stages (reflection, trace builder) read from
/// one source rather than re-deriving from the raw metrics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpectralOverlayState {
    pub dominant_cycle_energy: f64,
    pub dominant_cycle_period_bars: f64,
    pub cycle_phase_alignment: f64,
    pub spectral_entropy: f64,
    pub high_freq_noise_ratio: f64,
    /// True when the series is too chaotic to execute against (entropy > cap
    /// AND dominant energy < floor). Consumed by the readiness penalty.
    pub chaotic_regime: bool,
    /// True when readiness has cleared the observe threshold, so spectral
    /// evidence is allowed to feed attribution surfaces. Below observe we
    /// record metrics but don't let them influence explanation.
    pub attribution_influence_enabled: bool,
}

pub fn apply_spectral_overlay(
    pipeline_state: &mut PipelineState,
    metrics: &SpectralExecutionMetrics,
    execution_readiness: f64,
) -> SpectralOverlayState {
    let chaotic_regime = metrics.spectral_entropy > SPECTRAL_ENTROPY_CHAOS_CAP
        && metrics.dominant_cycle_energy < DOMINANT_ENERGY_FLOOR;
    let overlay = SpectralOverlayState {
        dominant_cycle_energy: metrics.dominant_cycle_energy,
        dominant_cycle_period_bars: metrics.dominant_cycle_period_bars,
        cycle_phase_alignment: metrics.cycle_phase_alignment,
        spectral_entropy: metrics.spectral_entropy,
        high_freq_noise_ratio: metrics.high_freq_noise_ratio,
        chaotic_regime,
        attribution_influence_enabled: execution_readiness >= EXECUTION_GATE_OBSERVE,
    };
    pipeline_state.spectral_overlay = Some(overlay.clone());
    overlay
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rhythmic_metrics() -> SpectralExecutionMetrics {
        SpectralExecutionMetrics {
            dominant_cycle_energy: 0.92,
            dominant_cycle_period_bars: 32.0,
            cycle_phase_alignment: 0.65,
            spectral_entropy: 0.10,
            high_freq_noise_ratio: 0.05,
            softshrink_lambda: 0.02,
            sample_count: 256,
            padded_length: 256,
        }
    }

    fn chaotic_metrics() -> SpectralExecutionMetrics {
        SpectralExecutionMetrics {
            dominant_cycle_energy: 0.05,
            dominant_cycle_period_bars: 4.0,
            cycle_phase_alignment: 0.0,
            spectral_entropy: 0.95,
            high_freq_noise_ratio: 0.80,
            softshrink_lambda: 0.20,
            sample_count: 256,
            padded_length: 256,
        }
    }

    #[test]
    fn rhythmic_series_is_not_flagged_chaotic() {
        let mut state = PipelineState::new("NQ", Some("NQ"), "test");
        let overlay = apply_spectral_overlay(&mut state, &rhythmic_metrics(), 0.80);
        assert!(!overlay.chaotic_regime);
        assert!(overlay.attribution_influence_enabled);
        assert!(state.spectral_overlay.is_some());
    }

    #[test]
    fn chaotic_series_trips_the_chaotic_flag() {
        let mut state = PipelineState::new("NQ", Some("NQ"), "test");
        let overlay = apply_spectral_overlay(&mut state, &chaotic_metrics(), 0.70);
        assert!(overlay.chaotic_regime);
    }

    #[test]
    fn below_observe_gate_disables_attribution_influence() {
        let mut state = PipelineState::new("NQ", Some("NQ"), "test");
        let overlay = apply_spectral_overlay(&mut state, &rhythmic_metrics(), 0.30);
        assert!(!overlay.attribution_influence_enabled);
    }
}
