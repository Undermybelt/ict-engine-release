use anyhow::Result;
use std::path::PathBuf;

use crate::bbn::{
    dag::BayesianNetwork,
    node::{ConditionalProbabilityTable, Node, NodeType},
};

use super::cpt_init::{apply_trading_cpt_init, load_trading_cpt_init};

fn trading_cpt_init_search_paths() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    vec![
        root.join("state/policy_training/repo_bbn_trading_cpt_init_smoothed.json"),
        root.join("state/policy_training/repo_bbn_trading_cpt_init.json"),
    ]
}

pub fn build_trading_network() -> Result<BayesianNetwork> {
    let mut network = BayesianNetwork::new();

    let nodes = vec![
        root_node(
            "market_regime",
            "Market Regime",
            vec!["bull", "bear", "range"],
            vec![0.35, 0.30, 0.35],
        ),
        root_node(
            "liquidity_context",
            "Liquidity Context",
            vec!["favorable", "neutral", "hostile"],
            vec![0.4, 0.35, 0.25],
        ),
        root_node(
            "factor_alignment",
            "Factor Alignment",
            vec!["bullish", "mixed", "bearish"],
            vec![0.33, 0.34, 0.33],
        ),
        root_node(
            "factor_uncertainty",
            "Factor Uncertainty",
            vec!["low", "high"],
            vec![0.65, 0.35],
        ),
        root_node(
            "multi_timeframe_resonance",
            "Multi Timeframe Resonance",
            vec!["aligned", "mixed", "dislocated"],
            vec![0.35, 0.40, 0.25],
        ),
        conditional_node(
            "entry_quality",
            "Entry Quality",
            vec!["high", "medium", "low"],
            vec![
                "market_regime",
                "liquidity_context",
                "factor_alignment",
                "factor_uncertainty",
                "multi_timeframe_resonance",
            ],
        ),
        conditional_node(
            "trade_outcome",
            "Trade Outcome",
            vec!["win", "breakeven", "loss"],
            vec!["entry_quality", "factor_alignment", "factor_uncertainty"],
        ),
    ];

    for node in nodes {
        network.add_node(node)?;
    }

    network.add_edge("market_regime".into(), "entry_quality".into())?;
    network.add_edge("liquidity_context".into(), "entry_quality".into())?;
    network.add_edge("factor_alignment".into(), "entry_quality".into())?;
    network.add_edge("factor_uncertainty".into(), "entry_quality".into())?;
    network.add_edge("multi_timeframe_resonance".into(), "entry_quality".into())?;
    network.add_edge("entry_quality".into(), "trade_outcome".into())?;
    network.add_edge("factor_alignment".into(), "trade_outcome".into())?;
    network.add_edge("factor_uncertainty".into(), "trade_outcome".into())?;

    populate_entry_quality_cpt(&mut network, None)?;
    populate_trade_outcome_cpt(&mut network, None)?;

    for path in trading_cpt_init_search_paths() {
        if let Ok(init) = load_trading_cpt_init(&path) {
            apply_trading_cpt_init(&mut network, &init)?;
            break;
        }
    }

    Ok(network)
}

pub fn upgrade_trading_network(network: &mut BayesianNetwork) -> Result<()> {
    if is_current_trading_network(network) {
        network.topological_sort()?;
        return Ok(());
    }

    let old_entry_quality = network
        .nodes
        .get("entry_quality")
        .map(|node| node.cpt.clone());
    let old_trade_outcome = network
        .nodes
        .get("trade_outcome")
        .map(|node| node.cpt.clone());

    *network = build_trading_network()?;
    populate_entry_quality_cpt(network, old_entry_quality.as_ref())?;
    populate_trade_outcome_cpt(network, old_trade_outcome.as_ref())?;
    network.topological_sort()?;
    Ok(())
}

fn is_current_trading_network(network: &BayesianNetwork) -> bool {
    matches!(
        network.nodes.get("entry_quality"),
        Some(node)
            if node.parents
                == vec![
                    "market_regime".to_string(),
                    "liquidity_context".to_string(),
                    "factor_alignment".to_string(),
                    "factor_uncertainty".to_string(),
                    "multi_timeframe_resonance".to_string()
                ]
    ) && matches!(
        network.nodes.get("trade_outcome"),
        Some(node)
            if node.parents
                == vec![
                    "entry_quality".to_string(),
                    "factor_alignment".to_string(),
                    "factor_uncertainty".to_string()
                ]
    )
}

fn root_node(id: &str, label: &str, states: Vec<&str>, prior: Vec<f64>) -> Node {
    let mut cpt = ConditionalProbabilityTable::new();
    cpt.insert(Vec::new(), prior);

    Node {
        id: id.into(),
        name: label.into(),
        node_type: NodeType::Observed,
        states: states.into_iter().map(|s| s.to_string()).collect(),
        parents: Vec::new(),
        cpt,
    }
}

