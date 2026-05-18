use crate::types::{Candle, Direction, FairValueGap, OrderBlock};

/// Optimal Trade Entry zone finder
pub struct OTE;

impl OTE {
    /// Find OTE zone based on FVG and OB
    pub fn find_ote(
        candles: &[Candle],
        fvgs: &[FairValueGap],
        obs: &[OrderBlock],
        direction: Direction,
    ) -> Option<(f64, f64, String)> {
        let current_price = candles.last()?.close;

        // Find nearest FVG
        let nearest_fvg = fvgs
            .iter()
            .filter(|f| f.direction == direction && !f.filled)
            .min_by(|a, b| {
                let dist_a = ((a.top + a.bottom) / 2.0 - current_price).abs();
                let dist_b = ((b.top + b.bottom) / 2.0 - current_price).abs();
                dist_a.partial_cmp(&dist_b).unwrap()
            });

        // Find nearest OB
        let nearest_ob = obs
            .iter()
            .filter(|o| o.ob_type == direction && !o.tested)
            .min_by(|a, b| {
                let dist_a = ((a.high + a.low) / 2.0 - current_price).abs();
                let dist_b = ((b.high + b.low) / 2.0 - current_price).abs();
                dist_a.partial_cmp(&dist_b).unwrap()
            });

        // Prefer FVG over OB
        if let Some(fvg) = nearest_fvg {
            return Some((
                fvg.bottom,
                fvg.top,
                format!("FVG [{:.2}-{:.2}]", fvg.bottom, fvg.top),
            ));
        }

        if let Some(ob) = nearest_ob {
            return Some((
                ob.low,
                ob.high,
                format!("OB [{:.2}-{:.2}]", ob.low, ob.high),
            ));
        }

        None
    }

    /// Calculate optimal entry price within OTE zone
    pub fn optimal_entry(ote_low: f64, ote_high: f64) -> f64 {
        (ote_low + ote_high) / 2.0
    }
}
