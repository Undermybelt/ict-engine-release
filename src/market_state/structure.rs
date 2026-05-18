//! Market Structure Regime Classification
//!
//! 分类市场结构状态：Trending / MeanReverting / Ranging
//! 及 Wyckoff 周期：Accumulation / Markup / Distribution / Markdown

use crate::types::Candle;
use serde::{Deserialize, Serialize};

/// 市场结构状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MarketStructureRegime {
    /// 趋势状态：价格持续朝一个方向运动
    Trending,
    /// 均值回归状态：价格在均值附近震荡
    MeanReverting,
    /// 区间震荡：价格在支撑阻力间波动
    Ranging,
    /// Wyckoff 积累阶段
    Accumulation,
    /// Wyckoff 派发阶段
    Distribution,
    /// 突破中
    Breakout,
    /// 突破失败
    Breakdown,
    #[default]
    Unknown,
}

impl MarketStructureRegime {
    pub fn label(&self) -> &'static str {
        match self {
            MarketStructureRegime::Trending => "trending",
            MarketStructureRegime::MeanReverting => "mean_reverting",
            MarketStructureRegime::Ranging => "ranging",
            MarketStructureRegime::Accumulation => "accumulation",
            MarketStructureRegime::Distribution => "distribution",
            MarketStructureRegime::Breakout => "breakout",
            MarketStructureRegime::Breakdown => "breakdown",
            MarketStructureRegime::Unknown => "unknown",
        }
    }
}

/// 市场结构分类器阈值配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureThresholds {
    /// ADX 趋势强度阈值：> trend_threshold 为趋势
    pub trend_threshold: f64,
    /// 均值回归阈值：价格偏离均值的程度
    pub mean_revert_threshold: f64,
    /// 区间识别：价格范围占比
    pub range_ratio_threshold: f64,
    /// ADX 计算周期
    pub adx_period: usize,
    /// 均值计算周期
    pub ma_period: usize,
    /// 区间识别回看窗口
    pub range_lookback: usize,
}

impl Default for StructureThresholds {
    fn default() -> Self {
        Self {
            trend_threshold: 25.0,       // ADX > 25 为趋势
            mean_revert_threshold: 0.02, // 价格偏离 2% 触发均值回归
            range_ratio_threshold: 0.6,  // 区间占比 > 60% 为震荡
            adx_period: 14,
            ma_period: 20,
            range_lookback: 50,
        }
    }
}

/// 市场结构分类器
pub struct MarketStructureClassifier {
    thresholds: StructureThresholds,
}

impl MarketStructureClassifier {
    pub fn new() -> Self {
        Self::with_thresholds(StructureThresholds::default())
    }

    pub fn with_thresholds(thresholds: StructureThresholds) -> Self {
        Self { thresholds }
    }

    /// 分类市场结构状态，返回 (状态, 置信度)
    pub fn classify(&self, candles: &[Candle]) -> (MarketStructureRegime, f64) {
        if candles.len() < self.thresholds.adx_period + 1 {
            return (MarketStructureRegime::Unknown, 0.0);
        }

        // 1. 计算 ADX（趋势强度）
        let adx = self.compute_adx(candles);
        let current_adx = *adx.last().unwrap_or(&20.0);

        // 2. 计算价格相对于均值的偏离
        let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
        let ma = self.compute_ma(&closes);
        let last_close = closes.last().copied().unwrap_or(0.0);
        let deviation = (last_close - ma).abs() / ma.max(1e-10);

        // 3. 计算区间特征
        let range_score = self.compute_range_score(&closes);

        // 4. 检测 Wyckoff 阶段
        let wyckoff_score = self.detect_wyckoff_phase(candles);

        // 5. 综合分类
        let (regime, confidence) = if current_adx >= self.thresholds.trend_threshold {
            // 强趋势
            let conf = (current_adx / 50.0).min(1.0); // ADX 50 = 最高置信
            (MarketStructureRegime::Trending, conf)
        } else if wyckoff_score.0 > 0.6 {
            // Wyckoff 阶段检测
            (wyckoff_score.1, wyckoff_score.0)
        } else if deviation > self.thresholds.mean_revert_threshold {
            // 价格偏离均值较大，可能均值回归
            let conf = (deviation / 0.05).min(1.0);
            (MarketStructureRegime::MeanReverting, conf)
        } else if range_score > self.thresholds.range_ratio_threshold {
            // 区间震荡
            (MarketStructureRegime::Ranging, range_score)
        } else {
            (MarketStructureRegime::Unknown, 0.3)
        };

        (regime, confidence)
    }

