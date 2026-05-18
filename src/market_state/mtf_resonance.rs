//! Multi-Timeframe Resonance Filter
//!
//! 低周期因子信号必须与高周期市场状态对齐
//! 设计原则：
//! - 共振对齐 = 高置信度
//! - 共振矛盾 = 低置信度或阻断
//! - 零配置：默认共振栈

use crate::market_state::{
    MarketStateClassifier, MarketStateConfig, MarketStateSnapshot, PrimaryMarketRegime,
};
use crate::types::Candle;
use serde::{Deserialize, Serialize};

/// 共振结果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ResonanceResult {
    /// 高周期与低周期方向一致
    Aligned,
    /// 高周期与低周期方向矛盾
    Contradicted,
    /// 高周期状态不明确
    Neutral,
    /// 高周期数据缺失
    #[default]
    Missing,
}

/// 时间周期共振配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeframeResonanceConfig {
    /// 基础周期（分钟）
    pub base_timeframe: i64,
    /// 共振检查栈（分钟）
    pub resonance_stack: Vec<i64>,
    /// 共振对齐权重加成
    pub aligned_weight_bonus: f64,
    /// 共振矛盾权重惩罚
    pub contradicted_weight_penalty: f64,
    /// 最小共振周期数（低于此值降权）
    pub min_resonance_count: usize,
}

impl Default for TimeframeResonanceConfig {
    fn default() -> Self {
        Self {
            base_timeframe: 5,                  // 5分钟基础周期
            resonance_stack: vec![15, 60, 240], // 15m, 1h, 4h
            aligned_weight_bonus: 0.15,
            contradicted_weight_penalty: 0.30,
            min_resonance_count: 2,
        }
    }
}

/// 时间周期共振结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeframeResonanceResult {
    /// 基础周期快照
    pub base_snapshot: MarketStateSnapshot,
    /// 高周期快照列表
    pub higher_timeframe_snapshots: Vec<(i64, MarketStateSnapshot)>,
    /// 主大类共振结果
    pub primary_regime_resonance: ResonanceResult,
    /// 波动率共振结果
    pub volatility_resonance: ResonanceResult,
    /// 流动性共振结果
    pub liquidity_resonance: ResonanceResult,
    /// 综合共振分数（0.0-1.0）
    pub overall_resonance_score: f64,
    /// 置信度调整因子（乘以原置信度）
    pub confidence_adjustment: f64,
    /// 共振详情
    pub resonance_details: Vec<String>,
}

/// 多周期共振滤波器
pub struct TimeframeResonanceFilter {
    classifier: MarketStateClassifier,
    config: TimeframeResonanceConfig,
}

impl TimeframeResonanceFilter {
    /// 创建默认配置
    pub fn new() -> Self {
        Self::with_config(
            TimeframeResonanceConfig::default(),
            MarketStateConfig::default(),
        )
    }

    /// 创建自定义配置
    pub fn with_config(config: TimeframeResonanceConfig, state_config: MarketStateConfig) -> Self {
        Self {
            classifier: MarketStateClassifier::with_config(state_config),
            config,
        }
    }

