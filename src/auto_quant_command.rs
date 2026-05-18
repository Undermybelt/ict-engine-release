use super::*;
use std::path::Path;

/// Resolve the Auto-Quant output directory from the given state_dir.
/// Auto-Quant artifacts are always isolated from the repo root:
/// - If ICT_ENGINE_AUTO_QUANT_OUTPUT_DIR is set, use that path.
/// - Otherwise, use <state_dir>/auto-quant/ subdirectory.
fn aq_state_dir(state_dir: &str) -> String {
    let custom = std::env::var(AUTO_QUANT_OUTPUT_DIR_ENV_VAR).ok();
    aq_state_dir_with_custom(state_dir, custom.as_deref())
}

fn aq_state_dir_with_custom(state_dir: &str, custom_output_dir: Option<&str>) -> String {
    if let Some(custom) = custom_output_dir
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return custom.to_string();
    }
    if state_dir_has_auto_quant_workspace(Path::new(state_dir)) {
        return state_dir.to_string();
    }
    resolve_auto_quant_output_dir(state_dir)
}

fn state_dir_has_auto_quant_workspace(state_dir: &Path) -> bool {
    state_dir.join("auto_quant_config.json").exists()
        || state_dir.join(".deps").join("auto-quant").exists()
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
    ensure_state_dir_ready(input.state_dir)?;
    auto_quant_ingest_real_trades_command(input)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aq_state_dir_uses_existing_handoff_state_without_extra_subdir() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join(".deps/auto-quant")).unwrap();

        assert_eq!(
            aq_state_dir_with_custom(temp.path().to_str().unwrap(), None),
            temp.path().to_string_lossy()
        );
    }

    #[test]
    fn aq_state_dir_keeps_isolated_subdir_for_fresh_state() {
        let temp = tempfile::tempdir().unwrap();

        assert_eq!(
            aq_state_dir_with_custom(temp.path().to_str().unwrap(), None),
            format!(
                "{}/{}",
                temp.path().to_string_lossy(),
                DEFAULT_AUTO_QUANT_SUBDIR
            )
        );
    }

    #[test]
    fn aq_state_dir_respects_explicit_output_dir_override() {
        let temp = tempfile::tempdir().unwrap();

        assert_eq!(
            aq_state_dir_with_custom(temp.path().to_str().unwrap(), Some("/tmp/custom-aq")),
            "/tmp/custom-aq"
        );
    }

    #[test]
    fn auto_quant_ingest_real_trades_shell_feeds_requested_downstream_state_dir() {
        let temp = tempfile::tempdir().unwrap();
        let trades_path = temp.path().join("real_trades.jsonl");
        let branch_path =
            "Range -> ProviderCryptoPullback -> MeanRevertBounce -> ProviderCryptoPullbackRevertV1";
        let record = serde_json::json!({
            "schema_version": "1.0",
            "symbol": "NQ",
            "trade_id": "trade-1",
            "strategy_name": "ProviderCryptoPullbackRevertV1",
            "strategy_mutation_id": "provider-pullback-v1",
            "auto_quant_run_id": "aq-run-1",
            "open_ts_ms": 1717707600000_i64,
            "close_ts_ms": 1717740000000_i64,
            "direction": "Bull",
            "pnl": 21.69,
            "realized_outcome": "win",
            "regime_at_entry": "Range",
            "entry_signal": "provider_crypto_pullback_revert",
            "regime_profit_branch_path": branch_path,
            "main_regime": "Range",
            "sub_regime": "ProviderCryptoPullback",
            "sub_sub_regime_or_profit_factor": "MeanRevertBounce",
            "profit_factor": "ProviderCryptoPullbackRevertV1",
            "factors_used": []
        });
        std::fs::write(&trades_path, format!("{record}\n")).unwrap();

        auto_quant_ingest_real_trades_shell(AutoQuantIngestRealTradesInput {
            symbol: "NQ",
            state_dir: temp.path().to_str().unwrap(),
            trades_path: trades_path.to_str().unwrap(),
            source: "auto_quant_real_trades",
            dry_run: false,
            force: false,
        })
        .unwrap();

        ict_engine::application::entry_models::export_structural_path_ranking_target_command(
            temp.path().to_str().unwrap(),
            "NQ",
        )
        .unwrap();

        let target_jsonl = temp
            .path()
            .join("NQ")
            .join("policy_training")
            .join("structural_path_ranking_target.jsonl");
        let target_rows = std::fs::read_to_string(target_jsonl).unwrap();
        assert!(
            target_rows.contains(branch_path),
            "real-trade branch feedback must be visible to downstream structural target export"
        );
    }
}
