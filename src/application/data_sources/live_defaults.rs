use std::collections::BTreeMap;

use crate::data::realtime::LiveDataBackend;
use crate::market_catalog::MarketCatalog;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzeLiveSymbolDefaults {
    pub futures_symbol: String,
    pub spot_symbol: String,
    pub options_symbol: String,
    pub spot_kind: String,
}

pub fn resolve_live_backend_base_url(
    backend: &str,
    external_http_base_url: &str,
    crypto_public_base_url: &str,
) -> String {
    match backend.trim().to_ascii_lowercase().as_str() {
        "yfinance" => "native://yfinance".to_string(),
        "external_http" | "external_http_runtime" => external_http_base_url.to_string(),
        "crypto_public" | "crypto_public_runtime" => crypto_public_base_url.to_string(),
        "tradingview" | "tradingview_mcp" | "tv_mcp" => "mcp://tradingview_mcp".to_string(),
        _ => "native://yfinance".to_string(),
    }
}

pub fn analyze_live_inferred_symbols(
    catalog: &MarketCatalog,
    symbol: &str,
) -> Option<AnalyzeLiveSymbolDefaults> {
    let defaults = catalog.live_defaults(symbol)?;
    Some(AnalyzeLiveSymbolDefaults {
        futures_symbol: defaults.futures_symbol,
        spot_symbol: defaults.spot_symbol,
        options_symbol: defaults.options_symbol,
        spot_kind: defaults.spot_kind,
    })
}

pub fn build_inferable_live_defaults_map(
    catalog: &MarketCatalog,
) -> BTreeMap<String, BTreeMap<String, String>> {
    catalog
        .market_keys_with_live_defaults()
        .into_iter()
        .filter_map(|market_key| {
            analyze_live_inferred_symbols(catalog, &market_key).map(|defaults| {
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

pub fn parse_live_backend(backend: &str) -> anyhow::Result<LiveDataBackend> {
    LiveDataBackend::parse(backend)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_catalog::load_market_catalog;
    use std::path::PathBuf;

    #[test]
    fn resolve_live_backend_base_url_uses_expected_sources() {
        assert_eq!(
            resolve_live_backend_base_url("yfinance", "http://ext", "http://crypto"),
            "native://yfinance"
        );
        assert_eq!(
            resolve_live_backend_base_url("external_http_runtime", "http://ext", "http://crypto"),
            "http://ext"
        );
        assert_eq!(
            resolve_live_backend_base_url("crypto_public_runtime", "http://ext", "http://crypto"),
            "http://crypto"
        );
        assert_eq!(
            resolve_live_backend_base_url("tradingview_mcp", "http://ext", "http://crypto"),
            "mcp://tradingview_mcp"
        );
        assert_eq!(
            resolve_live_backend_base_url("unknown", "http://ext", "http://crypto"),
            "native://yfinance"
        );
    }

    #[test]
    fn analyze_live_symbol_can_infer_gc_and_cl_defaults() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let catalog = load_market_catalog(&repo_root).unwrap();
        let gc = analyze_live_inferred_symbols(&catalog, "GC").unwrap();
        let cl = analyze_live_inferred_symbols(&catalog, "CL").unwrap();
        assert_eq!(gc.futures_symbol, "GC=F");
        assert_eq!(gc.spot_symbol, "GLD");
        assert_eq!(cl.futures_symbol, "CL=F");
        assert_eq!(cl.spot_symbol, "USO");
    }

    #[test]
    fn build_inferable_live_defaults_map_matches_catalog_markets() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let catalog = load_market_catalog(&repo_root).unwrap();
        let defaults = build_inferable_live_defaults_map(&catalog);
        assert_eq!(defaults["YM"]["spot_symbol"], "DIA");
        assert_eq!(defaults["CL"]["options_symbol"], "USO");
    }

    #[test]
    fn parse_live_backend_accepts_supported_values() {
        assert_eq!(parse_live_backend("yfinance").unwrap().as_str(), "yfinance");
        assert_eq!(
            parse_live_backend("external_http_runtime")
                .unwrap()
                .as_str(),
            "external_http_runtime"
        );
        assert_eq!(
            parse_live_backend("crypto_public_runtime")
                .unwrap()
                .as_str(),
            "crypto_public_runtime"
        );
        assert_eq!(
            parse_live_backend("tradingview_mcp").unwrap().as_str(),
            "tradingview_mcp"
        );
        assert_eq!(
            parse_live_backend("tv_mcp").unwrap().as_str(),
            "tradingview_mcp"
        );
    }
}
