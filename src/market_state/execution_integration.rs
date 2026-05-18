//! Market State to Execution Tree Integration
//!
//! 将市场状态分类结果集成到执行树决策流程
//! 流水线：市场状态 → BBN 证据 → PolicyFeatureVector → 执行树决策
//!
//! 设计原则：
//! - 零配置：默认参数直接可用
//! - 热插拔：用户可覆盖配置
//! - Token 友好：简洁输出
//! - 高置信度：基于统计学阈值

use crate::application::orchestration::PolicyFeatureVector;
use crate::bbn::Evidence;
use crate::market_state::mtf_resonance::TimeframeResonanceResult;
use crate::market_state::{
    EvidenceMappingConfig, MarketStateEvidenceMapper, MarketStateSnapshot, PrimaryMarketRegime,
};
use serde::{Deserialize, Serialize};

/// 执行树配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTreeConfig {
    /// 危机状态：阻断所有执行
    pub crisis_veto_enabled: bool,
    /// 低置信度阈值：低于此值转为 observe_only
    pub low_confidence_threshold: f64,
    /// 共振对齐加权系数
    pub resonance_weight: f64,
    /// 多周期矛盾降权系数
    pub contradiction_penalty: f64,
}

impl Default for ExecutionTreeConfig {
    fn default() -> Self {
        Self {
            crisis_veto_enabled: true,
            low_confidence_threshold: 0.45,
            resonance_weight: 0.15,
            contradiction_penalty: 0.25,
        }
    }
}

/// 市场状态执行决策
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketStateExecutionDecision {
    /// 执行许可：allow / veto / observe_only
    pub permission: ExecutionPermission,
    /// 市场状态摘要
    pub regime_summary: RegimeSummary,
    /// 调整后的置信度
    pub adjusted_confidence: f64,
    /// 阻断原因（若有）
    pub veto_reason: Option<String>,
    /// 共振影响
    pub resonance_impact: Option<ResonanceImpact>,
}

/// 执行许可
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionPermission {
    /// 允许执行
    Allow,
    /// 阻断执行
    Veto,
    /// 仅观察
    ObserveOnly,
}

/// 市场状态摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeSummary {
    /// 主大类
    pub primary: String,
    /// 波动率状态
    pub volatility: String,
    /// 流动性状态
    pub liquidity: String,
    /// 原始置信度
    pub base_confidence: f64,
}

/// 共振影响
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceImpact {
    /// 共振类型：aligned / contradicted / neutral
    pub resonance_type: String,
    /// 共振分数
    pub score: f64,
    /// 置信度调整
    pub confidence_adjustment: f64,
}

/// 市场状态执行树集成器
pub struct MarketStateExecutionIntegrator {
    config: ExecutionTreeConfig,
    evidence_mapper: MarketStateEvidenceMapper,
}

impl MarketStateExecutionIntegrator {
    pub fn new() -> Self {
        Self::with_configs(
            ExecutionTreeConfig::default(),
            EvidenceMappingConfig::default(),
        )
    }

    pub fn with_configs(
        execution_config: ExecutionTreeConfig,
        evidence_config: EvidenceMappingConfig,
    ) -> Self {
        Self {
            config: execution_config,
            evidence_mapper: MarketStateEvidenceMapper::with_config(evidence_config),
        }
    }

    /// 评估市场状态并生成执行决策
    pub fn evaluate(&self, snapshot: &MarketStateSnapshot) -> MarketStateExecutionDecision {
        self.evaluate_with_resonance(snapshot, None)
    }

