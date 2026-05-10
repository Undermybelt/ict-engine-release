use super::*;

pub(crate) struct FactorResearchShellInput<'a> {
    pub(crate) symbol: &'a str,
    pub(crate) data: &'a str,
    pub(crate) objective: &'a str,
    pub(crate) data_1m: Option<&'a str>,
    pub(crate) data_5m: Option<&'a str>,
    pub(crate) data_15m: Option<&'a str>,
    pub(crate) data_30m: Option<&'a str>,
    pub(crate) data_1h: Option<&'a str>,
    pub(crate) data_4h: Option<&'a str>,
    pub(crate) data_1d: Option<&'a str>,
    pub(crate) paired_data: Option<&'a str>,
    pub(crate) provider_profile: Option<&'a str>,
    pub(crate) auto_quant_profile: Option<&'a str>,
    pub(crate) auxiliary_evidence: Option<&'a str>,
    pub(crate) mutation_spec: Option<&'a str>,
    pub(crate) control_matrix_pb12: bool,
    pub(crate) strategy_material_root: Option<&'a str>,
    pub(crate) emit_mutation_evaluation: bool,
    pub(crate) ensemble: bool,
    pub(crate) state_dir: &'a str,
    pub(crate) output_format: &'a str,
    pub(crate) compact: bool,
    pub(crate) agent: bool,
    pub(crate) human: bool,
    pub(crate) backend: &'a str,
}

pub(crate) fn factor_research_shell(input: FactorResearchShellInput<'_>) -> Result<()> {
    let FactorResearchShellInput {
        symbol,
        data,
        objective,
        data_1m,
        data_5m,
        data_15m,
        data_30m,
        data_1h,
        data_4h,
        data_1d,
        paired_data,
        provider_profile,
        auto_quant_profile,
        auxiliary_evidence,
        mutation_spec,
        control_matrix_pb12,
        strategy_material_root,
        emit_mutation_evaluation,
        ensemble,
        state_dir,
        output_format,
        compact,
        agent,
        human,
        backend,
    } = input;

    ensure_state_dir_ready(state_dir)?;
    let cli_auxiliary_override = load_auxiliary_evidence_override(auxiliary_evidence)?;
    let cli_runtime_notes =
        build_auxiliary_runtime_notes(auxiliary_evidence, cli_auxiliary_override.as_ref());
    let output_format = match resolve_output_format(output_format, compact, agent, human)? {
        OutputFormat::Json => "json",
        OutputFormat::Compact => "compact",
        OutputFormat::Agent => "agent",
        OutputFormat::Human => "human",
    };
    if backend == "auto-quant" {
        auto_quant_factor_research_command(AutoQuantFactorResearchCommandInput {
            symbol,
            data,
            objective,
            provider_profile_selector: provider_profile,
            paired_data,
            auto_quant_profile,
            auxiliary_evidence_path: auxiliary_evidence,
            mutation_spec_path: mutation_spec,
            strategy_material_root,
            state_dir,
            output_format,
        })
    } else {
        ict_engine::application::backtest::factor_research_command(
            ict_engine::application::backtest::FactorResearchCommandInput {
                symbol,
                data,
                objective,
                mutation_spec_path: mutation_spec,
                control_matrix_pb12,
                emit_mutation_evaluation,
                ensemble,
                state_dir,
                output_format,
            },
            load_factor_mutation_spec,
            |objective_mode,
             mutation_spec,
             control_matrix_plan,
             _control_matrix_run,
             runtime_overrides,
             run_state_dir| {
                let mut runtime_notes = runtime_overrides.runtime_notes;
                runtime_notes.extend(cli_runtime_notes.clone());
                run_factor_research(RunFactorResearchInput {
                    symbol,
                    data,
                    objective: objective_mode,
                    data_1m,
                    data_5m,
                    data_15m,
                    data_30m,
                    data_1h,
                    data_4h,
                    data_1d,
                    paired_data,
                    paired_candles_override: runtime_overrides.paired_candles,
                    auxiliary_override: cli_auxiliary_override
                        .clone()
                        .or(runtime_overrides.auxiliary),
                    runtime_notes,
                    mutation_spec: mutation_spec.as_ref(),
                    control_matrix_plan,
                    state_dir: run_state_dir,
                })
            },
        )
    }
}

