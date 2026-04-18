use anyhow::{anyhow, bail, Result};
use std::collections::BTreeMap;

pub const MULTI_TIMEFRAME_INTERVALS: [&str; 6] = ["1m", "5m", "15m", "1h", "4h", "1d"];

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
