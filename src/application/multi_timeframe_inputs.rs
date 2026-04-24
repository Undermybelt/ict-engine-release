use anyhow::{anyhow, bail, Result};
use std::collections::BTreeMap;

use crate::application::data_sources::discover_tomac_futures_datasets;
use crate::data::load_candles;

pub const MULTI_TIMEFRAME_INTERVALS: [&str; 6] = ["1m", "5m", "15m", "1h", "4h", "1d"];

#[derive(Debug, Clone, Default)]
pub struct MultiTimeframeResearchSignal {
    pub summary: Vec<String>,
}

pub trait MultiTimeframeCleanReportView {
    fn interval_dataset_output_pairs<'a>(
        &'a self,
        market: &'a str,
    ) -> Box<dyn Iterator<Item = (&'a str, &'a str)> + 'a>;
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedMultiTimeframeInputs {
    pub source: String,
    pub paths: BTreeMap<String, String>,
}

impl ResolvedMultiTimeframeInputs {
    pub fn get(&self, interval: &str) -> Option<&str> {
        self.paths.get(interval).map(String::as_str)
    }
}

pub fn parse_cleaned_continuous_identity(path: &str) -> Option<(String, String)> {
    let file_name = std::path::Path::new(path).file_name()?.to_str()?;
    let stem = file_name.strip_suffix(".json")?;
    let (market, interval) = stem.rsplit_once(".continuous-")?;
    Some((market.to_string(), interval.to_string()))
}

pub fn auto_resolve_multi_timeframe_inputs(primary_data: &str) -> ResolvedMultiTimeframeInputs {
    let mut resolved = ResolvedMultiTimeframeInputs::default();
    let primary_path = std::path::Path::new(primary_data);
    let Some((market, primary_interval)) = parse_cleaned_continuous_identity(primary_data) else {
        resolved.source = "primary_only".to_string();
        return resolved;
    };

    resolved
        .paths
        .insert(primary_interval.clone(), primary_data.to_string());

    let mut root_candidates = Vec::new();
    if let Some(parent) = primary_path.parent() {
        root_candidates.push(parent.to_path_buf());
        if let Some(root) = parent.parent() {
            root_candidates.push(root.to_path_buf());
        }
    }

    for interval in MULTI_TIMEFRAME_INTERVALS {
        if interval == primary_interval {
            continue;
        }
        for root in &root_candidates {
            let candidate = root
                .join(format!("cleaned-{}", interval))
                .join(format!("{}.continuous-{}.json", market, interval));
            if candidate.exists() {
                resolved.paths.insert(
                    interval.to_string(),
                    candidate.to_string_lossy().to_string(),
                );
                break;
            }
        }
    }

    resolved.source = if resolved.paths.len() > 1 {
        "auto_from_cleaned_siblings".to_string()
    } else {
        "primary_only".to_string()
    };
    resolved
}

pub fn resolve_multi_timeframe_inputs(
    primary_data: &str,
    data_1m: Option<&str>,
    data_5m: Option<&str>,
    data_15m: Option<&str>,
    data_1h: Option<&str>,
    data_4h: Option<&str>,
    data_1d: Option<&str>,
) -> ResolvedMultiTimeframeInputs {
    let mut resolved = auto_resolve_multi_timeframe_inputs(primary_data);
    let explicit = [
        ("1m", data_1m),
        ("5m", data_5m),
        ("15m", data_15m),
        ("1h", data_1h),
        ("4h", data_4h),
        ("1d", data_1d),
    ];
    let explicit_count = explicit.iter().filter(|(_, path)| path.is_some()).count();
    for (interval, path) in explicit {
        if let Some(path) = path {
            resolved
                .paths
                .insert(interval.to_string(), path.to_string());
        }
    }
    if explicit_count > 0 {
        resolved.source = if resolved.paths.len() > explicit_count {
            "explicit_with_auto_fill".to_string()
        } else {
            "explicit".to_string()
        };
    }
    if resolved.source.is_empty() {
        resolved.source = "primary_only".to_string();
    }
    resolved
}

pub fn infer_interval_for_analyze_frame(path: &str, fallback: &str) -> String {
    parse_cleaned_continuous_identity(path)
        .map(|(_, interval)| interval)
        .unwrap_or_else(|| fallback.to_string())
}

pub fn resolve_analyze_multi_timeframe_inputs(
    data_htf: &str,
    data_mtf: &str,
    data_ltf: &str,
) -> ResolvedMultiTimeframeInputs {
    let mut resolved = resolve_multi_timeframe_inputs(data_ltf, None, None, None, None, None, None);
    for (path, fallback) in [(data_htf, "1d"), (data_mtf, "1h"), (data_ltf, "15m")] {
        let interval = infer_interval_for_analyze_frame(path, fallback);
        resolved.paths.insert(interval, path.to_string());
    }
    resolved.source = match resolved.source.as_str() {
        "auto_from_cleaned_siblings" => "analyze_explicit_with_auto_fill".to_string(),
        "primary_only" => "analyze_explicit_frames".to_string(),
        other => other.to_string(),
    };
    resolved
}

