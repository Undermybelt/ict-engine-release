//! Hard-gate regression for the spectral execution penalty.
//!
//! Sprint 2 acceptance: a chaotic series (high entropy + low dominant energy)
//! must cause readiness to shrink through the spectral penalty path.
//! A rhythmic series must leave readiness untouched. The overlay struct
//! written to PipelineState must also record the chaotic flag and the
//! observe-gate toggle for attribution influence.

use ict_engine::application::belief::apply_spectral_overlay;
use ict_engine::application::execution::{
    build_execution_artifact_from_snapshot, ExecutionArtifactBuildContext, ExecutionInputSnapshot,
};
use ict_engine::application::orchestration::PipelineState;
use ict_engine::domain::execution::{
    apply_spectral_execution_penalty, SpectralExecutionMetrics, SPECTRAL_READINESS_PENALTY,
};
use ict_engine::state::RunProvenance;

fn rhythmic_metrics() -> SpectralExecutionMetrics {
    SpectralExecutionMetrics {
        dominant_cycle_energy: 0.92,
        dominant_cycle_period_bars: 32.0,
        cycle_phase_alignment: 0.55,
        spectral_entropy: 0.12,
        high_freq_noise_ratio: 0.08,
        softshrink_lambda: 0.02,
        sample_count: 256,
        padded_length: 256,
    }
}

fn chaotic_metrics() -> SpectralExecutionMetrics {
    SpectralExecutionMetrics {
        dominant_cycle_energy: 0.04,
        dominant_cycle_period_bars: 3.0,
        cycle_phase_alignment: 0.0,
        spectral_entropy: 0.93,
        high_freq_noise_ratio: 0.78,
        softshrink_lambda: 0.18,
        sample_count: 256,
        padded_length: 256,
    }
}

#[test]
fn chaotic_series_shrinks_readiness_by_penalty_factor() {
    let baseline = 0.80_f64;
    let chaotic = chaotic_metrics();
    let penalized = apply_spectral_execution_penalty(baseline, Some(&chaotic));
    let expected = baseline * SPECTRAL_READINESS_PENALTY;
    assert!(
        (penalized - expected).abs() < 1e-9,
        "penalized={penalized} expected={expected}"
    );
}

#[test]
fn rhythmic_series_leaves_readiness_untouched() {
    let baseline = 0.80_f64;
    let rhythmic = rhythmic_metrics();
    let penalized = apply_spectral_execution_penalty(baseline, Some(&rhythmic));
    assert!(
        (penalized - baseline).abs() < 1e-9,
        "penalized={penalized} baseline={baseline}"
    );
}

#[test]
fn absent_spectral_metrics_never_penalize() {
    let baseline = 0.72_f64;
    let penalized = apply_spectral_execution_penalty(baseline, None);
    assert_eq!(penalized, baseline);
}

#[test]
fn overlay_state_records_chaotic_flag_and_attribution_toggle() {
    let mut state = PipelineState::new("NQ", Some("NQ"), "test");
    let overlay_ready = apply_spectral_overlay(&mut state, &chaotic_metrics(), 0.70);
    assert!(overlay_ready.chaotic_regime);
    assert!(overlay_ready.attribution_influence_enabled);

    let mut state_low = PipelineState::new("NQ", Some("NQ"), "test");
    let overlay_low = apply_spectral_overlay(&mut state_low, &rhythmic_metrics(), 0.30);
    assert!(!overlay_low.chaotic_regime);
    assert!(
        !overlay_low.attribution_influence_enabled,
        "below observe gate, attribution influence must be disabled"
    );
}

#[test]
fn artifact_readiness_downgrades_when_physics_overlay_carries_chaotic_spectral() {
    let snapshot = ExecutionInputSnapshot {
        aggression_bias: 0.2,
        completion_pressure: 0.9,
        liquidity_absorption_bias: 0.8,
        evidence_quality: 0.9,
        prediction_score: 0.3,
    };
    let provenance = RunProvenance {
        data_fingerprint: "fp".to_string(),
        ..RunProvenance::default()
    };

    let chaotic_overlay = ict_engine::application::execution::ExecutionPhysicsOverlay {
        spectral: Some(chaotic_metrics()),
        ..ict_engine::application::execution::ExecutionPhysicsOverlay::default()
    };

    let rhythmic_overlay = ict_engine::application::execution::ExecutionPhysicsOverlay {
        spectral: Some(rhythmic_metrics()),
        ..ict_engine::application::execution::ExecutionPhysicsOverlay::default()
    };

    let chaotic_artifact = build_execution_artifact_from_snapshot(
        "NQ",
        &snapshot,
        ExecutionArtifactBuildContext {
            prices: None,
            timestamps: None,
            fallback_ou: None,
            physics_overlay: Some(&chaotic_overlay),
        },
        &provenance,
    );
    let rhythmic_artifact = build_execution_artifact_from_snapshot(
        "NQ",
        &snapshot,
        ExecutionArtifactBuildContext {
            prices: None,
            timestamps: None,
            fallback_ou: None,
            physics_overlay: Some(&rhythmic_overlay),
        },
        &provenance,
    );

    assert!(
        chaotic_artifact.features.execution_readiness
            < rhythmic_artifact.features.execution_readiness,
        "chaotic={} rhythmic={}",
        chaotic_artifact.features.execution_readiness,
        rhythmic_artifact.features.execution_readiness,
    );
    assert_eq!(
        chaotic_artifact.features.dominant_cycle_energy,
        Some(chaotic_metrics().dominant_cycle_energy)
    );
    assert_eq!(
        chaotic_artifact.features.spectral_entropy,
        Some(chaotic_metrics().spectral_entropy)
    );
}
