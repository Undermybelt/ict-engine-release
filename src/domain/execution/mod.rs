pub mod gates;
pub mod ou;
pub mod score;
pub mod spectral;
pub mod types;

pub use gates::{
    apply_spectral_execution_penalty, classify_execution_gate, DOMINANT_ENERGY_FLOOR,
    EXECUTION_GATE_OBSERVE, EXECUTION_GATE_READY, SPECTRAL_ENTROPY_CHAOS_CAP,
    SPECTRAL_READINESS_PENALTY,
};
pub use ou::{build_ou_execution_metrics, estimate_ou_execution_metrics, OuExecutionMetrics};
pub use score::{execution_edge_split, execution_readiness, ExecutionEdgeSplit};
pub use spectral::{
    estimate_spectral_execution_metrics, SpectralExecutionMetrics, SPECTRAL_DEFAULT_LAMBDA_RATIO,
    SPECTRAL_MIN_SAMPLES,
};
pub use types::{ExecutionArtifact, ExecutionFeatures};