pub(crate) struct FactorAutoresearchShellInput<'a> {
    pub(crate) symbol: &'a str,
    pub(crate) data: &'a str,
    pub(crate) objective: &'a str,
    pub(crate) mutation_spec: Option<&'a str>,
    pub(crate) iterations: usize,
    pub(crate) data_1m: Option<&'a str>,
    pub(crate) data_5m: Option<&'a str>,
    pub(crate) data_15m: Option<&'a str>,
    pub(crate) data_30m: Option<&'a str>,
    pub(crate) data_1h: Option<&'a str>,
    pub(crate) data_4h: Option<&'a str>,
    pub(crate) data_1d: Option<&'a str>,
    pub(crate) paired_data: Option<&'a str>,
    pub(crate) provider_profile: Option<&'a str>,
    pub(crate) auto_quant_profile: Option<&'a str>,
    pub(crate) auxiliary_evidence: Option<&'a str>,
    pub(crate) strategy_material_root: Option<&'a str>,
    pub(crate) session_id: Option<&'a str>,
    pub(crate) resume_latest: bool,
    pub(crate) max_cluster_fail_streak: usize,
    pub(crate) state_dir: &'a str,
    pub(crate) backend: &'a str,
}

pub(crate) fn factor_autoresearch_shell(input: FactorAutoresearchShellInput<'_>) -> Result<()> {
    let FactorAutoresearchShellInput {
        symbol,
        data,
        objective,
        mutation_spec,
        iterations,
        data_1m,
        data_5m,
        data_15m,
        data_30m,
        data_1h,
        data_4h,
        data_1d,
        paired_data,
        provider_profile,
        auto_quant_profile,
        auxiliary_evidence,
        strategy_material_root,
        session_id,
        resume_latest,
        max_cluster_fail_streak,
        state_dir,
        backend,
    } = input;

    ensure_state_dir_ready(state_dir)?;
    let cli_auxiliary_override = load_auxiliary_evidence_override(auxiliary_evidence)?;
    let cli_runtime_notes =
        build_auxiliary_runtime_notes(auxiliary_evidence, cli_auxiliary_override.as_ref());
    if backend == "auto-quant" {
        auto_quant_factor_autoresearch_command(AutoQuantFactorAutoresearchCommandInput {
            symbol,
            data,
            objective,
            provider_profile_selector: provider_profile,
            paired_data,
            auto_quant_profile,
            auxiliary_evidence_path: auxiliary_evidence,
            mutation_spec_path: mutation_spec,
            strategy_material_root,
            iterations,
            session_id,
            state_dir,
        })
    } else {
        ict_engine::application::factor_lifecycle::factor_autoresearch_command(
            ict_engine::application::factor_lifecycle::FactorAutoresearchCommandInput {
                symbol,
                data,
                objective,
                mutation_spec_path: mutation_spec,
                iterations,
                data_1m,
                data_5m,
                data_15m,
                data_30m,
                data_1h,
                data_4h,
                data_1d,
                paired_data,
                session_id,
                resume_latest,
                max_cluster_fail_streak,
                state_dir,
            },
            load_factor_mutation_spec,
            |objective_mode, mutation_spec| {
                run_factor_research(RunFactorResearchInput {
                    symbol,
                    data,
                    objective: objective_mode,
                    data_1m,
                    data_5m,
                    data_15m,
                    data_30m,
                    data_1h,
                    data_4h,
                    data_1d,
                    paired_data,
                    paired_candles_override: None,
                    auxiliary_override: cli_auxiliary_override.clone(),
                    runtime_notes: cli_runtime_notes.clone(),
                    mutation_spec: Some(mutation_spec),
                    control_matrix_plan: None,
                    state_dir,
                })
            },
        )
    }
}

fn load_auxiliary_evidence_override(path: Option<&str>) -> Result<Option<AuxiliaryMarketEvidence>> {
    path.map(load_auxiliary_evidence_from_path).transpose()
}