pub fn resolved_multi_timeframe_inputs_for_market<T>(
    clean_report: &T,
    market: &str,
) -> ResolvedMultiTimeframeInputs
where
    T: MultiTimeframeCleanReportView,
{
    let mut resolved = ResolvedMultiTimeframeInputs {
        source: "clean_futures_multi_timeframe".to_string(),
        ..ResolvedMultiTimeframeInputs::default()
    };
    for (interval, output_path) in clean_report.interval_dataset_output_pairs(market) {
        resolved
            .paths
            .insert(interval.to_string(), output_path.to_string());
    }
    if resolved.paths.is_empty() {
        resolved.source = "primary_only".to_string();
    }
    resolved
}

pub fn resolve_analyze_cli_inputs(
    symbol: &str,
    data_htf: Option<&str>,
    data_mtf: Option<&str>,
    data_ltf: Option<&str>,
    data_root: Option<&str>,
    demo: bool,
) -> Result<(String, String, String)> {
    if demo {
        let demo_path = "examples/demo/demo-15m.json".to_string();
        return Ok((demo_path.clone(), demo_path.clone(), demo_path));
    }
    if let (Some(htf), Some(mtf), Some(ltf)) = (data_htf, data_mtf, data_ltf) {
        return Ok((htf.to_string(), mtf.to_string(), ltf.to_string()));
    }
    let data_root = data_root.ok_or_else(|| {
        anyhow!("analyze requires either --demo, --data-htf/--data-mtf/--data-ltf, or --data-root")
    })?;
    let market = symbol.to_ascii_lowercase();
    let resolve = |interval: &str| -> Result<String> {
        let path = std::path::Path::new(data_root)
            .join(format!("cleaned-{}", interval))
            .join(format!("{}.continuous-{}.json", market, interval));
        if path.exists() {
            Ok(path.to_string_lossy().to_string())
        } else {
            bail!(
                "missing analyze input for interval '{}' under root '{}'",
                interval,
                data_root
            )
        }
    };
    Ok((resolve("1d")?, resolve("1h")?, resolve("15m")?))
}

