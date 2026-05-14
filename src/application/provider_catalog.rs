use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::application::data_sources::control_matrix_providers::ibkr_runtime_probe_details;
use crate::application::data_sources::{
    build_provider_summary_for_requirements, ControlMatrixDataRequirement,
    IBKR_CAPABILITIES_RELATIVE_PATH, IBKR_CONSENT_RELATIVE_PATH,
};
use crate::application::entry_models::{entry_model_providers, ConsumerDefaultMode};
use crate::config::shell_quote;

const PROVIDER_STATUS_AGENT_COMMAND: &str = "ict-engine provider-status --agent";
const EXTERNAL_HTTP_DEFAULT_URL: &str = "http://127.0.0.1:6901/api/v1";
const PROVIDER_PROFILE_SCHEMA_VERSION: &str = "provider-profile/v1";
const REPO_PROVIDER_PROFILE_DIR: &str = "support/examples/provider_profiles";
const KRAKEN_API_KEY_ENV: &str = "KRAKEN_API_KEY";
const KRAKEN_API_SECRET_ENV: &str = "KRAKEN_API_SECRET";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderCatalogDomain {
    MarketData,
    LiveRuntime,
    LocalRuntime,
    EntryModel,
}

impl ProviderCatalogDomain {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MarketData => "market_data",
            Self::LiveRuntime => "live_runtime",
            Self::LocalRuntime => "local_runtime",
            Self::EntryModel => "entry_model",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "market_data" | "market-data" => Ok(Self::MarketData),
            "live_runtime" | "live-runtime" => Ok(Self::LiveRuntime),
            "local_runtime" | "local-runtime" | "local_bridge" | "local-bridge" => {
                Ok(Self::LocalRuntime)
            }
            "entry_model" | "entry-model" => Ok(Self::EntryModel),
            other => bail!(
                "unsupported provider-status domain '{}'; available: market_data, live_runtime, local_runtime, entry_model",
                other
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderCatalogItem {
    pub provider_id: String,
    pub domain: String,
    pub selectable_by_user: bool,
    pub adopted_by_default: bool,
    pub access_mode: String,
    pub user_access: String,
    pub market_fit: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_priority: Option<u8>,
    pub user_summary: String,
    pub ready: bool,
    pub status: String,
    pub reason: String,
    pub capabilities: Vec<String>,
    pub notes: Vec<String>,
    pub install_prompts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ProviderCatalogDomainSummary {
    pub domain: String,
    pub total: usize,
    pub ready: usize,
    pub selectable: usize,
    pub default_enabled: usize,
    pub provider_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ProviderCatalogSurface {
    pub providers: Vec<ProviderCatalogItem>,
    pub domains: Vec<ProviderCatalogDomainSummary>,
    pub summary_line: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub available_opt_in_profiles: Vec<ProviderProfileReferenceSurface>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_profile: Option<ProviderProfileSelectionSurface>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ProviderCatalogAgentSurface {
    pub summary_line: String,
    pub ready_by_domain: BTreeMap<String, String>,
    pub providers: Vec<ProviderCatalogAgentItem>,
    pub ready_providers: Vec<String>,
    pub pending_providers: Vec<String>,
    pub pending_provider_details: Vec<ProviderCatalogPendingAgentItem>,
    pub selectable_providers: Vec<String>,
    pub default_enabled_providers: Vec<String>,
    pub install_prompts: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub available_opt_in_profiles: Vec<ProviderProfileReferenceSurface>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_profile: Option<ProviderProfileAgentSelectionSurface>,
    #[serde(skip)]
    pub selected_profile_full: Option<ProviderProfileSelectionSurface>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ProviderCatalogAgentItem {
    pub provider_id: String,
    pub domain: String,
    pub selectable_by_user: bool,
    pub adopted_by_default: bool,
    pub ready: bool,
    pub access_mode: String,
    pub user_access: String,
    pub market_fit: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_priority: Option<u8>,
    pub status: String,
    pub reason: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub install_prompts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ProviderCatalogPendingAgentItem {
    pub provider_id: String,
    pub domain: String,
    pub status: String,
    pub reason: String,
    pub install_prompts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ProviderProfileReferenceSurface {
    pub profile_id: String,
    pub display_name: String,
    pub selector: String,
    pub opt_in_only: bool,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct WorkflowProviderSupportSurface {
    pub active: bool,
    pub profile_id: String,
    pub support_reason: String,
    pub provider_status_command: String,
    pub summary_line: String,
    pub pending_providers: Vec<String>,
    pub pending_provider_details: Vec<ProviderCatalogPendingAgentItem>,
    pub install_prompts: Vec<String>,
    pub ask_user_prompts: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_profile: Option<ProviderProfileAgentSelectionSurface>,
    #[serde(skip)]
    pub selected_profile_full: Option<ProviderProfileSelectionSurface>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderProfileDocument {
    pub schema_version: String,
    pub profile_id: String,
    pub display_name: String,
    pub opt_in_only: bool,
    pub summary: String,
    #[serde(default)]
    pub data_contracts: Vec<ProviderProfileDataContract>,
    #[serde(default)]
    pub provider_tracks: Vec<ProviderProfileTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderProfileDataContract {
    pub contract_id: String,
    pub category: String,
    pub required: bool,
    pub label: String,
    #[serde(default)]
    pub symbols: Vec<String>,
    #[serde(default)]
    pub timeframes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderProfileTrackMode {
    AnyOf,
    AllOf,
}

impl ProviderProfileTrackMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::AnyOf => "any_of",
            Self::AllOf => "all_of",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderProfileTrack {
    pub track_id: String,
    pub label: String,
    pub required: bool,
    pub mode: ProviderProfileTrackMode,
    #[serde(default)]
    pub provider_ids: Vec<String>,
    #[serde(default)]
    pub activation_hints: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ProviderProfileTrackSelection {
    pub track_id: String,
    pub label: String,
    pub required: bool,
    pub mode: String,
    pub activation_hints: Vec<String>,
    pub status: String,
    pub ready_provider_ids: Vec<String>,
    pub pending_provider_ids: Vec<String>,
    pub install_prompts: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ProviderProfileSelectionSurface {
    pub profile_id: String,
    pub display_name: String,
    pub opt_in_only: bool,
    pub source: String,
    pub selector: String,
    pub summary: String,
    pub data_contracts: Vec<ProviderProfileDataContract>,
    pub data_contract_labels: Vec<String>,
    pub track_details: Vec<ProviderProfileTrackSelection>,
    pub track_statuses: Vec<String>,
    pub ready_provider_ids: Vec<String>,
    pub pending_provider_ids: Vec<String>,
    pub install_prompts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ProviderProfileAgentSelectionSurface {
    pub profile_id: String,
    pub display_name: String,
    pub opt_in_only: bool,
    pub source_kind: String,
    pub selector: String,
    pub summary: String,
    pub data_contract_labels: Vec<String>,
    pub track_statuses: Vec<String>,
    pub ready_provider_ids: Vec<String>,
    pub pending_provider_ids: Vec<String>,
    pub install_prompts: Vec<String>,
}

pub trait ProviderCatalogSource {
    fn domain(&self) -> ProviderCatalogDomain;
    fn collect_items(&self) -> Result<Vec<ProviderCatalogItem>>;
}

pub fn provider_catalog_sources() -> Vec<Box<dyn ProviderCatalogSource>> {
    vec![
        Box::new(MarketDataProviderCatalogSource),
        Box::new(LiveRuntimeProviderCatalogSource),
        Box::new(LocalRuntimeProviderCatalogSource),
        Box::new(EntryModelProviderCatalogSource),
    ]
}

pub fn provider_status_surface(
    domain_filter: Option<&str>,
    provider_filter: Option<&str>,
    profile_selector: Option<&str>,
) -> Result<ProviderCatalogSurface> {
    let parsed_domain = domain_filter
        .map(ProviderCatalogDomain::parse)
        .transpose()?;
    let mut providers = Vec::new();
    for source in provider_catalog_sources() {
        if parsed_domain
            .map(|domain| domain == source.domain())
            .unwrap_or(true)
        {
            providers.extend(source.collect_items()?);
        }
    }
    let available_opt_in_profiles = list_repo_example_profiles()?;
    let selected_profile = if let Some(selector) = profile_selector {
        let (profile, source_path) = load_provider_profile_with_source(selector)?;
        let source = provider_profile_source_kind(&source_path);
        let command_selector = provider_profile_command_selector(&source_path);
        Some(build_selected_profile_surface_from_items(
            &providers,
            &profile,
            &source,
            &command_selector,
        )?)
    } else {
        None
    };
    if let Some(filter) = provider_filter {
        providers.retain(|item| item.provider_id == filter);
        if providers.is_empty() {
            bail!("unknown provider id '{}'", filter);
        }
    }
    providers.sort_by(|a, b| {
        a.domain
            .cmp(&b.domain)
            .then(a.provider_id.cmp(&b.provider_id))
    });

    let mut grouped = BTreeMap::<String, Vec<&ProviderCatalogItem>>::new();
    for item in &providers {
        grouped.entry(item.domain.clone()).or_default().push(item);
    }
    let domains = grouped
        .into_iter()
        .map(|(domain, items)| ProviderCatalogDomainSummary {
            domain,
            total: items.len(),
            ready: items.iter().filter(|item| item.ready).count(),
            selectable: items.iter().filter(|item| item.selectable_by_user).count(),
            default_enabled: items.iter().filter(|item| item.adopted_by_default).count(),
            provider_ids: items
                .iter()
                .map(|item| item.provider_id.clone())
                .collect::<Vec<_>>(),
        })
        .collect::<Vec<_>>();

    let summary_line = domains
        .iter()
        .map(|domain| format!("{}:{}/{} ready", domain.domain, domain.ready, domain.total))
        .collect::<Vec<_>>()
        .join(" | ");

    Ok(ProviderCatalogSurface {
        providers,
        domains,
        summary_line,
        available_opt_in_profiles,
        selected_profile,
    })
}

pub fn provider_status_command(
    domain_filter: Option<&str>,
    provider_filter: Option<&str>,
    compact: bool,
    agent: bool,
    jsonl: bool,
    profile_selector: Option<&str>,
) -> Result<()> {
    let surface = provider_status_surface(domain_filter, provider_filter, profile_selector)?;
    if agent {
        println!(
            "{}",
            serde_json::to_string_pretty(&build_provider_catalog_agent_surface(&surface))?
        );
    } else if jsonl {
        print!("{}", render_provider_catalog_jsonl(&surface)?);
    } else if compact {
        println!("{}", render_provider_catalog_compact(&surface));
    } else {
        println!("{}", serde_json::to_string_pretty(&surface)?);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Default)]
struct MarketDataProviderCatalogSource;

impl ProviderCatalogSource for MarketDataProviderCatalogSource {
    fn domain(&self) -> ProviderCatalogDomain {
        ProviderCatalogDomain::MarketData
    }

    fn collect_items(&self) -> Result<Vec<ProviderCatalogItem>> {
        let summary = build_provider_summary_for_requirements(all_market_data_requirements());
        let mut items = summary
            .provider_statuses
            .into_iter()
            .map(|status| ProviderCatalogItem {
                provider_id: status.provider,
                domain: self.domain().as_str().to_string(),
                selectable_by_user: true,
                adopted_by_default: false,
                access_mode: market_data_access_mode(&status.status, &status.reason),
                user_access: String::new(),
                market_fit: Vec::new(),
                fallback_priority: None,
                user_summary: String::new(),
                ready: status.healthy,
                status: status.status,
                reason: status.reason,
                capabilities: status.supported_requirements,
                notes: Vec::new(),
                install_prompts: status.install_prompts,
            })
            .collect::<Vec<_>>();

        let public_fetch_runtime = probe_public_fetch_python_runtime();
        items.extend([
            public_provider_item(
                "binance_public",
                &public_fetch_runtime,
                vec!["ohlcv".to_string(), "options_chain".to_string()],
                vec!["public_rest".to_string(), "no_api_key_required".to_string()],
            ),
            public_provider_item(
                "bybit_public",
                &public_fetch_runtime,
                vec!["ohlcv".to_string(), "options_chain".to_string()],
                vec!["public_rest".to_string(), "no_api_key_required".to_string()],
            ),
            public_provider_item(
                "kraken_public",
                &public_fetch_runtime,
                vec![
                    "ohlcv".to_string(),
                    "spot".to_string(),
                    "futures".to_string(),
                    "tokenized_equity".to_string(),
                ],
                vec!["public_rest".to_string(), "no_api_key_required".to_string()],
            ),
            public_provider_item(
                "polymarket_public",
                &public_fetch_runtime,
                vec!["prediction_market_history".to_string()],
                vec![
                    "public_rest".to_string(),
                    "network_path_dependent".to_string(),
                ],
            ),
        ]);
        for item in &mut items {
            apply_provider_user_semantics(item);
        }
        Ok(items)
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct LiveRuntimeProviderCatalogSource;

impl ProviderCatalogSource for LiveRuntimeProviderCatalogSource {
    fn domain(&self) -> ProviderCatalogDomain {
        ProviderCatalogDomain::LiveRuntime
    }

    fn collect_items(&self) -> Result<Vec<ProviderCatalogItem>> {
        Ok(vec![
            ProviderCatalogItem {
                provider_id: "yfinance".to_string(),
                domain: self.domain().as_str().to_string(),
                selectable_by_user: true,
                adopted_by_default: true,
                access_mode: "local_library".to_string(),
                user_access: "free_no_login".to_string(),
                market_fit: vec!["tradfi".to_string()],
                fallback_priority: Some(1),
                user_summary:
                    "Zero-config live runtime backed by yfinance-compatible fetches for observation and replay-adjacent workflows."
                        .to_string(),
                ready: true,
                status: "ready".to_string(),
                reason: "native_yfinance_runtime_available".to_string(),
                capabilities: vec![
                    "futures_candles".to_string(),
                    "spot_candles".to_string(),
                    "options_summary".to_string(),
                ],
                notes: vec!["zero_config_first_run_fallback".to_string()],
                install_prompts: Vec::new(),
            },
            ProviderCatalogItem {
                provider_id: "external_http_runtime".to_string(),
                domain: self.domain().as_str().to_string(),
                selectable_by_user: true,
                adopted_by_default: false,
                access_mode: "external_http_runtime".to_string(),
                user_access: "operator_runtime_optional".to_string(),
                market_fit: vec!["tradfi".to_string(), "crypto".to_string()],
                fallback_priority: Some(20),
                user_summary:
                    "Optional external HTTP runtime when the user already has a compatible market-data service."
                        .to_string(),
                ready: false,
                status: "operator_runtime_required".to_string(),
                reason: "base_url_and_service_required".to_string(),
                capabilities: vec![
                    "futures_candles".to_string(),
                    "spot_candles".to_string(),
                    "options_summary".to_string(),
                ],
                notes: vec!["usable when operator supplies running service".to_string()],
                install_prompts: vec![
                    "Consumer agent request: ask whether the user wants zero-config yfinance or a generic external HTTP runtime."
                        .to_string(),
                    format!(
                        "Consumer agent follow-up: if the user chooses the external HTTP runtime, keep the external_http_runtime backend and pass --external-http-base-url <url> (default {}).",
                        EXTERNAL_HTTP_DEFAULT_URL
                    ),
                ],
            },
            ProviderCatalogItem {
                provider_id: "crypto_public_runtime".to_string(),
                domain: self.domain().as_str().to_string(),
                selectable_by_user: true,
                adopted_by_default: false,
                access_mode: "public_runtime_bundle".to_string(),
                user_access: "operator_runtime_optional".to_string(),
                market_fit: vec!["crypto".to_string()],
                fallback_priority: Some(21),
                user_summary:
                    "Optional crypto-public runtime bundle for CoinAnk/Hyperliquid-style futures observation when the user explicitly wants that lane."
                        .to_string(),
                ready: false,
                status: "operator_runtime_required".to_string(),
                reason: "explicit_opt_in_required".to_string(),
                capabilities: vec![
                    "futures_candles".to_string(),
                ],
                notes: vec!["public-crypto helper lane, not a tradfi default".to_string()],
                install_prompts: vec![
                    "Consumer agent request: ask whether the user wants zero-config yfinance or the optional crypto_public_runtime lane."
                        .to_string(),
                ],
            },
        ])
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct LocalRuntimeProviderCatalogSource;

impl ProviderCatalogSource for LocalRuntimeProviderCatalogSource {
    fn domain(&self) -> ProviderCatalogDomain {
        ProviderCatalogDomain::LocalRuntime
    }

    fn collect_items(&self) -> Result<Vec<ProviderCatalogItem>> {
        let ibkr_probe = probe_ibkr_bridge();
        let kraken_probe = probe_kraken_cli();
        Ok(vec![
            ProviderCatalogItem {
                provider_id: "ibkr_bridge".to_string(),
                domain: self.domain().as_str().to_string(),
                selectable_by_user: false,
                adopted_by_default: false,
                access_mode: "local_consent_runtime".to_string(),
                user_access: "login_and_local_runtime".to_string(),
                market_fit: vec!["tradfi".to_string()],
                fallback_priority: Some(40),
                user_summary:
                    "Local IBKR bridge reused by broker-linked workflows after the user enables the local API."
                        .to_string(),
                ready: ibkr_probe.ready,
                status: ibkr_probe.status,
                reason: ibkr_probe.reason,
                capabilities: vec![
                    "local_ibkr_historical".to_string(),
                    "local_ibkr_stream".to_string(),
                ],
                notes: ibkr_probe.notes,
                install_prompts: ibkr_probe.install_prompts,
            },
            ProviderCatalogItem {
                provider_id: "kraken_cli".to_string(),
                domain: self.domain().as_str().to_string(),
                selectable_by_user: false,
                adopted_by_default: false,
                access_mode: "local_cli_runtime".to_string(),
                user_access: "login_and_local_runtime".to_string(),
                market_fit: vec!["crypto".to_string()],
                fallback_priority: Some(40),
                user_summary:
                    "Credentialed local Kraken CLI/runtime path for wallet or execution-adjacent flows."
                        .to_string(),
                ready: kraken_probe.ready,
                status: kraken_probe.status,
                reason: kraken_probe.reason,
                capabilities: vec![
                    "local_cli_execution".to_string(),
                    "operator_wallet_flow".to_string(),
                ],
                notes: kraken_probe.notes,
                install_prompts: kraken_probe.install_prompts,
            },
        ])
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct EntryModelProviderCatalogSource;

impl ProviderCatalogSource for EntryModelProviderCatalogSource {
    fn domain(&self) -> ProviderCatalogDomain {
        ProviderCatalogDomain::EntryModel
    }

    fn collect_items(&self) -> Result<Vec<ProviderCatalogItem>> {
        Ok(entry_model_providers()
            .into_iter()
            .map(|provider| ProviderCatalogItem {
                provider_id: provider.provider_id().to_string(),
                domain: self.domain().as_str().to_string(),
                selectable_by_user: !matches!(
                    provider.consumer_default_mode(),
                    ConsumerDefaultMode::InternalTrainingOnly
                ),
                adopted_by_default: provider.consumer_default_mode().adopted_by_default(),
                access_mode: "internal_model_registry".to_string(),
                user_access: "builtin_registry".to_string(),
                market_fit: vec!["entry_model".to_string()],
                fallback_priority: None,
                user_summary:
                    "Built-in entry-model registry member used by status and training surfaces."
                        .to_string(),
                ready: true,
                status: "registered".to_string(),
                reason: "entry_model_registry_member".to_string(),
                capabilities: vec!["training_rows".to_string(), "status_surface".to_string()],
                notes: vec![provider.consumer_default_mode().effect_label().to_string()],
                install_prompts: Vec::new(),
            })
            .collect())
    }
}

fn all_market_data_requirements() -> std::collections::BTreeSet<ControlMatrixDataRequirement> {
    [
        ControlMatrixDataRequirement::EtfReference,
        ControlMatrixDataRequirement::CfdReference,
        ControlMatrixDataRequirement::VixOverlay,
        ControlMatrixDataRequirement::OptionsGreeks,
        ControlMatrixDataRequirement::OptionsOpenInterest,
        ControlMatrixDataRequirement::OptionsImpliedVolatility,
    ]
    .into_iter()
    .collect()
}

fn market_data_access_mode(status: &str, reason: &str) -> String {
    if status.starts_with("ready") && reason.contains("consent") {
        "local_consent_runtime".to_string()
    } else if status.starts_with("ready") {
        "public_or_env_ready".to_string()
    } else if reason.contains("api_key") {
        "api_key_required".to_string()
    } else {
        "operator_runtime_required".to_string()
    }
}

fn public_provider_item(
    provider_id: &str,
    runtime: &PublicFetchPythonRuntimeProbe,
    capabilities: Vec<String>,
    notes: Vec<String>,
) -> ProviderCatalogItem {
    let ready = runtime.ready;
    ProviderCatalogItem {
        provider_id: provider_id.to_string(),
        domain: ProviderCatalogDomain::MarketData.as_str().to_string(),
        selectable_by_user: true,
        adopted_by_default: false,
        access_mode: "public_script_adapter".to_string(),
        user_access: "public_no_login".to_string(),
        market_fit: match provider_id {
            "polymarket_public" => vec!["prediction_market".to_string()],
            "kraken_public" => vec![
                "crypto".to_string(),
                "fx".to_string(),
                "tokenized_assets".to_string(),
            ],
            _ => vec!["crypto".to_string()],
        },
        fallback_priority: match provider_id {
            "bybit_public" => Some(1),
            "binance_public" => Some(2),
            "kraken_public" => Some(3),
            "polymarket_public" => Some(4),
            _ => None,
        },
        user_summary: match provider_id {
            "bybit_public" => {
                "Public no-login crypto market-data path for exchange-style replay and factor work."
                    .to_string()
            }
            "binance_public" => {
                "Public no-login crypto market-data path for broad spot/perp history."
                    .to_string()
            }
            "kraken_public" => {
                "Public no-login crypto, forex, and tokenized-asset data path; later wallet/runtime flows use kraken_cli separately."
                    .to_string()
            }
            "polymarket_public" => {
                "Public no-login prediction-market history path when network access is available."
                    .to_string()
            }
            _ => "Public no-login market-data path.".to_string(),
        },
        ready,
        status: runtime.status.clone(),
        reason: runtime.reason.clone(),
        capabilities,
        notes,
        install_prompts: runtime.install_prompts.clone(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PublicFetchPythonRuntimeProbe {
    ready: bool,
    status: String,
    reason: String,
    install_prompts: Vec<String>,
}

fn probe_public_fetch_python_runtime() -> PublicFetchPythonRuntimeProbe {
    let script_present = provider_fetch_script_exists();
    let python_present = python3_exists();
    let missing_modules = if python_present {
        missing_public_fetch_python_modules()
    } else {
        Vec::new()
    };
    let ready = script_present && python_present && missing_modules.is_empty();
    let (status, reason) = if ready {
        ("ready", "fetch_external_script_available")
    } else if !script_present {
        ("install_required", "fetch_external_script_missing")
    } else if !python_present {
        ("install_required", "python3_missing")
    } else {
        (
            "configured_runtime_unhealthy",
            "python3_provider_dependencies_missing",
        )
    };
    let install_prompts = if ready {
        Vec::new()
    } else if !missing_modules.is_empty() {
        vec![format!(
            "System python3 is missing provider script modules: {}. Install with: python3 -m pip install --user --break-system-packages {}",
            missing_modules.join(", "),
            pip_packages_for_missing_public_fetch_modules(&missing_modules).join(" ")
        )]
    } else if !python_present {
        vec!["Install python3 before using public provider fetch scripts.".to_string()]
    } else {
        vec!["Restore support/scripts/auto_quant_external/fetch_external.py before using public provider fetch scripts.".to_string()]
    };
    PublicFetchPythonRuntimeProbe {
        ready,
        status: status.to_string(),
        reason: reason.to_string(),
        install_prompts,
    }
}

fn provider_fetch_script_exists() -> bool {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("support/scripts/auto_quant_external/fetch_external.py")
        .exists()
}

fn python3_exists() -> bool {
    command_exists(&["python3"])
}

fn public_fetch_python_modules() -> &'static [&'static str] {
    &[
        "requests", "pandas", "ccxt", "ib_async", "redis", "yaml", "sklearn", "pyarrow",
    ]
}

fn missing_public_fetch_python_modules() -> Vec<String> {
    let imports = public_fetch_python_modules()
        .iter()
        .map(|module| format!("import {module}"))
        .collect::<Vec<_>>()
        .join("\n");
    let Ok(output) = std::process::Command::new("python3")
        .args(["-c", &imports])
        .output()
    else {
        return public_fetch_python_modules()
            .iter()
            .map(|module| (*module).to_string())
            .collect();
    };
    if output.status.success() {
        return Vec::new();
    }
    public_fetch_python_modules()
        .iter()
        .filter(|module| !python3_can_import(module))
        .map(|module| (*module).to_string())
        .collect()
}

fn python3_can_import(module: &str) -> bool {
    std::process::Command::new("python3")
        .args(["-c", &format!("import {module}")])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn pip_packages_for_missing_public_fetch_modules(missing_modules: &[String]) -> Vec<String> {
    missing_modules
        .iter()
        .map(|module| match module.as_str() {
            "yaml" => "PyYAML".to_string(),
            "sklearn" => "scikit-learn".to_string(),
            other => other.to_string(),
        })
        .collect()
}

#[derive(Debug, Clone)]
struct LocalRuntimeProbe {
    ready: bool,
    status: String,
    reason: String,
    notes: Vec<String>,
    install_prompts: Vec<String>,
}

fn probe_ibkr_bridge() -> LocalRuntimeProbe {
    let script_present = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("support/scripts/ibkr_bridge/__init__.py")
        .exists();
    let home = home_dir();
    let consent_present = home
        .as_ref()
        .map(|root| root.join(IBKR_CONSENT_RELATIVE_PATH).exists())
        .unwrap_or(false);
    let capabilities_present = home
        .as_ref()
        .map(|root| root.join(IBKR_CAPABILITIES_RELATIVE_PATH).exists())
        .unwrap_or(false);
    let runtime_probe = ibkr_runtime_probe_details();
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
    let (ready, status, reason) = if !script_present || !python3_exists() {
        (
            false,
            "install_required".to_string(),
            "ibkr_bridge_not_installed".to_string(),
        )
    } else if !consent_present && !capabilities_present {
        (
            false,
            "installed_unconfigured".to_string(),
            "ibkr_bridge_installed_but_consent_missing".to_string(),
        )
    } else if !runtime_probe.ready
        && !runtime_probe.missing_modules.is_empty()
        && !reachable_candidates.is_empty()
    {
        (
            false,
            "configured_runtime_unhealthy".to_string(),
            "ibkr_bridge_runtime_dependencies_missing_with_gateway_reachable".to_string(),
        )
    } else if !runtime_probe.ready {
        (
            false,
            "configured_runtime_unhealthy".to_string(),
            "ibkr_bridge_config_present_but_runtime_probe_failed".to_string(),
        )
    } else if reachable_candidates.is_empty() {
        (
            false,
            "configured_runtime_unhealthy".to_string(),
            "ibkr_bridge_gateway_unreachable".to_string(),
        )
    } else {
        (
            true,
            "ready".to_string(),
            "local_ibkr_bridge_ready".to_string(),
        )
    };
    let mut install_prompts = Vec::new();
    if !ready {
        if !consent_present && !capabilities_present {
            install_prompts.push(
                "Install and enable the local IBKR bridge if you want IBKR-backed workflows."
                    .to_string(),
            );
        }
        if !runtime_probe.missing_modules.is_empty() {
            install_prompts.push(
                "Make sure the runtime that executes provider-status and provider fetches can import redis and ib_async."
                    .to_string(),
            );
        }
        if reachable_candidates.is_empty() {
            install_prompts.push(
                "Launch TWS or IB Gateway and enable the local API; probe 7497, 7496, 4002, and 4001 before treating IBKR as unavailable."
                    .to_string(),
            );
        } else if reachable_candidates.len() > 1 {
            install_prompts.push(format!(
                "Multiple local IBKR API ports are reachable; ask the user which runtime to use and prefer --gateway-port {} or the chosen alternative explicitly.",
                preferred_gateway_port.unwrap_or_default()
            ));
        } else {
            install_prompts.push(format!(
                "A local IBKR API is reachable on port {}; reuse it unless the user says otherwise.",
                preferred_gateway_port.unwrap_or_default()
            ));
        }
    }
    LocalRuntimeProbe {
        ready,
        status,
        reason,
        notes: vec![
            "reused by ibkr market-data provider".to_string(),
            format!("consent_present={}", consent_present),
            format!("capabilities_present={}", capabilities_present),
            format!(
                "reachable_gateway_ports={}",
                if reachable_candidates.is_empty() {
                    "<none>".to_string()
                } else {
                    reachable_candidates
                        .iter()
                        .map(|candidate| format!("{}:{}", candidate.label, candidate.port))
                        .collect::<Vec<_>>()
                        .join("|")
                }
            ),
            format!(
                "runtime_missing_modules={}",
                if runtime_probe.missing_modules.is_empty() {
                    "<none>".to_string()
                } else {
                    runtime_probe.missing_modules.join(",")
                }
            ),
        ],
        install_prompts,
    }
}

fn probe_kraken_cli() -> LocalRuntimeProbe {
    let binary_on_path = command_exists(&["kraken", "kraken-cli"]);
    let local_binary = find_kraken_cli_local_binary();
    let installed = binary_on_path || local_binary.is_some();
    let configured = kraken_cli_config_present();
    let (ready, status, reason) = if !installed {
        (
            false,
            "install_required".to_string(),
            "kraken_cli_not_found_on_path".to_string(),
        )
    } else if !binary_on_path {
        (
            false,
            "installed_off_path".to_string(),
            "kraken_cli_binary_found_off_path".to_string(),
        )
    } else if !configured {
        (
            false,
            "installed_unconfigured".to_string(),
            "kraken_cli_installed_but_config_missing".to_string(),
        )
    } else {
        (
            true,
            "ready".to_string(),
            "kraken_cli_config_detected".to_string(),
        )
    };
    LocalRuntimeProbe {
        ready,
        status,
        reason,
        notes: {
            let mut notes =
                vec!["see support/docs/external/kraken-cli-agent-patterns.md".to_string()];
            if let Some(path) = local_binary {
                notes.push(format!("local_binary={}", path.display()));
            }
            notes
        },
        install_prompts: if ready {
            Vec::new()
        } else {
            vec![
                "Consumer agent request: ask the user to install kraken-cli from https://github.com/krakenfx/kraken-cli if Kraken workflows are needed.".to_string(),
                "Consumer agent request: ask the user to create or retrieve Kraken API credentials before authenticated kraken-cli use.".to_string(),
                format!(
                    "Consumer agent follow-up: configure kraken-cli with {} / {} or ~/.config/kraken/config.toml once the user approves.",
                    KRAKEN_API_KEY_ENV, KRAKEN_API_SECRET_ENV
                ),
            ]
        },
    }
}

fn kraken_cli_config_present() -> bool {
    let env_config = [KRAKEN_API_KEY_ENV, KRAKEN_API_SECRET_ENV]
        .into_iter()
        .all(|name| env::var_os(name).is_some());
    if env_config {
        return true;
    }
    if kraken_cli_auth_present_from_cli() {
        return true;
    }
    let Some(home) = home_dir() else {
        return false;
    };
    [
        ".config/kraken-cli",
        ".config/kraken",
        ".kraken-cli",
        ".kraken",
    ]
    .into_iter()
    .map(|rel| home.join(rel))
    .any(|path| path.exists())
}

fn kraken_cli_auth_present_from_cli() -> bool {
    let Ok(output) = std::process::Command::new("kraken")
        .args(["auth", "show", "-o", "json"])
        .output()
    else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return false;
    };
    value
        .get("api_key")
        .and_then(|api_key| api_key.get("present"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn find_kraken_cli_local_binary() -> Option<PathBuf> {
    let home = home_dir()?;
    let candidates = [
        home.join(".cargo/bin/kraken"),
        home.join(".cargo/bin/kraken-cli"),
        home.join("kraken-cli/target/debug/kraken"),
        home.join("kraken-cli/target/release/kraken"),
        home.join("kraken-cli/target/debug/kraken-cli"),
        home.join("kraken-cli/target/release/kraken-cli"),
    ];
    candidates.into_iter().find(|path| path.exists())
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

fn command_exists(names: &[&str]) -> bool {
    let Some(path_os) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&path_os).any(|dir| {
        names
            .iter()
            .map(|name| dir.join(name))
            .any(|candidate| candidate.exists())
    })
}

fn apply_provider_user_semantics(item: &mut ProviderCatalogItem) {
    match item.provider_id.as_str() {
        "yfinance" => {
            item.user_access = "free_no_login".to_string();
            item.market_fit = vec!["tradfi".to_string()];
            item.fallback_priority = Some(1);
            item.user_summary =
                "Free historical tradfi fallback for replay, factor research, and factor backtests."
                    .to_string();
            item.notes
                .push("zero_config_first_run_fallback".to_string());
        }
        "external_http_runtime" => {
            item.user_access = "operator_runtime_optional".to_string();
            item.market_fit = vec!["tradfi".to_string(), "crypto".to_string()];
            item.fallback_priority = Some(20);
            item.user_summary =
                "Optional generic external HTTP runtime when the operator already has a compatible service."
                    .to_string();
        }
        "crypto_public_runtime" => {
            item.user_access = "operator_runtime_optional".to_string();
            item.market_fit = vec!["crypto".to_string()];
            item.fallback_priority = Some(21);
            item.user_summary =
                "Optional crypto-public runtime bundle for explicit live crypto observation."
                    .to_string();
        }
        "ibkr" => {
            item.user_access = "login_and_local_runtime".to_string();
            item.market_fit = vec!["tradfi".to_string()];
            item.fallback_priority = Some(30);
            item.user_summary =
                "Setup-required IBKR market-data path for broker-linked futures and equities workflows."
                    .to_string();
        }
        "tradingview_mcp" => {
            item.user_access = "local_stdio_or_remote_api_key".to_string();
            item.market_fit = vec!["tradfi".to_string(), "crypto".to_string()];
            item.fallback_priority = Some(31);
            item.user_summary =
                "Hot-pluggable TradingView MCP path: zero-config local stdio for OHLCV, optional remote key for enriched lanes."
                    .to_string();
            item.notes.push("zero_config_stdio_ohlcv".to_string());
        }
        _ => {}
    }
    if item.user_access.is_empty() {
        item.user_access = match item.domain.as_str() {
            "market_data" => "operator_guided".to_string(),
            "live_runtime" => "operator_runtime_optional".to_string(),
            "local_runtime" => "local_runtime".to_string(),
            _ => "builtin".to_string(),
        };
    }
    if item.market_fit.is_empty() {
        item.market_fit = match item.domain.as_str() {
            "market_data" | "live_runtime" => vec!["tradfi".to_string(), "crypto".to_string()],
            "local_runtime" => vec!["tradfi".to_string()],
            _ => vec!["entry_model".to_string()],
        };
    }
    if item.user_summary.is_empty() {
        item.user_summary = format!(
            "{} provider available through {}.",
            item.provider_id, item.access_mode
        );
    }
}

fn compact_provider_guide_line(providers: &[ProviderCatalogItem]) -> Option<String> {
    if providers.is_empty() {
        return None;
    }
    if providers.len() <= 4 {
        return Some(
            providers
                .iter()
                .map(|provider| {
                    let fit = if provider.market_fit.is_empty() {
                        "general".to_string()
                    } else {
                        provider.market_fit.join("/")
                    };
                    let priority = provider
                        .fallback_priority
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "n/a".to_string());
                    format!(
                        "{}: access={} fit={} fallback={} summary={}",
                        provider.provider_id,
                        provider.user_access,
                        fit,
                        priority,
                        provider.user_summary
                    )
                })
                .collect::<Vec<_>>()
                .join(" | "),
        );
    }
    let tradfi = providers
        .iter()
        .filter(|provider| {
            provider.ready
                && provider.user_access == "free_no_login"
                && provider.market_fit.iter().any(|fit| fit == "tradfi")
        })
        .min_by_key(|provider| provider.fallback_priority.unwrap_or(u8::MAX))
        .map(|provider| provider.provider_id.clone())
        .unwrap_or_else(|| "none".to_string());
    let live_zero_config = providers
        .iter()
        .filter(|provider| {
            provider.ready && provider.domain == "live_runtime" && provider.adopted_by_default
        })
        .map(|provider| provider.provider_id.clone())
        .collect::<Vec<_>>();
    let crypto = providers
        .iter()
        .filter(|provider| {
            provider.ready
                && provider.user_access == "public_no_login"
                && provider.market_fit.iter().any(|fit| fit == "crypto")
        })
        .map(|provider| provider.provider_id.clone())
        .collect::<Vec<_>>();
    let setup_required = providers
        .iter()
        .filter(|provider| {
            !provider.ready
                && matches!(
                    provider.user_access.as_str(),
                    "login_and_local_runtime" | "api_key_required" | "operator_runtime_optional"
                )
        })
        .map(|provider| provider.provider_id.clone())
        .collect::<Vec<_>>();
    let live_zero_config = if live_zero_config.is_empty() {
        "none".to_string()
    } else {
        live_zero_config.join(", ")
    };
    let crypto = if crypto.is_empty() {
        "none".to_string()
    } else {
        crypto.join(", ")
    };
    let setup_required = if setup_required.is_empty() {
        "none".to_string()
    } else {
        setup_required.join(", ")
    };
    Some(format!(
        "tradfi free fallback={} | live zero-config={} | crypto public={} | setup required={}",
        tradfi, live_zero_config, crypto, setup_required
    ))
}

fn render_provider_catalog_compact(surface: &ProviderCatalogSurface) -> String {
    let mut lines = Vec::new();
    let pending_provider_ids = surface
        .providers
        .iter()
        .filter(|provider| !provider.ready && provider.selectable_by_user)
        .map(|provider| provider.provider_id.clone())
        .collect::<Vec<_>>();
    lines.push(surface.summary_line.clone());
    if let Some(profile) = surface.selected_profile.as_ref() {
        lines.push(format!(
            "profile: {} pending {}",
            profile.profile_id,
            profile.pending_provider_ids.join(", ")
        ));
        lines.push(format!("  summary: {}", profile.summary));
        if !profile.data_contract_labels.is_empty() {
            lines.push(format!(
                "  data_contracts: {}",
                profile
                    .data_contract_labels
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" | ")
            ));
        }
        if !profile.track_statuses.is_empty() {
            lines.push(format!(
                "  tracks: {}",
                profile
                    .track_statuses
                    .iter()
                    .take(4)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" | ")
            ));
        }
    }
    if let Some(guide) = compact_provider_guide_line(&surface.providers) {
        lines.push(format!("guide: {}", guide));
    }
    if surface.providers.len() > 1 && !pending_provider_ids.is_empty() {
        lines.push(
            "details: use ict-engine provider-status --provider <id> --compact for provider-specific setup prompts".to_string(),
        );
    }
    if surface.providers.len() == 1 {
        let provider = &surface.providers[0];
        lines.push(format!(
            "detail: {} | access={} | {}",
            provider.provider_id, provider.user_access, provider.user_summary
        ));
        if !provider.install_prompts.is_empty() {
            lines.push(format!(
                "setup: {}",
                provider
                    .install_prompts
                    .iter()
                    .take(2)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" | ")
            ));
        }
    }
    for domain in &surface.domains {
        lines.push(format!(
            "{}: ready {}/{} selectable {} default {}",
            domain.domain, domain.ready, domain.total, domain.selectable, domain.default_enabled
        ));
        let ready = surface
            .providers
            .iter()
            .filter(|provider| provider.domain == domain.domain && provider.ready)
            .map(|provider| provider.provider_id.clone())
            .collect::<Vec<_>>();
        let pending = surface
            .providers
            .iter()
            .filter(|provider| provider.domain == domain.domain && !provider.ready)
            .map(|provider| {
                format!(
                    "{}({}:{})",
                    provider.provider_id, provider.status, provider.reason
                )
            })
            .collect::<Vec<_>>();
        if !ready.is_empty() {
            lines.push(format!("  ready: {}", ready.join(", ")));
        }
        if !pending.is_empty() {
            lines.push(format!("  pending: {}", pending.join(", ")));
        }
    }
    lines.join("\n")
}

fn build_provider_catalog_agent_surface(
    surface: &ProviderCatalogSurface,
) -> ProviderCatalogAgentSurface {
    let ready_by_domain = surface
        .domains
        .iter()
        .map(|domain| {
            (
                domain.domain.clone(),
                format!("{}/{}", domain.ready, domain.total),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let providers = surface
        .providers
        .iter()
        .map(|provider| ProviderCatalogAgentItem {
            provider_id: provider.provider_id.clone(),
            domain: provider.domain.clone(),
            selectable_by_user: provider.selectable_by_user,
            adopted_by_default: provider.adopted_by_default,
            ready: provider.ready,
            access_mode: provider.access_mode.clone(),
            user_access: provider.user_access.clone(),
            market_fit: provider.market_fit.clone(),
            fallback_priority: provider.fallback_priority,
            status: provider.status.clone(),
            reason: provider.reason.clone(),
            summary: provider.user_summary.clone(),
            install_prompts: provider.install_prompts.clone(),
        })
        .collect::<Vec<_>>();
    let ready_providers = surface
        .providers
        .iter()
        .filter(|provider| provider.ready)
        .map(|provider| provider.provider_id.clone())
        .collect::<Vec<_>>();
    let pending_providers = surface
        .providers
        .iter()
        .filter(|provider| !provider.ready)
        .map(|provider| {
            format!(
                "{}@{}:{}:{}",
                provider.provider_id, provider.domain, provider.status, provider.reason
            )
        })
        .collect::<Vec<_>>();
    let pending_provider_details = surface
        .providers
        .iter()
        .filter(|provider| !provider.ready)
        .map(|provider| ProviderCatalogPendingAgentItem {
            provider_id: provider.provider_id.clone(),
            domain: provider.domain.clone(),
            status: provider.status.clone(),
            reason: provider.reason.clone(),
            install_prompts: provider.install_prompts.clone(),
        })
        .collect::<Vec<_>>();
    let selectable_providers = surface
        .providers
        .iter()
        .filter(|provider| provider.selectable_by_user)
        .map(|provider| provider.provider_id.clone())
        .collect::<Vec<_>>();
    let default_enabled_providers = surface
        .providers
        .iter()
        .filter(|provider| provider.adopted_by_default)
        .map(|provider| provider.provider_id.clone())
        .collect::<Vec<_>>();
    let install_prompts = surface
        .providers
        .iter()
        .flat_map(|provider| provider.install_prompts.iter().cloned())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    ProviderCatalogAgentSurface {
        summary_line: surface.summary_line.clone(),
        ready_by_domain,
        providers,
        ready_providers,
        pending_providers,
        pending_provider_details,
        selectable_providers,
        default_enabled_providers,
        install_prompts,
        available_opt_in_profiles: if surface.selected_profile.is_some() {
            surface.available_opt_in_profiles.clone()
        } else {
            Vec::new()
        },
        selected_profile: surface
            .selected_profile
            .as_ref()
            .map(build_agent_selected_profile_surface),
        selected_profile_full: surface.selected_profile.clone(),
    }
}

fn build_agent_selected_profile_surface(
    profile: &ProviderProfileSelectionSurface,
) -> ProviderProfileAgentSelectionSurface {
    let source_kind =
        if profile.source.starts_with("http://") || profile.source.starts_with("https://") {
            "remote".to_string()
        } else if profile.source == "repo-example" {
            "repo-example".to_string()
        } else {
            "local_path".to_string()
        };
    ProviderProfileAgentSelectionSurface {
        profile_id: profile.profile_id.clone(),
        display_name: profile.display_name.clone(),
        opt_in_only: profile.opt_in_only,
        source_kind,
        selector: profile.selector.clone(),
        summary: profile.summary.clone(),
        data_contract_labels: profile.data_contract_labels.clone(),
        track_statuses: profile.track_statuses.clone(),
        ready_provider_ids: profile.ready_provider_ids.clone(),
        pending_provider_ids: profile.pending_provider_ids.clone(),
        install_prompts: profile.install_prompts.clone(),
    }
}

pub fn provider_status_agent_surface(
    domain_filter: Option<&str>,
    provider_filter: Option<&str>,
    profile_selector: Option<&str>,
) -> Result<ProviderCatalogAgentSurface> {
    let surface = provider_status_surface(domain_filter, provider_filter, profile_selector)?;
    Ok(build_provider_catalog_agent_surface(&surface))
}

pub fn build_workflow_provider_support(
    surface: &ProviderCatalogAgentSurface,
    next_command: &str,
    blocking_reason: Option<&str>,
) -> WorkflowProviderSupportSurface {
    let selected_profile = surface.selected_profile.clone();
    let relevant_provider_ids = workflow_relevant_provider_ids(next_command, blocking_reason);
    let mut support = WorkflowProviderSupportSurface {
        profile_id: selected_profile
            .as_ref()
            .map(|profile| profile.profile_id.clone())
            .unwrap_or_else(|| "workflow_auto".to_string()),
        support_reason: blocking_reason.unwrap_or_default().to_string(),
        provider_status_command: provider_status_agent_command_for_surface(surface),
        summary_line: surface.summary_line.clone(),
        selected_profile,
        selected_profile_full: surface.selected_profile_full.clone(),
        ..WorkflowProviderSupportSurface::default()
    };
    if relevant_provider_ids.is_empty() {
        return support;
    }

    let mut pending_provider_details = surface
        .pending_provider_details
        .iter()
        .filter(|provider| relevant_provider_ids.contains(provider.provider_id.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    pending_provider_details.sort_by(|a, b| a.provider_id.cmp(&b.provider_id));
    if pending_provider_details.is_empty() {
        return support;
    }

    let pending_providers = pending_provider_details
        .iter()
        .map(|provider| provider.provider_id.clone())
        .collect::<Vec<_>>();
    let install_prompts = pending_provider_details
        .iter()
        .flat_map(|provider| provider.install_prompts.iter().cloned())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let ask_user_prompts = pending_provider_details
        .iter()
        .flat_map(provider_ask_user_prompts)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    support.active = true;
    support.pending_providers = pending_providers;
    support.pending_provider_details = pending_provider_details;
    support.install_prompts = install_prompts;
    support.ask_user_prompts = ask_user_prompts;
    support
}

pub fn provider_status_agent_command_for_surface(surface: &ProviderCatalogAgentSurface) -> String {
    provider_status_agent_command(surface.selected_profile.as_ref())
}

pub fn provider_status_agent_command(
    selected_profile: Option<&ProviderProfileAgentSelectionSurface>,
) -> String {
    if let Some(profile) = selected_profile {
        return format!(
            "{} --profile {}",
            PROVIDER_STATUS_AGENT_COMMAND,
            shell_quote(&profile.selector)
        );
    }
    PROVIDER_STATUS_AGENT_COMMAND.to_string()
}

fn workflow_relevant_provider_ids(
    next_command: &str,
    blocking_reason: Option<&str>,
) -> std::collections::BTreeSet<&'static str> {
    let haystack = format!(
        "{} {}",
        next_command.to_ascii_lowercase(),
        blocking_reason.unwrap_or_default().to_ascii_lowercase()
    );
    let mut ids = std::collections::BTreeSet::new();

    if haystack.contains("--futures-backend external_http_runtime")
        || haystack.contains("--aux-backend external_http_runtime")
        || haystack.contains("--futures-backend external_http_runtime")
        || haystack.contains("--aux-backend external_http_runtime")
        || haystack.contains("external_http_base_url")
        || haystack.contains("external_http_runtime_base_url")
    {
        ids.insert("external_http_runtime");
    }
    if haystack.contains("--futures-backend crypto_public_runtime")
        || haystack.contains("--aux-backend crypto_public_runtime")
        || haystack.contains("--futures-backend crypto_public_runtime")
        || haystack.contains("--aux-backend crypto_public_runtime")
        || haystack.contains("crypto_public_base_url")
        || haystack.contains("crypto_public_runtime_base_url")
    {
        ids.insert("crypto_public_runtime");
    }
    if haystack.contains("tradingview") {
        ids.insert("tradingview_mcp");
    }
    if haystack.contains("ibkr") || haystack.contains("gateway") || haystack.contains("tws") {
        ids.insert("ibkr");
        ids.insert("ibkr_bridge");
    }
    if haystack.contains("kraken") {
        ids.insert("kraken_cli");
    }
    if ids.is_empty()
        && haystack.contains("provider_runtime_required")
        && haystack.contains("analyze-live")
    {
        ids.insert("external_http_runtime");
        ids.insert("crypto_public_runtime");
    }

    ids
}

fn provider_ask_user_prompts(provider: &ProviderCatalogPendingAgentItem) -> Vec<String> {
    match provider.provider_id.as_str() {
        "tradingview_mcp" => vec![
            "Use local stdio TradingView MCP for OHLCV by default; ask for ICT_ENGINE_TVREMIX_MCP_API_KEY only when the selected lane explicitly needs remote/options enrichment.".to_string(),
        ],
        "kraken_cli" => vec![
            format!(
                "Ask the user for Kraken credentials for this run and set {} plus {} before retrying Kraken-authenticated workflows.",
                KRAKEN_API_KEY_ENV, KRAKEN_API_SECRET_ENV
            ),
        ],
        _ => Vec::new(),
    }
}

fn render_provider_catalog_jsonl(surface: &ProviderCatalogSurface) -> Result<String> {
    let selected_profile = surface
        .selected_profile
        .as_ref()
        .map(build_agent_selected_profile_surface);
    let mut lines = Vec::new();
    lines.push(serde_json::to_string(&serde_json::json!({
        "type": "summary",
        "summary_line": surface.summary_line,
        "domains": surface.domains,
        "available_opt_in_profiles": surface.available_opt_in_profiles,
        "selected_profile": selected_profile,
    }))?);
    for provider in &surface.providers {
        lines.push(serde_json::to_string(&serde_json::json!({
            "type": "provider",
            "provider_id": provider.provider_id,
            "domain": provider.domain,
            "selectable_by_user": provider.selectable_by_user,
            "adopted_by_default": provider.adopted_by_default,
            "access_mode": provider.access_mode,
            "user_access": provider.user_access,
            "market_fit": provider.market_fit,
            "fallback_priority": provider.fallback_priority,
            "user_summary": provider.user_summary,
            "ready": provider.ready,
            "status": provider.status,
            "reason": provider.reason,
            "capabilities": provider.capabilities,
            "notes": provider.notes,
            "install_prompts": provider.install_prompts,
        }))?);
    }
    Ok(lines.join("\n"))
}

pub fn load_provider_profile(selector: &str) -> Result<ProviderProfileDocument> {
    load_provider_profile_with_source(selector).map(|(profile, _)| profile)
}

fn load_provider_profile_with_source(selector: &str) -> Result<(ProviderProfileDocument, PathBuf)> {
    let path = resolve_provider_profile_path(selector)?;
    let raw = fs::read_to_string(&path)?;
    let profile: ProviderProfileDocument = serde_json::from_str(&raw)?;
    if profile.schema_version != PROVIDER_PROFILE_SCHEMA_VERSION {
        bail!(
            "unsupported provider profile schema_version '{}'; expected '{}'",
            profile.schema_version,
            PROVIDER_PROFILE_SCHEMA_VERSION
        );
    }
    if profile.profile_id.trim().is_empty() {
        bail!("provider profile id must not be empty");
    }
    Ok((profile, path))
}

fn provider_profile_command_selector(path: &Path) -> String {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(REPO_PROVIDER_PROFILE_DIR);
    if path.starts_with(&repo_root) {
        return path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(ToString::to_string)
            .unwrap_or_else(|| path.to_string_lossy().to_string());
    }
    path.to_string_lossy().to_string()
}

fn provider_profile_source_kind(path: &Path) -> String {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(REPO_PROVIDER_PROFILE_DIR);
    if path.starts_with(&repo_root) {
        return "repo-example".to_string();
    }
    let raw = path.to_string_lossy();
    if raw.starts_with("http://") || raw.starts_with("https://") {
        "remote".to_string()
    } else {
        "local_path".to_string()
    }
}

fn resolve_provider_profile_path(selector: &str) -> Result<PathBuf> {
    let trimmed = selector.trim();
    if trimmed.is_empty() {
        bail!("provider profile selector must not be empty");
    }
    let direct = PathBuf::from(trimmed);
    if direct.exists() {
        return Ok(direct);
    }
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(REPO_PROVIDER_PROFILE_DIR);
    let repo_exact = repo_root.join(trimmed);
    if repo_exact.exists() {
        return Ok(repo_exact);
    }
    let repo_json = repo_root.join(format!("{trimmed}.json"));
    if repo_json.exists() {
        return Ok(repo_json);
    }
    bail!(
        "unknown provider profile '{}'; pass a JSON path or a repo example id from {}",
        trimmed,
        REPO_PROVIDER_PROFILE_DIR
    )
}

fn list_repo_example_profiles() -> Result<Vec<ProviderProfileReferenceSurface>> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(REPO_PROVIDER_PROFILE_DIR);
    if !repo_root.exists() {
        return Ok(Vec::new());
    }

    let mut profiles = Vec::new();
    for entry in fs::read_dir(&repo_root)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let raw = fs::read_to_string(&path)?;
        let profile: ProviderProfileDocument = serde_json::from_str(&raw)?;
        if profile.schema_version != PROVIDER_PROFILE_SCHEMA_VERSION
            || !profile.opt_in_only
            || profile.profile_id.trim().is_empty()
        {
            continue;
        }
        let selector = provider_profile_command_selector(&path);
        profiles.push(ProviderProfileReferenceSurface {
            profile_id: profile.profile_id,
            display_name: profile.display_name,
            selector,
            opt_in_only: profile.opt_in_only,
            summary: profile.summary,
        });
    }
    profiles.sort_by(|a, b| a.selector.cmp(&b.selector));
    Ok(profiles)
}

fn build_selected_profile_surface_from_items(
    items: &[ProviderCatalogItem],
    profile: &ProviderProfileDocument,
    source: &str,
    selector: &str,
) -> Result<ProviderProfileSelectionSurface> {
    let item_map = items
        .iter()
        .map(|item| (item.provider_id.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let mut ready_provider_ids = Vec::new();
    let mut pending_provider_ids = Vec::new();
    let mut install_prompts = std::collections::BTreeSet::new();
    let mut track_details = Vec::new();
    let mut track_statuses = Vec::new();

    for track in &profile.provider_tracks {
        let mut ready = Vec::new();
        let mut pending = Vec::new();
        let mut track_prompts = std::collections::BTreeSet::new();
        for provider_id in &track.provider_ids {
            match item_map.get(provider_id.as_str()) {
                Some(item) if item.ready => ready.push(provider_id.clone()),
                Some(item) => {
                    pending.push(provider_id.clone());
                    for prompt in &item.install_prompts {
                        track_prompts.insert(prompt.clone());
                    }
                }
                None => pending.push(provider_id.clone()),
            }
        }
        let status = match track.mode {
            ProviderProfileTrackMode::AnyOf => {
                if ready.is_empty() {
                    "pending"
                } else {
                    "ready"
                }
            }
            ProviderProfileTrackMode::AllOf => {
                if pending.is_empty() {
                    "ready"
                } else if ready.is_empty() {
                    "pending"
                } else {
                    "partial"
                }
            }
        };
        ready_provider_ids.extend(ready.iter().cloned());
        pending_provider_ids.extend(pending.iter().cloned());
        install_prompts.extend(track_prompts.iter().cloned());
        let status_target = if !pending.is_empty() {
            pending.join(",")
        } else if !ready.is_empty() {
            ready.join(",")
        } else {
            "none".to_string()
        };
        track_statuses.push(format!("{}:{}:{}", track.track_id, status, status_target));
        track_details.push(ProviderProfileTrackSelection {
            track_id: track.track_id.clone(),
            label: track.label.clone(),
            required: track.required,
            mode: track.mode.as_str().to_string(),
            activation_hints: track.activation_hints.clone(),
            status: status.to_string(),
            ready_provider_ids: ready,
            pending_provider_ids: pending,
            install_prompts: track_prompts.into_iter().collect(),
            notes: track.notes.clone(),
        });
    }

    ready_provider_ids.sort();
    ready_provider_ids.dedup();
    pending_provider_ids.sort();
    pending_provider_ids.dedup();
    track_statuses.sort();

    Ok(ProviderProfileSelectionSurface {
        profile_id: profile.profile_id.clone(),
        display_name: profile.display_name.clone(),
        opt_in_only: profile.opt_in_only,
        source: source.to_string(),
        selector: selector.to_string(),
        summary: profile.summary.clone(),
        data_contracts: profile.data_contracts.clone(),
        data_contract_labels: profile
            .data_contracts
            .iter()
            .map(|contract| contract.label.clone())
            .collect(),
        track_details,
        track_statuses,
        ready_provider_ids,
        pending_provider_ids,
        install_prompts: install_prompts.into_iter().collect(),
    })
}

#[cfg(test)]
fn build_selected_profile_surface(
    surface: &ProviderCatalogSurface,
    profile: &ProviderProfileDocument,
    source: &str,
    selector: &str,
) -> Result<ProviderProfileSelectionSurface> {
    build_selected_profile_surface_from_items(&surface.providers, profile, source, selector)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_surface() -> ProviderCatalogSurface {
        ProviderCatalogSurface {
            providers: vec![
                ProviderCatalogItem {
                    provider_id: "yfinance".to_string(),
                    domain: "market_data".to_string(),
                    selectable_by_user: true,
                    adopted_by_default: false,
                    access_mode: "public".to_string(),
                    user_access: "free_no_login".to_string(),
                    market_fit: vec!["tradfi".to_string()],
                    fallback_priority: Some(1),
                    user_summary: "Free historical tradfi fallback.".to_string(),
                    ready: true,
                    status: "ready".to_string(),
                    reason: "ok".to_string(),
                    capabilities: vec!["ohlcv".to_string()],
                    notes: vec![],
                    install_prompts: vec![],
                },
                ProviderCatalogItem {
                    provider_id: "ibkr".to_string(),
                    domain: "market_data".to_string(),
                    selectable_by_user: true,
                    adopted_by_default: false,
                    access_mode: "local_consent_runtime".to_string(),
                    user_access: "login_and_local_runtime".to_string(),
                    market_fit: vec!["tradfi".to_string()],
                    fallback_priority: Some(30),
                    user_summary: "Setup-required IBKR path.".to_string(),
                    ready: false,
                    status: "install_required".to_string(),
                    reason: "missing_runtime".to_string(),
                    capabilities: vec!["ohlcv".to_string()],
                    notes: vec![],
                    install_prompts: vec!["install ibkr".to_string()],
                },
            ],
            domains: vec![ProviderCatalogDomainSummary {
                domain: "market_data".to_string(),
                total: 2,
                ready: 1,
                selectable: 2,
                default_enabled: 0,
                provider_ids: vec!["yfinance".to_string(), "ibkr".to_string()],
            }],
            summary_line: "market_data:1/2 ready".to_string(),
            available_opt_in_profiles: vec![ProviderProfileReferenceSurface {
                profile_id: "thrill3r_nq_closed_loop_v1".to_string(),
                display_name: "Thrill3r NQ Closed Loop v1".to_string(),
                selector: "thrill3r-nq-closed-loop-v1".to_string(),
                opt_in_only: true,
                summary: "Personal NQ workflow".to_string(),
            }],
            selected_profile: None,
        }
    }

    #[test]
    fn agent_surface_keeps_low_token_summary() {
        let agent = build_provider_catalog_agent_surface(&sample_surface());
        assert_eq!(agent.summary_line, "market_data:1/2 ready");
        assert_eq!(
            agent.ready_by_domain.get("market_data").map(String::as_str),
            Some("1/2")
        );
        assert!(agent.ready_providers.contains(&"yfinance".to_string()));
        assert!(agent
            .pending_providers
            .iter()
            .any(|item| item.contains("ibkr@market_data")));
        assert!(agent.available_opt_in_profiles.is_empty());
        assert_eq!(agent.providers[0].provider_id, "yfinance");
        assert_eq!(agent.providers[0].user_access, "free_no_login");
        assert_eq!(
            agent.providers[0].summary,
            "Free historical tradfi fallback."
        );
    }

    #[test]
    fn agent_surface_selected_profile_omits_heavy_contract_and_track_details() {
        let mut surface = sample_surface();
        let profile = load_provider_profile("thrill3r-nq-closed-loop-v1").unwrap();
        surface.selected_profile = Some(
            build_selected_profile_surface(
                &surface,
                &profile,
                "repo-example",
                "thrill3r-nq-closed-loop-v1",
            )
            .unwrap(),
        );

        let agent = build_provider_catalog_agent_surface(&surface);
        let value = serde_json::to_value(&agent).unwrap();
        let selected = &value["selected_profile"];

        assert_eq!(selected["profile_id"], "thrill3r_nq_closed_loop_v1");
        assert_eq!(selected["source_kind"], "repo-example");
        assert!(selected["data_contract_labels"].is_array());
        assert!(selected["track_statuses"].is_array());
        assert!(selected.get("data_contracts").is_none());
        assert!(selected.get("track_details").is_none());
        assert!(selected.get("source").is_none());
    }

    #[test]
    fn compact_surface_summarizes_selected_profile_contracts_and_tracks() {
        let mut surface = sample_surface();
        let profile = load_provider_profile("thrill3r-nq-closed-loop-v1").unwrap();
        surface.selected_profile = Some(
            build_selected_profile_surface(
                &surface,
                &profile,
                "repo-example",
                "thrill3r-nq-closed-loop-v1",
            )
            .unwrap(),
        );

        let compact = render_provider_catalog_compact(&surface);

        assert!(compact.contains("profile: thrill3r_nq_closed_loop_v1"));
        assert!(compact.contains("summary: Personal NQ workflow"));
        assert!(compact.contains("data_contracts:"));
        assert!(compact.contains("Tomac cleaned multi-timeframe futures root"));
        assert!(compact.contains("tracks:"));
        assert!(compact.contains("live_zero_config:ready:yfinance"));
    }

    #[test]
    fn compact_surface_hides_available_opt_in_profiles_by_default() {
        let compact = render_provider_catalog_compact(&sample_surface());
        assert!(!compact.contains("opt_in_profiles:"));
        assert!(compact.contains("guide:"));
        assert!(compact.contains("yfinance: access=free_no_login"));
        assert!(
            compact.contains("details: use ict-engine provider-status --provider <id> --compact")
        );
    }

    #[test]
    fn compact_surface_shows_single_provider_setup_prompts() {
        let surface = ProviderCatalogSurface {
            providers: vec![ProviderCatalogItem {
                provider_id: "ibkr".to_string(),
                domain: "market_data".to_string(),
                selectable_by_user: true,
                adopted_by_default: false,
                access_mode: "operator_runtime_required".to_string(),
                user_access: "login_and_local_runtime".to_string(),
                market_fit: vec!["tradfi".to_string()],
                fallback_priority: Some(30),
                user_summary: "Setup-required IBKR market-data path.".to_string(),
                ready: false,
                status: "install_required".to_string(),
                reason: "missing_local_ibkr_consent".to_string(),
                capabilities: vec!["etf_reference".to_string()],
                notes: Vec::new(),
                install_prompts: vec![
                    "install IBKR TWS or Gateway".to_string(),
                    "enable the local API".to_string(),
                ],
            }],
            domains: vec![ProviderCatalogDomainSummary {
                domain: "market_data".to_string(),
                total: 1,
                ready: 0,
                selectable: 1,
                default_enabled: 0,
                provider_ids: vec!["ibkr".to_string()],
            }],
            summary_line: "market_data:0/1 ready".to_string(),
            available_opt_in_profiles: Vec::new(),
            selected_profile: None,
        };

        let compact = render_provider_catalog_compact(&surface);
        assert!(compact.contains("detail: ibkr | access=login_and_local_runtime"));
        assert!(compact.contains("setup: install IBKR TWS or Gateway | enable the local API"));
    }

    #[test]
    fn jsonl_surface_starts_with_summary_record() {
        let jsonl = render_provider_catalog_jsonl(&sample_surface()).unwrap();
        let mut lines = jsonl.lines();
        let first = lines.next().unwrap_or("");
        assert!(first.contains("\"type\":\"summary\""));
        let second = lines.next().unwrap_or("");
        assert!(second.contains("\"type\":\"provider\""));
    }

    #[test]
    fn jsonl_summary_uses_lightweight_selected_profile_shape() {
        let mut surface = sample_surface();
        let profile = load_provider_profile("thrill3r-nq-closed-loop-v1").unwrap();
        surface.selected_profile = Some(
            build_selected_profile_surface(
                &surface,
                &profile,
                "repo-example",
                "thrill3r-nq-closed-loop-v1",
            )
            .unwrap(),
        );

        let jsonl = render_provider_catalog_jsonl(&surface).unwrap();
        let first = jsonl.lines().next().unwrap_or("");
        let value: serde_json::Value = serde_json::from_str(first).unwrap();
        assert_eq!(
            value["available_opt_in_profiles"][0]["selector"],
            "thrill3r-nq-closed-loop-v1"
        );
        let selected = &value["selected_profile"];

        assert_eq!(selected["profile_id"], "thrill3r_nq_closed_loop_v1");
        assert!(selected["data_contract_labels"].is_array());
        assert!(selected["track_statuses"].is_array());
        assert!(selected.get("data_contracts").is_none());
        assert!(selected.get("track_details").is_none());
    }

    #[test]
    fn workflow_provider_support_filters_to_relevant_pending_runtime_providers() {
        let support = build_workflow_provider_support(
            &ProviderCatalogAgentSurface {
                summary_line: "live_runtime:1/3 ready | local_runtime:0/2 ready".to_string(),
                ready_by_domain: BTreeMap::from([
                    ("live_runtime".to_string(), "1/3".to_string()),
                    ("local_runtime".to_string(), "0/2".to_string()),
                ]),
                providers: Vec::new(),
                ready_providers: vec!["yfinance".to_string()],
                pending_providers: vec![
                    "external_http_runtime@live_runtime:operator_runtime_required:base_url_and_service_required"
                        .to_string(),
                    "crypto_public_runtime@live_runtime:operator_runtime_required:base_url_and_service_required"
                        .to_string(),
                    "ibkr_bridge@local_runtime:configured_runtime_unhealthy:ibkr_bridge_config_present_but_runtime_probe_failed"
                        .to_string(),
                ],
                pending_provider_details: vec![
                    ProviderCatalogPendingAgentItem {
                        provider_id: "external_http_runtime".to_string(),
                        domain: "live_runtime".to_string(),
                        status: "operator_runtime_required".to_string(),
                        reason: "base_url_and_service_required".to_string(),
                        install_prompts: vec![
                            "ask whether the user wants zero-config yfinance or external_http_runtime"
                                .to_string(),
                        ],
                    },
                    ProviderCatalogPendingAgentItem {
                        provider_id: "crypto_public_runtime".to_string(),
                        domain: "live_runtime".to_string(),
                        status: "operator_runtime_required".to_string(),
                        reason: "base_url_and_service_required".to_string(),
                        install_prompts: vec![
                            "ask whether the user wants zero-config yfinance or crypto_public_runtime"
                                .to_string(),
                        ],
                    },
                    ProviderCatalogPendingAgentItem {
                        provider_id: "ibkr_bridge".to_string(),
                        domain: "local_runtime".to_string(),
                        status: "configured_runtime_unhealthy".to_string(),
                        reason: "ibkr_bridge_config_present_but_runtime_probe_failed".to_string(),
                        install_prompts: vec!["start ibkr bridge".to_string()],
                    },
                ],
                selectable_providers: vec!["external_http_runtime".to_string(), "crypto_public_runtime".to_string()],
                default_enabled_providers: vec!["yfinance".to_string()],
                install_prompts: vec![],
                available_opt_in_profiles: Vec::new(),
                selected_profile: None,
                selected_profile_full: None,
            },
            "ict-engine analyze-live --symbol NQ --futures-backend external_http_runtime --aux-backend crypto_public_runtime",
            Some("provider_runtime_required"),
        );

        assert!(support.active);
        assert_eq!(support.profile_id, "workflow_auto");
        assert_eq!(support.pending_providers.len(), 2);
        assert!(support
            .pending_providers
            .iter()
            .all(|item| item.contains("external_http_runtime")
                || item.contains("crypto_public_runtime")));
        assert!(support
            .install_prompts
            .iter()
            .any(|prompt| prompt.contains("zero-config yfinance")));
        assert!(support.ask_user_prompts.is_empty());
        assert!(!support
            .pending_providers
            .iter()
            .any(|item| item.contains("ibkr_bridge")));
    }

    #[test]
    fn workflow_provider_support_stays_inactive_when_command_has_no_provider_gap() {
        let support = build_workflow_provider_support(
            &ProviderCatalogAgentSurface {
                summary_line: "market_data:5/7 ready".to_string(),
                ready_by_domain: BTreeMap::new(),
                providers: Vec::new(),
                ready_providers: vec!["yfinance".to_string()],
                pending_providers: vec![
                    "tradingview_mcp@market_data:install_required:missing_tradingview_mcp_api_key"
                        .to_string(),
                ],
                pending_provider_details: vec![ProviderCatalogPendingAgentItem {
                    provider_id: "tradingview_mcp".to_string(),
                    domain: "market_data".to_string(),
                    status: "install_required".to_string(),
                    reason: "missing_tradingview_mcp_api_key".to_string(),
                    install_prompts: vec!["ask for key".to_string()],
                }],
                selectable_providers: vec!["tradingview_mcp".to_string()],
                default_enabled_providers: vec!["yfinance".to_string()],
                install_prompts: vec!["ask for key".to_string()],
                available_opt_in_profiles: Vec::new(),
                selected_profile: None,
                selected_profile_full: None,
            },
            "ict-engine factor-research --symbol NQ --backend native",
            None,
        );

        assert!(!support.active);
        assert!(support.pending_providers.is_empty());
        assert!(support.install_prompts.is_empty());
        assert!(support.ask_user_prompts.is_empty());
    }

    #[test]
    fn workflow_provider_support_generates_explicit_credential_asks() {
        let support = build_workflow_provider_support(
            &ProviderCatalogAgentSurface {
                summary_line: "market_data:0/2 ready | local_runtime:0/1 ready".to_string(),
                ready_by_domain: BTreeMap::new(),
                providers: Vec::new(),
                ready_providers: Vec::new(),
                pending_providers: vec![
                    "tradingview_mcp@market_data:install_required:missing_tradingview_mcp_api_key".to_string(),
                    "kraken_cli@local_runtime:installed_unconfigured:kraken_cli_installed_but_config_missing".to_string(),
                ],
                pending_provider_details: vec![
                    ProviderCatalogPendingAgentItem {
                        provider_id: "tradingview_mcp".to_string(),
                        domain: "market_data".to_string(),
                        status: "install_required".to_string(),
                        reason: "missing_tradingview_mcp_api_key".to_string(),
                        install_prompts: vec!["ask for tradingview key".to_string()],
                    },
                    ProviderCatalogPendingAgentItem {
                        provider_id: "kraken_cli".to_string(),
                        domain: "local_runtime".to_string(),
                        status: "installed_unconfigured".to_string(),
                        reason: "kraken_cli_installed_but_config_missing".to_string(),
                        install_prompts: vec!["ask for kraken credentials".to_string()],
                    },
                ],
                selectable_providers: vec!["tradingview_mcp".to_string()],
                default_enabled_providers: vec![],
                install_prompts: vec![],
                available_opt_in_profiles: Vec::new(),
                selected_profile: None,
                selected_profile_full: None,
            },
            "ict-engine analyze-live --symbol BTCUSD --aux-backend kraken --options-provider tradingview_mcp",
            Some("provider_runtime_required"),
        );

        assert!(support.active);
        assert!(support
            .ask_user_prompts
            .iter()
            .any(|item| item.contains("ICT_ENGINE_TVREMIX_MCP_API_KEY")));
        assert!(support
            .ask_user_prompts
            .iter()
            .any(|item| item.contains("KRAKEN_API_KEY")));
        assert!(support
            .ask_user_prompts
            .iter()
            .any(|item| item.contains("KRAKEN_API_SECRET")));
    }

    #[test]
    fn provider_status_agent_command_is_profile_aware_only_when_opted_in() {
        let default_command = provider_status_agent_command(None);
        assert_eq!(default_command, "ict-engine provider-status --agent");

        let command = provider_status_agent_command(Some(&ProviderProfileAgentSelectionSurface {
            profile_id: "thrill3r_nq_closed_loop_v1".to_string(),
            display_name: "Thrill3r NQ Closed Loop v1".to_string(),
            opt_in_only: true,
            source_kind: "local_path".to_string(),
            selector: "/tmp/provider profile.json".to_string(),
            summary: "Personal NQ workflow".to_string(),
            data_contract_labels: Vec::new(),
            track_statuses: Vec::new(),
            ready_provider_ids: Vec::new(),
            pending_provider_ids: Vec::new(),
            install_prompts: Vec::new(),
        }));
        assert_eq!(
            command,
            "ict-engine provider-status --agent --profile '/tmp/provider profile.json'"
        );
    }

    #[test]
    fn repo_example_profile_can_be_loaded_by_id() {
        let profile = load_provider_profile("thrill3r-nq-closed-loop-v1").unwrap();
        assert_eq!(profile.profile_id, "thrill3r_nq_closed_loop_v1");
        assert!(profile.opt_in_only);
        assert!(profile
            .data_contracts
            .iter()
            .any(|contract| contract.label.contains("Tomac cleaned")));
    }

    #[test]
    fn selected_profile_surface_marks_missing_options_track_pending() {
        let mut surface = sample_surface();
        surface.providers.push(ProviderCatalogItem {
            provider_id: "tradingview_mcp".to_string(),
            domain: "market_data".to_string(),
            selectable_by_user: true,
            adopted_by_default: false,
            access_mode: "api_key_required".to_string(),
            user_access: "api_key_required".to_string(),
            market_fit: vec!["tradfi".to_string(), "crypto".to_string()],
            fallback_priority: Some(31),
            user_summary: "Setup-required TradingViewRemix MCP path.".to_string(),
            ready: false,
            status: "install_required".to_string(),
            reason: "missing_tradingview_mcp_api_key".to_string(),
            capabilities: vec![
                "options_greeks".to_string(),
                "options_implied_volatility".to_string(),
            ],
            notes: vec![],
            install_prompts: vec![
                "Consumer agent request: ask the user for a TradingViewRemix MCP API key."
                    .to_string(),
            ],
        });
        let profile = load_provider_profile("thrill3r-nq-closed-loop-v1").unwrap();
        let selected = build_selected_profile_surface(
            &surface,
            &profile,
            "repo-example",
            "thrill3r-nq-closed-loop-v1",
        )
        .unwrap();

        assert_eq!(selected.profile_id, "thrill3r_nq_closed_loop_v1");
        assert_eq!(selected.selector, "thrill3r-nq-closed-loop-v1");
        assert!(selected
            .track_statuses
            .iter()
            .any(|track| track.contains("options_enriched:pending:tradingview_mcp")));
        assert!(selected
            .install_prompts
            .iter()
            .any(|prompt| prompt.contains("TradingViewRemix MCP API key")));
    }
}
