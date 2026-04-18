use crate::bbn::node::{ConditionalProbabilityTable, Node, NodeType};

pub fn market_regime_node() -> Node {
    let mut cpt = ConditionalProbabilityTable::new();
    cpt.insert(Vec::new(), vec![0.35, 0.30, 0.35]);

    Node {
        id: "market_regime".into(),
        name: "Market Regime".into(),
        node_type: NodeType::Observed,
        states: vec!["bull".into(), "bear".into(), "range".into()],
        parents: Vec::new(),
        cpt,
    }
}

pub fn liquidity_context_node() -> Node {
    let mut cpt = ConditionalProbabilityTable::new();
    cpt.insert(Vec::new(), vec![0.4, 0.35, 0.25]);

    Node {
        id: "liquidity_context".into(),
        name: "Liquidity Context".into(),
        node_type: NodeType::Observed,
        states: vec!["favorable".into(), "neutral".into(), "hostile".into()],
        parents: Vec::new(),
        cpt,
    }
}

pub fn entry_quality_node() -> Node {
    Node {
        id: "entry_quality".into(),
        name: "Entry Quality".into(),
        node_type: NodeType::Hidden,
        states: vec!["high".into(), "medium".into(), "low".into()],
        parents: vec!["market_regime".into(), "liquidity_context".into()],
        cpt: ConditionalProbabilityTable::new(),
    }
}

pub fn trade_outcome_node() -> Node {
    Node {
        id: "trade_outcome".into(),
        name: "Trade Outcome".into(),
        node_type: NodeType::Hidden,
        states: vec!["win".into(), "breakeven".into(), "loss".into()],
        parents: vec!["entry_quality".into()],
        cpt: ConditionalProbabilityTable::new(),
    }
}
