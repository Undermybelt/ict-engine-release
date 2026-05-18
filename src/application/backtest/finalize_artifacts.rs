use anyhow::Result;

use crate::application::artifacts::{
    artifact_action_summary, artifact_decision_section_from_parts,
    artifact_decision_summary_from_trends, build_artifact_consumed_impact_summary,
    build_artifact_factor_trends, build_artifact_family_trends, build_artifact_lineage_summaries,
    build_artifact_rule_break_effects,
};
use crate::state::{
    ArtifactConsumedImpactSummary, ArtifactDecisionSection, ArtifactDecisionSummary,
    ArtifactFactorTrendSummary, ArtifactFamilyTrendSummary,
};

pub struct FinalizeBacktestArtifactSurfaces {
    pub factor_trends: Vec<ArtifactFactorTrendSummary>,
    pub family_trends: Vec<ArtifactFamilyTrendSummary>,
    pub consumed_impact_summary: ArtifactConsumedImpactSummary,
    pub action_summary: Vec<String>,
    pub decision_summary: ArtifactDecisionSummary,
    pub decision_section: ArtifactDecisionSection,
}

pub fn load_finalize_backtest_artifact_surfaces(
    state_dir: &str,
    symbol: &str,
) -> Result<FinalizeBacktestArtifactSurfaces> {
    let ledger = crate::state::load_artifact_ledger(state_dir, symbol)?;
    let factor_trends = build_artifact_factor_trends(&ledger, &None, &None, &None);
    let family_trends = build_artifact_family_trends(&ledger, &None, &None, &None);
    let consumed_impact_summary = build_artifact_consumed_impact_summary(&ledger);
    let action_summary =
        artifact_action_summary(&factor_trends, &family_trends, &consumed_impact_summary);
    let actionable_artifacts = ledger
        .iter()
        .filter(|entry| entry.actionable && entry.consumed_by_update_run_id.is_none())
        .cloned()
        .collect::<Vec<_>>();
    let latest_promotable_artifact = ledger
        .iter()
        .filter(|entry| entry.promote_candidate && entry.consumed_by_update_run_id.is_none())
        .max_by_key(|entry| crate::application::artifacts::artifact_generated_recency_key(entry));
    let lineage = build_artifact_lineage_summaries(&ledger);
    let decision_summary = artifact_decision_summary_from_trends(
        &actionable_artifacts,
        latest_promotable_artifact,
        &lineage,
        &factor_trends,
        &family_trends,
        &consumed_impact_summary,
    );
    let rule_break_effects = build_artifact_rule_break_effects(&ledger);
    let decision_section = artifact_decision_section_from_parts(
        &decision_summary,
        &action_summary,
        &factor_trends,
        &family_trends,
        &rule_break_effects,
        &consumed_impact_summary,
    );

    Ok(FinalizeBacktestArtifactSurfaces {
        factor_trends,
        family_trends,
        consumed_impact_summary,
        action_summary,
        decision_summary,
        decision_section,
    })
}
