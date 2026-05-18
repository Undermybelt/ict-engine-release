use crate::types::TimesFMPrediction;
use anyhow::{Context, Result};
use std::process::Command;

/// Bridge to call TimesFM Python model
pub struct TimesFMBridge {
    pub python_path: String,
    pub script_path: String,
}

impl TimesFMBridge {
    pub fn new(python_path: &str, script_path: &str) -> Self {
        Self {
            python_path: python_path.to_string(),
            script_path: script_path.to_string(),
        }
    }

    /// Call TimesFM for forecasting
    pub fn forecast(
        &self,
        prices: &[f64],
        horizon: usize,
        symbol: &str,
    ) -> Result<TimesFMPrediction> {
        // Create temporary input file
        let input_json = serde_json::json!({
            "prices": prices,
            "horizon": horizon,
            "symbol": symbol
        });

        let input_file = std::env::temp_dir().join("timesfm_input.json");
        std::fs::write(&input_file, input_json.to_string())
            .context("Failed to write input file")?;

        // Call Python script
        let output = Command::new(&self.python_path)
            .arg(&self.script_path)
            .arg(&input_file)
            .output()
            .context("Failed to execute Python script")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("TimesFM script failed: {}", stderr);
        }

        // Parse output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: serde_json::Value =
            serde_json::from_str(&stdout).context("Failed to parse TimesFM output")?;

        let point_forecast: Vec<f64> = serde_json::from_value(result["point_forecast"].clone())
            .context("Invalid point_forecast")?;

        let quantile_forecast: Vec<Vec<f64>> =
            serde_json::from_value(result["quantile_forecast"].clone())
                .context("Invalid quantile_forecast")?;

        // Clean up
        let _ = std::fs::remove_file(input_file);

        Ok(TimesFMPrediction {
            point_forecast,
            quantile_forecast,
            symbol: symbol.to_string(),
            horizon,
        })
    }
}
