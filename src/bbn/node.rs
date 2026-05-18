use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

use super::evidence::{Evidence, EvidenceType};

pub type NodeId = String;
pub type ParentConfig = Vec<usize>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeType {
    Observed,
    Hidden,
    Decision,
    Utility,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConditionalProbabilityTable {
    #[serde(with = "cpt_entries_serde")]
    pub entries: HashMap<ParentConfig, Vec<f64>>,
}

impl ConditionalProbabilityTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, parent_config: ParentConfig, probabilities: Vec<f64>) {
        self.entries.insert(parent_config, probabilities);
    }

    pub fn get(&self, parent_config: &ParentConfig) -> Option<&Vec<f64>> {
        self.entries.get(parent_config)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub name: String,
    pub node_type: NodeType,
    pub states: Vec<String>,
    pub parents: Vec<NodeId>,
    pub cpt: ConditionalProbabilityTable,
}

impl Node {
    pub fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            bail!("node id cannot be empty");
        }
        if self.states.is_empty() {
            bail!("node '{}' must define at least one state", self.id);
        }

        for (config, probs) in &self.cpt.entries {
            if probs.len() != self.states.len() {
                bail!(
                    "node '{}' CPT entry {:?} has {} probs but {} states",
                    self.id,
                    config,
                    probs.len(),
                    self.states.len()
                );
            }
            validate_probabilities(probs)?;
        }

        Ok(())
    }

    pub fn state_index(&self, state: &str) -> Option<usize> {
        self.states.iter().position(|s| s == state)
    }

    pub fn state_name(&self, index: usize) -> Option<&str> {
        self.states.get(index).map(|s| s.as_str())
    }

    pub fn probabilities_for_evidence(&self, evidence: &Evidence) -> Result<Vec<f64>> {
        if self.parents.is_empty() {
            return self
                .cpt
                .get(&Vec::new())
                .cloned()
                .ok_or_else(|| anyhow!("missing prior distribution for root node '{}'", self.id));
        }

        let mut distribution = vec![0.0; self.states.len()];
        let mut total_weight = 0.0;

        for (config, probabilities) in &self.cpt.entries {
            if config.len() != self.parents.len() {
                bail!(
                    "node '{}' CPT config length {} does not match parent count {}",
                    self.id,
                    config.len(),
                    self.parents.len()
                );
            }

            let mut weight = 1.0;
            for (parent_index, parent) in self.parents.iter().enumerate() {
                let state_index = config[parent_index];
                match evidence.get(parent) {
                    Some(EvidenceType::Hard(index)) => {
                        if *index != state_index {
                            weight = 0.0;
                            break;
                        }
                    }
                    Some(EvidenceType::Soft(distribution)) => {
                        let probability =
                            distribution.get(state_index).copied().ok_or_else(|| {
                                anyhow!(
                                    "soft evidence for '{}' missing state index {}",
                                    parent,
                                    state_index
                                )
                            })?;
                        weight *= probability;
                    }
                    None => return Err(anyhow!("missing evidence for parent '{}'", parent)),
                }
            }

            if weight <= f64::EPSILON {
                continue;
            }

            total_weight += weight;
            for (value, conditional_probability) in
                distribution.iter_mut().zip(probabilities.iter())
            {
                *value += weight * conditional_probability;
            }
        }

        if total_weight <= f64::EPSILON {
            return Err(anyhow!(
                "no compatible CPT entries for node '{}' under supplied evidence",
                self.id
            ));
        }

        for value in &mut distribution {
            *value /= total_weight;
        }
        Ok(distribution)
    }

    pub fn parent_config_from_evidence(
        parents: &[NodeId],
        evidence: &Evidence,
    ) -> Result<ParentConfig> {
        parents
            .iter()
            .map(|parent| match evidence.get(parent) {
                Some(EvidenceType::Hard(index)) => Ok(*index),
                Some(EvidenceType::Soft(distribution)) => distribution
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(idx, _)| idx)
                    .ok_or_else(|| anyhow!("soft evidence for '{}' is empty", parent)),
                None => Err(anyhow!("missing evidence for parent '{}'", parent)),
            })
            .collect()
    }
}

fn validate_probabilities(probs: &[f64]) -> Result<()> {
    if probs.iter().any(|p| *p < 0.0 || !p.is_finite()) {
        bail!("probabilities must be finite and non-negative");
    }
    let sum: f64 = probs.iter().sum();
    if (sum - 1.0).abs() > 1e-6 {
        bail!("probabilities must sum to 1.0, got {}", sum);
    }
    Ok(())
}

mod cpt_entries_serde {
    use std::collections::HashMap;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::ParentConfig;

    pub fn serialize<S>(
        entries: &HashMap<ParentConfig, Vec<f64>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let entries: Vec<(&ParentConfig, &Vec<f64>)> = entries.iter().collect();
        entries.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<ParentConfig, Vec<f64>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let entries = Vec::<(ParentConfig, Vec<f64>)>::deserialize(deserializer)?;
        Ok(entries.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probabilities_for_evidence_marginalizes_soft_parent_evidence() {
        let mut cpt = ConditionalProbabilityTable::new();
        cpt.insert(vec![0], vec![0.9, 0.1]);
        cpt.insert(vec![1], vec![0.2, 0.8]);
        let node = Node {
            id: "child".to_string(),
            name: "child".to_string(),
            node_type: NodeType::Hidden,
            states: vec!["a".to_string(), "b".to_string()],
            parents: vec!["parent".to_string()],
            cpt,
        };
        let evidence = std::collections::HashMap::from([(
            "parent".to_string(),
            EvidenceType::Soft(vec![0.25, 0.75]),
        )]);

        let distribution = node.probabilities_for_evidence(&evidence).unwrap();

        assert!((distribution[0] - 0.375).abs() < 1e-9);
        assert!((distribution[1] - 0.625).abs() < 1e-9);
    }
}
