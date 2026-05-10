//! Market State Classification Validation Tool
//!
//! 市场状态分类验证工具：用真实历史数据验证准确率
//!
//! 功能：
//! 1. 加载历史 OHLCV 数据
//! 2. 运行市场状态分类
//! 3. 统计各主大类/次小类的分布
//! 4. 计算置信度分布
//! 5. 生成验证报告

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::market_state::MarketStateClassifier;
use crate::types::Candle;

/// 验证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// 最小窗口大小（用于分类）
    pub min_window_size: usize,
    /// 滑动步长
    pub step_size: usize,
    /// 是否输出详细日志
    pub verbose: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            min_window_size: 100,
            step_size: 1,
            verbose: false,
        }
    }
}

/// 验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// 总样本数
    pub total_samples: usize,
    /// 主大类分布
    pub primary_distribution: HashMap<String, usize>,
    /// 次小类分布
    pub secondary_distribution: HashMap<String, usize>,
    /// 置信度分布
    pub confidence_distribution: ConfidenceDistribution,
    /// 平均置信度
    pub avg_confidence: f64,
    /// 高置信样本占比
    pub high_confidence_ratio: f64,
    /// 可交易样本占比
    pub tradeable_ratio: f64,
}

/// 置信度分布
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfidenceDistribution {
    pub high: usize,     // >= 0.75
    pub medium: usize,   // >= 0.55
    pub low: usize,      // >= 0.35
    pub very_low: usize, // < 0.35
}

/// 验证器
pub struct MarketStateValidator {
    classifier: MarketStateClassifier,
    config: ValidationConfig,
}

impl MarketStateValidator {
    pub fn new() -> Self {
        Self::with_config(ValidationConfig::default())
    }

    pub fn with_config(config: ValidationConfig) -> Self {
        Self {
            classifier: MarketStateClassifier::new(),
            config,
        }
    }

    pub fn with_classifier(classifier: MarketStateClassifier, config: ValidationConfig) -> Self {
        Self { classifier, config }
    }

    /// 验证历史数据
    pub fn validate(&self, candles: &[Candle]) -> ValidationResult {
        let mut primary_dist: HashMap<String, usize> = HashMap::new();
        let mut secondary_dist: HashMap<String, usize> = HashMap::new();
        let mut confidence_dist = ConfidenceDistribution::default();
        let mut total_confidence = 0.0;
        let mut total_samples = 0;
        let mut high_confidence_count = 0;
        let mut tradeable_count = 0;

        // 滑动窗口验证
        let window_size = self.config.min_window_size;
        let step = self.config.step_size;

        let last_start = candles.len() - window_size;
        let mut start_indices = Vec::new();
        let mut i = 0;
        while i < last_start {
            start_indices.push(i);
            i = i.saturating_add(step);
        }
        start_indices.push(last_start);

        for i in start_indices {
            let window = &candles[i..i + window_size];
            let snapshot = self.classifier.classify(window);

            // 统计主大类
            let primary_key = format!("{:?}", snapshot.primary_regime);
            *primary_dist.entry(primary_key).or_insert(0) += 1;

            // 统计次小类
            let secondary_key = format!("{:?}", snapshot.secondary_regime);
            *secondary_dist.entry(secondary_key).or_insert(0) += 1;

            // 统计置信度
            total_confidence += snapshot.overall_confidence;
            total_samples += 1;

            // 置信度分级
            if snapshot.overall_confidence >= 0.75 {
                confidence_dist.high += 1;
                high_confidence_count += 1;
                tradeable_count += 1;
            } else if snapshot.overall_confidence >= 0.55 {
                confidence_dist.medium += 1;
                tradeable_count += 1;
            } else if snapshot.overall_confidence >= 0.35 {
                confidence_dist.low += 1;
            } else {
                confidence_dist.very_low += 1;
            }

            if self.config.verbose && i % (step * 100) == 0 {
                println!(
                    "[{}/{}] primary={:?} secondary={:?} conf={:.2}",
                    i,
                    candles.len(),
                    snapshot.primary_regime,
                    snapshot.secondary_regime,
                    snapshot.overall_confidence
                );
            }
        }

        let avg_confidence = if total_samples > 0 {
            total_confidence / total_samples as f64
        } else {
            0.0
        };

        let high_confidence_ratio = if total_samples > 0 {
            high_confidence_count as f64 / total_samples as f64
        } else {
            0.0
        };

        let tradeable_ratio = if total_samples > 0 {
            tradeable_count as f64 / total_samples as f64
        } else {
            0.0
        };

        ValidationResult {
            total_samples,
            primary_distribution: primary_dist,
            secondary_distribution: secondary_dist,
            confidence_distribution: confidence_dist,
            avg_confidence,
            high_confidence_ratio,
            tradeable_ratio,
        }
    }

