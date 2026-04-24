use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::application::multi_timeframe_inputs::{
    MultiTimeframeCleanReportView, MULTI_TIMEFRAME_INTERVALS,
};
use crate::config::parse_interval_minutes;
use crate::data::{
    aggregate_candles_by_minutes, load_tomac_continuous_candles, CleanedContinuousFuturesSummary,
};
use crate::types::Candle;

#[derive(Debug, Serialize)]
pub struct CleanedCandleOutput {
    pub symbol: String,
    pub candles: Vec<Candle>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CleanFuturesReport {
    pub root: String,
    pub output_dir: String,
    pub interval: String,
    pub datasets: Vec<CleanFuturesDatasetReport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MultiTimeframeCleanFuturesReport {
    pub root: String,
    pub output_dir: String,
    pub intervals: Vec<String>,
    pub reports: Vec<CleanFuturesReport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CleanFuturesDatasetReport {
    pub market: String,
    pub source_path: String,
    pub symbology_path: String,
    pub output_path: String,
    pub summary: CleanedContinuousFuturesSummary,
}

impl MultiTimeframeCleanReportView for MultiTimeframeCleanFuturesReport {
    fn interval_dataset_output_pairs<'a>(
        &'a self,
        market: &'a str,
    ) -> Box<dyn Iterator<Item = (&'a str, &'a str)> + 'a> {
        Box::new(self.reports.iter().filter_map(move |report| {
            report
                .datasets
                .iter()
                .find(|dataset| dataset.market == market)
                .map(|dataset| (report.interval.as_str(), dataset.output_path.as_str()))
        }))
    }
}

pub fn run_clean_futures_multi_timeframe(
    root: &str,
    output_dir: &str,
) -> Result<MultiTimeframeCleanFuturesReport> {
    let intervals = MULTI_TIMEFRAME_INTERVALS
        .iter()
        .map(|interval| (*interval).to_string())
        .collect::<Vec<_>>();
    std::fs::create_dir_all(output_dir)?;
    let mut reports = Vec::new();
    for interval in &intervals {
        let interval_output_dir = std::path::Path::new(output_dir)
            .join(format!("cleaned-{}", interval))
            .to_string_lossy()
            .to_string();
        reports.push(run_clean_futures(root, &interval_output_dir, interval)?);
    }
    let manifest_path = std::path::Path::new(output_dir)
        .join("cleaned-multi-timeframe-manifest.json")
        .to_string_lossy()
        .to_string();
    let report = MultiTimeframeCleanFuturesReport {
        root: root.to_string(),
        output_dir: output_dir.to_string(),
        intervals,
        reports,
    };
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&report)?)?;
    Ok(report)
}

pub fn run_clean_futures(
    root: &str,
    output_dir: &str,
    interval: &str,
) -> Result<CleanFuturesReport> {
    let interval_minutes = parse_interval_minutes(interval)?;
    std::fs::create_dir_all(output_dir)?;
    let datasets = discover_tomac_futures_datasets(root)?;
    if datasets.is_empty() {
        bail!("no TOMAC futures datasets found under '{}'", root);
    }

    let mut reports = Vec::new();
    for (ohlcv_path, symbology_path) in datasets {
        let market = infer_market_code_from_path(&ohlcv_path);
        let (continuous, mut summary) =
            load_tomac_continuous_candles(&ohlcv_path, &symbology_path)?;
        let cleaned = aggregate_candles_by_minutes(&continuous, interval_minutes)?;
        summary.matched_front_rows = continuous.len();
        summary.continuous_candles = continuous.len();
        summary.aggregated_candles = cleaned.len();

        let output_path = std::path::Path::new(output_dir)
            .join(format!(
                "{}.continuous-{}.json",
                market.to_ascii_lowercase(),
                interval
            ))
            .to_string_lossy()
            .to_string();
        std::fs::write(
            &output_path,
            serde_json::to_string_pretty(&CleanedCandleOutput {
                symbol: market.clone(),
                candles: cleaned,
            })?,
        )?;
        reports.push(CleanFuturesDatasetReport {
            market,
            source_path: ohlcv_path,
            symbology_path,
            output_path,
            summary,
        });
    }

    reports.sort_by(|a, b| a.market.cmp(&b.market));
    Ok(CleanFuturesReport {
        root: root.to_string(),
        output_dir: output_dir.to_string(),
        interval: interval.to_string(),
        datasets: reports,
    })
}

pub fn discover_tomac_futures_datasets(root: &str) -> Result<Vec<(String, String)>> {
    let mut stack = vec![std::path::PathBuf::from(root)];
    let mut datasets = Vec::new();

    while let Some(path) = stack.pop() {
        if path.is_dir() {
            for entry in std::fs::read_dir(&path)
                .with_context(|| format!("failed to read directory '{}'", path.display()))?
            {
                let entry = entry?;
                stack.push(entry.path());
            }
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !name.ends_with(".ohlcv-1m.csv") {
            continue;
        }
        let Some(parent) = path.parent() else {
            continue;
        };
        let symbology = parent.join("symbology.csv");
        if symbology.exists() {
            datasets.push((
                path.to_string_lossy().to_string(),
                symbology.to_string_lossy().to_string(),
            ));
        }
    }

    datasets.sort();
    Ok(datasets)
}

pub fn infer_market_code_from_path(path: &str) -> String {
    let parent = std::path::Path::new(path)
        .parent()
        .and_then(|value| value.file_name())
        .and_then(|value| value.to_str())
        .unwrap_or("market");
    parent
        .split_whitespace()
        .next()
        .unwrap_or(parent)
        .to_ascii_uppercase()
}
