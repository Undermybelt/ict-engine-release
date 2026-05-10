//! Volatility Regime Classification
//!
//! 分类波动率状态：LowVol / NormalVol / ElevatedVol / CrisisVol
//! 基于 ATR 百分位 + 波动率聚类检测

use crate::types::Candle;
use serde::{Deserialize, Serialize};

/// 波动率状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VolatilityRegime {
    /// 低波动：ATR 处于历史低位（<20% 分位）
    LowVol,
    /// 正常波动：ATR 处于正常区间（20-60% 分位）
    #[default]
    NormalVol,
    /// 高波动：ATR 处于高位（60-90% 分位）
    ElevatedVol,
    /// 危机波动：ATR 处于极端高位（>90% 分位）
    CrisisVol,
    /// 未知/数据不足
    Unknown,
}

impl VolatilityRegime {
    /// 是否为看涨趋势中的波动率（用于辅助判断趋势方向）
    pub fn is_bullish(&self) -> bool {
        // 波动率本身无方向，这里返回 true 作为占位
        // 实际方向由价格趋势决定
        true
    }

    /// 状态标签
    pub fn label(&self) -> &'static str {
        match self {
            VolatilityRegime::LowVol => "low_vol",
            VolatilityRegime::NormalVol => "normal_vol",
            VolatilityRegime::ElevatedVol => "elevated_vol",
            VolatilityRegime::CrisisVol => "crisis_vol",
            VolatilityRegime::Unknown => "unknown",
        }
    }
}

/// 波动率分类器阈值配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolatilityThresholds {
    /// 低波动阈值：ATR 百分位 < low_threshold
    pub low_threshold: f64,
    /// 正常波动上限
    pub normal_threshold: f64,
    /// 高波动上限
    pub elevated_threshold: f64,
    /// 危机阈值：ATR 百分位 > crisis_threshold
    pub crisis_threshold: f64,
    /// ATR 计算周期
    pub atr_period: usize,
    /// 百分位计算回看窗口
    pub percentile_lookback: usize,
    /// 波动率聚类检测窗口
    pub clustering_window: usize,
}

impl Default for VolatilityThresholds {
    fn default() -> Self {
        Self {
            low_threshold: 0.20,
            normal_threshold: 0.60,
            elevated_threshold: 0.90,
            crisis_threshold: 0.95,
            atr_period: 14,
            percentile_lookback: 252, // 约1年交易日
            clustering_window: 20,
        }
    }
}

/// 波动率分类器
pub struct VolatilityClassifier {
    thresholds: VolatilityThresholds,
}

impl VolatilityClassifier {
    /// 默认配置
    pub fn new() -> Self {
        Self::with_thresholds(VolatilityThresholds::default())
    }

    /// 自定义阈值
    pub fn with_thresholds(thresholds: VolatilityThresholds) -> Self {
        Self { thresholds }
    }

    /// 分类波动率状态，返回 (状态, 置信度)
    pub fn classify(&self, candles: &[Candle]) -> (VolatilityRegime, f64) {
        if candles.len() < self.thresholds.atr_period + 2 {
            return (VolatilityRegime::Unknown, 0.0);
        }

        // 1. 计算 ATR 序列
        let atr = self.compute_atr(candles);
        if atr.is_empty() {
            return (VolatilityRegime::Unknown, 0.0);
        }

        // 2. 计算当前 ATR 的历史百分位
        let current_atr = *atr.last().unwrap();
        let percentile = self.compute_percentile(&atr, current_atr);

        // 3. 计算波动率聚类度（连续高/低波动的持续性）
        let clustering_score = self.compute_clustering(&atr);

        // 4. 分类
        let regime = if percentile >= self.thresholds.crisis_threshold {
            VolatilityRegime::CrisisVol
        } else if percentile >= self.thresholds.normal_threshold {
            VolatilityRegime::ElevatedVol
        } else if percentile >= self.thresholds.low_threshold {
            VolatilityRegime::NormalVol
        } else {
            VolatilityRegime::LowVol
        };

        // 5. 置信度 = 百分位确定性 + 聚类持续性
        // 改进：添加基础置信度，避免中间值置信度过低
        let base_confidence = 0.35;
        let percentile_confidence = if matches!(
            regime,
            VolatilityRegime::CrisisVol | VolatilityRegime::LowVol
        ) {
            // 极端值置信度更高
            (percentile.abs() - 0.5).abs() * 1.5
        } else {
            // 正常值
            0.3 + (percentile - 0.5).abs()
        };

        let confidence =
            (base_confidence + percentile_confidence * 0.5 + clustering_score * 0.25).min(1.0);

        (regime, confidence)
    }

