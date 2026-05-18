use anyhow::{bail, Context, Result};
use csv::Writer;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::data::load_candles;

use super::handoff::AutoQuantWorkspaceConfig;

pub const AUTO_QUANT_WORKSPACE_PROFILE_FILE: &str = "auto_quant_workspace_profile.json";
pub const AUTO_QUANT_PROFILE_SYNTHETIC_OHLCV: &str = "synthetic_ohlcv";
pub const AUTO_QUANT_PROFILE_MANAGED: &str = "managed";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoQuantWorkspaceProfileConfig {
    pub profile: String,
    pub symbol: String,
    pub source_data_path: String,
    pub pair: String,
    pub base_timeframe: String,
    pub additional_timeframes: Vec<String>,
    pub notes: Vec<String>,
}

pub fn load_workspace_profile(state_dir: &str) -> Result<Option<AutoQuantWorkspaceProfileConfig>> {
    let path = workspace_profile_path(state_dir);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let profile =
        serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
    Ok(Some(profile))
}

pub fn persist_workspace_profile_selection(
    state_dir: &str,
    profile_name: Option<&str>,
    symbol: &str,
    source_data_path: &str,
) -> Result<Option<AutoQuantWorkspaceProfileConfig>> {
    match profile_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        None => load_workspace_profile(state_dir),
        Some(value)
            if value.eq_ignore_ascii_case(AUTO_QUANT_PROFILE_MANAGED)
                || value.eq_ignore_ascii_case("default") =>
        {
            clear_workspace_profile(state_dir)?;
            Ok(None)
        }
        Some(value) if value.eq_ignore_ascii_case(AUTO_QUANT_PROFILE_SYNTHETIC_OHLCV) => {
            let profile = AutoQuantWorkspaceProfileConfig {
                profile: AUTO_QUANT_PROFILE_SYNTHETIC_OHLCV.to_string(),
                symbol: symbol.to_string(),
                source_data_path: source_data_path.to_string(),
                pair: format!("{symbol}/USD"),
                base_timeframe: "1h".to_string(),
                additional_timeframes: vec!["4h".to_string(), "1d".to_string()],
                notes: vec![
                    "profile_materializes_additive_external_runner".to_string(),
                    "profile_reuses_primary_cleaned_candle_json_as_prepare_external_source"
                        .to_string(),
                    "profile_is_opt_in_and_state_dir_scoped".to_string(),
                ],
            };
            save_workspace_profile(state_dir, &profile)?;
            Ok(Some(profile))
        }
        Some(value) => bail!("unknown auto-quant profile '{}'", value),
    }
}

pub fn apply_workspace_profile(
    state_dir: &str,
    workspace: &mut AutoQuantWorkspaceConfig,
) -> Result<Option<AutoQuantWorkspaceProfileConfig>> {
    let Some(profile) = load_workspace_profile(state_dir)? else {
        return Ok(None);
    };
    if profile.profile == AUTO_QUANT_PROFILE_SYNTHETIC_OHLCV {
        let repo_root = PathBuf::from(&workspace.repo_root);
        workspace.profile_name = Some(profile.profile.clone());
        workspace.prepare_script = repo_root
            .join("prepare_external.py")
            .to_string_lossy()
            .to_string();
        workspace.run_script = repo_root.join("run_tomac.py").to_string_lossy().to_string();
        workspace.config_json = repo_root
            .join("config.tomac.json")
            .to_string_lossy()
            .to_string();
        workspace.strategies_dir = repo_root
            .join("user_data/strategies_external")
            .to_string_lossy()
            .to_string();
        workspace.expected_data_files = expected_data_files(&profile);
        workspace.strategy_seed_source_dir = Some(
            repo_root
                .join("user_data/strategies")
                .to_string_lossy()
                .to_string(),
        );
    }
    Ok(Some(profile))
}

