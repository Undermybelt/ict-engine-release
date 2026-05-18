use serde::{Deserialize, Serialize};

use crate::domain::execution::{OuExecutionMetrics, EXECUTION_GATE_OBSERVE};

use crate::application::orchestration::PipelineState;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OuOverlayState {
    pub overextension_distance: f64,
    pub reversion_speed_per_bar: f64,
    pub expected_pullback_bps: f64,
    pub regime_influence_enabled: bool,
}

pub fn apply_ou_overlay(
    pipeline_state: &mut PipelineState,
    ou_metrics: &OuExecutionMetrics,
    execution_readiness: f64,
) -> OuOverlayState {
    // Activation threshold previously gated by EXECUTION_GATE_READY (0.65),
    // which created a chicken-and-egg: OU regime evidence is meant to lift
    // readiness, but was disabled until readiness was already ready. Aligned
    // with the spectral overlay's activation gate (EXECUTION_GATE_OBSERVE,
    // 0.45) so OU evidence can contribute from the observe band upward.
    let overlay = OuOverlayState {
        overextension_distance: ou_metrics.overextension_distance,
        reversion_speed_per_bar: ou_metrics.reversion_speed_per_bar,
        expected_pullback_bps: ou_metrics.expected_pullback_bps,
        regime_influence_enabled: execution_readiness >= EXECUTION_GATE_OBSERVE,
    };
    pipeline_state.ou_overlay = Some(overlay.clone());
    overlay
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ou_overlay_enables_regime_influence_from_observe_gate_upward() {
        let mut state = PipelineState::new("NQ", Some("NQ"), "test");
        let metrics = OuExecutionMetrics {
            half_life_bars: 3.0,
            reversion_speed_per_bar: 0.22,
            pullback_expectation_zscore: 1.4,
            overextension_distance: 0.4,
            expected_pullback_bps: 55.0,
        };

        // Below EXECUTION_GATE_OBSERVE (0.45): overlay is gated off.
        let blocked = apply_ou_overlay(&mut state, &metrics, 0.30);
        assert!(!blocked.regime_influence_enabled);

        // At or above EXECUTION_GATE_OBSERVE: overlay activates so its
        // evidence can lift readiness toward the ready band.
        let observe_band = apply_ou_overlay(&mut state, &metrics, 0.50);
        assert!(observe_band.regime_influence_enabled);

        let ready_band = apply_ou_overlay(&mut state, &metrics, 0.80);
        assert!(ready_band.regime_influence_enabled);
        assert!(state.ou_overlay.is_some());
    }
}
