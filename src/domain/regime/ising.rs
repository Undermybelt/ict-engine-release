use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IsingState {
    pub magnetization: f64,
    pub coupling_strength: f64,
    pub phase_transition_risk: f64,
    pub herding_bias: f64,
}

pub fn estimate_ising_state(
    aligned_signals: &[f64],
    participation_weights: &[f64],
) -> Option<IsingState> {
    if aligned_signals.is_empty() || aligned_signals.len() != participation_weights.len() {
        return None;
    }

    let total_weight = participation_weights
        .iter()
        .copied()
        .filter(|value| value.is_finite() && *value > 0.0)
        .sum::<f64>();
    if total_weight <= f64::EPSILON {
        return None;
    }

    let weighted_signal = aligned_signals
        .iter()
        .copied()
        .zip(participation_weights.iter().copied())
        .filter(|(signal, weight)| signal.is_finite() && weight.is_finite() && *weight > 0.0)
        .map(|(signal, weight)| signal.clamp(-1.0, 1.0) * weight)
        .sum::<f64>();
    let magnetization = (weighted_signal / total_weight).clamp(-1.0, 1.0);
    let herding_bias = magnetization.abs();

    let participation_concentration = participation_weights
        .iter()
        .copied()
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|weight| {
            let normalized = weight / total_weight;
            normalized * normalized
        })
        .sum::<f64>()
        .clamp(0.0, 1.0);
    let coupling_strength =
        ((herding_bias * 0.65) + (participation_concentration * 0.35)).clamp(0.0, 1.0);
    let phase_transition_risk = (coupling_strength * (1.0 - herding_bias + 0.25)).clamp(0.0, 1.0);

    Some(IsingState {
        magnetization,
        coupling_strength,
        phase_transition_risk,
        herding_bias,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_high_herding_phase_risk() {
        let state = estimate_ising_state(&[1.0, 0.9, 0.8], &[1.0, 1.0, 0.9]).unwrap();
        assert!(state.herding_bias > 0.8);
        assert!(state.phase_transition_risk < 0.5);
    }

    #[test]
    fn detects_calm_low_herding_state() {
        let state = estimate_ising_state(&[0.2, -0.1, 0.0], &[1.0, 1.0, 1.0]).unwrap();
        assert!(state.herding_bias < 0.2);
        assert!(state.phase_transition_risk >= 0.0);
        assert!(state.phase_transition_risk < 0.5);
    }
}
