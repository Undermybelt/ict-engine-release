use crate::application::execution::{build_execution_physics_overlay, ExecutionPhysicsOverlay};
use crate::application::orchestration::PipelineState;
use crate::config::FrameFeatures;
use crate::types::Candle;

pub fn apply_physics_overlay(
    pipeline_state: &mut PipelineState,
    candles: &[Candle],
    frame_features: &FrameFeatures,
) -> ExecutionPhysicsOverlay {
    let overlay = build_execution_physics_overlay(candles, frame_features);
    pipeline_state.physics_overlay = Some(overlay.clone());
    overlay
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

    #[test]
    fn applies_physics_overlay_to_pipeline_state() {
        let mut state = PipelineState::new("NQ", Some("NQ"), "test");

        // Sample candles for testing - need enough data for OU fitting
        let start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let candles: Vec<Candle> = (0..48)
            .map(|i| {
                let close = 100.0 + (i as f64 * 0.1).sin() + i as f64 * 0.02;
                Candle {
                    timestamp: start + Duration::minutes(i as i64),
                    open: close - 0.1,
                    high: close + 0.2,
                    low: close - 0.2,
                    close,
                    volume: 1_000.0 + i as f64,
                }
            })
            .collect();

        // Create minimal frame features
        let frame_features = FrameFeatures {
            regime_label: "bull".to_string(),
            liquidity_label: "favorable".to_string(),
            normalized_distance_to_range_mid_bps: 100.0,
            normalized_distance_to_projected_trend_bps: 200.0,
            ou_half_life_bars: 5.0,
            ou_reversion_speed_per_bar: 0.1,
            ou_pullback_expectation_zscore: 1.0,
            pythagorean_overstretch: Some(0.3),
            sweep_count: 2,
            fvg_count: 1,
            ..Default::default()
        };

        let _overlay = apply_physics_overlay(&mut state, &candles, &frame_features);

        assert!(state.physics_overlay.is_some());
        // OU may or may not be present depending on fitting success
        // Ising should always be present (derived from frame features)
        assert!(state.physics_overlay.as_ref().unwrap().ising.is_some());
        // Pythagorean requires at least 2 candles
        assert!(state
            .physics_overlay
            .as_ref()
            .unwrap()
            .pythagorean
            .is_some());
    }
}