    /// 计算 ATR 序列
    fn compute_atr(&self, candles: &[Candle]) -> Vec<f64> {
        let period = self.thresholds.atr_period;
        if candles.len() < period + 1 {
            return Vec::new();
        }

        let mut tr = Vec::with_capacity(candles.len() - 1);
        for i in 1..candles.len() {
            let high = candles[i].high;
            let low = candles[i].low;
            let prev_close = candles[i - 1].close;

            let tr_val = (high - low)
                .max((high - prev_close).abs())
                .max((low - prev_close).abs());
            tr.push(tr_val);
        }

        // EMA 平滑
        let multiplier = 2.0 / (period as f64 + 1.0);
        let mut atr = vec![tr[0]];
        for i in 1..tr.len() {
            let atr_val = tr[i] * multiplier + atr[i - 1] * (1.0 - multiplier);
            atr.push(atr_val);
        }

        atr
    }

    /// 计算当前值在历史序列中的百分位
    fn compute_percentile(&self, series: &[f64], current: f64) -> f64 {
        let lookback = self.thresholds.percentile_lookback.min(series.len());
        if lookback == 0 {
            return 0.5;
        }

        let slice = &series[series.len() - lookback..];
        let count_below = slice.iter().filter(|&&x| x < current).count();
        let count_equal = slice
            .iter()
            .filter(|&&x| (x - current).abs() < 1e-10)
            .count();

        (count_below as f64 + count_equal as f64 * 0.5) / slice.len() as f64
    }

    /// 计算波动率聚类度：连续同向变化的持续性
    fn compute_clustering(&self, atr: &[f64]) -> f64 {
        let window = self.thresholds.clustering_window.min(atr.len());
        if window < 2 {
            return 0.0;
        }

        let recent = &atr[atr.len() - window..];
        let mut direction_changes = 0;
        let mut prev_direction = 0i32;

        for i in 1..recent.len() {
            let direction = if recent[i] > recent[i - 1] {
                1
            } else if recent[i] < recent[i - 1] {
                -1
            } else {
                0
            };
            if prev_direction != 0 && direction != 0 && direction != prev_direction {
                direction_changes += 1;
            }
            if direction != 0 {
                prev_direction = direction;
            }
        }

        // 方向变化越少，聚类度越高
        let max_changes = window as f64 / 2.0;
        1.0 - (direction_changes as f64 / max_changes).min(1.0)
    }
}

impl Default for VolatilityClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn sample_candles(count: usize, volatility: f64) -> Vec<Candle> {
        (0..count)
            .map(|i| {
                let base = 100.0 + i as f64 * 0.1;
                let noise = volatility * (i as f64 % 10.0 - 5.0);
                Candle {
                    timestamp: Utc.timestamp_opt(1_700_000_000 + i as i64 * 60, 0).unwrap(),
                    open: base,
                    high: base + volatility + noise.abs(),
                    low: base - volatility - noise.abs(),
                    close: base + noise * 0.5,
                    volume: 1000.0,
                }
            })
            .collect()
    }

    #[test]
    fn low_volatility_detection() {
        let candles = sample_candles(300, 0.1); // 低波动
        let classifier = VolatilityClassifier::new();
        let (regime, conf) = classifier.classify(&candles);

        assert!(matches!(
            regime,
            VolatilityRegime::LowVol | VolatilityRegime::NormalVol
        ));
        assert!((0.0..=1.0).contains(&conf));
    }

    #[test]
    fn high_volatility_detection() {
        let candles = sample_candles(300, 5.0); // 高波动
        let classifier = VolatilityClassifier::new();
        let (regime, conf) = classifier.classify(&candles);

        assert!(matches!(
            regime,
            VolatilityRegime::ElevatedVol
                | VolatilityRegime::CrisisVol
                | VolatilityRegime::NormalVol
        ));
        assert!((0.0..=1.0).contains(&conf));
    }
}
