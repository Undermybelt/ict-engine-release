//! Market State Classification Module
//!
//! 提供多维度市场状态分类：波动率、流动性、结构、行为。
//! 设计原则：
//! - 零配置：默认参数直接可用
//! - 热插拔：通过 MarketStateConfig 覆盖
//! - Token 友好：简洁输出
//! - 无污染：不修改现有代码
//! - 高置信度：基于统计学阈值

mod behavior;
pub mod confidence_validation;
mod config;
pub mod enhanced_aggregation;
pub mod evidence_mapping;
pub mod execution_integration;
pub mod filter;
mod liquidity;
pub mod mtf_resonance;
mod structure;
pub mod validation_tool;
mod volatility;

pub use behavior::{InvestorBehaviorClassifier, InvestorBehaviorRegime};
pub use confidence_validation::{
    ConfidenceLevel, ConfidenceValidationConfig, ConfidenceValidator, HistorySample, RegimeStats,
    RollingAccuracyTracker, ValidationResult as ConfidenceValidationResult,
};
pub use config::{available_profiles, MarketStateConfig, MarketStateProfile, UserWeightsTemplate};
pub use enhanced_aggregation::{EnhancedAggregationConfig, EnhancedAggregator, PriceDirection};
pub use evidence_mapping::{
    EvidenceMappingConfig, EvidenceSummary, LiquidityStateIndex, MarketStateEvidenceMapper,
    MarketStateNodeId, PrimaryRegimeStateIndex, ResonanceStateIndex, VolatilityStateIndex,
};
pub use execution_integration::{
    ExecutionPermission, ExecutionTreeConfig, ExecutionTreePipeline, MarketStateExecutionDecision,
    MarketStateExecutionIntegrator, PipelineResult, RegimeSummary, ResonanceImpact,
};
pub use filter::{
    FactorFilterDeclaration, MarketStateFilter, MarketStateFilterConfig, MarketStateFilterResult,
    StateChange, StateChangeDimension,
};
pub use liquidity::{detect_session, LiquidityClassifier, LiquidityRegime, SessionState};
pub use mtf_resonance::{
    ResonanceResult, TimeframeResonanceConfig, TimeframeResonanceFilter, TimeframeResonanceResult,
};
pub use structure::{MarketStructureClassifier, MarketStructureRegime};
pub use validation_tool::{
    ConfidenceDistribution, MarketStateValidator, ValidationConfig, ValidationResult,
};
pub use volatility::{VolatilityClassifier, VolatilityRegime};

use serde::{Deserialize, Serialize};

/// 市场状态总览：聚合所有维度分类结果
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarketStateSnapshot {
    /// 波动率状态
    pub volatility: VolatilityRegime,
    /// 波动率置信度 (0.0-1.0)
    pub volatility_confidence: f64,
    /// 流动性状态
    pub liquidity: LiquidityRegime,
    /// 流动性置信度
    pub liquidity_confidence: f64,
    /// 市场结构状态
    pub structure: MarketStructureRegime,
    /// 结构置信度
    pub structure_confidence: f64,
    /// 投资者行为状态
    pub behavior: InvestorBehaviorRegime,
    /// 行为置信度
    pub behavior_confidence: f64,
    /// 聚合后的主大类标签
    pub primary_regime: PrimaryMarketRegime,
    /// 聚合后的次小类标签
    pub secondary_regime: SecondaryMarketRegime,
    /// 整体置信度
    pub overall_confidence: f64,
    /// 分类理由（可追溯）
    pub rationale: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct MarketStateAggregationInputs<'a> {
    pub volatility: &'a VolatilityRegime,
    pub volatility_confidence: f64,
    pub liquidity: &'a LiquidityRegime,
    pub liquidity_confidence: f64,
    pub structure: &'a MarketStructureRegime,
    pub structure_confidence: f64,
    pub behavior: &'a InvestorBehaviorRegime,
    pub behavior_confidence: f64,
}

/// 主大类：综合各维度后的顶层状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PrimaryMarketRegime {
    /// 趋势扩展 - 高波动 + 高流动性 + 强结构
    TrendExpansion,
    /// 震荡整理 - 低波动 + 正常流动性 + 弱结构
    RangeConsolidation,
    /// 极端状态 - 危机波动 或 流动性枯竭
    ExtremeStress,
    /// 反转酝酿 - 行为极端 + 结构弱化
    ReversalBrewing,
    /// 默认/未知
    #[default]
    Unknown,
}

