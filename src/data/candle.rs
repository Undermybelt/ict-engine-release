use crate::types::Candle;

impl Candle {
    pub fn true_range(&self, prev_candle: &Candle) -> f64 {
        let hl = self.high - self.low;
        let hc = (self.high - prev_candle.close).abs();
        let lc = (self.low - prev_candle.close).abs();
        hl.max(hc).max(lc)
    }

    pub fn body(&self) -> f64 {
        (self.close - self.open).abs()
    }

    pub fn body_top(&self) -> f64 {
        self.open.max(self.close)
    }

    pub fn body_bottom(&self) -> f64 {
        self.open.min(self.close)
    }

    pub fn upper_wick(&self) -> f64 {
        self.high - self.body_top()
    }

    pub fn lower_wick(&self) -> f64 {
        self.body_bottom() - self.low
    }

    pub fn range(&self) -> f64 {
        self.high - self.low
    }

    pub fn is_bullish(&self) -> bool {
        self.close > self.open
    }

    pub fn is_bearish(&self) -> bool {
        self.close < self.open
    }

    pub fn midpoint(&self) -> f64 {
        (self.high + self.low) / 2.0
    }
}
