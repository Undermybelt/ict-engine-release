use crate::data::realtime::LiveDataBackend;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalyzeLiveSymbolDefaults {
    pub futures_symbol: &'static str,
    pub spot_symbol: &'static str,
    pub options_symbol: &'static str,
    pub spot_kind: &'static str,
}

pub fn resolve_live_backend_base_url(
    backend: &str,
    openalice_base_url: &str,
    nofx_base_url: &str,
) -> String {
    match backend.trim().to_ascii_lowercase().as_str() {
        "openbb" => "native://openbb".to_string(),
        "openalice" => openalice_base_url.to_string(),
        "nofx" => nofx_base_url.to_string(),
        _ => "native://openbb".to_string(),
    }
}

pub fn analyze_live_inferred_symbols(symbol: &str) -> Option<AnalyzeLiveSymbolDefaults> {
    match symbol.to_ascii_uppercase().as_str() {
        "NQ" => Some(AnalyzeLiveSymbolDefaults {
            futures_symbol: "NQ=F",
            spot_symbol: "QQQ",
            options_symbol: "QQQ",
            spot_kind: "equity",
        }),
        "ES" => Some(AnalyzeLiveSymbolDefaults {
            futures_symbol: "ES=F",
            spot_symbol: "SPY",
            options_symbol: "SPY",
            spot_kind: "equity",
        }),
        "YM" => Some(AnalyzeLiveSymbolDefaults {
            futures_symbol: "YM=F",
            spot_symbol: "DIA",
            options_symbol: "DIA",
            spot_kind: "equity",
        }),
        "GC" => Some(AnalyzeLiveSymbolDefaults {
            futures_symbol: "GC=F",
            spot_symbol: "GLD",
            options_symbol: "GLD",
            spot_kind: "etf",
        }),
        "CL" => Some(AnalyzeLiveSymbolDefaults {
            futures_symbol: "CL=F",
            spot_symbol: "USO",
            options_symbol: "USO",
            spot_kind: "etf",
        }),
        _ => None,
    }
}

pub fn parse_live_backend(backend: &str) -> anyhow::Result<LiveDataBackend> {
    LiveDataBackend::parse(backend)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_live_backend_base_url_uses_expected_sources() {
        assert_eq!(
            resolve_live_backend_base_url("openbb", "http://oa", "http://nofx"),
            "native://openbb"
        );
        assert_eq!(
            resolve_live_backend_base_url("openalice", "http://oa", "http://nofx"),
            "http://oa"
        );
        assert_eq!(
            resolve_live_backend_base_url("nofx", "http://oa", "http://nofx"),
            "http://nofx"
        );
        assert_eq!(
            resolve_live_backend_base_url("unknown", "http://oa", "http://nofx"),
            "native://openbb"
        );
    }

    #[test]
    fn analyze_live_symbol_can_infer_gc_and_cl_defaults() {
        let gc = analyze_live_inferred_symbols("GC").unwrap();
        let cl = analyze_live_inferred_symbols("CL").unwrap();
        assert_eq!(gc.futures_symbol, "GC=F");
        assert_eq!(gc.spot_symbol, "GLD");
        assert_eq!(cl.futures_symbol, "CL=F");
        assert_eq!(cl.spot_symbol, "USO");
    }

    #[test]
    fn parse_live_backend_accepts_supported_values() {
        assert_eq!(parse_live_backend("openbb").unwrap().as_str(), "openbb");
        assert_eq!(
            parse_live_backend("openalice").unwrap().as_str(),
            "openalice"
        );
        assert_eq!(parse_live_backend("nofx").unwrap().as_str(), "nofx");
    }
}
