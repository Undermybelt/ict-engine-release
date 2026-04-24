use crate::application::belief::infer_market_from_symbol;

pub fn market_category_for_symbol(symbol: &str) -> Option<&'static str> {
    match infer_market_from_symbol(symbol).as_str() {
        "ES" | "NQ" | "RTY" | "YM" => Some("futures_index"),
        "GC" | "SI" | "HG" => Some("metals"),
        "CL" | "NG" | "RB" => Some("energy"),
        _ => None,
    }
}

pub fn market_behavior_profile_for_family(category: &str) -> &'static str {
    match category {
        "futures_index" => "index_beta_regime_sensitive",
        "metals" => "metals_defensive_liquidity_sensitive",
        "energy" => "energy_volatility_shock_sensitive",
        _ => "generic",
    }
}
