#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WeightedBetaMassUpdate {
    pub observation_mass: f64,
    pub success_mass: f64,
    pub failure_mass: f64,
}

impl WeightedBetaMassUpdate {
    pub fn apply_to(
        self,
        observation_mass: &mut f64,
        success_mass: &mut f64,
        failure_mass: &mut f64,
    ) {
        *observation_mass += self.observation_mass;
        *success_mass += self.success_mass;
        *failure_mass += self.failure_mass;
    }
}

pub fn beta_update_factor(source_weight: f64, quality_weight: f64, recency_weight: f64) -> f64 {
    source_weight.max(0.0) * quality_weight.max(0.0) * recency_weight.max(0.0)
}

pub fn weighted_beta_update(
    alpha: f64,
    beta: f64,
    success_mass: f64,
    failure_mass: f64,
    source_weight: f64,
    quality_weight: f64,
    recency_weight: f64,
) -> (f64, f64) {
    let factor = beta_update_factor(source_weight, quality_weight, recency_weight);
    (
        alpha.max(0.0) + factor * success_mass.max(0.0),
        beta.max(0.0) + factor * failure_mass.max(0.0),
    )
}

pub fn weighted_beta_observation_update(
    observation_mass: f64,
    success_mass: f64,
    failure_mass: f64,
    source_weight: f64,
    quality_weight: f64,
    recency_weight: f64,
) -> WeightedBetaMassUpdate {
    let factor = beta_update_factor(source_weight, quality_weight, recency_weight);
    let (success_mass, failure_mass) = weighted_beta_update(
        0.0,
        0.0,
        success_mass,
        failure_mass,
        source_weight,
        quality_weight,
        recency_weight,
    );
    WeightedBetaMassUpdate {
        observation_mass: factor * observation_mass.max(0.0),
        success_mass,
        failure_mass,
    }
}

pub fn weighted_success_credit_beta_update(
    success_credit: f64,
    source_weight: f64,
    quality_weight: f64,
    recency_weight: f64,
) -> WeightedBetaMassUpdate {
    let success_credit = success_credit.clamp(0.0, 1.0);
    weighted_beta_observation_update(
        1.0,
        success_credit,
        1.0 - success_credit,
        source_weight,
        quality_weight,
        recency_weight,
    )
}

pub fn weighted_seed_beta_update(input: WeightedSeedBetaUpdateInput) -> WeightedBetaMassUpdate {
    let WeightedSeedBetaUpdateInput {
        followed_observation_count,
        wins,
        losses,
        breakevens,
        invalidated,
        abandoned,
        source_weight,
        quality_weight,
        recency_weight,
    } = input;
    weighted_beta_observation_update(
        followed_observation_count as f64,
        wins as f64 + breakevens as f64 * 0.5,
        losses as f64
            + breakevens as f64 * 0.5
            + invalidated as f64 * 1.25
            + abandoned as f64 * 0.75,
        source_weight,
        quality_weight,
        recency_weight,
    )
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WeightedSeedBetaUpdateInput {
    pub followed_observation_count: usize,
    pub wins: usize,
    pub losses: usize,
    pub breakevens: usize,
    pub invalidated: usize,
    pub abandoned: usize,
    pub source_weight: f64,
    pub quality_weight: f64,
    pub recency_weight: f64,
}

pub fn beta_posterior_mean(success_mass: f64, failure_mass: f64) -> f64 {
    let alpha = 1.0 + success_mass.max(0.0);
    let beta = 1.0 + failure_mass.max(0.0);
    (alpha / (alpha + beta)).clamp(0.0, 1.0)
}

pub fn beta_posterior_lower_bound(success_mass: f64, failure_mass: f64, z_score: f64) -> f64 {
    let mean = beta_posterior_mean(success_mass, failure_mass);
    let sample_size = 2.0 + success_mass.max(0.0) + failure_mass.max(0.0);
    let standard_error = (mean * (1.0 - mean) / (sample_size + 1.0)).sqrt();
    (mean - z_score.abs() * standard_error).clamp(0.0, 1.0)
}

pub fn dirichlet_component_mean(component_mass: f64, total_mass: f64) -> f64 {
    if total_mass <= f64::EPSILON {
        0.0
    } else {
        (component_mass.max(0.0) / total_mass.max(f64::EPSILON)).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        beta_posterior_lower_bound, beta_posterior_mean, beta_update_factor,
        dirichlet_component_mean, weighted_seed_beta_update, weighted_success_credit_beta_update,
        WeightedSeedBetaUpdateInput,
    };

    #[test]
    fn weighted_success_credit_update_scales_all_masses() {
        let update = weighted_success_credit_beta_update(0.25, 0.8, 1.25, 0.5);
        assert!((update.observation_mass - 0.5).abs() < 1e-9);
        assert!((update.success_mass - 0.125).abs() < 1e-9);
        assert!((update.failure_mass - 0.375).abs() < 1e-9);
    }

    #[test]
    fn weighted_seed_update_matches_structural_outcome_heuristic() {
        let update = weighted_seed_beta_update(WeightedSeedBetaUpdateInput {
            followed_observation_count: 3,
            wins: 1,
            losses: 0,
            breakevens: 1,
            invalidated: 1,
            abandoned: 0,
            source_weight: 0.75,
            quality_weight: 0.5,
            recency_weight: 1.0,
        });
        let expected_factor = beta_update_factor(0.75, 0.5, 1.0);
        let expected_success_mass = expected_factor * (1.0 + 0.5);
        let expected_failure_mass = expected_factor * (0.5 + 1.25);
        assert!((update.observation_mass - 1.125).abs() < 1e-9);
        assert!((update.success_mass - expected_success_mass).abs() < 1e-9);
        assert!((update.failure_mass - expected_failure_mass).abs() < 1e-9);
    }

    #[test]
    fn beta_lower_bound_is_below_mean_for_sparse_evidence() {
        let mean = beta_posterior_mean(2.0, 1.0);
        let lower_bound = beta_posterior_lower_bound(2.0, 1.0, 1.64);
        assert!(lower_bound < mean);
        assert!(lower_bound >= 0.0);
    }

    #[test]
    fn dirichlet_component_mean_returns_zero_without_mass() {
        assert_eq!(dirichlet_component_mean(2.0, 0.0), 0.0);
        assert!((dirichlet_component_mean(2.0, 8.0) - 0.25).abs() < 1e-9);
    }
}
