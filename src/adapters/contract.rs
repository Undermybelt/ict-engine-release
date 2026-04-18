use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExternalErrorCategory {
    Api,
    Auth,
    Network,
    RateLimit,
    Validation,
    Config,
    Io,
    Parse,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolSafetyLevel {
    Safe,
    Dangerous,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExternalDiagnostics {
    pub exit_code: Option<i32>,
    pub stderr_excerpt: Option<String>,
    pub provider_detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalToolError {
    pub category: ExternalErrorCategory,
    pub message: String,
    pub retryable: bool,
    #[serde(default)]
    pub diagnostics: ExternalDiagnostics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalToolResult {
    pub ok: bool,
    pub provider: String,
    pub operation: String,
    #[serde(default)]
    pub data: serde_json::Value,
    #[serde(default)]
    pub error: Option<ExternalToolError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolCatalogParameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolCatalogEntry {
    pub name: String,
    pub provider: String,
    pub operation: String,
    pub description: String,
    pub auth_required: bool,
    pub dangerous: bool,
    #[serde(default)]
    pub asset_classes: Vec<String>,
    #[serde(default)]
    pub parameters: Vec<ToolCatalogParameter>,
    #[serde(default)]
    pub notes: Vec<String>,
}

pub trait ExternalMarketDataAdapter {
    fn provider_name(&self) -> &'static str;
    fn tool_catalog(&self) -> Vec<ToolCatalogEntry>;
    fn fetch_ticker(
        &self,
        symbol: &str,
        params: BTreeMap<String, String>,
    ) -> anyhow::Result<ExternalToolResult>;
    fn fetch_ohlc(
        &self,
        symbol: &str,
        params: BTreeMap<String, String>,
    ) -> anyhow::Result<ExternalToolResult>;
    fn fetch_orderbook(
        &self,
        symbol: &str,
        params: BTreeMap<String, String>,
    ) -> anyhow::Result<ExternalToolResult>;
}
