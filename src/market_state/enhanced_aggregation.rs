//! Enhanced Market State Aggregation
//!
//! 增强版市场状态聚合：提高主大类/次小类分类准确率
//!
//! 改进点：
//! 1. 价格方向判断：区分 Bull/Bear 趋势
//! 2. 置信度惩罚：多维度冲突时降低置信度
//! 3. 严格阈值：提高分类门槛
//! 4. 多维度交叉验证：要求多个维度一致

use serde::{Deserialize, Serialize};

use crate::market_state::{
    InvestorBehaviorRegime, LiquidityRegime, MarketStateAggregationInputs, MarketStructureRegime,
    PrimaryMarketRegime, SecondaryMarketRegime, VolatilityRegime,
};
use crate::types::Candle;

/// 增强聚合配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedAggregationConfig {
    /// 极端状态最低置信度阈值
    pub extreme_min_confidence: f64,
    /// 趋势扩展最低置信度阈值
    pub trend_min_confidence: f64,
    /// 反转酝酿最低置信度阈值
    pub reversal_min_confidence: f64,
    /// 多维度一致性权重
    pub consistency_weight: f64,
    /// 基础置信度，避免冷启动窗口置信度过低
    pub base_confidence: f64,
    /// 波动率维度权重
    pub volatility_weight: f64,
    /// 流动性维度权重
    pub liquidity_weight: f64,
    /// 市场结构维度权重
    pub structure_weight: f64,
    /// 投资者行为维度权重
    pub behavior_weight: f64,
    /// 高一致性额外加成
    pub high_consistency_bonus: f64,
    /// 中等一致性额外加成
    pub medium_consistency_bonus: f64,
    /// 价格方向窗口（用于判断 Bull/Bear）
    pub price_direction_window: usize,
    /// 价格方向阈值（涨跌幅 %）
    pub price_direction_threshold: f64,
}

impl Default for EnhancedAggregationConfig {
    fn default() -> Self {
        Self {
            extreme_min_confidence: 0.65,  // 极端状态要求高置信
            trend_min_confidence: 0.50,    // 趋势扩展要求中等置信
            reversal_min_confidence: 0.50, // 反转酝酿要求中等置信
            consistency_weight: 0.30,      // 一致性贡献30%
            base_confidence: 0.25,
            volatility_weight: 0.15,
            liquidity_weight: 0.10,
            structure_weight: 0.50,
            behavior_weight: 0.25,
            high_consistency_bonus: 0.05,
            medium_consistency_bonus: 0.03,
            price_direction_window: 20,     // 20 根 K 线判断方向
            price_direction_threshold: 2.0, // 2% 涨跌幅阈值
        }
    }
}

/// 价格方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceDirection {
    Bullish,
    Bearish,
    Neutral,
}

/// 增强聚合器
pub struct EnhancedAggregator {
    config: EnhancedAggregationConfig,
}

impl EnhancedAggregator {
    pub fn new() -> Self {
        Self::with_config(EnhancedAggregationConfig::default())
    }

