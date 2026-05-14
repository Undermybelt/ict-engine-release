//! Market State Configuration
//!
//! 热插拔配置：用户可覆盖各分类器阈值和聚合权重
//! 零配置：默认值可直接使用

use serde::{Deserialize, Serialize};
use std::path::Path;

use super::behavior::BehaviorThresholds;
use super::enhanced_aggregation::EnhancedAggregationConfig;
use super::liquidity::LiquidityThresholds;
use super::structure::StructureThresholds;
use super::volatility::VolatilityThresholds;

/// 市场状态分类器总配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketStateConfig {
    /// 波动率分类器阈值
    pub volatility: VolatilityThresholds,
    /// 流动性分类器阈值
    pub liquidity: LiquidityThresholds,
    /// 市场结构分类器阈值
    pub structure: StructureThresholds,
    /// 投资者行为分类器阈值
    pub behavior: BehaviorThresholds,
    /// 聚合权重
    pub aggregate_weights: AggregateWeights,
    /// 增强聚合器配置
    #[serde(default)]
    pub enhanced_aggregation: EnhancedAggregationConfig,
    /// 是否启用各分类器
    pub enabled: EnabledClassifiers,
    /// 用户自定义标签（可选）
    pub user_label: Option<String>,
}

/// 聚合权重：各维度对最终分类的贡献
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateWeights {
    /// 波动率权重
    pub volatility: f64,
    /// 流动性权重
    pub liquidity: f64,
    /// 市场结构权重
    pub structure: f64,
    /// 投资者行为权重
    pub behavior: f64,
}

impl Default for AggregateWeights {
    fn default() -> Self {
        Self {
            volatility: 0.30,
            liquidity: 0.20,
            structure: 0.30,
            behavior: 0.20,
        }
    }
}

/// 启用/禁用分类器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnabledClassifiers {
    pub volatility: bool,
    pub liquidity: bool,
    pub structure: bool,
    pub behavior: bool,
}

impl Default for EnabledClassifiers {
    fn default() -> Self {
        Self {
            volatility: true,
            liquidity: true,
            structure: true,
            behavior: true,
        }
    }
}

/// 市场状态配置 Profile：预设模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketStateProfile {
    pub name: String,
    pub description: String,
    pub config: MarketStateConfig,
}

impl MarketStateProfile {
    /// 默认配置
    pub fn default_profile() -> Self {
        Self {
            name: "default".to_string(),
            description: "均衡配置：各维度权重均等".to_string(),
            config: MarketStateConfig::default(),
        }
    }

    /// 趋势交易配置：强调结构和行为
    pub fn trend_trading_profile() -> Self {
        Self {
            name: "trend_trading".to_string(),
            description: "趋势交易：强调市场结构和投资者行为".to_string(),
            config: MarketStateConfig {
                aggregate_weights: AggregateWeights {
                    volatility: 0.20,
                    liquidity: 0.15,
                    structure: 0.40,
                    behavior: 0.25,
                },
                enhanced_aggregation: EnhancedAggregationConfig {
                    volatility_weight: 0.20,
                    liquidity_weight: 0.15,
                    structure_weight: 0.40,
                    behavior_weight: 0.25,
                    ..EnhancedAggregationConfig::default()
                },
                ..MarketStateConfig::default()
            },
        }
    }

    /// 波动率交易配置：强调波动率和流动性
    pub fn volatility_trading_profile() -> Self {
        Self {
            name: "volatility_trading".to_string(),
            description: "波动率交易：强调波动率和流动性状态".to_string(),
            config: MarketStateConfig {
                aggregate_weights: AggregateWeights {
                    volatility: 0.40,
                    liquidity: 0.30,
                    structure: 0.15,
                    behavior: 0.15,
                },
                enhanced_aggregation: EnhancedAggregationConfig {
                    volatility_weight: 0.40,
                    liquidity_weight: 0.30,
                    structure_weight: 0.15,
                    behavior_weight: 0.15,
                    ..EnhancedAggregationConfig::default()
                },
                ..MarketStateConfig::default()
            },
        }
    }

