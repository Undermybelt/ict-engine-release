use ict_engine::application::data_sources::{
    build_market_data_harness_plan, MarketDataHarnessRequest, MarketDataHarnessSymbolSpec,
};
use ict_engine::market_catalog::load_market_catalog;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn public_help_avoids_repo_market_examples() {
    let binary = env!("CARGO_BIN_EXE_ict-engine");

    let root_help = Command::new(binary).arg("--help").output().unwrap();
    let root_help = String::from_utf8(root_help.stdout).unwrap();
    assert!(!root_help.contains("NQ, ES, GC"));
    assert!(root_help.contains("Start here:"));
    assert!(root_help.contains("workflow-status --symbol DEMO"));
    assert!(root_help.contains("provider-status --compact"));

    let harness_help = Command::new(binary)
        .args(["market-data-harness", "--help"])
        .output()
        .unwrap();
    let harness_help = String::from_utf8(harness_help.stdout).unwrap();
    assert!(!harness_help.contains("NQ, ES, AAPL, BTCUSDT"));
}

#[test]
fn harness_plan_rejects_repo_label_without_explicit_symbol_specs() {
    let err = build_market_data_harness_plan(MarketDataHarnessRequest {
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
    })
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("market-data-harness request validation failed"));
}

#[test]
fn harness_plan_uses_only_explicit_request_data() {
    let plan = build_market_data_harness_plan(MarketDataHarnessRequest {
        market_key: "caller-label".to_string(),
        primary_data_path: None,
        interval: Some("1d".to_string()),
        start: None,
        end: None,
        count: Some(30),
        related_roles: vec![
            "etf_reference".to_string(),
            "options_underlying".to_string(),
        ],
        provider_preferences: BTreeMap::from([
            ("etf_reference".to_string(), "yfinance".to_string()),
            (
                "options_underlying".to_string(),
                "tradingview_mcp".to_string(),
            ),
        ]),
        symbol_overrides: BTreeMap::from([
            (
                "etf_reference".to_string(),
                MarketDataHarnessSymbolSpec {
                    display_symbol: Some("SPY".to_string()),
                    yfinance: Some("SPY".to_string()),
                    ..MarketDataHarnessSymbolSpec::default()
                },
            ),
            (
                "options_underlying".to_string(),
                MarketDataHarnessSymbolSpec {
                    display_symbol: Some("AMEX:SPY".to_string()),
                    tradingview_mcp: Some("AMEX:SPY".to_string()),
                    ..MarketDataHarnessSymbolSpec::default()
                },
            ),
        ]),
        options_volatility_proxy_symbol: Some("^VIX".to_string()),
    })
    .unwrap();

    assert_eq!(plan.tasks.len(), 2);
    assert!(plan
        .tasks
        .iter()
        .any(|task| task.role == "etf_reference" && task.request_symbol() == "SPY"));
    assert!(plan.tasks.iter().any(|task| {
        task.role == "options_underlying"
            && task.request_symbol() == "AMEX:SPY"
            && task.fallback_options_proxy_symbol.as_deref() == Some("^VIX")
    }));
}

#[test]
fn repo_market_pack_is_loadable_only_when_called_explicitly() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let catalog = load_market_catalog(&repo_root).unwrap();

    let es = catalog.live_defaults("ES").unwrap();
    assert_eq!(es.futures_symbol, "ES=F");
    assert_eq!(es.spot_symbol, "SPY");
}

