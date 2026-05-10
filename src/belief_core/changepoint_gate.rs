use std::collections::BTreeMap;

use crate::state::{
    structural_event_outcome_pseudo_counts, structural_prior_source_weight,
    structural_sorted_prior_events, StructuralNodeDurationBucket, StructuralNodeDurationPrior,
    StructuralNodeTemporalPosteriorState, StructuralPriorEvent,
};

const CHANGEPOINT_PROBABILITY_EPSILON: f64 = 1e-3;

#[derive(Debug, Clone, Default)]
pub(crate) struct StructuralNodeStreakRecord {
    pub(crate) streak_length: usize,
    pub(crate) weighted_success_mass: f64,
    pub(crate) weighted_failure_mass: f64,
    pub(crate) last_recommended_at: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct StructuralNodeDurationDistributionFit {
    pub(crate) distribution: Vec<StructuralNodeDurationBucket>,
    pub(crate) entropy: f64,
    pub(crate) survival_probability: f64,
    pub(crate) completion_hazard: f64,
    pub(crate) run_length_mode: usize,
    pub(crate) run_length_mode_probability: f64,
    pub(crate) run_length_tail_probability: f64,
    pub(crate) run_length_observation_mass: f64,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct StructuralNodeBocpdRecursiveRunLengthFit {
    pub(crate) reset_probability: f64,
    pub(crate) run_length_mode: usize,
    pub(crate) run_length_mode_probability: f64,
    pub(crate) expected_run_length: f64,
    pub(crate) entropy: f64,
}

pub(crate) fn structural_duration_surprise(survival_probability: f64) -> f64 {
    if survival_probability <= f64::EPSILON {
        20.0
    } else {
        (-survival_probability.clamp(f64::EPSILON, 1.0).ln()).min(20.0)
    }
}

pub(crate) fn structural_duration_break_hazard(
    last_streak_length: usize,
    expected_dwell_steps: f64,
) -> f64 {
    if last_streak_length == 0 || expected_dwell_steps <= f64::EPSILON {
        return 0.0;
    }
    let elapsed = last_streak_length as f64;
    (elapsed / (elapsed + expected_dwell_steps)).clamp(0.0, 1.0)
}

fn structural_changepoint_clamped_probability(value: f64) -> f64 {
    value.clamp(
        CHANGEPOINT_PROBABILITY_EPSILON,
        1.0 - CHANGEPOINT_PROBABILITY_EPSILON,
    )
}

fn structural_changepoint_log_odds_update(
    prior_probability: f64,
    weighted_likelihoods: &[(f64, f64)],
) -> f64 {
    let prior = structural_changepoint_clamped_probability(prior_probability);
    let prior_log_odds = (prior / (1.0 - prior)).ln();
    let evidence_log_odds = weighted_likelihoods
        .iter()
        .map(|(probability, _weight)| {
            let probability = structural_changepoint_clamped_probability(*probability);
            probability / (1.0 - probability)
        })
        .zip(weighted_likelihoods.iter().map(|(_, weight)| *weight))
        .map(|(odds, weight)| weight.clamp(0.0, 1.0) * odds.ln())
        .sum::<f64>();
    (1.0 / (1.0 + (-(prior_log_odds + evidence_log_odds)).exp())).clamp(0.0, 1.0)
}

pub(crate) fn structural_bocpd_break_probability(
    completion_hazard: f64,
    duration_surprise: f64,
    duration_outcome_support: f64,
) -> f64 {
    let surprise_pressure = if duration_surprise <= f64::EPSILON {
        0.0
    } else {
        duration_surprise / (1.0 + duration_surprise)
    };
    let negative_outcome_pressure = (1.0 - duration_outcome_support).clamp(0.0, 1.0);
    structural_changepoint_log_odds_update(
        completion_hazard.clamp(0.0, 1.0),
        &[
            (surprise_pressure.clamp(0.0, 1.0), 0.6),
            (negative_outcome_pressure, 0.4),
        ],
    )
}

pub(crate) fn structural_node_duration_distribution_fit(
    duration_length_stats: &BTreeMap<usize, (usize, f64)>,
    total_weighted_streak_mass: f64,
    elapsed_dwell_steps: usize,
) -> StructuralNodeDurationDistributionFit {
    if duration_length_stats.is_empty() || total_weighted_streak_mass <= f64::EPSILON {
        return StructuralNodeDurationDistributionFit::default();
    }

    let mut entropy = 0.0;
    let mut distribution = Vec::with_capacity(duration_length_stats.len());
    let mut run_length_mode = 0;
    let mut run_length_mode_mass = 0.0;
    let mut run_length_mode_probability = 0.0;
    for (dwell_steps, (streak_count, weighted_streak_mass)) in duration_length_stats {
        let probability = (*weighted_streak_mass / total_weighted_streak_mass).clamp(0.0, 1.0);
        if probability > f64::EPSILON {
            entropy -= probability * probability.ln();
        }
        if *weighted_streak_mass > run_length_mode_mass {
            run_length_mode = *dwell_steps;
            run_length_mode_mass = *weighted_streak_mass;
            run_length_mode_probability = probability;
        }
        let survival_mass: f64 = duration_length_stats
            .iter()
            .filter(|(candidate_steps, _)| *candidate_steps >= dwell_steps)
            .map(|(_, (_, weighted_mass))| *weighted_mass)
            .sum();
        let survival_probability = (survival_mass / total_weighted_streak_mass).clamp(0.0, 1.0);
        let completion_hazard = if survival_mass <= f64::EPSILON {
            0.0
        } else {
            (*weighted_streak_mass / survival_mass).clamp(0.0, 1.0)
        };
        distribution.push(StructuralNodeDurationBucket {
            dwell_steps: *dwell_steps,
            streak_count: *streak_count,
            weighted_streak_mass: *weighted_streak_mass,
            probability,
            survival_probability,
            completion_hazard,
        });
    }

    let elapsed_survival_mass: f64 = duration_length_stats
        .iter()
        .filter(|(candidate_steps, _)| **candidate_steps >= elapsed_dwell_steps)
        .map(|(_, (_, weighted_mass))| *weighted_mass)
        .sum();
    let elapsed_completion_mass = duration_length_stats
        .get(&elapsed_dwell_steps)
        .map(|(_, weighted_mass)| *weighted_mass)
        .unwrap_or_default();
    let survival_probability = (elapsed_survival_mass / total_weighted_streak_mass).clamp(0.0, 1.0);
    let completion_hazard = if elapsed_dwell_steps == 0 {
        0.0
    } else if elapsed_survival_mass <= f64::EPSILON {
        1.0
    } else {
        (elapsed_completion_mass / elapsed_survival_mass).clamp(0.0, 1.0)
    };

    StructuralNodeDurationDistributionFit {
        distribution,
        entropy,
        survival_probability,
        completion_hazard,
        run_length_mode,
        run_length_mode_probability,
        run_length_tail_probability: survival_probability,
        run_length_observation_mass: total_weighted_streak_mass,
    }
}

pub(crate) fn structural_node_bocpd_recursive_run_length_fit(
    distribution: &[StructuralNodeDurationBucket],
    evidence_weight: f64,
    fallback_break_probability: f64,
) -> StructuralNodeBocpdRecursiveRunLengthFit {
    if distribution.is_empty() {
        return StructuralNodeBocpdRecursiveRunLengthFit::default();
    }

    let evidence_weight = evidence_weight.clamp(0.0, 1.0);
    let fallback_break_probability = fallback_break_probability.clamp(0.0, 1.0);
    let mut posterior = BTreeMap::<usize, f64>::new();
    for bucket in distribution {
        let prior_probability = bucket.probability.clamp(0.0, 1.0);
        if prior_probability <= f64::EPSILON {
            continue;
        }
        let hazard = ((1.0 - evidence_weight) * fallback_break_probability
            + evidence_weight * bucket.completion_hazard.clamp(0.0, 1.0))
        .clamp(0.0, 1.0);
        *posterior.entry(0).or_default() += prior_probability * hazard;
        *posterior
            .entry(bucket.dwell_steps.saturating_add(1))
            .or_default() += prior_probability * (1.0 - hazard);
    }

    let total_probability: f64 = posterior.values().copied().sum();
    if total_probability <= f64::EPSILON {
        return StructuralNodeBocpdRecursiveRunLengthFit::default();
    }

    let mut fit = StructuralNodeBocpdRecursiveRunLengthFit::default();
    for (run_length, probability) in posterior {
        let probability = (probability / total_probability).clamp(0.0, 1.0);
        if run_length == 0 {
            fit.reset_probability = probability;
        }
        if probability > fit.run_length_mode_probability {
            fit.run_length_mode = run_length;
            fit.run_length_mode_probability = probability;
        }
        fit.expected_run_length += run_length as f64 * probability;
        if probability > f64::EPSILON {
            fit.entropy -= probability * probability.ln();
        }
    }
    fit
}

fn structural_node_streak_outcome_support(streak: &StructuralNodeStreakRecord) -> f64 {
    let success = streak.weighted_success_mass.max(0.0);
    let failure = streak.weighted_failure_mass.max(0.0);
    ((1.0 + success) / (2.0 + success + failure)).clamp(0.0, 1.0)
}

fn structural_node_streak_pair_change(
    previous: &StructuralNodeStreakRecord,
    current: &StructuralNodeStreakRecord,
    expected_dwell_steps: f64,
) -> f64 {
    let duration_denominator = previous
        .streak_length
        .max(current.streak_length)
        .max(expected_dwell_steps.ceil() as usize)
        .max(1) as f64;
    let duration_change =
        (current.streak_length as f64 - previous.streak_length as f64).abs() / duration_denominator;
    let outcome_change = (structural_node_streak_outcome_support(current)
        - structural_node_streak_outcome_support(previous))
    .abs();
    (duration_change.clamp(0.0, 1.0) * 0.7 + outcome_change.clamp(0.0, 1.0) * 0.3).clamp(0.0, 1.0)
}

pub(crate) fn structural_node_bocpd_sequence_change_intensity(
    streaks: &[StructuralNodeStreakRecord],
    expected_dwell_steps: f64,
) -> f64 {
    if streaks.len() < 2 {
        return 0.0;
    }
    let total_streaks = streaks.len();
    let mut weighted_change_sum = 0.0;
    let mut weighted_pair_mass = 0.0;
    for index in 1..streaks.len() {
        let previous = &streaks[index - 1];
        let current = &streaks[index];
        let recency_rank = total_streaks.saturating_sub(index + 1) as f64;
        let recency_decay = 0.85_f64.powf(recency_rank);
        let pair_change =
            structural_node_streak_pair_change(previous, current, expected_dwell_steps);
        weighted_change_sum += recency_decay * pair_change;
        weighted_pair_mass += recency_decay;
    }
    if weighted_pair_mass <= f64::EPSILON {
        0.0
    } else {
        (weighted_change_sum / weighted_pair_mass).clamp(0.0, 1.0)
    }
}

pub(crate) fn structural_node_bocpd_sequence_break_probability(
    recursive_break_probability: f64,
    sequence_change_intensity: f64,
    evidence_weight: f64,
) -> f64 {
    structural_changepoint_log_odds_update(
        recursive_break_probability.clamp(0.0, 1.0),
        &[(
            sequence_change_intensity.clamp(0.0, 1.0),
            (evidence_weight * 0.75).clamp(0.0, 0.75),
        )],
    )
}

pub(crate) fn structural_node_bocpd_sequence_recursive_run_length_fit(
    streaks: &[StructuralNodeStreakRecord],
    expected_dwell_steps: f64,
    fallback_break_probability: f64,
    evidence_weight: f64,
) -> StructuralNodeBocpdRecursiveRunLengthFit {
    if streaks.len() < 2 {
        return StructuralNodeBocpdRecursiveRunLengthFit::default();
    }

    let evidence_weight = evidence_weight.clamp(0.0, 1.0);
    let fallback_break_probability = fallback_break_probability.clamp(0.0, 1.0);
    let mut posterior = BTreeMap::<usize, f64>::new();
    posterior.insert(0, 1.0);
    for index in 1..streaks.len() {
        let sequence_change = structural_node_streak_pair_change(
            &streaks[index - 1],
            &streaks[index],
            expected_dwell_steps,
        );
        let adaptive_hazard = ((1.0 - evidence_weight) * fallback_break_probability
            + evidence_weight * sequence_change)
            .clamp(0.0, 1.0);
        let break_likelihood = (0.05 + sequence_change * 0.95).clamp(0.05, 1.0);
        let continue_likelihood = (0.05 + (1.0 - sequence_change) * 0.95).clamp(0.05, 1.0);
        let mut next = BTreeMap::<usize, f64>::new();
        for (run_length, probability) in posterior {
            if probability <= f64::EPSILON {
                continue;
            }
            *next.entry(0).or_default() += probability * adaptive_hazard * break_likelihood;
            *next
                .entry(run_length.saturating_add(1).min(64))
                .or_default() += probability * (1.0 - adaptive_hazard) * continue_likelihood;
        }
        let total_probability: f64 = next.values().copied().sum();
        if total_probability <= f64::EPSILON {
            return StructuralNodeBocpdRecursiveRunLengthFit::default();
        }
        posterior = next
            .into_iter()
            .map(|(run_length, probability)| {
                (
                    run_length,
                    (probability / total_probability).clamp(0.0, 1.0),
                )
            })
            .collect();
    }

    let mut fit = StructuralNodeBocpdRecursiveRunLengthFit::default();
    for (run_length, probability) in posterior {
        if run_length == 0 {
            fit.reset_probability = probability;
        }
        if probability > fit.run_length_mode_probability {
            fit.run_length_mode = run_length;
            fit.run_length_mode_probability = probability;
        }
        fit.expected_run_length += run_length as f64 * probability;
        if probability > f64::EPSILON {
            fit.entropy -= probability * probability.ln();
        }
    }
    fit
}

pub(crate) fn rebuild_discounted_node_duration_priors(
    node_duration_priors: &mut BTreeMap<String, StructuralNodeDurationPrior>,
    node_temporal_posteriors: &mut BTreeMap<String, StructuralNodeTemporalPosteriorState>,
    node_streaks: &BTreeMap<String, Vec<StructuralNodeStreakRecord>>,
) {
    for (node_id, streaks) in node_streaks {
        let mut prior = StructuralNodeDurationPrior::default();
        prior.streak_count = streaks.len();
        prior.observations = streaks.iter().map(|streak| streak.streak_length).sum();
        prior.total_streak_length = prior.observations;
        prior.max_streak_length = streaks
            .iter()
            .map(|streak| streak.streak_length)
            .max()
            .unwrap_or(0);
        prior.last_streak_length = streaks
            .last()
            .map(|streak| streak.streak_length)
            .unwrap_or(0);
        prior.last_recommended_at = streaks
            .last()
            .and_then(|streak| streak.last_recommended_at.clone());
        prior.avg_streak_length = if prior.streak_count == 0 {
            0.0
        } else {
            prior.total_streak_length as f64 / prior.streak_count as f64
        };

        let total_streaks = streaks.len();
        let mut weighted_streak_mass = 0.0;
        let mut weighted_length_sum = 0.0;
        let mut weighted_success_mass = 0.0;
        let mut weighted_failure_mass = 0.0;
        let mut duration_length_stats = BTreeMap::<usize, (usize, f64)>::new();
        for (index, streak) in streaks.iter().enumerate() {
            let recency_rank = total_streaks.saturating_sub(index + 1) as f64;
            let recency_decay = 0.85_f64.powf(recency_rank);
            weighted_streak_mass += recency_decay;
            weighted_length_sum += streak.streak_length as f64 * recency_decay;
            weighted_success_mass += streak.weighted_success_mass * recency_decay;
            weighted_failure_mass += streak.weighted_failure_mass * recency_decay;
            let duration_entry = duration_length_stats
                .entry(streak.streak_length)
                .or_insert((0, 0.0));
            duration_entry.0 += 1;
            duration_entry.1 += recency_decay;
        }
        prior.weighted_streak_mass = weighted_streak_mass;
        prior.weighted_success_mass = weighted_success_mass;
        prior.weighted_failure_mass = weighted_failure_mass;
        let weighted_avg_length = if weighted_streak_mass <= f64::EPSILON {
            prior.avg_streak_length
        } else {
            weighted_length_sum / weighted_streak_mass
        };
        prior.expected_dwell_steps = weighted_avg_length;
        prior.remaining_dwell_steps =
            (weighted_avg_length - prior.last_streak_length as f64).max(0.0);
        let parametric_break_hazard =
            structural_duration_break_hazard(prior.last_streak_length, prior.expected_dwell_steps);
        let duration_fit = structural_node_duration_distribution_fit(
            &duration_length_stats,
            weighted_streak_mass,
            prior.last_streak_length,
        );
        prior.duration_distribution = duration_fit.distribution;
        prior.duration_distribution_entropy = duration_fit.entropy;
        prior.empirical_duration_survival = duration_fit.survival_probability;
        prior.empirical_duration_completion_hazard = duration_fit.completion_hazard;
        let fit_weight = (weighted_streak_mass / 3.0).min(1.0);
        prior.break_hazard = ((1.0 - fit_weight) * parametric_break_hazard
            + fit_weight * prior.empirical_duration_completion_hazard)
            .clamp(0.0, 1.0);
        prior.persistence_prior =
            (weighted_avg_length / (weighted_avg_length + 1.0)).clamp(0.0, 1.0);
        let alpha = 1.0 + weighted_success_mass.max(0.0);
        let beta = 1.0 + weighted_failure_mass.max(0.0);
        prior.duration_outcome_support = (alpha / (alpha + beta)).clamp(0.0, 1.0);
        prior.bocpd_duration_surprise =
            structural_duration_surprise(prior.empirical_duration_survival);
        prior.bocpd_evidence_weight = fit_weight;
        prior.bocpd_raw_break_probability = structural_bocpd_break_probability(
            prior.empirical_duration_completion_hazard,
            prior.bocpd_duration_surprise,
            prior.duration_outcome_support,
        );
        prior.bocpd_break_probability = structural_changepoint_log_odds_update(
            parametric_break_hazard,
            &[(
                prior.bocpd_raw_break_probability,
                prior.bocpd_evidence_weight,
            )],
        );
        prior.bocpd_continue_probability = (1.0 - prior.bocpd_break_probability).clamp(0.0, 1.0);
        prior.bocpd_run_length_mode = duration_fit.run_length_mode;
        prior.bocpd_run_length_mode_probability = duration_fit.run_length_mode_probability;
        prior.bocpd_run_length_tail_probability = duration_fit.run_length_tail_probability;
        prior.bocpd_run_length_observation_mass = duration_fit.run_length_observation_mass;
        let recursive_run_length_fit = structural_node_bocpd_recursive_run_length_fit(
            &prior.duration_distribution,
            prior.bocpd_evidence_weight,
            prior.bocpd_break_probability,
        );
        prior.bocpd_recursive_reset_probability = recursive_run_length_fit.reset_probability;
        prior.bocpd_recursive_run_length_mode = recursive_run_length_fit.run_length_mode;
        prior.bocpd_recursive_run_length_mode_probability =
            recursive_run_length_fit.run_length_mode_probability;
        prior.bocpd_recursive_run_length_expected_value =
            recursive_run_length_fit.expected_run_length;
        prior.bocpd_recursive_run_length_entropy = recursive_run_length_fit.entropy;
        prior.bocpd_sequence_change_intensity =
            structural_node_bocpd_sequence_change_intensity(streaks, prior.expected_dwell_steps);
        prior.bocpd_sequence_break_probability = structural_node_bocpd_sequence_break_probability(
            prior.bocpd_recursive_reset_probability,
            prior.bocpd_sequence_change_intensity,
            prior.bocpd_evidence_weight,
        );
        let sequence_recursive_run_length_fit =
            structural_node_bocpd_sequence_recursive_run_length_fit(
                streaks,
                prior.expected_dwell_steps,
                prior.bocpd_sequence_break_probability,
                prior.bocpd_evidence_weight,
            );
        prior.bocpd_sequence_recursive_reset_probability =
            sequence_recursive_run_length_fit.reset_probability;
        prior.bocpd_sequence_recursive_run_length_mode =
            sequence_recursive_run_length_fit.run_length_mode;
        prior.bocpd_sequence_recursive_run_length_mode_probability =
            sequence_recursive_run_length_fit.run_length_mode_probability;
        prior.bocpd_sequence_recursive_run_length_expected_value =
            sequence_recursive_run_length_fit.expected_run_length;
        prior.bocpd_sequence_recursive_run_length_entropy =
            sequence_recursive_run_length_fit.entropy;
        prior.sticky_self_transition_strength = ((1.0 - prior.break_hazard) * 0.7
            + prior.duration_outcome_support * 0.3)
            .clamp(0.0, 1.0);
        prior.temporal_posterior_support =
            (prior.persistence_prior * 0.7 + prior.duration_outcome_support * 0.3).clamp(0.0, 1.0);
        let observation_weight = (prior.weighted_streak_mass / 3.0).min(1.0);
        let streak_weight = (prior.streak_count as f64 / 3.0).min(1.0);
        let posterior_blend_weight = (observation_weight * streak_weight * 0.5).clamp(0.0, 0.5);
        let temporal_state = StructuralNodeTemporalPosteriorState {
            node_id: node_id.clone(),
            observations: prior.observations,
            streak_count: prior.streak_count,
            weighted_streak_mass: prior.weighted_streak_mass,
            expected_dwell_steps: prior.expected_dwell_steps,
            remaining_dwell_steps: prior.remaining_dwell_steps,
            break_hazard: prior.break_hazard,
            sticky_self_transition_strength: prior.sticky_self_transition_strength,
            bocpd_recursive_reset_probability: prior.bocpd_recursive_reset_probability,
            bocpd_recursive_run_length_mode: prior.bocpd_recursive_run_length_mode,
            bocpd_recursive_run_length_mode_probability:
                prior.bocpd_recursive_run_length_mode_probability,
            bocpd_recursive_run_length_expected_value:
                prior.bocpd_recursive_run_length_expected_value,
            bocpd_recursive_run_length_entropy: prior.bocpd_recursive_run_length_entropy,
            bocpd_sequence_change_intensity: prior.bocpd_sequence_change_intensity,
            bocpd_sequence_break_probability: prior.bocpd_sequence_break_probability,
            bocpd_sequence_recursive_reset_probability:
                prior.bocpd_sequence_recursive_reset_probability,
            bocpd_sequence_recursive_run_length_mode:
                prior.bocpd_sequence_recursive_run_length_mode,
            bocpd_sequence_recursive_run_length_mode_probability:
                prior.bocpd_sequence_recursive_run_length_mode_probability,
            bocpd_sequence_recursive_run_length_expected_value:
                prior.bocpd_sequence_recursive_run_length_expected_value,
            bocpd_sequence_recursive_run_length_entropy:
                prior.bocpd_sequence_recursive_run_length_entropy,
            duration_outcome_support: prior.duration_outcome_support,
            temporal_posterior_support: prior.temporal_posterior_support,
            posterior_blend_weight,
            summary_line: format!(
                "duration_mass={:.3} expected_dwell={:.3} break_hazard={:.3} sequence_break={:.3} sequence_reset={:.3} sticky_self_transition={:.3} duration_support={:.3} duration_temporal={:.3} blend={:.3}",
                prior.weighted_streak_mass,
                prior.expected_dwell_steps,
                prior.break_hazard,
                prior.bocpd_sequence_break_probability,
                prior.bocpd_sequence_recursive_reset_probability,
                prior.sticky_self_transition_strength,
                prior.duration_outcome_support,
                prior.temporal_posterior_support,
                posterior_blend_weight
            ),
            last_recommended_at: prior.last_recommended_at.clone(),
        };
        node_duration_priors.insert(node_id.clone(), prior);
        node_temporal_posteriors.insert(node_id.clone(), temporal_state);
    }
}

pub(crate) fn rebuild_node_duration_priors_from_events(
    node_duration_priors: &mut BTreeMap<String, StructuralNodeDurationPrior>,
    node_temporal_posteriors: &mut BTreeMap<String, StructuralNodeTemporalPosteriorState>,
    event_ledger: &[StructuralPriorEvent],
) {
    node_duration_priors.clear();
    node_temporal_posteriors.clear();
    let events = structural_sorted_prior_events(event_ledger);
    let mut current_symbol: Option<String> = None;
    let mut current_node_id: Option<String> = None;
    let mut current_streak_length: usize = 0;
    let mut current_recommended_at: Option<String> = None;
    let mut current_streak_success_mass = 0.0;
    let mut current_streak_failure_mass = 0.0;
    let mut node_streaks = BTreeMap::<String, Vec<StructuralNodeStreakRecord>>::new();

    for event in &events {
        if current_symbol.as_deref() != Some(event.symbol.as_str()) {
            finalize_node_duration_streak(
                &mut node_streaks,
                current_node_id.take(),
                current_streak_length,
                current_streak_success_mass,
                current_streak_failure_mass,
                current_recommended_at.take(),
            );
            current_symbol = Some(event.symbol.clone());
            current_node_id = Some(event.node_id.clone());
            current_streak_length = 1;
            current_streak_success_mass = 0.0;
            current_streak_failure_mass = 0.0;
        } else if current_node_id.as_deref() == Some(event.node_id.as_str()) {
            current_streak_length += 1;
        } else {
            finalize_node_duration_streak(
                &mut node_streaks,
                current_node_id.replace(event.node_id.clone()),
                current_streak_length,
                current_streak_success_mass,
                current_streak_failure_mass,
                current_recommended_at.take(),
            );
            current_streak_length = 1;
            current_streak_success_mass = 0.0;
            current_streak_failure_mass = 0.0;
        }
        current_recommended_at = Some(event.recommended_at.clone());
        let event_weight = structural_prior_source_weight(&event.source_label);
        if let Some(pseudo_counts) =
            structural_event_outcome_pseudo_counts(event.realized_outcome.as_deref())
        {
            let weighted_observation = event_weight * pseudo_counts.observation_weight;
            current_streak_success_mass += weighted_observation * pseudo_counts.success_credit;
            current_streak_failure_mass +=
                weighted_observation * (1.0 - pseudo_counts.success_credit);
        }
    }

    finalize_node_duration_streak(
        &mut node_streaks,
        current_node_id.take(),
        current_streak_length,
        current_streak_success_mass,
        current_streak_failure_mass,
        current_recommended_at.take(),
    );
    rebuild_discounted_node_duration_priors(
        node_duration_priors,
        node_temporal_posteriors,
        &node_streaks,
    );
}

fn finalize_node_duration_streak(
    node_streaks: &mut BTreeMap<String, Vec<StructuralNodeStreakRecord>>,
    node_id: Option<String>,
    streak_length: usize,
    weighted_success_mass: f64,
    weighted_failure_mass: f64,
    last_recommended_at: Option<String>,
) {
    if streak_length == 0 {
        return;
    }
    let Some(node_id) = node_id else {
        return;
    };
    node_streaks
        .entry(node_id)
        .or_default()
        .push(StructuralNodeStreakRecord {
            streak_length,
            weighted_success_mass,
            weighted_failure_mass,
            last_recommended_at,
        });
}

#[cfg(test)]
mod tests {
    use super::{
        structural_bocpd_break_probability, structural_node_bocpd_sequence_break_probability,
    };

    #[test]
    fn bocpd_break_probability_rises_with_surprise_and_negative_outcomes() {
        let quiet = structural_bocpd_break_probability(0.35, 0.1, 0.9);
        let stressed = structural_bocpd_break_probability(0.35, 3.0, 0.2);
        assert!(stressed > quiet);
        assert!((0.0..=1.0).contains(&quiet));
        assert!((0.0..=1.0).contains(&stressed));
    }

    #[test]
    fn sequence_break_probability_increases_with_sequence_change() {
        let calm = structural_node_bocpd_sequence_break_probability(0.30, 0.10, 0.6);
        let unstable = structural_node_bocpd_sequence_break_probability(0.30, 0.85, 0.6);
        assert!(unstable > calm);
        assert!((0.0..=1.0).contains(&calm));
        assert!((0.0..=1.0).contains(&unstable));
    }
}
