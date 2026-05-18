use anyhow::{bail, Result};

use crate::bbn::{dag::BayesianNetwork, evidence::Evidence, node::NodeId};

#[derive(Debug, Clone)]
pub struct TradeOutcome {
    pub node_id: NodeId,
    pub realized_state_index: usize,
}

#[derive(Debug, Clone)]
pub struct CPTUpdater {
    pub learning_rate: f64,
    pub prior_strength: f64,
}

impl Default for CPTUpdater {
    fn default() -> Self {
        Self {
            learning_rate: 0.1,
            prior_strength: 1.0,
        }
    }
}

impl CPTUpdater {
    pub fn update_from_trade(
        &self,
        network: &mut BayesianNetwork,
        evidence: &Evidence,
        outcome: TradeOutcome,
    ) -> Result<()> {
        let node = network
            .nodes
            .get_mut(&outcome.node_id)
            .ok_or_else(|| anyhow::anyhow!("outcome node '{}' not found", outcome.node_id))?;

        if outcome.realized_state_index >= node.states.len() {
            bail!(
                "state index {} out of range for node '{}'",
                outcome.realized_state_index,
                outcome.node_id
            );
        }

        let parent_config =
            crate::bbn::node::Node::parent_config_from_evidence(&node.parents, evidence)?;
        let state_len = node.states.len();

        let probs = node
            .cpt
            .entries
            .entry(parent_config)
            .or_insert_with(|| vec![1.0 / state_len as f64; state_len]);

        let lr = self.learning_rate.clamp(0.0, 1.0);
        let mut target = vec![0.0; state_len];
        target[outcome.realized_state_index] = 1.0;

        for (p, t) in probs.iter_mut().zip(target.iter()) {
            *p = (1.0 - lr) * *p + lr * *t;
        }

        normalize(probs);
        Ok(())
    }

    pub fn batch_update(
        &self,
        network: &mut BayesianNetwork,
        trades: &[(Evidence, TradeOutcome)],
    ) -> Result<()> {
        for (evidence, outcome) in trades {
            self.update_from_trade(network, evidence, outcome.clone())?;
        }
        Ok(())
    }

    pub fn exponential_decay_update(
        &self,
        network: &mut BayesianNetwork,
        trades: &[(Evidence, TradeOutcome)],
        decay_factor: f64,
    ) -> Result<()> {
        if trades.is_empty() {
            return Ok(());
        }

        let decay = decay_factor.clamp(0.0, 1.0);
        for (idx, (evidence, outcome)) in trades.iter().enumerate() {
            let recency = (trades.len() - idx) as i32;
            let weighted = Self {
                learning_rate: self.learning_rate * decay.powi(recency.saturating_sub(1)),
                prior_strength: self.prior_strength,
            };
            weighted.update_from_trade(network, evidence, outcome.clone())?;
        }
        Ok(())
    }
}

fn normalize(values: &mut [f64]) {
    let sum: f64 = values.iter().sum();
    if sum <= f64::EPSILON {
        let uniform = 1.0 / values.len() as f64;
        values.fill(uniform);
    } else {
        for value in values.iter_mut() {
            *value /= sum;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::bbn::{
        evidence::EvidenceType,
        node::{ConditionalProbabilityTable, Node, NodeType},
    };

    fn outcome_network() -> BayesianNetwork {
        let mut network = BayesianNetwork::new();
        network
            .add_node(Node {
                id: "entry_signal".into(),
                name: "Entry Signal".into(),
                node_type: NodeType::Observed,
                states: vec!["valid".into(), "invalid".into()],
                parents: vec![],
                cpt: ConditionalProbabilityTable {
                    entries: HashMap::from([(Vec::<usize>::new(), vec![0.5, 0.5])]),
                },
            })
            .unwrap();
        network
            .add_node(Node {
                id: "trade_outcome".into(),
                name: "Trade Outcome".into(),
                node_type: NodeType::Observed,
                states: vec!["win".into(), "loss".into()],
                parents: vec!["entry_signal".into()],
                cpt: ConditionalProbabilityTable {
                    entries: HashMap::from([(vec![0usize], vec![0.5, 0.5])]),
                },
            })
            .unwrap();
        network
            .add_edge("entry_signal".into(), "trade_outcome".into())
            .unwrap();
        network
    }

    #[test]
    fn test_cpt_update() {
        let mut network = outcome_network();
        let mut evidence = Evidence::new();
        evidence.insert("entry_signal".into(), EvidenceType::Hard(0));

        let updater = CPTUpdater {
            learning_rate: 0.5,
            prior_strength: 1.0,
        };

        updater
            .update_from_trade(
                &mut network,
                &evidence,
                TradeOutcome {
                    node_id: "trade_outcome".into(),
                    realized_state_index: 0,
                },
            )
            .unwrap();

        let updated = &network.nodes["trade_outcome"].cpt.entries[&vec![0usize]];
        assert!(updated[0] > updated[1]);
        assert!((updated.iter().sum::<f64>() - 1.0).abs() < 1e-9);
    }
}
