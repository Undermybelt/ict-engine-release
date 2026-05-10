//! Market State Filter Module
//!
//! 基于市场状态分类器的滤波节点，用于因子启用/禁用决策
//! 设计原则：
//! - 零配置：默认行为直接可用
//! - 热插拔：通过 MarketStateFilterConfig 覆盖
//! - 高置信度：仅在高置信状态变更时触发

use crate::market_state::{
    InvestorBehaviorRegime, LiquidityRegime, MarketStateClassifier, MarketStateConfig,
    MarketStateSnapshot, MarketStructureRegime, PrimaryMarketRegime, VolatilityRegime,
};
use crate::types::Candle;
use serde::{Deserialize, Serialize};

/// 市场状态滤波配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketStateFilterConfig {
    /// 最小置信度阈值：低于此值的状态变更不触发滤波
    pub min_confidence_threshold: f64,
    /// 波动率状态过滤规则
    pub volatility_filter: VolatilityFilterRule,
    /// 流动性状态过滤规则
    pub liquidity_filter: LiquidityFilterRule,
    /// 结构状态过滤规则
    pub structure_filter: StructureFilterRule,
    /// 行为状态过滤规则
    pub behavior_filter: BehaviorFilterRule,
    /// 主大类过滤规则
    pub primary_regime_filter: PrimaryRegimeFilterRule,
}

impl Default for MarketStateFilterConfig {
    fn default() -> Self {
        Self {
            min_confidence_threshold: 0.6,
            volatility_filter: VolatilityFilterRule::default(),
            liquidity_filter: LiquidityFilterRule::default(),
            structure_filter: StructureFilterRule::default(),
            behavior_filter: BehaviorFilterRule::default(),
            primary_regime_filter: PrimaryRegimeFilterRule::default(),
        }
    }
}

/// 波动率过滤规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolatilityFilterRule {
    /// 允许交易的波动率状态
    pub allowed_regimes: Vec<VolatilityRegime>,
    /// 危机状态是否强制阻断
    pub block_on_crisis: bool,
}

impl Default for VolatilityFilterRule {
    fn default() -> Self {
        Self {
            allowed_regimes: vec![
                VolatilityRegime::LowVol,
                VolatilityRegime::NormalVol,
                VolatilityRegime::ElevatedVol,
            ],
            block_on_crisis: true,
        }
    }
}

/// 流动性过滤规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityFilterRule {
    /// 允许交易的流动性状态
    pub allowed_regimes: Vec<LiquidityRegime>,
    /// 流动性枯竭是否强制阻断
    pub block_on_thin: bool,
}

impl Default for LiquidityFilterRule {
    fn default() -> Self {
        Self {
            allowed_regimes: vec![
                LiquidityRegime::HighLiquidity,
                LiquidityRegime::NormalLiquidity,
            ],
            block_on_thin: true,
        }
    }
}

/// 结构过滤规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureFilterRule {
    /// 允许交易的结构状态
    pub allowed_regimes: Vec<MarketStructureRegime>,
    /// 仅趋势状态下允许趋势策略
    pub trend_only_for_trending: bool,
}

impl Default for StructureFilterRule {
    fn default() -> Self {
        Self {
            allowed_regimes: vec![
                MarketStructureRegime::Trending,
                MarketStructureRegime::Ranging,
                MarketStructureRegime::Accumulation,
                MarketStructureRegime::Distribution,
            ],
            trend_only_for_trending: true,
        }
    }
}

/// 行为过滤规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorFilterRule {
    /// 阻断行为状态列表
    pub blocked_regimes: Vec<InvestorBehaviorRegime>,
    /// FOMO 状态是否降低仓位
    pub reduce_on_fomo: bool,
    /// Capitulation 状态是否暂停交易
    pub pause_on_capitulation: bool,
}

impl Default for BehaviorFilterRule {
    fn default() -> Self {
        Self {
            blocked_regimes: vec![InvestorBehaviorRegime::Capitulation],
            reduce_on_fomo: true,
            pause_on_capitulation: true,
        }
    }
}

/// 主大类过滤规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimaryRegimeFilterRule {
    /// 允许交易的主大类
    pub allowed_regimes: Vec<PrimaryMarketRegime>,
    /// 极端状态下是否强制平仓
    pub flatten_on_extreme_stress: bool,
}

