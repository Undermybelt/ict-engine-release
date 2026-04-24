use crate::domain::execution::SpectralExecutionMetrics;

pub const EXECUTION_GATE_READY: f64 = 0.65;
pub const EXECUTION_GATE_OBSERVE: f64 = 0.45;

/// Upper bound on normalized spectral entropy before we consider the series
/// "chaotic" (rhythmic structure too weak to execute against). Calibrated so
/// rhythmic sine → entropy ~0 and white noise → entropy ~1; anything above
/// 0.80 is dominated by broadband energy.
pub const SPECTRAL_ENTROPY_CHAOS_CAP: f64 = 0.80;

/// Lower bound on dominant cycle energy fraction. Below this the "dominant"
/// mode is not meaningfully dominant; phase alignment becomes noise.
pub const DOMINANT_ENERGY_FLOOR: f64 = 0.15;

/// Multiplicative readiness penalty when both spectral thresholds fail.
/// 0.7 = "cut 30% of readiness budget" — enough to drop a marginal ready run
/// to observe_only, but not enough to invalidate a strongly-ready run.
pub const SPECTRAL_READINESS_PENALTY: f64 = 0.70;

pub fn classify_execution_gate(readiness: f64) -> &'static str {
    if readiness >= EXECUTION_GATE_READY {
        "execution_ready"
    } else if readiness >= EXECUTION_GATE_OBSERVE {
        "execution_observe_only"
    } else {
        "execution_blocked"
    }
}

/// Apply the spectral hard-gate penalty. The rule is conjunctive: high entropy
/// AND low dominant energy together mean the signal has no rhythmic structure
/// we can time against, so we shrink readiness. Either condition alone leaves
/// readiness untouched — a strongly rhythmic series at high entropy (multiple
/// concurrent cycles) is still executable, and a clean-but-weak dominant mode
/// can still align phase.
pub fn apply_spectral_execution_penalty(
    readiness: f64,
    spectral: Option<&SpectralExecutionMetrics>,
) -> f64 {
    let Some(metrics) = spectral else {
        return readiness;
    };
    let chaotic = metrics.spectral_entropy > SPECTRAL_ENTROPY_CHAOS_CAP
        && metrics.dominant_cycle_energy < DOMINANT_ENERGY_FLOOR;
    if chaotic {
        (readiness * SPECTRAL_READINESS_PENALTY).clamp(0.0, 1.0)
    } else {
        readiness
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_execution_gates_from_shared_thresholds() {
        assert_eq!(
            classify_execution_gate(EXECUTION_GATE_READY),
            "execution_ready"
        );
        assert_eq!(
            classify_execution_gate(EXECUTION_GATE_OBSERVE),
            "execution_observe_only"
        );
        assert_eq!(
            classify_execution_gate(EXECUTION_GATE_OBSERVE - 0.01),
            "execution_blocked"
        );
    }

    fn metrics(entropy: f64, energy: f64) -> SpectralExecutionMetrics {
        SpectralExecutionMetrics {
            dominant_cycle_energy: energy,
            dominant_cycle_period_bars: 16.0,
            cycle_phase_alignment: 0.0,
            spectral_entropy: entropy,
            high_freq_noise_ratio: 0.0,
            softshrink_lambda: 0.0,
            sample_count: 128,
            padded_length: 128,
        }
    }

    #[test]
    fn spectral_penalty_is_noop_when_structure_is_present() {
        let rhythmic = metrics(0.10, 0.90);
        assert_eq!(
            apply_spectral_execution_penalty(0.80, Some(&rhythmic)),
            0.80
        );
    }

    #[test]
    fn spectral_penalty_is_noop_when_only_one_condition_fails() {
        let high_entropy_strong_mode = metrics(0.95, 0.50);
        let low_entropy_weak_mode = metrics(0.40, 0.05);
        assert_eq!(
            apply_spectral_execution_penalty(0.80, Some(&high_entropy_strong_mode)),
            0.80
        );
        assert_eq!(
            apply_spectral_execution_penalty(0.80, Some(&low_entropy_weak_mode)),
            0.80
        );
    }

    #[test]
    fn spectral_penalty_shrinks_readiness_when_both_conditions_fail() {
        let chaotic = metrics(0.95, 0.05);
        let penalized = apply_spectral_execution_penalty(0.80, Some(&chaotic));
        assert!((penalized - 0.56).abs() < 1e-9, "penalized={penalized}");
    }

    #[test]
    fn spectral_penalty_is_noop_when_metrics_are_absent() {
        assert_eq!(apply_spectral_execution_penalty(0.75, None), 0.75);
    }
}