/// 次小类：细化状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SecondaryMarketRegime {
    // 趋势扩展细分
    BullTrendAcceleration,
    BearTrendAcceleration,
    BullTrendExhaustion,
    BearTrendExhaustion,

    // 震荡整理细分
    TightRange,
    WideRange,
    Accumulation,
    Distribution,

    // 极端状态细分
    VolatilitySpike,
    LiquidityCrunch,
    PanicSelling,
    PanicBuying,

    // 反转酝酿细分
    TrendFatigue,
    SentimentExtreme,
    StructureBreakdown,

    #[default]
    Unknown,
}

/// 市场状态分类器：聚合各维度分类器
pub struct MarketStateClassifier {
    volatility: VolatilityClassifier,
    liquidity: LiquidityClassifier,
    structure: MarketStructureClassifier,
    behavior: InvestorBehaviorClassifier,
    config: MarketStateConfig,
    /// 增强聚合器（可选，提高置信度）
    enhanced_aggregator: Option<crate::market_state::enhanced_aggregation::EnhancedAggregator>,
}

impl MarketStateClassifier {
    /// 创建默认配置的分类器（零配置）
    pub fn new() -> Self {
        Self::with_config(MarketStateConfig::default())
    }

    /// 创建自定义配置的分类器（热插拔）
    pub fn with_config(config: MarketStateConfig) -> Self {
        let enhanced_config = config.enhanced_aggregation.clone();
        Self {
            volatility: VolatilityClassifier::with_thresholds(config.volatility.clone()),
            liquidity: LiquidityClassifier::with_thresholds(config.liquidity.clone()),
            structure: MarketStructureClassifier::with_thresholds(config.structure.clone()),
            behavior: InvestorBehaviorClassifier::with_thresholds(config.behavior.clone()),
            config,
            enhanced_aggregator: Some(
                crate::market_state::enhanced_aggregation::EnhancedAggregator::with_config(
                    enhanced_config,
                ),
            ),
        }
    }

    /// 启用增强聚合器（提高置信度）
    pub fn with_enhanced_aggregation(mut self, enabled: bool) -> Self {
        if enabled {
            self.enhanced_aggregator = Some(
                crate::market_state::enhanced_aggregation::EnhancedAggregator::with_config(
                    self.config.enhanced_aggregation.clone(),
                ),
            );
        } else {
            self.enhanced_aggregator = None;
        }
        self
    }

    /// 分类：输入 OHLCV 数据，输出市场状态快照
    pub fn classify(&self, candles: &[crate::types::Candle]) -> MarketStateSnapshot {
        let mut rationale = Vec::new();

        // 1. 波动率分类
        let (vol, vol_conf) = self.volatility.classify(candles);
        if vol_conf > 0.5 {
            rationale.push(format!("volatility={:?} conf={:.2}", vol, vol_conf));
        }

        // 2. 流动性分类（基于成交量 + 价格范围）
        let (liq, liq_conf) = self.liquidity.classify(candles);
        if liq_conf > 0.5 {
            rationale.push(format!("liquidity={:?} conf={:.2}", liq, liq_conf));
        }

        // 3. 结构分类
        let (struct_regime, struct_conf) = self.structure.classify(candles);
        if struct_conf > 0.5 {
            rationale.push(format!(
                "structure={:?} conf={:.2}",
                struct_regime, struct_conf
            ));
        }

        // 4. 行为分类
        let (behav, behav_conf) = self.behavior.classify(candles);
        if behav_conf > 0.5 {
            rationale.push(format!("behavior={:?} conf={:.2}", behav, behav_conf));
        }

        // 5. 聚合主大类和次小类
        let (primary, secondary, overall_conf) =
            if let Some(ref enhanced) = self.enhanced_aggregator {
                // 使用增强聚合器（提高置信度）
                enhanced.aggregate(
                    MarketStateAggregationInputs {
                        volatility: &vol,
                        volatility_confidence: vol_conf,
                        liquidity: &liq,
                        liquidity_confidence: liq_conf,
                        structure: &struct_regime,
                        structure_confidence: struct_conf,
                        behavior: &behav,
                        behavior_confidence: behav_conf,
                    },
                    candles,
                )
            } else {
                // 使用基础聚合器
                self.aggregate_regimes(MarketStateAggregationInputs {
                    volatility: &vol,
                    volatility_confidence: vol_conf,
                    liquidity: &liq,
                    liquidity_confidence: liq_conf,
                    structure: &struct_regime,
                    structure_confidence: struct_conf,
                    behavior: &behav,
                    behavior_confidence: behav_conf,
                })
            };
        rationale.push(format!(
            "primary={:?} secondary={:?} overall={:.2}",
            primary, secondary, overall_conf
        ));

        MarketStateSnapshot {
            volatility: vol,
            volatility_confidence: vol_conf,
            liquidity: liq,
            liquidity_confidence: liq_conf,
            structure: struct_regime,
            structure_confidence: struct_conf,
            behavior: behav,
            behavior_confidence: behav_conf,
            primary_regime: primary,
            secondary_regime: secondary,
            overall_confidence: overall_conf,
            rationale,
        }
    }

