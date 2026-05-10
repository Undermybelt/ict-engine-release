use super::*;
use ict_engine::application::regime::consumer_bundle_adapter::RegimeConsumerBundleAdapter;
use std::path::Path;

#[derive(Debug, Serialize)]
struct PersistedCandlesFile {
    candles: Vec<Candle>,
}

fn persist_candle_snapshot(
    state_dir: &str,
    symbol: &str,
    filename: &str,
    candles: &[Candle],
) -> Result<String> {
    let path = std::path::Path::new(state_dir)
        .join(symbol)
        .join(filename)
        .to_string_lossy()
        .to_string();
    save_state(
        state_dir,
        symbol,
        filename,
        &PersistedCandlesFile {
            candles: candles.to_vec(),
        },
    )?;
    Ok(path)
}

struct PersistLiveDataSourceInput<'a> {
    state_dir: &'a str,
    symbol: &'a str,
    timestamp: chrono::DateTime<Utc>,
    futures_backend: &'a str,
    aux_backend: &'a str,
    futures_base_url: &'a str,
    aux_base_url: &'a str,
    futures_symbol: &'a str,
    spot_symbol: &'a str,
    options_symbol: &'a str,
    spot_kind: &'a str,
    htf: &'a [Candle],
    h4: &'a [Candle],
    mtf: &'a [Candle],
    m5: &'a [Candle],
    ltf: &'a [Candle],
    m1: &'a [Candle],
    spot_candles: &'a [Candle],
}

fn persist_live_data_source(
    input: PersistLiveDataSourceInput<'_>,
) -> Result<LiveDataSourceProvenance> {
    let PersistLiveDataSourceInput {
        state_dir,
        symbol,
        timestamp,
        futures_backend,
        aux_backend,
        futures_base_url,
        aux_base_url,
        futures_symbol,
        spot_symbol,
        options_symbol,
        spot_kind,
        htf,
        h4,
        mtf,
        m5,
        ltf,
        m1,
        spot_candles,
    } = input;
    let stamp = timestamp.format("%Y%m%dT%H%M%S").to_string();
    Ok(LiveDataSourceProvenance {
        futures_backend: futures_backend.to_string(),
        aux_backend: aux_backend.to_string(),
        futures_base_url: futures_base_url.to_string(),
        aux_base_url: aux_base_url.to_string(),
        futures_symbol: futures_symbol.to_string(),
        spot_symbol: spot_symbol.to_string(),
        options_symbol: options_symbol.to_string(),
        spot_kind: spot_kind.to_string(),
        fetched_at: timestamp,
        persisted_htf_path: Some(persist_candle_snapshot(
            state_dir,
            symbol,
            &format!("analyze_live_{}_htf.json", stamp),
            htf,
        )?),
        persisted_h4_path: Some(persist_candle_snapshot(
            state_dir,
            symbol,
            &format!("analyze_live_{}_h4.json", stamp),
            h4,
        )?),
        persisted_mtf_path: Some(persist_candle_snapshot(
            state_dir,
            symbol,
            &format!("analyze_live_{}_mtf.json", stamp),
            mtf,
        )?),
        persisted_m5_path: Some(persist_candle_snapshot(
            state_dir,
            symbol,
            &format!("analyze_live_{}_m5.json", stamp),
            m5,
        )?),
        persisted_ltf_path: Some(persist_candle_snapshot(
            state_dir,
            symbol,
            &format!("analyze_live_{}_ltf.json", stamp),
            ltf,
        )?),
        persisted_m1_path: Some(persist_candle_snapshot(
            state_dir,
            symbol,
            &format!("analyze_live_{}_m1.json", stamp),
            m1,
        )?),
        persisted_spot_path: Some(persist_candle_snapshot(
            state_dir,
            symbol,
            &format!("analyze_live_{}_spot.json", stamp),
            spot_candles,
        )?),
    })
}

fn build_live_multi_timeframe_summary(source: &str, frames: &[(&str, &[Candle])]) -> Vec<String> {
    let covered = frames
        .iter()
        .map(|(interval, _)| *interval)
        .collect::<Vec<_>>();
    let mut summary = vec![format!(
        "multi_timeframe_source={} covered_intervals={}",
        source,
        covered.join(",")
    )];
    for (interval, candles) in frames {
        summary.push(format!("{}:{} bars source=live", interval, candles.len()));
    }
    summary
}

