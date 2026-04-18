use crate::types::Candle;
use crate::types::{CascadeLayer, CascadeResult, CascadeStep, Direction};

/// 7-Layer Cascade Decision Tree Configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CascadeConfig {
    pub p0: f64,
    pub lr1: f64,
    pub lr2: f64,
    pub lr3: f64,
    pub lr4: f64,
    pub lr5: f64,
    pub lr6: f64,
    pub lr7: f64,
}

impl Default for CascadeConfig {
    fn default() -> Self {
        Self {
            p0: 0.15,
            lr1: 1.3,
            lr2: 2.5,
            lr3: 4.0,
            lr4: 1.8,
            lr5: 1.5,
            lr6: 4.0,
            lr7: 7.0,
        }
    }
}

/// Run bullish cascade
pub fn cascade_bull(
    htf: &[Candle],
    mtf: &[Candle],
    ltf: &[Candle],
    config: &CascadeConfig,
    _atr_htf: &[f64],
    atr_ltf: &[f64],
) -> CascadeResult {
    let mut steps = Vec::new();
    let mut prior = config.p0;

    // Layer 1: HTF trend alignment
    let l1_satisfied = check_htf_trend_bull(htf);
    let lr1 = if l1_satisfied { config.lr1 } else { 1.0 };
    prior = update_posterior(prior, lr1);
    steps.push(CascadeStep {
        layer: CascadeLayer::L1,
        satisfied: l1_satisfied,
        lr: lr1,
        prior: prior / lr1,
        posterior: prior,
        description: "HTF Trend Alignment".to_string(),
    });

    // Layer 2: MTF structure
    let l2_satisfied = check_mtf_structure_bull(mtf);
    let lr2 = if l2_satisfied { config.lr2 } else { 1.0 };
    prior = update_posterior(prior, lr2);
    steps.push(CascadeStep {
        layer: CascadeLayer::L2,
        satisfied: l2_satisfied,
        lr: lr2,
        prior: prior / lr2,
        posterior: prior,
        description: "MTF Structure".to_string(),
    });

    // Layer 3: Liquidity sweep confirmation
    let l3_satisfied = check_liquidity_sweep_bull(mtf);
    let lr3 = if l3_satisfied { config.lr3 } else { 1.0 };
    prior = update_posterior(prior, lr3);
    steps.push(CascadeStep {
        layer: CascadeLayer::L3,
        satisfied: l3_satisfied,
        lr: lr3,
        prior: prior / lr3,
        posterior: prior,
        description: "Liquidity Sweep".to_string(),
    });

    // Layer 4: FVG confirmation
    let l4_satisfied = check_fvg_bull(mtf);
    let lr4 = if l4_satisfied { config.lr4 } else { 1.0 };
    prior = update_posterior(prior, lr4);
    steps.push(CascadeStep {
        layer: CascadeLayer::L4,
        satisfied: l4_satisfied,
        lr: lr4,
        prior: prior / lr4,
        posterior: prior,
        description: "FVG Confirmation".to_string(),
    });

    // Layer 5: Order Block touch
    let l5_satisfied = check_ob_touch_bull(mtf);
    let lr5 = if l5_satisfied { config.lr5 } else { 1.0 };
    prior = update_posterior(prior, lr5);
    steps.push(CascadeStep {
        layer: CascadeLayer::L5,
        satisfied: l5_satisfied,
        lr: lr5,
        prior: prior / lr5,
        posterior: prior,
        description: "Order Block Touch".to_string(),
    });

    // Layer 6: CISD confirmation
    let l6_satisfied = check_cisd_bull(ltf);
    let lr6 = if l6_satisfied { config.lr6 } else { 1.0 };
    prior = update_posterior(prior, lr6);
    steps.push(CascadeStep {
        layer: CascadeLayer::L6,
        satisfied: l6_satisfied,
        lr: lr6,
        prior: prior / lr6,
        posterior: prior,
        description: "CISD Confirmation".to_string(),
    });

    // Layer 7: Entry trigger (pinbar/rejection block)
    let l7_satisfied = check_entry_trigger_bull(ltf, atr_ltf);
    let lr7 = if l7_satisfied { config.lr7 } else { 1.0 };
    prior = update_posterior(prior, lr7);
    steps.push(CascadeStep {
        layer: CascadeLayer::L7,
        satisfied: l7_satisfied,
        lr: lr7,
        prior: prior / lr7,
        posterior: prior,
        description: "Entry Trigger".to_string(),
    });

    // Find where cascade stopped
    let stopped_at = steps.iter().find(|s| !s.satisfied).map(|s| s.layer);

    CascadeResult {
        direction: Direction::Bull,
        stopped_at,
        steps,
        final_posterior: prior,
    }
}

