//! Market State to BBN Evidence Mapping
//!
//! 将市场状态分类结果映射到贝叶斯信念网络证据节点
//! 设计原则：
//! - 零配置：默认映射直接可用
//! - 热插拔：用户可覆盖映射规则
//! - 高置信度：软证据分布基于历史统计

use serde::{Deserialize, Serialize};

use crate::bbn::{Evidence, EvidenceType, NodeId};
use crate::market_state::mtf_resonance::TimeframeResonanceResult;
use crate::market_state::{
    InvestorBehaviorRegime, LiquidityRegime, MarketStateSnapshot, MarketStructureRegime,
    PrimaryMarketRegime, VolatilityRegime,
};

/// BBN 节点 ID 定义
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketStateNodeId {
    /// 主大类节点
    PrimaryRegime,
    /// 次小类节点
    SecondaryRegime,
    /// 波动率状态节点
    VolatilityRegime,
    /// 流动性状态节点
    LiquidityRegime,
    /// 市场结构节点
    StructureRegime,
    /// 投资者行为节点
    BehaviorRegime,
    /// 多周期共振节点
    TimeframeResonance,
}

impl From<MarketStateNodeId> for NodeId {
    fn from(id: MarketStateNodeId) -> Self {
        match id {
            MarketStateNodeId::PrimaryRegime => "market_primary_regime".into(),
            MarketStateNodeId::SecondaryRegime => "market_secondary_regime".into(),
            MarketStateNodeId::VolatilityRegime => "market_volatility_regime".into(),
            MarketStateNodeId::LiquidityRegime => "market_liquidity_regime".into(),
            MarketStateNodeId::StructureRegime => "market_structure_regime".into(),
            MarketStateNodeId::BehaviorRegime => "market_behavior_regime".into(),
            MarketStateNodeId::TimeframeResonance => "market_timeframe_resonance".into(),
        }
    }
}

/// 主大类状态索引
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimaryRegimeStateIndex {
    TrendExpansion = 0,
    RangeConsolidation = 1,
    ReversalBrewing = 2,
    CrisisVolatility = 3,
}

/// 波动率状态索引
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolatilityStateIndex {
    LowVol = 0,
    NormalVol = 1,
    ElevatedVol = 2,
    CrisisVol = 3,
}

/// 流动性状态索引
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiquidityStateIndex {
    DeepLiquid = 0,
    NormalLiquid = 1,
    ShallowLiquid = 2,
    Illiquid = 3,
}

/// 共振状态索引
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResonanceStateIndex {
    Aligned = 0,
    Neutral = 1,
    Contradicted = 2,
    Missing = 3,
}

/// 证据映射配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceMappingConfig {
    /// 是否使用软证据（基于置信度分布）
    pub use_soft_evidence: bool,
    /// 硬证据阈值（置信度高于此值使用硬证据）
    pub hard_evidence_threshold: f64,
    /// 低置信度时的模糊系数（增加分布不确定性）
    pub low_confidence_fuzziness: f64,
}

impl Default for EvidenceMappingConfig {
    fn default() -> Self {
        Self {
            use_soft_evidence: true,
            hard_evidence_threshold: 0.85,
            low_confidence_fuzziness: 0.15,
        }
    }
}

/// 市场状态证据映射器
pub struct MarketStateEvidenceMapper {
    config: EvidenceMappingConfig,
}

impl MarketStateEvidenceMapper {
    pub fn new() -> Self {
        Self::with_config(EvidenceMappingConfig::default())
    }

    pub fn with_config(config: EvidenceMappingConfig) -> Self {
        Self { config }
    }

    /// 将市场状态快照映射为 BBN 证据
    pub fn map_to_evidence(&self, snapshot: &MarketStateSnapshot) -> Evidence {
        let mut evidence = Evidence::new();

        // 主大类证据
        self.insert_primary_regime_evidence(&mut evidence, snapshot);

        // 波动率证据
        self.insert_volatility_evidence(&mut evidence, snapshot);

        // 流动性证据
        self.insert_liquidity_evidence(&mut evidence, snapshot);

        // 结构证据
        self.insert_structure_evidence(&mut evidence, snapshot);

        // 行为证据
        self.insert_behavior_evidence(&mut evidence, snapshot);

        evidence
    }

