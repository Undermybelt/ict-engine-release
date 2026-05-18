use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IctBbnNodeSchema {
    pub schema_version: String,
    pub painless_iteration: bool,
    pub nodes: Vec<IctBbnNodeDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IctBbnNodeDefinition {
    pub name: String,
    pub states: Vec<String>,
    pub evidence_fields: Vec<String>,
    pub parent_candidates: Vec<String>,
    pub notes: String,
}

pub fn default_ict_bbn_node_schema() -> IctBbnNodeSchema {
    IctBbnNodeSchema {
        schema_version: "1.0.0".to_string(),
        painless_iteration: true,
        nodes: vec![
            IctBbnNodeDefinition {
                name: "imbalance_context".to_string(),
                states: vec![
                    "absent".into(),
                    "active".into(),
                    "mitigated".into(),
                    "inversed".into(),
                ],
                evidence_fields: vec!["fvg_state".into(), "mitigation_progress".into()],
                parent_candidates: vec!["regime".into(), "session_context".into()],
                notes: "FVG/iFVG/BPR 共用语义面。".to_string(),
            },
            IctBbnNodeDefinition {
                name: "liquidity_event".to_string(),
                states: vec![
                    "none".into(),
                    "approach".into(),
                    "swept".into(),
                    "reclaimed".into(),
                ],
                evidence_fields: vec!["liquidity_sweep_state".into(), "pool_side".into()],
                parent_candidates: vec!["session_context".into(), "regime".into()],
                notes: "等高低、流动池、SFP 可合流。".to_string(),
            },
            IctBbnNodeDefinition {
                name: "entry_zone_quality".to_string(),
                states: vec!["poor".into(), "ok".into(), "good".into(), "high".into()],
                evidence_fields: vec![
                    "fvg_state".into(),
                    "ote_state".into(),
                    "rebalance_state".into(),
                ],
                parent_candidates: vec!["imbalance_context".into(), "liquidity_event".into()],
                notes: "面向实战决策，不直接暴露底层脚本术语。".to_string(),
            },
            IctBbnNodeDefinition {
                name: "execution_window".to_string(),
                states: vec![
                    "closed".into(),
                    "session_open".into(),
                    "killzone".into(),
                    "macro".into(),
                ],
                evidence_fields: vec!["session_context_state".into(), "macro_window_state".into()],
                parent_candidates: vec![],
                notes: "承接时效性约束。".to_string(),
            },
        ],
    }
}

pub fn write_ict_bbn_node_schema(path: impl AsRef<Path>) -> Result<()> {
    let schema = default_ict_bbn_node_schema();
    fs::write(path, serde_json::to_string_pretty(&schema)?)?;
    Ok(())
}