    pub fn with_config(config: EnhancedAggregationConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &EnhancedAggregationConfig {
        &self.config
    }

    /// 聚合各维度状态到主大类/次小类
    pub fn aggregate(
        &self,
        inputs: MarketStateAggregationInputs<'_>,
        candles: &[Candle],
    ) -> (PrimaryMarketRegime, SecondaryMarketRegime, f64) {
        let MarketStateAggregationInputs {
            volatility: vol,
            volatility_confidence: vol_conf,
            liquidity: liq,
            liquidity_confidence: liq_conf,
            structure: struct_regime,
            structure_confidence: struct_conf,
            behavior: behav,
            behavior_confidence: behav_conf,
        } = inputs;
        // 1. 计算价格方向
        let price_dir = self.calculate_price_direction(candles);

        // 2. 计算多维度一致性
        let consistency = self.calculate_consistency(vol, liq, struct_regime, behav, &price_dir);

        // 3. 基础加权置信度
        // 提高结构权重，降低波动率权重（因为结构对趋势识别更重要）
        // 添加基础置信度，避免过低的综合置信度
        let weighted_conf = vol_conf * self.config.volatility_weight
            + liq_conf * self.config.liquidity_weight
            + struct_conf * self.config.structure_weight
            + behav_conf * self.config.behavior_weight;
        let base_conf =
            self.config.base_confidence + weighted_conf * (1.0 - self.config.base_confidence);

        // 4. 应用一致性加成
        // 一致性高时额外加成
        let consistency_bonus = if consistency > 0.8 {
            self.config.high_consistency_bonus
        } else if consistency > 0.6 {
            self.config.medium_consistency_bonus
        } else {
            0.0
        };
        let overall_conf = (base_conf * (1.0 - self.config.consistency_weight)
            + consistency * self.config.consistency_weight
            + consistency_bonus)
            .clamp(0.0, 1.0);

        // 5. 按优先级聚合（严格阈值）

        // 极端状态：要求高置信 + 明确信号
        if self.is_extreme_stress(vol, vol_conf, liq, liq_conf, behav, behav_conf) {
            let secondary = self.classify_extreme_secondary(vol, liq, behav, &price_dir);
            return (PrimaryMarketRegime::ExtremeStress, secondary, overall_conf);
        }

        // 反转酝酿：要求行为极端 + 结构弱化 + 中等置信
        if self.is_reversal_brewing(behav, behav_conf, struct_regime, struct_conf, &price_dir) {
            let secondary = self.classify_reversal_secondary(behav, struct_regime, &price_dir);
            return (
                PrimaryMarketRegime::ReversalBrewing,
                secondary,
                overall_conf,
            );
        }

        // 趋势扩展：要求结构强 + 流动性好 + 中高置信
        if self.is_trend_expansion(struct_regime, struct_conf, liq, liq_conf, vol, vol_conf) {
            let secondary = self.classify_trend_secondary(vol, behav, &price_dir);
            return (PrimaryMarketRegime::TrendExpansion, secondary, overall_conf);
        }

        // Wyckoff 阶段
        if matches!(struct_regime, MarketStructureRegime::Accumulation) && struct_conf > 0.6 {
            return (
                PrimaryMarketRegime::RangeConsolidation,
                SecondaryMarketRegime::Accumulation,
                overall_conf,
            );
        }
        if matches!(struct_regime, MarketStructureRegime::Distribution) && struct_conf > 0.6 {
            return (
                PrimaryMarketRegime::RangeConsolidation,
                SecondaryMarketRegime::Distribution,
                overall_conf,
            );
        }

        // 默认：震荡整理
        let secondary = if matches!(vol, VolatilityRegime::LowVol) {
            SecondaryMarketRegime::TightRange
        } else {
            SecondaryMarketRegime::WideRange
        };
        (
            PrimaryMarketRegime::RangeConsolidation,
            secondary,
            overall_conf,
        )
    }

    /// 计算价格方向
    fn calculate_price_direction(&self, candles: &[Candle]) -> PriceDirection {
        if candles.len() < self.config.price_direction_window {
            return PriceDirection::Neutral;
        }

        let window = &candles[candles.len() - self.config.price_direction_window..];
        let start_price = window[0].close;
        let end_price = window[window.len() - 1].close;
        let change_pct = ((end_price - start_price) / start_price) * 100.0;

        if change_pct > self.config.price_direction_threshold {
            PriceDirection::Bullish
        } else if change_pct < -self.config.price_direction_threshold {
            PriceDirection::Bearish
        } else {
            PriceDirection::Neutral
        }
    }

    /// 计算多维度一致性（0.0-1.0）
    fn calculate_consistency(
        &self,
        vol: &VolatilityRegime,
        liq: &LiquidityRegime,
        struct_regime: &MarketStructureRegime,
        behav: &InvestorBehaviorRegime,
        price_dir: &PriceDirection,
    ) -> f64 {
        let mut consistency_score = 0.0;
        let mut checks = 0;

        // 检查 1：趋势结构 + 高流动性 = 一致
        if matches!(struct_regime, MarketStructureRegime::Trending)
            && matches!(
                liq,
                LiquidityRegime::HighLiquidity | LiquidityRegime::NormalLiquidity
            )
        {
            consistency_score += 1.0;
        }
        checks += 1;

        // 检查 2：高波动 + 趋势结构 = 一致（加速）
        if matches!(vol, VolatilityRegime::ElevatedVol)
            && matches!(struct_regime, MarketStructureRegime::Trending)
        {
            consistency_score += 1.0;
        }
        checks += 1;

        // 检查 3：低波动 + 震荡结构 = 一致
        if matches!(vol, VolatilityRegime::LowVol)
            && matches!(
                struct_regime,
                MarketStructureRegime::Ranging | MarketStructureRegime::MeanReverting
            )
        {
            consistency_score += 1.0;
        }
        checks += 1;

        // 检查 4：行为极端或趋势结构与价格方向一致
        match (behav, price_dir, struct_regime) {
            (InvestorBehaviorRegime::FOMO, PriceDirection::Bullish, _)
            | (InvestorBehaviorRegime::Capitulation, PriceDirection::Bearish, _)
            | (_, PriceDirection::Bullish, MarketStructureRegime::Trending)
            | (_, PriceDirection::Bearish, MarketStructureRegime::Trending) => {
                consistency_score += 1.0;
            }
            // 部分匹配：价格有方向
            (_, PriceDirection::Bullish | PriceDirection::Bearish, _) => {
                consistency_score += 0.5;
            }
            _ => {
                consistency_score += 0.2; // 默认部分得分
            }
        }
        checks += 1;

        // 检查 5：流动性枯竭 + 极端波动 = 一致（危机）
        if matches!(liq, LiquidityRegime::ThinLiquidity)
            && matches!(
                vol,
                VolatilityRegime::CrisisVol | VolatilityRegime::ElevatedVol
            )
        {
            consistency_score += 1.0;
        } else if matches!(liq, LiquidityRegime::ThinLiquidity)
            || matches!(
                vol,
                VolatilityRegime::CrisisVol | VolatilityRegime::ElevatedVol
            )
        {
            consistency_score += 0.4; // 部分得分
        }
        checks += 1;

        consistency_score / checks as f64
    }

    /// 判断是否为极端状态
    fn is_extreme_stress(
        &self,
        vol: &VolatilityRegime,
        vol_conf: f64,
        liq: &LiquidityRegime,
        liq_conf: f64,
        behav: &InvestorBehaviorRegime,
        behav_conf: f64,
    ) -> bool {
        // 危机波动 + 高置信（提高阈值到 0.75）
        if matches!(vol, VolatilityRegime::CrisisVol) && vol_conf > 0.75 {
            return true;
        }

        // 流动性枯竭 + 极高置信（提高阈值到 0.80）
        // 因为流动性基础置信度已提高，需要更严格条件
        if matches!(liq, LiquidityRegime::ThinLiquidity) && liq_conf >= 0.80 {
            return true;
        }

        // 行为恐慌 + 危机波动 + 高置信
        if matches!(
            behav,
            InvestorBehaviorRegime::Capitulation | InvestorBehaviorRegime::FOMO
        ) && matches!(vol, VolatilityRegime::CrisisVol)
            && behav_conf > 0.70
            && vol_conf > 0.70
        {
            return true;
        }

        false
    }

    /// 判断是否为反转酝酿
    fn is_reversal_brewing(
        &self,
        behav: &InvestorBehaviorRegime,
        behav_conf: f64,
        struct_regime: &MarketStructureRegime,
        struct_conf: f64,
        _price_dir: &PriceDirection,
    ) -> bool {
        // 行为极端 + 结构弱化 + 中等置信
        matches!(
            behav,
            InvestorBehaviorRegime::Exhaustion | InvestorBehaviorRegime::Crowding
        ) && behav_conf > self.config.reversal_min_confidence
            && matches!(
                struct_regime,
                MarketStructureRegime::MeanReverting | MarketStructureRegime::Ranging
            )
            && struct_conf > 0.5
    }

    /// 判断是否为趋势扩展
    fn is_trend_expansion(
        &self,
        struct_regime: &MarketStructureRegime,
        struct_conf: f64,
        liq: &LiquidityRegime,
        liq_conf: f64,
        _vol: &VolatilityRegime,
        _vol_conf: f64,
    ) -> bool {
        // 趋势结构 + 高流动性 + 中高置信
        // 降低流动性置信度要求（原 0.55 → 0.35），因为流动性置信度对中间值很敏感
        matches!(struct_regime, MarketStructureRegime::Trending)
            && struct_conf > self.config.trend_min_confidence
            && matches!(
                liq,
                LiquidityRegime::HighLiquidity | LiquidityRegime::NormalLiquidity
            )
            && liq_conf > 0.35 // 降低要求
    }

    /// 分类极端状态次小类
    fn classify_extreme_secondary(
        &self,
        vol: &VolatilityRegime,
        liq: &LiquidityRegime,
        behav: &InvestorBehaviorRegime,
        price_dir: &PriceDirection,
    ) -> SecondaryMarketRegime {
        // 恐慌性抛售：Capitulation + Bearish
        if matches!(behav, InvestorBehaviorRegime::Capitulation)
            && matches!(price_dir, PriceDirection::Bearish)
        {
            return SecondaryMarketRegime::PanicSelling;
        }

        // 恐慌性买入：FOMO + Bullish
        if matches!(behav, InvestorBehaviorRegime::FOMO)
            && matches!(price_dir, PriceDirection::Bullish)
        {
            return SecondaryMarketRegime::PanicBuying;
        }

        // 流动性危机
        if matches!(liq, LiquidityRegime::ThinLiquidity) {
            return SecondaryMarketRegime::LiquidityCrunch;
        }

        // 波动率飙升
        if matches!(
            vol,
            VolatilityRegime::CrisisVol | VolatilityRegime::ElevatedVol
        ) {
            return SecondaryMarketRegime::VolatilitySpike;
        }

        SecondaryMarketRegime::VolatilitySpike
    }

    /// 分类反转酝酿次小类
    fn classify_reversal_secondary(
        &self,
        behav: &InvestorBehaviorRegime,
        struct_regime: &MarketStructureRegime,
        _price_dir: &PriceDirection,
    ) -> SecondaryMarketRegime {
        if matches!(behav, InvestorBehaviorRegime::Exhaustion) {
            SecondaryMarketRegime::TrendFatigue
        } else if matches!(behav, InvestorBehaviorRegime::Crowding) {
            SecondaryMarketRegime::SentimentExtreme
        } else if matches!(struct_regime, MarketStructureRegime::MeanReverting) {
            SecondaryMarketRegime::StructureBreakdown
        } else {
            SecondaryMarketRegime::TrendFatigue
        }
    }

    /// 分类趋势扩展次小类
    fn classify_trend_secondary(
        &self,
        vol: &VolatilityRegime,
        behav: &InvestorBehaviorRegime,
        price_dir: &PriceDirection,
    ) -> SecondaryMarketRegime {
        // 高波动 + 趋势 = 加速
        let is_acceleration = matches!(vol, VolatilityRegime::ElevatedVol)
            || matches!(behav, InvestorBehaviorRegime::FOMO);

        // 低波动 + 趋势 = 疲劳
        let is_exhaustion = matches!(vol, VolatilityRegime::LowVol)
            || matches!(behav, InvestorBehaviorRegime::Exhaustion);

        match price_dir {
            PriceDirection::Bullish => {
                if is_acceleration {
                    SecondaryMarketRegime::BullTrendAcceleration
                } else if is_exhaustion {
                    SecondaryMarketRegime::BullTrendExhaustion
                } else {
                    SecondaryMarketRegime::BullTrendAcceleration
                }
            }
            PriceDirection::Bearish => {
                if is_acceleration {
                    SecondaryMarketRegime::BearTrendAcceleration
                } else if is_exhaustion {
                    SecondaryMarketRegime::BearTrendExhaustion
                } else {
                    SecondaryMarketRegime::BearTrendAcceleration
                }
            }
            PriceDirection::Neutral => {
                // 中性方向默认用 Bull（保守）
                if is_exhaustion {
                    SecondaryMarketRegime::BullTrendExhaustion
                } else {
                    SecondaryMarketRegime::BullTrendAcceleration
                }
            }
        }
    }
}

impl Default for EnhancedAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn mock_candles_bullish() -> Vec<Candle> {
        (0..30)
            .map(|i| Candle {
                timestamp: Utc.timestamp_opt(1_700_000_000 + i as i64 * 60, 0).unwrap(),
                open: 100.0 + i as f64,
                high: 101.0 + i as f64,
                low: 99.0 + i as f64,
                close: 100.5 + i as f64,
                volume: 1000.0,
            })
            .collect()
    }