fn conditional_node(id: &str, label: &str, states: Vec<&str>, parents: Vec<&str>) -> Node {
    Node {
        id: id.into(),
        name: label.into(),
        node_type: NodeType::Hidden,
        states: states.into_iter().map(|s| s.to_string()).collect(),
        parents: parents.into_iter().map(|s| s.to_string()).collect(),
        cpt: ConditionalProbabilityTable::new(),
    }
}

fn populate_entry_quality_cpt(
    network: &mut BayesianNetwork,
    previous: Option<&ConditionalProbabilityTable>,
) -> Result<()> {
    let node = network.nodes.get_mut("entry_quality").unwrap();
    node.cpt.entries.clear();

    for regime in 0..3 {
        for liquidity in 0..3 {
            let legacy_base = previous
                .and_then(|cpt| cpt.get(&vec![regime, liquidity]).cloned())
                .unwrap_or_else(|| legacy_entry_quality_distribution(regime, liquidity));
            for alignment in 0..3 {
                for uncertainty in 0..2 {
                    let preadjusted = previous
                        .and_then(|cpt| {
                            cpt.get(&vec![regime, liquidity, alignment, uncertainty])
                                .cloned()
                        })
                        .unwrap_or_else(|| {
                            adjust_entry_quality_distribution(
                                &legacy_base,
                                alignment,
                                uncertainty,
                                1,
                            )
                        });
                    for resonance in 0..3 {
                        let distribution = previous
                            .and_then(|cpt| {
                                cpt.get(&vec![regime, liquidity, alignment, uncertainty, resonance])
                                    .cloned()
                            })
                            .unwrap_or_else(|| {
                                adjust_entry_quality_resonance_only(&preadjusted, resonance)
                            });
                        node.cpt.insert(
                            vec![regime, liquidity, alignment, uncertainty, resonance],
                            distribution,
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

fn populate_trade_outcome_cpt(
    network: &mut BayesianNetwork,
    previous: Option<&ConditionalProbabilityTable>,
) -> Result<()> {
    let node = network.nodes.get_mut("trade_outcome").unwrap();
    node.cpt.entries.clear();

    for entry_quality in 0..3 {
        let legacy_base = previous
            .and_then(|cpt| cpt.get(&vec![entry_quality]).cloned())
            .unwrap_or_else(|| legacy_trade_outcome_distribution(entry_quality));
        for alignment in 0..3 {
            for uncertainty in 0..2 {
                node.cpt.insert(
                    vec![entry_quality, alignment, uncertainty],
                    previous
                        .and_then(|cpt| {
                            cpt.get(&vec![entry_quality, alignment, uncertainty])
                                .cloned()
                        })
                        .unwrap_or_else(|| {
                            adjust_trade_outcome_distribution(&legacy_base, alignment, uncertainty)
                        }),
                );
            }
        }
    }

    Ok(())
}

fn legacy_entry_quality_distribution(regime: usize, liquidity: usize) -> Vec<f64> {
    match (regime, liquidity) {
        (0, 0) => vec![0.65, 0.25, 0.10],
        (0, 1) => vec![0.50, 0.35, 0.15],
        (0, 2) => vec![0.35, 0.40, 0.25],
        (1, 0) => vec![0.40, 0.40, 0.20],
        (1, 1) => vec![0.30, 0.45, 0.25],
        (1, 2) => vec![0.20, 0.40, 0.40],
        (2, 0) => vec![0.25, 0.45, 0.30],
        (2, 1) => vec![0.20, 0.45, 0.35],
        _ => vec![0.10, 0.35, 0.55],
    }
}

fn legacy_trade_outcome_distribution(entry_quality: usize) -> Vec<f64> {
    match entry_quality {
        0 => vec![0.58, 0.22, 0.20],
        1 => vec![0.36, 0.28, 0.36],
        _ => vec![0.18, 0.17, 0.65],
    }
}

fn adjust_entry_quality_distribution(
    base: &[f64],
    alignment: usize,
    uncertainty: usize,
    resonance: usize,
) -> Vec<f64> {
    let mut adjusted = base.to_vec();
    match alignment {
        0 => {
            adjusted[0] *= 1.25;
            adjusted[2] *= 0.75;
        }
        1 => {
            adjusted[1] *= 1.10;
        }
        2 => {
            adjusted[0] *= 0.75;
            adjusted[2] *= 1.25;
        }
        _ => {}
    }

    if uncertainty == 1 {
        adjusted[0] *= 0.80;
        adjusted[1] *= 1.05;
        adjusted[2] *= 1.15;
    }

    apply_resonance_adjustment(&mut adjusted, resonance, 1.22, 1.18);

    normalize(&mut adjusted);
    adjusted
}

fn adjust_trade_outcome_distribution(
    base: &[f64],
    alignment: usize,
    uncertainty: usize,
) -> Vec<f64> {
    let mut adjusted = base.to_vec();
    match alignment {
        0 => {
            adjusted[0] *= 1.18;
            adjusted[2] *= 0.86;
        }
        1 => {
            adjusted[1] *= 1.10;
        }
        2 => {
            adjusted[0] *= 0.86;
            adjusted[2] *= 1.18;
        }
        _ => {}
    }

    if uncertainty == 1 {
        adjusted[0] *= 0.85;
        adjusted[1] *= 1.10;
        adjusted[2] *= 1.10;
    }

    normalize(&mut adjusted);
    adjusted
}

fn adjust_entry_quality_resonance_only(base: &[f64], resonance: usize) -> Vec<f64> {
    let mut adjusted = base.to_vec();
    apply_resonance_adjustment(&mut adjusted, resonance, 1.22, 1.18);
    normalize(&mut adjusted);
    adjusted
}

fn apply_resonance_adjustment(
    adjusted: &mut [f64],
    resonance: usize,
    aligned_boost: f64,
    dislocated_penalty: f64,
) {
    match resonance {
        0 => {
            adjusted[0] *= aligned_boost;
            adjusted[2] *= 0.88;
        }
        1 => {
            adjusted[1] *= 1.08;
        }
        2 => {
            adjusted[0] *= 1.0 / dislocated_penalty;
            adjusted[1] *= 1.05;
            adjusted[2] *= dislocated_penalty;
        }
        _ => {}
    }
}

fn normalize(values: &mut [f64]) {
    let sum: f64 = values.iter().sum();
    if sum <= f64::EPSILON {
        let uniform = 1.0 / values.len() as f64;
        values.fill(uniform);
    } else {
        for value in values {
            *value /= sum;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_trading_network_includes_factor_nodes() {
        let network = build_trading_network().unwrap();
        assert!(network.nodes.contains_key("factor_alignment"));
        assert!(network.nodes.contains_key("factor_uncertainty"));
        assert_eq!(
            network.nodes["entry_quality"].parents,
            vec![
                "market_regime".to_string(),
                "liquidity_context".to_string(),
                "factor_alignment".to_string(),
                "factor_uncertainty".to_string(),
                "multi_timeframe_resonance".to_string()
            ]
        );
        assert_eq!(
            network.nodes["trade_outcome"].parents,
            vec![
                "entry_quality".to_string(),
                "factor_alignment".to_string(),
                "factor_uncertainty".to_string()
            ]
        );
    }

    #[test]
    fn test_trading_network_prefers_smoothed_tomac_cpt_when_available() {
        let network = build_trading_network().unwrap();
        let trade_outcome = network.nodes.get("trade_outcome").unwrap();
        let high = trade_outcome.cpt.get(&vec![0, 0, 0]).unwrap();
        assert!(high[0] < 1.0);
        assert!(high[1] > 0.0);
        assert!(high[2] > 0.0);
    }

    #[test]
    fn test_upgrade_trading_network_migrates_legacy_topology() {
        let mut legacy = BayesianNetwork::new();
        for node in [
            root_node(
                "market_regime",
                "Market Regime",
                vec!["bull", "bear", "range"],
                vec![0.35, 0.30, 0.35],
            ),
            root_node(
                "liquidity_context",
                "Liquidity Context",
                vec!["favorable", "neutral", "hostile"],
                vec![0.4, 0.35, 0.25],
            ),
            conditional_node(
                "entry_quality",
                "Entry Quality",
                vec!["high", "medium", "low"],
                vec!["market_regime", "liquidity_context"],
            ),
            conditional_node(
                "trade_outcome",
                "Trade Outcome",
                vec!["win", "breakeven", "loss"],
                vec!["entry_quality"],
            ),
        ] {
            legacy.add_node(node).unwrap();
        }
        legacy
            .add_edge("market_regime".into(), "entry_quality".into())
            .unwrap();
        legacy
            .add_edge("liquidity_context".into(), "entry_quality".into())
            .unwrap();
        legacy
            .add_edge("entry_quality".into(), "trade_outcome".into())
            .unwrap();
        legacy
            .nodes
            .get_mut("entry_quality")
            .unwrap()
            .cpt
            .insert(vec![0, 0], vec![0.65, 0.25, 0.10]);
        legacy
            .nodes
            .get_mut("trade_outcome")
            .unwrap()
            .cpt
            .insert(vec![0], vec![0.58, 0.22, 0.20]);

        upgrade_trading_network(&mut legacy).unwrap();

        assert!(legacy.nodes.contains_key("factor_alignment"));
        assert!(legacy.nodes.contains_key("factor_uncertainty"));
        assert!(legacy.nodes.contains_key("multi_timeframe_resonance"));
        assert!(legacy.nodes["trade_outcome"]
            .cpt
            .get(&vec![0, 0, 0])
            .is_some());
    }
}
