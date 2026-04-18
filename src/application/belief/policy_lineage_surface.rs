use serde::Serialize;

use crate::state::{PreBayesPolicyLineageSummary, PreBayesPolicyRecord};

#[derive(Debug, Clone, Serialize, Default)]
pub struct BeliefPolicyLineageSurface {
    pub latest_version: String,
    pub previous_version: String,
    pub total_versions: usize,
    pub changed_fields_union: Vec<String>,
    pub rollback_candidate_version: String,
    pub rollback_reason: String,
}

pub fn build_belief_policy_lineage_surface(
    history: &[PreBayesPolicyRecord],
    latest_gate_status: &str,
) -> BeliefPolicyLineageSurface {
    let latest = history.last();
    let previous = history.iter().rev().nth(1);
    let changed_fields_union = history
        .iter()
        .flat_map(|record| record.diff_from_previous.changed_fields.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let rollback_candidate_version =
        if matches!(latest_gate_status, "observe_only" | "pass_neutralized") {
            previous
                .map(|record| record.policy.version.clone())
                .unwrap_or_default()
        } else {
            String::new()
        };
    let rollback_reason = if rollback_candidate_version.is_empty() {
        if history.is_empty() {
            "policy_history_unavailable".to_string()
        } else {
            "current_policy_stable".to_string()
        }
    } else {
        format!("consider_rollback_to={}", rollback_candidate_version)
    };
    BeliefPolicyLineageSurface {
        latest_version: latest
            .map(|record| record.policy.version.clone())
            .unwrap_or_else(|| "policy_version_unavailable".to_string()),
        previous_version: previous
            .map(|record| record.policy.version.clone())
            .unwrap_or_else(|| "policy_version_unavailable".to_string()),
        total_versions: history.len(),
        changed_fields_union,
        rollback_candidate_version,
        rollback_reason,
    }
}

impl From<BeliefPolicyLineageSurface> for PreBayesPolicyLineageSummary {
    fn from(value: BeliefPolicyLineageSurface) -> Self {
        Self {
            latest_version: (!value.latest_version.is_empty()).then_some(value.latest_version),
            previous_version: (!value.previous_version.is_empty())
                .then_some(value.previous_version),
            total_versions: value.total_versions,
            changed_fields_union: value.changed_fields_union,
            rollback_candidate_version: (!value.rollback_candidate_version.is_empty())
                .then_some(value.rollback_candidate_version),
            rollback_reason: value.rollback_reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lineage_surface_defaults_when_empty() {
        let surface = build_belief_policy_lineage_surface(&[], "pass_hard");
        assert_eq!(surface.total_versions, 0);
        assert_eq!(surface.latest_version, "policy_version_unavailable");
        assert_eq!(surface.previous_version, "policy_version_unavailable");
        assert_eq!(surface.rollback_reason, "policy_history_unavailable");
    }

    #[test]
    fn lineage_surface_marks_stable_only_when_history_exists() {
        let mut record = PreBayesPolicyRecord::default();
        record.policy.version = "v1".to_string();
        let surface = build_belief_policy_lineage_surface(&[record], "pass_hard");
        assert_eq!(surface.rollback_reason, "current_policy_stable");
    }
}