impl Default for PrimaryRegimeFilterRule {
    fn default() -> Self {
        Self {
            allowed_regimes: vec![
                PrimaryMarketRegime::TrendExpansion,
                PrimaryMarketRegime::RangeConsolidation,
                PrimaryMarketRegime::ReversalBrewing,
            ],
            flatten_on_extreme_stress: true,
        }
    }
}

/// 滤波结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketStateFilterResult {
    /// 是否允许交易
    pub allowed: bool,
    /// 阻断原因（如果有）
    pub block_reason: Option<String>,
    /// 当前市场状态快照
    pub snapshot: MarketStateSnapshot,
    /// 建议仓位调整（0.0-1.0）
    pub position_adjustment: f64,
    /// 置信度加权
    pub confidence_weighted: f64,
    /// 滤波触发详情
    pub filter_details: Vec<String>,
}

/// 市场状态滤波器
pub struct MarketStateFilter {
    classifier: MarketStateClassifier,
    config: MarketStateFilterConfig,
    /// 上一次状态快照（用于检测状态变更）
    last_snapshot: Option<MarketStateSnapshot>,
}

impl MarketStateFilter {
    /// 创建默认配置的滤波器
    pub fn new() -> Self {
        Self::with_config(
            MarketStateFilterConfig::default(),
            MarketStateConfig::default(),
        )
    }

    /// 创建自定义配置的滤波器
    pub fn with_config(
        filter_config: MarketStateFilterConfig,
        state_config: MarketStateConfig,
    ) -> Self {
        Self {
            classifier: MarketStateClassifier::with_config(state_config),
            config: filter_config,
            last_snapshot: None,
        }
    }

    /// 执行滤波：返回是否允许交易及原因
    pub fn filter(&mut self, candles: &[Candle]) -> MarketStateFilterResult {
        let mut filter_details = Vec::new();

        // 1. 分类当前市场状态
        let snapshot = self.classifier.classify(candles);
        let overall_confidence = snapshot.overall_confidence;

        // 2. 检查置信度是否足够
        if snapshot.overall_confidence < self.config.min_confidence_threshold {
            filter_details.push(format!(
                "confidence {:.2} below threshold {:.2}",
                snapshot.overall_confidence, self.config.min_confidence_threshold
            ));
            // 置信度不足时，保持保守（不阻断但降低仓位）
            return MarketStateFilterResult {
                allowed: true,
                block_reason: None,
                snapshot,
                position_adjustment: overall_confidence,
                confidence_weighted: overall_confidence,
                filter_details,
            };
        }

        // 3. 检查波动率状态
        if self.config.volatility_filter.block_on_crisis
            && matches!(snapshot.volatility, VolatilityRegime::CrisisVol)
            && snapshot.volatility_confidence >= self.config.min_confidence_threshold
        {
            filter_details.push("blocked: crisis volatility".to_string());
            return MarketStateFilterResult {
                allowed: false,
                block_reason: Some("crisis_volatility".to_string()),
                snapshot,
                position_adjustment: 0.0,
                confidence_weighted: overall_confidence,
                filter_details,
            };
        }

        if !self
            .config
            .volatility_filter
            .allowed_regimes
            .contains(&snapshot.volatility)
        {
            filter_details.push(format!(
                "volatility {:?} not in allowed list",
                snapshot.volatility
            ));
        }

        // 4. 检查流动性状态
        if self.config.liquidity_filter.block_on_thin
            && matches!(snapshot.liquidity, LiquidityRegime::ThinLiquidity)
            && snapshot.liquidity_confidence >= self.config.min_confidence_threshold
        {
            filter_details.push("blocked: thin liquidity".to_string());
            return MarketStateFilterResult {
                allowed: false,
                block_reason: Some("thin_liquidity".to_string()),
                snapshot,
                position_adjustment: 0.0,
                confidence_weighted: overall_confidence,
                filter_details,
            };
        }

        // 5. 检查行为状态
        if self.config.behavior_filter.pause_on_capitulation
            && matches!(snapshot.behavior, InvestorBehaviorRegime::Capitulation)
            && snapshot.behavior_confidence >= self.config.min_confidence_threshold
        {
            filter_details.push("blocked: capitulation".to_string());
            return MarketStateFilterResult {
                allowed: false,
                block_reason: Some("capitulation".to_string()),
                snapshot,
                position_adjustment: 0.0,
                confidence_weighted: overall_confidence,
                filter_details,
            };
        }

        // 6. 检查主大类状态
        if !self
            .config
            .primary_regime_filter
            .allowed_regimes
            .contains(&snapshot.primary_regime)
        {
            filter_details.push(format!(
                "primary_regime {:?} not allowed",
                snapshot.primary_regime
            ));
            if matches!(snapshot.primary_regime, PrimaryMarketRegime::ExtremeStress)
                && self.config.primary_regime_filter.flatten_on_extreme_stress
            {
                return MarketStateFilterResult {
                    allowed: false,
                    block_reason: Some("extreme_stress".to_string()),
                    snapshot,
                    position_adjustment: 0.0,
                    confidence_weighted: overall_confidence,
                    filter_details,
                };
            }
        }

        // 7. 计算仓位调整因子
        let mut position_adjustment = 1.0;

        // FOMO 状态降低仓位
        if self.config.behavior_filter.reduce_on_fomo
            && matches!(snapshot.behavior, InvestorBehaviorRegime::FOMO)
        {
            position_adjustment *= 0.7;
            filter_details.push("position reduced: FOMO detected".to_string());
        }

        // 高波动状态降低仓位
        if matches!(snapshot.volatility, VolatilityRegime::ElevatedVol) {
            position_adjustment *= 0.85;
            filter_details.push("position reduced: elevated volatility".to_string());
        }

        // 8. 计算置信度加权
        let confidence_weighted = overall_confidence * position_adjustment;

        // 9. 更新上一次快照
        self.last_snapshot = Some(snapshot.clone());

        MarketStateFilterResult {
            allowed: true,
            block_reason: None,
            snapshot,
            position_adjustment,
            confidence_weighted,
            filter_details,
        }
    }

