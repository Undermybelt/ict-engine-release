# Canonical Market Catalog Debt-Closure Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make harness, live bootstrap, workflow bootstrap, and analysis-time companion/proxy resolution derive market truth from versioned repo config instead of local hardcoded tables.

**Architecture:** Keep responsibilities split instead of stuffing everything into one god file. `config/market_data_harness_presets.json` owns provider-executable symbol specs plus live-default derivation metadata. A new `config/market_relationships.json` owns analysis companion universes and options volatility proxies. A typed `market_catalog` module loads both, and all callers consume read-only queries from that module instead of re-implementing symbol/provider logic.

**Tech Stack:** Rust, serde/serde_json, existing market-data harness CLI, existing realtime providers, Cargo test, Cargo clippy.

---

This plan supersedes the shadow-registry cleanup that was only implicit in [2026-04-27-market-data-harness-refactor-plan.md](/Users/thrill3r/projects-ict-engine/ict-engine/docs/plans/2026-04-27-market-data-harness-refactor-plan.md).

## File Structure

- Create: `src/market_catalog/mod.rs`
- Create: `config/market_relationships.json`
- Create: `src/application/data_sources/provider_fetch.rs`
- Create: `src/application/data_sources/options_summary.rs`
- Create: `tests/canonical_market_catalog.rs`
- Modify: `config/market_data_harness_presets.json`
- Modify: `src/lib.rs`
- Modify: `src/application/data_sources/mod.rs`
- Modify: `src/application/data_sources/harness.rs`
- Modify: `src/application/data_sources/control_matrix_runtime.rs`
- Modify: `src/application/data_sources/live_defaults.rs`
- Modify: `src/application/orchestration/workflow_status.rs`
- Modify: `src/analyze_live_command.rs`
- Modify: `src/analyze/smt_correlation_section.rs`
- Modify: `src/data/realtime/live_data.rs`
- Modify: `src/data/realtime/openbb.rs`
- Modify: `src/data/realtime/openalice.rs`
- Modify: `src/data/realtime/nofx.rs`
- Modify: `src/main.rs`

### Task 1: Introduce the canonical market catalog

**Files:**
- Create: `src/market_catalog/mod.rs`
- Create: `config/market_relationships.json`
- Create: `tests/canonical_market_catalog.rs`
- Modify: `config/market_data_harness_presets.json`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write the failing catalog load test**

```rust
use ict_engine::market_catalog::load_market_catalog;
use std::path::PathBuf;

#[test]
fn catalog_derives_live_defaults_and_relationships_from_repo_config() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let catalog = load_market_catalog(&repo_root).unwrap();

    let es = catalog.live_defaults("ES").unwrap();
    assert_eq!(es.futures_symbol, "ES=F");
    assert_eq!(es.spot_symbol, "SPY");
    assert_eq!(es.options_symbol, "SPY");
    assert_eq!(es.spot_kind, "equity");

    let cl = catalog.live_defaults("CL").unwrap();
    assert_eq!(cl.futures_symbol, "CL=F");
    assert_eq!(cl.spot_symbol, "USO");

    let btc = catalog.relationships("BTC").unwrap();
    assert!(btc.related_crypto_symbols.contains(&"ETH".to_string()));
    assert_eq!(btc.options_volatility_proxy, None);
}
```

- [ ] **Step 2: Run the test to verify the catalog does not exist yet**

Run: `cargo test catalog_derives_live_defaults_and_relationships_from_repo_config -- --nocapture`

Expected: FAIL with an unresolved import or missing loader/query methods for `ict_engine::market_catalog`.

