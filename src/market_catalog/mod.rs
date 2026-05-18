use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::application::data_sources::harness::{
    load_market_data_harness_preset_config, MarketDataHarnessPreset, MarketDataHarnessPresetConfig,
    MarketDataHarnessSymbolSpec,
};

pub const MARKET_RELATIONSHIPS_FILE: &str = "config/market_relationships.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketRelationshipConfig {
    pub version: u32,
    #[serde(default)]
    pub markets: Vec<MarketRelationshipSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketRelationshipSpec {
    pub market_key: String,
    #[serde(default)]
    pub related_futures_symbols: Vec<String>,
    #[serde(default)]
    pub related_etf_companions: Vec<String>,
    #[serde(default)]
    pub related_options_companions: Vec<String>,
    #[serde(default)]
    pub related_cfd_symbols: Vec<String>,
    #[serde(default)]
    pub related_crypto_symbols: Vec<String>,
    #[serde(default)]
    pub options_volatility_proxy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketCatalogLiveDefaults {
    pub futures_symbol: String,
    pub spot_symbol: String,
    pub options_symbol: String,
    pub spot_kind: String,
}

#[derive(Debug, Clone)]
pub struct MarketCatalog {
    pub presets: MarketDataHarnessPresetConfig,
    pub relationships: MarketRelationshipConfig,
}

pub fn load_market_relationship_config<P: AsRef<Path>>(
    repo_root: P,
) -> Result<MarketRelationshipConfig> {
    let path = repo_root.as_ref().join(MARKET_RELATIONSHIPS_FILE);
    let raw = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "failed to read market relationship config '{}'",
            path.display()
        )
    })?;
    serde_json::from_str(&raw).with_context(|| {
        format!(
            "failed to parse market relationship config '{}'",
            path.display()
        )
    })
}

pub fn load_market_catalog<P: AsRef<Path>>(repo_root: P) -> Result<MarketCatalog> {
    let repo_root = repo_root.as_ref();
    Ok(MarketCatalog {
        presets: load_market_data_harness_preset_config(repo_root)?,
        relationships: load_market_relationship_config(repo_root)?,
    })
}

impl MarketCatalog {
    pub fn live_defaults(&self, market_key: &str) -> Option<MarketCatalogLiveDefaults> {
        let preset = self.find_preset(market_key)?;
        let defaults = preset.live_defaults.as_ref()?;
        let spot_symbol = display_symbol_for_role(preset, &defaults.spot_role)?;
        let options_symbol = display_symbol_for_role(preset, &defaults.options_role)?;
        Some(MarketCatalogLiveDefaults {
            futures_symbol: defaults.futures_symbol.clone(),
            spot_symbol,
            options_symbol,
            spot_kind: defaults.spot_kind.clone(),
        })
    }

    pub fn relationships(&self, market_key: &str) -> Option<&MarketRelationshipSpec> {
        let normalized = market_key.trim();
        self.relationships
            .markets
            .iter()
            .find(|item| item.market_key.eq_ignore_ascii_case(normalized))
    }

    pub fn market_keys_with_live_defaults(&self) -> Vec<String> {
        self.presets
            .markets
            .iter()
            .filter(|preset| preset.live_defaults.is_some())
            .map(|preset| preset.market_key.clone())
            .collect()
    }

    fn find_preset(&self, market_key: &str) -> Option<&MarketDataHarnessPreset> {
        let normalized = market_key.trim();
        self.presets.markets.iter().find(|preset| {
            preset.market_key.eq_ignore_ascii_case(normalized)
                || preset
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(normalized))
        })
    }
}

fn display_symbol_for_role(preset: &MarketDataHarnessPreset, role: &str) -> Option<String> {
    let spec = preset.related.get(role)?;
    spec.display_symbol
        .clone()
        .or_else(|| spec.yfinance.clone())
        .or_else(|| spec.tradingview_mcp.clone())
        .or_else(|| spec.ibkr.as_ref().map(|item| item.symbol.clone()))
}

#[allow(dead_code)]
fn _symbol_for_spec(spec: &MarketDataHarnessSymbolSpec) -> Option<String> {
    spec.display_symbol
        .clone()
        .or_else(|| spec.yfinance.clone())
        .or_else(|| spec.tradingview_mcp.clone())
        .or_else(|| spec.ibkr.as_ref().map(|item| item.symbol.clone()))
}