    /// 反转交易配置：强调行为极端
    pub fn reversal_trading_profile() -> Self {
        Self {
            name: "reversal_trading".to_string(),
            description: "反转交易：强调投资者行为极端和结构弱化".to_string(),
            config: MarketStateConfig {
                aggregate_weights: AggregateWeights {
                    volatility: 0.25,
                    liquidity: 0.15,
                    structure: 0.30,
                    behavior: 0.30,
                },
                enhanced_aggregation: EnhancedAggregationConfig {
                    volatility_weight: 0.25,
                    liquidity_weight: 0.15,
                    structure_weight: 0.30,
                    behavior_weight: 0.30,
                    ..EnhancedAggregationConfig::default()
                },
                behavior: BehaviorThresholds {
                    rsi_extreme_threshold: 70.0, // 更宽松的极端阈值
                    ..BehaviorThresholds::default()
                },
                ..MarketStateConfig::default()
            },
        }
    }

    /// 风险控制配置：强调极端状态检测
    pub fn risk_control_profile() -> Self {
        Self {
            name: "risk_control".to_string(),
            description: "风险控制：快速响应极端状态".to_string(),
            config: MarketStateConfig {
                volatility: VolatilityThresholds {
                    elevated_threshold: 0.85, // 更早触发高波动警告
                    crisis_threshold: 0.92,
                    ..VolatilityThresholds::default()
                },
                liquidity: LiquidityThresholds {
                    low_threshold: 0.35, // 更早触发流动性警告
                    ..LiquidityThresholds::default()
                },
                aggregate_weights: AggregateWeights {
                    volatility: 0.35,
                    liquidity: 0.30,
                    structure: 0.20,
                    behavior: 0.15,
                },
                enhanced_aggregation: EnhancedAggregationConfig {
                    volatility_weight: 0.35,
                    liquidity_weight: 0.30,
                    structure_weight: 0.20,
                    behavior_weight: 0.15,
                    ..EnhancedAggregationConfig::default()
                },
                ..MarketStateConfig::default()
            },
        }
    }

    /// 高置信配置：在保持零配置默认不变的前提下，提供 opt-in 高置信热插拔模板
    pub fn high_confidence_profile() -> Self {
        Self {
            name: "high_confidence".to_string(),
            description: "高置信 opt-in：提高基础置信度并强化结构权重".to_string(),
            config: MarketStateConfig {
                enhanced_aggregation: EnhancedAggregationConfig {
                    base_confidence: 0.50,
                    consistency_weight: 0.10,
                    volatility_weight: 0.12,
                    liquidity_weight: 0.08,
                    structure_weight: 0.55,
                    behavior_weight: 0.25,
                    high_consistency_bonus: 0.05,
                    medium_consistency_bonus: 0.02,
                    ..EnhancedAggregationConfig::default()
                },
                user_label: Some("nq_confidence_profile".to_string()),
                ..MarketStateConfig::default()
            },
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "default" => Some(Self::default_profile()),
            "trend_trading" => Some(Self::trend_trading_profile()),
            "volatility_trading" => Some(Self::volatility_trading_profile()),
            "reversal_trading" => Some(Self::reversal_trading_profile()),
            "risk_control" => Some(Self::risk_control_profile()),
            "high_confidence" => Some(Self::high_confidence_profile()),
            _ => None,
        }
    }

    pub fn supported_names() -> &'static [&'static str] {
        &[
            "default",
            "trend_trading",
            "volatility_trading",
            "reversal_trading",
            "risk_control",
            "high_confidence",
        ]
    }
}

/// 配置加载/保存
impl MarketStateConfig {
    /// 从 JSON 文件加载
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: MarketStateConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// 保存到 JSON 文件
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// 从 Profile 创建
    pub fn from_profile(profile: &MarketStateProfile) -> Self {
        profile.config.clone()
    }

    /// 验证配置有效性
    pub fn validate(&self) -> anyhow::Result<()> {
        // 验证权重和为 1
        let weight_sum = self.aggregate_weights.volatility
            + self.aggregate_weights.liquidity
            + self.aggregate_weights.structure
            + self.aggregate_weights.behavior;

        if (weight_sum - 1.0).abs() > 0.01 {
            anyhow::bail!("聚合权重和必须为 1.0，当前为 {}", weight_sum);
        }

        // 验证阈值范围
        if self.volatility.low_threshold >= self.volatility.normal_threshold {
            anyhow::bail!("波动率低阈值必须小于正常阈值");
        }

        let enhanced_weight_sum = self.enhanced_aggregation.volatility_weight
            + self.enhanced_aggregation.liquidity_weight
            + self.enhanced_aggregation.structure_weight
            + self.enhanced_aggregation.behavior_weight;

        if (enhanced_weight_sum - 1.0).abs() > 0.01 {
            anyhow::bail!("增强聚合权重和必须为 1.0，当前为 {}", enhanced_weight_sum);
        }

        Ok(())
    }
}