    fn mock_candles_bearish() -> Vec<Candle> {
        (0..30)
            .map(|i| Candle {
                timestamp: Utc.timestamp_opt(1_700_000_000 + i as i64 * 60, 0).unwrap(),
                open: 130.0 - i as f64,
                high: 131.0 - i as f64,
                low: 129.0 - i as f64,
                close: 130.5 - i as f64,
                volume: 1000.0,
            })
            .collect()
    }

    #[test]
    fn price_direction_detection() {
        let agg = EnhancedAggregator::new();

        let bullish = mock_candles_bullish();
        assert_eq!(
            agg.calculate_price_direction(&bullish),
            PriceDirection::Bullish
        );

        let bearish = mock_candles_bearish();
        assert_eq!(
            agg.calculate_price_direction(&bearish),
            PriceDirection::Bearish
        );
    }

    #[test]
    fn extreme_stress_detection() {
        let agg = EnhancedAggregator::new();

        // 危机波动 + 高置信 → ExtremeStress
        assert!(agg.is_extreme_stress(
            &VolatilityRegime::CrisisVol,
            0.8,
            &LiquidityRegime::NormalLiquidity,
            0.5,
            &InvestorBehaviorRegime::Neutral,
            0.5,
        ));

        // 流动性枯竭 + 高置信 → ExtremeStress
        assert!(agg.is_extreme_stress(
            &VolatilityRegime::NormalVol,
            0.5,
            &LiquidityRegime::ThinLiquidity,
            0.8,
            &InvestorBehaviorRegime::Neutral,
            0.5,
        ));
    }