/// Run bearish cascade
pub fn cascade_bear(
    htf: &[Candle],
    mtf: &[Candle],
    ltf: &[Candle],
    config: &CascadeConfig,
    _atr_htf: &[f64],
    atr_ltf: &[f64],
) -> CascadeResult {
    let mut steps = Vec::new();
    let mut prior = config.p0;

    // Layer 1: HTF trend alignment (bearish)
    let l1_satisfied = check_htf_trend_bear(htf);
    let lr1 = if l1_satisfied { config.lr1 } else { 1.0 };
    prior = update_posterior(prior, lr1);
    steps.push(CascadeStep {
        layer: CascadeLayer::L1,
        satisfied: l1_satisfied,
        lr: lr1,
        prior: prior / lr1,
        posterior: prior,
        description: "HTF Trend Alignment".to_string(),
    });

    // Layer 2: MTF structure (bearish)
    let l2_satisfied = check_mtf_structure_bear(mtf);
    let lr2 = if l2_satisfied { config.lr2 } else { 1.0 };
    prior = update_posterior(prior, lr2);
    steps.push(CascadeStep {
        layer: CascadeLayer::L2,
        satisfied: l2_satisfied,
        lr: lr2,
        prior: prior / lr2,
        posterior: prior,
        description: "MTF Structure".to_string(),
    });

    // Layer 3: Liquidity sweep confirmation (bearish)
    let l3_satisfied = check_liquidity_sweep_bear(mtf);
    let lr3 = if l3_satisfied { config.lr3 } else { 1.0 };
    prior = update_posterior(prior, lr3);
    steps.push(CascadeStep {
        layer: CascadeLayer::L3,
        satisfied: l3_satisfied,
        lr: lr3,
        prior: prior / lr3,
        posterior: prior,
        description: "Liquidity Sweep".to_string(),
    });

    // Layer 4: FVG confirmation (bearish)
    let l4_satisfied = check_fvg_bear(mtf);
    let lr4 = if l4_satisfied { config.lr4 } else { 1.0 };
    prior = update_posterior(prior, lr4);
    steps.push(CascadeStep {
        layer: CascadeLayer::L4,
        satisfied: l4_satisfied,
        lr: lr4,
        prior: prior / lr4,
        posterior: prior,
        description: "FVG Confirmation".to_string(),
    });

    // Layer 5: Order Block touch (bearish)
    let l5_satisfied = check_ob_touch_bear(mtf);
    let lr5 = if l5_satisfied { config.lr5 } else { 1.0 };
    prior = update_posterior(prior, lr5);
    steps.push(CascadeStep {
        layer: CascadeLayer::L5,
        satisfied: l5_satisfied,
        lr: lr5,
        prior: prior / lr5,
        posterior: prior,
        description: "Order Block Touch".to_string(),
    });

    // Layer 6: CISD confirmation (bearish)
    let l6_satisfied = check_cisd_bear(ltf);
    let lr6 = if l6_satisfied { config.lr6 } else { 1.0 };
    prior = update_posterior(prior, lr6);
    steps.push(CascadeStep {
        layer: CascadeLayer::L6,
        satisfied: l6_satisfied,
        lr: lr6,
        prior: prior / lr6,
        posterior: prior,
        description: "CISD Confirmation".to_string(),
    });

    // Layer 7: Entry trigger (bearish pinbar)
    let l7_satisfied = check_entry_trigger_bear(ltf, atr_ltf);
    let lr7 = if l7_satisfied { config.lr7 } else { 1.0 };
    prior = update_posterior(prior, lr7);
    steps.push(CascadeStep {
        layer: CascadeLayer::L7,
        satisfied: l7_satisfied,
        lr: lr7,
        prior: prior / lr7,
        posterior: prior,
        description: "Entry Trigger".to_string(),
    });

    // Find where cascade stopped
    let stopped_at = steps.iter().find(|s| !s.satisfied).map(|s| s.layer);

    CascadeResult {
        direction: Direction::Bear,
        stopped_at,
        steps,
        final_posterior: prior,
    }
}

fn update_posterior(prior: f64, lr: f64) -> f64 {
    let posterior = prior * lr / (prior * lr + (1.0 - prior));
    posterior.clamp(0.001, 0.999)
}

// Layer check functions (simplified implementations)
fn check_htf_trend_bull(htf: &[Candle]) -> bool {
    if htf.len() < 10 {
        return false;
    }
    let recent = &htf[htf.len() - 10..];
    let up_moves = recent
        .windows(2)
        .filter(|w| w[1].close > w[0].close)
        .count();
    up_moves >= 6
}