    /// 生成验证报告
    pub fn generate_report(&self, result: &ValidationResult) -> String {
        let mut report = String::new();

        report.push_str("=== Market State Classification Validation Report ===\n\n");

        // 总览
        report.push_str(&format!("Total Samples: {}\n", result.total_samples));
        report.push_str(&format!(
            "Average Confidence: {:.2}%\n",
            result.avg_confidence * 100.0
        ));
        report.push_str(&format!(
            "High Confidence Ratio: {:.2}%\n",
            result.high_confidence_ratio * 100.0
        ));
        report.push_str(&format!(
            "Tradeable Ratio: {:.2}%\n\n",
            result.tradeable_ratio * 100.0
        ));

        // 主大类分布
        report.push_str("--- Primary Regime Distribution ---\n");
        let mut primary_sorted: Vec<_> = result.primary_distribution.iter().collect();
        primary_sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (regime, count) in primary_sorted {
            let pct = (*count as f64 / result.total_samples as f64) * 100.0;
            report.push_str(&format!("  {:<25} {:>6} ({:>5.1}%)\n", regime, count, pct));
        }
        report.push('\n');

        // 次小类分布
        report.push_str("--- Secondary Regime Distribution ---\n");
        let mut secondary_sorted: Vec<_> = result.secondary_distribution.iter().collect();
        secondary_sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (regime, count) in secondary_sorted.iter().take(10) {
            let pct = (**count as f64 / result.total_samples as f64) * 100.0;
            report.push_str(&format!("  {:<25} {:>6} ({:>5.1}%)\n", regime, count, pct));
        }
        if secondary_sorted.len() > 10 {
            report.push_str(&format!("  ... and {} more\n", secondary_sorted.len() - 10));
        }
        report.push('\n');

        // 置信度分布
        report.push_str("--- Confidence Distribution ---\n");
        let dist = &result.confidence_distribution;
        let high_pct = (dist.high as f64 / result.total_samples as f64) * 100.0;
        let medium_pct = (dist.medium as f64 / result.total_samples as f64) * 100.0;
        let low_pct = (dist.low as f64 / result.total_samples as f64) * 100.0;
        let very_low_pct = (dist.very_low as f64 / result.total_samples as f64) * 100.0;

        report.push_str(&format!(
            "  High    (≥0.75): {:>6} ({:>5.1}%)\n",
            dist.high, high_pct
        ));
        report.push_str(&format!(
            "  Medium  (≥0.55): {:>6} ({:>5.1}%)\n",
            dist.medium, medium_pct
        ));
        report.push_str(&format!(
            "  Low     (≥0.35): {:>6} ({:>5.1}%)\n",
            dist.low, low_pct
        ));
        report.push_str(&format!(
            "  VeryLow (<0.35): {:>6} ({:>5.1}%)\n",
            dist.very_low, very_low_pct
        ));
        report.push('\n');

        // 质量评估
        report.push_str("--- Quality Assessment ---\n");
        if result.high_confidence_ratio > 0.5 {
            report.push_str("  ✅ High confidence ratio > 50% (EXCELLENT)\n");
        } else if result.high_confidence_ratio > 0.3 {
            report.push_str("  ⚠️  High confidence ratio 30-50% (GOOD)\n");
        } else {
            report.push_str("  ❌ High confidence ratio < 30% (NEEDS IMPROVEMENT)\n");
        }

        if result.tradeable_ratio > 0.6 {
            report.push_str("  ✅ Tradeable ratio > 60% (EXCELLENT)\n");
        } else if result.tradeable_ratio > 0.4 {
            report.push_str("  ⚠️  Tradeable ratio 40-60% (GOOD)\n");
        } else {
            report.push_str("  ❌ Tradeable ratio < 40% (NEEDS IMPROVEMENT)\n");
        }

        if result.avg_confidence > 0.65 {
            report.push_str("  ✅ Average confidence > 65% (EXCELLENT)\n");
        } else if result.avg_confidence > 0.50 {
            report.push_str("  ⚠️  Average confidence 50-65% (GOOD)\n");
        } else {
            report.push_str("  ❌ Average confidence < 50% (NEEDS IMPROVEMENT)\n");
        }

        report.push_str("\n=== End of Report ===\n");

        report
    }