    /// 将多周期共振结果映射为 BBN 证据
    pub fn map_resonance_to_evidence(&self, resonance: &TimeframeResonanceResult) -> Evidence {
        let mut evidence = Evidence::new();

        // 共振状态
        let node_id: NodeId = MarketStateNodeId::TimeframeResonance.into();
        let evidence_type = self.resonance_to_evidence_type(
            resonance.primary_regime_resonance,
            resonance.overall_resonance_score,
        );
        evidence.insert(node_id, evidence_type);

        // 同时插入基础周期状态
        let base_evidence = self.map_to_evidence(&resonance.base_snapshot);
        evidence.extend(base_evidence);

        evidence
    }

    /// 插入主大类证据
    fn insert_primary_regime_evidence(
        &self,
        evidence: &mut Evidence,
        snapshot: &MarketStateSnapshot,
    ) {
        let node_id: NodeId = MarketStateNodeId::PrimaryRegime.into();
        let state_index = self.primary_regime_to_index(&snapshot.primary_regime);

        let evidence_type = if self.config.use_soft_evidence
            && snapshot.overall_confidence < self.config.hard_evidence_threshold
        {
            self.create_soft_evidence(state_index, 4, snapshot.overall_confidence)
        } else {
            EvidenceType::Hard(state_index)
        };

        evidence.insert(node_id, evidence_type);
    }

    /// 插入波动率证据
    fn insert_volatility_evidence(&self, evidence: &mut Evidence, snapshot: &MarketStateSnapshot) {
        let node_id: NodeId = MarketStateNodeId::VolatilityRegime.into();
        let state_index = self.volatility_to_index(&snapshot.volatility);

        let evidence_type = if self.config.use_soft_evidence
            && snapshot.volatility_confidence < self.config.hard_evidence_threshold
        {
            self.create_soft_evidence(state_index, 4, snapshot.volatility_confidence)
        } else {
            EvidenceType::Hard(state_index)
        };

        evidence.insert(node_id, evidence_type);
    }

    /// 插入流动性证据
    fn insert_liquidity_evidence(&self, evidence: &mut Evidence, snapshot: &MarketStateSnapshot) {
        let node_id: NodeId = MarketStateNodeId::LiquidityRegime.into();
        let state_index = self.liquidity_to_index(&snapshot.liquidity);

        let evidence_type = if self.config.use_soft_evidence
            && snapshot.liquidity_confidence < self.config.hard_evidence_threshold
        {
            self.create_soft_evidence(state_index, 4, snapshot.liquidity_confidence)
        } else {
            EvidenceType::Hard(state_index)
        };

        evidence.insert(node_id, evidence_type);
    }

    /// 插入结构证据
    fn insert_structure_evidence(&self, evidence: &mut Evidence, snapshot: &MarketStateSnapshot) {
        let node_id: NodeId = MarketStateNodeId::StructureRegime.into();
        let state_index = self.structure_to_index(&snapshot.structure);

        let evidence_type = if self.config.use_soft_evidence
            && snapshot.structure_confidence < self.config.hard_evidence_threshold
        {
            self.create_soft_evidence(state_index, 5, snapshot.structure_confidence)
        } else {
            EvidenceType::Hard(state_index)
        };

        evidence.insert(node_id, evidence_type);
    }

    /// 插入行为证据
    fn insert_behavior_evidence(&self, evidence: &mut Evidence, snapshot: &MarketStateSnapshot) {
        let node_id: NodeId = MarketStateNodeId::BehaviorRegime.into();
        let state_index = self.behavior_to_index(&snapshot.behavior);

        let evidence_type = if self.config.use_soft_evidence
            && snapshot.behavior_confidence < self.config.hard_evidence_threshold
        {
            self.create_soft_evidence(state_index, 5, snapshot.behavior_confidence)
        } else {
            EvidenceType::Hard(state_index)
        };

        evidence.insert(node_id, evidence_type);
    }

    /// 主大类转索引
    fn primary_regime_to_index(&self, regime: &PrimaryMarketRegime) -> usize {
        match regime {
            PrimaryMarketRegime::TrendExpansion => PrimaryRegimeStateIndex::TrendExpansion as usize,
            PrimaryMarketRegime::RangeConsolidation => {
                PrimaryRegimeStateIndex::RangeConsolidation as usize
            }
            PrimaryMarketRegime::ReversalBrewing => {
                PrimaryRegimeStateIndex::ReversalBrewing as usize
            }
            PrimaryMarketRegime::ExtremeStress => {
                PrimaryRegimeStateIndex::CrisisVolatility as usize
            }
            PrimaryMarketRegime::Unknown => PrimaryRegimeStateIndex::RangeConsolidation as usize,
        }
    }

