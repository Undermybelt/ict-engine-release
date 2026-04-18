use crate::state::{
    FeedbackFactorUsage, FeedbackRecord, ModelProbabilitySnapshot, ReflectionBeliefUpdate,
    ReflectionCompactStatus, ReflectionMismatchTag, ReflectionRecord, ReflectionStatus,
};
use crate::types::{Direction, Regime};

pub fn reflection_status(record: &ReflectionRecord) -> ReflectionStatus {
    ReflectionStatus {
        compact: ReflectionCompactStatus {
            hypothesis: format!("{:?}", record.hypothesis),
            prior: record.prior,
            posterior_before_trade: record.posterior_before_trade,
            outcome: record.realized_outcome.clone(),
            verified: record.verified,
            falsified: record.falsified,
        },
        expected_chain: record.expected_chain.clone(),
        realized_chain: record.realized_chain.clone(),
        mismatch_tags: record.belief_update.mismatch_tags.clone(),
        structured_mismatch_tags: record.belief_update.structured_tags.clone(),
        belief_update: record.belief_update.clone(),
        suggested_belief_update: record.suggested_belief_update.clone(),
    }
}

pub fn build_belief_update(
    mismatch_tags: &[ReflectionMismatchTag],
    suggested_belief_update: impl Into<String>,
) -> ReflectionBeliefUpdate {
    let summary = suggested_belief_update.into();
    ReflectionBeliefUpdate {
        mismatch_tags: mismatch_tags
            .iter()
            .map(|tag| tag.as_str().to_string())
            .collect(),
        structured_tags: mismatch_tags.to_vec(),
        summary,
    }
}

pub fn structured_belief_update(
    mismatch_tags: &[ReflectionMismatchTag],
    suggested_belief_update: impl Into<String>,
) -> ReflectionBeliefUpdate {
    build_belief_update(mismatch_tags, suggested_belief_update)
}

pub fn build_mismatch_tags(
    expected_chain: &[String],
    realized_chain: &[String],
    verified: bool,
    falsified: bool,
) -> Vec<ReflectionMismatchTag> {
    let mut tags = Vec::new();

    if expected_chain == realized_chain {
        tags.push(ReflectionMismatchTag::ExpectedChainMatched);
    } else {
        tags.push(ReflectionMismatchTag::ExpectedChainDiverged);
    }

    if expected_chain.iter().any(|step| step.contains("confirm"))
        && !realized_chain.iter().any(|step| step.contains("confirm"))
    {
        tags.push(ReflectionMismatchTag::ConfirmationMissing);
    }

    if expected_chain.iter().any(|step| step.contains("follow"))
        && !realized_chain.iter().any(|step| step.contains("follow"))
    {
        tags.push(ReflectionMismatchTag::FollowThroughFailed);
    }

    if falsified
        && expected_chain.iter().any(|step| step.contains("regime="))
        && realized_chain
            .iter()
            .any(|step| step.contains("regime=") && !expected_chain.contains(step))
    {
        tags.push(ReflectionMismatchTag::RegimeMisclassified);
    }

    if verified && !tags.contains(&ReflectionMismatchTag::ExpectedChainMatched) {
        tags.push(ReflectionMismatchTag::ExpectedChainMatched);
    }

    tags.sort_by_key(ReflectionMismatchTag::as_str);
    tags.dedup();
    tags
}

pub fn reflection_feedback_record(
    timestamp: chrono::DateTime<chrono::Utc>,
    symbol: &str,
    factors_used: Vec<FeedbackFactorUsage>,
    probability_snapshot: ModelProbabilitySnapshot,
    outcome: String,
    pnl: f64,
    mismatch_tags: &[ReflectionMismatchTag],
) -> FeedbackRecord {
    FeedbackRecord {
        timestamp,
        symbol: symbol.to_string(),
        source: "reflection".to_string(),
        run_id: None,
        trade_id: None,
        prompt_version: None,
        factor_version: None,
        data_fingerprint: None,
        factors_used,
        model_probabilities_before_trade: probability_snapshot,
        realized_outcome: outcome,
        pnl,
        regime_at_entry: Regime::ManipulationExpansion,
        reflection_mismatch_tags: mismatch_tags
            .iter()
            .map(|tag| tag.as_str().to_string())
            .collect(),
    }
}