/// 获取所有预设 Profile
pub fn available_profiles() -> Vec<MarketStateProfile> {
    vec![
        MarketStateProfile::default_profile(),
        MarketStateProfile::trend_trading_profile(),
        MarketStateProfile::volatility_trading_profile(),
        MarketStateProfile::reversal_trading_profile(),
        MarketStateProfile::risk_control_profile(),
        MarketStateProfile::high_confidence_profile(),
    ]
}

/// 用户权重模板（用于热插拔）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWeightsTemplate {
    /// 聚合权重
    pub aggregate_weights: AggregateWeights,
    /// 增强聚合权重（可选）
    pub enhanced_aggregation: Option<EnhancedAggregationConfig>,
    /// 是否覆盖波动率阈值
    pub override_volatility: Option<VolatilityThresholds>,
    /// 是否覆盖流动性阈值
    pub override_liquidity: Option<LiquidityThresholds>,
    /// 是否覆盖结构阈值
    pub override_structure: Option<StructureThresholds>,
    /// 是否覆盖行为阈值
    pub override_behavior: Option<BehaviorThresholds>,
}

impl UserWeightsTemplate {
    /// 生成默认模板
    pub fn template() -> Self {
        Self {
            aggregate_weights: AggregateWeights::default(),
            enhanced_aggregation: None,
            override_volatility: None,
            override_liquidity: None,
            override_structure: None,
            override_behavior: None,
        }
    }

    /// 应用到现有配置
    pub fn apply_to(&self, config: &mut MarketStateConfig) {
        config.aggregate_weights = self.aggregate_weights.clone();
        if let Some(ref enhanced) = self.enhanced_aggregation {
            config.enhanced_aggregation = enhanced.clone();
        }
        if let Some(ref vol) = self.override_volatility {
            config.volatility = vol.clone();
        }
        if let Some(ref liq) = self.override_liquidity {
            config.liquidity = liq.clone();
        }
        if let Some(ref struct_thres) = self.override_structure {
            config.structure = struct_thres.clone();
        }
        if let Some(ref behav) = self.override_behavior {
            config.behavior = behav.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn default_config_is_valid() {
        let config = MarketStateConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn profile_configs_are_valid() {
        for profile in available_profiles() {
            assert!(
                profile.config.validate().is_ok(),
                "Profile {} is invalid",
                profile.name
            );
        }
    }

    #[test]
    fn weight_sum_is_one() {
        let weights = AggregateWeights::default();
        let sum = weights.volatility + weights.liquidity + weights.structure + weights.behavior;
        assert!((sum - 1.0).abs() < 0.001);
    }

    #[test]
    fn user_template_apply() {
        let mut config = MarketStateConfig::default();
        let template = UserWeightsTemplate {
            aggregate_weights: AggregateWeights {
                volatility: 0.5,
                liquidity: 0.2,
                structure: 0.2,
                behavior: 0.1,
            },
            ..UserWeightsTemplate::template()
        };
        template.apply_to(&mut config);
        assert!((config.aggregate_weights.volatility - 0.5).abs() < 0.001);
    }

    #[test]
    fn profiles_tune_enhanced_aggregation_weights() {
        let profile = MarketStateProfile::risk_control_profile();

        assert!((profile.config.aggregate_weights.volatility - 0.35).abs() < 0.001);
        assert!((profile.config.enhanced_aggregation.volatility_weight - 0.35).abs() < 0.001);
        assert!((profile.config.enhanced_aggregation.structure_weight - 0.20).abs() < 0.001);
    }

    #[test]
    fn high_confidence_profile_matches_repo_example() {
        let profile = MarketStateProfile::high_confidence_profile();
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("support/docs/examples/market-state-nq-confidence-profile.json");
        let config = MarketStateConfig::load(&path).expect("load repo example profile");

        assert_eq!(
            serde_json::to_value(&profile.config).unwrap(),
            serde_json::to_value(&config).unwrap()
        );
    }
}