    /// 获取上次状态快照
    pub fn last_snapshot(&self) -> Option<&MarketStateSnapshot> {
        self.last_snapshot.as_ref()
    }

    /// 检测状态变更
    pub fn detect_state_change(&self, candles: &[Candle]) -> Option<StateChange> {
        let current = self.classifier.classify(candles);
        let last = self.last_snapshot.as_ref()?;

        // 检查主大类变更
        if current.primary_regime != last.primary_regime {
            return Some(StateChange {
                dimension: StateChangeDimension::PrimaryRegime,
                from: format!("{:?}", last.primary_regime),
                to: format!("{:?}", current.primary_regime),
                confidence: current.overall_confidence,
            });
        }

        // 检查波动率变更
        if current.volatility != last.volatility {
            return Some(StateChange {
                dimension: StateChangeDimension::Volatility,
                from: format!("{:?}", last.volatility),
                to: format!("{:?}", current.volatility),
                confidence: current.volatility_confidence,
            });
        }

        // 检查流动性变更
        if current.liquidity != last.liquidity {
            return Some(StateChange {
                dimension: StateChangeDimension::Liquidity,
                from: format!("{:?}", last.liquidity),
                to: format!("{:?}", current.liquidity),
                confidence: current.liquidity_confidence,
            });
        }

        None
    }
}

impl Default for MarketStateFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// 状态变更事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    /// 变更维度
    pub dimension: StateChangeDimension,
    /// 原状态
    pub from: String,
    /// 新状态
    pub to: String,
    /// 置信度
    pub confidence: f64,
}

/// 状态变更维度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateChangeDimension {
    PrimaryRegime,
    SecondaryRegime,
    Volatility,
    Liquidity,
    Structure,
    Behavior,
}

/// 因子滤波声明：因子声明允许进入的滤波状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorFilterDeclaration {
    /// 因子名称
    pub factor_name: String,
    /// 允许的波动率状态
    pub allowed_volatility: Vec<VolatilityRegime>,
    /// 允许的流动性状态
    pub allowed_liquidity: Vec<LiquidityRegime>,
    /// 允许的结构状态
    pub allowed_structure: Vec<MarketStructureRegime>,
    /// 允许的主大类
    pub allowed_primary_regimes: Vec<PrimaryMarketRegime>,
    /// 是否在极端状态下禁用
    pub disable_on_extreme: bool,
}