    /// 评估市场状态 + 多周期共振
    pub fn evaluate_with_resonance(
        &self,
        snapshot: &MarketStateSnapshot,
        resonance: Option<&TimeframeResonanceResult>,
    ) -> MarketStateExecutionDecision {
        // 1. 检查危机状态
        if self.config.crisis_veto_enabled && self.is_crisis_state(&snapshot.primary_regime) {
            return MarketStateExecutionDecision {
                permission: ExecutionPermission::Veto,
                regime_summary: self.build_regime_summary(snapshot),
                adjusted_confidence: 0.0,
                veto_reason: Some("crisis_volatility_detected".to_string()),
                resonance_impact: None,
            };
        }

        // 2. 计算基础置信度
        let mut adjusted_confidence = snapshot.overall_confidence;

        // 3. 应用共振影响
        let resonance_impact =
            resonance.map(|r| self.apply_resonance_impact(r, &mut adjusted_confidence));

        // 4. 决定执行许可
        let (permission, veto_reason) =
            if adjusted_confidence < self.config.low_confidence_threshold {
                (
                    ExecutionPermission::ObserveOnly,
                    Some("low_confidence".to_string()),
                )
            } else {
                (ExecutionPermission::Allow, None)
            };

        MarketStateExecutionDecision {
            permission,
            regime_summary: self.build_regime_summary(snapshot),
            adjusted_confidence,
            veto_reason,
            resonance_impact,
        }
    }

    /// 映射市场状态到 BBN 证据
    pub fn map_to_evidence(&self, snapshot: &MarketStateSnapshot) -> Evidence {
        self.evidence_mapper.map_to_evidence(snapshot)
    }

    /// 映射共振结果到 BBN 证据
    pub fn map_resonance_to_evidence(&self, resonance: &TimeframeResonanceResult) -> Evidence {
        self.evidence_mapper.map_resonance_to_evidence(resonance)
    }

    /// 增强 PolicyFeatureVector
    pub fn enhance_feature_vector(
        &self,
        features: &mut PolicyFeatureVector,
        decision: &MarketStateExecutionDecision,
    ) {
        // 根据执行许可设置 gating_status
        match decision.permission {
            ExecutionPermission::Allow => {
                features.gating_status = "ready".to_string();
            }
            ExecutionPermission::Veto => {
                features.gating_status = "veto".to_string();
            }
            ExecutionPermission::ObserveOnly => {
                features.gating_status = "observe_only".to_string();
            }
        }

        // 根据主大类设置 factor_alignment
        features.factor_alignment = decision.regime_summary.primary.clone();

        // 应用调整后的置信度
        features.evidence_quality_score = decision.adjusted_confidence;

        // 应用共振影响
        if let Some(ref impact) = decision.resonance_impact {
            if impact.resonance_type == "contradicted" {
                features.factor_uncertainty = "high".to_string();
            } else if impact.resonance_type == "aligned" {
                features.factor_uncertainty = "low".to_string();
            }
        }
    }

    /// 检查是否为危机状态
    fn is_crisis_state(&self, regime: &PrimaryMarketRegime) -> bool {
        matches!(regime, PrimaryMarketRegime::ExtremeStress)
    }

    /// 应用共振影响
    fn apply_resonance_impact(
        &self,
        resonance: &TimeframeResonanceResult,
        confidence: &mut f64,
    ) -> ResonanceImpact {
        use crate::market_state::mtf_resonance::ResonanceResult;

        let (resonance_type, adjustment) = match resonance.primary_regime_resonance {
            ResonanceResult::Aligned => ("aligned".to_string(), self.config.resonance_weight),
            ResonanceResult::Contradicted => (
                "contradicted".to_string(),
                -self.config.contradiction_penalty,
            ),
            ResonanceResult::Neutral => ("neutral".to_string(), 0.0),
            ResonanceResult::Missing => ("missing".to_string(), 0.0),
        };

        *confidence = (*confidence + adjustment).clamp(0.0, 1.0);

        ResonanceImpact {
            resonance_type,
            score: resonance.overall_resonance_score,
            confidence_adjustment: adjustment,
        }
    }

    /// 构建市场状态摘要
    fn build_regime_summary(&self, snapshot: &MarketStateSnapshot) -> RegimeSummary {
        RegimeSummary {
            primary: format!("{:?}", snapshot.primary_regime),
            volatility: format!("{:?}", snapshot.volatility),
            liquidity: format!("{:?}", snapshot.liquidity),
            base_confidence: snapshot.overall_confidence,
        }
    }
}

