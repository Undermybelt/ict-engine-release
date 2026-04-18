use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegimeSpecificSubgraph {
    pub regime_key: String,
    pub node_ids: Vec<String>,
    pub edge_descriptions: Vec<String>,
    pub cpt_surface_id: String,
}