    /// 执行共振检查
    ///
    /// 参数：
    /// - base_candles: 基础周期K线
    /// - higher_tf_candles: 高周期K线映射 (timeframe_minutes -> candles)
    pub fn check_resonance(
        &self,
        base_candles: &[Candle],
        higher_tf_candles: &std::collections::HashMap<i64, Vec<Candle>>,
    ) -> TimeframeResonanceResult {
        let mut resonance_details = Vec::new();

        // 1. 分类基础周期状态
        let base_snapshot = self.classifier.classify(base_candles);
        resonance_details.push(format!(
            "base_{}m: primary={:?} vol={:?} liq={:?} conf={:.2}",
            self.config.base_timeframe,
            base_snapshot.primary_regime,
            base_snapshot.volatility,
            base_snapshot.liquidity,
            base_snapshot.overall_confidence
        ));

        // 2. 分类高周期状态
        let mut higher_timeframe_snapshots = Vec::new();
        for &tf in &self.config.resonance_stack {
            if let Some(candles) = higher_tf_candles.get(&tf) {
                if candles.len() >= 20 {
                    let snapshot = self.classifier.classify(candles);
                    resonance_details.push(format!(
                        "htf_{}m: primary={:?} vol={:?} conf={:.2}",
                        tf,
                        snapshot.primary_regime,
                        snapshot.volatility,
                        snapshot.overall_confidence
                    ));
                    higher_timeframe_snapshots.push((tf, snapshot));
                }
            }
        }

        // 3. 计算主大类共振
        let primary_regime_resonance =
            self.compute_primary_resonance(&base_snapshot, &higher_timeframe_snapshots);

        // 4. 计算波动率共振
        let volatility_resonance =
            self.compute_volatility_resonance(&base_snapshot, &higher_timeframe_snapshots);

        // 5. 计算流动性共振
        let liquidity_resonance =
            self.compute_liquidity_resonance(&base_snapshot, &higher_timeframe_snapshots);

        // 6. 计算综合共振分数
        let overall_resonance_score = self.compute_overall_score(
            &primary_regime_resonance,
            &volatility_resonance,
            &liquidity_resonance,
            &higher_timeframe_snapshots,
        );

        // 7. 计算置信度调整因子
        let confidence_adjustment = self
            .compute_confidence_adjustment(overall_resonance_score, &higher_timeframe_snapshots);

        resonance_details.push(format!(
            "resonance: primary={:?} vol={:?} liq={:?} score={:.2} conf_adj={:.2}",
            primary_regime_resonance,
            volatility_resonance,
            liquidity_resonance,
            overall_resonance_score,
            confidence_adjustment
        ));

        TimeframeResonanceResult {
            base_snapshot,
            higher_timeframe_snapshots,
            primary_regime_resonance,
            volatility_resonance,
            liquidity_resonance,
            overall_resonance_score,
            confidence_adjustment,
            resonance_details,
        }
    }

    /// 计算主大类共振
    fn compute_primary_resonance(
        &self,
        base: &MarketStateSnapshot,
        higher: &[(i64, MarketStateSnapshot)],
    ) -> ResonanceResult {
        if higher.is_empty() {
            return ResonanceResult::Missing;
        }

        // 检查高周期是否与基础周期主大类一致
        let mut aligned_count = 0;
        let mut contradicted_count = 0;

        for (_, snapshot) in higher {
            if snapshot.overall_confidence < 0.5 {
                continue; // 低置信度跳过
            }

            match (base.primary_regime, snapshot.primary_regime) {
                // 同向共振
                (PrimaryMarketRegime::TrendExpansion, PrimaryMarketRegime::TrendExpansion)
                | (
                    PrimaryMarketRegime::RangeConsolidation,
                    PrimaryMarketRegime::RangeConsolidation,
                )
                | (PrimaryMarketRegime::ReversalBrewing, PrimaryMarketRegime::ReversalBrewing) => {
                    aligned_count += 1;
                }
                // 矛盾
                (PrimaryMarketRegime::TrendExpansion, PrimaryMarketRegime::RangeConsolidation)
                | (PrimaryMarketRegime::RangeConsolidation, PrimaryMarketRegime::TrendExpansion)
                | (PrimaryMarketRegime::TrendExpansion, PrimaryMarketRegime::ReversalBrewing)
                | (PrimaryMarketRegime::ReversalBrewing, PrimaryMarketRegime::TrendExpansion) => {
                    contradicted_count += 1;
                }
                _ => {} // 其他情况为中性
            }
        }

        if aligned_count >= self.config.min_resonance_count {
            ResonanceResult::Aligned
        } else if contradicted_count > 0 && contradicted_count > aligned_count {
            ResonanceResult::Contradicted
        } else if aligned_count > 0 || contradicted_count > 0 {
            ResonanceResult::Neutral
        } else {
            ResonanceResult::Missing
        }
    }