    /// 计算 ADX（Average Directional Index）
    fn compute_adx(&self, candles: &[Candle]) -> Vec<f64> {
        let period = self.thresholds.adx_period;
        if candles.len() < period + 1 {
            return vec![20.0]; // 默认中等趋势
        }

        // 计算 +DM 和 -DM
        let mut plus_dm = vec![0.0];
        let mut minus_dm = vec![0.0];
        let mut tr = vec![0.0];

        for i in 1..candles.len() {
            let up_move = candles[i].high - candles[i - 1].high;
            let down_move = candles[i - 1].low - candles[i].low;

            let plus = if up_move > down_move && up_move > 0.0 {
                up_move
            } else {
                0.0
            };
            let minus = if down_move > up_move && down_move > 0.0 {
                down_move
            } else {
                0.0
            };

            plus_dm.push(plus);
            minus_dm.push(minus);

            let true_range = (candles[i].high - candles[i].low)
                .max((candles[i].high - candles[i - 1].close).abs())
                .max((candles[i].low - candles[i - 1].close).abs());
            tr.push(true_range);
        }

        // 平滑处理
        let smooth_plus = self.ema(&plus_dm, period);
        let smooth_minus = self.ema(&minus_dm, period);
        let smooth_tr = self.ema(&tr, period);

        // 计算 +DI 和 -DI
        let mut adx = Vec::new();
        for i in 0..smooth_plus
            .len()
            .min(smooth_minus.len())
            .min(smooth_tr.len())
        {
            let plus_di = if smooth_tr[i] > 0.0 {
                100.0 * smooth_plus[i] / smooth_tr[i]
            } else {
                0.0
            };
            let minus_di = if smooth_tr[i] > 0.0 {
                100.0 * smooth_minus[i] / smooth_tr[i]
            } else {
                0.0
            };

            let dx = if plus_di + minus_di > 0.0 {
                100.0 * (plus_di - minus_di).abs() / (plus_di + minus_di)
            } else {
                0.0
            };
            adx.push(dx);
        }

        // 再次平滑得到 ADX
        self.ema(&adx, period)
    }

    /// EMA 平滑
    fn ema(&self, data: &[f64], period: usize) -> Vec<f64> {
        if data.is_empty() {
            return Vec::new();
        }

        let multiplier = 2.0 / (period as f64 + 1.0);
        let mut result = vec![data[0]];

        for i in 1..data.len() {
            let ema_val = data[i] * multiplier + result[i - 1] * (1.0 - multiplier);
            result.push(ema_val);
        }

        result
    }

    /// 计算简单移动平均
    fn compute_ma(&self, closes: &[f64]) -> f64 {
        let period = self.thresholds.ma_period.min(closes.len());
        if period == 0 {
            return closes.last().copied().unwrap_or(0.0);
        }

        let slice = &closes[closes.len() - period..];
        slice.iter().sum::<f64>() / period as f64
    }