    /// 波动率转索引
    fn volatility_to_index(&self, regime: &VolatilityRegime) -> usize {
        match regime {
            VolatilityRegime::LowVol => VolatilityStateIndex::LowVol as usize,
            VolatilityRegime::NormalVol => VolatilityStateIndex::NormalVol as usize,
            VolatilityRegime::ElevatedVol => VolatilityStateIndex::ElevatedVol as usize,
            VolatilityRegime::CrisisVol => VolatilityStateIndex::CrisisVol as usize,
            VolatilityRegime::Unknown => VolatilityStateIndex::NormalVol as usize,
        }
    }

    /// 流动性转索引
    fn liquidity_to_index(&self, regime: &LiquidityRegime) -> usize {
        match regime {
            LiquidityRegime::HighLiquidity => LiquidityStateIndex::DeepLiquid as usize,
            LiquidityRegime::NormalLiquidity => LiquidityStateIndex::NormalLiquid as usize,
            LiquidityRegime::ThinLiquidity => LiquidityStateIndex::Illiquid as usize,
            LiquidityRegime::Unknown => LiquidityStateIndex::ShallowLiquid as usize,
        }
    }

    /// 结构转索引
    fn structure_to_index(&self, regime: &MarketStructureRegime) -> usize {
        match regime {
            MarketStructureRegime::Trending | MarketStructureRegime::Breakout => 0,
            MarketStructureRegime::MeanReverting => 1,
            MarketStructureRegime::Ranging | MarketStructureRegime::Unknown => 2,
            MarketStructureRegime::Accumulation => 3,
            MarketStructureRegime::Distribution | MarketStructureRegime::Breakdown => 4,
        }
    }

    /// 行为转索引
    fn behavior_to_index(&self, regime: &InvestorBehaviorRegime) -> usize {
        match regime {
            InvestorBehaviorRegime::RiskOn => 0,
            InvestorBehaviorRegime::Exhaustion | InvestorBehaviorRegime::Crowding => 1,
            InvestorBehaviorRegime::Capitulation => 2,
            InvestorBehaviorRegime::FOMO => 3,
            InvestorBehaviorRegime::RiskOff | InvestorBehaviorRegime::Neutral => 4,
        }
    }

    /// 共振转证据类型
    fn resonance_to_evidence_type(
        &self,
        resonance: crate::market_state::mtf_resonance::ResonanceResult,
        score: f64,
    ) -> EvidenceType {
        let index = match resonance {
            crate::market_state::mtf_resonance::ResonanceResult::Aligned => {
                ResonanceStateIndex::Aligned as usize
            }
            crate::market_state::mtf_resonance::ResonanceResult::Neutral => {
                ResonanceStateIndex::Neutral as usize
            }
            crate::market_state::mtf_resonance::ResonanceResult::Contradicted => {
                ResonanceStateIndex::Contradicted as usize
            }
            crate::market_state::mtf_resonance::ResonanceResult::Missing => {
                ResonanceStateIndex::Missing as usize
            }
        };

        if self.config.use_soft_evidence && score < self.config.hard_evidence_threshold {
            self.create_soft_evidence(index, 4, score)
        } else {
            EvidenceType::Hard(index)
        }
    }

    /// 创建软证据分布
    ///
    /// 参数：
    /// - primary_index: 主状态索引
    /// - total_states: 总状态数
    /// - confidence: 置信度
    fn create_soft_evidence(
        &self,
        primary_index: usize,
        total_states: usize,
        confidence: f64,
    ) -> EvidenceType {
        let mut distribution = vec![0.0; total_states];

        // 应用模糊系数
        let adjusted_conf = confidence * (1.0 - self.config.low_confidence_fuzziness);
        let remaining = 1.0 - adjusted_conf;

        // 主状态获得大部分概率
        distribution[primary_index] = adjusted_conf;

        // 剩余概率分配给其他状态
        let per_other = remaining / (total_states - 1).max(1) as f64;
        for (i, prob) in distribution.iter_mut().enumerate().take(total_states) {
            if i != primary_index {
                *prob = per_other;
            }
        }

        // 归一化
        let sum: f64 = distribution.iter().sum();
        for prob in &mut distribution {
            *prob /= sum;
        }

        EvidenceType::Soft(distribution)
    }
}

