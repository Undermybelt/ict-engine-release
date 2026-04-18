use std::collections::BTreeMap;

use crate::factor_lab::factor_definition::FactorDefinition;
use crate::state::LearningState;

#[derive(Clone)]
pub struct FactorRegistry {
    factors: BTreeMap<String, FactorDefinition>,
}

impl Default for FactorRegistry {
    fn default() -> Self {
        let mut registry = Self {
            factors: BTreeMap::new(),
        };
        registry.register(FactorDefinition::trend_momentum());
        registry.register(FactorDefinition::volatility_mean_reversion());
        registry.register(FactorDefinition::structure_ict());
        registry.register(FactorDefinition::cross_market_smt());
        registry.register(FactorDefinition::options_hedging());
        registry
    }
}

impl FactorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, factor: FactorDefinition) {
        self.factors.insert(factor.name.clone(), factor);
    }

    pub fn get(&self, name: &str) -> Option<&FactorDefinition> {
        self.factors.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut FactorDefinition> {
        self.factors.get_mut(name)
    }

    pub fn list(&self) -> Vec<&FactorDefinition> {
        self.factors.values().collect()
    }

    pub fn enabled_factors(&self) -> Vec<&FactorDefinition> {
        self.factors
            .values()
            .filter(|factor| factor.enabled)
            .collect()
    }

    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> bool {
        self.get_mut(name)
            .map(|factor| factor.enabled = enabled)
            .is_some()
    }

    pub fn set_parameter(&mut self, name: &str, key: &str, value: f64) -> bool {
        self.get_mut(name)
            .map(|factor| factor.set_parameter(key.to_string(), value))
            .is_some()
    }

    pub fn apply_learning_state(&mut self, learning_state: &LearningState) {
        for (name, profile) in &learning_state.factor_profiles {
            if let Some(factor) = self.get_mut(name) {
                factor.enabled = profile.enabled;
                for (key, value) in &profile.parameters {
                    factor.set_parameter(key.clone(), *value);
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.factors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.factors.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_supports_enabling_and_parameter_updates() {
        let mut registry = FactorRegistry::default();
        assert!(registry.len() >= 5);
        assert!(registry.set_enabled("options_hedging", false));
        assert!(!registry.get("options_hedging").unwrap().enabled);
        assert!(registry.set_parameter("trend_momentum", "fast_period", 12.0));
        assert_eq!(
            registry
                .get("trend_momentum")
                .unwrap()
                .parameters
                .get("fast_period")
                .copied(),
            Some(12.0)
        );
    }
}
