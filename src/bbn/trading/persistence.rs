//! Load / fallback persistence for the trading-domain Bayesian network.
//!
//! Centralises the snapshot-or-rebuild logic so the binary CLI
//! (`main.rs`) and library command entry points
//! (`application::auto_quant::command_entry`) share a single
//! implementation rather than maintaining duplicate copies.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use crate::bbn::BayesianNetwork;
use crate::state::{load_state, state_exists, BBN_STATE_FILE};

use super::topology::{build_trading_network, upgrade_trading_network};

pub const BBN_STRUCTURE_LEARNING_EXPORT_FILE: &str = "bbn_structure_learning_rows.jsonl";
pub const BBN_STRUCTURE_CANDIDATE_ARTIFACT_FILE: &str = "bbn_structure_candidate_artifact.json";
pub const BBN_STRUCTURE_CANDIDATE_PROTOCOL_VERSION: &str = "bbn-structure-candidate-v1";

const TRADING_BBN_ALLOWED_NODES: &[&str] = &[
    "market_regime",
    "liquidity_context",
    "factor_alignment",
    "factor_uncertainty",
    "multi_timeframe_resonance",
    "entry_quality",
    "trade_outcome",
];

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct BbnStructureLearningExportRow {
    pub market_regime: String,
    pub liquidity_context: String,
    pub factor_alignment: String,
    pub factor_uncertainty: String,
    pub multi_timeframe_resonance: String,
    pub entry_quality: String,
    pub trade_outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct BbnStructureEdge {
    pub parent: String,
    pub child: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbnStructureCandidateArtifact {
    pub protocol_version: String,
    pub required_edges_satisfied: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forbidden_edges_violated: Vec<BbnStructureEdge>,
    pub max_parent_count: usize,
    pub score_name: String,
    pub score_value: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub structure_edges: Vec<BbnStructureEdge>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub cpt_overrides: BTreeMap<String, serde_json::Value>,
    pub source_dataset_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbnStructureCandidateReviewSummary {
    pub required_edges_satisfied: bool,
    pub forbidden_edges_violated_count: usize,
    pub candidate_edge_count: usize,
    pub current_edge_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_edges: Vec<BbnStructureEdge>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_edges: Vec<BbnStructureEdge>,
    pub max_parent_count: usize,
    pub score_name: String,
    pub score_value: f64,
    pub source_dataset_hash: String,
}

pub fn render_bbn_structure_learning_rows_jsonl(
    rows: &[BbnStructureLearningExportRow],
) -> Result<String> {
    let mut out = String::new();
    for row in rows {
        out.push_str(&serde_json::to_string(row)?);
        out.push('\n');
    }
    Ok(out)
}

pub fn validate_bbn_structure_candidate_artifact(
    artifact: &BbnStructureCandidateArtifact,
) -> Result<()> {
    if artifact.protocol_version.trim() != BBN_STRUCTURE_CANDIDATE_PROTOCOL_VERSION {
        anyhow::bail!(
            "unsupported BBN structure candidate protocol version '{}'",
            artifact.protocol_version
        );
    }
    if artifact.max_parent_count == 0 {
        anyhow::bail!("BBN structure candidate max_parent_count must be greater than zero");
    }
    if artifact.score_name.trim().is_empty() {
        anyhow::bail!("BBN structure candidate score_name must not be empty");
    }
    if artifact.source_dataset_hash.trim().is_empty() {
        anyhow::bail!("BBN structure candidate source_dataset_hash must not be empty");
    }
    let allowed = TRADING_BBN_ALLOWED_NODES
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let mut parent_counts = BTreeMap::<String, usize>::new();
    for edge in &artifact.structure_edges {
        if !allowed.contains(edge.parent.as_str()) || !allowed.contains(edge.child.as_str()) {
            anyhow::bail!(
                "BBN structure candidate uses unknown node edge '{} -> {}'",
                edge.parent,
                edge.child
            );
        }
        if edge.parent == edge.child {
            anyhow::bail!(
                "BBN structure candidate contains self edge '{} -> {}'",
                edge.parent,
                edge.child
            );
        }
        let count = parent_counts.entry(edge.child.clone()).or_insert(0);
        *count += 1;
        if *count > artifact.max_parent_count {
            anyhow::bail!(
                "BBN structure candidate exceeds max_parent_count={} at child '{}'",
                artifact.max_parent_count,
                edge.child
            );
        }
    }
    for edge in &artifact.forbidden_edges_violated {
        if edge.parent.trim().is_empty() || edge.child.trim().is_empty() {
            anyhow::bail!(
                "BBN structure candidate forbidden_edges_violated must be fully specified"
            );
        }
    }
    Ok(())
}

pub fn load_bbn_structure_candidate_artifact<P: AsRef<Path>>(
    path: P,
) -> Result<BbnStructureCandidateArtifact> {
    let raw = std::fs::read_to_string(path)?;
    let artifact = serde_json::from_str::<BbnStructureCandidateArtifact>(&raw)?;
    validate_bbn_structure_candidate_artifact(&artifact)?;
    Ok(artifact)
}

pub fn review_bbn_structure_candidate_artifact(
    artifact: &BbnStructureCandidateArtifact,
    network: &BayesianNetwork,
) -> BbnStructureCandidateReviewSummary {
    let current_edges = network
        .edges
        .iter()
        .map(|edge| BbnStructureEdge {
            parent: edge.0.clone(),
            child: edge.1.clone(),
        })
        .collect::<Vec<_>>();
    let added_edges = artifact
        .structure_edges
        .iter()
        .filter(|edge| !current_edges.contains(edge))
        .cloned()
        .collect::<Vec<_>>();
    let removed_edges = current_edges
        .iter()
        .filter(|edge| !artifact.structure_edges.contains(edge))
        .cloned()
        .collect::<Vec<_>>();
    BbnStructureCandidateReviewSummary {
        required_edges_satisfied: artifact.required_edges_satisfied,
        forbidden_edges_violated_count: artifact.forbidden_edges_violated.len(),
        candidate_edge_count: artifact.structure_edges.len(),
        current_edge_count: current_edges.len(),
        added_edges,
        removed_edges,
        max_parent_count: artifact.max_parent_count,
        score_name: artifact.score_name.clone(),
        score_value: artifact.score_value,
        source_dataset_hash: artifact.source_dataset_hash.clone(),
    }
}

/// Load the trading network from `<state_dir>/<symbol>/bbn_network.json`
/// if a persisted snapshot exists, otherwise initialise a fresh
/// network from the canonical topology builder. A malformed snapshot
/// is logged to `stderr` and falls back to a fresh build, matching
/// the long-standing CLI behaviour.
pub fn load_or_init_trading_network(symbol: &str, state_dir: &str) -> Result<BayesianNetwork> {
    if !state_exists(state_dir, symbol, BBN_STATE_FILE) {
        return build_trading_network();
    }
    match load_state::<BayesianNetwork, _>(state_dir, symbol, BBN_STATE_FILE) {
        Ok(mut network) => {
            upgrade_trading_network(&mut network)?;
            Ok(network)
        }
        Err(err) => {
            eprintln!(
                "warning: failed to load BBN state for '{}' from '{}': {}",
                symbol, state_dir, err
            );
            build_trading_network()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{save_state, BBN_STATE_FILE};

    #[test]
    fn returns_fresh_network_when_no_snapshot_exists() {
        let temp = tempfile::tempdir().unwrap();
        let net = load_or_init_trading_network("NQ", temp.path().to_str().unwrap()).unwrap();
        assert!(
            !net.nodes.is_empty(),
            "fresh trading network must have nodes"
        );
    }

    #[test]
    fn round_trips_persisted_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let original = build_trading_network().unwrap();
        save_state(temp.path(), "NQ", BBN_STATE_FILE, &original).unwrap();
        let loaded = load_or_init_trading_network("NQ", temp.path().to_str().unwrap()).unwrap();
        // Same topology, same node count.
        assert_eq!(loaded.nodes.len(), original.nodes.len());
    }

    #[test]
    fn falls_back_to_fresh_build_on_malformed_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let symbol = "NQ";
        std::fs::create_dir_all(temp.path().join(symbol)).unwrap();
        std::fs::write(
            temp.path().join(symbol).join(BBN_STATE_FILE),
            "{not valid json",
        )
        .unwrap();
        // Must not bubble up the parse error.
        let net = load_or_init_trading_network(symbol, temp.path().to_str().unwrap()).unwrap();
        assert!(!net.nodes.is_empty());
    }

    #[test]
    fn bbn_structure_learning_export_rows_jsonl_round_trip() {
        let rows = vec![BbnStructureLearningExportRow {
            market_regime: "bull".to_string(),
            liquidity_context: "favorable".to_string(),
            factor_alignment: "bullish".to_string(),
            factor_uncertainty: "low".to_string(),
            multi_timeframe_resonance: "aligned".to_string(),
            entry_quality: "high".to_string(),
            trade_outcome: "win".to_string(),
        }];

        let jsonl = render_bbn_structure_learning_rows_jsonl(&rows).unwrap();
        assert!(jsonl.contains("\"trade_outcome\":\"win\""));
        assert_eq!(jsonl.lines().count(), 1);
    }

    #[test]
    fn validate_bbn_structure_candidate_artifact_rejects_unknown_nodes() {
        let artifact = BbnStructureCandidateArtifact {
            protocol_version: BBN_STRUCTURE_CANDIDATE_PROTOCOL_VERSION.to_string(),
            required_edges_satisfied: true,
            max_parent_count: 2,
            score_name: "bic".to_string(),
            score_value: 1.0,
            structure_edges: vec![BbnStructureEdge {
                parent: "unknown_node".to_string(),
                child: "trade_outcome".to_string(),
            }],
            cpt_overrides: BTreeMap::new(),
            source_dataset_hash: "hash".to_string(),
            ..BbnStructureCandidateArtifact::default()
        };

        let err = validate_bbn_structure_candidate_artifact(&artifact).unwrap_err();
        assert!(err.to_string().contains("unknown node edge"));
    }

    #[test]
    fn validate_bbn_structure_candidate_artifact_rejects_parent_overflow() {
        let artifact = BbnStructureCandidateArtifact {
            protocol_version: BBN_STRUCTURE_CANDIDATE_PROTOCOL_VERSION.to_string(),
            required_edges_satisfied: true,
            max_parent_count: 1,
            score_name: "bic".to_string(),
            score_value: 1.0,
            structure_edges: vec![
                BbnStructureEdge {
                    parent: "market_regime".to_string(),
                    child: "entry_quality".to_string(),
                },
                BbnStructureEdge {
                    parent: "liquidity_context".to_string(),
                    child: "entry_quality".to_string(),
                },
            ],
            cpt_overrides: BTreeMap::new(),
            source_dataset_hash: "hash".to_string(),
            ..BbnStructureCandidateArtifact::default()
        };

        let err = validate_bbn_structure_candidate_artifact(&artifact).unwrap_err();
        assert!(err.to_string().contains("exceeds max_parent_count"));
    }

    #[test]
    fn load_bbn_structure_candidate_artifact_round_trips_valid_candidate() {
        let temp = tempfile::tempdir().unwrap();
        let candidate = BbnStructureCandidateArtifact {
            protocol_version: BBN_STRUCTURE_CANDIDATE_PROTOCOL_VERSION.to_string(),
            required_edges_satisfied: true,
            forbidden_edges_violated: Vec::new(),
            max_parent_count: 3,
            score_name: "bic".to_string(),
            score_value: 1.23,
            structure_edges: vec![
                BbnStructureEdge {
                    parent: "market_regime".to_string(),
                    child: "entry_quality".to_string(),
                },
                BbnStructureEdge {
                    parent: "entry_quality".to_string(),
                    child: "trade_outcome".to_string(),
                },
            ],
            cpt_overrides: BTreeMap::new(),
            source_dataset_hash: "hash".to_string(),
        };
        let path = temp.path().join(BBN_STRUCTURE_CANDIDATE_ARTIFACT_FILE);
        std::fs::write(&path, serde_json::to_string_pretty(&candidate).unwrap()).unwrap();

        let loaded = load_bbn_structure_candidate_artifact(&path).unwrap();
        assert_eq!(loaded, candidate);
    }

    #[test]
    fn review_bbn_structure_candidate_artifact_diffs_against_current_topology() {
        let candidate = BbnStructureCandidateArtifact {
            protocol_version: BBN_STRUCTURE_CANDIDATE_PROTOCOL_VERSION.to_string(),
            required_edges_satisfied: true,
            forbidden_edges_violated: Vec::new(),
            max_parent_count: 3,
            score_name: "bic".to_string(),
            score_value: 1.23,
            structure_edges: vec![
                BbnStructureEdge {
                    parent: "market_regime".to_string(),
                    child: "entry_quality".to_string(),
                },
                BbnStructureEdge {
                    parent: "entry_quality".to_string(),
                    child: "trade_outcome".to_string(),
                },
                BbnStructureEdge {
                    parent: "multi_timeframe_resonance".to_string(),
                    child: "trade_outcome".to_string(),
                },
            ],
            cpt_overrides: BTreeMap::new(),
            source_dataset_hash: "hash".to_string(),
        };
        let network = build_trading_network().unwrap();

        let summary = review_bbn_structure_candidate_artifact(&candidate, &network);

        assert_eq!(summary.score_name, "bic");
        assert_eq!(summary.added_edges.len(), 1);
        assert!(summary.added_edges.iter().any(
            |edge| edge.parent == "multi_timeframe_resonance" && edge.child == "trade_outcome"
        ));
        assert!(!summary.removed_edges.is_empty());
    }
}