fn load_auxiliary_evidence_from_path(path: &str) -> Result<AuxiliaryMarketEvidence> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading auxiliary/options evidence from {}", path))?;
    if let Ok(auxiliary) = serde_json::from_str::<AuxiliaryMarketEvidence>(&raw) {
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
    serde_json::from_value::<AuxiliaryMarketEvidence>(nested)
        .with_context(|| format!("deserializing AuxiliaryMarketEvidence from {}", path))
}

fn build_auxiliary_runtime_notes(
    path: Option<&str>,
    auxiliary: Option<&AuxiliaryMarketEvidence>,
) -> Vec<String> {
    let mut notes = Vec::new();
    if let Some(path) = path {
        notes.push(format!("auxiliary_evidence_path={path}"));
    }
    if let Some(auxiliary) = auxiliary {
        notes.push(format!("auxiliary_spot_symbol={}", auxiliary.spot_symbol));
        notes.push(format!(
            "auxiliary_options_symbol={}",
            auxiliary.options_symbol
        ));
    }
    notes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_auxiliary_evidence_from_direct_json() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let payload = serde_json::json!({
            "spot_symbol": "QQQ",
            "options_symbol": "QQQ",
            "spot_kind": "Equity",
            "spot_last_close": 500.0,
            "futures_last_close": 19999.0,
            "spot_return": 0.01,
            "futures_return": 0.02,
            "raw_basis_bps": 1.5,
            "normalized_basis_bps": 0.5,
            "rolling_price_ratio_mean": 1.02,
            "put_call_oi_ratio": 0.8,
            "put_call_volume_ratio": 0.9,
            "near_atm_implied_volatility": 0.2,
            "near_atm_delta": 0.55,
            "near_atm_gamma": 0.12,
            "near_atm_vega": 0.18,
            "call_gamma_oi": 100.0,
            "put_gamma_oi": 80.0,
            "gamma_skew": 0.25,
            "hedge_pressure_direction": "long",
            "hedge_pressure_score": 0.4,
            "long_bias": 0.15,
            "short_bias": 0.05,
            "uncertainty_penalty": 0.1,
            "notes": ["direct_auxiliary"]
        });
        std::fs::write(temp.path(), serde_json::to_string(&payload).unwrap()).unwrap();

        let auxiliary = load_auxiliary_evidence_from_path(temp.path().to_str().unwrap()).unwrap();
        assert_eq!(auxiliary.spot_symbol, "QQQ");
        assert_eq!(auxiliary.options_symbol, "QQQ");
        assert_eq!(auxiliary.hedge_pressure_direction.as_deref(), Some("long"));
    }

    #[test]
    fn loads_auxiliary_evidence_from_analyze_report_wrapper() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let payload = serde_json::json!({
            "supporting": {
                "auxiliary": {
                    "spot_symbol": "SPY",
                    "options_symbol": "SPY",
                    "spot_kind": "Equity",
                    "spot_last_close": 500.0,
                    "futures_last_close": 5200.0,
                    "spot_return": 0.01,
                    "futures_return": 0.02,
                    "raw_basis_bps": 2.0,
                    "normalized_basis_bps": 0.6,
                    "rolling_price_ratio_mean": 1.01,
                    "put_call_oi_ratio": 1.1,
                    "put_call_volume_ratio": 1.2,
                    "near_atm_implied_volatility": 0.21,
                    "near_atm_delta": 0.45,
                    "near_atm_gamma": 0.10,
                    "near_atm_vega": 0.17,
                    "call_gamma_oi": 120.0,
                    "put_gamma_oi": 130.0,
                    "gamma_skew": -0.15,
                    "hedge_pressure_direction": "short",
                    "hedge_pressure_score": -0.35,
                    "long_bias": 0.02,
                    "short_bias": 0.14,
                    "uncertainty_penalty": 0.12,
                    "notes": ["wrapped_auxiliary"]
                }
            }
        });
        std::fs::write(temp.path(), serde_json::to_string(&payload).unwrap()).unwrap();

        let auxiliary = load_auxiliary_evidence_from_path(temp.path().to_str().unwrap()).unwrap();
        assert_eq!(auxiliary.spot_symbol, "SPY");
        assert_eq!(auxiliary.options_symbol, "SPY");
        assert_eq!(auxiliary.hedge_pressure_direction.as_deref(), Some("short"));
    }
}
