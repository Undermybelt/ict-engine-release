use anyhow::{anyhow, Result};

use super::{
    dag::BayesianNetwork,
    evidence::{validate_evidence, Evidence, EvidenceType},
    node::{Node, NodeId},
};

#[derive(Debug, Clone, Copy)]
pub enum InferenceMethod {
    Exact,
    Sampling(SamplingMethod),
}

#[derive(Debug, Clone, Copy)]
pub enum SamplingMethod {
    LikelihoodWeighting,
    Gibbs,
    Prior,
}

pub struct InferenceEngine;

pub struct VariableEliminationEngine;

impl VariableEliminationEngine {
    pub fn query(
        network: &BayesianNetwork,
        node_id: &str,
        evidence: &Evidence,
    ) -> Result<Vec<f64>> {
        InferenceEngine::query(network, node_id, evidence)
    }
}

impl InferenceEngine {
    pub fn query(
        network: &BayesianNetwork,
        node_id: &str,
        evidence: &Evidence,
    ) -> Result<Vec<f64>> {
        validate_evidence(evidence)?;

        let node = network
            .nodes
            .get(node_id)
            .ok_or_else(|| anyhow!("node '{}' not found", node_id))?;

        if let Some(observed) = evidence.get(node_id) {
            return Self::distribution_from_observation(node, observed);
        }

        if node.parents.is_empty() {
            return node
                .cpt
                .get(&Vec::new())
                .cloned()
                .ok_or_else(|| anyhow!("root node '{}' missing prior distribution", node_id));
        }

        node.probabilities_for_evidence(evidence)
    }

    pub fn query_with_method(
        network: &BayesianNetwork,
        node_id: &str,
        evidence: &Evidence,
        _method: InferenceMethod,
    ) -> Result<Vec<f64>> {
        Self::query(network, node_id, evidence)
    }

    pub fn most_likely_state(
        network: &BayesianNetwork,
        node_id: &str,
        evidence: &Evidence,
    ) -> Result<(usize, f64)> {
        let distribution = Self::query(network, node_id, evidence)?;
        distribution
            .iter()
            .copied()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .ok_or_else(|| anyhow!("empty distribution for node '{}'", node_id))
    }

    pub fn joint_probability(network: &BayesianNetwork, evidence: &Evidence) -> Result<f64> {
        validate_evidence(evidence)?;

        let mut joint = 1.0;
        for node_id in &network.topological_order {
            let node = network
                .nodes
                .get(node_id)
                .ok_or_else(|| anyhow!("node '{}' missing from network", node_id))?;

            let state_index = match evidence.get(node_id) {
                Some(EvidenceType::Hard(index)) => *index,
                Some(EvidenceType::Soft(_)) => continue,
                None => continue,
            };

            let distribution = if node.parents.is_empty() {
                node.cpt
                    .get(&Vec::new())
                    .cloned()
                    .ok_or_else(|| anyhow!("root node '{}' missing prior distribution", node_id))?
            } else {
                node.probabilities_for_evidence(evidence)?
            };

            joint *= distribution.get(state_index).copied().ok_or_else(|| {
                anyhow!(
                    "state index {} out of bounds for '{}'",
                    state_index,
                    node_id
                )
            })?;
        }

        Ok(joint)
    }

    fn distribution_from_observation(node: &Node, evidence: &EvidenceType) -> Result<Vec<f64>> {
        match evidence {
            EvidenceType::Hard(index) => {
                if *index >= node.states.len() {
                    return Err(anyhow!(
                        "observed state index {} out of bounds for node '{}'",
                        index,
                        node.id
                    ));
                }
                let mut distribution = vec![0.0; node.states.len()];
                distribution[*index] = 1.0;
                Ok(distribution)
            }
            EvidenceType::Soft(distribution) => {
                if distribution.len() != node.states.len() {
                    return Err(anyhow!(
                        "soft evidence length {} does not match node '{}' state count {}",
                        distribution.len(),
                        node.id,
                        node.states.len()
                    ));
                }
                Ok(distribution.clone())
            }
        }
    }
}

pub fn infer_node_distribution(
    network: &BayesianNetwork,
    node_id: &NodeId,
    evidence: &Evidence,
) -> Result<Vec<f64>> {
    InferenceEngine::query(network, node_id, evidence)
}
