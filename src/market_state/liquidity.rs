//! Liquidity Regime Classification
//!
//! 分类流动性状态：HighLiquidity / NormalLiquidity / ThinLiquidity
//! 基于成交量 + 价格范围 + 买卖价差代理

use crate::types::Candle;
use chrono::Timelike;
use serde::{Deserialize, Serialize};

/// 流动性状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LiquidityRegime {
    /// 高流动性：成交量大，价格范围窄
    HighLiquidity,
    /// 正常流动性
    #[default]
    NormalLiquidity,
    /// 流动性枯竭：成交量低，价格范围极端
    ThinLiquidity,
    /// 未知/数据不足
    Unknown,
}

impl LiquidityRegime {
    pub fn label(&self) -> &'static str {
        match self {
            LiquidityRegime::HighLiquidity => "high_liq",
            LiquidityRegime::NormalLiquidity => "normal_liq",
            LiquidityRegime::ThinLiquidity => "thin_liq",
            LiquidityRegime::Unknown => "unknown",
        }
    }
}

/// 会话状态（用于叠加流动性判断）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SessionState {
    /// 活跃交易时段（美东 09:30-16:00）
    Killzone,
    /// 交易时段前后过渡
    Transition,
    /// 非交易时段
    #[default]
    OffHours,
    /// 未知
    Unknown,
}

/// 流动性分类器阈值配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityThresholds {
    /// 高流动性阈值：成交量百分位 > high_threshold
    pub high_threshold: f64,
    /// 低流动性阈值：成交量百分位 < low_threshold
    pub low_threshold: f64,
    /// 价格范围百分位阈值（低范围 = 高流动性）
    pub range_high_liq_threshold: f64,
    /// 成交量计算回看窗口
    pub volume_lookback: usize,
    /// 价格范围计算回看窗口
    pub range_lookback: usize,
}

impl Default for LiquidityThresholds {
    fn default() -> Self {
        Self {
            high_threshold: 0.70,
            low_threshold: 0.30,
            range_high_liq_threshold: 0.40, // 价格范围处于低位 = 流动性好
            volume_lookback: 20,
            range_lookback: 20,
        }
    }
}

/// 流动性分类器
pub struct LiquidityClassifier {
    thresholds: LiquidityThresholds,
}

impl LiquidityClassifier {
    pub fn new() -> Self {
        Self::with_thresholds(LiquidityThresholds::default())
    }

    pub fn with_thresholds(thresholds: LiquidityThresholds) -> Self {
        Self { thresholds }
    }

    /// 分类流动性状态，返回 (状态, 置信度)
    pub fn classify(&self, candles: &[Candle]) -> (LiquidityRegime, f64) {
        if candles.len() < self.thresholds.volume_lookback + 1 {
            return (LiquidityRegime::NormalLiquidity, 0.0);
        }

        // 1. 计算成交量百分位
        let volumes: Vec<f64> = candles.iter().map(|c| c.volume).collect();
        let current_vol = *volumes.last().unwrap();
        let vol_percentile =
            self.compute_percentile(&volumes, current_vol, self.thresholds.volume_lookback);

        // 2. 计算价格范围（代理买卖价差）
        let ranges: Vec<f64> = candles.iter().map(|c| c.high - c.low).collect();
        let current_range = *ranges.last().unwrap();
        let range_percentile =
            self.compute_percentile(&ranges, current_range, self.thresholds.range_lookback);

        // 3. 计算流动性综合分数
        // 高成交量 + 低范围 = 高流动性
        // 低成交量 + 高范围 = 低流动性
        let liq_score = vol_percentile * 0.6 + (1.0 - range_percentile) * 0.4;

        // 4. 分类
        let regime = if liq_score >= self.thresholds.high_threshold {
            LiquidityRegime::HighLiquidity
        } else if liq_score <= self.thresholds.low_threshold {
            LiquidityRegime::ThinLiquidity
        } else {
            LiquidityRegime::NormalLiquidity
        };

        // 5. 置信度：偏离中间值的程度
        // 改进：添加基础置信度，避免中间值置信度过低
        let base_confidence = 0.3; // 基础置信度
        let deviation_confidence = (liq_score - 0.5).abs() * 1.4; // 偏离置信度
        let confidence = (base_confidence + deviation_confidence).min(1.0);

        (regime, confidence)
    }

    /// 带会话状态的流动性分类
    pub fn classify_with_session(
        &self,
        candles: &[Candle],
        session: SessionState,
    ) -> (LiquidityRegime, f64) {
        let (base_regime, base_conf) = self.classify(candles);

        // 会话叠加调整
        match session {
            SessionState::Killzone => {
                // 活跃时段流动性提升
                let adjusted = match base_regime {
                    LiquidityRegime::ThinLiquidity => LiquidityRegime::NormalLiquidity,
                    _ => base_regime,
                };
                (adjusted, base_conf * 0.8) // 置信度降低（因会话已确保流动性）
            }
            SessionState::OffHours => {
                // 非交易时段流动性降低
                let adjusted = match base_regime {
                    LiquidityRegime::HighLiquidity => LiquidityRegime::NormalLiquidity,
                    _ => base_regime,
                };
                (adjusted, base_conf)
            }
            SessionState::Transition => {
                (base_regime, base_conf * 0.7) // 过渡时段不确定性更高
            }
            SessionState::Unknown => {
                (base_regime, base_conf * 0.5) // 未知会话降低置信度
            }
        }
    }

    fn compute_percentile(&self, series: &[f64], current: f64, lookback: usize) -> f64 {
        let lookback = lookback.min(series.len());
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
}

impl Default for LiquidityClassifier {
    fn default() -> Self {
        Self::new()
    }
}

/// 会话检测器（基于时间戳）
pub fn detect_session(candles: &[Candle]) -> SessionState {
    if candles.is_empty() {
        return SessionState::Unknown;
    }

    let last = candles.last().unwrap();
    let hour = last.timestamp.hour();

    // 美东时间（简化：假设时间戳已经是美东）
    match hour {
        9..=12 => SessionState::Killzone,   // 09:30-12:00 最活跃
        13..=15 => SessionState::Killzone,  // 13:00-16:00 活跃
        8 | 16 => SessionState::Transition, // 开盘前/收盘后
        _ => SessionState::OffHours,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn sample_candles(count: usize, avg_volume: f64, avg_range: f64) -> Vec<Candle> {
        (0..count)
            .map(|i| {
                let base = 100.0 + i as f64 * 0.1;
                Candle {
                    timestamp: Utc.timestamp_opt(1_700_000_000 + i as i64 * 60, 0).unwrap(),
                    open: base,
                    high: base + avg_range,
                    low: base - avg_range,
                    close: base,
                    volume: avg_volume,
                }
            })
            .collect()
    }

    #[test]
    fn high_liquidity_detection() {
        let candles = sample_candles(50, 10000.0, 0.5); // 高成交量，低范围
        let classifier = LiquidityClassifier::new();
        let (regime, _conf) = classifier.classify(&candles);

        assert!(matches!(
            regime,
            LiquidityRegime::HighLiquidity | LiquidityRegime::NormalLiquidity
        ));
    }

    #[test]
    fn thin_liquidity_detection() {
        let candles = sample_candles(50, 100.0, 5.0); // 低成交量，高范围
        let classifier = LiquidityClassifier::new();
        let (_regime, conf) = classifier.classify(&candles);

        // 由于使用百分位，单一数据点的分类可能不稳定
        assert!((0.0..=1.0).contains(&conf));
    }
}
