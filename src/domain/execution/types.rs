use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::execution::OuExecutionMetrics;
use crate::domain::execution::SpectralExecutionMetrics;
use crate::domain::regime::IsingState;
use crate::ict::PythagoreanExtensionMetrics;
use crate::state::RunProvenance;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionFeatures {
    pub execution_score: f64,
    pub prediction_score: f64,
    pub execution_edge_share: f64,
    pub prediction_edge_share: f64,
    pub execution_readiness: f64,
    pub aggression_bias: f64,
    pub completion_pressure: f64,
    pub liquidity_absorption_bias: f64,
    pub evidence_quality: f64,
    pub overextension_distance: Option<f64>,
    pub reversion_speed: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dominant_cycle_energy: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycle_phase_alignment: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spectral_entropy: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ou_metrics: Option<OuExecutionMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ising_state: Option<IsingState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pythagorean_metrics: Option<PythagoreanExtensionMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spectral_metrics: Option<SpectralExecutionMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionArtifact {
    pub artifact_id: String,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub features: ExecutionFeatures,
    pub hard_gate_status: String,
    pub provenance: RunProvenance,
}
