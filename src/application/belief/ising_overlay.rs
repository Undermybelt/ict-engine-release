use serde::{Deserialize, Serialize};

use crate::application::orchestration::PipelineState;
use crate::domain::execution::EXECUTION_GATE_OBSERVE;
use crate::domain::regime::IsingState;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IsingOverlayState {
    pub magnetization: f64,
    pub coupling_strength: f64,
    pub phase_transition_risk: f64,
    pub herding_bias: f64,
    pub branch_influence_enabled: bool,
}

pub fn apply_ising_overlay(
    pipeline_state: &mut PipelineState,
    ising_state: &IsingState,
    execution_readiness: f64,
) -> IsingOverlayState {
    let overlay = IsingOverlayState {
        magnetization: ising_state.magnetization,
        coupling_strength: ising_state.coupling_strength,
        phase_transition_risk: ising_state.phase_transition_risk,
        herding_bias: ising_state.herding_bias,
        branch_influence_enabled: execution_readiness >= EXECUTION_GATE_OBSERVE,
    };
    pipeline_state.ising_overlay = Some(overlay.clone());
    overlay
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ising_overlay_respects_observe_gate() {
        let mut state = PipelineState::new("NQ", Some("NQ"), "test");
        let ising = IsingState {
            magnetization: 0.8,
            coupling_strength: 0.7,
            phase_transition_risk: 0.3,
            herding_bias: 0.8,
        };

        let blocked = apply_ising_overlay(&mut state, &ising, 0.20);
        assert!(!blocked.branch_influence_enabled);

        let observing = apply_ising_overlay(&mut state, &ising, 0.55);
        assert!(observing.branch_influence_enabled);
        assert!(state.ising_overlay.is_some());
    }
}
