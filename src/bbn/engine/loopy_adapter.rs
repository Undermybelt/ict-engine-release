use anyhow::Result;
use loopybayesnet::BayesNet;
use ndarray::Array1;
use std::collections::BTreeMap;

use crate::domain::belief::BeliefNodePosteriorSnapshot;

pub fn loopy_adapter_ready() -> bool {
    true
}

pub fn infer_with_loopy_adapter(
    assignments: &BTreeMap<String, String>,
    market_behavior_profile: Option<&str>,
) -> Result<Vec<BeliefNodePosteriorSnapshot>> {
    let mut net = BayesNet::new();
    let market_regime = net.add_node_from_probabilities(
        &[],
        Array1::from(vec![0.45_f32, 0.35_f32, 0.10_f32, 0.10_f32]),
    );
    let liquidity_context =
        net.add_node_from_probabilities(&[], Array1::from(vec![0.30_f32, 0.40_f32, 0.30_f32]));
    let entry_quality = net.add_node_from_probabilities(
        &[market_regime],
        ndarray::arr2(&[
            [0.65_f32, 0.30_f32, 0.15_f32, 0.20_f32],
            [0.25_f32, 0.50_f32, 0.45_f32, 0.40_f32],
            [0.10_f32, 0.20_f32, 0.40_f32, 0.40_f32],
        ]),
    );
    let trade_outcome = net.add_node_from_probabilities(
        &[entry_quality, liquidity_context],
        ndarray::Array::from_shape_vec(
            (3, 3, 3),
            vec![
                0.62, 0.48, 0.30, 0.55, 0.45, 0.28, 0.40, 0.34, 0.20, 0.20, 0.27, 0.35, 0.25, 0.30,
                0.40, 0.30, 0.33, 0.45, 0.18, 0.25, 0.35, 0.20, 0.25, 0.35, 0.40, 0.22, 0.20,
            ],
        )?,
    );
    let risk_posture = net.add_node_from_probabilities(
        &[liquidity_context],
        ndarray::arr2(&[
            [0.20_f32, 0.45_f32, 0.70_f32],
            [0.55_f32, 0.40_f32, 0.20_f32],
            [0.25_f32, 0.15_f32, 0.10_f32],
        ]),
    );

    let mut evidence = Vec::new();
    if let Some(value) = assignments.get("market_regime") {
        if let Some(idx) = ["bull", "bear", "range", "transition"]
            .iter()
            .position(|v| v == value)
        {
            evidence.push((market_regime, idx));
        }
    }
    if let Some(value) = assignments.get("liquidity_context") {
        if let Some(idx) = ["favorable", "neutral", "hostile"]
            .iter()
            .position(|v| v == value)
        {
            evidence.push((liquidity_context, idx));
        }
    }
    if let Some(value) = assignments.get("entry_quality") {
        if let Some(idx) = ["high", "medium", "low"].iter().position(|v| v == value) {
            evidence.push((entry_quality, idx));
        }
    }

    net.reset_state();
    net.set_evidence(&evidence);
    for _ in 0..4 {
        net.step();
    }
    let beliefs = net.beliefs();

    let decode = |node_id: &str, states: &[&str], probs: Vec<f64>| -> BeliefNodePosteriorSnapshot {
        let probabilities = states
            .iter()
            .enumerate()
            .map(|(idx, state)| ((*state).to_string(), probs.get(idx).copied().unwrap_or(0.0)))
            .collect::<BTreeMap<_, _>>();
        let (top_state, top_probability) = probabilities
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(state, prob)| (state.clone(), *prob))
            .unwrap_or_else(|| ("state_unavailable".to_string(), 0.0));
        let entropy = probabilities
            .values()
            .filter(|p| **p > 0.0)
            .map(|p| -p * p.ln())
            .sum();
        BeliefNodePosteriorSnapshot {
            node_id: node_id.to_string(),
            top_state,
            top_probability,
            entropy,
            probabilities,
        }
    };

    let mut snapshots = vec![
        decode(
            "market_regime",
            &["bull", "bear", "range", "transition"],
            beliefs[market_regime]
                .as_probabilities()
                .iter()
                .map(|v| *v as f64)
                .collect(),
        ),
        decode(
            "liquidity_context",
            &["favorable", "neutral", "hostile"],
            beliefs[liquidity_context]
                .as_probabilities()
                .iter()
                .map(|v| *v as f64)
                .collect(),
        ),
        decode(
            "entry_quality",
            &["high", "medium", "low"],
            beliefs[entry_quality]
                .as_probabilities()
                .iter()
                .map(|v| *v as f64)
                .collect(),
        ),
        decode(
            "trade_outcome",
            &["win", "scratch", "loss"],
            beliefs[trade_outcome]
                .as_probabilities()
                .iter()
                .map(|v| *v as f64)
                .collect(),
        ),
        decode(
            "risk_posture",
            &["conservative", "balanced", "aggressive"],
            beliefs[risk_posture]
                .as_probabilities()
                .iter()
                .map(|v| *v as f64)
                .collect(),
        ),
    ];

    if let Some(profile) = market_behavior_profile {
        snapshots.push(BeliefNodePosteriorSnapshot {
            node_id: "market_behavior_profile".to_string(),
            top_state: profile.to_string(),
            top_probability: 1.0,
            entropy: 0.0,
            probabilities: BTreeMap::from([(profile.to_string(), 1.0)]),
        });
    }

    Ok(snapshots)
}
