//! Market State Validation Command

use anyhow::{Context, Result};

use ict_engine::data::load_candles;
use ict_engine::market_state::{
    MarketStateClassifier, MarketStateConfig, MarketStateProfile, MarketStateValidator,
    ValidationConfig, ValidationResult,
};

pub struct ValidateMarketStateInput {
    pub data_path: String,
    pub window_size: usize,
    pub step_size: usize,
    pub verbose: bool,
    pub compact: bool,
    pub enhanced: bool,
    pub config_path: Option<String>,
    pub profile: Option<String>,
}

pub fn validate_market_state_shell(input: ValidateMarketStateInput) -> Result<()> {
    if input.window_size == 0 {
        anyhow::bail!("window-size must be greater than 0");
    }
    if input.step_size == 0 {
        anyhow::bail!("step-size must be greater than 0");
    }
    if input.config_path.is_some() && input.profile.is_some() {
        anyhow::bail!("use only one of --config or --profile");
    }

    let candles = load_candles(&input.data_path)
        .with_context(|| format!("failed to load candles from {}", input.data_path))?;

    if candles.len() < input.window_size {
        anyhow::bail!(
            "Not enough candles: {} < {}",
            candles.len(),
            input.window_size
        );
    }

    let config = if let Some(ref profile_name_or_path) = input.config_path {
        let path = std::path::Path::new(profile_name_or_path);
        let loaded = MarketStateConfig::load(path).with_context(|| {
            format!("failed to load market-state config from {}", path.display())
        })?;
        loaded
            .validate()
            .with_context(|| format!("invalid market-state config at {}", path.display()))?;
        loaded
    } else if let Some(ref profile_name) = input.profile {
        let profile = MarketStateProfile::from_name(profile_name).ok_or_else(|| {
            anyhow::anyhow!(
                "unknown market-state profile: {} (use {})",
                profile_name,
                MarketStateProfile::supported_names().join(", ")
            )
        })?;
        let config = MarketStateConfig::from_profile(&profile);
        config
            .validate()
            .context("invalid market-state profile config")?;
        config
    } else {
        let config = MarketStateConfig::default();
        config
            .validate()
            .context("invalid default market-state config")?;
        config
    };

    let classifier =
        MarketStateClassifier::with_config(config).with_enhanced_aggregation(input.enhanced);

    let config = ValidationConfig {
        min_window_size: input.window_size,
        step_size: input.step_size,
        verbose: input.verbose,
    };
    let validator = MarketStateValidator::with_classifier(classifier, config);

    let result = validator.validate(&candles);
    let output = format_validate_market_state_output(&input, candles.len(), &result, &validator);
    print!("{}", output);

    Ok(())
}

fn format_validate_market_state_output(
    input: &ValidateMarketStateInput,
    candle_count: usize,
    result: &ValidationResult,
    validator: &MarketStateValidator,
) -> String {
    if input.compact {
        return format!("{}\n", validator.generate_compact_report(result));
    }

    let mut output = String::new();
    output.push_str("=== Market State Classification Validation ===\n\n");
    output.push_str(&format!("Loading data from: {}\n", input.data_path));
    output.push_str(&format!("Loaded {} candles\n\n", candle_count));
    output.push_str(&format!(
        "Classifier: {} aggregation\n\n",
        if input.enhanced { "Enhanced" } else { "Basic" }
    ));
    output.push_str("Running validation...\n");
    output.push_str(&format!(
        "  Window: {} | Step: {} | Samples: {}\n\n",
        input.window_size, input.step_size, result.total_samples
    ));
    output.push_str(&validator.generate_report(result));
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use ict_engine::market_state::ConfidenceDistribution;
    use std::collections::HashMap;

    fn sample_result() -> ValidationResult {
        let mut primary_distribution = HashMap::new();
        primary_distribution.insert("TrendExpansion".to_string(), 12);
        primary_distribution.insert("RangeConsolidation".to_string(), 8);

        let mut secondary_distribution = HashMap::new();
        secondary_distribution.insert("BullTrendAcceleration".to_string(), 7);
        secondary_distribution.insert("WideRange".to_string(), 5);

        ValidationResult {
            total_samples: 20,
            primary_distribution,
            secondary_distribution,
            confidence_distribution: ConfidenceDistribution {
                high: 8,
                medium: 7,
                low: 5,
                very_low: 0,
            },
            avg_confidence: 0.7123,
            high_confidence_ratio: 0.4,
            tradeable_ratio: 0.75,
        }
    }

    fn sample_input(compact: bool) -> ValidateMarketStateInput {
        ValidateMarketStateInput {
            data_path: "candles.json".to_string(),
            window_size: 200,
            step_size: 50,
            verbose: false,
            compact,
            enhanced: true,
            config_path: None,
            profile: None,
        }
    }

    #[test]
    fn compact_output_is_thin() {
        let input = sample_input(true);
        let validator = MarketStateValidator::new();
        let rendered =
            format_validate_market_state_output(&input, 2000, &sample_result(), &validator);

        assert!(rendered.contains("samples=20"));
        assert!(rendered.contains("avg_confidence=71.23%"));
        assert!(!rendered.contains("Loading data from"));
        assert!(!rendered.contains("Primary Regime Distribution"));
    }

    #[test]
    fn full_output_reports_exact_sample_count() {
        let input = sample_input(false);
        let validator = MarketStateValidator::new();
        let rendered =
            format_validate_market_state_output(&input, 2000, &sample_result(), &validator);

        assert!(rendered.contains("Window: 200 | Step: 50 | Samples: 20"));
        assert!(rendered.contains("Loading data from: candles.json"));
        assert!(rendered.contains("Primary Regime Distribution"));
    }
}
