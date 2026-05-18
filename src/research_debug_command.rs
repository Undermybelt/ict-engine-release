use super::*;

pub(crate) struct FactorBacktestShellInput<'a> {
    pub symbol: &'a str,
    pub data: &'a str,
    pub multi_timeframe_inputs: MultiTimeframeInputPaths<'a>,
    pub paired_data: Option<&'a str>,
    pub auxiliary_evidence: Option<&'a str>,
    pub ensemble: bool,
    pub state_dir: &'a str,
    pub output_format: &'a str,
}

pub(crate) fn factor_backtest_shell(input: FactorBacktestShellInput<'_>) -> Result<()> {
    let FactorBacktestShellInput {
        symbol,
        data,
        multi_timeframe_inputs,
        paired_data,
        auxiliary_evidence,
        ensemble,
        state_dir,
        output_format,
    } = input;
    ensure_state_dir_ready(state_dir)?;
    let auxiliary_override = auxiliary_evidence
        .map(load_auxiliary_evidence_from_path)
        .transpose()?;
    ict_engine::application::backtest::factor_backtest_command(
        symbol,
        data,
        paired_data,
        ensemble,
        state_dir,
        output_format,
        |symbol, data, paired_data, state_dir| {
            run_factor_backtest(RunFactorBacktestInput {
                symbol,
                data,
                multi_timeframe_inputs,
                paired_data,
                auxiliary_override: auxiliary_override.as_ref(),
                state_dir,
            })
        },
    )
}

fn load_auxiliary_evidence_from_path(
    path: &str,
) -> Result<ict_engine::data::realtime::market_support::AuxiliaryMarketEvidence> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading auxiliary/options evidence from {}", path))?;
    if let Ok(auxiliary) = serde_json::from_str(&raw) {
        return Ok(auxiliary);
    }
    let value: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("parsing auxiliary/options evidence JSON from {}", path))?;
    let nested = value
        .get("supporting")
        .and_then(|supporting| supporting.get("auxiliary"))
        .cloned()
        .or_else(|| value.get("auxiliary").cloned())
        .context(
            "expected AuxiliaryMarketEvidence JSON or an object at supporting.auxiliary / auxiliary",
        )?;
    serde_json::from_value(nested)
        .with_context(|| format!("deserializing AuxiliaryMarketEvidence from {}", path))
}

pub(crate) fn factor_pipeline_debug_shell(
    input: ict_engine::application::factor_pipeline_debug::FactorPipelineDebugCommandInput<'_>,
) -> Result<()> {
    ict_engine::application::factor_pipeline_debug::factor_pipeline_debug_command(input)
}
