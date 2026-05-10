//! Confidence Validation Module
//!
//! 基于历史统计的置信度验证：
//! - 统计学阈值：基于历史数据回测
//! - 滚动窗口验证
//! - 自适应置信度校准
//!
//! 设计原则：
//! - 零配置：默认参数直接可用
//! - 热插拔：用户可覆盖配置
//! - Token 友好：简洁输出
//! - 高置信度：基于历史统计

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::market_state::{MarketStateSnapshot, PrimaryMarketRegime};

/// 置信度验证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceValidationConfig {
    /// 历史窗口大小
    pub history_window: usize,
    /// 最小样本数
    pub min_samples: usize,
    /// 置信度阈值：高置信
    pub high_confidence_threshold: f64,
    /// 置信度阈值：中等置信
    pub medium_confidence_threshold: f64,
    /// 置信度阈值：低置信
    pub low_confidence_threshold: f64,
    /// 校准系数
    pub calibration_factor: f64,
    /// 是否启用自适应校准
    pub adaptive_calibration: bool,
}

impl Default for ConfidenceValidationConfig {
    fn default() -> Self {
        Self {
            history_window: 252, // 一年交易日
            min_samples: 30,     // 最小样本
            high_confidence_threshold: 0.75,
            medium_confidence_threshold: 0.55,
            low_confidence_threshold: 0.35,
            calibration_factor: 0.1,
            adaptive_calibration: true,
        }
    }
}

/// 历史样本记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistorySample {
    /// 市场状态
    pub regime: PrimaryMarketRegime,
    /// 原始置信度
    pub raw_confidence: f64,
    /// 实际结果：成功/失败
    pub outcome: bool,
    /// 时间戳
    pub timestamp: i64,
}

/// 状态统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RegimeStats {
    /// 样本数
    pub samples: usize,
    /// 成功数
    pub successes: usize,
    /// 平均原始置信度
    pub avg_raw_confidence: f64,
    /// 实际成功率
    pub actual_success_rate: f64,
    /// 校准偏移
    pub calibration_offset: f64,
}

impl RegimeStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// 更新统计
    pub fn update(&mut self, sample: &HistorySample) {
        self.samples += 1;
        if sample.outcome {
            self.successes += 1;
        }
        self.avg_raw_confidence = (self.avg_raw_confidence * (self.samples - 1) as f64
            + sample.raw_confidence)
            / self.samples as f64;
        self.actual_success_rate = self.successes as f64 / self.samples as f64;
        // 校准偏移：实际成功率 - 平均原始置信度
        self.calibration_offset = self.actual_success_rate - self.avg_raw_confidence;
    }

    /// 是否有足够样本
    pub fn has_sufficient_samples(&self, min_samples: usize) -> bool {
        self.samples >= min_samples
    }

    /// 获取校准后置信度
    pub fn calibrated_confidence(&self, raw: f64, factor: f64) -> f64 {
        (raw + self.calibration_offset * factor).clamp(0.0, 1.0)
    }
}

/// 置信度验证器
pub struct ConfidenceValidator {
    config: ConfidenceValidationConfig,
    /// 滚动历史窗口
    history: VecDeque<HistorySample>,
    /// 各状态的统计
    regime_stats: std::collections::HashMap<String, RegimeStats>,
}

impl ConfidenceValidator {
    pub fn new() -> Self {
        Self::with_config(ConfidenceValidationConfig::default())
    }

    pub fn with_config(config: ConfidenceValidationConfig) -> Self {
        Self {
            config,
            history: VecDeque::with_capacity(512),
            regime_stats: std::collections::HashMap::new(),
        }
    }

