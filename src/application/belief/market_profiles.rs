use crate::application::belief::infer_market_from_symbol;

pub fn market_category_for_symbol(symbol: &str) -> Option<&'static str> {
    let inferred = infer_market_from_symbol(symbol);
    let compact = inferred.replace(['/', '='], "");
    match compact.as_str() {
        "ES" | "NQ" | "RTY" | "YM" | "SPY" | "QQQ" | "IWM" | "DIA" => Some("futures_index"),
        "GC" | "SI" | "HG" | "GLD" | "SLV" | "CPER" => Some("metals"),
        "CL" | "NG" | "RB" | "USO" | "UNG" | "BNO" => Some("energy"),
        "BTC" | "BTCUSD" | "BTCUSDT" | "ETH" | "ETHUSD" | "ETHUSDT" | "SOL" | "SOLUSD"
        | "SOLUSDT" | "BNB" | "BNBUSD" | "BNBUSDT" | "AVAX" | "AVAXUSD" | "AVAXUSDT" => {
            Some("crypto")
        }
        "EURUSD" | "EURUSDX" | "GBPUSD" | "GBPUSDX" | "USDJPY" | "USDJPYX" | "EURJPY"
        | "EURJPYX" | "AUDUSD" | "AUDUSDX" => Some("fx"),
        _ => None,
    }
}

pub fn market_behavior_profile_for_family(category: &str) -> &'static str {
    match category {
        "futures_index" => "index_beta_regime_sensitive",
        "metals" => "metals_defensive_liquidity_sensitive",
        "energy" => "energy_volatility_shock_sensitive",
        "crypto" => "crypto_trend_volatility_sensitive",
        "fx" => "fx_macro_trend_liquidity_sensitive",
        _ => "generic",
    }
}
