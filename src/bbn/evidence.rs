use std::collections::HashMap;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::factor_lab::factor_definition::{FactorSignal, FactorUsagePhase};
use crate::types::PdaLifecycleState;

use super::node::NodeId;

fn band_width_bps(top: f64, bottom: f64) -> f64 {
    let mid = ((top + bottom) / 2.0).abs();
    if mid <= f64::EPSILON {
        0.0
    } else {
        ((top - bottom).abs() / mid) * 10_000.0
    }
}

fn inferred_sweep_depth_bps(state: &crate::types::TimedPdaState) -> f64 {
    let width = band_width_bps(state.band.top, state.band.bottom);
    if matches!(state.concept, crate::types::PdaConceptKind::LiquidityPool)
        || matches!(state.concept, crate::types::PdaConceptKind::EqualHighsLows)
        || matches!(
            state.concept,
            crate::types::PdaConceptKind::SwingFailurePattern
        )
    {
        width.max(10.0)
    } else {
        0.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvidenceType {
    Hard(usize),
    Soft(Vec<f64>),
}

pub type Evidence = HashMap<NodeId, EvidenceType>;
pub type IndicatorValues = HashMap<String, f64>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBinding {
    pub node_id: NodeId,
    pub signal: FactorSignal,
    pub value: EvidenceType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ICTStructureSummary {
    pub bias: Option<String>,
    pub dealing_range: Option<String>,
    pub session: Option<String>,
    pub active_pda_count: usize,
    pub inversed_pda_count: usize,
    pub stale_pda_count: usize,
    pub nearest_active_pda: Option<String>,
    pub nearest_inversed_pda: Option<String>,
    pub notes: Vec<String>,
}

pub fn summarize_timed_pda_states(states: &[crate::types::TimedPdaState]) -> ICTStructureSummary {
    let active = states
        .iter()
        .filter(|state| {
            matches!(
                state.state,
                PdaLifecycleState::Active
                    | PdaLifecycleState::Touched
                    | PdaLifecycleState::Mitigated
            )
        })
        .count();
    let inversed = states
        .iter()
        .filter(|state| matches!(state.state, PdaLifecycleState::Inversed))
        .count();
    let stale = states
        .iter()
        .filter(|state| {
            matches!(
                state.state,
                PdaLifecycleState::Invalidated | PdaLifecycleState::Expired
            )
        })
        .count();
    let nearest_active = states
        .iter()
        .find(|state| {
            matches!(
                state.state,
                PdaLifecycleState::Active
                    | PdaLifecycleState::Touched
                    | PdaLifecycleState::Mitigated
            )
        })
        .map(|state| {
            format!(
                "{:?}:{:?}|top={:.6}|bottom={:.6}|width_bps={:.3}|sweep_depth_bps={:.3}",
                state.concept,
                state.direction,
                state.band.top,
                state.band.bottom,
                band_width_bps(state.band.top, state.band.bottom),
                inferred_sweep_depth_bps(state)
            )
        });
    let nearest_inversed = states
        .iter()
        .find(|state| matches!(state.state, PdaLifecycleState::Inversed))
        .map(|state| {
            format!(
                "{:?}:{:?}|top={:.6}|bottom={:.6}|width_bps={:.3}|sweep_depth_bps={:.3}",
                state.concept,
                state.direction,
                state.band.top,
                state.band.bottom,
                band_width_bps(state.band.top, state.band.bottom),
                inferred_sweep_depth_bps(state)
            )
        });

    ICTStructureSummary {
        bias: None,
        dealing_range: None,
        session: None,
        active_pda_count: active,
        inversed_pda_count: inversed,
        stale_pda_count: stale,
        nearest_active_pda: nearest_active,
        nearest_inversed_pda: nearest_inversed,
        notes: Vec::new(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvidenceSource {
    pub indicator_values: IndicatorValues,
    pub structure_summary: ICTStructureSummary,
}

#[derive(Debug, Clone, Default)]
pub struct EvidenceManager {
    evidence: Evidence,
}

impl EvidenceManager {
    pub fn new() -> Self {
        Self {
            evidence: Evidence::new(),
        }
    }

    pub fn with_evidence(evidence: Evidence) -> Result<Self> {
        validate_evidence(&evidence)?;
        Ok(Self { evidence })
    }

    pub fn insert_hard(&mut self, node_id: impl Into<NodeId>, state_index: usize) {
        self.evidence
            .insert(node_id.into(), EvidenceType::Hard(state_index));
    }

    pub fn insert_soft(&mut self, node_id: impl Into<NodeId>, distribution: Vec<f64>) {
        self.evidence
            .insert(node_id.into(), EvidenceType::Soft(distribution));
    }

    pub fn insert_factor_binding(&mut self, binding: EvidenceBinding) -> Result<()> {
        binding.signal.ensure_phase(FactorUsagePhase::Evidence)?;
        self.evidence.insert(binding.node_id, binding.value);
        Ok(())
    }

    pub fn get(&self, node_id: &str) -> Option<&EvidenceType> {
        self.evidence.get(node_id)
    }

    pub fn as_map(&self) -> &Evidence {
        &self.evidence
    }

    pub fn into_map(self) -> Evidence {
        self.evidence
    }
}

pub fn validate_evidence(evidence: &Evidence) -> Result<()> {
    for (node_id, value) in evidence {
        match value {
            EvidenceType::Hard(_) => {}
            EvidenceType::Soft(probs) => {
                if probs.is_empty() {
                    bail!("soft evidence for '{}' cannot be empty", node_id);
                }
                if probs.iter().any(|p| *p < 0.0 || !p.is_finite()) {
                    bail!(
                        "soft evidence for '{}' contains invalid probability",
                        node_id
                    );
                }
                let sum: f64 = probs.iter().sum();
                if (sum - 1.0).abs() > 1e-6 {
                    bail!(
                        "soft evidence for '{}' must sum to 1.0, got {}",
                        node_id,
                        sum
                    );
                }
            }
        }
    }
    Ok(())
}

pub trait EvidenceExt {
    fn insert_hard(&mut self, node_id: impl Into<NodeId>, state_index: usize);
    fn insert_soft(&mut self, node_id: impl Into<NodeId>, distribution: Vec<f64>);
}

impl EvidenceExt for Evidence {
    fn insert_hard(&mut self, node_id: impl Into<NodeId>, state_index: usize) {
        self.insert(node_id.into(), EvidenceType::Hard(state_index));
    }

    fn insert_soft(&mut self, node_id: impl Into<NodeId>, distribution: Vec<f64>) {
        self.insert(node_id.into(), EvidenceType::Soft(distribution));
    }
}

pub fn validate_factor_evidence_binding(signal: &FactorSignal) -> Result<()> {
    signal.ensure_phase(FactorUsagePhase::Evidence)
}

#[cfg(test)]
mod summary_tests {
    use super::*;
    use crate::types::{
        Direction, PdaConceptKind, PdaInvalidationRule, PdaInverseMode, PdaLifecycleState,
        PdaStateTransition, PriceLevelBand, TimedPdaState,
    };

    #[test]
    fn summarizes_timed_pda_states_counts() {
        let states = vec![
            TimedPdaState {
                concept: PdaConceptKind::FairValueGap,
                direction: Direction::Bull,
                band: PriceLevelBand {
                    top: 2.0,
                    bottom: 1.0,
                },
                anchor_bar: 1,
                last_updated_bar: 2,
                state: PdaLifecycleState::Active,
                invalidation_rule: PdaInvalidationRule::FullFill,
                inverse_mode: PdaInverseMode::FlipNeedsConfirmation,
                validity_bars: 10,
                touch_count: 0,
                mitigation_progress: 0.0,
                inverse_confirmed: false,
                transitions: vec![PdaStateTransition {
                    state: PdaLifecycleState::Active,
                    at_bar: 1,
                    note: "x".into(),
                }],
            },
            TimedPdaState {
                concept: PdaConceptKind::Ndog,
                direction: Direction::Bear,
                band: PriceLevelBand {
                    top: 3.0,
                    bottom: 2.0,
                },
                anchor_bar: 1,
                last_updated_bar: 2,
                state: PdaLifecycleState::Inversed,
                invalidation_rule: PdaInvalidationRule::CloseThrough,
                inverse_mode: PdaInverseMode::FlipNeedsConfirmation,
                validity_bars: 10,
                touch_count: 0,
                mitigation_progress: 0.0,
                inverse_confirmed: true,
                transitions: vec![PdaStateTransition {
                    state: PdaLifecycleState::Inversed,
                    at_bar: 2,
                    note: "x".into(),
                }],
            },
            TimedPdaState {
                concept: PdaConceptKind::Nwog,
                direction: Direction::Bear,
                band: PriceLevelBand {
                    top: 4.0,
                    bottom: 3.0,
                },
                anchor_bar: 1,
                last_updated_bar: 2,
                state: PdaLifecycleState::Expired,
                invalidation_rule: PdaInvalidationRule::TimeExpiry,
                inverse_mode: PdaInverseMode::None,
                validity_bars: 10,
                touch_count: 0,
                mitigation_progress: 0.0,
                inverse_confirmed: false,
                transitions: vec![PdaStateTransition {
                    state: PdaLifecycleState::Expired,
                    at_bar: 2,
                    note: "x".into(),
                }],
            },
        ];
        let summary = summarize_timed_pda_states(&states);
        assert_eq!(summary.active_pda_count, 1);
        assert_eq!(summary.inversed_pda_count, 1);
        assert_eq!(summary.stale_pda_count, 1);
        assert!(summary.nearest_active_pda.is_some());
        assert!(summary.nearest_inversed_pda.is_some());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factor_lab::factor_definition::{FactorCategory, FactorRole};
    use crate::types::Direction;
    use chrono::Utc;

    fn signal(category: FactorCategory, roles: Vec<FactorRole>) -> FactorSignal {
        FactorSignal {
            factor_name: "test".to_string(),
            category,
            roles,
            timestamp: Utc::now(),
            value: 0.6,
            direction: Direction::Bull,
            confidence: 0.8,
            explanation: "x".to_string(),
            paired_market_quality_report: None,
            weight: 1.0,
            posterior_reliability: 1.0,
            regime_multiplier: 1.0,
            regime_adjusted_score: 0.6,
        }
    }

    #[test]
    fn rejects_footprint_signal_as_evidence_binding() {
        let mut manager = EvidenceManager::new();
        let err = manager
            .insert_factor_binding(EvidenceBinding {
                node_id: "node".to_string(),
                signal: signal(
                    FactorCategory::StructureIct,
                    vec![FactorRole::PriorAdjuster, FactorRole::Evidence],
                ),
                value: EvidenceType::Hard(0),
            })
            .unwrap_err()
            .to_string();
        assert!(err.contains("cannot be used as evidence"));
    }

    #[test]
    fn accepts_true_evidence_binding() {
        let mut manager = EvidenceManager::new();
        manager
            .insert_factor_binding(EvidenceBinding {
                node_id: "node".to_string(),
                signal: signal(FactorCategory::TrendMomentum, vec![FactorRole::Evidence]),
                value: EvidenceType::Soft(vec![0.2, 0.8]),
            })
            .unwrap();
        assert!(manager.get("node").is_some());
    }
}
