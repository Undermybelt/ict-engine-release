use super::*;

pub(crate) struct WorkflowStatusShellInput<'a> {
    pub(crate) symbol: &'a str,
    pub(crate) state_dir: &'a str,
    pub(crate) refresh: bool,
    pub(crate) provider_profile: Option<&'a str>,
    pub(crate) phase: Option<&'a str>,
    pub(crate) actionable_only: bool,
    pub(crate) conflicts_only: bool,
    pub(crate) latest_promotable: bool,
    pub(crate) hard_block_only: bool,
    pub(crate) hard_block_reason: Option<&'a str>,
    pub(crate) limit: Option<usize>,
    pub(crate) output_format: &'a str,
    pub(crate) stable: bool,
}

pub(crate) fn workflow_status_shell(input: WorkflowStatusShellInput<'_>) -> Result<()> {
    let WorkflowStatusShellInput {
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
    ict_engine::application::orchestration::workflow_status_command(
        ict_engine::application::orchestration::WorkflowStatusCommandInput {
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
        },
        refresh_workflow_snapshot,
    )
}

pub(crate) fn pre_bayes_status_shell(
    symbol: &str,
    state_dir: &str,
    refresh: bool,
    section: Option<&str>,
    output_format: &str,
) -> Result<()> {
    ict_engine::application::orchestration::pre_bayes_status_command(
        symbol,
        state_dir,
        refresh,
        section,
        output_format,
        refresh_workflow_snapshot,
    )
}

pub(crate) fn provider_status_shell(
    domain: Option<&str>,
    provider: Option<&str>,
    compact: bool,
    agent: bool,
    jsonl: bool,
    profile: Option<&str>,
) -> Result<()> {
    provider_status_command(domain, provider, compact, agent, jsonl, profile)
}

pub(crate) struct ArtifactStatusShellInput<'a> {
    pub(crate) symbol: &'a str,
    pub(crate) state_dir: &'a str,
    pub(crate) artifact_id: Option<&'a str>,
    pub(crate) kind: Option<&'a str>,
    pub(crate) latest_only: bool,
    pub(crate) actionable_only: bool,
    pub(crate) rule_break_only: bool,
    pub(crate) sort_by: &'a str,
    pub(crate) descending: bool,
    pub(crate) limit: Option<usize>,
    pub(crate) recent_n: Option<usize>,
    pub(crate) consumed_only: bool,
    pub(crate) bucket_by_kind: bool,
    pub(crate) bucket_order_by: &'a str,
    pub(crate) bucket_limit: Option<usize>,
}

pub(crate) fn artifact_status_shell(input: ArtifactStatusShellInput<'_>) -> Result<()> {
    let ArtifactStatusShellInput {
        symbol,
        state_dir,
        artifact_id,
        kind,
        latest_only,
        actionable_only,
        rule_break_only,
        sort_by,
        descending,
        limit,
        recent_n,
        consumed_only,
        bucket_by_kind,
        bucket_order_by,
        bucket_limit,
    } = input;
    artifact_status_command(ArtifactStatusCommandInput {
        symbol,
        state_dir,
        artifact_id,
        kind,
        latest_only,
        actionable_only,
        rule_break_only,
        sort_by,
        descending,
        limit,
        recent_n,
        consumed_only,
        bucket_by_kind,
        bucket_order_by,
        bucket_limit,
    })
}

pub(crate) fn pre_bayes_diff_shell(symbol: &str, state_dir: &str, refresh: bool) -> Result<()> {
    ict_engine::application::orchestration::pre_bayes_diff_command(
        symbol,
        state_dir,
        refresh,
        refresh_workflow_snapshot,
    )
}

pub(crate) struct ArtifactDiffShellInput<'a> {
    pub(crate) symbol: &'a str,
    pub(crate) state_dir: &'a str,
    pub(crate) left_artifact_id: &'a str,
    pub(crate) right_artifact_id: &'a str,
}

pub(crate) fn artifact_diff_shell(input: ArtifactDiffShellInput<'_>) -> Result<()> {
    let ArtifactDiffShellInput {
        symbol,
        state_dir,
        left_artifact_id,
        right_artifact_id,
    } = input;
    artifact_diff_command(ArtifactDiffCommandInput {
        symbol,
        state_dir,
        left_artifact_id,
        right_artifact_id,
    })
}

pub(crate) struct ArtifactLineageShellInput<'a> {
    pub(crate) symbol: &'a str,
    pub(crate) state_dir: &'a str,
    pub(crate) artifact_id: Option<&'a str>,
    pub(crate) latest_only: bool,
    pub(crate) improving_only: bool,
    pub(crate) regressing_only: bool,
    pub(crate) rule_break_only: bool,
}

pub(crate) fn artifact_lineage_shell(input: ArtifactLineageShellInput<'_>) -> Result<()> {
    let ArtifactLineageShellInput {
        symbol,
        state_dir,
        artifact_id,
        latest_only,
        improving_only,
        regressing_only,
        rule_break_only,
    } = input;
    let ledger = load_artifact_ledger(state_dir, symbol)?;
    let snapshot = refresh_workflow_snapshot(state_dir, symbol)?;
    artifact_lineage_command(ArtifactLineageCommandInput {
        symbol,
        ledger: &ledger,
        summaries: snapshot.artifact_lineage_summaries,
        artifact_id,
        latest_only,
        improving_only,
        regressing_only,
        rule_break_only,
    })
}

pub(crate) fn factor_mutation_status_shell(
    symbol: &str,
    state_dir: &str,
    source_command: Option<&str>,
    latest_only: bool,
    accepted_only: bool,
    bucket_by_source: bool,
    limit: Option<usize>,
) -> Result<()> {
    ict_engine::application::factor_lifecycle::factor_mutation_status_command(
        symbol,
        state_dir,
        source_command,
        latest_only,
        accepted_only,
        bucket_by_source,
        limit,
    )
}

pub(crate) fn factor_autoresearch_status_shell(
    symbol: &str,
    state_dir: &str,
    session_id: Option<&str>,
    latest_only: bool,
    limit: Option<usize>,
) -> Result<()> {
    ict_engine::application::factor_lifecycle::factor_autoresearch_status_command(
        symbol,
        state_dir,
        session_id,
        latest_only,
        limit,
    )
}
