use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParticleBeliefSummary {
    pub particle_count: usize,
    pub effective_sample_size: f64,
    pub dominant_regime: String,
    pub market_family: Option<String>,
    pub market_behavior_profile: Option<String>,
}

pub fn bootstrap_particle_summary(
    regime: &str,
    market_family: Option<&str>,
    market_behavior_profile: Option<&str>,
) -> ParticleBeliefSummary {
    let (particle_count, effective_sample_size) = match market_family {
        Some("energy") => (96, 60.0),
        Some("metals") => (80, 54.0),
        Some("futures_index") => (64, 48.0),
        _ => (64, 48.0),
    };
    ParticleBeliefSummary {
        particle_count,
        effective_sample_size,
        dominant_regime: regime.to_string(),
        market_family: market_family.map(str::to_string),
        market_behavior_profile: market_behavior_profile.map(str::to_string),
    }
}
