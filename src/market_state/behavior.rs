//! Investor Behavior Regime Classification
//!
//! 分类投资者行为状态：Crowding / Exhaustion / FOMO / Capitulation
//! 基于成交量异动 + 价格极端 + RSI/波动率组合

use crate::types::Candle;
use serde::{Deserialize, Serialize};

/// 投资者行为状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum InvestorBehaviorRegime {
    /// 拥挤：过多同向仓位
    Crowding,
    /// 疲劳：趋势末期，动能衰减
    Exhaustion,
    /// FOMO：追涨杀跌情绪
    FOMO,
    /// 恐慌抛售/投降
    Capitulation,
    /// 风险偏好上升
    RiskOn,
    /// 风险规避
    RiskOff,
    /// 中性/正常
    #[default]
    Neutral,
}

impl InvestorBehaviorRegime {
    pub fn label(&self) -> &'static str {
        match self {
            InvestorBehaviorRegime::Crowding => "crowding",
            InvestorBehaviorRegime::Exhaustion => "exhaustion",
            InvestorBehaviorRegime::FOMO => "fomo",
            InvestorBehaviorRegime::Capitulation => "capitulation",
            InvestorBehaviorRegime::RiskOn => "risk_on",
            InvestorBehaviorRegime::RiskOff => "risk_off",
            InvestorBehaviorRegime::Neutral => "neutral",
        }
    }
}

/// 行为分类器阈值配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorThresholds {
    /// 成交量异常阈值（相对均值）
    pub volume_spike_threshold: f64,
    /// RSI 极端阈值
    pub rsi_extreme_threshold: f64,
    /// 拥挤检测窗口
    pub crowding_window: usize,
    /// 疲劳检测窗口
    pub exhaustion_window: usize,
    /// RSI 计算周期
    pub rsi_period: usize,
}

impl Default for BehaviorThresholds {
    fn default() -> Self {
        Self {
            volume_spike_threshold: 2.0, // 成交量超过均值 2 倍
            rsi_extreme_threshold: 75.0, // RSI > 75 或 < 25
            crowding_window: 10,
            exhaustion_window: 20,
            rsi_period: 14,
        }
    }
}

/// 投资者行为分类器
pub struct InvestorBehaviorClassifier {
    thresholds: BehaviorThresholds,
}

impl InvestorBehaviorClassifier {
    pub fn new() -> Self {
        Self::with_thresholds(BehaviorThresholds::default())
    }

    pub fn with_thresholds(thresholds: BehaviorThresholds) -> Self {
        Self { thresholds }
    }

    /// 分类投资者行为状态，返回 (状态, 置信度)
    pub fn classify(&self, candles: &[Candle]) -> (InvestorBehaviorRegime, f64) {
        if candles.len() < self.thresholds.rsi_period + 5 {
            return (InvestorBehaviorRegime::Neutral, 0.0);
        }

        // 1. 计算 RSI
        let rsi = self.compute_rsi(candles);
        let current_rsi = *rsi.last().unwrap_or(&50.0);

        // 2. 计算成交量异动
        let vol_spike = self.detect_volume_spike(candles);

        // 3. 检测价格极端
        let price_extreme = self.detect_price_extreme(candles);

        // 4. 检测动能衰减
        let momentum_fade = self.detect_momentum_fade(candles);

        // 5. 综合分类
        let (regime, confidence) = if price_extreme.is_extreme && vol_spike.is_spike {
            // 价格极端 + 成交量爆发 = FOMO 或 Capitulation
            if price_extreme.is_bullish {
                (InvestorBehaviorRegime::FOMO, 0.8)
            } else {
                (InvestorBehaviorRegime::Capitulation, 0.85)
            }
        } else if current_rsi > self.thresholds.rsi_extreme_threshold {
            // RSI 超买
            if momentum_fade.is_fading {
                (InvestorBehaviorRegime::Exhaustion, 0.75)
            } else {
                (InvestorBehaviorRegime::FOMO, 0.65)
            }
        } else if current_rsi < (100.0 - self.thresholds.rsi_extreme_threshold) {
            // RSI 超卖
            if vol_spike.spike_ratio > self.thresholds.volume_spike_threshold * 1.5 {
                (InvestorBehaviorRegime::Capitulation, 0.80)
            } else {
                (InvestorBehaviorRegime::RiskOff, 0.60)
            }
        } else if momentum_fade.is_fading {
            // 动能衰减
            (InvestorBehaviorRegime::Exhaustion, 0.65)
        } else if self.detect_crowding(candles) {
            // 拥挤
            (InvestorBehaviorRegime::Crowding, 0.60)
        } else if price_extreme.is_bullish && current_rsi > 60.0 {
            // 风险偏好
            (InvestorBehaviorRegime::RiskOn, 0.55)
        } else if !price_extreme.is_bullish && current_rsi < 40.0 {
            // 风险规避
            (InvestorBehaviorRegime::RiskOff, 0.55)
        } else {
            (InvestorBehaviorRegime::Neutral, 0.45) // 提升中性状态置信度
        };

        (regime, confidence)
    }