pub fn materialize_workspace_profile(
    state_dir: &str,
    workspace: &AutoQuantWorkspaceConfig,
) -> Result<Option<AutoQuantWorkspaceProfileConfig>> {
    let Some(profile) = load_workspace_profile(state_dir)? else {
        return Ok(None);
    };
    if profile.profile != AUTO_QUANT_PROFILE_SYNTHETIC_OHLCV {
        return Ok(Some(profile));
    }
    let workspace_root = PathBuf::from(&workspace.repo_root);
    fs::create_dir_all(workspace_root.join("user_data/strategies_external"))?;
    fs::create_dir_all(workspace_root.join("user_data/data"))?;

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::copy(
        repo_root.join("support/scripts/auto_quant_external/run_tomac.py"),
        workspace_root.join("run_tomac.py"),
    )?;
    fs::copy(
        repo_root.join("support/scripts/auto_quant_external/prepare_external.py"),
        workspace_root.join("prepare_external.py"),
    )?;
    fs::copy(
        repo_root.join("support/scripts/auto_quant_external/config.tomac.json"),
        workspace_root.join("config.tomac.json"),
    )?;
    write_profile_config(&workspace_root.join("config.tomac.json"), &profile)?;
    write_profile_source_csv(
        &profile.source_data_path,
        &workspace_root.join("profile_source.csv"),
    )?;
    seed_profile_strategies(
        workspace
            .strategy_seed_source_dir
            .as_deref()
            .map(Path::new)
            .unwrap_or_else(|| Path::new(&workspace.strategies_dir)),
        &workspace_root.join("user_data/strategies_external"),
        &repo_root
            .join("support/scripts/auto_quant_external/strategies/TomacNQ_KillzoneBreakout.py"),
        &profile.symbol,
    )?;
    Ok(Some(profile))
}

fn workspace_profile_path(state_dir: &str) -> PathBuf {
    PathBuf::from(state_dir).join(AUTO_QUANT_WORKSPACE_PROFILE_FILE)
}

fn save_workspace_profile(
    state_dir: &str,
    profile: &AutoQuantWorkspaceProfileConfig,
) -> Result<()> {
    let path = workspace_profile_path(state_dir);
    fs::write(&path, serde_json::to_string_pretty(profile)?)
        .with_context(|| format!("writing {}", path.display()))
}

fn clear_workspace_profile(state_dir: &str) -> Result<()> {
    let path = workspace_profile_path(state_dir);
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
    }
    Ok(())
}

fn expected_data_files(profile: &AutoQuantWorkspaceProfileConfig) -> Vec<String> {
    std::iter::once(profile.base_timeframe.clone())
        .chain(profile.additional_timeframes.clone())
        .map(|timeframe| format!("{}-{timeframe}.feather", profile.pair.replace('/', "_")))
        .collect()
}

fn write_profile_config(path: &Path, profile: &AutoQuantWorkspaceProfileConfig) -> Result<()> {
    let mut config: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(path)?).context("parsing config.tomac.json")?;
    config["timeframe"] = serde_json::Value::String(profile.base_timeframe.clone());
    config["exchange"]["pair_whitelist"] = serde_json::json!([profile.pair.clone()]);
    config["trading_mode"] = serde_json::Value::String("spot".to_string());
    config
        .as_object_mut()
        .map(|root| root.remove("margin_mode"));
    config["entry_pricing"]["use_order_book"] = serde_json::Value::Bool(false);
    config["exit_pricing"]["use_order_book"] = serde_json::Value::Bool(false);
    if let Some(exchange) = config["exchange"].as_object_mut() {
        exchange.remove("_ft_has_params");
    }
    fs::write(path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}

fn write_profile_source_csv(input_path: &str, csv_path: &Path) -> Result<()> {
    let candles = load_candles(input_path)?;
    let mut writer = Writer::from_path(csv_path)?;
    writer.write_record(["date", "open", "high", "low", "close", "volume"])?;
    for candle in candles {
        writer.write_record([
            candle.timestamp.to_rfc3339(),
            candle.open.to_string(),
            candle.high.to_string(),
            candle.low.to_string(),
            candle.close.to_string(),
            candle.volume.to_string(),
        ])?;
    }
    writer.flush()?;
    Ok(())
}

fn seed_profile_strategies(
    source_dir: &Path,
    target_dir: &Path,
    fallback_strategy_path: &Path,
    profile_symbol: &str,
) -> Result<()> {
    fs::create_dir_all(target_dir)?;
    for entry in fs::read_dir(target_dir)? {
        let entry = entry?;
        let path = entry.path();
        let is_python = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("py"))
            .unwrap_or(false);
        if is_python {
            fs::remove_file(path)?;
        }
    }
    let mut copied = 0usize;
    if source_dir.exists() {
        for entry in fs::read_dir(source_dir)? {
            let entry = entry?;
            let path = entry.path();
            let is_python = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("py"))
                .unwrap_or(false);
            let is_active = entry
                .file_name()
                .to_str()
                .map(|name| !name.starts_with('_'))
                .unwrap_or(false);
            if is_python && is_active {
                write_strategy_with_meta(
                    &path,
                    &target_dir.join(entry.file_name()),
                    profile_symbol,
                )?;
                copied += 1;
            }
        }
    }
    if copied == 0 {
        let filename = fallback_strategy_path
            .file_name()
            .context("missing fallback strategy filename")?;
        write_strategy_with_meta(
            fallback_strategy_path,
            &target_dir.join(filename),
            profile_symbol,
        )?;
    }
    Ok(())
}

