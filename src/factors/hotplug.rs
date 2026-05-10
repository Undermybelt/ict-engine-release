use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::factors::registry::FactorRegistry;

pub const FACTOR_HOTPLUG_CONFIG_FILE: &str = "factor_hotplug.yaml";
pub const FACTOR_HOTPLUG_ENV_VAR: &str = "ICT_ENGINE_FACTOR_HOTPLUG_CONFIG";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FactorHotplugConfig {
    /// Map of factor family snake_case name → enabled (true/false).
    /// Missing keys default to true (family enabled).
    pub families: BTreeMap<String, bool>,
}

impl Default for FactorHotplugConfig {
    fn default() -> Self {
        let mut families = BTreeMap::new();
        for name in &[
            "trend_momentum",
            "volatility_mean_reversion",
            "structure_ict",
            "cross_market_smt",
            "options_hedging",
            "crowding_herding",
            "spectral_rhythm",
            "session_liquidity",
        ] {
            families.insert(name.to_string(), true);
        }
        Self { families }
    }
}

impl FactorHotplugConfig {
    pub fn is_enabled(&self, family_name: &str) -> bool {
        self.families.get(family_name).copied().unwrap_or(true)
    }

    pub fn resolve_config_path(state_dir: &str) -> PathBuf {
        if let Ok(custom) = std::env::var(FACTOR_HOTPLUG_ENV_VAR) {
            let trimmed = custom.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed);
            }
        }
        Path::new(state_dir).join(FACTOR_HOTPLUG_CONFIG_FILE)
    }

    pub fn load(state_dir: &str) -> Result<Option<Self>> {
        let path = Self::resolve_config_path(state_dir);
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("reading factor hotplug config '{}'", path.display()))?;
        let config: Self = serde_yaml::from_str(&content)
            .with_context(|| format!("parsing factor hotplug config '{}'", path.display()))?;
        Ok(Some(config))
    }

    pub fn apply_to_registry(&self, registry: &mut FactorRegistry) {
        for (name, enabled) in &self.families {
            registry.set_enabled(name, *enabled);
        }
    }

    pub fn apply_to_registry_if_present(state_dir: &str, registry: &mut FactorRegistry) {
        if let Ok(Some(config)) = Self::load(state_dir) {
            config.apply_to_registry(registry);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_all_enabled() {
        let config = FactorHotplugConfig::default();
        assert!(config.is_enabled("trend_momentum"));
        assert!(config.is_enabled("crowding_herding"));
        assert!(config.is_enabled("session_liquidity"));
        // Unknown families default to true
        assert!(config.is_enabled("unknown_future_factor"));
    }

    #[test]
    fn test_custom_config_disables_family() {
        let mut config = FactorHotplugConfig::default();
        config.families.insert("options_hedging".to_string(), false);
        assert!(!config.is_enabled("options_hedging"));
        assert!(config.is_enabled("trend_momentum"));
    }

    #[test]
    fn test_apply_to_registry() {
        let mut registry = FactorRegistry::default();
        assert!(registry.get("options_hedging").unwrap().enabled);
        let mut config = FactorHotplugConfig::default();
        config.families.insert("options_hedging".to_string(), false);
        config.apply_to_registry(&mut registry);
        assert!(!registry.get("options_hedging").unwrap().enabled);
    }
}
