#[derive(Debug, Clone)]
pub enum DurationDistribution {
    Geometric { p: f64 },
    NegativeBinomial { r: f64, p: f64 },
}

#[derive(Debug, Clone)]
pub struct DurationState {
    pub elapsed_bars: usize,
    pub hazard_rate: f64,
    pub survival_prob: f64,
    pub remaining_expected_bars: f64,
    pub model: String,
}

pub fn geometric_duration(mean_bars: f64) -> DurationDistribution {
    let safe_mean = mean_bars.max(1.0);
    DurationDistribution::Geometric {
        p: (1.0 / safe_mean).clamp(1e-6, 0.999999),
    }
}

pub fn negative_binomial_duration(mean_bars: f64, variance_bars: f64) -> DurationDistribution {
    let mean = mean_bars.max(1.0);
    let variance = variance_bars.max(mean + 1e-6);
    let p = (mean / variance).clamp(1e-6, 0.999999);
    let r = (mean * p / (1.0 - p)).max(1e-6);
    DurationDistribution::NegativeBinomial { r, p }
}

pub fn estimate_duration_state(
    elapsed_bars: usize,
    distribution: &DurationDistribution,
) -> DurationState {
    let elapsed = elapsed_bars.max(1) as f64;
    match distribution {
        DurationDistribution::Geometric { p } => {
            let hazard_rate = (*p).clamp(0.0, 1.0);
            let survival_prob = (1.0 - p).powf(elapsed).clamp(0.0, 1.0);
            let remaining_expected_bars = ((1.0 - p) / p).max(0.0);
            DurationState {
                elapsed_bars,
                hazard_rate,
                survival_prob,
                remaining_expected_bars,
                model: "geometric".to_string(),
            }
        }
        DurationDistribution::NegativeBinomial { r, p } => {
            let failures = (elapsed - 1.0).max(0.0);
            let next_failures = elapsed;
            let pmf_now =
                (combination_with_repetition(*r, failures) * p.powf(*r) * (1.0 - p).powf(failures))
                    .max(1e-12);
            let pmf_next = (combination_with_repetition(*r, next_failures)
                * p.powf(*r)
                * (1.0 - p).powf(next_failures))
            .max(1e-12);
            let survival_prob = (1.0 - cumulative_mass_approx(*r, *p, failures)).clamp(0.0, 1.0);
            let hazard_rate = (pmf_next / survival_prob.max(1e-6)).clamp(0.0, 1.0);
            let mean_total = (*r * (1.0 - p) / p).max(1.0);
            let remaining_expected_bars = (mean_total - elapsed).max(0.0);
            let _ = pmf_now;
            DurationState {
                elapsed_bars,
                hazard_rate,
                survival_prob,
                remaining_expected_bars,
                model: "negative_binomial".to_string(),
            }
        }
    }
}

fn combination_with_repetition(r: f64, k: f64) -> f64 {
    let mut numerator = 1.0;
    let mut denominator = 1.0;
    let steps = k.round() as usize;
    for i in 0..steps {
        numerator *= r + i as f64;
        denominator *= (i + 1) as f64;
    }
    (numerator / denominator.max(1e-12)).max(1e-12)
}

fn cumulative_mass_approx(r: f64, p: f64, max_failures: f64) -> f64 {
    let steps = max_failures.round() as usize;
    let mut total = 0.0;
    for k in 0..=steps {
        total += combination_with_repetition(r, k as f64) * p.powf(r) * (1.0 - p).powf(k as f64);
    }
    total.clamp(0.0, 1.0)
}