    /// 计算区间震荡分数
    fn compute_range_score(&self, closes: &[f64]) -> f64 {
        let lookback = self.thresholds.range_lookback.min(closes.len());
        if lookback < 2 {
            return 0.0;
        }

        let slice = &closes[closes.len() - lookback..];
        let high = slice.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let low = slice.iter().cloned().fold(f64::INFINITY, f64::min);
        let range = high - low;

        if range < 1e-10 {
            return 0.0;
        }

        // 计算价格穿越中值的次数
        let mid = (high + low) / 2.0;
        let mut crosses = 0;
        for i in 1..slice.len() {
            if (slice[i] - mid) * (slice[i - 1] - mid) < 0.0 {
                crosses += 1;
            }
        }

        // 穿越次数越多，区间特征越明显
        crosses as f64 / (lookback as f64 / 2.0)
    }

    /// 检测 Wyckoff 阶段（简化版）
    fn detect_wyckoff_phase(&self, candles: &[Candle]) -> (f64, MarketStructureRegime) {
        if candles.len() < 100 {
            return (0.0, MarketStructureRegime::Unknown);
        }

        let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
        let volumes: Vec<f64> = candles.iter().map(|c| c.volume).collect();

        let lookback = 50.min(closes.len());
        let recent = &closes[closes.len() - lookback..];
        let recent_vol = &volumes[volumes.len() - lookback..];

        let high = recent.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let low = recent.iter().cloned().fold(f64::INFINITY, f64::min);
        let mid = (high + low) / 2.0;

        // 低位缩量震荡 + 突破放量 = Accumulation
        // 高位放量震荡 + 破位缩量 = Distribution

        let avg_vol = recent_vol.iter().sum::<f64>() / recent_vol.len() as f64;
        let last_vol = *recent_vol.last().unwrap_or(&avg_vol);
        let last_close = *recent.last().unwrap_or(&mid);

        // 价格在低位区间 + 成交量萎缩
        let is_accumulation = last_close < mid && last_vol < avg_vol * 0.8;
        // 价格在高位区间 + 成交量放大
        let is_distribution = last_close > mid && last_vol > avg_vol * 1.2;

        if is_accumulation {
            (0.65, MarketStructureRegime::Accumulation)
        } else if is_distribution {
            (0.65, MarketStructureRegime::Distribution)
        } else {
            (0.0, MarketStructureRegime::Unknown)
        }
    }
}

impl Default for MarketStructureClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn trend_candles(count: usize, direction: f64) -> Vec<Candle> {
        (0..count)
            .map(|i| {
                let base = 100.0 + direction * i as f64;
                Candle {
                    timestamp: Utc.timestamp_opt(1_700_000_000 + i as i64 * 60, 0).unwrap(),
                    open: base,
                    high: base + 1.0,
                    low: base - 0.5,
                    close: base + direction * 0.5,
                    volume: 1000.0,
                }
            })
            .collect()
    }

    fn range_candles(count: usize) -> Vec<Candle> {
        (0..count)
            .map(|i| {
                let base = 100.0 + (i as f64 % 10.0 - 5.0) * 2.0; // 震荡
                Candle {
                    timestamp: Utc.timestamp_opt(1_700_000_000 + i as i64 * 60, 0).unwrap(),
                    open: base,
                    high: base + 1.0,
                    low: base - 1.0,
                    close: base,
                    volume: 1000.0,
                }
            })
            .collect()
    }

    #[test]
    fn trending_detection() {
        let candles = trend_candles(100, 0.5); // 上涨趋势
        let classifier = MarketStructureClassifier::new();
        let (regime, _conf) = classifier.classify(&candles);

        assert!(matches!(
            regime,
            MarketStructureRegime::Trending | MarketStructureRegime::Unknown
        ));
    }

    #[test]
    fn ranging_detection() {
        let candles = range_candles(100); // 震荡
        let classifier = MarketStructureClassifier::new();
        let (regime, _conf) = classifier.classify(&candles);

        assert!(matches!(
            regime,
            MarketStructureRegime::Ranging
                | MarketStructureRegime::MeanReverting
                | MarketStructureRegime::Unknown
        ));
    }
}