    #[test]
    fn trend_expansion_with_direction() {
        let agg = EnhancedAggregator::new();
        let candles = mock_candles_bullish();

        let (primary, secondary, _conf) = agg.aggregate(
            MarketStateAggregationInputs {
                volatility: &VolatilityRegime::ElevatedVol,
                volatility_confidence: 0.7,
                liquidity: &LiquidityRegime::HighLiquidity,
                liquidity_confidence: 0.8,
                structure: &MarketStructureRegime::Trending,
                structure_confidence: 0.75,
                behavior: &InvestorBehaviorRegime::Neutral,
                behavior_confidence: 0.5,
            },
            &candles,
        );

        assert_eq!(primary, PrimaryMarketRegime::TrendExpansion);
        assert_eq!(secondary, SecondaryMarketRegime::BullTrendAcceleration);
    }

    #[test]
    fn consistency_boosts_confidence() {
        let agg = EnhancedAggregator::new();

        // 高一致性：趋势 + 高流动性 + 高波动
        let consistency = agg.calculate_consistency(
            &VolatilityRegime::ElevatedVol,
            &LiquidityRegime::HighLiquidity,
            &MarketStructureRegime::Trending,
            &InvestorBehaviorRegime::Neutral,
            &PriceDirection::Bullish,
        );

        assert!(consistency > 0.5, "一致性应 > 0.5，实际 {}", consistency);
    }

