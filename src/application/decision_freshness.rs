use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct DecisionFreshnessGate {
    pub status: String,
    pub reason: String,
    pub max_age_seconds: i64,
    pub observed_age_seconds: i64,
    pub penalty: f64,
    pub should_block: bool,
    pub next_action: String,
}

pub fn build_decision_freshness_gate(
    max_age_seconds: i64,
    observed_age_seconds: i64,
) -> DecisionFreshnessGate {
    let (status, penalty, should_block, next_action) = if observed_age_seconds <= max_age_seconds {
        ("fresh", 0.0, false, "proceed".to_string())
    } else if observed_age_seconds <= max_age_seconds * 2 {
        (
            "aging",
            0.15,
            false,
            "degrade_confidence_and_refresh_soon".to_string(),
        )
    } else {
        (
            "stale",
            0.35,
            true,
            "refresh_data_before_execution".to_string(),
        )
    };
    let reason = format!(
        "observed_age_seconds={} max_age_seconds={} penalty={:.2}",
        observed_age_seconds, max_age_seconds, penalty
    );
    DecisionFreshnessGate {
        status: status.to_string(),
        reason,
        max_age_seconds,
        observed_age_seconds,
        penalty,
        should_block,
        next_action,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freshness_gate_marks_stale_and_blocks() {
        let gate = build_decision_freshness_gate(300, 900);
        assert_eq!(gate.status, "stale");
        assert!(gate.should_block);
        assert_eq!(gate.next_action, "refresh_data_before_execution");
    }
}
