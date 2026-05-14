use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::application::backtest::{ControlMatrixPlan, Pb12Toggle, PB12_TOGGLES};

pub const TVREMIX_MCP_DEFAULT_URL: &str = "https://tvremix.xyz/api/mcp/v1";
pub const TVREMIX_MCP_URL_ENV: &str = "ICT_ENGINE_TVREMIX_MCP_URL";
pub const TVREMIX_MCP_API_KEY_ENV: &str = "ICT_ENGINE_TVREMIX_MCP_API_KEY";
pub const TVREMIX_MCP_LOCAL_CONFIG_RELATIVE_PATH: &str = ".ict-engine/tvremix_mcp.json";
pub const TRADINGVIEW_MCP_CMD_ENV: &str = "ICT_ENGINE_TRADINGVIEW_MCP_CMD";
pub const TRADINGVIEW_MCP_ARGS_ENV: &str = "ICT_ENGINE_TRADINGVIEW_MCP_ARGS";
pub const IBKR_CONSENT_RELATIVE_PATH: &str = ".ict-engine/ibkr_consent.json";
pub const IBKR_CAPABILITIES_RELATIVE_PATH: &str = ".ict-engine/ibkr_capabilities.json";
pub const IBKR_GATEWAY_PORT_CANDIDATES: [(&str, u16); 4] = [
    ("TWS paper", 7497u16),
    ("TWS live", 7496u16),
    ("IB Gateway paper", 4002u16),
    ("IB Gateway live", 4001u16),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IbkrGatewayCandidate {
    pub label: &'static str,
    pub port: u16,
    pub reachable: bool,
    pub recommended: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct IbkrRuntimeProbeDetails {
    pub ready: bool,
    pub missing_modules: Vec<String>,
    pub stderr_excerpt: Option<String>,
    pub gateway_candidates: Vec<IbkrGatewayCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct TradingviewMcpProbeDetails {
    connectivity_ok: bool,
    ohlcv_ok: Option<bool>,
    options_ok: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TradingviewMcpRuntimeConfig {
    pub url: String,
    pub api_key: Option<String>,
    pub credential_source: &'static str,
    pub local_config_present: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlMatrixDataRequirement {
    EtfReference,
    CfdReference,
    VixOverlay,
    OptionsGreeks,
    OptionsOpenInterest,
    OptionsImpliedVolatility,
}

impl ControlMatrixDataRequirement {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EtfReference => "etf_reference",
            Self::CfdReference => "cfd_reference",
            Self::VixOverlay => "vix_overlay",
            Self::OptionsGreeks => "options_greeks",
            Self::OptionsOpenInterest => "options_open_interest",
            Self::OptionsImpliedVolatility => "options_implied_volatility",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ControlMatrixProviderKind {
    Ibkr,
    YahooFinance,
    TradingViewMcp,
}

impl ControlMatrixProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ibkr => "ibkr",
            Self::YahooFinance => "yfinance",
            Self::TradingViewMcp => "tradingview_mcp",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlMatrixProviderStatus {
    pub provider: String,
    pub status: String,
    pub healthy: bool,
    pub reason: String,
    pub supported_requirements: Vec<String>,
    pub install_prompts: Vec<String>,
    pub redacted_config: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ControlMatrixProviderSummary {
    pub required_requirements: Vec<String>,
    pub provider_statuses: Vec<ControlMatrixProviderStatus>,
    pub actionable_install_prompts: Vec<String>,
}

pub fn build_control_matrix_provider_summary(
    plan: &ControlMatrixPlan,
) -> ControlMatrixProviderSummary {
    build_provider_summary_for_requirements_with_env(
        required_requirements_for_plan(plan),
        &|name| std::env::var(name).ok(),
        home_dir(),
        &ibkr_runtime_probe_details,
        &tradingview_mcp_probe_details,
    )
}

pub fn build_provider_summary_for_requirements(
    required: BTreeSet<ControlMatrixDataRequirement>,
) -> ControlMatrixProviderSummary {
    build_provider_summary_for_requirements_with_env(
        required,
        &|name| std::env::var(name).ok(),
        home_dir(),
        &ibkr_runtime_probe_details,
        &tradingview_mcp_probe_details,
    )
}

fn build_provider_summary_for_requirements_with_env<F, T>(
    required: BTreeSet<ControlMatrixDataRequirement>,
    env_lookup: &F,
    home_dir: Option<PathBuf>,
    ibkr_runtime_probe: &dyn Fn() -> IbkrRuntimeProbeDetails,
    tradingview_probe: &T,
) -> ControlMatrixProviderSummary
where
    F: Fn(&str) -> Option<String>,
    T: Fn(&BTreeSet<ControlMatrixDataRequirement>, &str, &str) -> TradingviewMcpProbeDetails,
{
    let provider_statuses = vec![
        ibkr_provider_status(&required, home_dir.as_deref(), &ibkr_runtime_probe()),
        yfinance_provider_status(&required, env_lookup),
        tradingview_mcp_provider_status(
            &required,
            env_lookup,
            home_dir.as_deref(),
            tradingview_probe,
        ),
    ];
    let actionable_install_prompts = provider_statuses
        .iter()
        .flat_map(|status| status.install_prompts.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    ControlMatrixProviderSummary {
        required_requirements: required
            .iter()
            .map(|item| item.as_str().to_string())
            .collect(),
        provider_statuses,
        actionable_install_prompts,
    }
}

fn required_requirements_for_plan(
    plan: &ControlMatrixPlan,
) -> BTreeSet<ControlMatrixDataRequirement> {
    let mut required = BTreeSet::new();
    for run in &plan.runs {
        for toggle in PB12_TOGGLES {
            if run.toggle_value(toggle) {
                if let Some(requirement) = requirement_for_toggle(toggle) {
                    required.insert(requirement);
                }
            }
        }
    }
    required
}

fn requirement_for_toggle(toggle: Pb12Toggle) -> Option<ControlMatrixDataRequirement> {
    match toggle {
        Pb12Toggle::UseGreeks => Some(ControlMatrixDataRequirement::OptionsGreeks),
        Pb12Toggle::UseOi => Some(ControlMatrixDataRequirement::OptionsOpenInterest),
        Pb12Toggle::UseIv => Some(ControlMatrixDataRequirement::OptionsImpliedVolatility),
        Pb12Toggle::UseEtf => Some(ControlMatrixDataRequirement::EtfReference),
        Pb12Toggle::UseCfd => Some(ControlMatrixDataRequirement::CfdReference),
        Pb12Toggle::UseVix => Some(ControlMatrixDataRequirement::VixOverlay),
        Pb12Toggle::UseDailyStructure | Pb12Toggle::UseWeeklyStructure => None,
    }
}

fn ibkr_provider_status(
    required: &BTreeSet<ControlMatrixDataRequirement>,
    home_dir: Option<&Path>,
    runtime_probe: &IbkrRuntimeProbeDetails,
) -> ControlMatrixProviderStatus {
    let supported = [
        ControlMatrixDataRequirement::EtfReference,
        ControlMatrixDataRequirement::CfdReference,
        ControlMatrixDataRequirement::VixOverlay,
    ];
    let consent_path = home_dir.map(|home| home.join(IBKR_CONSENT_RELATIVE_PATH));
    let capabilities_path = home_dir.map(|home| home.join(IBKR_CAPABILITIES_RELATIVE_PATH));
    let consent_present = consent_path
        .as_ref()
        .map(|path| path.exists())
        .unwrap_or(false);
    let capabilities_present = capabilities_path
        .as_ref()
        .map(|path| path.exists())
        .unwrap_or(false);
    let reachable_candidates = runtime_probe
        .gateway_candidates
        .iter()
        .filter(|candidate| candidate.reachable)
        .collect::<Vec<_>>();
    let preferred_gateway_port = runtime_probe
        .gateway_candidates
        .iter()
        .find(|candidate| candidate.recommended)
        .map(|candidate| candidate.port);
    let healthy = consent_present && runtime_probe.ready && !reachable_candidates.is_empty();
    let status = if healthy {
        "ready"
    } else if !consent_present {
        "install_required"
    } else {
        "configured_runtime_unhealthy"
    }
    .to_string();
    let reason = if !consent_present {
        "missing_local_ibkr_consent".to_string()
    } else if !runtime_probe.missing_modules.is_empty() && !reachable_candidates.is_empty() {
        "ibkr_runtime_dependencies_missing_with_gateway_reachable".to_string()
    } else if !runtime_probe.missing_modules.is_empty() {
        "ibkr_runtime_dependencies_missing".to_string()
    } else if runtime_probe.ready && reachable_candidates.is_empty() {
        "ibkr_gateway_unreachable".to_string()
    } else if !runtime_probe.ready && !reachable_candidates.is_empty() {
        "ibkr_gateway_reachable_but_runtime_probe_failed".to_string()
    } else if runtime_probe.ready {
        "local_ibkr_runtime_ready".to_string()
    } else {
        "ibkr_runtime_probe_failed".to_string()
    };
    let mut install_prompts = Vec::new();
    if !consent_present {
        install_prompts.push("Consumer agent request: ask the user to install IBKR TWS or IB Gateway, then enable the IBKR API locally. Search keywords: Interactive Brokers TWS download, IB Gateway download.".to_string());
        install_prompts.push("Consumer agent follow-up: after the user installs IBKR TWS or Gateway, run python support/scripts/ibkr_bridge/setup.py --enable".to_string());
    }
    if !runtime_probe.missing_modules.is_empty() {
        install_prompts.push("Consumer agent follow-up: make sure the runtime that executes provider-status and provider fetches can import redis and ib_async. Low-pollution path: use uv run --with redis --with ib_async --with pandas for ad-hoc IBKR historical fetches.".to_string());
    }
    if reachable_candidates.is_empty() {
        install_prompts.push("Consumer agent request: ask the user to launch TWS or IB Gateway and enable the local API. Probe standard ports 7497, 7496, 4002, and 4001 before declaring IBKR unavailable.".to_string());
    } else if reachable_candidates.len() == 1 {
        install_prompts.push(format!(
            "Consumer agent follow-up: reuse the single reachable local IBKR API on port {} unless the user says otherwise.",
            preferred_gateway_port.unwrap_or_default()
        ));
    } else {
        install_prompts.push(format!(
            "Consumer agent request: multiple IBKR API ports are reachable; ask the user which runtime to use and pass --gateway-port {} or the chosen alternative explicitly.",
            preferred_gateway_port.unwrap_or_default()
        ));
    }
    ControlMatrixProviderStatus {
        provider: ControlMatrixProviderKind::Ibkr.as_str().to_string(),
        status,
        healthy,
        reason,
        supported_requirements: supported
            .into_iter()
            .filter(|item| required.contains(item))
            .map(|item| item.as_str().to_string())
            .collect(),
        install_prompts: if healthy { Vec::new() } else { install_prompts },
        redacted_config: vec![
            format!("consent_path={}", redact_path(consent_path.as_deref())),
            format!(
                "capabilities_path={}",
                redact_path(capabilities_path.as_deref())
            ),
            format!("consent_present={consent_present}"),
            format!("capabilities_present={capabilities_present}"),
            format!(
                "reachable_gateway_ports={}",
                format_ibkr_reachable_ports(&runtime_probe.gateway_candidates)
            ),
            format!(
                "ibkr_runtime_missing_modules={}",
                if runtime_probe.missing_modules.is_empty() {
                    "<none>".to_string()
                } else {
                    runtime_probe.missing_modules.join(",")
                }
            ),
        ],
    }
}

pub(crate) fn ibkr_runtime_probe_details() -> IbkrRuntimeProbeDetails {
    let scripts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts");
    let probe = format!(
        "import sys; sys.path.insert(0, {:?}); import redis; import ib_async; import ibkr_bridge",
        scripts_dir.display().to_string()
    );
    let output = std::process::Command::new("python3")
        .arg("-c")
        .arg(probe)
        .output()
        .ok();
    let stderr = output
        .as_ref()
        .map(|item| String::from_utf8_lossy(&item.stderr).trim().to_string())
        .filter(|item| !item.is_empty());
    let missing_modules = stderr
        .as_deref()
        .map(parse_missing_modules)
        .unwrap_or_default();
    IbkrRuntimeProbeDetails {
        ready: output.map(|item| item.status.success()).unwrap_or(false),
        missing_modules,
        stderr_excerpt: stderr,
        gateway_candidates: ibkr_gateway_candidates("127.0.0.1"),
    }
}

fn ibkr_gateway_candidates(host: &str) -> Vec<IbkrGatewayCandidate> {
    let recommended_port = IBKR_GATEWAY_PORT_CANDIDATES
        .iter()
        .map(|(_, port)| *port)
        .find(|port| ibkr_gateway_port_reachable(host, *port));
    IBKR_GATEWAY_PORT_CANDIDATES
        .into_iter()
        .map(|(label, port)| IbkrGatewayCandidate {
            label,
            port,
            reachable: ibkr_gateway_port_reachable(host, port),
            recommended: recommended_port == Some(port),
        })
        .collect()
}

fn ibkr_gateway_port_reachable(host: &str, port: u16) -> bool {
    let Ok(addr) = format!("{host}:{port}").parse::<SocketAddr>() else {
        return false;
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok()
}

fn parse_missing_modules(stderr: &str) -> Vec<String> {
    ["redis", "ib_async", "ibkr_bridge"]
        .into_iter()
        .filter(|module| stderr.contains(&format!("No module named '{}'", module)))
        .map(str::to_string)
        .collect()
}

fn format_ibkr_reachable_ports(candidates: &[IbkrGatewayCandidate]) -> String {
    let ports = candidates
        .iter()
        .filter(|candidate| candidate.reachable)
        .map(|candidate| format!("{}:{}", candidate.label, candidate.port))
        .collect::<Vec<_>>();
    if ports.is_empty() {
        "<none>".to_string()
    } else {
        ports.join("|")
    }
}

fn yfinance_provider_status<F>(
    required: &BTreeSet<ControlMatrixDataRequirement>,
    _env_lookup: &F,
) -> ControlMatrixProviderStatus
where
    F: Fn(&str) -> Option<String>,
{
    let supported = [
        ControlMatrixDataRequirement::EtfReference,
        ControlMatrixDataRequirement::CfdReference,
        ControlMatrixDataRequirement::VixOverlay,
        ControlMatrixDataRequirement::OptionsOpenInterest,
        ControlMatrixDataRequirement::OptionsImpliedVolatility,
    ];
    ControlMatrixProviderStatus {
        provider: ControlMatrixProviderKind::YahooFinance.as_str().to_string(),
        status: "ready".to_string(),
        healthy: true,
        reason: "public_yahoo_http_endpoints".to_string(),
        supported_requirements: supported
            .into_iter()
            .filter(|item| required.contains(item))
            .map(|item| item.as_str().to_string())
            .collect(),
        install_prompts: Vec::new(),
        redacted_config: vec!["provider_mode=public_http".to_string()],
    }
}

fn tradingview_mcp_provider_status<F, T>(
    required: &BTreeSet<ControlMatrixDataRequirement>,
    env_lookup: &F,
    home_dir: Option<&Path>,
    probe: &T,
) -> ControlMatrixProviderStatus
where
    F: Fn(&str) -> Option<String>,
    T: Fn(&BTreeSet<ControlMatrixDataRequirement>, &str, &str) -> TradingviewMcpProbeDetails,
{
    let supported = [
        ControlMatrixDataRequirement::EtfReference,
        ControlMatrixDataRequirement::CfdReference,
        ControlMatrixDataRequirement::VixOverlay,
        ControlMatrixDataRequirement::OptionsGreeks,
        ControlMatrixDataRequirement::OptionsImpliedVolatility,
    ];
    let config = tradingview_mcp_config_from_env_or_home(env_lookup, home_dir);
    let configured_url = config.url.clone();
    let has_api_key = config.api_key.is_some();
    let probe_details = config
        .api_key
        .as_deref()
        .map(|key| probe(required, &configured_url, key))
        .unwrap_or_else(|| TradingviewMcpProbeDetails {
            connectivity_ok: true,
            ohlcv_ok: None,
            options_ok: if required.iter().any(|item| {
                matches!(
                    item,
                    ControlMatrixDataRequirement::OptionsGreeks
                        | ControlMatrixDataRequirement::OptionsImpliedVolatility
                )
            }) {
                Some(false)
            } else {
                None
            },
        });
    let needs_ohlcv = required.iter().any(|item| {
        matches!(
            item,
            ControlMatrixDataRequirement::EtfReference
                | ControlMatrixDataRequirement::CfdReference
                | ControlMatrixDataRequirement::VixOverlay
        )
    });
    let needs_options = required.iter().any(|item| {
        matches!(
            item,
            ControlMatrixDataRequirement::OptionsGreeks
                | ControlMatrixDataRequirement::OptionsImpliedVolatility
        )
    });
    let (healthy, status, reason, install_prompts) = if !has_api_key
        && needs_options
        && !needs_ohlcv
    {
        (
            false,
            "ready_for_ohlcv_options_need_remote".to_string(),
            "local_stdio_options_tools_unverified".to_string(),
            vec![
                format!(
                    "Consumer agent optional remote mode for options lanes: set {}=<redacted> or save ~/.ict-engine/tvremix_mcp.json with fields api_key and optional url.",
                    TVREMIX_MCP_API_KEY_ENV
                ),
            ],
        )
    } else if !has_api_key && needs_options {
        (
            true,
            "ready_degraded".to_string(),
            "local_stdio_ohlcv_ready_options_unverified".to_string(),
            vec![
                format!(
                    "Consumer agent optional remote mode for options enrichment: set {}=<redacted>; OHLCV remains available through local stdio.",
                    TVREMIX_MCP_API_KEY_ENV
                ),
            ],
        )
    } else if !has_api_key {
        (
            true,
            "ready".to_string(),
            "local_stdio_ohlcv_available".to_string(),
            Vec::new(),
        )
    } else if !probe_details.connectivity_ok {
        (
            false,
            "configured_runtime_unhealthy".to_string(),
            "tradingview_mcp_connectivity_probe_failed".to_string(),
            vec![
                format!(
                    "Consumer agent request: TradingViewRemix MCP credentials were present but the live probe failed. Ask the user to re-enter {} and verify the MCP endpoint at {}.",
                    TVREMIX_MCP_API_KEY_ENV, configured_url
                ),
                "Consumer agent follow-up: retry a lightweight MCP health check such as tools/list before treating TradingViewRemix as usable.".to_string(),
            ],
        )
    } else if needs_ohlcv && probe_details.ohlcv_ok == Some(false) {
        (
            false,
            "configured_runtime_unhealthy".to_string(),
            "tradingview_mcp_ohlcv_probe_failed".to_string(),
            vec![
                "Consumer agent follow-up: TradingViewRemix MCP is reachable but the OHLCV tool path failed on the built-in smoke check. Retry later or fall back to yfinance / IBKR for OHLCV lanes.".to_string(),
            ],
        )
    } else if needs_options && probe_details.options_ok == Some(false) && !needs_ohlcv {
        (
            false,
            "configured_runtime_unhealthy".to_string(),
            "tradingview_mcp_options_probe_failed".to_string(),
            vec![
                "Consumer agent follow-up: TradingViewRemix MCP is reachable but the options-tool smoke check failed. Retry later or treat options_enriched lanes as temporarily degraded.".to_string(),
            ],
        )
    } else if needs_options && probe_details.options_ok == Some(false) {
        (
            true,
            "ready_degraded".to_string(),
            "tradingview_mcp_options_probe_degraded".to_string(),
            vec![
                "Consumer agent follow-up: TradingViewRemix MCP OHLCV is usable, but the options-tool smoke check failed. Treat options_enriched lanes as temporarily degraded and keep OHLCV lanes available.".to_string(),
            ],
        )
    } else {
        (
            true,
            "ready".to_string(),
            "mcp_url_and_api_key_available".to_string(),
            Vec::new(),
        )
    };
    ControlMatrixProviderStatus {
        provider: ControlMatrixProviderKind::TradingViewMcp
            .as_str()
            .to_string(),
        status,
        healthy,
        reason,
        supported_requirements: supported
            .into_iter()
            .filter(|item| required.contains(item))
            .filter(|item| {
                probe_details.options_ok != Some(false)
                    || !matches!(
                        item,
                        ControlMatrixDataRequirement::OptionsGreeks
                            | ControlMatrixDataRequirement::OptionsImpliedVolatility
                    )
            })
            .map(|item| item.as_str().to_string())
            .collect(),
        install_prompts,
        redacted_config: vec![
            format!("mcp_url={configured_url}"),
            format!("credential_source={}", config.credential_source),
            format!(
                "local_config={}",
                redact_secret_presence(config.local_config_present)
            ),
            format!(
                "{}={}",
                TVREMIX_MCP_API_KEY_ENV,
                redact_secret_presence(has_api_key)
            ),
            format!("stdio_cmd_env={}", env_presence(TRADINGVIEW_MCP_CMD_ENV)),
            format!("stdio_args_env={}", env_presence(TRADINGVIEW_MCP_ARGS_ENV)),
            format!("probe_connectivity={}", probe_details.connectivity_ok),
            format!(
                "probe_ohlcv={}",
                probe_details
                    .ohlcv_ok
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "<skipped>".to_string())
            ),
            format!(
                "probe_options={}",
                probe_details
                    .options_ok
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "<skipped>".to_string())
            ),
        ],
    }
}

pub(crate) fn tradingview_mcp_config_from_env_or_local() -> TradingviewMcpRuntimeConfig {
    tradingview_mcp_config_from_env_or_home(&|name| std::env::var(name).ok(), home_dir().as_deref())
}

fn tradingview_mcp_config_from_env_or_home<F>(
    env_lookup: &F,
    home_dir: Option<&Path>,
) -> TradingviewMcpRuntimeConfig
where
    F: Fn(&str) -> Option<String>,
{
    let local = home_dir
        .map(|home| home.join(TVREMIX_MCP_LOCAL_CONFIG_RELATIVE_PATH))
        .and_then(|path| load_local_tradingview_mcp_config(&path));
    let local_config_present = local.is_some();
    let env_url = env_lookup(TVREMIX_MCP_URL_ENV).filter(|value| !value.trim().is_empty());
    let env_key = env_lookup(TVREMIX_MCP_API_KEY_ENV).filter(|value| !value.trim().is_empty());
    let url = env_url
        .or_else(|| local.as_ref().and_then(|config| config.url.clone()))
        .unwrap_or_else(|| TVREMIX_MCP_DEFAULT_URL.to_string());
    let (api_key, credential_source) = if let Some(key) = env_key {
        (Some(key), "env")
    } else if let Some(key) = local.as_ref().and_then(|config| config.api_key.clone()) {
        (Some(key), "local_config")
    } else {
        (None, "missing")
    };
    TradingviewMcpRuntimeConfig {
        url,
        api_key,
        credential_source,
        local_config_present,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalTradingviewMcpConfig {
    api_key: Option<String>,
    url: Option<String>,
}

fn load_local_tradingview_mcp_config(path: &Path) -> Option<LocalTradingviewMcpConfig> {
    let raw = std::fs::read_to_string(path).ok()?;
    let payload: Value = serde_json::from_str(&raw).ok()?;
    Some(LocalTradingviewMcpConfig {
        api_key: first_non_empty_string(
            &payload,
            &["api_key", "key", "ICT_ENGINE_TVREMIX_MCP_API_KEY"],
        ),
        url: first_non_empty_string(&payload, &["url", "mcp_url", "ICT_ENGINE_TVREMIX_MCP_URL"]),
    })
}

fn first_non_empty_string(payload: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        payload
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn tradingview_mcp_probe_details(
    required: &BTreeSet<ControlMatrixDataRequirement>,
    url: &str,
    api_key: &str,
) -> TradingviewMcpProbeDetails {
    let connectivity_ok = tradingview_mcp_tools_list_ok(url, api_key);
    if !connectivity_ok {
        return TradingviewMcpProbeDetails {
            connectivity_ok: false,
            ..Default::default()
        };
    }
    let needs_ohlcv = required.iter().any(|item| {
        matches!(
            item,
            ControlMatrixDataRequirement::EtfReference
                | ControlMatrixDataRequirement::CfdReference
                | ControlMatrixDataRequirement::VixOverlay
        )
    });
    let needs_options = required.iter().any(|item| {
        matches!(
            item,
            ControlMatrixDataRequirement::OptionsGreeks
                | ControlMatrixDataRequirement::OptionsImpliedVolatility
        )
    });
    TradingviewMcpProbeDetails {
        connectivity_ok: true,
        ohlcv_ok: if needs_ohlcv {
            Some(tradingview_mcp_tool_success(
                url,
                api_key,
                "get_ohlcv",
                serde_json::json!({
                    "symbol": "NASDAQ:QQQ",
                    "interval": "1d",
                    "count": 10,
                    "summary": false
                }),
            ))
        } else {
            None
        },
        options_ok: if needs_options {
            Some(tradingview_mcp_tool_success(
                url,
                api_key,
                "get_option_expirations",
                serde_json::json!({
                    "symbol": "NASDAQ:QQQ",
                }),
            ))
        } else {
            None
        },
    }
}

fn tradingview_mcp_tools_list_ok(url: &str, api_key: &str) -> bool {
    let Ok(client) = Client::builder().timeout(Duration::from_secs(20)).build() else {
        return false;
    };
    let Ok(response) = client
        .post(url)
        .header(
            reqwest::header::ACCEPT,
            "application/json, text/event-stream",
        )
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        }))
        .send()
    else {
        return false;
    };
    let Ok(response) = response.error_for_status() else {
        return false;
    };
    let Ok(payload) = response.json::<Value>() else {
        return false;
    };
    payload
        .pointer("/result/tools")
        .and_then(Value::as_array)
        .map(|items| !items.is_empty())
        .unwrap_or(false)
}

fn tradingview_mcp_tool_success(url: &str, api_key: &str, name: &str, arguments: Value) -> bool {
    let Ok(client) = Client::builder().timeout(Duration::from_secs(20)).build() else {
        return false;
    };
    let Ok(response) = client
        .post(url)
        .header(
            reqwest::header::ACCEPT,
            "application/json, text/event-stream",
        )
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": name,
                "arguments": arguments,
            }
        }))
        .send()
    else {
        return false;
    };
    let Ok(response) = response.error_for_status() else {
        return false;
    };
    let Ok(payload) = response.json::<Value>() else {
        return false;
    };
    if payload
        .pointer("/result/isError")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return false;
    }
    if let Some(success) = payload
        .pointer("/result/structuredContent/success")
        .and_then(Value::as_bool)
    {
        return success;
    }
    true
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn redact_path(path: Option<&Path>) -> String {
    path.map(|_| "<local-path>".to_string())
        .unwrap_or_else(|| "unavailable".to_string())
}

fn redact_secret_presence(present: bool) -> &'static str {
    if present {
        "<set>"
    } else {
        "<unset>"
    }
}

fn env_presence(name: &str) -> &'static str {
    if std::env::var_os(name).is_some() {
        "<set>"
    } else {
        "<unset>"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_summary_requires_install_prompts_without_local_config() {
        let plan = ControlMatrixPlan::pb12();
        let summary = build_provider_summary_for_requirements_with_env(
            required_requirements_for_plan(&plan),
            &|_| None,
            Some(PathBuf::from("/tmp/does-not-exist")),
            &IbkrRuntimeProbeDetails::default,
            &|_, _, _| TradingviewMcpProbeDetails::default(),
        );
        assert!(summary
            .required_requirements
            .contains(&"etf_reference".to_string()));
        assert!(summary
            .required_requirements
            .contains(&"options_greeks".to_string()));
        assert!(summary
            .provider_statuses
            .iter()
            .any(|status| status.provider == "ibkr" && status.status == "install_required"));
        let tradingview = summary
            .provider_statuses
            .iter()
            .find(|status| status.provider == "tradingview_mcp")
            .unwrap();
        assert!(tradingview.healthy);
        assert_eq!(tradingview.status, "ready_degraded");
        assert_eq!(
            tradingview.reason,
            "local_stdio_ohlcv_ready_options_unverified"
        );
        assert!(summary
            .actionable_install_prompts
            .iter()
            .any(|prompt| prompt.contains("options enrichment")));
        assert!(summary
            .actionable_install_prompts
            .iter()
            .any(|prompt| prompt.contains("install IBKR TWS or IB Gateway")));
    }

    #[test]
    fn tradingview_provider_redacts_secret_value() {
        let plan = ControlMatrixPlan::pb12();
        let summary = build_provider_summary_for_requirements_with_env(
            required_requirements_for_plan(&plan),
            &|name| match name {
                TVREMIX_MCP_API_KEY_ENV => Some("secret-token-value".to_string()),
                _ => None,
            },
            None,
            &IbkrRuntimeProbeDetails::default,
            &|_, _, _| TradingviewMcpProbeDetails {
                connectivity_ok: true,
                ohlcv_ok: Some(true),
                options_ok: Some(true),
            },
        );
        let provider = summary
            .provider_statuses
            .iter()
            .find(|status| status.provider == "tradingview_mcp")
            .unwrap();
        assert_eq!(provider.status, "ready");
        assert!(provider
            .redacted_config
            .iter()
            .all(|item| !item.contains("secret-token-value")));
        assert!(provider
            .redacted_config
            .iter()
            .any(|item| item.contains("<set>")));
    }

    #[test]
    fn ibkr_requires_runtime_probe_even_with_consent_files() {
        let home = tempfile::tempdir().unwrap();
        let ict_engine_dir = home.path().join(".ict-engine");
        std::fs::create_dir_all(&ict_engine_dir).unwrap();
        std::fs::write(
            ict_engine_dir.join("ibkr_consent.json"),
            r#"{"enabled":true}"#,
        )
        .unwrap();
        std::fs::write(
            ict_engine_dir.join("ibkr_capabilities.json"),
            r#"{"historical_data":true}"#,
        )
        .unwrap();

        let required = BTreeSet::from([ControlMatrixDataRequirement::CfdReference]);
        let summary = build_provider_summary_for_requirements_with_env(
            required,
            &|_| None,
            Some(home.path().to_path_buf()),
            &IbkrRuntimeProbeDetails::default,
            &|_, _, _| TradingviewMcpProbeDetails::default(),
        );
        let provider = summary
            .provider_statuses
            .iter()
            .find(|status| status.provider == "ibkr")
            .unwrap();
        assert_eq!(provider.status, "configured_runtime_unhealthy");
        assert!(!provider.healthy);
        assert_eq!(provider.reason, "ibkr_runtime_probe_failed");
    }

    #[test]
    fn tradingview_provider_reports_ohlcv_probe_failure_after_connectivity() {
        let required = BTreeSet::from([ControlMatrixDataRequirement::EtfReference]);
        let summary = build_provider_summary_for_requirements_with_env(
            required,
            &|name| match name {
                TVREMIX_MCP_API_KEY_ENV => Some("secret-token-value".to_string()),
                _ => None,
            },
            None,
            &IbkrRuntimeProbeDetails::default,
            &|_, _, _| TradingviewMcpProbeDetails {
                connectivity_ok: true,
                ohlcv_ok: Some(false),
                options_ok: None,
            },
        );
        let provider = summary
            .provider_statuses
            .iter()
            .find(|status| status.provider == "tradingview_mcp")
            .unwrap();
        assert_eq!(provider.status, "configured_runtime_unhealthy");
        assert_eq!(provider.reason, "tradingview_mcp_ohlcv_probe_failed");
    }

    #[test]
    fn tradingview_provider_keeps_ohlcv_ready_when_options_probe_degrades() {
        let required = BTreeSet::from([
            ControlMatrixDataRequirement::EtfReference,
            ControlMatrixDataRequirement::OptionsGreeks,
            ControlMatrixDataRequirement::OptionsImpliedVolatility,
        ]);
        let summary = build_provider_summary_for_requirements_with_env(
            required,
            &|name| match name {
                TVREMIX_MCP_API_KEY_ENV => Some("secret-token-value".to_string()),
                _ => None,
            },
            None,
            &IbkrRuntimeProbeDetails::default,
            &|_, _, _| TradingviewMcpProbeDetails {
                connectivity_ok: true,
                ohlcv_ok: Some(true),
                options_ok: Some(false),
            },
        );
        let provider = summary
            .provider_statuses
            .iter()
            .find(|status| status.provider == "tradingview_mcp")
            .unwrap();
        assert!(provider.healthy);
        assert_eq!(provider.status, "ready_degraded");
        assert_eq!(provider.reason, "tradingview_mcp_options_probe_degraded");
        assert!(provider
            .supported_requirements
            .contains(&"etf_reference".to_string()));
        assert!(!provider
            .supported_requirements
            .contains(&"options_greeks".to_string()));
        assert!(provider
            .install_prompts
            .iter()
            .any(|prompt| prompt.contains("OHLCV is usable")));
    }

    #[test]
    fn tradingview_provider_uses_local_credential_file_when_env_is_absent() {
        let home = tempfile::tempdir().unwrap();
        let config_dir = home.path().join(".ict-engine");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("tvremix_mcp.json"),
            r#"{"api_key":"local-secret-token","url":"https://tvremix.example/mcp"}"#,
        )
        .unwrap();

        let required = BTreeSet::from([ControlMatrixDataRequirement::EtfReference]);
        let summary = build_provider_summary_for_requirements_with_env(
            required,
            &|_| None,
            Some(home.path().to_path_buf()),
            &IbkrRuntimeProbeDetails::default,
            &|_, url, key| {
                assert_eq!(url, "https://tvremix.example/mcp");
                assert_eq!(key, "local-secret-token");
                TradingviewMcpProbeDetails {
                    connectivity_ok: true,
                    ohlcv_ok: Some(true),
                    options_ok: None,
                }
            },
        );
        let provider = summary
            .provider_statuses
            .iter()
            .find(|status| status.provider == "tradingview_mcp")
            .unwrap();
        assert_eq!(provider.status, "ready");
        assert_eq!(provider.reason, "mcp_url_and_api_key_available");
        assert!(provider
            .redacted_config
            .iter()
            .all(|item| !item.contains("local-secret-token")));
        assert!(summary
            .actionable_install_prompts
            .iter()
            .all(|prompt| !prompt.contains("TradingViewRemix MCP API key")));
    }
}