- [ ] **Step 3: Add typed catalog loaders and queries**

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketCatalog {
    pub presets: MarketDataHarnessPresetConfig,
    pub relationships: MarketRelationshipConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketLiveDefaultsSpec {
    pub futures_symbol: String,
    pub spot_role: String,
    pub options_role: String,
    pub spot_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketRelationshipSpec {
    pub market_key: String,
    #[serde(default)]
    pub related_futures_symbols: Vec<String>,
    #[serde(default)]
    pub related_etf_companions: Vec<String>,
    #[serde(default)]
    pub related_options_companions: Vec<String>,
    #[serde(default)]
    pub related_cfd_symbols: Vec<String>,
    #[serde(default)]
    pub related_crypto_symbols: Vec<String>,
    #[serde(default)]
    pub options_volatility_proxy: Option<String>,
}

pub fn load_market_catalog(repo_root: impl AsRef<Path>) -> Result<MarketCatalog> {
    let repo_root = repo_root.as_ref();
    Ok(MarketCatalog {
        presets: load_market_data_harness_preset_config(repo_root)?,
        relationships: load_market_relationship_config(repo_root)?,
    })
}
```

- [ ] **Step 4: Upgrade the preset JSON and add the relationship JSON**

```json
{
  "version": 2,
  "markets": [
    {
      "market_key": "NQ",
      "aliases": ["CME:NQ1!", "NASDAQ_FUTURES"],
      "live_defaults": {
        "futures_symbol": "NQ=F",
        "spot_role": "etf_reference",
        "options_role": "options_underlying",
        "spot_kind": "equity"
      },
      "related": {
        "etf_reference": { "display_symbol": "QQQ", "yfinance": "QQQ", "tradingview_mcp": "NASDAQ:QQQ", "ibkr": { "symbol": "QQQ", "sec_type": "STK", "exchange": "SMART", "currency": "USD" } },
        "cfd_reference": { "display_symbol": "NDX", "tradingview_mcp": "NASDAQ:NDX", "ibkr": { "symbol": "NDX", "sec_type": "IND", "exchange": "NASDAQ", "currency": "USD" } },
        "volatility_reference": { "display_symbol": "VIX", "yfinance": "^VIX", "tradingview_mcp": "CBOE:VIX", "ibkr": { "symbol": "VIX", "sec_type": "IND", "exchange": "CBOE", "currency": "USD" } },
        "options_underlying": { "display_symbol": "QQQ", "yfinance": "QQQ", "tradingview_mcp": "NASDAQ:QQQ" }
      }
    },
    {
      "market_key": "ES",
      "aliases": ["CME:ES1!", "SP500_FUTURES"],
      "live_defaults": {
        "futures_symbol": "ES=F",
        "spot_role": "etf_reference",
        "options_role": "options_underlying",
        "spot_kind": "equity"
      },
      "related": {
        "etf_reference": { "display_symbol": "SPY", "yfinance": "SPY", "tradingview_mcp": "AMEX:SPY", "ibkr": { "symbol": "SPY", "sec_type": "STK", "exchange": "SMART", "currency": "USD" } },
        "cfd_reference": { "display_symbol": "SPX", "tradingview_mcp": "SP:SPX", "ibkr": { "symbol": "SPX", "sec_type": "IND", "exchange": "CBOE", "currency": "USD" } },
        "volatility_reference": { "display_symbol": "VIX", "yfinance": "^VIX", "tradingview_mcp": "CBOE:VIX", "ibkr": { "symbol": "VIX", "sec_type": "IND", "exchange": "CBOE", "currency": "USD" } },
        "options_underlying": { "display_symbol": "SPY", "yfinance": "SPY", "tradingview_mcp": "AMEX:SPY" }
      }
    },
    {
      "market_key": "YM",
      "aliases": ["CBOT:YM1!", "DOW_FUTURES"],
      "live_defaults": {
        "futures_symbol": "YM=F",
        "spot_role": "etf_reference",
        "options_role": "options_underlying",
        "spot_kind": "equity"
      },
      "related": {
        "etf_reference": { "display_symbol": "DIA", "yfinance": "DIA", "tradingview_mcp": "AMEX:DIA", "ibkr": { "symbol": "DIA", "sec_type": "STK", "exchange": "SMART", "currency": "USD" } },
        "cfd_reference": { "display_symbol": "DJI", "tradingview_mcp": "DJ:DJI", "ibkr": { "symbol": "DJI", "sec_type": "IND", "exchange": "NYSE", "currency": "USD" } },
        "volatility_reference": { "display_symbol": "VIX", "yfinance": "^VIX", "tradingview_mcp": "CBOE:VIX", "ibkr": { "symbol": "VIX", "sec_type": "IND", "exchange": "CBOE", "currency": "USD" } },
        "options_underlying": { "display_symbol": "DIA", "yfinance": "DIA", "tradingview_mcp": "AMEX:DIA" }
      }
    },
    {
      "market_key": "GC",
      "aliases": ["COMEX:GC1!", "GOLD_FUTURES"],
      "live_defaults": {
        "futures_symbol": "GC=F",
        "spot_role": "etf_reference",
        "options_role": "options_underlying",
        "spot_kind": "etf"
      },
      "related": {
        "etf_reference": { "display_symbol": "GLD", "yfinance": "GLD", "tradingview_mcp": "AMEX:GLD", "ibkr": { "symbol": "GLD", "sec_type": "STK", "exchange": "SMART", "currency": "USD" } },
        "cfd_reference": { "display_symbol": "XAUUSD", "tradingview_mcp": "OANDA:XAUUSD", "ibkr": { "symbol": "XAUUSD", "sec_type": "CASH", "exchange": "IDEALPRO", "currency": "USD" } },
        "volatility_reference": { "display_symbol": "VIX", "yfinance": "^VIX", "tradingview_mcp": "CBOE:VIX", "ibkr": { "symbol": "VIX", "sec_type": "IND", "exchange": "CBOE", "currency": "USD" } },
        "options_underlying": { "display_symbol": "GLD", "yfinance": "GLD", "tradingview_mcp": "AMEX:GLD" }
      }
    },
    {
      "market_key": "CL",
      "aliases": ["NYMEX:CL1!", "WTI_FUTURES"],
      "live_defaults": {
        "futures_symbol": "CL=F",
        "spot_role": "etf_reference",
        "options_role": "options_underlying",
        "spot_kind": "etf"
      },
      "related": {
        "etf_reference": { "display_symbol": "USO", "yfinance": "USO", "tradingview_mcp": "AMEX:USO", "ibkr": { "symbol": "USO", "sec_type": "STK", "exchange": "SMART", "currency": "USD" } },
        "cfd_reference": { "display_symbol": "USOIL", "tradingview_mcp": "TVC:USOIL" },
        "volatility_reference": { "display_symbol": "VIX", "yfinance": "^VIX", "tradingview_mcp": "CBOE:VIX", "ibkr": { "symbol": "VIX", "sec_type": "IND", "exchange": "CBOE", "currency": "USD" } },
        "options_underlying": { "display_symbol": "USO", "yfinance": "USO", "tradingview_mcp": "AMEX:USO" }
      }
    }
  ]
}
```

```json
{
  "version": 1,
  "markets": [
    { "market_key": "NQ", "related_futures_symbols": ["ES", "YM"], "related_etf_companions": ["SPY", "DIA"], "related_options_companions": ["SPY", "DIA"], "related_cfd_symbols": ["NAS100", "US500", "US30"], "related_crypto_symbols": ["BTC", "ETH", "SOL"], "options_volatility_proxy": "^VXN" },
    { "market_key": "ES", "related_futures_symbols": ["NQ", "YM"], "related_etf_companions": ["QQQ", "DIA"], "related_options_companions": ["QQQ", "DIA"], "related_cfd_symbols": ["US500", "NAS100", "US30"], "related_crypto_symbols": ["BTC", "ETH"], "options_volatility_proxy": "^VIX" },
    { "market_key": "YM", "related_futures_symbols": ["NQ", "ES"], "related_etf_companions": ["QQQ", "SPY"], "related_options_companions": ["QQQ", "SPY"], "related_cfd_symbols": ["US30", "NAS100", "US500"], "related_crypto_symbols": ["BTC", "ETH"], "options_volatility_proxy": "^VIX" },
    { "market_key": "GC", "related_futures_symbols": ["SI"], "related_etf_companions": ["SLV"], "related_options_companions": ["SLV"], "related_cfd_symbols": ["XAUUSD", "XAGUSD"], "related_crypto_symbols": ["BTC", "ETH"], "options_volatility_proxy": "^GVZ" },
    { "market_key": "CL", "related_futures_symbols": ["BZ"], "related_etf_companions": ["BNO"], "related_options_companions": ["BNO"], "related_cfd_symbols": ["USOIL", "UKOIL"], "related_crypto_symbols": ["BTC", "ETH", "SOL"], "options_volatility_proxy": "^OVX" },
    { "market_key": "BTC", "related_futures_symbols": [], "related_etf_companions": [], "related_options_companions": [], "related_cfd_symbols": ["BTCUSD"], "related_crypto_symbols": ["ETH", "SOL"], "options_volatility_proxy": null },
    { "market_key": "ETH", "related_futures_symbols": [], "related_etf_companions": [], "related_options_companions": [], "related_cfd_symbols": ["ETHUSD"], "related_crypto_symbols": ["BTC", "SOL"], "options_volatility_proxy": null },
    { "market_key": "SOL", "related_futures_symbols": [], "related_etf_companions": [], "related_options_companions": [], "related_cfd_symbols": ["SOLUSD"], "related_crypto_symbols": ["BTC", "ETH"], "options_volatility_proxy": null }
  ]
}
```

- [ ] **Step 5: Run the catalog test to verify it passes**

Run: `cargo test catalog_derives_live_defaults_and_relationships_from_repo_config -- --nocapture`

Expected: PASS with one test executed and zero failures.

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/market_catalog/mod.rs config/market_data_harness_presets.json config/market_relationships.json tests/canonical_market_catalog.rs
git commit -m "refactor: add canonical market catalog"
```

### Task 2: Make harness plans executable without private remapping

**Files:**
- Create: `src/application/data_sources/provider_fetch.rs`
- Modify: `src/application/data_sources/harness.rs`
- Modify: `src/application/data_sources/mod.rs`
- Modify: `src/application/data_sources/control_matrix_runtime.rs`
- Modify: `tests/canonical_market_catalog.rs`

- [ ] **Step 1: Write the failing harness execution-spec test**

```rust
use ict_engine::application::data_sources::{
    build_market_data_harness_plan, MarketDataHarnessRequest,
};
use ict_engine::market_catalog::load_market_catalog;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[test]
fn harness_plan_preserves_provider_execution_specs_from_catalog() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let catalog = load_market_catalog(&repo_root).unwrap();

    let plan = build_market_data_harness_plan(
        MarketDataHarnessRequest {
            market_key: "ES".to_string(),
            primary_data_path: None,
            interval: Some("1d".to_string()),
            start: None,
            end: None,
            count: Some(30),
            related_roles: vec!["etf_reference".to_string(), "volatility_reference".to_string()],
            provider_preferences: BTreeMap::from([
                ("etf_reference".to_string(), "tradingview_mcp".to_string()),
                ("volatility_reference".to_string(), "ibkr".to_string()),
            ]),
            symbol_overrides: BTreeMap::new(),
        },
        &catalog,
    )
    .unwrap();

    let spy = plan.tasks.iter().find(|task| task.role == "etf_reference").unwrap();
    assert_eq!(spy.request_symbol(), "AMEX:SPY");

    let vix = plan.tasks.iter().find(|task| task.role == "volatility_reference").unwrap();
    let contract = vix.ibkr_contract().unwrap();
    assert_eq!(contract.exchange, "CBOE");
    assert_eq!(contract.sec_type, "IND");
}
```

- [ ] **Step 2: Run the test to verify tasks do not carry execution specs yet**

Run: `cargo test harness_plan_preserves_provider_execution_specs_from_catalog -- --nocapture`

Expected: FAIL because `MarketDataHarnessPlan` still accepts the old preset config type and `MarketDataHarnessTask` has no execution-spec accessors.

- [ ] **Step 3: Move generic provider fetching out of `control_matrix_runtime.rs` and into harness-owned execution types**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderExecutionRequest {
    Yahoo { symbol: String },
    TradingViewMcp { symbol: String },
    Ibkr {
        symbol: String,
        sec_type: String,
        exchange: String,
        currency: String,
        primary_exchange: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketDataHarnessTask {
    pub role: String,
    pub provider: String,
    pub operation: MarketDataHarnessOperation,
    pub display_symbol: String,
    pub request: ProviderExecutionRequest,
}

pub fn execute_provider_request(
    task: &MarketDataHarnessTask,
    interval: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    count: usize,
) -> Result<MarketDataHarnessEnvelope> {
    match (&task.operation, &task.request) {
        (MarketDataHarnessOperation::Ohlcv, ProviderExecutionRequest::Yahoo { symbol }) => {
            fetch_yahoo_candles(symbol, interval, start, end)
        }
        (MarketDataHarnessOperation::Ohlcv, ProviderExecutionRequest::TradingViewMcp { symbol }) => {
            fetch_tradingview_ohlcv(symbol, interval, start, end, count)
        }
        (MarketDataHarnessOperation::Ohlcv, ProviderExecutionRequest::Ibkr { symbol, sec_type, exchange, currency, primary_exchange }) => {
            fetch_ibkr_historical_candles(symbol, sec_type, exchange, currency, primary_exchange.as_deref(), interval, start, end)
        }
        _ => bail!("unsupported provider request"),
    }
}
```

- [ ] **Step 4: Remove private remap helpers from runtime-only code**

Run: `rg -n 'fn tradingview_symbol|fn ibkr_security_type|fn ibkr_exchange' src/application/data_sources/control_matrix_runtime.rs`

Expected: no output, because execution-spec selection now comes from catalog-backed harness tasks.

- [ ] **Step 5: Run the harness execution-spec test to verify it passes**

Run: `cargo test harness_plan_preserves_provider_execution_specs_from_catalog -- --nocapture`

Expected: PASS with one test executed and zero failures.

- [ ] **Step 6: Commit**

```bash
git add src/application/data_sources/provider_fetch.rs src/application/data_sources/harness.rs src/application/data_sources/mod.rs src/application/data_sources/control_matrix_runtime.rs tests/canonical_market_catalog.rs
git commit -m "refactor: make harness execute catalog-backed provider specs"
```

### Task 3: Route live bootstrap and workflow surfaces through the catalog

**Files:**
- Modify: `src/application/data_sources/live_defaults.rs`
- Modify: `src/application/orchestration/workflow_status.rs`
- Modify: `src/analyze_live_command.rs`
- Modify: `src/main.rs`
- Modify: `tests/canonical_market_catalog.rs`

- [ ] **Step 1: Write the failing bootstrap/live-default drift test**

```rust
use ict_engine::application::data_sources::build_inferable_live_defaults_map;
use ict_engine::market_catalog::load_market_catalog;
use std::path::PathBuf;

#[test]
fn workflow_bootstrap_defaults_match_catalog_defaults() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let catalog = load_market_catalog(&repo_root).unwrap();
    let defaults = build_inferable_live_defaults_map(&catalog);

    assert_eq!(defaults["YM"]["spot_symbol"], "DIA");
    assert_eq!(defaults["CL"]["options_symbol"], "USO");
}
```

- [ ] **Step 2: Run the test to verify workflow bootstrap still owns a local table**

Run: `cargo test workflow_bootstrap_defaults_match_catalog_defaults -- --nocapture`

Expected: FAIL because no shared `build_inferable_live_defaults_map` helper exists and `workflow_status.rs` still builds its own `BTreeMap`.

- [ ] **Step 3: Make live-default helpers pure catalog readers and load the catalog once at entry points**

```rust
pub fn analyze_live_inferred_symbols(
    catalog: &MarketCatalog,
    symbol: &str,
) -> Option<AnalyzeLiveSymbolDefaults> {
    catalog.live_defaults(symbol)
}

pub fn build_inferable_live_defaults_map(
    catalog: &MarketCatalog,
) -> BTreeMap<String, BTreeMap<String, String>> {
    catalog
        .market_keys_with_live_defaults()
        .into_iter()
        .filter_map(|market_key| {
            catalog.live_defaults(&market_key).map(|defaults| {
                (
                    market_key,
                    BTreeMap::from([
                        ("futures_symbol".to_string(), defaults.futures_symbol),
                        ("spot_symbol".to_string(), defaults.spot_symbol),
                        ("options_symbol".to_string(), defaults.options_symbol),
                        ("spot_kind".to_string(), defaults.spot_kind),
                    ]),
                )
            })
        })
        .collect()
}
```

- [ ] **Step 4: Remove the duplicate backend-base-url resolver from `main.rs`**

```rust
let futures_base_url = ict_engine::application::data_sources::resolve_live_backend_base_url(
    &futures_backend,
    &openalice_base_url,
    &nofx_base_url,
);
let aux_base_url = ict_engine::application::data_sources::resolve_live_backend_base_url(
    &aux_backend,
    &openalice_base_url,
    &nofx_base_url,
);
```

- [ ] **Step 5: Run the drift test and one CLI-focused regression test**

Run: `cargo test workflow_bootstrap_defaults_match_catalog_defaults -- --nocapture`

Expected: PASS.

Run: `cargo test analyze_live_command_input_carries_backend_and_symbols -- --nocapture`

Expected: PASS, proving the public CLI input type remains intact while default resolution moves behind the catalog.

- [ ] **Step 6: Commit**

```bash
git add src/application/data_sources/live_defaults.rs src/application/orchestration/workflow_status.rs src/analyze_live_command.rs src/main.rs tests/canonical_market_catalog.rs
git commit -m "refactor: route live defaults through market catalog"
```

### Task 4: Route analysis relationships and options proxy fallback through the catalog

**Files:**
- Create: `src/application/data_sources/options_summary.rs`
- Modify: `src/application/data_sources/mod.rs`
- Modify: `src/analyze/smt_correlation_section.rs`
- Modify: `src/analyze_live_command.rs`
- Modify: `src/data/realtime/live_data.rs`
- Modify: `src/data/realtime/openbb.rs`
- Modify: `src/data/realtime/openalice.rs`
- Modify: `src/data/realtime/nofx.rs`
- Modify: `tests/canonical_market_catalog.rs`

- [ ] **Step 1: Write the failing relationship/proxy tests**

```rust
use ict_engine::market_catalog::load_market_catalog;
use std::path::PathBuf;

#[test]
fn catalog_relationships_define_es_companions_and_proxy() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let catalog = load_market_catalog(&repo_root).unwrap();

    let es = catalog.relationships("ES").unwrap();
    assert_eq!(es.options_volatility_proxy.as_deref(), Some("^VIX"));
    assert!(es.related_cfd_symbols.contains(&"US500".to_string()));
}
```

```rust
pub fn fetch_options_summary_with_fallback(
    provider: &dyn IntegratedLiveDataSource,
    catalog: &MarketCatalog,
    market_key: &str,
    options_symbol: &str,
) -> Result<OptionsChainSummary> {
    match provider.fetch_options_chain_summary(options_symbol) {
        Ok(summary) => Ok(summary),
        Err(primary_err) => {
            let proxy_symbol = catalog
                .relationships(market_key)
                .and_then(|item| item.options_volatility_proxy.clone())
                .ok_or(primary_err)?;
            provider.fetch_options_volatility_proxy_summary(&proxy_symbol, options_symbol)
        }
    }
}
```

- [ ] **Step 2: Run the relationship/proxy tests to verify the shared helper does not exist yet**

Run: `cargo test catalog_relationships_define_es_companions_and_proxy -- --nocapture`

Expected: FAIL because the relationship registry and `fetch_options_volatility_proxy_summary` contract do not exist yet.

- [ ] **Step 3: Replace local analysis-time tables with catalog-backed views**

```rust
pub fn correlation_assets_for(
    catalog: &MarketCatalog,
    symbol: &str,
    spot_symbol: &str,
    options_symbol: &str,
) -> CorrelationAssetMap {
    let related = catalog.relationships(symbol).cloned().unwrap_or_default();
    CorrelationAssetMap {
        related_futures_symbols: related.related_futures_symbols,
        related_etf_symbols: std::iter::once(spot_symbol.to_string())
            .chain(related.related_etf_companions.into_iter())
            .collect(),
        related_options_symbols: std::iter::once(options_symbol.to_string())
            .chain(related.related_options_companions.into_iter())
            .collect(),
        related_cfd_symbols: related.related_cfd_symbols,
        related_crypto_symbols: related.related_crypto_symbols,
    }
}
```

- [ ] **Step 4: Make `OpenBBProvider` execute a caller-supplied proxy symbol instead of selecting one itself**

```rust
fn fetch_options_volatility_proxy_summary(
    &self,
    proxy_symbol: &str,
    underlying_symbol: &str,
) -> Result<OptionsChainSummary> {
    let candles = self.fetch_chart_candles(
        proxy_symbol,
        "1d",
        Utc::now() - chrono::Duration::days(45),
        Utc::now(),
    )?;
    let latest = candles.last().ok_or_else(|| anyhow!("no volatility proxy candles"))?;
    Ok(OptionsChainSummary {
        symbol: underlying_symbol.to_string(),
        source: Some(format!("openbb:volatility_proxy:{proxy_symbol}")),
        underlying_price: None,
        call_open_interest: 0.0,
        put_open_interest: 0.0,
        put_call_oi_ratio: None,
        call_volume: 0.0,
        put_volume: 0.0,
        put_call_volume_ratio: None,
        near_atm_implied_volatility: Some((latest.close / 100.0).max(0.0)),
        near_atm_delta: None,
        near_atm_gamma: None,
        near_atm_vega: None,
        call_gamma_oi: None,
        put_gamma_oi: None,
        gamma_skew: None,
        nearest_expiration_dte: None,
    })
}
```

- [ ] **Step 5: Run the relationship/proxy test and an analyze-section regression**

Run: `cargo test catalog_relationships_define_es_companions_and_proxy -- --nocapture`

Expected: PASS.

Run: `cargo test options_hedging_section -- --nocapture`

Expected: PASS, proving the `options_volatility_proxy_only` narrative still works after proxy selection moves out of `openbb.rs`.

- [ ] **Step 6: Commit**

```bash
git add src/application/data_sources/options_summary.rs src/application/data_sources/mod.rs src/analyze/smt_correlation_section.rs src/analyze_live_command.rs src/data/realtime/live_data.rs src/data/realtime/openbb.rs src/data/realtime/openalice.rs src/data/realtime/nofx.rs tests/canonical_market_catalog.rs
git commit -m "refactor: route analysis relationships through market catalog"
```

### Task 5: Delete the remaining shadow registries and prove drift is gone

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/canonical_market_catalog.rs`

- [ ] **Step 1: Write the final coverage test for catalog-owned production markets**

```rust
use ict_engine::market_catalog::load_market_catalog;
use std::path::PathBuf;

#[test]
fn catalog_covers_all_production_markets() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let catalog = load_market_catalog(&repo_root).unwrap();

    for market in ["NQ", "ES", "YM", "GC", "CL"] {
        assert!(catalog.live_defaults(market).is_some(), "missing live defaults for {market}");
    }

    for market in ["NQ", "ES", "YM", "GC", "CL", "BTC", "ETH", "SOL"] {
        assert!(catalog.relationships(market).is_some(), "missing relationships for {market}");
    }
}
```

- [ ] **Step 2: Run the coverage test to verify at least one shadow table still exists**

Run: `cargo test catalog_covers_all_production_markets -- --nocapture`

Expected: FAIL until all hardcoded production tables are replaced and the final config set is complete.

- [ ] **Step 3: Remove stale inline truth from `main.rs` and keep only catalog-backed tests**

Run: `rg -n 'test_live_inferable_defaults_cover_gc_and_cl|test_analyze_live_symbol_can_infer_gc_and_cl_defaults|resolve_live_backend_base_url\\(' src/main.rs`

Expected before edit: matches for legacy duplicate tests and the local backend resolver.

Expected after edit: no duplicate live-default tests and no local `resolve_live_backend_base_url` implementation.

- [ ] **Step 4: Run shadow-registry grep checks**

Run: `rg -n 'Some\\(\\(\"NQ=F\"|inferable_live_defaults =|fn tradingview_symbol|fn ibkr_security_type|fn ibkr_exchange|fn options_volatility_proxy' src`

Expected: no output.

- [ ] **Step 5: Run full verification**

Run: `cargo test --all -- --nocapture`

Expected: PASS.

Run: `cargo clippy --all-targets --all-features -- -D warnings`

Expected: PASS with zero warnings.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs tests/canonical_market_catalog.rs
git commit -m "test: remove shadow registry drift paths"
```
