//! Schema + loader for the Auto-Quant `strategy_library.json` manifest.
//!
//! The producer side lives in
//! `Auto-Quant/export_strategy_library.py`; field names and types here
//! must stay in lock-step with that script. `manifest_version` must be
//! present in `MANIFEST_SUPPORTED_VERSIONS` for the load to succeed.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

/// Versions of the `strategy_library.json` schema this loader accepts.
/// Auto-Quant currently emits "1.0".
pub const MANIFEST_SUPPORTED_VERSIONS: &[&str] = &["1.0"];

/// Top-level manifest produced by `export_strategy_library.py`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrategyLibraryManifest {
    #[serde(default)]
    pub manifest_version: String,
    #[serde(default)]
    pub exported_at: String,
    #[serde(default)]
    pub auto_quant_repo_url: String,
    #[serde(default)]
    pub auto_quant_pinned_ref: String,
    #[serde(default)]
    pub config_path: String,
    #[serde(default)]
    pub timeframe: String,
    #[serde(default)]
    pub log_path: String,
    #[serde(default)]
    pub strategies: Vec<StrategyLibraryEntry>,
    #[serde(default)]
    pub validation_errors: Vec<StrategyLibraryValidationError>,
}

/// One strategy's worth of metadata + validation evidence.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrategyLibraryEntry {
    pub name: String,
    #[serde(default)]
    pub file_path: String,
    pub metadata: StrategyLibraryMetadata,
    /// `ok` (backtest succeeded) | `error` (backtest threw) | `not_run`.
    pub status: String,
    #[serde(default)]
    pub validation_metrics: Option<StrategyLibraryValidationMetrics>,
    #[serde(default)]
    pub per_pair_metrics: BTreeMap<String, StrategyLibraryValidationMetrics>,
    #[serde(default)]
    pub pairs: Vec<String>,
    #[serde(default)]
    pub timerange: String,
    #[serde(default)]
    pub commit: String,
    #[serde(default)]
    pub error: Option<StrategyLibraryEntryError>,
}

/// Mirror of the `auto_quant_meta.StrategyMeta.to_json_dict` payload.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrategyLibraryMetadata {
    #[serde(default)]
    pub strategy: String,
    #[serde(default)]
    pub mutation_id: String,
    #[serde(default)]
    pub base_factor: String,
    #[serde(default)]
    pub hypothesis: String,
    #[serde(default)]
    pub paradigm: String,
    #[serde(default)]
    pub expected_regime: String,
    #[serde(default)]
    pub factors_used: Vec<String>,
    #[serde(default)]
    pub parent: String,
    #[serde(default)]
    pub asset_class: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub created: String,
}

/// Aggregate (and per-pair) backtest metrics. All fields default to 0.0
/// / 0 so a partially-populated manifest entry round-trips cleanly.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrategyLibraryValidationMetrics {
    #[serde(default)]
    pub sharpe: f64,
    #[serde(default)]
    pub sortino: f64,
    #[serde(default)]
    pub calmar: f64,
    #[serde(default)]
    pub total_profit_pct: f64,
    #[serde(default)]
    pub max_drawdown_pct: f64,
    #[serde(default)]
    pub trade_count: u32,
    #[serde(default)]
    pub win_rate_pct: f64,
    #[serde(default)]
    pub profit_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrategyLibraryEntryError {
    #[serde(default)]
    #[serde(rename = "type")]
    pub error_type: String,
    #[serde(default)]
    pub message: String,
}

/// One row in the manifest's top-level `validation_errors[]` —
/// strategies whose source-file `AUTO_QUANT_META` block failed to parse.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrategyLibraryValidationError {
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub error: String,
}

/// Strongly-typed status enum (decoupled from the wire string so future
/// statuses remain parseable through `Other`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrategyLibraryEntryStatus {
    Ok,
    Error,
    NotRun,
    Other(String),
}

impl StrategyLibraryEntry {
    pub fn status_kind(&self) -> StrategyLibraryEntryStatus {
        match self.status.as_str() {
            "ok" => StrategyLibraryEntryStatus::Ok,
            "error" => StrategyLibraryEntryStatus::Error,
            "not_run" => StrategyLibraryEntryStatus::NotRun,
            other => StrategyLibraryEntryStatus::Other(other.to_string()),
        }
    }
}

