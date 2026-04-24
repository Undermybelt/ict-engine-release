pub mod autoresearch_surface;
pub mod command_entry;
pub mod expansion_evaluation;
pub mod expansion_objective;
pub mod expansion_regressions;
pub mod expansion_scoring;
pub mod lifecycle_surface;
pub mod mutation_preferences;
pub mod mutation_routing;
pub mod mutation_spec;
pub mod mutation_summary;
pub mod mutation_templates;

pub use autoresearch_surface::{
    build_factor_autoresearch_retrospective, build_factor_autoresearch_status_surface,
    build_factor_autoresearch_warning_surface, render_factor_autoresearch_retrospective_markdown,
    sync_factor_autoresearch_experiments_tsv, sync_factor_autoresearch_retrospective,
    FactorAutoresearchClusterScorecardEntry, FactorAutoresearchDerivedWarning,
    FactorAutoresearchExperimentRow, FactorAutoresearchRetrospective,
    FactorAutoresearchStatusSurface,
};
pub use command_entry::{
    factor_autoresearch_branch_summary, factor_autoresearch_command, factor_autoresearch_decision,
    factor_autoresearch_status_command, factor_mutation_status_command,
    FactorAutoresearchCommandInput,
};

pub use expansion_evaluation::{
    build_expansion_sop_metrics_from_market_reports, build_expansion_sop_mutation_metrics,
    evaluate_expansion_sop_mutation, mechanical_mutation_score,
};

pub use expansion_objective::apply_expansion_manipulation_objective;

pub use expansion_regressions::expansion_regression_reasons_by_market;

pub use expansion_scoring::{expansion_factor_scores_for_market, ExpansionFactorScore};

pub use lifecycle_surface::{build_factor_lifecycle_view, FactorLifecycleView};

pub use mutation_preferences::{
    build_hint_effectiveness_summary, compare_hint_effectiveness, factor_specific_hint_preferences,
    FactorMutationHintEffectivenessSummary, FactorMutationPerFactorHintSummary,
};

pub use mutation_routing::{
    no_superior_mutation_found, recommended_mutation_directions_from_failure_tags,
};

pub use mutation_spec::apply_factor_mutation_spec;

pub use mutation_summary::{
    factor_mutation_priority_markets, factor_mutation_priority_reasons,
    factor_mutation_recommended_focus,
};

pub use mutation_templates::{
    factor_mutation_direction_hint_summary, factor_mutation_focus_prompt,
    factor_mutation_step_size_hint_summary, forced_cluster_jump_template,
    next_mutation_spec_template, next_mutation_spec_template_with_preferences,
};