impl Default for MarketStateExecutionIntegrator {
    fn default() -> Self {
        Self::new()
    }
}

/// 执行树管道：市场状态 → BBN → 执行决策
pub struct ExecutionTreePipeline {
    integrator: MarketStateExecutionIntegrator,
}

impl ExecutionTreePipeline {
    pub fn new() -> Self {
        Self::with_config(ExecutionTreeConfig::default())
    }

    pub fn with_config(config: ExecutionTreeConfig) -> Self {
        Self {
            integrator: MarketStateExecutionIntegrator::with_configs(
                config,
                EvidenceMappingConfig::default(),
            ),
        }
    }

    /// 完整流水线执行
    pub fn execute(
        &self,
        snapshot: &MarketStateSnapshot,
        resonance: Option<&TimeframeResonanceResult>,
    ) -> PipelineResult {
        // 1. 评估执行决策
        let decision = self.integrator.evaluate_with_resonance(snapshot, resonance);

        // 2. 映射 BBN 证据
        let evidence = match resonance {
            Some(r) => self.integrator.map_resonance_to_evidence(r),
            None => self.integrator.map_to_evidence(snapshot),
        };

        // 3. 创建增强后的特征向量
        let mut features = PolicyFeatureVector::default();
        self.integrator
            .enhance_feature_vector(&mut features, &decision);

        PipelineResult {
            decision,
            evidence,
            enhanced_features: features,
        }
    }
}

impl Default for ExecutionTreePipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// 流水线执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    /// 执行决策
    pub decision: MarketStateExecutionDecision,
    /// BBN 证据
    pub evidence: Evidence,
    /// 增强后的特征向量
    pub enhanced_features: PolicyFeatureVector,
}

impl PipelineResult {
    /// 是否允许执行
    pub fn is_allowed(&self) -> bool {
        self.decision.permission == ExecutionPermission::Allow
    }

    /// 是否被阻断
    pub fn is_vetoed(&self) -> bool {
        self.decision.permission == ExecutionPermission::Veto
    }

    /// 获取调整后置信度
    pub fn confidence(&self) -> f64 {
        self.decision.adjusted_confidence
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
    fn integrator_evaluates_normal_state() {
        let classifier = MarketStateClassifier::new();
        let integrator = MarketStateExecutionIntegrator::new();

        let candles = sample_candles(100);
        let snapshot = classifier.classify(&candles);
        let decision = integrator.evaluate(&snapshot);

        // 正常状态不应被阻断
        assert_ne!(decision.permission, ExecutionPermission::Veto);
        assert!(decision.adjusted_confidence >= 0.0);
    }

    #[test]
    fn pipeline_produces_complete_result() {
        let classifier = MarketStateClassifier::new();
        let pipeline = ExecutionTreePipeline::new();

        let candles = sample_candles(100);
        let snapshot = classifier.classify(&candles);
        let result = pipeline.execute(&snapshot, None);

        // 结果完整
        assert!(result.evidence.len() >= 5);
        assert!(!result.enhanced_features.gating_status.is_empty());
    }

    #[test]
    fn crisis_state_triggers_veto() {
        let snapshot = MarketStateSnapshot {
            primary_regime: PrimaryMarketRegime::ExtremeStress,
            overall_confidence: 0.8,
            ..MarketStateSnapshot::default()
        };

        let integrator = MarketStateExecutionIntegrator::new();
        let decision = integrator.evaluate(&snapshot);

        assert_eq!(decision.permission, ExecutionPermission::Veto);
        assert!(decision.veto_reason.is_some());
    }

    #[test]
    fn low_confidence_triggers_observe_only() {
        let snapshot = MarketStateSnapshot {
            primary_regime: PrimaryMarketRegime::RangeConsolidation,
            overall_confidence: 0.3, // 低于阈值
            ..MarketStateSnapshot::default()
        };

        let integrator = MarketStateExecutionIntegrator::new();
        let decision = integrator.evaluate(&snapshot);

        assert_eq!(decision.permission, ExecutionPermission::ObserveOnly);
    }
}