fn write_strategy_with_meta(
    source_path: &Path,
    target_path: &Path,
    profile_symbol: &str,
) -> Result<()> {
    let source = fs::read_to_string(source_path)
        .with_context(|| format!("reading strategy source {}", source_path.display()))?;
    let rendered = if source.contains("# AUTO_QUANT_META") {
        source
    } else {
        let strategy = target_path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("SyntheticProfileStrategy");
        let base_factor = camel_to_snake(strategy);
        let hypothesis = sanitize_auto_quant_meta_value(
            &extract_doc_field(&source, "Hypothesis")
                .unwrap_or_else(|| format!("{strategy} hypothesis under synthetic_ohlcv profile.")),
        );
        let paradigm = sanitize_auto_quant_meta_value(
            &extract_doc_field(&source, "Paradigm").unwrap_or_else(|| "other".to_string()),
        );
        let parent = sanitize_auto_quant_meta_value(
            &extract_doc_field(&source, "Parent").unwrap_or_else(|| "root".to_string()),
        );
        let status = sanitize_auto_quant_meta_value(
            &extract_doc_field(&source, "Status").unwrap_or_else(|| "active".to_string()),
        );
        let created = sanitize_auto_quant_meta_value(
            &extract_doc_field(&source, "Created").unwrap_or_default(),
        );
        let expected_regime =
            if source.contains("@informative(\"1d\")") || source.contains("@informative(\"4h\")") {
                "multi_timeframe_intraday_resonance"
            } else {
                "single_timeframe_intraday"
            };
        let asset_class = match profile_symbol {
            "NQ" | "ES" | "YM" => "futures_index",
            "GC" | "CL" => "futures_commodity",
            _ => "synthetic_ohlcv",
        };
        let meta_block = format!(
            "# AUTO_QUANT_META v1\nStrategy:        {strategy}\nMutation_id:     synthetic-ohlcv-{strategy}\nBase_factor:     {base_factor}\nHypothesis:      {hypothesis}\nParadigm:        {paradigm}\nExpected_regime: {expected_regime}\nFactors_used:    {base_factor}\nParent:          {parent}\nAsset_class:     {asset_class}\nStatus:          {status}\nCreated:         {created}\n# END_AUTO_QUANT_META"
        );
        inject_auto_quant_meta_into_docstring(&source, &meta_block)
    };
    fs::write(target_path, rendered)
        .with_context(|| format!("writing exportable strategy {}", target_path.display()))
}

fn extract_doc_field(source: &str, label: &str) -> Option<String> {
    source
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            let (left, right) = trimmed.split_once(':')?;
            if left.trim() == label {
                Some(right.trim().to_string())
            } else {
                None
            }
        })
        .filter(|value| !value.is_empty())
}

fn camel_to_snake(raw: &str) -> String {
    let mut out = String::new();
    for (index, ch) in raw.chars().enumerate() {
        if ch.is_ascii_uppercase() && index > 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

fn sanitize_auto_quant_meta_value(raw: &str) -> String {
    raw.replace("<=", " at_or_below ")
        .replace(">=", " at_or_above ")
        .replace('<', " below ")
        .replace('>', " above ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn inject_auto_quant_meta_into_docstring(source: &str, meta_block: &str) -> String {
    for delimiter in ["\"\"\"", "'''"] {
        if let Some(rest) = source.strip_prefix(delimiter) {
            if let Some(end) = rest.find(delimiter) {
                let doc = &rest[..end];
                let suffix = &rest[end..];
                let mut merged = String::new();
                merged.push_str(delimiter);
                merged.push_str(doc.trim_end());
                if !doc.trim_end().is_empty() {
                    merged.push_str("\n\n");
                }
                merged.push_str(meta_block);
                merged.push('\n');
                merged.push_str(suffix);
                return merged;
            }
        }
    }
    format!("\"\"\"\n{meta_block}\n\"\"\"\n\n{source}")
}