pub fn empty_reflection_record(
    hypothesis: crate::factor_lab::ReversalHypothesis,
) -> ReflectionRecord {
    ReflectionRecord {
        hypothesis,
        prior: 0.0,
        posterior_before_trade: 0.0,
        realized_outcome: "pending".to_string(),
        verified: false,
        falsified: false,
        expected_chain: Vec::new(),
        realized_chain: Vec::new(),
        mismatch_tags: Vec::new(),
        belief_update: build_belief_update(&[], "belief_update_unavailable"),
        suggested_belief_update: "belief_update_unavailable".to_string(),
    }
}

pub fn default_probability_snapshot() -> ModelProbabilitySnapshot {
    ModelProbabilitySnapshot {
        selected_direction: Direction::Neutral,
        selected_probability: 0.0,
        long_score: 0.0,
        short_score: 0.0,
        win_prob_long: 0.0,
        win_prob_short: 0.0,
        uncertainty: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_belief_update_keeps_structured_tags() {
        let update = build_belief_update(
            &[
                ReflectionMismatchTag::ConfirmationMissing,
                ReflectionMismatchTag::ExpectedChainDiverged,
            ],
            "tighten confirmation gate",
        );
        assert_eq!(update.structured_tags.len(), 2);
        assert_eq!(
            update.structured_tags[0],
            ReflectionMismatchTag::ConfirmationMissing
        );
        assert!(update
            .mismatch_tags
            .contains(&"confirmation_missing".to_string()));
    }

    #[test]
    fn empty_reflection_record_marks_belief_update_unavailable() {
        let record = empty_reflection_record(crate::factor_lab::ReversalHypothesis::BearTrap);
        assert_eq!(record.belief_update.summary, "belief_update_unavailable");
        assert_eq!(record.suggested_belief_update, "belief_update_unavailable");
    }

    #[test]
    fn test_build_mismatch_tags_covers_structured_labels() {
        let tags = build_mismatch_tags(
            &[
                "regime=trend".to_string(),
                "confirm".to_string(),
                "follow".to_string(),
            ],
            &["regime=range".to_string()],
            false,
            true,
        );
        let values: Vec<_> = tags.iter().map(|tag| tag.as_str()).collect();
        assert!(values.contains(&"confirmation_missing"));
        assert!(values.contains(&"follow_through_failed"));
        assert!(values.contains(&"regime_misclassified"));
        assert!(values.contains(&"expected_chain_diverged"));
    }

    #[test]
    fn test_reflection_status_prefers_structured_belief_update_surface() {
        let record = ReflectionRecord {
            hypothesis: crate::factor_lab::ReversalHypothesis::BullishExpansionContinuation,
            prior: 0.4,
            posterior_before_trade: 0.6,
            realized_outcome: "win".to_string(),
            verified: true,
            falsified: false,
            expected_chain: vec!["confirm".to_string()],
            realized_chain: vec!["confirm".to_string()],
            mismatch_tags: vec![ReflectionMismatchTag::ExpectedChainMatched],
            belief_update: structured_belief_update(
                &[ReflectionMismatchTag::ExpectedChainMatched],
                "hold prior",
            ),
            suggested_belief_update: "legacy text".to_string(),
        };

        let status = reflection_status(&record);
        assert_eq!(status.belief_update.summary, "hold prior");
        assert_eq!(
            status.structured_mismatch_tags,
            vec![ReflectionMismatchTag::ExpectedChainMatched]
        );
    }
}
