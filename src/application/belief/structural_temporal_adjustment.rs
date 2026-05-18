#[cfg(test)]
use std::collections::BTreeMap;

pub use crate::belief_core::regime_filter::{
    blend_branch_prior_with_transition_prior, blend_node_posterior_with_duration_prior,
    transition_adjusted_branch_posteriors, transition_adjusted_node_posteriors,
};
#[cfg(test)]
use crate::state::{
    StructuralBranchTemporalPosteriorState, StructuralBranchTransitionPrior,
    StructuralNodeDurationPrior, StructuralNodeTemporalPosteriorState,
    StructuralNodeTransitionPosteriorState,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_adjusted_branch_posteriors_respects_transition_outcome_support() {
        let mut priors = BTreeMap::new();
        priors.insert(
            "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:trend:transition_confirmation".to_string(),
            StructuralBranchTransitionPrior {
                from_node_id: "NQ:belief_regime_node:trend".to_string(),
                to_node_id: "NQ:belief_regime_node:trend".to_string(),
                from_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                to_branch_id: "NQ:belief_regime_node:trend:transition_confirmation".to_string(),
                observations: 2,
                weighted_observation_mass: 1.5,
                wins: 2,
                losses: 0,
                invalidated: 0,
                transition_prior: 0.5,
                transition_outcome_support: 0.8,
                temporal_posterior_support: 0.59,
                weighted_success_mass: 1.5,
                weighted_failure_mass: 0.0,
                last_recommended_at: None,
            },
        );
        priors.insert(
            "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:trend:range_mean_reversion".to_string(),
            StructuralBranchTransitionPrior {
                from_node_id: "NQ:belief_regime_node:trend".to_string(),
                to_node_id: "NQ:belief_regime_node:trend".to_string(),
                from_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                to_branch_id: "NQ:belief_regime_node:trend:range_mean_reversion".to_string(),
                observations: 2,
                weighted_observation_mass: 1.5,
                wins: 0,
                losses: 2,
                invalidated: 0,
                transition_prior: 0.5,
                transition_outcome_support: 0.2,
                temporal_posterior_support: 0.41,
                weighted_success_mass: 0.0,
                weighted_failure_mass: 1.5,
                last_recommended_at: None,
            },
        );
        let mut temporal = BTreeMap::new();
        crate::belief_core::regime_filter::refresh_branch_transition_posteriors(
            &mut priors,
            &mut temporal,
        );

        let adjusted = transition_adjusted_branch_posteriors(
            "NQ:belief_regime_node:trend",
            &[
                ("transition".to_string(), 0.2),
                ("range".to_string(), 0.2),
                ("trend".to_string(), 0.6),
            ],
            Some("NQ:belief_regime_node:trend:trend_follow_through"),
            &priors,
            &temporal,
            |regime| match regime {
                "transition" => "transition_confirmation",
                "range" => "range_mean_reversion",
                _ => "trend_follow_through",
            },
        );

        assert!(
            adjusted["NQ:belief_regime_node:trend:transition_confirmation"]
                > adjusted["NQ:belief_regime_node:trend:range_mean_reversion"]
        );
    }

    #[test]
    fn blend_node_posterior_prefers_persisted_temporal_state_over_duration_prior() {
        let duration_prior = StructuralNodeDurationPrior {
            observations: 6,
            streak_count: 3,
            weighted_streak_mass: 2.4,
            weighted_success_mass: 2.4,
            weighted_failure_mass: 0.0,
            total_streak_length: 6,
            avg_streak_length: 2.0,
            max_streak_length: 3,
            last_streak_length: 3,
            persistence_prior: 0.90,
            duration_outcome_support: 0.77,
            temporal_posterior_support: 0.86,
            last_recommended_at: None,
            ..StructuralNodeDurationPrior::default()
        };
        let temporal_state = StructuralNodeTemporalPosteriorState {
            node_id: "NQ:belief_regime_node:trend".to_string(),
            observations: 6,
            streak_count: 3,
            weighted_streak_mass: 2.4,
            duration_outcome_support: 0.20,
            temporal_posterior_support: 0.30,
            posterior_blend_weight: 0.5,
            summary_line:
                "duration_mass=2.400 duration_support=0.200 duration_temporal=0.300 blend=0.500"
                    .to_string(),
            last_recommended_at: None,
            ..StructuralNodeTemporalPosteriorState::default()
        };

        let blended = blend_node_posterior_with_duration_prior(
            0.60,
            Some(&duration_prior),
            Some(&temporal_state),
        );

        assert!((blended - 0.45).abs() < 1e-9);
    }

    #[test]
    fn transition_adjusted_branch_posteriors_prefers_persisted_temporal_state_over_transition_prior(
    ) {
        let mut priors = BTreeMap::new();
        priors.insert(
            "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:trend:transition_confirmation".to_string(),
            StructuralBranchTransitionPrior {
                from_node_id: "NQ:belief_regime_node:trend".to_string(),
                to_node_id: "NQ:belief_regime_node:trend".to_string(),
                from_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                to_branch_id: "NQ:belief_regime_node:trend:transition_confirmation".to_string(),
                observations: 2,
                weighted_observation_mass: 1.5,
                wins: 2,
                losses: 0,
                invalidated: 0,
                transition_prior: 0.8,
                transition_outcome_support: 0.8,
                temporal_posterior_support: 0.86,
                weighted_success_mass: 1.5,
                weighted_failure_mass: 0.0,
                last_recommended_at: None,
            },
        );
        let mut temporal_states = BTreeMap::new();
        temporal_states.insert(
            "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:trend:transition_confirmation".to_string(),
            StructuralBranchTemporalPosteriorState {
                transition_key: "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:trend:transition_confirmation".to_string(),
                from_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                to_branch_id: "NQ:belief_regime_node:trend:transition_confirmation".to_string(),
                observations: 2,
                weighted_observation_mass: 1.5,
                transition_prior: 0.8,
                transition_outcome_support: 0.20,
                temporal_posterior_support: 0.30,
                posterior_multiplier: 0.6,
                normalized_transition_posterior: 0.8,
                summary_line: "transition_mass=1.500 transition_support=0.200 transition_temporal=0.300 multiplier=0.600".to_string(),
                last_recommended_at: None,
            },
        );

        let adjusted = transition_adjusted_branch_posteriors(
            "NQ:belief_regime_node:trend",
            &[("transition".to_string(), 0.4), ("trend".to_string(), 0.6)],
            Some("NQ:belief_regime_node:trend:trend_follow_through"),
            &priors,
            &temporal_states,
            |regime| match regime {
                "transition" => "transition_confirmation",
                _ => "trend_follow_through",
            },
        );

        assert!(
            (adjusted["NQ:belief_regime_node:trend:transition_confirmation"] - 0.8).abs() < 1e-9
        );
        assert!((adjusted["NQ:belief_regime_node:trend:trend_follow_through"] - 0.2).abs() < 1e-9);
    }

    #[test]
    fn transition_adjusted_branch_posteriors_prefers_complete_normalized_posterior_state() {
        let mut temporal_states = BTreeMap::new();
        temporal_states.insert(
            "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:trend:transition_confirmation".to_string(),
            StructuralBranchTemporalPosteriorState {
                transition_key: "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:trend:transition_confirmation".to_string(),
                from_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                to_branch_id: "NQ:belief_regime_node:trend:transition_confirmation".to_string(),
                observations: 3,
                weighted_observation_mass: 2.1,
                transition_prior: 0.5,
                transition_outcome_support: 0.8,
                temporal_posterior_support: 0.7,
                posterior_multiplier: 1.0,
                normalized_transition_posterior: 0.8,
                summary_line: String::new(),
                last_recommended_at: None,
            },
        );
        temporal_states.insert(
            "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:trend:trend_follow_through".to_string(),
            StructuralBranchTemporalPosteriorState {
                transition_key: "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                from_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                to_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                observations: 1,
                weighted_observation_mass: 0.5,
                transition_prior: 0.5,
                transition_outcome_support: 0.5,
                temporal_posterior_support: 0.5,
                posterior_multiplier: 1.0,
                normalized_transition_posterior: 0.2,
                summary_line: String::new(),
                last_recommended_at: None,
            },
        );

        let adjusted = transition_adjusted_branch_posteriors(
            "NQ:belief_regime_node:trend",
            &[("transition".to_string(), 0.5), ("trend".to_string(), 0.5)],
            Some("NQ:belief_regime_node:trend:trend_follow_through"),
            &BTreeMap::new(),
            &temporal_states,
            |regime| match regime {
                "transition" => "transition_confirmation",
                _ => "trend_follow_through",
            },
        );

        assert!(
            (adjusted["NQ:belief_regime_node:trend:transition_confirmation"] - 0.8).abs() < 1e-9
        );
        assert!((adjusted["NQ:belief_regime_node:trend:trend_follow_through"] - 0.2).abs() < 1e-9);
    }

    #[test]
    fn transition_adjusted_branch_posteriors_uses_partial_normalized_posterior_state() {
        let mut temporal_states = BTreeMap::new();
        temporal_states.insert(
            "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:trend:transition_confirmation".to_string(),
            StructuralBranchTemporalPosteriorState {
                transition_key: "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:trend:transition_confirmation".to_string(),
                from_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                to_branch_id: "NQ:belief_regime_node:trend:transition_confirmation".to_string(),
                observations: 3,
                weighted_observation_mass: 2.1,
                transition_prior: 0.7,
                transition_outcome_support: 0.8,
                temporal_posterior_support: 0.7,
                posterior_multiplier: 0.2,
                normalized_transition_posterior: 0.7,
                summary_line: String::new(),
                last_recommended_at: None,
            },
        );

        let adjusted = transition_adjusted_branch_posteriors(
            "NQ:belief_regime_node:trend",
            &[
                ("transition".to_string(), 0.4),
                ("range".to_string(), 0.3),
                ("trend".to_string(), 0.3),
            ],
            Some("NQ:belief_regime_node:trend:trend_follow_through"),
            &BTreeMap::new(),
            &temporal_states,
            |regime| match regime {
                "transition" => "transition_confirmation",
                "range" => "range_mean_reversion",
                _ => "trend_follow_through",
            },
        );

        assert!(
            (adjusted["NQ:belief_regime_node:trend:transition_confirmation"] - 0.7).abs() < 1e-9
        );
        assert!((adjusted["NQ:belief_regime_node:trend:range_mean_reversion"] - 0.15).abs() < 1e-9);
        assert!((adjusted["NQ:belief_regime_node:trend:trend_follow_through"] - 0.15).abs() < 1e-9);
    }

    #[test]
    fn transition_adjusted_branch_posteriors_use_recursive_branch_fallback() {
        let mut temporal_states = BTreeMap::new();
        temporal_states.insert(
            "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:trend:transition_confirmation".to_string(),
            StructuralBranchTemporalPosteriorState {
                transition_key: "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:trend:transition_confirmation".to_string(),
                from_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                to_branch_id: "NQ:belief_regime_node:trend:transition_confirmation".to_string(),
                observations: 3,
                weighted_observation_mass: 2.1,
                transition_prior: 0.7,
                transition_outcome_support: 0.8,
                temporal_posterior_support: 0.7,
                posterior_multiplier: 1.2,
                normalized_transition_posterior: 0.7,
                summary_line: String::new(),
                last_recommended_at: None,
            },
        );
        temporal_states.insert(
            "NQ:belief_regime_node:trend:transition_confirmation=>NQ:belief_regime_node:trend:range_mean_reversion".to_string(),
            StructuralBranchTemporalPosteriorState {
                transition_key: "NQ:belief_regime_node:trend:transition_confirmation=>NQ:belief_regime_node:trend:range_mean_reversion".to_string(),
                from_branch_id: "NQ:belief_regime_node:trend:transition_confirmation".to_string(),
                to_branch_id: "NQ:belief_regime_node:trend:range_mean_reversion".to_string(),
                observations: 2,
                weighted_observation_mass: 1.8,
                transition_prior: 0.8,
                transition_outcome_support: 0.75,
                temporal_posterior_support: 0.78,
                posterior_multiplier: 1.1,
                normalized_transition_posterior: 0.8,
                summary_line: String::new(),
                last_recommended_at: None,
            },
        );

        let adjusted = transition_adjusted_branch_posteriors(
            "NQ:belief_regime_node:trend",
            &[
                ("transition".to_string(), 0.4),
                ("range".to_string(), 0.3),
                ("trend".to_string(), 0.3),
            ],
            Some("NQ:belief_regime_node:trend:trend_follow_through"),
            &BTreeMap::new(),
            &temporal_states,
            |regime| match regime {
                "transition" => "transition_confirmation",
                "range" => "range_mean_reversion",
                _ => "trend_follow_through",
            },
        );

        assert!(
            (adjusted["NQ:belief_regime_node:trend:transition_confirmation"] - 0.7).abs() < 1e-9
        );
        assert!((adjusted["NQ:belief_regime_node:trend:range_mean_reversion"] - 0.28).abs() < 1e-9);
        assert!((adjusted["NQ:belief_regime_node:trend:trend_follow_through"] - 0.02).abs() < 1e-9);
    }

    #[test]
    fn transition_adjusted_node_posteriors_use_maintained_branch_transition_state() {
        let mut transition_priors = BTreeMap::new();
        let key = "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:transition:transition_confirmation".to_string();
        transition_priors.insert(
            key.clone(),
            StructuralBranchTransitionPrior {
                from_node_id: "NQ:belief_regime_node:trend".to_string(),
                to_node_id: "NQ:belief_regime_node:transition".to_string(),
                from_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                to_branch_id: "NQ:belief_regime_node:transition:transition_confirmation"
                    .to_string(),
                observations: 3,
                weighted_observation_mass: 2.1,
                wins: 2,
                losses: 1,
                invalidated: 0,
                transition_prior: 0.7,
                transition_outcome_support: 0.8,
                temporal_posterior_support: 0.7,
                weighted_success_mass: 1.4,
                weighted_failure_mass: 0.7,
                last_recommended_at: None,
            },
        );
        let mut temporal_states = BTreeMap::new();
        temporal_states.insert(
            key.clone(),
            StructuralBranchTemporalPosteriorState {
                transition_key: key,
                from_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                to_branch_id: "NQ:belief_regime_node:transition:transition_confirmation"
                    .to_string(),
                observations: 3,
                weighted_observation_mass: 2.1,
                transition_prior: 0.7,
                transition_outcome_support: 0.8,
                temporal_posterior_support: 0.7,
                posterior_multiplier: 0.2,
                normalized_transition_posterior: 0.7,
                summary_line: String::new(),
                last_recommended_at: None,
            },
        );

        let adjusted = transition_adjusted_node_posteriors(
            "NQ",
            &[
                ("trend".to_string(), 0.6),
                ("range".to_string(), 0.2),
                ("transition".to_string(), 0.2),
            ],
            Some("NQ:belief_regime_node:trend:trend_follow_through"),
            &transition_priors,
            &temporal_states,
            &BTreeMap::new(),
        );

        assert!((adjusted["transition"] - 0.7).abs() < 1e-9);
        assert!((adjusted["trend"] - 0.225).abs() < 1e-9);
        assert!((adjusted["range"] - 0.075).abs() < 1e-9);
    }

    #[test]
    fn transition_adjusted_node_posteriors_prefer_node_transition_state() {
        let mut node_states = BTreeMap::new();
        node_states.insert(
            "NQ:belief_regime_node:trend=>NQ:belief_regime_node:transition".to_string(),
            StructuralNodeTransitionPosteriorState {
                transition_key: "NQ:belief_regime_node:trend=>NQ:belief_regime_node:transition"
                    .to_string(),
                from_node_id: "NQ:belief_regime_node:trend".to_string(),
                to_node_id: "NQ:belief_regime_node:transition".to_string(),
                observations: 3,
                weighted_observation_mass: 2.1,
                transition_prior: 0.7,
                weighted_success_mass: 1.4,
                weighted_failure_mass: 0.7,
                transition_outcome_support: 0.8,
                temporal_posterior_support: 0.7,
                posterior_multiplier: 1.2,
                normalized_transition_posterior: 0.8,
                summary_line: String::new(),
                last_recommended_at: None,
            },
        );

        let adjusted = transition_adjusted_node_posteriors(
            "NQ",
            &[
                ("trend".to_string(), 0.6),
                ("range".to_string(), 0.2),
                ("transition".to_string(), 0.2),
            ],
            Some("NQ:belief_regime_node:trend:trend_follow_through"),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &node_states,
        );

        assert!((adjusted["transition"] - 0.8).abs() < 1e-9);
        assert!((adjusted["trend"] - 0.15).abs() < 1e-9);
        assert!((adjusted["range"] - 0.05).abs() < 1e-9);
    }

    #[test]
    fn transition_adjusted_node_posteriors_use_discounted_two_step_fallback() {
        let mut node_states = BTreeMap::new();
        node_states.insert(
            "NQ:belief_regime_node:trend=>NQ:belief_regime_node:transition".to_string(),
            StructuralNodeTransitionPosteriorState {
                transition_key: "NQ:belief_regime_node:trend=>NQ:belief_regime_node:transition"
                    .to_string(),
                from_node_id: "NQ:belief_regime_node:trend".to_string(),
                to_node_id: "NQ:belief_regime_node:transition".to_string(),
                observations: 3,
                weighted_observation_mass: 2.1,
                transition_prior: 0.7,
                weighted_success_mass: 1.4,
                weighted_failure_mass: 0.7,
                transition_outcome_support: 0.8,
                temporal_posterior_support: 0.7,
                posterior_multiplier: 1.2,
                normalized_transition_posterior: 0.7,
                summary_line: String::new(),
                last_recommended_at: None,
            },
        );
        node_states.insert(
            "NQ:belief_regime_node:transition=>NQ:belief_regime_node:range".to_string(),
            StructuralNodeTransitionPosteriorState {
                transition_key: "NQ:belief_regime_node:transition=>NQ:belief_regime_node:range"
                    .to_string(),
                from_node_id: "NQ:belief_regime_node:transition".to_string(),
                to_node_id: "NQ:belief_regime_node:range".to_string(),
                observations: 2,
                weighted_observation_mass: 1.6,
                transition_prior: 0.8,
                weighted_success_mass: 1.2,
                weighted_failure_mass: 0.4,
                transition_outcome_support: 0.75,
                temporal_posterior_support: 0.78,
                posterior_multiplier: 1.1,
                normalized_transition_posterior: 0.8,
                summary_line: String::new(),
                last_recommended_at: None,
            },
        );

        let adjusted = transition_adjusted_node_posteriors(
            "NQ",
            &[
                ("trend".to_string(), 0.6),
                ("range".to_string(), 0.2),
                ("transition".to_string(), 0.2),
            ],
            Some("NQ:belief_regime_node:trend:trend_follow_through"),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &node_states,
        );

        assert!((adjusted["transition"] - 0.7).abs() < 1e-9);
        assert!((adjusted["range"] - 0.28).abs() < 1e-9);
        assert!((adjusted["trend"] - 0.02).abs() < 1e-9);
    }

    #[test]
    fn transition_adjusted_node_posteriors_use_recursive_fallback_beyond_two_steps() {
        let mut node_states = BTreeMap::new();
        node_states.insert(
            "NQ:belief_regime_node:trend=>NQ:belief_regime_node:transition".to_string(),
            StructuralNodeTransitionPosteriorState {
                transition_key: "NQ:belief_regime_node:trend=>NQ:belief_regime_node:transition"
                    .to_string(),
                from_node_id: "NQ:belief_regime_node:trend".to_string(),
                to_node_id: "NQ:belief_regime_node:transition".to_string(),
                observations: 3,
                weighted_observation_mass: 2.1,
                transition_prior: 0.7,
                weighted_success_mass: 1.4,
                weighted_failure_mass: 0.7,
                transition_outcome_support: 0.8,
                temporal_posterior_support: 0.7,
                posterior_multiplier: 1.2,
                normalized_transition_posterior: 0.5,
                summary_line: String::new(),
                last_recommended_at: None,
            },
        );
        node_states.insert(
            "NQ:belief_regime_node:transition=>NQ:belief_regime_node:range".to_string(),
            StructuralNodeTransitionPosteriorState {
                transition_key: "NQ:belief_regime_node:transition=>NQ:belief_regime_node:range"
                    .to_string(),
                from_node_id: "NQ:belief_regime_node:transition".to_string(),
                to_node_id: "NQ:belief_regime_node:range".to_string(),
                observations: 2,
                weighted_observation_mass: 1.6,
                transition_prior: 0.8,
                weighted_success_mass: 1.2,
                weighted_failure_mass: 0.4,
                transition_outcome_support: 0.75,
                temporal_posterior_support: 0.78,
                posterior_multiplier: 1.1,
                normalized_transition_posterior: 0.4,
                summary_line: String::new(),
                last_recommended_at: None,
            },
        );
        node_states.insert(
            "NQ:belief_regime_node:range=>NQ:belief_regime_node:trend".to_string(),
            StructuralNodeTransitionPosteriorState {
                transition_key: "NQ:belief_regime_node:range=>NQ:belief_regime_node:trend"
                    .to_string(),
                from_node_id: "NQ:belief_regime_node:range".to_string(),
                to_node_id: "NQ:belief_regime_node:trend".to_string(),
                observations: 2,
                weighted_observation_mass: 1.3,
                transition_prior: 0.65,
                weighted_success_mass: 0.9,
                weighted_failure_mass: 0.4,
                transition_outcome_support: 0.69,
                temporal_posterior_support: 0.67,
                posterior_multiplier: 1.05,
                normalized_transition_posterior: 0.6,
                summary_line: String::new(),
                last_recommended_at: None,
            },
        );

        let adjusted = transition_adjusted_node_posteriors(
            "NQ",
            &[
                ("trend".to_string(), 0.6),
                ("range".to_string(), 0.2),
                ("transition".to_string(), 0.2),
            ],
            Some("NQ:belief_regime_node:trend:trend_follow_through"),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &node_states,
        );

        assert!(adjusted["trend"] > 0.0);
        assert!(adjusted["transition"] > adjusted["range"]);
        assert!(adjusted["range"] > adjusted["trend"]);
    }
}