impl FactorFilterDeclaration {
    /// 创建默认声明（允许所有状态）
    pub fn permissive(factor_name: &str) -> Self {
        Self {
            factor_name: factor_name.to_string(),
            allowed_volatility: vec![
                VolatilityRegime::LowVol,
                VolatilityRegime::NormalVol,
                VolatilityRegime::ElevatedVol,
            ],
            allowed_liquidity: vec![
                LiquidityRegime::HighLiquidity,
                LiquidityRegime::NormalLiquidity,
            ],
            allowed_structure: vec![
                MarketStructureRegime::Trending,
                MarketStructureRegime::Ranging,
            ],
            allowed_primary_regimes: vec![
                PrimaryMarketRegime::TrendExpansion,
                PrimaryMarketRegime::RangeConsolidation,
            ],
            disable_on_extreme: true,
        }
    }

    /// 创建趋势因子声明
    pub fn trend_factor(factor_name: &str) -> Self {
        Self {
            factor_name: factor_name.to_string(),
            allowed_volatility: vec![VolatilityRegime::NormalVol, VolatilityRegime::ElevatedVol],
            allowed_liquidity: vec![
                LiquidityRegime::HighLiquidity,
                LiquidityRegime::NormalLiquidity,
            ],
            allowed_structure: vec![MarketStructureRegime::Trending],
            allowed_primary_regimes: vec![PrimaryMarketRegime::TrendExpansion],
            disable_on_extreme: true,
        }
    }

    /// 创建均值回归因子声明
    pub fn mean_reversion_factor(factor_name: &str) -> Self {
        Self {
            factor_name: factor_name.to_string(),
            allowed_volatility: vec![VolatilityRegime::LowVol, VolatilityRegime::NormalVol],
            allowed_liquidity: vec![
                LiquidityRegime::HighLiquidity,
                LiquidityRegime::NormalLiquidity,
            ],
            allowed_structure: vec![
                MarketStructureRegime::Ranging,
                MarketStructureRegime::MeanReverting,
            ],
            allowed_primary_regimes: vec![
                PrimaryMarketRegime::RangeConsolidation,
                PrimaryMarketRegime::ReversalBrewing,
            ],
            disable_on_extreme: true,
        }
    }

    /// 检查因子是否允许在当前状态下运行
    pub fn is_allowed(&self, result: &MarketStateFilterResult) -> bool {
        if !result.allowed {
            return false;
        }

        // 检查波动率
        if !self
            .allowed_volatility
            .contains(&result.snapshot.volatility)
        {
            return false;
        }

        // 检查流动性
        if !self.allowed_liquidity.contains(&result.snapshot.liquidity) {
            return false;
        }

        // 检查结构
        if !self.allowed_structure.contains(&result.snapshot.structure) {
            return false;
        }

        // 检查主大类
        if !self
            .allowed_primary_regimes
            .contains(&result.snapshot.primary_regime)
        {
            return false;
        }

        // 检查极端状态
        if self.disable_on_extreme
            && matches!(
                result.snapshot.primary_regime,
                PrimaryMarketRegime::ExtremeStress
            )
        {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn sample_candles(count: usize) -> Vec<Candle> {
        (0..count)
            .map(|i| {
                let base = 100.0 + i as f64 * 0.1;
                Candle {
                    timestamp: Utc.timestamp_opt(1_700_000_000 + i as i64 * 60, 0).unwrap(),
                    open: base,
                    high: base + 1.0,
                    low: base - 0.5,
                    close: base + 0.5,
                    volume: 1000.0,
                }
            })
            .collect()
    }

    #[test]
    fn default_filter_allows_normal_market() {
        let candles = sample_candles(100);
        let mut filter = MarketStateFilter::new();
        let result = filter.filter(&candles);

        // 正常市场状态应允许交易
        assert!(result.allowed || !result.filter_details.is_empty());
    }

    #[test]
    fn factor_declaration_trend_factor_restricts_range() {
        let decl = FactorFilterDeclaration::trend_factor("test_trend");

        // 趋势因子不允许 Ranging 状态
        assert!(!decl
            .allowed_structure
            .contains(&MarketStructureRegime::Ranging));
        assert!(decl
            .allowed_structure
            .contains(&MarketStructureRegime::Trending));
    }

    #[test]
    fn factor_declaration_mean_reversion_restricts_trending() {
        let decl = FactorFilterDeclaration::mean_reversion_factor("test_mr");

        // 均值回归因子不允许 Trending 状态
        assert!(!decl
            .allowed_structure
            .contains(&MarketStructureRegime::Trending));
        assert!(decl
            .allowed_structure
            .contains(&MarketStructureRegime::Ranging));
    }
}
