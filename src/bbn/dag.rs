use std::collections::{HashMap, VecDeque};

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

use super::node::{Node, NodeId};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BayesianNetwork {
    pub nodes: HashMap<NodeId, Node>,
    pub edges: Vec<(NodeId, NodeId)>,
    pub topological_order: Vec<NodeId>,
}

impl BayesianNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: Node) -> Result<()> {
        if self.nodes.contains_key(&node.id) {
            bail!("node '{}' already exists", node.id);
        }
        self.nodes.insert(node.id.clone(), node);
        self.topological_sort()?;
        Ok(())
    }

    pub fn add_edge(&mut self, parent: NodeId, child: NodeId) -> Result<()> {
        if !self.nodes.contains_key(&parent) {
            bail!("parent node '{}' not found", parent);
        }
        if !self.nodes.contains_key(&child) {
            bail!("child node '{}' not found", child);
        }
        if self.edges.iter().any(|(p, c)| p == &parent && c == &child) {
            return Ok(());
        }
        self.edges.push((parent.clone(), child.clone()));

        if let Some(node) = self.nodes.get_mut(&child) {
            if !node.parents.contains(&parent) {
                node.parents.push(parent);
            }
        }

        self.validate()?;
        self.topological_sort()?;
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        for (parent, child) in &self.edges {
            if !self.nodes.contains_key(parent) {
                bail!("edge parent '{}' missing from node set", parent);
            }
            if !self.nodes.contains_key(child) {
                bail!("edge child '{}' missing from node set", child);
            }
        }

        let mut indegree: HashMap<NodeId, usize> =
            self.nodes.keys().map(|id| (id.clone(), 0usize)).collect();

        for (_, child) in &self.edges {
            *indegree.entry(child.clone()).or_default() += 1;
        }

        let mut queue: VecDeque<NodeId> = indegree
            .iter()
            .filter_map(|(id, degree)| (*degree == 0).then_some(id.clone()))
            .collect();

        let mut visited = 0usize;
        let mut outgoing: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for (parent, child) in &self.edges {
            outgoing
                .entry(parent.clone())
                .or_default()
                .push(child.clone());
        }

        while let Some(node) = queue.pop_front() {
            visited += 1;
            if let Some(children) = outgoing.get(&node) {
                for child in children {
                    if let Some(degree) = indegree.get_mut(child) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(child.clone());
                        }
                    }
                }
            }
        }

        if visited != self.nodes.len() {
            bail!("network contains a cycle; graph is not a DAG");
        }

        Ok(())
    }

    pub fn topological_sort(&mut self) -> Result<()> {
        self.validate()?;

        let mut indegree: HashMap<NodeId, usize> =
            self.nodes.keys().map(|id| (id.clone(), 0usize)).collect();
        let mut outgoing: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

        for (parent, child) in &self.edges {
            *indegree.entry(child.clone()).or_default() += 1;
            outgoing
                .entry(parent.clone())
                .or_default()
                .push(child.clone());
        }

        let mut queue: VecDeque<NodeId> = indegree
            .iter()
            .filter_map(|(id, degree)| (*degree == 0).then_some(id.clone()))
            .collect();
        let mut order = Vec::with_capacity(self.nodes.len());

        while let Some(node) = queue.pop_front() {
            order.push(node.clone());
            if let Some(children) = outgoing.get(&node) {
                for child in children {
                    let degree = indegree
                        .get_mut(child)
                        .ok_or_else(|| anyhow!("missing indegree for child '{}'", child))?;
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(child.clone());
                    }
                }
            }
        }

        if order.len() != self.nodes.len() {
            bail!("failed to produce complete topological ordering");
        }

        self.topological_order = order;
        Ok(())
    }

    pub fn parents_of(&self, node_id: &NodeId) -> Vec<NodeId> {
        self.edges
            .iter()
            .filter_map(|(parent, child)| (child == node_id).then_some(parent.clone()))
            .collect()
    }

    pub fn children_of(&self, node_id: &NodeId) -> Vec<NodeId> {
        self.edges
            .iter()
            .filter_map(|(parent, child)| (parent == node_id).then_some(child.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bbn::node::{ConditionalProbabilityTable, Node, NodeType};

    fn binary_node(id: &str) -> Node {
        Node {
            id: id.to_string(),
            name: id.to_string(),
            node_type: NodeType::Observed,
            states: vec!["true".into(), "false".into()],
            parents: Vec::new(),
            cpt: ConditionalProbabilityTable::new(),
        }
    }

    #[test]
    fn test_dag_validation() {
        let mut network = BayesianNetwork::new();
        network.add_node(binary_node("a")).unwrap();
        network.add_node(binary_node("b")).unwrap();
        network.add_edge("a".into(), "b".into()).unwrap();
        assert!(network.validate().is_ok());
        assert_eq!(
            network.topological_order,
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn test_cycle_detection() {
        let mut network = BayesianNetwork::new();
        network.add_node(binary_node("a")).unwrap();
        network.add_node(binary_node("b")).unwrap();
        network.add_edge("a".into(), "b".into()).unwrap();
        assert!(network.add_edge("b".into(), "a".into()).is_err());
    }
}