    /// 验证并校准置信度
    pub fn validate(&mut self, snapshot: &MarketStateSnapshot) -> ValidationResult {
        let regime_key = format!("{:?}", snapshot.primary_regime);
        let (calibrated, samples_available, calibration_applied, regime_accuracy) = {
            let stats = self.regime_stats.entry(regime_key).or_default();

            let calibration_applied = stats.has_sufficient_samples(self.config.min_samples);
            let calibrated = if calibration_applied {
                stats.calibrated_confidence(
                    snapshot.overall_confidence,
                    self.config.calibration_factor,
                )
            } else {
                snapshot.overall_confidence
            };
            let regime_accuracy = if calibration_applied {
                Some(stats.actual_success_rate)
            } else {
                None
            };

            (
                calibrated,
                stats.samples,
                calibration_applied,
                regime_accuracy,
            )
        };

        let confidence_level = self.classify_confidence(calibrated);

        ValidationResult {
            raw_confidence: snapshot.overall_confidence,
            calibrated_confidence: calibrated,
            confidence_level,
            samples_available,
            calibration_applied,
            regime_accuracy,
        }
    }

    /// 记录结果（用于后续验证）
    pub fn record_outcome(
        &mut self,
        snapshot: &MarketStateSnapshot,
        outcome: bool,
        timestamp: i64,
    ) {
        let sample = HistorySample {
            regime: snapshot.primary_regime,
            raw_confidence: snapshot.overall_confidence,
            outcome,
            timestamp,
        };

        // 更新滚动窗口
        if self.history.len() >= self.config.history_window {
            let old = self.history.pop_front();
            // 减少旧样本统计
            if let Some(old_sample) = old {
                self.decrement_stats(&old_sample);
            }
        }
        self.history.push_back(sample.clone());

        // 更新状态统计
        let regime_key = format!("{:?}", sample.regime);
        let stats = self.regime_stats.entry(regime_key).or_default();
        stats.update(&sample);
    }

    /// 减少统计（移除旧样本）
    fn decrement_stats(&mut self, sample: &HistorySample) {
        let regime_key = format!("{:?}", sample.regime);
        if let Some(stats) = self.regime_stats.get_mut(&regime_key) {
            if stats.samples > 0 {
                stats.samples -= 1;
                if sample.outcome && stats.successes > 0 {
                    stats.successes -= 1;
                }
                if stats.samples > 0 {
                    stats.actual_success_rate = stats.successes as f64 / stats.samples as f64;
                    stats.calibration_offset = stats.actual_success_rate - stats.avg_raw_confidence;
                }
            }
        }
    }

    /// 分类置信度级别
    fn classify_confidence(&self, confidence: f64) -> ConfidenceLevel {
        if confidence >= self.config.high_confidence_threshold {
            ConfidenceLevel::High
        } else if confidence >= self.config.medium_confidence_threshold {
            ConfidenceLevel::Medium
        } else if confidence >= self.config.low_confidence_threshold {
            ConfidenceLevel::Low
        } else {
            ConfidenceLevel::VeryLow
        }
    }

    /// 获取所有状态统计
    pub fn get_all_stats(&self) -> &std::collections::HashMap<String, RegimeStats> {
        &self.regime_stats
    }

    /// 获取特定状态统计
    pub fn get_regime_stats(&self, regime: &PrimaryMarketRegime) -> Option<&RegimeStats> {
        let key = format!("{:?}", regime);
        self.regime_stats.get(&key)
    }
}

impl Default for ConfidenceValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// 置信度级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    /// 高置信 (>=0.75)
    High,
    /// 中等置信 (>=0.55)
    Medium,
    /// 低置信 (>=0.35)
    Low,
    /// 极低置信 (<0.35)
    VeryLow,
}

impl ConfidenceLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::VeryLow => "very_low",
        }
    }

    pub fn is_tradeable(&self) -> bool {
        matches!(self, Self::High | Self::Medium)
    }
}

/// 验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// 原始置信度
    pub raw_confidence: f64,
    /// 校准后置信度
    pub calibrated_confidence: f64,
    /// 置信度级别
    pub confidence_level: ConfidenceLevel,
    /// 可用样本数
    pub samples_available: usize,
    /// 是否应用了校准
    pub calibration_applied: bool,
    /// 该状态历史准确率
    pub regime_accuracy: Option<f64>,
}

impl ValidationResult {
    /// 是否可交易
    pub fn is_tradeable(&self) -> bool {
        self.confidence_level.is_tradeable()
    }

    /// 置信度摘要
    pub fn summary(&self) -> String {
        format!(
            "confidence={:.2}%({}) samples={} calibrated={}",
            self.calibrated_confidence * 100.0,
            self.confidence_level.as_str(),
            self.samples_available,
            self.calibration_applied
        )
    }
}