    /// 聚合各维度状态到主大类/次小类
    fn aggregate_regimes(
        &self,
        inputs: MarketStateAggregationInputs<'_>,
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
        // 计算加权平均置信度
        let weights = &self.config.aggregate_weights;
        let overall_conf = vol_conf * weights.volatility
            + liq_conf * weights.liquidity
            + struct_conf * weights.structure
            + behav_conf * weights.behavior;

        // 聚合逻辑：优先级 决定主大类
        // 1. 极端波动 → ExtremeStress
        if matches!(vol, VolatilityRegime::CrisisVol) && vol_conf > 0.6 {
            let secondary = if matches!(behav, InvestorBehaviorRegime::Capitulation) {
                SecondaryMarketRegime::PanicSelling
            } else if matches!(behav, InvestorBehaviorRegime::FOMO) {
                SecondaryMarketRegime::PanicBuying
            } else {
                SecondaryMarketRegime::VolatilitySpike
            };
            return (PrimaryMarketRegime::ExtremeStress, secondary, overall_conf);
        }

        // 2. 流动性枯竭 → ExtremeStress
        if matches!(liq, LiquidityRegime::ThinLiquidity) && liq_conf > 0.6 {
            return (
                PrimaryMarketRegime::ExtremeStress,
                SecondaryMarketRegime::LiquidityCrunch,
                overall_conf,
            );
        }

        // 3. 行为极端 + 结构弱化 → ReversalBrewing
        if matches!(
            behav,
            InvestorBehaviorRegime::Exhaustion | InvestorBehaviorRegime::Crowding
        ) && behav_conf > 0.5
            && matches!(
                struct_regime,
                MarketStructureRegime::MeanReverting | MarketStructureRegime::Ranging
            )
        {
            let secondary = if matches!(behav, InvestorBehaviorRegime::Exhaustion) {
                SecondaryMarketRegime::TrendFatigue
            } else {
                SecondaryMarketRegime::SentimentExtreme
            };
            return (
                PrimaryMarketRegime::ReversalBrewing,
                secondary,
                overall_conf,
            );
        }

        // 4. 趋势结构 + 高流动性 → TrendExpansion
        if matches!(struct_regime, MarketStructureRegime::Trending)
            && struct_conf > 0.5
            && matches!(
                liq,
                LiquidityRegime::HighLiquidity | LiquidityRegime::NormalLiquidity
            )
        {
            // 根据波动率判断是加速还是疲劳
            let secondary = if matches!(vol, VolatilityRegime::ElevatedVol) {
                // 高波动趋势 = 加速（暂时用 Bull 加速，实际应由价格方向决定）
                SecondaryMarketRegime::BullTrendAcceleration
            } else {
                // 低波动趋势 = 疲劳
                SecondaryMarketRegime::BullTrendExhaustion
            };
            return (PrimaryMarketRegime::TrendExpansion, secondary, overall_conf);
        }

        // 5. Wyckoff 阶段
        if matches!(struct_regime, MarketStructureRegime::Accumulation) && struct_conf > 0.5 {
            return (
                PrimaryMarketRegime::RangeConsolidation,
                SecondaryMarketRegime::Accumulation,
                overall_conf,
            );
        }
        if matches!(struct_regime, MarketStructureRegime::Distribution) && struct_conf > 0.5 {
            return (
                PrimaryMarketRegime::RangeConsolidation,
                SecondaryMarketRegime::Distribution,
                overall_conf,
            );
        }

        // 6. 默认：RangeConsolidation
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

    /// 获取配置（用于序列化/热插拔）
    pub fn config(&self) -> &MarketStateConfig {
        &self.config
    }
}

impl Default for MarketStateClassifier {
    fn default() -> Self {
        Self::new()
    }
}