pub fn default_tomac_root_candidates() -> Vec<String> {
    let mut candidates = Vec::new();
    if let Ok(root) = std::env::var("ICT_ENGINE_TOMAC_ROOT") {
        if !root.trim().is_empty() {
            candidates.push(root);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        for candidate in [
            format!("{home}/Downloads/Tomac"),
            format!("{home}/Downloads/tomac"),
            format!("{home}/Documents/Tomac"),
            format!("{home}/Documents/tomac"),
        ] {
            candidates.push(candidate);
        }
    }
    candidates
}

pub fn find_tomac_root_from_candidates(candidates: &[String]) -> Option<String> {
    candidates.iter().find_map(|candidate| {
        let path = std::path::Path::new(candidate);
        if !path.is_dir() {
            return None;
        }
        discover_tomac_futures_datasets(candidate)
            .ok()
            .filter(|datasets| !datasets.is_empty())
            .map(|_| candidate.clone())
    })
}

pub fn detected_tomac_root() -> Option<String> {
    find_tomac_root_from_candidates(&default_tomac_root_candidates())
}

pub fn resolve_tomac_root(root: Option<&str>) -> Result<String> {
    if let Some(root) = root {
        return Ok(root.to_string());
    }
    detected_tomac_root().ok_or_else(|| {
        anyhow!(
            "no TOMAC root provided and no default TOMAC history directory detected; set --root or ICT_ENGINE_TOMAC_ROOT"
        )
    })
}

pub fn detected_tomac_root_or_placeholder() -> String {
    detected_tomac_root().unwrap_or_else(|| "<root>".to_string())
}

pub fn build_multi_timeframe_summary(
    primary_data: &str,
    resolved: &ResolvedMultiTimeframeInputs,
) -> Result<Vec<String>> {
    let mut summary = Vec::new();
    if !resolved.paths.is_empty() {
        let covered = MULTI_TIMEFRAME_INTERVALS
            .iter()
            .filter(|interval| resolved.get(interval).is_some())
            .copied()
            .collect::<Vec<_>>();
        let missing = MULTI_TIMEFRAME_INTERVALS
            .iter()
            .filter(|interval| resolved.get(interval).is_none())
            .copied()
            .collect::<Vec<_>>();
        summary.push(format!(
            "multi_timeframe_source={} covered_intervals={}",
            resolved.source,
            covered.join(",")
        ));
        if !missing.is_empty() {
            summary.push(format!("multi_timeframe_missing={}", missing.join(",")));
        }
    }
    for interval in MULTI_TIMEFRAME_INTERVALS {
        let Some(path) = resolved.get(interval) else {
            continue;
        };
        let candles = load_candles(path)?;
        summary.push(format!("{}:{} bars path={}", interval, candles.len(), path));
    }
    if summary.is_empty() {
        let primary = load_candles(primary_data)?;
        summary.push(format!(
            "primary:{} bars path={}",
            primary.len(),
            primary_data
        ));
    }
    Ok(summary)
}

fn candle_trend(candles: &[crate::types::Candle]) -> Option<f64> {
    if candles.len() < 2 {
        return None;
    }
    let first = candles
        .first()
        .map(|candle| candle.close)
        .unwrap_or_default();
    let last = candles
        .last()
        .map(|candle| candle.close)
        .unwrap_or_default();
    if first.abs() <= f64::EPSILON {
        return None;
    }
    Some((last - first) / first)
}

fn multi_timeframe_signal_from_trends(
    long_term: &[f64],
    short_term: &[f64],
) -> MultiTimeframeResearchSignal {
    let long_avg = if long_term.is_empty() {
        0.0
    } else {
        long_term.iter().sum::<f64>() / long_term.len() as f64
    };
    let short_avg = if short_term.is_empty() {
        0.0
    } else {
        short_term.iter().sum::<f64>() / short_term.len() as f64
    };
    let alignment_score = 1.0 - (long_avg - short_avg).abs().min(1.0);
    let entry_alignment_score = 1.0 - short_avg.abs().min(1.0);
    let direction_bias = if long_avg > 0.001 {
        "bullish".to_string()
    } else if long_avg < -0.001 {
        "bearish".to_string()
    } else {
        "neutral".to_string()
    };
    MultiTimeframeResearchSignal {
        summary: vec![
            format!("higher_timeframe_direction_bias={}", direction_bias),
            format!("higher_timeframe_alignment_score={:.4}", alignment_score),
            format!(
                "lower_timeframe_entry_alignment_score={:.4}",
                entry_alignment_score
            ),
        ],
    }
}

pub fn build_multi_timeframe_research_signal(
    resolved: &ResolvedMultiTimeframeInputs,
) -> Result<MultiTimeframeResearchSignal> {
    let load_trend = |path: Option<&str>| -> Result<Option<f64>> {
        Ok(path
            .map(load_candles)
            .transpose()?
            .as_deref()
            .and_then(candle_trend))
    };
    let long_term = [
        load_trend(resolved.get("1d"))?,
        load_trend(resolved.get("4h"))?,
        load_trend(resolved.get("1h"))?,
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let short_term = [
        load_trend(resolved.get("15m"))?,
        load_trend(resolved.get("5m"))?,
        load_trend(resolved.get("1m"))?,
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    Ok(multi_timeframe_signal_from_trends(&long_term, &short_term))
}

pub fn build_live_multi_timeframe_signal(
    frames: &[(&str, &[crate::types::Candle])],
) -> MultiTimeframeResearchSignal {
    let frame_map = frames
        .iter()
        .map(|(interval, candles)| ((*interval).to_string(), *candles))
        .collect::<BTreeMap<_, _>>();
    let long_term = ["1d", "4h", "1h"]
        .into_iter()
        .filter_map(|interval| {
            frame_map
                .get(interval)
                .and_then(|candles| candle_trend(candles))
        })
        .collect::<Vec<_>>();
    let short_term = ["15m", "5m", "1m"]
        .into_iter()
        .filter_map(|interval| {
            frame_map
                .get(interval)
                .and_then(|candles| candle_trend(candles))
        })
        .collect::<Vec<_>>();
    multi_timeframe_signal_from_trends(&long_term, &short_term)
}

pub fn is_multi_timeframe_clean_root(path: &std::path::Path) -> bool {
    path.join("cleaned-1d").is_dir()
        && path.join("cleaned-4h").is_dir()
        && path.join("cleaned-1h").is_dir()
        && path.join("cleaned-15m").is_dir()
        && path.join("cleaned-5m").is_dir()
        && path.join("cleaned-1m").is_dir()
}

pub fn detected_multi_timeframe_clean_root(tomac_root: Option<&str>) -> Option<String> {
    let tomac_root = tomac_root?;
    let root = std::path::Path::new(tomac_root);
    if is_multi_timeframe_clean_root(root) {
        return Some(root.to_string_lossy().to_string());
    }
    std::fs::read_dir(root)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.is_dir() && is_multi_timeframe_clean_root(path))
        .map(|path| path.to_string_lossy().to_string())
}