#[test]
fn bootstrap_output_keeps_actionable_local_paths_and_valid_commands() {
    let binary = env!("CARGO_BIN_EXE_ict-engine");
    let state = TempDir::new().unwrap();

    let output = Command::new(binary)
        .args([
            "workflow-status",
            "--symbol",
            "DEMO",
            "--state-dir",
            state.path().to_str().unwrap(),
            "--phase",
            "agent-bootstrap",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("--market "));
    assert!(!stdout.contains("<local-path>"));
    assert!(stdout.contains(state.path().to_str().unwrap()));
}

#[test]
fn bootstrap_output_does_not_auto_reuse_personal_tomac_paths_without_profile() {
    let binary = env!("CARGO_BIN_EXE_ict-engine");
    let state = TempDir::new().unwrap();

    let output = Command::new(binary)
        .args([
            "workflow-status",
            "--symbol",
            "NQ",
            "--state-dir",
            state.path().to_str().unwrap(),
            "--phase",
            "agent-bootstrap",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("/Users/thrill3r/Downloads/Tomac"));
    assert!(!stdout.contains("ict-cleaned-mtf"));
    assert!(stdout.contains("<tomac-root>"));
    assert!(stdout.contains("<output-dir>"));
}

#[test]
fn provider_status_agent_accepts_opt_in_profile_path() {
    let binary = env!("CARGO_BIN_EXE_ict-engine");
    let profile_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/provider_profiles/thrill3r-nq-closed-loop-v1.json");

    let output = Command::new(binary)
        .args([
            "provider-status",
            "--agent",
            "--profile",
            profile_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value["selected_profile"]["profile_id"],
        "thrill3r_nq_closed_loop_v1"
    );
    assert_eq!(value["selected_profile"]["opt_in_only"], true);
    assert!(value["selected_profile"]["data_contract_labels"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item
            .as_str()
            .unwrap()
            .contains("Tomac cleaned multi-timeframe futures root")));
}

#[test]
fn provider_status_agent_hides_opt_in_profiles_without_selecting_one() {
    let binary = env!("CARGO_BIN_EXE_ict-engine");

    let output = Command::new(binary)
        .args(["provider-status", "--agent"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(value.get("available_opt_in_profiles").is_none());
    assert!(value["selected_profile"].is_null());
    assert!(value["providers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|provider| provider["provider_id"] == "yfinance"
            && provider["user_access"] == "free_no_login"));
}

#[test]
fn workflow_status_agent_accepts_opt_in_profile_path() {
    let binary = env!("CARGO_BIN_EXE_ict-engine");
    let state = TempDir::new().unwrap();
    let profile_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/provider_profiles/thrill3r-nq-closed-loop-v1.json");

    let output = Command::new(binary)
        .args([
            "workflow-status",
            "--symbol",
            "DEMO",
            "--state-dir",
            state.path().to_str().unwrap(),
            "--agent",
            "--profile",
            profile_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["selected_profile_id"], "thrill3r_nq_closed_loop_v1");
    assert_eq!(
        value["provider_support"]["profile_id"],
        "thrill3r_nq_closed_loop_v1"
    );
    assert!(value["selected_profile_track_statuses"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item.as_str().unwrap().contains("live_zero_config")));
    assert!(value["provider_support"]["workflow_support"]["selected_profile"].is_null());
}

#[test]
fn workflow_status_human_hides_opt_in_profile_hint_without_selecting_one() {
    let binary = env!("CARGO_BIN_EXE_ict-engine");
    let state = TempDir::new().unwrap();

    let output = Command::new(binary)
        .args([
            "workflow-status",
            "--symbol",
            "DEMO",
            "--state-dir",
            state.path().to_str().unwrap(),
            "--human",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("Profiles: opt-in only."));
    assert!(stdout.contains("Provider: tradfi free fallback="));
    assert!(stdout.contains("Routes: replay="));
    assert!(stdout.contains("--phase bootstrap --human"));
}

#[test]
fn provider_status_single_provider_compact_surfaces_setup_prompts() {
    let binary = env!("CARGO_BIN_EXE_ict-engine");
    let temp_home = TempDir::new().unwrap();

    let output = Command::new(binary)
        .env("HOME", temp_home.path())
        .env("XDG_CONFIG_HOME", temp_home.path().join(".config"))
        .env("XDG_DATA_HOME", temp_home.path().join(".local/share"))
        .env("XDG_STATE_HOME", temp_home.path().join(".local/state"))
        .args(["provider-status", "--provider", "ibkr", "--compact"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("detail: ibkr | access=login_and_local_runtime"));
    assert!(stdout.contains("setup:"));
}

#[test]
fn analyze_live_and_pre_bayes_help_expose_human_output_modes() {
    let binary = env!("CARGO_BIN_EXE_ict-engine");

    let analyze_live_help = Command::new(binary)
        .args(["analyze-live", "--help"])
        .output()
        .unwrap();
    assert!(analyze_live_help.status.success());
    let analyze_live_help = String::from_utf8(analyze_live_help.stdout).unwrap();
    assert!(analyze_live_help.contains("--human"));
    assert!(analyze_live_help.contains("--output-format"));

    let pre_bayes_help = Command::new(binary)
        .args(["pre-bayes-status", "--help"])
        .output()
        .unwrap();
    assert!(pre_bayes_help.status.success());
    let pre_bayes_help = String::from_utf8(pre_bayes_help.stdout).unwrap();
    assert!(pre_bayes_help.contains("--human"));
    assert!(pre_bayes_help.contains("--compact"));
}

#[test]
fn auto_quant_status_help_and_human_surface_expose_consumer_output_modes() {
    let binary = env!("CARGO_BIN_EXE_ict-engine");
    let state = TempDir::new().unwrap();

    let help = Command::new(binary)
        .args(["auto-quant-status", "--help"])
        .output()
        .unwrap();
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).unwrap();
    assert!(help.contains("--human"));
    assert!(help.contains("--compact"));

    let output = Command::new(binary)
        .args([
            "auto-quant-status",
            "--state-dir",
            state.path().to_str().unwrap(),
            "--human",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Auto-Quant status | missing_dependency"));
    assert!(stdout.contains("Run: ict-engine auto-quant-bootstrap"));
    assert!(!stdout.trim_start().starts_with('{'));
}

#[test]
fn workflow_status_human_empty_state_suppresses_validation_noise() {
    let binary = env!("CARGO_BIN_EXE_ict-engine");
    let state = TempDir::new().unwrap();

    let output = Command::new(binary)
        .args([
            "workflow-status",
            "--symbol",
            "DEMO",
            "--state-dir",
            state.path().to_str().unwrap(),
            "--human",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("Validation:"));
}

#[test]
fn workflow_status_human_empty_state_suppresses_ranker_noise() {
    let binary = env!("CARGO_BIN_EXE_ict-engine");
    let state = TempDir::new().unwrap();

    let output = Command::new(binary)
        .args([
            "workflow-status",
            "--symbol",
            "DEMO",
            "--state-dir",
            state.path().to_str().unwrap(),
            "--human",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("Ranker:"));
}

#[test]
fn workflow_status_structural_validation_phase_is_available() {
    let binary = env!("CARGO_BIN_EXE_ict-engine");
    let state = TempDir::new().unwrap();

    let output = Command::new(binary)
        .args([
            "workflow-status",
            "--symbol",
            "DEMO",
            "--state-dir",
            state.path().to_str().unwrap(),
            "--phase",
            "structural-validation",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(value["source_reliability"]["status"].is_string());
    assert!(value.get("recommended_next_step").is_some());
}

#[test]
fn workflow_status_structural_ranker_runtime_phase_is_available() {
    let binary = env!("CARGO_BIN_EXE_ict-engine");
    let state = TempDir::new().unwrap();

    let output = Command::new(binary)
        .args([
            "workflow-status",
            "--symbol",
            "DEMO",
            "--state-dir",
            state.path().to_str().unwrap(),
            "--phase",
            "structural-ranker-runtime",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(value.get("recommended_next_step").is_some());
}