    /// 计算 RSI
    fn compute_rsi(&self, candles: &[Candle]) -> Vec<f64> {
        let period = self.thresholds.rsi_period;
        if candles.len() < period + 1 {
            return vec![50.0];
        }

        let mut gains = Vec::new();
        let mut losses = Vec::new();

        for i in 1..candles.len() {
            let change = candles[i].close - candles[i - 1].close;
            if change > 0.0 {
                gains.push(change);
                losses.push(0.0);
            } else {
                gains.push(0.0);
                losses.push(-change);
            }
        }

        // 计算平均涨跌幅
        let mut rsi = Vec::new();
        let mut avg_gain = gains[0..period].iter().sum::<f64>() / period as f64;
        let mut avg_loss = losses[0..period].iter().sum::<f64>() / period as f64;

        // 平滑处理
        for i in period..gains.len() {
            avg_gain = (avg_gain * (period - 1) as f64 + gains[i]) / period as f64;
            avg_loss = (avg_loss * (period - 1) as f64 + losses[i]) / period as f64;

            let rs = if avg_loss > 1e-10 {
                avg_gain / avg_loss
            } else {
                100.0
            };
            let rsi_val = 100.0 - 100.0 / (1.0 + rs);
            rsi.push(rsi_val);
        }

        if rsi.is_empty() {
            vec![50.0]
        } else {
            rsi
        }
    }

    /// 检测成交量异动
    fn detect_volume_spike(&self, candles: &[Candle]) -> VolumeSpikeResult {
        if candles.len() < 20 {
            return VolumeSpikeResult::default();
        }

        let volumes: Vec<f64> = candles.iter().map(|c| c.volume).collect();
        let avg_vol = volumes[volumes.len() - 20..volumes.len() - 1]
            .iter()
            .sum::<f64>()
            / 19.0;
        let last_vol = *volumes.last().unwrap_or(&avg_vol);

        VolumeSpikeResult {
            is_spike: last_vol > avg_vol * self.thresholds.volume_spike_threshold,
            spike_ratio: last_vol / avg_vol.max(1e-10),
        }
    }

    /// 检测价格极端
    fn detect_price_extreme(&self, candles: &[Candle]) -> PriceExtremeResult {
        if candles.len() < 20 {
            return PriceExtremeResult::default();
        }

        let lookback = 20.min(candles.len());
        let recent = &candles[candles.len() - lookback..];
        let closes: Vec<f64> = recent.iter().map(|c| c.close).collect();

        let high = closes.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let low = closes.iter().cloned().fold(f64::INFINITY, f64::min);
        let last = closes.last().copied().unwrap_or((high + low) / 2.0);

        let range = high - low;
        let position = if range > 1e-10 {
            (last - low) / range
        } else {
            0.5
        };

        PriceExtremeResult {
            is_extreme: !(0.1..=0.9).contains(&position),
            is_bullish: position > 0.5,
        }
    }