    /// 计算波动率共振
    fn compute_volatility_resonance(
        &self,
        base: &MarketStateSnapshot,
        higher: &[(i64, MarketStateSnapshot)],
    ) -> ResonanceResult {
        if higher.is_empty() {
            return ResonanceResult::Missing;
        }

        // 波动率共振：高周期波动率状态与基础周期一致
        let mut aligned_count = 0;

        for (_, snapshot) in higher {
            if snapshot.volatility_confidence < 0.5 {
                continue;
            }

            // 波动率分类一致
            if std::mem::discriminant(&base.volatility)
                == std::mem::discriminant(&snapshot.volatility)
            {
                aligned_count += 1;
            }
        }

        if aligned_count >= self.config.min_resonance_count {
            ResonanceResult::Aligned
        } else if aligned_count > 0 {
            ResonanceResult::Neutral
        } else {
            ResonanceResult::Missing
        }
    }

    /// 计算流动性共振
    fn compute_liquidity_resonance(
        &self,
        base: &MarketStateSnapshot,
        higher: &[(i64, MarketStateSnapshot)],
    ) -> ResonanceResult {
        if higher.is_empty() {
            return ResonanceResult::Missing;
        }

        let mut aligned_count = 0;

        for (_, snapshot) in higher {
            if snapshot.liquidity_confidence < 0.5 {
                continue;
            }

            if std::mem::discriminant(&base.liquidity)
                == std::mem::discriminant(&snapshot.liquidity)
            {
                aligned_count += 1;
            }
        }

        if aligned_count >= self.config.min_resonance_count {
            ResonanceResult::Aligned
        } else if aligned_count > 0 {
            ResonanceResult::Neutral
        } else {
            ResonanceResult::Missing
        }
    }

    /// 计算综合共振分数
    fn compute_overall_score(
        &self,
        primary: &ResonanceResult,
        volatility: &ResonanceResult,
        liquidity: &ResonanceResult,
        higher: &[(i64, MarketStateSnapshot)],
    ) -> f64 {
        let mut score: f64 = 0.5; // 基准分数

        // 主大类共振权重 40%
        match primary {
            ResonanceResult::Aligned => score += 0.4,
            ResonanceResult::Contradicted => score -= 0.3,
            ResonanceResult::Neutral => score += 0.1,
            ResonanceResult::Missing => score -= 0.1,
        }

        // 波动率共振权重 30%
        match volatility {
            ResonanceResult::Aligned => score += 0.3,
            ResonanceResult::Contradicted => score -= 0.2,
            ResonanceResult::Neutral => score += 0.1,
            ResonanceResult::Missing => {}
        }

        // 流动性共振权重 30%
        match liquidity {
            ResonanceResult::Aligned => score += 0.3,
            ResonanceResult::Contradicted => score -= 0.2,
            ResonanceResult::Neutral => score += 0.1,
            ResonanceResult::Missing => {}
        }

        // 高周期数量加成
        let higher_count = higher.len() as f64;
        let expected_count = self.config.resonance_stack.len() as f64;
        if higher_count >= expected_count {
            score += 0.1; // 满覆盖加成
        } else if higher_count < self.config.min_resonance_count as f64 {
            score -= 0.1; // 覆盖不足惩罚
        }

        score.clamp(0.0, 1.0)
    }

    /// 计算置信度调整因子
    fn compute_confidence_adjustment(
        &self,
        resonance_score: f64,
        higher: &[(i64, MarketStateSnapshot)],
    ) -> f64 {
        // 基准调整因子
        let mut adjustment = 1.0;

        // 共振分数影响
        if resonance_score >= 0.7 {
            adjustment *= 1.0 + self.config.aligned_weight_bonus;
        } else if resonance_score <= 0.3 {
            adjustment *= 1.0 - self.config.contradicted_weight_penalty;
        }

        // 高周期置信度加权
        if !higher.is_empty() {
            let avg_higher_conf: f64 = higher
                .iter()
                .map(|(_, s)| s.overall_confidence)
                .sum::<f64>()
                / higher.len() as f64;

            // 高周期平均置信度影响
            adjustment *= 0.7 + 0.3 * avg_higher_conf;
        } else {
            // 无高周期数据时降低置信度
            adjustment *= 0.7;
        }

        adjustment.clamp(0.3, 1.3)
    }
}