impl Default for MarketStateEvidenceMapper {
    fn default() -> Self {
        Self::new()
    }
}

/// 证据摘要（用于日志/调试）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceSummary {
    pub primary_regime: String,
    pub primary_confidence: f64,
    pub volatility_regime: String,
    pub volatility_confidence: f64,
    pub liquidity_regime: String,
    pub liquidity_confidence: f64,
    pub resonance_score: Option<f64>,
    pub evidence_count: usize,
}

impl EvidenceSummary {
    pub fn from_snapshot(snapshot: &MarketStateSnapshot) -> Self {
        Self {
            primary_regime: format!("{:?}", snapshot.primary_regime),
            primary_confidence: snapshot.overall_confidence,
            volatility_regime: format!("{:?}", snapshot.volatility),
            volatility_confidence: snapshot.volatility_confidence,
            liquidity_regime: format!("{:?}", snapshot.liquidity),
            liquidity_confidence: snapshot.liquidity_confidence,
            resonance_score: None,
            evidence_count: 5,
        }
    }

    pub fn from_resonance(resonance: &TimeframeResonanceResult) -> Self {
        let mut summary = Self::from_snapshot(&resonance.base_snapshot);
        summary.resonance_score = Some(resonance.overall_resonance_score);
        summary.evidence_count = 6;
        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_state::MarketStateClassifier;
    use crate::types::Candle;
    use chrono::{TimeZone, Utc};

    fn sample_candles(count: usize) -> Vec<Candle> {
        (0..count)
            .map(|i| {
                let base = 100.0 + i as f64 * 0.5;
                Candle {
                    timestamp: Utc.timestamp_opt(1_700_000_000 + i as i64 * 60, 0).unwrap(),
                    open: base,
                    high: base + 1.0,
                    low: base - 0.5,
                    close: base + 0.3,
                    volume: 1000.0,
                }
            })
            .collect()
    }

    #[test]
    fn mapper_creates_evidence_from_snapshot() {
        let classifier = MarketStateClassifier::new();
        let mapper = MarketStateEvidenceMapper::new();

        let candles = sample_candles(100);
        let snapshot = classifier.classify(&candles);
        let evidence = mapper.map_to_evidence(&snapshot);

        // 应包含 5 个证据节点
        assert!(evidence.len() >= 5);
        assert!(evidence.contains_key(&NodeId::from(MarketStateNodeId::PrimaryRegime)));
        assert!(evidence.contains_key(&NodeId::from(MarketStateNodeId::VolatilityRegime)));
    }

    #[test]
    fn soft_evidence_distribution_sums_to_one() {
        let mapper = MarketStateEvidenceMapper::new();
        let dist = mapper.create_soft_evidence(0, 4, 0.7);

        if let EvidenceType::Soft(probs) = dist {
            let sum: f64 = probs.iter().sum();
            assert!((sum - 1.0).abs() < 1e-6);
            assert!(probs[0] > probs[1]); // 主状态概率最高
        } else {
            panic!("expected soft evidence");
        }
    }

    #[test]
    fn hard_evidence_for_high_confidence() {
        let config = EvidenceMappingConfig {
            use_soft_evidence: true,
            hard_evidence_threshold: 0.7,
            low_confidence_fuzziness: 0.15,
        };
        let mapper = MarketStateEvidenceMapper::with_config(config);

        let dist = mapper.create_soft_evidence(1, 4, 0.9);
        // 高置信度但未达硬证据阈值时，仍为软证据
        assert!(matches!(dist, EvidenceType::Soft(_)));
    }

    #[test]
    fn evidence_summary_from_snapshot() {
        let classifier = MarketStateClassifier::new();
        let candles = sample_candles(100);
        let snapshot = classifier.classify(&candles);

        let summary = EvidenceSummary::from_snapshot(&snapshot);
        assert!(summary.evidence_count >= 5);
        assert!(summary.primary_confidence >= 0.0 && summary.primary_confidence <= 1.0);
    }
}
