use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

use crate::bbn::dag::BayesianNetwork;
use crate::bbn::node::ParentConfig;

#[derive(Debug, Deserialize)]
pub struct LogicFamilyOverlayFile {
    pub schema_version: String,
    pub source: String,
    pub families: HashMap<String, LogicFamilyOverlay>,
}

#[derive(Debug, Deserialize)]
pub struct LogicFamilyOverlay {
    #[serde(default)]
    pub entry_quality: HashMap<String, Vec<f64>>,
    #[serde(default)]
    pub trade_outcome: HashMap<String, Vec<f64>>,
}

pub fn load_logic_family_overlays(path: impl AsRef<Path>) -> Result<LogicFamilyOverlayFile> {
    let path = path.as_ref();
    let text = fs::read_to_string(path).with_context(|| {
        format!(
            "failed to read logic-family overlay file {}",
            path.display()
        )
    })?;
    let parsed: LogicFamilyOverlayFile = serde_json::from_str(&text).with_context(|| {
        format!(
            "failed to parse logic-family overlay file {}",
            path.display()
        )
    })?;
    Ok(parsed)
}

fn parse_parent_config(key: &str) -> Result<ParentConfig> {
    serde_json::from_str(key).map_err(|err| anyhow!("invalid parent config key '{}': {}", key, err))
}

pub fn apply_trading_cpt_family_overlay(
    network: &mut BayesianNetwork,
    overlays: &LogicFamilyOverlayFile,
    family: &str,
) -> Result<bool> {
    if overlays.schema_version != "tomac-logic-family-overlay-v1" {
        bail!(
            "unsupported logic-family overlay schema: {}",
            overlays.schema_version
        );
    }
    let Some(overlay) = overlays.families.get(family) else {
        return Ok(false);
    };

    if let Some(node) = network.nodes.get_mut("entry_quality") {
        for (key, probs) in &overlay.entry_quality {
            node.cpt.insert(parse_parent_config(key)?, probs.clone());
        }
        node.validate()?;
    }
    if let Some(node) = network.nodes.get_mut("trade_outcome") {
        for (key, probs) in &overlay.trade_outcome {
            node.cpt.insert(parse_parent_config(key)?, probs.clone());
        }
        node.validate()?;
    }
    network.topological_sort()?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bbn::trading::topology::build_trading_network;
    use std::path::PathBuf;

    fn policy_training_fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/policy_training")
            .join(name)
    }

    #[test]
    fn applies_known_family_overlay() {
        let mut network = build_trading_network().unwrap();
        let overlays = load_logic_family_overlays(policy_training_fixture(
            "repo_bbn_logic_family_overlays.json",
        ))
        .unwrap();
        let applied =
            apply_trading_cpt_family_overlay(&mut network, &overlays, "purified_sweep").unwrap();
        assert!(applied);
        let trade = network.nodes.get("trade_outcome").unwrap();
        let row = trade.cpt.get(&vec![1, 0, 0]).unwrap();
        assert!((row[0] - 0.61).abs() < 1e-9);
    }
}