    #[test]
    fn custom_config_changes_confidence_without_changing_regime() {
        let base = EnhancedAggregator::new();
        let tuned = EnhancedAggregator::with_config(EnhancedAggregationConfig {
            base_confidence: 0.40,
            consistency_weight: 0.40,
            high_consistency_bonus: 0.08,
            medium_consistency_bonus: 0.04,
            ..EnhancedAggregationConfig::default()
        });
        let candles = mock_candles_bullish();

        let (base_primary, base_secondary, base_conf) = base.aggregate(
            MarketStateAggregationInputs {
                volatility: &VolatilityRegime::ElevatedVol,
                volatility_confidence: 0.7,
                liquidity: &LiquidityRegime::HighLiquidity,
                liquidity_confidence: 0.8,
                structure: &MarketStructureRegime::Trending,
                structure_confidence: 0.75,
                behavior: &InvestorBehaviorRegime::Neutral,
                behavior_confidence: 0.5,
            },
            &candles,
        );
        let (tuned_primary, tuned_secondary, tuned_conf) = tuned.aggregate(
            MarketStateAggregationInputs {
                volatility: &VolatilityRegime::ElevatedVol,
                volatility_confidence: 0.7,
                liquidity: &LiquidityRegime::HighLiquidity,
                liquidity_confidence: 0.8,
                structure: &MarketStructureRegime::Trending,
                structure_confidence: 0.75,
                behavior: &InvestorBehaviorRegime::Neutral,
                behavior_confidence: 0.5,
            },
            &candles,
        );

        assert_eq!(base_primary, tuned_primary);
        assert_eq!(base_secondary, tuned_secondary);
        assert!(tuned_conf > base_conf);
    }
}