pub(crate) struct AnalyzeLiveCommandInput<'a> {
    pub symbol: &'a str,
    pub futures_symbol: Option<&'a str>,
    pub spot_symbol: Option<&'a str>,
    pub options_symbol: Option<&'a str>,
    pub options_volatility_proxy_symbol: Option<&'a str>,
    pub spot_kind: Option<&'a str>,
    pub futures_backend: &'a str,
    pub aux_backend: &'a str,
    pub futures_base_url: &'a str,
    pub aux_base_url: &'a str,
    pub state_dir: &'a str,
    pub output_format: &'a str,
    pub regime_consumer_bundle: Option<&'a str>,
    pub regime_consumer_bundle_strict: bool,
    pub apply_regime_bundle_bbn_soft_evidence: bool,
}

pub(crate) struct AnalyzeLiveShellInput<'a> {
    pub symbol: &'a str,
    pub futures_symbol: Option<&'a str>,
    pub spot_symbol: Option<&'a str>,
    pub options_symbol: Option<&'a str>,
    pub options_volatility_proxy_symbol: Option<&'a str>,
    pub spot_kind: Option<&'a str>,
    pub futures_backend: &'a str,
    pub aux_backend: &'a str,
    pub external_http_base_url: &'a str,
    pub crypto_public_base_url: &'a str,
    pub state_dir: &'a str,
    pub output_format: &'a str,
    pub regime_consumer_bundle: Option<&'a str>,
    pub regime_consumer_bundle_strict: bool,
    pub apply_regime_bundle_bbn_soft_evidence: bool,
}