/// Read + validate a manifest from disk.
///
/// Errors with a contextful message if the file is missing, the JSON is
/// malformed, or the `manifest_version` is not in
/// `MANIFEST_SUPPORTED_VERSIONS`.
pub fn load_strategy_library_manifest<P: AsRef<Path>>(path: P) -> Result<StrategyLibraryManifest> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).with_context(|| {
        format!(
            "failed to read strategy library manifest '{}'",
            path.display()
        )
    })?;
    let manifest: StrategyLibraryManifest = serde_json::from_str(&raw).with_context(|| {
        format!(
            "failed to parse strategy library manifest '{}'",
            path.display()
        )
    })?;
    if manifest.manifest_version.is_empty() {
        bail!(
            "manifest at '{}' is missing the required 'manifest_version' field",
            path.display()
        );
    }
    if !MANIFEST_SUPPORTED_VERSIONS.contains(&manifest.manifest_version.as_str()) {
        return Err(anyhow!(
            "manifest at '{}' has unsupported manifest_version='{}'; supported: {:?}",
            path.display(),
            manifest.manifest_version,
            MANIFEST_SUPPORTED_VERSIONS
        ));
    }
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_tmp(json: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut f, json.as_bytes()).unwrap();
        f
    }

    #[test]
    fn loads_minimal_manifest_with_just_version() {
        let f = write_tmp(r#"{"manifest_version":"1.0"}"#);
        let m = load_strategy_library_manifest(f.path()).unwrap();
        assert_eq!(m.manifest_version, "1.0");
        assert!(m.strategies.is_empty());
        assert!(m.validation_errors.is_empty());
    }

    #[test]
    fn loads_full_entry_with_metrics() {
        let f = write_tmp(
            r#"{
              "manifest_version":"1.0",
              "exported_at":"2026-04-26T11:00:00Z",
              "auto_quant_repo_url":"git@host:org/auto-quant.git",
              "auto_quant_pinned_ref":"abc123",
              "config_path":"config.ibkr.json",
              "timeframe":"5m",
              "log_path":"run_ibkr.log",
              "strategies":[{
                "name":"GoodStrat",
                "file_path":"user_data/strategies_ibkr/GoodStrat.py",
                "metadata":{
                  "strategy":"GoodStrat",
                  "mutation_id":"mb-001",
                  "base_factor":"ict_breakout_5m",
                  "hypothesis":"breakout from manipulation",
                  "paradigm":"breakout",
                  "expected_regime":"expansion",
                  "factors_used":["bos","fvg","atr"],
                  "parent":"root",
                  "asset_class":"equities",
                  "status":"active",
                  "created":"abc123"
                },
                "status":"ok",
                "validation_metrics":{
                  "sharpe":1.42,"sortino":2.13,"calmar":4.5,
                  "total_profit_pct":12.3,"max_drawdown_pct":-3.2,
                  "trade_count":87,"win_rate_pct":54.5,"profit_factor":1.85
                },
                "per_pair_metrics":{
                  "SPY/USD":{"sharpe":1.5,"trade_count":50,"win_rate_pct":58.0,
                             "total_profit_pct":15.0,"max_drawdown_pct":-2.5,
                             "profit_factor":2.1,"sortino":0.0,"calmar":0.0}
                },
                "pairs":["SPY/USD"],
                "timerange":"20240101-20240201",
                "commit":"abc123"
              }],
              "validation_errors":[]
            }"#,
        );
        let m = load_strategy_library_manifest(f.path()).unwrap();
        assert_eq!(m.strategies.len(), 1);
        let s = &m.strategies[0];
        assert_eq!(s.name, "GoodStrat");
        assert_eq!(s.status_kind(), StrategyLibraryEntryStatus::Ok);
        assert_eq!(s.metadata.factors_used, vec!["bos", "fvg", "atr"]);
        let agg = s.validation_metrics.as_ref().unwrap();
        assert_eq!(agg.trade_count, 87);
        assert!((agg.win_rate_pct - 54.5).abs() < 1e-9);
        assert_eq!(s.per_pair_metrics.len(), 1);
    }

    #[test]
    fn rejects_unsupported_version() {
        let f = write_tmp(r#"{"manifest_version":"99.0"}"#);
        let err = load_strategy_library_manifest(f.path()).unwrap_err();
        assert!(err.to_string().contains("unsupported manifest_version"));
    }

    #[test]
    fn rejects_missing_version() {
        let f = write_tmp(r#"{}"#);
        let err = load_strategy_library_manifest(f.path()).unwrap_err();
        assert!(err.to_string().contains("manifest_version"));
    }

    #[test]
    fn rejects_malformed_json() {
        let f = write_tmp("not json");
        let err = load_strategy_library_manifest(f.path()).unwrap_err();
        assert!(err.to_string().contains("failed to parse"));
    }

    #[test]
    fn entry_status_kind_handles_unknown_string() {
        let e = StrategyLibraryEntry {
            status: "weird".into(),
            ..StrategyLibraryEntry::default()
        };
        assert_eq!(
            e.status_kind(),
            StrategyLibraryEntryStatus::Other("weird".into())
        );
    }
}
