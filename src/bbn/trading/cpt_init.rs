use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

use crate::bbn::dag::BayesianNetwork;
use crate::bbn::node::{ConditionalProbabilityTable, ParentConfig};

#[derive(Debug, Deserialize)]
pub struct TradingCptInitFile {
    pub schema_version: String,
    pub source_csv: String,
    pub nodes: HashMap<String, TradingCptNodeInit>,
}

#[derive(Debug, Deserialize)]
pub struct TradingCptNodeInit {
    pub states: Vec<String>,
    #[serde(default)]
    pub prior: Vec<f64>,
    #[serde(default)]
    pub parents: Vec<String>,
    pub cpt_entries: Vec<(ParentConfig, Vec<f64>)>,
}

pub fn load_trading_cpt_init(path: impl AsRef<Path>) -> Result<TradingCptInitFile> {
    let path = path.as_ref();
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read trading CPT init file {}", path.display()))?;
    let init: TradingCptInitFile = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse trading CPT init file {}", path.display()))?;
    Ok(init)
}

pub fn apply_trading_cpt_init(
    network: &mut BayesianNetwork,
    init: &TradingCptInitFile,
) -> Result<()> {
    if init.schema_version != "tomac-cpt-init-v1"
        && init.schema_version != "tomac-cpt-init-v2-smoothed"
    {
        bail!(
            "unsupported trading CPT init schema: {}",
            init.schema_version
        );
    }

    for node_id in [
        "market_regime",
        "liquidity_context",
        "entry_quality",
        "trade_outcome",
    ] {
        let node = network
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| anyhow!("network missing expected node '{}'", node_id))?;
        let seed = init
            .nodes
            .get(node_id)
            .ok_or_else(|| anyhow!("CPT init missing node '{}'", node_id))?;

        if node.states != seed.states {
            bail!(
                "state mismatch for node '{}': network={:?} init={:?}",
                node_id,
                node.states,
                seed.states
            );
        }
        let mut cpt = ConditionalProbabilityTable::new();
        if node.parents.len() == seed.parents.len() {
            for (config, probs) in &seed.cpt_entries {
                cpt.insert(config.clone(), probs.clone());
            }
        } else if node_id == "entry_quality"
            && node.parents
                == vec![
                    "market_regime".to_string(),
                    "liquidity_context".to_string(),
                    "factor_alignment".to_string(),
                    "factor_uncertainty".to_string(),
                    "multi_timeframe_resonance".to_string(),
                ]
            && seed.parents == vec!["market_regime".to_string(), "liquidity_context".to_string()]
        {
            for (config, probs) in &seed.cpt_entries {
                for alignment in 0..3 {
                    for uncertainty in 0..2 {
                        for resonance in 0..3 {
                            cpt.insert(
                                vec![config[0], config[1], alignment, uncertainty, resonance],
                                probs.clone(),
                            );
                        }
                    }
                }
            }
        } else if node_id == "trade_outcome"
            && node.parents
                == vec![
                    "entry_quality".to_string(),
                    "factor_alignment".to_string(),
                    "factor_uncertainty".to_string(),
                ]
            && seed.parents == vec!["entry_quality".to_string()]
        {
            for (config, probs) in &seed.cpt_entries {
                for alignment in 0..3 {
                    for uncertainty in 0..2 {
                        cpt.insert(vec![config[0], alignment, uncertainty], probs.clone());
                    }
                }
            }
        } else {
            bail!(
                "parent mismatch for node '{}': network={:?} init={:?}",
                node_id,
                node.parents,
                seed.parents
            );
        }
        node.cpt = cpt;
        node.validate()?;
    }

    network.topological_sort()?;
    Ok(())
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
    fn loads_and_applies_tomac_cpt_init() {
        let mut network = build_trading_network().unwrap();
        let init = load_trading_cpt_init(policy_training_fixture("repo_bbn_trading_cpt_init.json"))
            .unwrap();
        apply_trading_cpt_init(&mut network, &init).unwrap();

        let market = network.nodes.get("market_regime").unwrap();
        let prior = market.cpt.get(&Vec::new()).unwrap();
        assert_eq!(prior.len(), 3);
        assert!((prior.iter().sum::<f64>() - 1.0).abs() < 1e-6);

        let entry = network.nodes.get("entry_quality").unwrap();
        let bull_fav = entry.cpt.get(&vec![0, 0, 0, 0, 0]).unwrap();
        assert_eq!(bull_fav.len(), 3);
        assert!((bull_fav.iter().sum::<f64>() - 1.0).abs() < 1e-6);

        let outcome = network.nodes.get("trade_outcome").unwrap();
        let high = outcome.cpt.get(&vec![0, 0, 0]).unwrap();
        assert_eq!(high, &vec![1.0, 0.0, 0.0]);
    }
}