fn check_htf_trend_bear(htf: &[Candle]) -> bool {
    if htf.len() < 10 {
        return false;
    }
    let recent = &htf[htf.len() - 10..];
    let down_moves = recent
        .windows(2)
        .filter(|w| w[1].close < w[0].close)
        .count();
    down_moves >= 6
}

fn check_mtf_structure_bull(mtf: &[Candle]) -> bool {
    if mtf.len() < 5 {
        return false;
    }
    let recent_highs: Vec<f64> = mtf[mtf.len() - 5..].iter().map(|c| c.high).collect();
    recent_highs.windows(2).all(|w| w[1] >= w[0])
}

fn check_mtf_structure_bear(mtf: &[Candle]) -> bool {
    if mtf.len() < 5 {
        return false;
    }
    let recent_lows: Vec<f64> = mtf[mtf.len() - 5..].iter().map(|c| c.low).collect();
    recent_lows.windows(2).all(|w| w[1] <= w[0])
}

fn check_liquidity_sweep_bull(mtf: &[Candle]) -> bool {
    // Simplified: check if recent candle swept below previous low then closed higher
    if mtf.len() < 3 {
        return false;
    }
    let curr = &mtf[mtf.len() - 1];
    let prev = &mtf[mtf.len() - 2];
    curr.low < prev.low && curr.close > prev.close
}

fn check_liquidity_sweep_bear(mtf: &[Candle]) -> bool {
    if mtf.len() < 3 {
        return false;
    }
    let curr = &mtf[mtf.len() - 1];
    let prev = &mtf[mtf.len() - 2];
    curr.high > prev.high && curr.close < prev.close
}

fn check_fvg_bull(mtf: &[Candle]) -> bool {
    // Check for bullish FVG in recent candles
    if mtf.len() < 3 {
        return false;
    }
    for i in 1..mtf.len() - 1 {
        if mtf[i + 1].low > mtf[i - 1].high {
            return true;
        }
    }
    false
}

fn check_fvg_bear(mtf: &[Candle]) -> bool {
    if mtf.len() < 3 {
        return false;
    }
    for i in 1..mtf.len() - 1 {
        if mtf[i + 1].high < mtf[i - 1].low {
            return true;
        }
    }
    false
}

fn check_ob_touch_bull(mtf: &[Candle]) -> bool {
    // Check if price touched an order block area
    if mtf.len() < 5 {
        return false;
    }
    let curr = &mtf[mtf.len() - 1];
    for i in (mtf.len() - 5..mtf.len() - 1).rev() {
        if mtf[i].is_bearish() && curr.low <= mtf[i].high && curr.high >= mtf[i].low {
            return true;
        }
    }
    false
}

fn check_ob_touch_bear(mtf: &[Candle]) -> bool {
    if mtf.len() < 5 {
        return false;
    }
    let curr = &mtf[mtf.len() - 1];
    for i in (mtf.len() - 5..mtf.len() - 1).rev() {
        if mtf[i].is_bullish() && curr.high >= mtf[i].low && curr.low <= mtf[i].high {
            return true;
        }
    }
    false
}

fn check_cisd_bull(ltf: &[Candle]) -> bool {
    if ltf.len() < 3 {
        return false;
    }
    let prev2 = &ltf[ltf.len() - 3];
    let prev1 = &ltf[ltf.len() - 2];
    let curr = &ltf[ltf.len() - 1];
    prev2.is_bearish() && prev1.is_bearish() && curr.is_bullish()
}

fn check_cisd_bear(ltf: &[Candle]) -> bool {
    if ltf.len() < 3 {
        return false;
    }
    let prev2 = &ltf[ltf.len() - 3];
    let prev1 = &ltf[ltf.len() - 2];
    let curr = &ltf[ltf.len() - 1];
    prev2.is_bullish() && prev1.is_bullish() && curr.is_bearish()
}

fn check_entry_trigger_bull(ltf: &[Candle], atr: &[f64]) -> bool {
    if ltf.is_empty() || atr.is_empty() {
        return false;
    }
    let curr = &ltf[ltf.len() - 1];
    let atr_val = atr.last().unwrap();

    // Check for bullish pinbar
    curr.body() < curr.lower_wick() && curr.range() > atr_val * 1.5
}

fn check_entry_trigger_bear(ltf: &[Candle], atr: &[f64]) -> bool {
    if ltf.is_empty() || atr.is_empty() {
        return false;
    }
    let curr = &ltf[ltf.len() - 1];
    let atr_val = atr.last().unwrap();

    // Check for bearish pinbar
    curr.body() < curr.upper_wick() && curr.range() > atr_val * 1.5
}