impl Default for TimeframeResonanceFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// 预定义共振配置
impl TimeframeResonanceConfig {
    /// 1分钟基础周期配置
    pub fn for_1m_base() -> Self {
        Self {
            base_timeframe: 1,
            resonance_stack: vec![5, 15, 60, 240], // 5m, 15m, 1h, 4h
            aligned_weight_bonus: 0.20,
            contradicted_weight_penalty: 0.35,
            min_resonance_count: 2,
        }
    }

    /// 5分钟基础周期配置
    pub fn for_5m_base() -> Self {
        Self {
            base_timeframe: 5,
            resonance_stack: vec![15, 60, 240, 1440], // 15m, 1h, 4h, 1d
            aligned_weight_bonus: 0.15,
            contradicted_weight_penalty: 0.30,
            min_resonance_count: 2,
        }
    }

    /// 15分钟基础周期配置
    pub fn for_15m_base() -> Self {
        Self {
            base_timeframe: 15,
            resonance_stack: vec![60, 240, 1440], // 1h, 4h, 1d
            aligned_weight_bonus: 0.12,
            contradicted_weight_penalty: 0.25,
            min_resonance_count: 2,
        }
    }

    /// 1小时基础周期配置
    pub fn for_1h_base() -> Self {
        Self {
            base_timeframe: 60,
            resonance_stack: vec![240, 1440, 10080], // 4h, 1d, 1w
            aligned_weight_bonus: 0.10,
            contradicted_weight_penalty: 0.20,
            min_resonance_count: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use std::collections::HashMap;

    fn sample_candles(count: usize, trend: f64) -> Vec<Candle> {
        (0..count)
            .map(|i| {
                let base = 100.0 + trend * i as f64;
                Candle {
                    timestamp: Utc.timestamp_opt(1_700_000_000 + i as i64 * 60, 0).unwrap(),
                    open: base,
                    high: base + 1.0,
                    low: base - 0.5,
                    close: base + trend * 0.5,
                    volume: 1000.0,
                }
            })
            .collect()
    }

    #[test]
    fn resonance_filter_default_config() {
        let filter = TimeframeResonanceFilter::new();
        let base_candles = sample_candles(100, 0.5);

        // 无高周期数据
        let result = filter.check_resonance(&base_candles, &HashMap::new());

        // 缺失高周期时应返回 Missing
        assert!(matches!(
            result.primary_regime_resonance,
            ResonanceResult::Missing
        ));
        assert!(result.confidence_adjustment < 1.0);
    }

    #[test]
    fn resonance_filter_with_aligned_higher_tf() {
        let filter = TimeframeResonanceFilter::new();
        let base_candles = sample_candles(100, 0.5);

        let mut higher_tf = HashMap::new();
        higher_tf.insert(15, sample_candles(50, 0.5));
        higher_tf.insert(60, sample_candles(30, 0.5));

        let result = filter.check_resonance(&base_candles, &higher_tf);

        // 有高周期数据时应有共振结果
        assert!(!result.higher_timeframe_snapshots.is_empty());
        assert!(result.overall_resonance_score > 0.0);
    }

    #[test]
    fn predefined_configs_valid() {
        let configs = vec![
            TimeframeResonanceConfig::for_1m_base(),
            TimeframeResonanceConfig::for_5m_base(),
            TimeframeResonanceConfig::for_15m_base(),
            TimeframeResonanceConfig::for_1h_base(),
        ];

        for config in configs {
            assert!(!config.resonance_stack.is_empty());
            assert!(config.aligned_weight_bonus > 0.0);
            assert!(config.contradicted_weight_penalty > 0.0);
        }
    }
}
