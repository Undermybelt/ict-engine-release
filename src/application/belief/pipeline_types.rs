use serde::Serialize;
use std::collections::BTreeMap;

use crate::state::{FactorPipelineLabelSource, PreBayesEntryQualityBridge};

pub use super::pipeline_shared::{
    ExpansionBbnSupport, ExpansionLatestSignal, ExpansionProbabilitySupport,
};

#[derive(Debug, Serialize, Clone)]
pub struct ExpansionFactorPipelineReport {
    pub factor_name: String,
    pub parameters: BTreeMap<String, f64>,
    pub latest_signal: ExpansionLatestSignal,
    pub probability_support: ExpansionProbabilitySupport,
    pub paired_market_quality_report: Option<crate::factor_lab::PairedMarketQualityReport>,
    pub entry_quality_bridge: PreBayesEntryQualityBridge,
    pub bbn_support: ExpansionBbnSupport,
    pub pipeline_summary: String,
    pub recommended_actions: Vec<String>,
    pub frame_physics_trace: Vec<FactorPipelineLabelSource>,
}