pub(crate) fn analyze_live_shell(input: AnalyzeLiveShellInput<'_>) -> Result<()> {
    let AnalyzeLiveShellInput {
        symbol,
        futures_symbol,
        spot_symbol,
        options_symbol,
        options_volatility_proxy_symbol,
        spot_kind,
        futures_backend,
        aux_backend,
        external_http_base_url,
        crypto_public_base_url,
        state_dir,
        output_format,
        regime_consumer_bundle,
        regime_consumer_bundle_strict,
        apply_regime_bundle_bbn_soft_evidence,
    } = input;
    ensure_state_dir_ready(state_dir)?;
    let futures_base_url = ict_engine::application::data_sources::resolve_live_backend_base_url(
        futures_backend,
        external_http_base_url,
        crypto_public_base_url,
    );
    let aux_base_url = ict_engine::application::data_sources::resolve_live_backend_base_url(
        aux_backend,
        external_http_base_url,
        crypto_public_base_url,
    );
    analyze_live_command(AnalyzeLiveCommandInput {
        symbol,
        futures_symbol,
        spot_symbol,
        options_symbol,
        options_volatility_proxy_symbol,
        spot_kind,
        futures_backend,
        aux_backend,
        futures_base_url: &futures_base_url,
        aux_base_url: &aux_base_url,
        state_dir,
        output_format,
        regime_consumer_bundle,
        regime_consumer_bundle_strict,
        apply_regime_bundle_bbn_soft_evidence,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedLiveSymbolInputs {
    futures_symbol: String,
    spot_symbol: String,
    options_symbol: String,
    spot_kind: String,
}

fn resolve_live_symbol_inputs(
    symbol: &str,
    futures_symbol: Option<&str>,
    spot_symbol: Option<&str>,
    options_symbol: Option<&str>,
    spot_kind: Option<&str>,
) -> Result<ResolvedLiveSymbolInputs> {
    let inferred = ict_engine::market_catalog::load_market_catalog(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
    .ok()
    .and_then(|catalog| {
        ict_engine::application::data_sources::analyze_live_inferred_symbols(&catalog, symbol)
    });
    let futures_symbol = futures_symbol
        .map(str::to_string)
        .or_else(|| inferred.as_ref().map(|defaults| defaults.futures_symbol.clone()))
        .ok_or_else(|| {
            let inferred_examples = inferred
                .as_ref()
                .map(|defaults| {
                    format!(
                        "Catalog defaults exist: futures={} spot={} options={}.",
                        defaults.futures_symbol, defaults.spot_symbol, defaults.options_symbol
                    )
                })
                .unwrap_or_else(|| {
                    "Try a market key with built-in defaults such as NQ, ES, GC, or CL.".to_string()
                });
            anyhow!(
                "analyze-live '{}' needs live symbol defaults. {} Otherwise pass --futures-symbol <symbol> --spot-symbol <symbol> --options-symbol <symbol>. Provider-first path: ict-engine provider-status --domain live_runtime --agent",
                symbol,
                inferred_examples
            )
        })?;
    let spot_symbol = spot_symbol
        .map(str::to_string)
        .or_else(|| inferred.as_ref().map(|defaults| defaults.spot_symbol.clone()))
        .ok_or_else(|| {
            anyhow!(
                "analyze-live '{}' still needs a spot symbol. Pass --spot-symbol <symbol> or choose a market key with built-in defaults such as NQ, ES, GC, or CL. Provider-first path: ict-engine provider-status --domain live_runtime --agent",
                symbol
            )
        })?;
    let options_symbol = options_symbol
        .map(str::to_string)
        .or_else(|| inferred.as_ref().map(|defaults| defaults.options_symbol.clone()))
        .ok_or_else(|| {
            anyhow!(
                "analyze-live '{}' still needs an options symbol. Pass --options-symbol <symbol> or choose a market key with built-in defaults such as NQ, ES, GC, or CL. Provider-first path: ict-engine provider-status --domain live_runtime --agent",
                symbol
            )
        })?;
    Ok(ResolvedLiveSymbolInputs {
        futures_symbol,
        spot_symbol,
        options_symbol,
        spot_kind: spot_kind
            .map(str::to_string)
            .or_else(|| inferred.as_ref().map(|defaults| defaults.spot_kind.clone()))
            .unwrap_or_else(|| "equity".to_string()),
    })
}

pub(crate) fn analyze_live_command(input: AnalyzeLiveCommandInput<'_>) -> Result<()> {
    let AnalyzeLiveCommandInput {
        symbol,
        futures_symbol,
        spot_symbol,
        options_symbol,
        options_volatility_proxy_symbol,
        spot_kind,
        futures_backend,
        aux_backend,
        futures_base_url,
        aux_base_url,
        state_dir,
        output_format,
        regime_consumer_bundle,
        regime_consumer_bundle_strict,
        apply_regime_bundle_bbn_soft_evidence,
    } = input;
    let regime_bundle_adapter = regime_consumer_bundle
        .map(|bundle_path| {
            RegimeConsumerBundleAdapter::load_optional(
                Some(Path::new(bundle_path)),
                regime_consumer_bundle_strict,
            )
        })
        .transpose()?;
    let resolved_symbols = resolve_live_symbol_inputs(
        symbol,
        futures_symbol,
        spot_symbol,
        options_symbol,
        spot_kind,
    )?;
    let futures_symbol = resolved_symbols.futures_symbol.as_str();
    let spot_symbol = resolved_symbols.spot_symbol.as_str();
    let options_symbol = resolved_symbols.options_symbol.as_str();
    let spot_kind_raw = resolved_symbols.spot_kind.as_str();
    let spot_kind_label = resolved_symbols.spot_kind.clone();
    let spot_kind = SpotInstrumentKind::parse(spot_kind_raw)?;
    let futures_backend = LiveDataBackend::parse(futures_backend)?;
    let aux_backend = LiveDataBackend::parse(aux_backend)?;
    let futures_provider = build_live_data_source(futures_backend, futures_base_url);
    let aux_provider = build_live_data_source(aux_backend, aux_base_url);
    let now = Utc::now();

    let htf = futures_provider.fetch_futures_candles(
        futures_symbol,
        "1d",
        now - Duration::days(420),
        now,
    )?;
    let mtf = futures_provider.fetch_futures_candles(
        futures_symbol,
        "1h",
        now - Duration::days(120),
        now,
    )?;
    let ltf = futures_provider.fetch_futures_candles(
        futures_symbol,
        "15m",
        now - Duration::days(45),
        now,
    )?;
    let htf_4h = futures_provider.fetch_futures_candles(
        futures_symbol,
        "4h",
        now - Duration::days(420),
        now,
    )?;
    let ltf_5m = futures_provider.fetch_futures_candles(
        futures_symbol,
        "5m",
        now - Duration::days(21),
        now,
    )?;
    let ltf_1m = futures_provider.fetch_futures_candles(
        futures_symbol,
        "1m",
        now - Duration::days(7),
        now,
    )?;
    let live_multi_timeframe_summary = build_live_multi_timeframe_summary(
        "live_futures_multi_timeframe",
        &[
            ("1m", &ltf_1m),
            ("5m", &ltf_5m),
            ("15m", &ltf),
            ("1h", &mtf),
            ("4h", &htf_4h),
            ("1d", &htf),
        ],
    );
    let live_multi_timeframe_signal = build_live_multi_timeframe_signal(&[
        ("1m", &ltf_1m),
        ("5m", &ltf_5m),
        ("15m", &ltf),
        ("1h", &mtf),
        ("4h", &htf_4h),
        ("1d", &htf),
    ]);
    let analyze_multi_timeframe_summary = live_multi_timeframe_summary
        .iter()
        .chain(live_multi_timeframe_signal.summary.iter())
        .cloned()
        .collect::<Vec<_>>();

    let (spot_interval, spot_lookback_days) = match spot_kind {
        SpotInstrumentKind::Commodity => ("1d", 420),
        SpotInstrumentKind::Equity | SpotInstrumentKind::Index => ("15m", 45),
    };
    let futures_live_price = futures_provider
        .fetch_futures_last_price(futures_symbol)
        .ok();
    let spot_candles = aux_provider.fetch_spot_candles(
        spot_kind,
        spot_symbol,
        Some(spot_interval),
        now - Duration::days(spot_lookback_days),
        now,
    )?;
    let spot_live_price = aux_provider
        .fetch_spot_last_price(spot_kind, spot_symbol)
        .ok();
    let options_summary =
        ict_engine::application::data_sources::fetch_options_summary_with_fallback(
            aux_provider.as_ref(),
            options_symbol,
            options_volatility_proxy_symbol,
        )
        .unwrap_or_else(|_| neutral_options_summary(options_symbol));

    let auxiliary = aux_provider.build_auxiliary_evidence(
        spot_kind,
        spot_symbol,
        options_symbol,
        &ltf,
        &spot_candles,
        &options_summary,
    );
    let params = load_or_init_hmm_params(symbol, state_dir);
    let network = load_or_init_trading_network(symbol, state_dir)?;
    let learning_state = load_learning_state(state_dir, symbol)?;
    let mut report = build_analyze_report(BuildAnalyzeReportInput {
        symbol,
        state_dir,
        htf: &htf,
        mtf: &mtf,
        ltf: &ltf,
        params: &params,
        network: &network,
        build_context: AnalyzeBuildContext {
            symbol,
            paired_candles: Some(&spot_candles),
            auxiliary: Some(&auxiliary),
            learning_state: &learning_state,
            multi_timeframe_summary: &analyze_multi_timeframe_summary,
            native_frames: AnalyzeNativeFrames {
                d1: Some(&htf),
                h4: Some(&htf_4h),
                h1: Some(&mtf),
                m30: None,
                m15: Some(&ltf),
                m5: Some(&ltf_5m),
                m1: Some(&ltf_1m),
            },
        },
        regime_bundle_adapter: regime_bundle_adapter.as_ref(),
        apply_regime_bundle_bbn_soft_evidence,
        execution_focus: true,
    })?;

    let trade_outcome_states = &network
        .nodes
        .get("trade_outcome")
        .ok_or_else(|| anyhow!("missing node 'trade_outcome'"))?
        .states;
    let long_adjusted = aux_provider.apply_auxiliary_evidence_to_outcome(
        &distribution_from_map(trade_outcome_states, &report.supporting.trade_outcome.long),
        auxiliary.long_bias - auxiliary.short_bias * 0.5,
        auxiliary.uncertainty_penalty,
    );
    let short_adjusted = aux_provider.apply_auxiliary_evidence_to_outcome(
        &distribution_from_map(trade_outcome_states, &report.supporting.trade_outcome.short),
        auxiliary.short_bias - auxiliary.long_bias * 0.5,
        auxiliary.uncertainty_penalty,
    );

    let fvgs = find_unfilled_fvgs(&mtf);
    let obs = find_untested_obs(&mtf);
    let live_decision = probabilistic_decision_snapshot(
        &report.supporting.model_state.regime_probs,
        &report.supporting.raw_trade_plan.cascade_bull,
        &report.supporting.raw_trade_plan.cascade_bear,
        &long_adjusted,
        &short_adjusted,
    );
    let mut live_trade_plan = generate_probabilistic_trade_plan(ProbabilisticTradePlanInput {
        mtf: &mtf,
        ltf: &ltf,
        fvgs: &fvgs,
        obs: &obs,
        symbol,
        regime_probs: report.supporting.model_state.regime_probs,
        cascade_bull: &report.supporting.raw_trade_plan.cascade_bull,
        cascade_bear: &report.supporting.raw_trade_plan.cascade_bear,
        bull_trade_outcome: &long_adjusted,
        bear_trade_outcome: &short_adjusted,
        config: &ProbabilisticPlanConfig::default(),
    });
    live_trade_plan
        .uncertainties
        .extend(auxiliary.notes.iter().cloned());

    let live_data_source = persist_live_data_source(PersistLiveDataSourceInput {
        state_dir,
        symbol,
        timestamp: report.timestamp,
        futures_backend: futures_backend.as_str(),
        aux_backend: aux_backend.as_str(),
        futures_base_url,
        aux_base_url,
        futures_symbol,
        spot_symbol,
        options_symbol,
        spot_kind: &spot_kind_label,
        htf: &htf,
        h4: &htf_4h,
        mtf: &mtf,
        m5: &ltf_5m,
        ltf: &ltf,
        m1: &ltf_1m,
        spot_candles: &spot_candles,
    })?;
    report.supporting.multi_timeframe_summary.extend(
        [
            live_data_source
                .persisted_h4_path
                .as_ref()
                .map(|path| format!("persisted_4h_path={}", path)),
            live_data_source
                .persisted_m5_path
                .as_ref()
                .map(|path| format!("persisted_5m_path={}", path)),
            live_data_source
                .persisted_m1_path
                .as_ref()
                .map(|path| format!("persisted_1m_path={}", path)),
        ]
        .into_iter()
        .flatten(),
    );
    report.meta.data_source = Some(live_data_source.clone());
    report.supporting.auxiliary = Some(auxiliary);
    report.supporting.model_state.evidence_policy =
        "hmm_prior_times_pre_bayes_evidence_filter_times_bbn_trade_probability_plus_spot_options_auxiliary"
            .to_string();
    report.supporting.decision = live_decision;
    report.supporting.trade_outcome.long = probability_map(trade_outcome_states, &long_adjusted);
    report.supporting.trade_outcome.short = probability_map(trade_outcome_states, &short_adjusted);
    report.supporting.raw_trade_plan = live_trade_plan;
    let auxiliary = report
        .supporting
        .auxiliary
        .as_ref()
        .context("missing auxiliary live evidence after live data assembly")?;
    report.analysis.technical_price = build_technical_price_section(
        &ltf,
        futures_live_price,
        spot_live_price,
        report.supporting.auxiliary.as_ref(),
    );
    report.analysis.smt_correlation =
        build_smt_correlation_section(futures_symbol, spot_symbol, &ltf, &spot_candles, auxiliary)?;
    report.analysis.regime_bayesian = build_regime_bayesian_section(
        &report.supporting.model_state.hmm_state,
        &report.supporting.model_state.regime_probs,
        &report.supporting.labels.regime_label,
        &report.supporting.labels.liquidity_label,
        &report.supporting.decision,
        &report.supporting.model_state.evidence_policy,
        Some(&report.analysis.technical_price.options_hedging),
        None,
        None,
    );
    report.analysis.multi_timeframe = build_analyze_multi_timeframe_section(
        &report.supporting.multi_timeframe_summary,
        Some(&report.supporting.pre_bayes_evidence_filter),
    );
    report.analysis.trade_plan = build_trade_plan_section(
        &report.supporting.raw_trade_plan,
        Some(&report.analysis.technical_price.options_hedging),
    );
    let pending_update_file =
        persist_pending_update_artifact_from_analyze(state_dir, &report, "analyze-live")?;
    let _execution_candidate_file =
        persist_execution_candidate_from_analyze(state_dir, &report, "analyze-live")?;
    let (artifact_factor_trends, artifact_family_trends) =
        artifact_trend_summaries_for_symbol(state_dir, symbol)?;
    let artifact_consumed_impact_summary =
        artifact_consumed_impact_summary_for_symbol(state_dir, symbol)?;
    augment_action_plan_with_artifact_trends(
        &mut report.supporting.agent_action_plan,
        symbol,
        state_dir,
        &artifact_factor_trends,
        &artifact_family_trends,
        &artifact_consumed_impact_summary,
    );
    report.supporting.artifact_action_summary = artifact_action_summary(
        &artifact_factor_trends,
        &artifact_family_trends,
        &artifact_consumed_impact_summary,
    );
    if let Some(bundle_path) = regime_consumer_bundle {
        if let Some(adapter) = regime_bundle_adapter.as_ref() {
            let trace_entries = adapter.trace_entries(Some(Path::new(bundle_path)));
            report
                .supporting
                .artifact_action_summary
                .push(format!("regime_bundle_trace:{}", trace_entries.join("|")));
            report
                .supporting
                .artifact_action_summary
                .extend(trace_entries);
            adapter.append_read_only_bbn_diagnostics(
                &mut report.supporting.artifact_action_summary,
                &mut report.supporting.pre_bayes_evidence_filter,
            );
        }
    }
    report.supporting.artifact_decision_summary =
        artifact_decision_summary_for_symbol(state_dir, symbol)?;
    report.supporting.artifact_decision_section = artifact_decision_section_from_parts(
        &report.supporting.artifact_decision_summary,
        &report.supporting.artifact_action_summary,
        &artifact_factor_trends,
        &artifact_family_trends,
        &artifact_rule_break_effects_for_symbol(state_dir, symbol)?,
        &artifact_consumed_impact_summary,
    );
    apply_command_context_to_analyze_report(
        &mut report,
        &CommandContext {
            symbol: symbol.to_string(),
            state_dir: state_dir.to_string(),
            analyze: Some(AnalyzeCommandSource::Live {
                source: Box::new(live_data_source.clone()),
            }),
            research_data: live_data_source.persisted_ltf_path.clone(),
            paired_data: live_data_source.persisted_spot_path.clone(),
            update_outcome: None,
            update_entry_signal: None,
            update_feedback_file: Some(pending_update_file),
            user_data_selection_required: true,
        },
    );
    report.supporting.workflow_snapshot = persist_analyze_run(
        state_dir,
        &report,
        "analyze-live",
        None,
        None,
        None,
        Some(live_data_source),
    )?;
    report.supporting.artifact_decision_summary = artifact_decision_summary_from_snapshot(
        &report.supporting.workflow_snapshot,
        &report.supporting.artifact_action_summary,
    );
    report.supporting.artifact_decision_section =
        artifact_decision_section_from_snapshot(&report.supporting.workflow_snapshot);
    append_artifact_decision_prompt(
        &mut report.supporting.agent_prompts,
        symbol,
        &report.supporting.artifact_decision_section,
    );
    link_artifact_decision_summary_to_decisions(
        &report.supporting.artifact_decision_summary,
        &mut report.supporting.promotion_decision,
        &mut report.supporting.rollback_recommendation,
    );

    ict_engine::application::reporting::dispatch_analyze_live_output(
        &report,
        ict_engine::application::reporting::AnalyzeLiveOutputDispatchInput {
            output_format,
            include_pda_sequence_summary: false,
            redact_paths: false,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_live_symbol_inputs_infers_catalog_defaults_for_nq() {
        let resolved = resolve_live_symbol_inputs("NQ", None, None, None, None).unwrap();

        assert_eq!(resolved.futures_symbol, "NQ=F");
        assert_eq!(resolved.spot_symbol, "QQQ");
        assert_eq!(resolved.options_symbol, "QQQ");
        assert_eq!(resolved.spot_kind, "equity");
    }

    #[test]
    fn resolve_live_symbol_inputs_guides_when_defaults_are_missing() {
        let err = resolve_live_symbol_inputs("caller-symbol", None, None, None, None).unwrap_err();

        assert!(err
            .to_string()
            .contains("provider-status --domain live_runtime --agent"));
        assert!(err.to_string().contains("NQ, ES, GC, or CL"));
    }

    #[test]
    fn resolve_live_symbol_inputs_accepts_explicit_symbols() {
        let resolved = resolve_live_symbol_inputs(
            "caller-symbol",
            Some("ES=F"),
            Some("SPY"),
            Some("SPY"),
            Some("equity"),
        )
        .unwrap();

        assert_eq!(resolved.futures_symbol, "ES=F");
        assert_eq!(resolved.spot_symbol, "SPY");
        assert_eq!(resolved.options_symbol, "SPY");
        assert_eq!(resolved.spot_kind, "equity");
    }
}
