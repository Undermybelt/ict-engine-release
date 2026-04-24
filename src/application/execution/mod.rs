pub mod artifact;
pub mod inputs;
pub mod persistence;
pub mod physics;
pub mod physics_integration;
pub mod reflection;
pub mod run_fields;
pub mod workflow;

pub use artifact::{
    build_execution_artifact, build_execution_artifact_from_snapshot,
    ExecutionArtifactBuildContext, ExecutionInputSnapshot, ExecutionOuFallback,
};
pub use inputs::{derive_execution_inputs, ExecutionInputSources};
pub use persistence::{persist_execution_artifact, EXECUTION_ARTIFACT_FILE};
pub use physics::{build_execution_physics_overlay, ExecutionPhysicsOverlay};
pub use physics_integration::apply_physics_overlay;
pub use reflection::apply_execution_artifact_to_reflection_bundle;
pub use run_fields::{
    build_execution_phase_fields, derive_backtest_execution_fields,
    derive_research_execution_fields, derive_update_execution_fields, ExecutionPhaseFields,
};
pub use workflow::{
    apply_analyze_run_execution_fields, apply_backtest_run_execution_fields,
    apply_execution_fields_to_workflow_phase, apply_research_run_execution_fields,
    apply_round2_summary_fields_to_workflow_phase, apply_update_run_execution_fields,
    execution_phase_summary_suffix,
};
