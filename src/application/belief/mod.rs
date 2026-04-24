pub mod debug_report;
pub mod ising_overlay;
pub mod jump_model_sidecar;
pub mod logic_family;
pub mod market_profiles;
pub mod ou_overlay;
pub mod pipeline_builder;
pub mod pipeline_shared;
pub mod pipeline_types;
pub mod policy_lineage_surface;
pub mod pre_bayes_summary;
pub mod shadow_policy_surface;
pub mod shared;
pub mod spectral_overlay;

pub use ising_overlay::{apply_ising_overlay, IsingOverlayState};
pub use jump_model_sidecar::{
    backtest_calibrated_market_jump_weight, build_jump_model_regime_sidecar,
    build_jump_model_regime_sidecar_with_history, build_regime_disagreement_summary,
    historical_market_jump_objective_weight, historical_market_jump_weight,
    jump_calibration_gate_workflow_summary, jump_model_workflow_summary,
    objective_market_credibility_shrink, persist_market_jump_calibration_from_backtest_runs,
    persist_market_jump_calibration_from_research_runs,
    persist_market_jump_objective_calibration_from_research_runs,
};
pub use market_profiles::{market_behavior_profile_for_family, market_category_for_symbol};
pub use ou_overlay::{apply_ou_overlay, OuOverlayState};
pub use pipeline_builder::{
    adapt_factor_pipeline_debug_report, build_canonical_belief_report,
    build_canonical_belief_report_with_pda, build_canonical_belief_snapshot,
    build_canonical_belief_snapshot_with_pda, build_expansion_factor_pipeline_report,
    build_expansion_factor_pipeline_report_with_registry, build_factor_pipeline_debug_report,
    infer_market_from_symbol, pre_bayes_evidence_policy, FactorPipelineDebugReport,
};
pub use pipeline_shared::{
    apply_factor_outcome_overlay, build_pre_bayes_entry_quality_bridge, combine_bias_vectors,
    effective_trade_outcome_win_probability, multi_timeframe_entry_quality_bias, probability_map,
    raw_liquidity_context_trace, raw_market_regime_trace, raw_multi_timeframe_resonance_trace,
    AdaptFactorPipelineDebugReportInput, FactorPipelineDebugReportInput,
    PreBayesEntryQualityBridgeInput,
};
pub use pipeline_types::ExpansionFactorPipelineReport;
pub use policy_lineage_surface::{build_belief_policy_lineage_surface, BeliefPolicyLineageSurface};
pub use pre_bayes_summary::{
    combine_liquidity_labels, combine_regime_labels, pre_bayes_policy_lineage_summary,
    pre_bayes_report_summary,
};
pub use shadow_policy_surface::{build_belief_shadow_policy_surface, BeliefShadowPolicySurface};
pub use spectral_overlay::{apply_spectral_overlay, SpectralOverlayState};