/// 滚动准确率追踪器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollingAccuracyTracker {
    /// 窗口大小
    window: usize,
    /// 成功计数
    success_count: usize,
    /// 总计数
    total_count: usize,
    /// 滚动结果队列
    results: VecDeque<bool>,
}

impl RollingAccuracyTracker {
    pub fn new(window: usize) -> Self {
        Self {
            window,
            success_count: 0,
            total_count: 0,
            results: VecDeque::with_capacity(window),
        }
    }

    /// 记录结果
    pub fn record(&mut self, success: bool) {
        if self.results.len() >= self.window {
            if let Some(old) = self.results.pop_front() {
                if old {
                    self.success_count -= 1;
                }
                self.total_count -= 1;
            }
        }
        self.results.push_back(success);
        if success {
            self.success_count += 1;
        }
        self.total_count += 1;
    }

    /// 获取当前准确率
    pub fn accuracy(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            self.success_count as f64 / self.total_count as f64
        }
    }

    /// 获取样本数
    pub fn sample_count(&self) -> usize {
        self.total_count
    }

    /// 是否有足够样本
    pub fn has_sufficient_samples(&self, min: usize) -> bool {
        self.total_count >= min
    }
}

impl Default for RollingAccuracyTracker {
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validator_classifies_confidence_levels() {
        let mut validator = ConfidenceValidator::new();
        let mut snapshot = MarketStateSnapshot {
            overall_confidence: 0.85,
            ..MarketStateSnapshot::default()
        };

        let result = validator.validate(&snapshot);
        assert_eq!(result.confidence_level, ConfidenceLevel::High);

        snapshot.overall_confidence = 0.65;
        let result = validator.validate(&snapshot);
        assert_eq!(result.confidence_level, ConfidenceLevel::Medium);

        snapshot.overall_confidence = 0.45;
        let result = validator.validate(&snapshot);
        assert_eq!(result.confidence_level, ConfidenceLevel::Low);

        snapshot.overall_confidence = 0.25;
        let result = validator.validate(&snapshot);
        assert_eq!(result.confidence_level, ConfidenceLevel::VeryLow);
    }

    #[test]
    fn calibration_adjusts_confidence() {
        let mut validator = ConfidenceValidator::with_config(ConfidenceValidationConfig {
            min_samples: 3,
            ..Default::default()
        });

        let mut snapshot = MarketStateSnapshot {
            overall_confidence: 0.7,
            ..MarketStateSnapshot::default()
        };

        // 记录样本：70% 置信度但只有 40% 成功率
        for i in 0..5 {
            snapshot.overall_confidence = 0.7;
            let outcome = i < 2; // 40% 成功
            validator.record_outcome(&snapshot, outcome, 1700000000 + i as i64);
        }

        // 验证新的 70% 置信度样本
        snapshot.overall_confidence = 0.7;
        let result = validator.validate(&snapshot);

        // 校准后应该低于原始（因为实际成功率低于置信度）
        assert!(result.calibration_applied);
        // 历史准确率约 40%
        if let Some(accuracy) = result.regime_accuracy {
            assert!((accuracy - 0.4).abs() < 0.1);
        }
    }

    #[test]
    fn rolling_accuracy_tracker_works() {
        let mut tracker = RollingAccuracyTracker::new(5);

        tracker.record(true);
        tracker.record(true);
        tracker.record(false);

        assert!((tracker.accuracy() - 0.666).abs() < 0.01);
        assert_eq!(tracker.sample_count(), 3);

        // 超过窗口
        tracker.record(true);
        tracker.record(false);
        tracker.record(true);

        // 窗口内：true, false, true, false, true = 3/5
        assert!((tracker.accuracy() - 0.6).abs() < 0.01);
    }

    #[test]
    fn confidence_level_tradeability() {
        assert!(ConfidenceLevel::High.is_tradeable());
        assert!(ConfidenceLevel::Medium.is_tradeable());
        assert!(!ConfidenceLevel::Low.is_tradeable());
        assert!(!ConfidenceLevel::VeryLow.is_tradeable());
    }
}
