use chrono::{DateTime, Utc};

use crate::application::execution::ExecutionPhysicsOverlay;
use crate::domain::execution::{
    apply_spectral_execution_penalty, build_ou_execution_metrics, classify_execution_gate,
    estimate_ou_execution_metrics, estimate_spectral_execution_metrics, execution_edge_split,
    execution_readiness, ExecutionArtifact, ExecutionFeatures, SPECTRAL_DEFAULT_LAMBDA_RATIO,
};
use crate::state::RunProvenance;

#[derive(Debug, Clone, Copy, Default)]
pub struct ExecutionInputSnapshot {
    pub aggression_bias: f64,
    pub completion_pressure: f64,
    pub liquidity_absorption_bias: f64,
    pub evidence_quality: f64,
    pub prediction_score: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ExecutionOuFallback {
    pub normalized_distance_to_projected_trend_bps: f64,
    pub ou_half_life_bars: f64,
    pub ou_pullback_expectation_zscore: f64,
    pub ou_reversion_speed_per_bar: f64,
    pub ou_expected_pullback_bps: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ExecutionArtifactBuildContext<'a> {
    pub prices: Option<&'a [f64]>,
    pub timestamps: Option<&'a [DateTime<Utc>]>,
    pub fallback_ou: Option<&'a ExecutionOuFallback>,
    pub physics_overlay: Option<&'a ExecutionPhysicsOverlay>,
}

#[allow(clippy::too_many_arguments)]
pub fn build_execution_artifact(
    symbol: &str,
    aggression_bias: f64,
    completion_pressure: f64,
    liquidity_absorption_bias: f64,
    evidence_quality: f64,
    prediction_score: f64,
    normalized_distance_to_projected_trend_bps: Option<f64>,
    ou_half_life_bars: Option<f64>,
    ou_pullback_expectation_zscore: Option<f64>,
    ou_reversion_speed_per_bar: Option<f64>,
    ou_expected_pullback_bps: Option<f64>,
    physics_overlay: Option<ExecutionPhysicsOverlay>,
    provenance: &RunProvenance,
) -> ExecutionArtifact {
    let snapshot = ExecutionInputSnapshot {
        aggression_bias,
        completion_pressure,
        liquidity_absorption_bias,
        evidence_quality,
        prediction_score,
    };
    let fallback_ou = match (
        normalized_distance_to_projected_trend_bps,
        ou_half_life_bars,
        ou_pullback_expectation_zscore,
        ou_reversion_speed_per_bar,
        ou_expected_pullback_bps,
    ) {
        (
            Some(normalized_distance_to_projected_trend_bps),
            Some(ou_half_life_bars),
            Some(ou_pullback_expectation_zscore),
            Some(ou_reversion_speed_per_bar),
            Some(ou_expected_pullback_bps),
        ) => Some(ExecutionOuFallback {
            normalized_distance_to_projected_trend_bps,
            ou_half_life_bars,
            ou_pullback_expectation_zscore,
            ou_reversion_speed_per_bar,
            ou_expected_pullback_bps,
        }),
        _ => None,
    };

    build_execution_artifact_from_snapshot(
        symbol,
        &snapshot,
        ExecutionArtifactBuildContext {
            prices: None,
            timestamps: None,
            fallback_ou: fallback_ou.as_ref(),
            physics_overlay: physics_overlay.as_ref(),
        },
        provenance,
    )
}

pub fn build_execution_artifact_from_snapshot(
    symbol: &str,
    snapshot: &ExecutionInputSnapshot,
    context: ExecutionArtifactBuildContext<'_>,
    provenance: &RunProvenance,
) -> ExecutionArtifact {
    let execution_score = (snapshot.completion_pressure.max(0.0) * (45.0 / 100.0)
        + snapshot.liquidity_absorption_bias.max(0.0) * 0.20
        + snapshot.evidence_quality.max(0.0) * 0.35)
        .clamp(0.0, 1.0);
    let split = execution_edge_split(execution_score, snapshot.prediction_score);
    let ou_metrics = context
        .prices
        .zip(context.timestamps)
        .and_then(|(prices, timestamps)| estimate_ou_execution_metrics(prices, timestamps))
        .or_else(|| {
            context.fallback_ou.map(|fallback| {
                build_ou_execution_metrics(
                    fallback.normalized_distance_to_projected_trend_bps,
                    fallback.ou_half_life_bars,
                    fallback.ou_reversion_speed_per_bar,
                    fallback.ou_pullback_expectation_zscore,
                    fallback.ou_expected_pullback_bps,
                )
            })
        });
    let overextension_distance = ou_metrics
        .as_ref()
        .map(|value| value.overextension_distance);
    let reversion_speed = ou_metrics
        .as_ref()
        .map(|value| value.reversion_speed_per_bar);
    let base_readiness = execution_readiness(
        execution_score,
        snapshot.evidence_quality,
        overextension_distance,
        reversion_speed,
    );
    // Spectral metrics: prefer the overlay's cached fit, otherwise re-estimate
    // from prices. This matches the OU fallback pattern so the artifact stays
    // self-sufficient when called with prices-only contexts.
    let spectral_metrics = context
        .physics_overlay
        .and_then(|overlay| overlay.spectral.clone())
        .or_else(|| {
            context.prices.and_then(|prices| {
                estimate_spectral_execution_metrics(prices, SPECTRAL_DEFAULT_LAMBDA_RATIO)
            })
        });
    let dominant_cycle_energy = spectral_metrics
        .as_ref()
        .map(|value| value.dominant_cycle_energy);
    let cycle_phase_alignment = spectral_metrics
        .as_ref()
        .map(|value| value.cycle_phase_alignment);
    let spectral_entropy = spectral_metrics
        .as_ref()
        .map(|value| value.spectral_entropy);
    let readiness = apply_spectral_execution_penalty(base_readiness, spectral_metrics.as_ref());

    ExecutionArtifact {
        artifact_id: format!("execution:{}:{}", symbol, provenance.data_fingerprint),
        generated_at: Utc::now(),
        symbol: symbol.to_string(),
        features: ExecutionFeatures {
            execution_score,
            prediction_score: snapshot.prediction_score,
            execution_edge_share: split.execution_edge_share,
            prediction_edge_share: split.prediction_edge_share,
            execution_readiness: readiness,
            aggression_bias: snapshot.aggression_bias,
            completion_pressure: snapshot.completion_pressure,
            liquidity_absorption_bias: snapshot.liquidity_absorption_bias,
            evidence_quality: snapshot.evidence_quality,
            overextension_distance,
            reversion_speed,
            dominant_cycle_energy,
            cycle_phase_alignment,
            spectral_entropy,
            ou_metrics,
            ising_state: context.physics_overlay.and_then(|p| p.ising.clone()),
            pythagorean_metrics: context.physics_overlay.and_then(|p| p.pythagorean),
            spectral_metrics,
        },
        hard_gate_status: classify_execution_gate(readiness).to_string(),
        provenance: provenance.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn provenance() -> RunProvenance {
        RunProvenance {
            data_fingerprint: "fp".to_string(),
            ..RunProvenance::default()
        }
    }

    fn snapshot() -> ExecutionInputSnapshot {
        ExecutionInputSnapshot {
            aggression_bias: 0.2,
            completion_pressure: 0.7,
            liquidity_absorption_bias: 0.6,
            evidence_quality: 0.8,
            prediction_score: 0.4,
        }
    }

    fn stamps(n: usize) -> Vec<DateTime<Utc>> {
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        (0..n).map(|i| base + Duration::minutes(i as i64)).collect()
    }

    fn mean_reverting_prices(n: usize) -> Vec<f64> {
        let mu = 100.0;
        let phi = 0.82;
        let mut value = mu;
        let mut state: u64 = 0xA5A5A5A5A5A5A5A5;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let noise = ((state >> 33) as f64) / (u32::MAX as f64) - 0.5;
            value = mu + phi * (value - mu) + noise;
            out.push(value);
        }
        out
    }

    #[test]
    fn prefers_real_ou_fit_when_prices_and_timestamps_are_present() {
        let prices = mean_reverting_prices(128);
        let timestamps = stamps(prices.len());
        let fallback = ExecutionOuFallback {
            normalized_distance_to_projected_trend_bps: 9_999.0,
            ou_half_life_bars: 99.0,
            ou_pullback_expectation_zscore: 2.5,
            ou_reversion_speed_per_bar: 0.01,
            ou_expected_pullback_bps: 250.0,
        };
        let artifact = build_execution_artifact_from_snapshot(
            "NQ",
            &snapshot(),
            ExecutionArtifactBuildContext {
                prices: Some(&prices),
                timestamps: Some(&timestamps),
                fallback_ou: Some(&fallback),
                physics_overlay: None,
            },
            &provenance(),
        );

        let metrics = artifact.features.ou_metrics.expect("real ou metrics");
        assert!(metrics.half_life_bars < 20.0);
        assert_ne!(metrics.half_life_bars, fallback.ou_half_life_bars);
    }

    #[test]
    fn falls_back_to_ltf_ou_budget_when_fit_is_unavailable() {
        let prices = vec![100.0; 8];
        let timestamps = stamps(prices.len());
        let fallback = ExecutionOuFallback {
            normalized_distance_to_projected_trend_bps: 180.0,
            ou_half_life_bars: 5.0,
            ou_pullback_expectation_zscore: 1.2,
            ou_reversion_speed_per_bar: 0.31,
            ou_expected_pullback_bps: 42.0,
        };
        let artifact = build_execution_artifact_from_snapshot(
            "NQ",
            &snapshot(),
            ExecutionArtifactBuildContext {
                prices: Some(&prices),
                timestamps: Some(&timestamps),
                fallback_ou: Some(&fallback),
                physics_overlay: None,
            },
            &provenance(),
        );

        let metrics = artifact.features.ou_metrics.expect("fallback ou metrics");
        assert_eq!(metrics.half_life_bars, 5.0);
        assert_eq!(metrics.expected_pullback_bps, 42.0);
    }
}
