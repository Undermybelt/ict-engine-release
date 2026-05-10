use anyhow::Result;

use crate::application::multi_timeframe_inputs::{
    detected_multi_timeframe_clean_root, detected_tomac_root, detected_tomac_root_or_placeholder,
};
use crate::application::provider_catalog::provider_status_agent_surface;
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
    let provider_status_agent =
        provider_status_agent_surface(None, None, provider_profile).unwrap_or_default();
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
