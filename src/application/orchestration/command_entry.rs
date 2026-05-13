use anyhow::Result;

use crate::application::multi_timeframe_inputs::{
    detected_multi_timeframe_clean_root, detected_tomac_root, detected_tomac_root_or_placeholder,
};
use crate::application::provider_catalog::{
    load_provider_profile, provider_status_agent_surface, provider_status_surface,
    ProviderCatalogAgentSurface, ProviderProfileReferenceSurface,
};
use crate::state::{
    load_ensemble_executor_scorecards, load_learning_state, load_workflow_snapshot,
    migrate_ensemble_executor_scorecards, recommended_next_command_meta,
    RecommendedNextCommandKind, WorkflowSnapshot,
};

use super::{
    dispatch_workflow_status, emit_pre_bayes_diff_output, emit_pre_bayes_status_output,
    WorkflowStatusBootstrapInput, WorkflowStatusDispatchInput,
};

fn hydrate_recommended_next_command_meta(
    command: &str,
    meta: &mut crate::state::RecommendedNextCommandMeta,
) {
    if meta.kind == RecommendedNextCommandKind::Unknown && !command.is_empty() {
        *meta = recommended_next_command_meta(command);
    }
}

fn hydrate_workflow_snapshot_recommended_next_command_meta(snapshot: &mut WorkflowSnapshot) {
    hydrate_recommended_next_command_meta(
        &snapshot.recommended_next_command,
        &mut snapshot.recommended_next_command_meta,
    );
    for phase in [
        snapshot.latest_train.as_mut(),
        snapshot.latest_analyze.as_mut(),
        snapshot.latest_research.as_mut(),
        snapshot.latest_backtest.as_mut(),
        snapshot.latest_update.as_mut(),
    ]
    .into_iter()
    .flatten()
    {
        hydrate_recommended_next_command_meta(
            &phase.recommended_next_command,
            &mut phase.recommended_next_command_meta,
        );
    }
}

fn profile_reference_matches_symbol(
    profile: &ProviderProfileReferenceSurface,
    symbol: &str,
) -> bool {
    let Ok(document) = load_provider_profile(&profile.selector) else {
        return false;
    };
    document.data_contracts.iter().any(|contract| {
        contract.symbols.is_empty() || contract.symbols.iter().any(|item| item == symbol)
    })
}

fn attach_workflow_opt_in_profile_refs(
    surface: &mut ProviderCatalogAgentSurface,
    symbol: &str,
) -> Result<()> {
    if surface.selected_profile.is_some() || !surface.available_opt_in_profiles.is_empty() {
        return Ok(());
    }
    surface.available_opt_in_profiles = provider_status_surface(None, None, None)?
        .available_opt_in_profiles
        .into_iter()
        .filter(|profile| profile_reference_matches_symbol(profile, symbol))
        .collect();
    Ok(())
}

pub struct WorkflowStatusCommandInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
    pub refresh: bool,
    pub provider_profile: Option<&'a str>,
    pub phase: Option<&'a str>,
    pub actionable_only: bool,
    pub conflicts_only: bool,
    pub latest_promotable: bool,
    pub hard_block_only: bool,
    pub hard_block_reason: Option<&'a str>,
    pub limit: Option<usize>,
    pub output_format: &'a str,
    pub stable: bool,
}

pub fn workflow_status_command<F>(
    input: WorkflowStatusCommandInput<'_>,
    refresh_snapshot: F,
) -> Result<()>
where
    F: Fn(&str, &str) -> Result<WorkflowSnapshot>,
{
    let WorkflowStatusCommandInput {
        symbol,
        state_dir,
        refresh,
        provider_profile,
        phase,
        actionable_only,
        conflicts_only,
        latest_promotable,
        hard_block_only,
        hard_block_reason,
        limit,
        output_format,
        stable,
    } = input;
    let _ = migrate_ensemble_executor_scorecards(state_dir, symbol)?;
    let mut snapshot = if refresh {
        refresh_snapshot(state_dir, symbol)?
    } else {
        load_workflow_snapshot(state_dir, symbol)?
    };
    hydrate_workflow_snapshot_recommended_next_command_meta(&mut snapshot);
    let persisted_scorecards =
        load_ensemble_executor_scorecards(state_dir, symbol).unwrap_or_default();
    let learning_state = load_learning_state(state_dir, symbol).unwrap_or_default();
    let mut provider_status_agent =
        provider_status_agent_surface(None, None, provider_profile).unwrap_or_default();
    if provider_profile.is_none() {
        attach_workflow_opt_in_profile_refs(&mut provider_status_agent, symbol)?;
    }
    let (detected_tomac_root, multi_timeframe_clean_root, tomac_root_placeholder) =
        if provider_status_agent.selected_profile.is_some() {
            let detected_tomac_root = detected_tomac_root();
            let multi_timeframe_clean_root =
                detected_multi_timeframe_clean_root(detected_tomac_root.as_deref());
            let tomac_root_placeholder = detected_tomac_root_or_placeholder();
            (
                detected_tomac_root,
                multi_timeframe_clean_root,
                tomac_root_placeholder,
            )
        } else {
            (None, None, "<tomac-root>".to_string())
        };
    dispatch_workflow_status(
        &snapshot,
        &persisted_scorecards,
        &provider_status_agent,
        learning_state.feedback_history.as_slice(),
        &learning_state.structural_prior_state,
        WorkflowStatusDispatchInput {
            phase,
            actionable_only,
            conflicts_only,
            latest_promotable,
            hard_block_only,
            hard_block_reason,
            limit,
            output_format,
            stable,
        },
        WorkflowStatusBootstrapInput {
            symbol,
            state_dir,
            detected_tomac_root,
            multi_timeframe_clean_root,
            tomac_root_placeholder,
        },
    )
}

pub fn pre_bayes_status_command<F>(
    symbol: &str,
    state_dir: &str,
    refresh: bool,
    section: Option<&str>,
    output_format: &str,
    refresh_snapshot: F,
) -> Result<()>
where
    F: Fn(&str, &str) -> Result<WorkflowSnapshot>,
{
    let snapshot = if refresh {
        refresh_snapshot(state_dir, symbol)?
    } else {
        load_workflow_snapshot(state_dir, symbol)?
    };
    emit_pre_bayes_status_output(&snapshot, section, output_format)
}

pub fn pre_bayes_diff_command<F>(
    symbol: &str,
    state_dir: &str,
    refresh: bool,
    refresh_snapshot: F,
) -> Result<()>
where
    F: Fn(&str, &str) -> Result<WorkflowSnapshot>,
{
    let snapshot = if refresh {
        refresh_snapshot(state_dir, symbol)?
    } else {
        load_workflow_snapshot(state_dir, symbol)?
    };
    emit_pre_bayes_diff_output(&snapshot)
}
