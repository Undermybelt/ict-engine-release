use anyhow::Result;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct GovernorDecision {
    pub selected_label: String,
    pub confidence: f64,
    pub entropy: f64,
    pub committed: bool,
    pub min_hold_active: bool,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RegimeGovernor {
    confidence_floor: f64,
    entropy_ceiling: f64,
    min_hold_bars: usize,
}

impl RegimeGovernor {
    pub fn new(confidence_floor: f64, entropy_ceiling: f64, min_hold_bars: usize) -> Self {
        Self {
            confidence_floor,
            entropy_ceiling,
            min_hold_bars,
        }
    }

    pub fn decide(
        &self,
        candidate_label: &str,
        membership: &BTreeMap<String, f64>,
        bars_since_last_switch: usize,
        _min_hold_override: bool,
    ) -> Result<GovernorDecision> {
        self.decide_internal(candidate_label, membership, None, bars_since_last_switch)
    }

    pub fn decide_with_previous(
        &self,
        candidate_label: &str,
        membership: &BTreeMap<String, f64>,
        _elapsed_bars: usize,
        previous_label: Option<&str>,
        bars_since_last_switch: usize,
    ) -> Result<GovernorDecision> {
        self.decide_internal(
            candidate_label,
            membership,
            previous_label,
            bars_since_last_switch,
        )
    }

    fn decide_internal(
        &self,
        candidate_label: &str,
        membership: &BTreeMap<String, f64>,
        previous_label: Option<&str>,
        bars_since_last_switch: usize,
    ) -> Result<GovernorDecision> {
        if membership.is_empty() {
            anyhow::bail!("membership must not be empty");
        }
        let confidence = membership
            .values()
            .copied()
            .fold(0.0_f64, f64::max)
            .clamp(0.0, 1.0);
        let entropy = membership
            .values()
            .copied()
            .filter(|p| *p > 0.0)
            .map(|p| -p * p.ln())
            .sum::<f64>();
        let min_hold_active =
            previous_label.is_some() && bars_since_last_switch < self.min_hold_bars;
        let committed = !min_hold_active
            && confidence >= self.confidence_floor
            && entropy <= self.entropy_ceiling;
        let selected_label = if min_hold_active {
            previous_label.unwrap_or(candidate_label).to_string()
        } else {
            candidate_label.to_string()
        };
        let evidence = vec![
            format!("governor_candidate={candidate_label}"),
            format!("governor_selected={selected_label}"),
            format!("governor_confidence={confidence:.4}"),
            format!("governor_entropy={entropy:.4}"),
            format!("governor_min_hold_active={min_hold_active}"),
            format!("governor_committed={committed}"),
        ];
        Ok(GovernorDecision {
            selected_label,
            confidence,
            entropy,
            committed,
            min_hold_active,
            evidence,
        })
    }
}
