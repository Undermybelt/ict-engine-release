use super::*;

/// Resolve the Auto-Quant output directory from the given state_dir.
/// Auto-Quant artifacts are always isolated from the repo root:
/// - If ICT_ENGINE_AUTO_QUANT_OUTPUT_DIR is set, use that path.
/// - Otherwise, use <state_dir>/auto-quant/ subdirectory.
fn aq_state_dir(state_dir: &str) -> String {
    resolve_auto_quant_output_dir(state_dir)
}

pub(crate) fn auto_quant_status_shell(state_dir: &str, output_format: &str) -> Result<()> {
    let aq_dir = aq_state_dir(state_dir);
    auto_quant_status_command(&aq_dir, output_format)
}

pub(crate) fn auto_quant_bootstrap_shell(
    state_dir: &str,
    repo_url: Option<&str>,
    tracked_branch: Option<&str>,
) -> Result<()> {
    let aq_dir = aq_state_dir(state_dir);
    ensure_state_dir_ready(&aq_dir)?;
    auto_quant_bootstrap_command(&aq_dir, repo_url, tracked_branch)
}

pub(crate) fn auto_quant_update_shell(
    state_dir: &str,
    repo_url: Option<&str>,
    tracked_branch: Option<&str>,
    target_ref: Option<&str>,
) -> Result<()> {
    let aq_dir = aq_state_dir(state_dir);
    ensure_state_dir_ready(&aq_dir)?;
    auto_quant_update_command(&aq_dir, repo_url, tracked_branch, target_ref)
}

pub(crate) fn auto_quant_prepare_shell(state_dir: &str) -> Result<()> {
    let aq_dir = aq_state_dir(state_dir);
    ensure_state_dir_ready(&aq_dir)?;
    auto_quant_prepare_workspace_command(&aq_dir)
}

pub(crate) fn auto_quant_adoption_review_shell(
    symbol: &str,
    state_dir: &str,
    artifact_id: Option<&str>,
) -> Result<()> {
    let aq_dir = aq_state_dir(state_dir);
    auto_quant_adoption_review_command(symbol, &aq_dir, artifact_id)
}

pub(crate) fn auto_quant_adoption_decision_shell(
    symbol: &str,
    state_dir: &str,
    artifact_id: Option<&str>,
    decision: &str,
    rationale: &str,
    requested_by: &str,
) -> Result<()> {
    let aq_dir = aq_state_dir(state_dir);
    auto_quant_adoption_decision_command(
        symbol,
        &aq_dir,
        artifact_id,
        decision,
        rationale,
        requested_by,
    )
}

pub(crate) fn auto_quant_seed_evidence_shell(
    symbol: &str,
    state_dir: &str,
    strategy_material_root: &str,
    limit: usize,
) -> Result<()> {
    let aq_dir = aq_state_dir(state_dir);
    ensure_state_dir_ready(&aq_dir)?;
    auto_quant_seed_evidence_command(symbol, &aq_dir, strategy_material_root, limit)
}

pub(crate) fn auto_quant_pda_unit_batch_shell(
    input: AutoQuantPdaUnitBatchCommandInput<'_>,
) -> Result<()> {
    let aq_dir = aq_state_dir(input.state_dir);
    ensure_state_dir_ready(&aq_dir)?;
    let resolved = AutoQuantPdaUnitBatchCommandInput {
        state_dir: &aq_dir,
        ..input
    };
    auto_quant_pda_unit_batch_command(resolved)
}

pub(crate) fn auto_quant_pda_unit_dispatch_shell(
    input: AutoQuantPdaUnitDispatchCommandInput<'_>,
) -> Result<()> {
    let aq_dir = aq_state_dir(input.state_dir);
    ensure_state_dir_ready(&aq_dir)?;
    let resolved = AutoQuantPdaUnitDispatchCommandInput {
        state_dir: &aq_dir,
        ..input
    };
    auto_quant_pda_unit_dispatch_command(resolved)
}

pub(crate) fn auto_quant_agent_material_batch_shell(
    input: AutoQuantAgentMaterialBatchCommandInput<'_>,
) -> Result<()> {
    let aq_dir = aq_state_dir(input.state_dir);
    ensure_state_dir_ready(&aq_dir)?;
    let resolved = AutoQuantAgentMaterialBatchCommandInput {
        state_dir: &aq_dir,
        ..input
    };
    auto_quant_agent_material_batch_command(resolved)
}

pub(crate) fn auto_quant_agent_material_dispatch_shell(
    input: AutoQuantAgentMaterialDispatchCommandInput<'_>,
) -> Result<()> {
    let aq_dir = aq_state_dir(input.state_dir);
    ensure_state_dir_ready(&aq_dir)?;
    let resolved = AutoQuantAgentMaterialDispatchCommandInput {
        state_dir: &aq_dir,
        ..input
    };
    auto_quant_agent_material_dispatch_command(resolved)
}

pub(crate) fn auto_quant_agent_material_rank_shell(
    input: AutoQuantAgentMaterialRankCommandInput<'_>,
) -> Result<()> {
    let aq_dir = aq_state_dir(input.state_dir);
    ensure_state_dir_ready(&aq_dir)?;
    let resolved = AutoQuantAgentMaterialRankCommandInput {
        state_dir: &aq_dir,
        ..input
    };
    auto_quant_agent_material_rank_command(resolved)
}

pub(crate) fn auto_quant_results_import_shell(
    symbol: &str,
    state_dir: &str,
    library: &str,
    log: Option<&str>,
) -> Result<()> {
    let aq_dir = aq_state_dir(state_dir);
    ensure_state_dir_ready(&aq_dir)?;
    auto_quant_results_import_command(symbol, &aq_dir, library, log)
}

pub(crate) fn auto_quant_prior_init_shell(input: AutoQuantPriorInitCommandInput<'_>) -> Result<()> {
    let aq_dir = aq_state_dir(input.state_dir);
    ensure_state_dir_ready(&aq_dir)?;
    let resolved = AutoQuantPriorInitCommandInput {
        state_dir: &aq_dir,
        ..input
    };
    auto_quant_prior_init_command(resolved)
}

pub(crate) fn auto_quant_consume_live_signals_shell(
    input: AutoQuantConsumeLiveSignalsInput<'_>,
) -> Result<()> {
    let aq_dir = aq_state_dir(input.state_dir);
    ensure_state_dir_ready(&aq_dir)?;
    let resolved = AutoQuantConsumeLiveSignalsInput {
        state_dir: &aq_dir,
        ..input
    };
    auto_quant_consume_live_signals_command(resolved)
}

pub(crate) fn auto_quant_ingest_real_trades_shell(
    input: AutoQuantIngestRealTradesInput<'_>,
) -> Result<()> {
    let aq_dir = aq_state_dir(input.state_dir);
    ensure_state_dir_ready(&aq_dir)?;
    let resolved = AutoQuantIngestRealTradesInput {
        state_dir: &aq_dir,
        ..input
    };
    auto_quant_ingest_real_trades_command(resolved)
}

pub(crate) fn auto_quant_promote_canonical_setup_shell(
    input: ict_engine::application::backtest::PromoteCanonicalSetupCommandInput<'_>,
) -> Result<()> {
    let aq_dir = aq_state_dir(input.state_dir);
    ensure_state_dir_ready(&aq_dir)?;
    let resolved = ict_engine::application::backtest::PromoteCanonicalSetupCommandInput {
        state_dir: &aq_dir,
        ..input
    };
    let report =
        ict_engine::application::backtest::auto_quant_promote_canonical_setup_command(resolved)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