    pub fn generate_compact_report(&self, result: &ValidationResult) -> String {
        let primary_top = top_distribution_entry(&result.primary_distribution);
        let secondary_top = top_distribution_entry(&result.secondary_distribution);

        format!(
            "samples={} avg_confidence={:.2}% high_confidence={:.2}% tradeable={:.2}% primary_top={} secondary_top={}",
            result.total_samples,
            result.avg_confidence * 100.0,
            result.high_confidence_ratio * 100.0,
            result.tradeable_ratio * 100.0,
            primary_top,
            secondary_top
        )
    }
}

fn top_distribution_entry(distribution: &HashMap<String, usize>) -> String {
    let Some((label, count)) = distribution
        .iter()
        .max_by(|a, b| a.1.cmp(b.1).then_with(|| a.0.cmp(b.0).reverse()))
    else {
        return "none".to_string();
    };
    format!("{}:{}", label, count)
}

impl Default for MarketStateValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn mock_candles(count: usize) -> Vec<Candle> {
        (0..count)
            .map(|i| Candle {
                timestamp: Utc.timestamp_opt(1_700_000_000 + i as i64 * 60, 0).unwrap(),
                open: 100.0 + (i as f64 * 0.1),
                high: 101.0 + (i as f64 * 0.1),
                low: 99.0 + (i as f64 * 0.1),
                close: 100.5 + (i as f64 * 0.1),
                volume: 1000.0,
            })
            .collect()
    }

    #[test]
    fn validator_runs_on_mock_data() {
        let validator = MarketStateValidator::new();
        let candles = mock_candles(500);

        let result = validator.validate(&candles);

        assert!(result.total_samples > 0);
        assert!(result.avg_confidence >= 0.0 && result.avg_confidence <= 1.0);
        assert!(result.high_confidence_ratio >= 0.0 && result.high_confidence_ratio <= 1.0);
    }

    #[test]
    fn report_generation() {
        let validator = MarketStateValidator::new();
        let candles = mock_candles(500);
        let result = validator.validate(&candles);

        let report = validator.generate_report(&result);

        assert!(report.contains("Total Samples"));
        assert!(report.contains("Primary Regime Distribution"));
        assert!(report.contains("Confidence Distribution"));
    }

    #[test]
    fn compact_report_generation() {
        let validator = MarketStateValidator::new();
        let candles = mock_candles(500);
        let result = validator.validate(&candles);

        let report = validator.generate_compact_report(&result);

        assert!(report.contains("samples="));
        assert!(report.contains("avg_confidence="));
        assert!(report.contains("high_confidence="));
        assert!(report.contains("tradeable="));
        assert!(report.contains("primary_top="));
        assert!(report.contains("secondary_top="));
        assert!(!report.contains("Primary Regime Distribution"));
    }

    #[test]
    fn validator_counts_exact_window() {
        let validator = MarketStateValidator::with_config(ValidationConfig {
            min_window_size: 100,
            step_size: 1,
            verbose: false,
        });
        let candles = mock_candles(100);

        let result = validator.validate(&candles);

        assert_eq!(result.total_samples, 1);
    }

    #[test]
    fn validator_includes_trailing_full_window() {
        let validator = MarketStateValidator::with_config(ValidationConfig {
            min_window_size: 100,
            step_size: 200,
            verbose: false,
        });
        let candles = mock_candles(250);

        let result = validator.validate(&candles);

        assert_eq!(result.total_samples, 2);
    }
}
