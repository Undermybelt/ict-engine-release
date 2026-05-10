use anyhow::Result;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;

use crate::application::data_sources::{
    build_market_data_harness_plan, execute_market_data_harness_plan, MarketDataHarnessRequest,
    MarketDataHarnessSymbolSpec,
};
use crate::application::multi_timeframe_inputs::resolve_tomac_root;

pub struct ExpansionSopCommandInput<'a> {
    pub root: Option<&'a str>,
    pub output_dir: &'a str,
    pub interval: &'a str,
    pub lookback: usize,
    pub atr_multiplier: f64,
    pub objective: &'a str,
    pub mutation_spec_path: Option<&'a str>,
    pub emit_mutation_evaluation: bool,
}

pub struct MarketDataHarnessCommandInput<'a> {
    pub market: Option<&'a str>,
    pub primary_data: Option<&'a str>,
    pub interval: Option<&'a str>,
    pub related_roles: &'a [String],
    pub provider_preferences: &'a [String],
    pub request_json: Option<&'a str>,
    pub request_stdin: bool,
    pub symbol_specs: &'a [String],
    pub options_volatility_proxy_symbol: Option<&'a str>,
}

pub fn clean_futures_command<FMulti, FSingle, TMulti, TSingle>(
    root: Option<&str>,
    output_dir: &str,
    interval: &str,
    multi_timeframe: bool,
    run_multi_timeframe: FMulti,
    run_single: FSingle,
) -> Result<()>
where
    FMulti: Fn(&str, &str) -> Result<TMulti>,
    FSingle: Fn(&str, &str, &str) -> Result<TSingle>,
    TMulti: Serialize,
    TSingle: Serialize,
{
    let root = resolve_tomac_root(root)?;
    if multi_timeframe {
        let report = run_multi_timeframe(&root, output_dir)?;
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        let report = run_single(&root, output_dir, interval)?;
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}

pub fn futures_sop_command<FRun, TReport>(
    root: Option<&str>,
    output_dir: &str,
    interval: &str,
    run_sop: FRun,
) -> Result<()>
where
    FRun: Fn(&str, &str, &str) -> Result<TReport>,
    TReport: Serialize,
{
    let root = resolve_tomac_root(root)?;
    let report = run_sop(&root, output_dir, interval)?;
    let report_path = std::path::Path::new(output_dir)
        .join(format!("futures_sop_report.{}.json", interval))
        .to_string_lossy()
        .to_string();
    std::fs::write(&report_path, serde_json::to_string_pretty(&report)?)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

pub fn expansion_sop_command<FLoad, FObjective, FRun, FPayload, TMutationSpec, TReport>(
    input: ExpansionSopCommandInput<'_>,
    parse_objective: FObjective,
    load_mutation_spec: FLoad,
    run_sop: FRun,
    build_payload: FPayload,
) -> Result<()>
where
    FLoad: Fn(&str) -> Result<TMutationSpec>,
    FObjective: Fn(&str) -> Result<crate::application::decision_utils::ResearchObjectiveMode>,
    FRun: Fn(
        &str,
        &str,
        &str,
        usize,
        f64,
        crate::application::decision_utils::ResearchObjectiveMode,
        Option<&TMutationSpec>,
    ) -> Result<TReport>,
    FPayload: Fn(&TReport, Option<&TMutationSpec>, bool) -> Result<serde_json::Value>,
    TMutationSpec: Serialize,
    TReport: Serialize,
{
    let ExpansionSopCommandInput {
        root,
        output_dir,
        interval,
        lookback,
        atr_multiplier,
        objective,
        mutation_spec_path,
        emit_mutation_evaluation,
    } = input;
    let objective_mode = parse_objective(objective)?;
    let root = resolve_tomac_root(root)?;
    let mutation_spec = mutation_spec_path.map(load_mutation_spec).transpose()?;
    let report = run_sop(
        &root,
        output_dir,
        interval,
        lookback,
        atr_multiplier,
        objective_mode,
        mutation_spec.as_ref(),
    )?;
    let report_path = std::path::Path::new(output_dir)
        .join(format!("expansion_sop_report.{}.json", interval))
        .to_string_lossy()
        .to_string();
    std::fs::write(&report_path, serde_json::to_string_pretty(&report)?)?;
    let payload = build_payload(&report, mutation_spec.as_ref(), emit_mutation_evaluation)?;
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

pub fn market_data_harness_plan_command(input: MarketDataHarnessCommandInput<'_>) -> Result<()> {
    let request = load_or_build_market_data_harness_request(input)?;
    let plan = build_market_data_harness_plan(request)?;
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}

pub fn market_data_harness_fetch_command(input: MarketDataHarnessCommandInput<'_>) -> Result<()> {
    let request = load_or_build_market_data_harness_request(input)?;
    let plan = build_market_data_harness_plan(request)?;
    let bundle = execute_market_data_harness_plan(&plan)?;
    println!("{}", serde_json::to_string_pretty(&bundle)?);
    let failures = collect_harness_failures(&bundle);
    if !failures.is_empty() {
        anyhow::bail!(
            "market-data-harness fetch encountered failures: {}",
            failures.join(" | ")
        );
    }
    Ok(())
}

fn collect_harness_failures(
    bundle: &crate::application::data_sources::MarketDataHarnessBundle,
) -> Vec<String> {
    let failed_providers = bundle
        .results
        .iter()
        .filter(|result| !result.ok)
        .map(|result| result.provider.clone())
        .collect::<BTreeSet<_>>();
    let mut failures = bundle
        .plan
        .missing_roles
        .iter()
        .map(|role| format!("missing_role={role}"))
        .collect::<Vec<_>>();
    failures.extend(
        bundle
            .results
            .iter()
            .filter(|result| !result.ok)
            .map(|result| {
                let message = result
                    .error
                    .as_ref()
                    .map(|error| format!("{}:{}", error.category, error.message))
                    .unwrap_or_else(|| "unknown_error".to_string());
                format!(
                    "role={} provider={} symbol={} {}",
                    result.role,
                    result.provider,
                    result.symbol.as_deref().unwrap_or("<none>"),
                    message
                )
            }),
    );
    if !failures.is_empty() {
        let relevant_prompts = if failed_providers.is_empty() {
            bundle
                .plan
                .provider_summary
                .actionable_install_prompts
                .to_vec()
        } else {
            bundle
                .plan
                .provider_summary
                .provider_statuses
                .iter()
                .filter(|status| failed_providers.contains(&status.provider))
                .flat_map(|status| status.install_prompts.iter().cloned())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
        };
        failures.extend(
            relevant_prompts
                .iter()
                .map(|item| format!("install_prompt={item}")),
        );
    }
    failures
}

fn load_or_build_market_data_harness_request(
    input: MarketDataHarnessCommandInput<'_>,
) -> Result<MarketDataHarnessRequest> {
    if let Some(path) = input.request_json {
        let raw = std::fs::read_to_string(path)?;
        return Ok(serde_json::from_str(&raw)?);
    }
    if input.request_stdin {
        let mut raw = String::new();
        std::io::stdin().read_to_string(&mut raw)?;
        return Ok(serde_json::from_str(&raw)?);
    }
    let provider_preferences = input
        .provider_preferences
        .iter()
        .filter_map(|item| item.split_once('='))
        .map(|(role, provider)| (role.trim().to_string(), provider.trim().to_string()))
        .collect();
    let symbol_overrides = parse_symbol_specs(input.symbol_specs, &provider_preferences)?;
    Ok(MarketDataHarnessRequest {
        market_key: input.market.unwrap_or("caller-request").to_string(),
        primary_data_path: input.primary_data.map(str::to_string),
        interval: input.interval.map(str::to_string),
        start: None,
        end: None,
        count: None,
        related_roles: input.related_roles.to_vec(),
        provider_preferences,
        symbol_overrides,
        options_volatility_proxy_symbol: input.options_volatility_proxy_symbol.map(str::to_string),
    })
}

fn parse_symbol_specs(
    specs: &[String],
    provider_preferences: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, MarketDataHarnessSymbolSpec>> {
    let mut symbol_overrides = BTreeMap::new();
    for item in specs {
        let Some((role, symbol)) = item.split_once('=') else {
            anyhow::bail!("invalid --symbol-spec '{}'; expected role=symbol", item);
        };
        let role = role.trim().to_string();
        let symbol = symbol.trim().to_string();
        if symbol.is_empty() {
            anyhow::bail!("invalid --symbol-spec '{}'; symbol cannot be empty", item);
        }
        let mut spec = MarketDataHarnessSymbolSpec {
            display_symbol: Some(symbol.clone()),
            ..MarketDataHarnessSymbolSpec::default()
        };
        match provider_preferences.get(&role).map(String::as_str) {
            Some("yfinance") => spec.yfinance = Some(symbol),
            Some("tradingview_mcp") => spec.tradingview_mcp = Some(symbol),
            Some("ibkr") => {
                anyhow::bail!(
                    "role '{}' uses ibkr; provide a full request JSON with an explicit ibkr contract",
                    role
                );
            }
            _ => {}
        }
        symbol_overrides.insert(role, spec);
    }
    Ok(symbol_overrides)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::data_sources::harness::MarketDataHarnessError;
    use crate::application::data_sources::{
        MarketDataHarnessBundle, MarketDataHarnessEnvelope, MarketDataHarnessOperation,
        MarketDataHarnessPlan, MarketDataHarnessRequest,
    };
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    #[test]
    fn clean_futures_command_routes_to_single_interval_path() {
        let called_single = Arc::new(AtomicBool::new(false));
        let single_flag = Arc::clone(&called_single);

        clean_futures_command(
            Some("/tmp/root"),
            "/tmp/out",
            "15m",
            false,
            |_root, _out| Ok(json!({"mode": "multi"})),
            move |root, out, interval| {
                assert_eq!(root, "/tmp/root");
                assert_eq!(out, "/tmp/out");
                assert_eq!(interval, "15m");
                single_flag.store(true, Ordering::SeqCst);
                Ok(json!({"mode": "single"}))
            },
        )
        .unwrap();

        assert!(called_single.load(Ordering::SeqCst));
    }

    #[test]
    fn futures_sop_command_persists_report_file() {
        let temp = tempfile::tempdir().unwrap();
        futures_sop_command(
            Some("/tmp/root"),
            temp.path().to_str().unwrap(),
            "15m",
            |_root, _out, _interval| Ok(json!({"report": true})),
        )
        .unwrap();

        let report_path = temp.path().join("futures_sop_report.15m.json");
        assert!(report_path.exists());
    }

    #[test]
    fn expansion_sop_command_writes_payload_and_report() {
        let temp = tempfile::tempdir().unwrap();
        expansion_sop_command(
            ExpansionSopCommandInput {
                root: Some("/tmp/root"),
                output_dir: temp.path().to_str().unwrap(),
                interval: "15m",
                lookback: 20,
                atr_multiplier: 1.5,
                objective: "generic",
                mutation_spec_path: None,
                emit_mutation_evaluation: false,
            },
            |_objective| Ok(crate::application::decision_utils::ResearchObjectiveMode::Generic),
            |_path| Ok(json!({"mutation": true})),
            |_root, _out, _interval, _lookback, _atr_multiplier, _objective, _mutation_spec| {
                Ok(json!({"report": true}))
            },
            |report, mutation_spec, emit_mutation_evaluation| {
                Ok(json!({
                    "report": report,
                    "mutation_spec": mutation_spec,
                    "emit_mutation_evaluation": emit_mutation_evaluation
                }))
            },
        )
        .unwrap();

        let report_path = temp.path().join("expansion_sop_report.15m.json");
        assert!(report_path.exists());
    }

    #[test]
    fn harness_fetch_failures_are_reported_to_cli() {
        let bundle = MarketDataHarnessBundle {
            plan: MarketDataHarnessPlan {
                request: MarketDataHarnessRequest {
                    market_key: "ES".to_string(),
                    primary_data_path: None,
                    interval: Some("1d".to_string()),
                    start: None,
                    end: None,
                    count: Some(30),
                    related_roles: vec!["etf_reference".to_string()],
                    provider_preferences: BTreeMap::new(),
                    symbol_overrides: BTreeMap::new(),
                    options_volatility_proxy_symbol: None,
                },
                provider_summary: Default::default(),
                tasks: Vec::new(),
                missing_roles: vec!["volatility_reference".to_string()],
                warnings: vec!["missing_provider_for_role=volatility_reference".to_string()],
            },
            results: vec![MarketDataHarnessEnvelope {
                ok: false,
                provider: "yfinance".to_string(),
                operation: MarketDataHarnessOperation::Ohlcv.as_str().to_string(),
                role: "etf_reference".to_string(),
                symbol: Some("SPY".to_string()),
                data: None,
                error: Some(MarketDataHarnessError {
                    category: "fetch_failed".to_string(),
                    message: "rate limited".to_string(),
                    retryable: true,
                }),
            }],
        };

        let failures = collect_harness_failures(&bundle);
        assert!(failures
            .iter()
            .any(|item| item.contains("missing_role=volatility_reference")));
        assert!(failures
            .iter()
            .any(|item| item.contains("role=etf_reference")));
    }

    #[test]
    fn harness_fetch_success_does_not_report_install_prompts_as_failures() {
        let bundle = MarketDataHarnessBundle {
            plan: MarketDataHarnessPlan {
                request: MarketDataHarnessRequest {
                    market_key: "ES".to_string(),
                    primary_data_path: None,
                    interval: Some("1d".to_string()),
                    start: None,
                    end: None,
                    count: Some(30),
                    related_roles: vec!["etf_reference".to_string()],
                    provider_preferences: BTreeMap::new(),
                    symbol_overrides: BTreeMap::new(),
                    options_volatility_proxy_symbol: None,
                },
                provider_summary: crate::application::data_sources::ControlMatrixProviderSummary {
                    actionable_install_prompts: vec!["install me".to_string()],
                    ..Default::default()
                },
                tasks: Vec::new(),
                missing_roles: Vec::new(),
                warnings: Vec::new(),
            },
            results: vec![MarketDataHarnessEnvelope {
                ok: true,
                provider: "yfinance".to_string(),
                operation: MarketDataHarnessOperation::Ohlcv.as_str().to_string(),
                role: "etf_reference".to_string(),
                symbol: Some("SPY".to_string()),
                data: Some(json!([])),
                error: None,
            }],
        };

        let failures = collect_harness_failures(&bundle);
        assert!(failures.is_empty());
    }
}