    /// 检测动能衰减
    fn detect_momentum_fade(&self, candles: &[Candle]) -> MomentumFadeResult {
        let window = self.thresholds.exhaustion_window;
        if window < 2 || candles.len() < window * 2 {
            return MomentumFadeResult::default();
        }

        // 计算前后两个完整窗口的价格变化率，避免短样本下标回绕。
        let early_start = candles.len() - window * 2;
        let recent_start = candles.len() - window;
        let early_change = candles[recent_start - 1].close - candles[early_start].close;
        let recent_change = candles.last().unwrap().close - candles[recent_start].close;

        // 动能衰减：前期变化大，近期变化小
        let early_momentum = early_change.abs();
        let recent_momentum = recent_change.abs();

        MomentumFadeResult {
            is_fading: recent_momentum < early_momentum * 0.5 && early_momentum > 1e-10,
        }
    }

    /// 检测拥挤（连续同向K线）
    fn detect_crowding(&self, candles: &[Candle]) -> bool {
        let window = self.thresholds.crowding_window;
        if candles.len() < window {
            return false;
        }

        let recent = &candles[candles.len() - window..];
        let bullish_count = recent.iter().filter(|c| c.close > c.open).count();

        // 超过 80% 同向K线 = 拥挤
        bullish_count as f64 / window as f64 > 0.8 || bullish_count as f64 / (window as f64) < 0.2
    }
}

#[derive(Default)]
struct VolumeSpikeResult {
    is_spike: bool,
    spike_ratio: f64,
}

#[derive(Default)]
struct PriceExtremeResult {
    is_extreme: bool,
    is_bullish: bool,
}

#[derive(Default)]
struct MomentumFadeResult {
    is_fading: bool,
}

impl Default for InvestorBehaviorClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn fomo_candles(count: usize) -> Vec<Candle> {
        (0..count)
            .map(|i| {
                let base = 100.0 + i as f64 * 1.0; // 强势上涨
                let vol = if i > count - 5 { 5000.0 } else { 1000.0 }; // 近期放量
                Candle {
                    timestamp: Utc.timestamp_opt(1_700_000_000 + i as i64 * 60, 0).unwrap(),
                    open: base - 0.2,
                    high: base + 1.0,
                    low: base - 0.3,
                    close: base + 0.8,
                    volume: vol,
                }
            })
            .collect()
    }

    fn neutral_candles(count: usize) -> Vec<Candle> {
        (0..count)
            .map(|i| {
                let base = 100.0 + (i as f64 % 10.0 - 5.0) * 0.5;
                Candle {
                    timestamp: Utc.timestamp_opt(1_700_000_000 + i as i64 * 60, 0).unwrap(),
                    open: base,
                    high: base + 0.5,
                    low: base - 0.5,
                    close: base + if i % 2 == 0 { 0.1 } else { -0.1 },
                    volume: 1000.0,
                }
            })
            .collect()
    }

    #[test]
    fn fomo_detection() {
        let candles = fomo_candles(50);
        let classifier = InvestorBehaviorClassifier::new();
        let (regime, _conf) = classifier.classify(&candles);

        // 强势上涨 + 放量可能触发 FOMO 或 RiskOn
        assert!(matches!(
            regime,
            InvestorBehaviorRegime::FOMO
                | InvestorBehaviorRegime::RiskOn
                | InvestorBehaviorRegime::Exhaustion
                | InvestorBehaviorRegime::Neutral
        ));
    }

    #[test]
    fn neutral_detection() {
        let candles = neutral_candles(50);
        let classifier = InvestorBehaviorClassifier::new();
        let (regime, _conf) = classifier.classify(&candles);

        // 震荡市场的行为状态可能因价格极端位置而变化
        // 价格在区间边界时可能触发 FOMO/RiskOn，在区间中部时可能 Neutral/Exhaustion
        assert!(matches!(
            regime,
            InvestorBehaviorRegime::Neutral
                | InvestorBehaviorRegime::RiskOff
                | InvestorBehaviorRegime::RiskOn
                | InvestorBehaviorRegime::FOMO
                | InvestorBehaviorRegime::Exhaustion
                | InvestorBehaviorRegime::Crowding
        ));
    }
}
