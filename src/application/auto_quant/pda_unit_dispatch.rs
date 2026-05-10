use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
use csv::Writer;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use crate::data::loader::load_candles;
use crate::state::{
    append_artifact_ledger_entry, artifact_state_path, load_artifact_ledger, save_state,
    ArtifactLedgerEntry,
};

use super::handoff::AutoQuantResearchHandoffPayload;
use super::pda_unit_batch::{
    AutoQuantConsumerEvidenceProfile, AutoQuantPdaUnitBatchArtifact, AutoQuantPdaUnitJob,
};

pub const AUTO_QUANT_PDA_UNIT_DISPATCH_RULE_VERSION: &str = "auto-quant-pda-unit-dispatch-v1";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutoQuantPdaUnitDispatchArtifact {
    pub artifact_id: String,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub batch_artifact_id: String,
    pub selected_group_indices: Vec<usize>,
    pub groups: Vec<AutoQuantPdaDispatchGroupResult>,
    pub totals: AutoQuantPdaDispatchTotals,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutoQuantPdaDispatchGroupResult {
    pub group_index: usize,
    pub unit_results: Vec<AutoQuantPdaUnitResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutoQuantPdaDispatchTotals {
    pub total_units: usize,
    pub completed_units: usize,
    pub blocked_units: usize,
    pub failed_units: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutoQuantPdaUnitResult {
    pub unit_id: String,
    pub unit_label: String,
    pub status: String,
    pub reason: String,
    pub workspace_root: String,
    pub stdout_log_path: String,
    pub stderr_log_path: String,
    pub aggregate_metrics: Option<AutoQuantPdaUnitAggregateMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutoQuantPdaUnitAggregateMetrics {
    pub sharpe: f64,
    pub sortino: f64,
    pub calmar: f64,
    pub total_profit_pct: f64,
    pub max_drawdown_pct: f64,
    pub trade_count: usize,
    pub win_rate_pct: f64,
    pub profit_factor: f64,
}

pub struct AutoQuantPdaUnitDispatchInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
    pub batch_artifact_id: Option<&'a str>,
    pub group_indices: Option<&'a str>,
}

pub fn dispatch_pda_unit_batch(
    input: AutoQuantPdaUnitDispatchInput<'_>,
) -> Result<AutoQuantPdaUnitDispatchArtifact> {
    let batch = load_target_batch(input.state_dir, input.symbol, input.batch_artifact_id)?;
    let selected_group_indices =
        parse_group_indices(input.group_indices, batch.dispatch_groups.len())?;

    let mut groups = Vec::new();
    for group_index in &selected_group_indices {
        let group = batch
            .dispatch_groups
            .iter()
            .find(|item| item.group_index == *group_index)
            .ok_or_else(|| anyhow!("unknown dispatch group index {}", group_index))?;
        let jobs = group
            .unit_ids
            .iter()
            .filter_map(|unit_id| batch.unit_jobs.iter().find(|job| &job.unit_id == unit_id))
            .cloned()
            .collect::<Vec<_>>();
        let shared_workspace_root = batch.shared_workspace_root.clone();
        let handles = jobs
            .into_iter()
            .map(|job| {
                let shared_workspace_root = shared_workspace_root.clone();
                thread::spawn(move || dispatch_one_unit(job, &shared_workspace_root))
            })
            .collect::<Vec<_>>();
        let mut unit_results = Vec::new();
        for handle in handles {
            unit_results.push(
                handle
                    .join()
                    .map_err(|_| anyhow!("unit dispatch thread panicked"))??,
            );
        }
        groups.push(AutoQuantPdaDispatchGroupResult {
            group_index: *group_index,
            unit_results,
        });
    }

    let totals = summarize_dispatch_totals(&groups);
    let artifact = AutoQuantPdaUnitDispatchArtifact {
        artifact_id: format!(
            "auto-quant-pda-unit-dispatch:{}:{}",
            input.symbol,
            Utc::now().format("%Y%m%dT%H%M%S%.3fZ")
        ),
        generated_at: Utc::now(),
        symbol: input.symbol.to_string(),
        batch_artifact_id: batch.artifact_id,
        selected_group_indices,
        groups,
        totals,
    };
    persist_dispatch_artifact(input.state_dir, &artifact)?;
    Ok(artifact)
}

fn load_target_batch(
    state_dir: &str,
    symbol: &str,
    batch_artifact_id: Option<&str>,
) -> Result<AutoQuantPdaUnitBatchArtifact> {
    let ledger = load_artifact_ledger(state_dir, symbol)?;
    let target = ledger
        .iter()
        .rev()
        .find(|entry| {
            entry.artifact_kind == "auto_quant_pda_unit_batch"
                && batch_artifact_id.is_none_or(|target| entry.artifact_id == target)
        })
        .ok_or_else(|| anyhow!("no auto_quant_pda_unit_batch artifact found for {}", symbol))?;
    let content = fs::read_to_string(&target.path)
        .with_context(|| format!("reading batch artifact '{}'", target.path))?;
    serde_json::from_str(&content)
        .with_context(|| format!("parsing batch artifact '{}'", target.path))
}

fn parse_group_indices(raw: Option<&str>, group_count: usize) -> Result<Vec<usize>> {
    if let Some(raw) = raw {
        let mut out = Vec::new();
        for item in raw.split(',') {
            let trimmed = item.trim();
            if trimmed.is_empty() {
                continue;
            }
            let idx = trimmed
                .parse::<usize>()
                .with_context(|| format!("invalid group index '{}'", trimmed))?;
            if idx >= group_count {
                bail!("group index {} out of range 0..{}", idx, group_count);
            }
            if !out.contains(&idx) {
                out.push(idx);
            }
        }
        if out.is_empty() {
            bail!("at least one valid group index is required");
        }
        Ok(out)
    } else {
        Ok((0..group_count).collect())
    }
}

fn dispatch_one_unit(
    job: AutoQuantPdaUnitJob,
    shared_workspace_root: &str,
) -> Result<AutoQuantPdaUnitResult> {
    let Some(reason) = blocking_reason_for_profile(&job.brief.consumer_evidence_profile) else {
        return run_dispatch_unit(job, shared_workspace_root);
    };
    Ok(AutoQuantPdaUnitResult {
        unit_id: job.unit_id,
        unit_label: job.unit_label,
        status: "blocked".to_string(),
        reason,
        workspace_root: String::new(),
        stdout_log_path: String::new(),
        stderr_log_path: String::new(),
        aggregate_metrics: None,
    })
}

fn blocking_reason_for_profile(profile: &AutoQuantConsumerEvidenceProfile) -> Option<String> {
    let blocked = profile.required_surfaces.iter().any(|surface| {
        matches!(
            surface.as_str(),
            "greeks" | "open_interest" | "implied_volatility" | "options_chain" | "cross_market"
        )
    });
    blocked.then(|| {
        "dispatch_blocked_missing_external_evidence_provider: requested surfaces require external provider/runtime inputs not yet wired into direct dispatch"
            .to_string()
    })
}

fn run_dispatch_unit(
    job: AutoQuantPdaUnitJob,
    shared_workspace_root: &str,
) -> Result<AutoQuantPdaUnitResult> {
    let workspace_root = PathBuf::from(&job.isolated_state_dir).join("aq_workspace");
    let runtime_python =
        resolve_runtime_python(shared_workspace_root, job.handoff_artifact_path.as_deref())?;
    materialize_unit_workspace(
        &workspace_root,
        runtime_python.as_deref(),
        shared_workspace_root,
        &job,
    )?;
    let stdout_log_path = workspace_root.join("run_tomac.stdout.log");
    let stderr_log_path = workspace_root.join("run_tomac.stderr.log");

    let output = run_workspace_python_script(
        runtime_python.as_deref(),
        &workspace_root,
        "run_tomac.py",
        &[],
    )
    .with_context(|| format!("running run_tomac.py for unit '{}'", job.unit_label))?;
    fs::write(&stdout_log_path, &output.stdout)?;
    fs::write(&stderr_log_path, &output.stderr)?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        return Ok(AutoQuantPdaUnitResult {
            unit_id: job.unit_id,
            unit_label: job.unit_label,
            status: "failed".to_string(),
            reason: format!(
                "run_tomac_exit_nonzero: {}",
                stderr.lines().next().unwrap_or("unknown error")
            ),
            workspace_root: workspace_root.to_string_lossy().to_string(),
            stdout_log_path: stdout_log_path.to_string_lossy().to_string(),
            stderr_log_path: stderr_log_path.to_string_lossy().to_string(),
            aggregate_metrics: parse_run_tomac_aggregate_metrics(&stdout),
        });
    }

    Ok(AutoQuantPdaUnitResult {
        unit_id: job.unit_id,
        unit_label: job.unit_label,
        status: "completed".to_string(),
        reason: "external_auto_quant_run_completed".to_string(),
        workspace_root: workspace_root.to_string_lossy().to_string(),
        stdout_log_path: stdout_log_path.to_string_lossy().to_string(),
        stderr_log_path: stderr_log_path.to_string_lossy().to_string(),
        aggregate_metrics: parse_run_tomac_aggregate_metrics(&stdout),
    })
}

fn materialize_unit_workspace(
    workspace_root: &Path,
    runtime_python: Option<&Path>,
    shared_workspace_root: &str,
    job: &AutoQuantPdaUnitJob,
) -> Result<()> {
    fs::create_dir_all(workspace_root.join("user_data/strategies_external"))?;
    fs::create_dir_all(workspace_root.join("user_data/data"))?;

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let shared_root = PathBuf::from(shared_workspace_root);
    for filename in ["pyproject.toml", "uv.lock"] {
        fs::copy(shared_root.join(filename), workspace_root.join(filename))
            .with_context(|| format!("copying {}", filename))?;
    }
    fs::copy(
        repo_root.join("scripts/auto_quant_external/run_tomac.py"),
        workspace_root.join("run_tomac.py"),
    )?;
    fs::copy(
        repo_root.join("scripts/auto_quant_external/prepare_external.py"),
        workspace_root.join("prepare_external.py"),
    )?;
    fs::copy(
        repo_root.join("scripts/auto_quant_external/config.tomac.json"),
        workspace_root.join("config.tomac.json"),
    )?;

    write_unit_config(&workspace_root.join("config.tomac.json"), job)?;
    write_unit_candles_csv(&job.scope.data_path, &workspace_root.join("unit.csv"))?;
    write_unit_strategy_file(
        &workspace_root
            .join("user_data/strategies_external")
            .join(format!("{}.py", unit_strategy_class_name(&job.unit_id))),
        job,
    )?;

    let pair = format!("{}/USD", job.scope.symbol);
    let prepare = run_workspace_python_script(
        runtime_python,
        workspace_root,
        "prepare_external.py",
        &[
            "--csv",
            "unit.csv",
            "--pair",
            &pair,
            "--timeframes",
            &job.scope.timeframe,
            "--datadir",
            "user_data/data",
            "--column-map",
            "date:date,open:open,high:high,low:low,close:close,volume:volume",
            "--no-clean",
        ],
    )
    .with_context(|| format!("running prepare_external.py for '{}'", job.unit_label))?;
    if !prepare.status.success() {
        bail!(
            "prepare_external failed for '{}': {}",
            job.unit_label,
            String::from_utf8_lossy(&prepare.stderr)
        );
    }
    if job.scope.direction == "short" {
        materialize_futures_ohlcv_alias(workspace_root, &job.scope.symbol, &job.scope.timeframe)?;
    }
    Ok(())
}

fn materialize_futures_ohlcv_alias(
    workspace_root: &Path,
    symbol: &str,
    timeframe: &str,
) -> Result<()> {
    let pair_filename = format!("{}_USD", symbol);
    let source = workspace_root
        .join("user_data/data")
        .join(format!("{pair_filename}-{timeframe}.feather"));
    let target_dir = workspace_root.join("user_data/data/futures");
    fs::create_dir_all(&target_dir)?;
    let target = target_dir.join(format!("{pair_filename}-{timeframe}-futures.feather"));
    fs::copy(&source, &target).with_context(|| {
        format!(
            "copying futures candle alias from '{}' to '{}'",
            source.display(),
            target.display()
        )
    })?;
    Ok(())
}

fn resolve_runtime_python(
    shared_workspace_root: &str,
    handoff_artifact_path: Option<&str>,
) -> Result<Option<PathBuf>> {
    let shared_python = PathBuf::from(shared_workspace_root).join(".venv/bin/python");
    if shared_python.exists() {
        return Ok(Some(shared_python));
    }
    if let Some(handoff_path) = handoff_artifact_path {
        if let Ok(content) = fs::read_to_string(handoff_path) {
            if let Ok(payload) = serde_json::from_str::<AutoQuantResearchHandoffPayload>(&content) {
                let repo_url = payload.dependency_status.repo_url;
                if repo_url.starts_with('/') {
                    let candidate = PathBuf::from(repo_url).join(".venv/bin/python");
                    if candidate.exists() {
                        return Ok(Some(candidate));
                    }
                }
            }
        }
    }
    let output = Command::new("uv")
        .arg("sync")
        .arg("--frozen")
        .current_dir(shared_workspace_root)
        .output()
        .with_context(|| format!("running uv sync --frozen in '{}'", shared_workspace_root))?;
    if !output.status.success() {
        bail!(
            "failed to provision shared Auto-Quant runtime in '{}': {}",
            shared_workspace_root,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    if shared_python.exists() {
        Ok(Some(shared_python))
    } else {
        Ok(None)
    }
}

fn run_workspace_python_script(
    python_override: Option<&Path>,
    workspace_root: &Path,
    script_name: &str,
    args: &[&str],
) -> Result<std::process::Output> {
    let mut command = if let Some(shared_python) = python_override {
        let mut cmd = Command::new(shared_python);
        cmd.arg(script_name);
        cmd
    } else {
        let mut cmd = Command::new("uv");
        cmd.arg("run").arg(script_name);
        cmd
    };
    command.args(args);
    command.current_dir(workspace_root);
    command
        .output()
        .with_context(|| format!("running {} with shared AQ runtime", script_name))
}

fn write_unit_config(path: &Path, job: &AutoQuantPdaUnitJob) -> Result<()> {
    let mut config: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(path)?).context("parsing config.tomac.json")?;
    config["timeframe"] = serde_json::Value::String(job.scope.timeframe.clone());
    config["exchange"]["pair_whitelist"] = serde_json::json!([format!("{}/USD", job.scope.symbol)]);
    if job.scope.direction == "short" {
        config["trading_mode"] = serde_json::Value::String("futures".to_string());
        config["margin_mode"] = serde_json::Value::String("isolated".to_string());
        config["entry_pricing"]["use_order_book"] = serde_json::Value::Bool(true);
        config["exit_pricing"]["use_order_book"] = serde_json::Value::Bool(true);
        config["exchange"]["_ft_has_params"]["uses_leverage_tiers"] =
            serde_json::Value::Bool(false);
    } else {
        config["trading_mode"] = serde_json::Value::String("spot".to_string());
        config
            .as_object_mut()
            .map(|root| root.remove("margin_mode"));
        config["entry_pricing"]["use_order_book"] = serde_json::Value::Bool(false);
        config["exit_pricing"]["use_order_book"] = serde_json::Value::Bool(false);
        if let Some(exchange) = config["exchange"].as_object_mut() {
            exchange.remove("_ft_has_params");
        }
    }
    fs::write(path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}

fn write_unit_candles_csv(input_path: &str, csv_path: &Path) -> Result<()> {
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

fn write_unit_strategy_file(path: &Path, job: &AutoQuantPdaUnitJob) -> Result<()> {
    let class_name = unit_strategy_class_name(&job.unit_id);
    if job.scope.primitive_sequence.is_empty() {
        return Err(anyhow!(
            "unit '{}' has empty primitive sequence",
            job.unit_id
        ));
    }
    let long_side = job.scope.direction == "long";
    let source = render_strategy_source(
        &class_name,
        &job.scope.primitive_sequence,
        &job.scope.timeframe,
        long_side,
        &job.brief.thesis,
    );
    fs::write(path, source)?;
    Ok(())
}

fn unit_strategy_class_name(raw: &str) -> String {
    let mut out = String::new();
    for part in raw.split('_').filter(|part| !part.is_empty()) {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.push_str(chars.as_str());
        }
    }
    if out.is_empty() {
        "AutoQuantPdaUnit".to_string()
    } else {
        out
    }
}

fn sequence_window_bars(sequence_len: usize) -> usize {
    (sequence_len.saturating_mul(4)).max(6)
}

fn primitive_signal_column_name(primitive: &str) -> String {
    format!("sig_{primitive}")
}

fn primitive_signal_expression(primitive: &str, long_side: bool) -> &'static str {
    match primitive {
        "order_block" => {
            if long_side {
                "(dataframe['prev_bear'] > 0) & (dataframe['displacement_up'] > 0)"
            } else {
                "(dataframe['prev_bull'] > 0) & (dataframe['displacement_down'] > 0)"
            }
        }
        "fair_value_gap" => {
            if long_side {
                "dataframe['fvg_up'] > 0"
            } else {
                "dataframe['fvg_down'] > 0"
            }
        }
        "inverse_fvg" => {
            if long_side {
                "(dataframe['fvg_down'].shift(1).fillna(0) > 0) & (dataframe['close'] > dataframe['high'].shift(1))"
            } else {
                "(dataframe['fvg_up'].shift(1).fillna(0) > 0) & (dataframe['close'] < dataframe['low'].shift(1))"
            }
        }
        "breaker_block" => {
            if long_side {
                "(dataframe['close'] > dataframe['swing_high'].shift(1)) & (dataframe['prev_bear'] > 0)"
            } else {
                "(dataframe['close'] < dataframe['swing_low'].shift(1)) & (dataframe['prev_bull'] > 0)"
            }
        }
        "mitigation_block" => {
            if long_side {
                "(dataframe['low'] <= dataframe['swing_low'].shift(1)) & (dataframe['close'] > dataframe['open'])"
            } else {
                "(dataframe['high'] >= dataframe['swing_high'].shift(1)) & (dataframe['close'] < dataframe['open'])"
            }
        }
        "rejection_block" => {
            if long_side {
                "dataframe['bull_rejection'] > 0"
            } else {
                "dataframe['bear_rejection'] > 0"
            }
        }
        "propulsion_block" => {
            if long_side {
                "(dataframe['propulsion'] > 0) & (dataframe['close'] > dataframe['open'])"
            } else {
                "(dataframe['propulsion'] > 0) & (dataframe['close'] < dataframe['open'])"
            }
        }
        "liquidity_void" => {
            if long_side {
                "dataframe['void_up'] > 0"
            } else {
                "dataframe['void_down'] > 0"
            }
        }
        "volume_imbalance" => {
            if long_side {
                "(dataframe['volume_imbalance'] > 0) & (dataframe['close'] > dataframe['open'])"
            } else {
                "(dataframe['volume_imbalance'] > 0) & (dataframe['close'] < dataframe['open'])"
            }
        }
        "market_structure_shift" => {
            if long_side {
                "dataframe['mss_up'] > 0"
            } else {
                "dataframe['mss_down'] > 0"
            }
        }
        "cisd" => {
            if long_side {
                "dataframe['cisd_up'] > 0"
            } else {
                "dataframe['cisd_down'] > 0"
            }
        }
        _ => {
            if long_side {
                "dataframe['close'] > dataframe['ema20']"
            } else {
                "dataframe['close'] < dataframe['ema20']"
            }
        }
    }
}

fn render_signal_columns(sequence: &[String], long_side: bool) -> String {
    sequence
        .iter()
        .map(|primitive| {
            format!(
                "        dataframe[\"{}\"] = ({}).astype(int)",
                primitive_signal_column_name(primitive),
                primitive_signal_expression(primitive, long_side)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_sequence_progress_columns(sequence: &[String]) -> String {
    let sequence_window = sequence_window_bars(sequence.len());
    let mut lines = Vec::new();
    lines.push(format!(
        "        dataframe[\"seq_step_0\"] = dataframe[\"{}\"]",
        primitive_signal_column_name(&sequence[0])
    ));
    for (index, primitive) in sequence.iter().enumerate().skip(1) {
        lines.push(format!(
            "        dataframe[\"seq_step_{index}\"] = ((dataframe[\"{}\"] > 0) & (dataframe[\"seq_step_{}\"].shift(1).rolling({}).max().fillna(0) > 0)).astype(int)",
            primitive_signal_column_name(primitive),
            index - 1,
            sequence_window
        ));
    }
    lines.join("\n")
}

fn render_entry_condition(sequence: &[String]) -> String {
    format!("dataframe[\"seq_step_{}\"] > 0", sequence.len() - 1)
}

fn render_strategy_source(
    class_name: &str,
    sequence: &[String],
    timeframe: &str,
    long_side: bool,
    thesis: &str,
) -> String {
    let direction = if long_side { "long" } else { "short" };
    let entry_field = if long_side {
        "enter_long"
    } else {
        "enter_short"
    };
    let exit_field = if long_side { "exit_long" } else { "exit_short" };
    let sequence_label = sequence.join(" -> ");
    let signal_columns = render_signal_columns(sequence, long_side);
    let sequence_progress = render_sequence_progress_columns(sequence);
    let entry_condition = render_entry_condition(sequence);
    let exit_condition = if long_side {
        "dataframe['close'] < dataframe['ema20']"
    } else {
        "dataframe['close'] > dataframe['ema20']"
    };
    format!(
        r#""""
{thesis}
Primitive sequence: {sequence_label}
Direction: {direction}
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy
from pandas import DataFrame


class {class_name}(IStrategy):
    INTERFACE_VERSION = 3
    timeframe = "{timeframe}"
    can_short = {can_short}
    process_only_new_candles = True
    use_exit_signal = True
    exit_profit_only = False
    ignore_roi_if_entry_signal = False
    minimal_roi = {{"0": 0.03}}
    stoploss = -0.02
    startup_candle_count = 50

    def populate_indicators(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema20"] = ta.EMA(dataframe, timeperiod=20)
        dataframe["atr14"] = ta.ATR(dataframe, timeperiod=14)
        dataframe["body"] = (dataframe["close"] - dataframe["open"]).abs()
        dataframe["range"] = (dataframe["high"] - dataframe["low"]).clip(lower=1e-9)
        dataframe["prev_bear"] = (dataframe["close"].shift(1) < dataframe["open"].shift(1)).astype(int)
        dataframe["prev_bull"] = (dataframe["close"].shift(1) > dataframe["open"].shift(1)).astype(int)
        dataframe["displacement_up"] = ((dataframe["close"] - dataframe["open"]) > dataframe["atr14"]).astype(int)
        dataframe["displacement_down"] = ((dataframe["open"] - dataframe["close"]) > dataframe["atr14"]).astype(int)
        dataframe["fvg_up"] = (dataframe["low"] > dataframe["high"].shift(2)).astype(int)
        dataframe["fvg_down"] = (dataframe["high"] < dataframe["low"].shift(2)).astype(int)
        dataframe["swing_high"] = dataframe["high"].shift(1).rolling(5).max()
        dataframe["swing_low"] = dataframe["low"].shift(1).rolling(5).min()
        dataframe["upper_wick"] = dataframe["high"] - dataframe[["open", "close"]].max(axis=1)
        dataframe["lower_wick"] = dataframe[["open", "close"]].min(axis=1) - dataframe["low"]
        dataframe["bull_rejection"] = (dataframe["lower_wick"] >= dataframe["body"] * 2.0).astype(int)
        dataframe["bear_rejection"] = (dataframe["upper_wick"] >= dataframe["body"] * 2.0).astype(int)
        dataframe["volume_mean"] = dataframe["volume"].rolling(50).mean()
        dataframe["volume_std"] = dataframe["volume"].rolling(50).std().fillna(0.0)
        dataframe["volume_imbalance"] = (dataframe["volume"] > (dataframe["volume_mean"] + 3.0 * dataframe["volume_std"])).astype(int)
        dataframe["propulsion"] = ((dataframe["body"] / dataframe["range"]) > 0.65).astype(int) & (dataframe["volume"] > (dataframe["volume_mean"] + 2.0 * dataframe["volume_std"]))
        dataframe["void_up"] = ((dataframe["low"] - dataframe["high"].shift(2)) > dataframe["atr14"] * 0.8).astype(int)
        dataframe["void_down"] = ((dataframe["low"].shift(2) - dataframe["high"]) > dataframe["atr14"] * 0.8).astype(int)
        dataframe["mss_up"] = (dataframe["close"] > dataframe["swing_high"]).astype(int)
        dataframe["mss_down"] = (dataframe["close"] < dataframe["swing_low"]).astype(int)
        dataframe["cisd_up"] = ((dataframe["close"] > dataframe["open"]).rolling(3).sum() == 3).astype(int)
        dataframe["cisd_down"] = ((dataframe["close"] < dataframe["open"]).rolling(3).sum() == 3).astype(int)
{signal_columns}
{sequence_progress}
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe.loc[{entry_condition}, "{entry_field}"] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe.loc[{exit_condition}, "{exit_field}"] = 1
        return dataframe
"#,
        thesis = thesis.replace("\"\"\"", "'''"),
        sequence_label = sequence_label,
        direction = direction,
        class_name = class_name,
        timeframe = timeframe,
        can_short = if long_side { "False" } else { "True" },
        signal_columns = signal_columns,
        sequence_progress = sequence_progress,
        entry_condition = entry_condition,
        entry_field = entry_field,
        exit_condition = exit_condition,
        exit_field = exit_field,
    )
}

fn parse_run_tomac_aggregate_metrics(stdout: &str) -> Option<AutoQuantPdaUnitAggregateMetrics> {
    let mut metrics = AutoQuantPdaUnitAggregateMetrics::default();
    let mut seen_any = false;
    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("sharpe:") {
            metrics.sharpe = value.trim().parse().ok()?;
            seen_any = true;
        } else if let Some(value) = trimmed.strip_prefix("sortino:") {
            metrics.sortino = value.trim().parse().ok()?;
        } else if let Some(value) = trimmed.strip_prefix("calmar:") {
            metrics.calmar = value.trim().parse().ok()?;
        } else if let Some(value) = trimmed.strip_prefix("total_profit_pct:") {
            metrics.total_profit_pct = value.trim().parse().ok()?;
        } else if let Some(value) = trimmed.strip_prefix("max_drawdown_pct:") {
            metrics.max_drawdown_pct = value.trim().parse().ok()?;
        } else if let Some(value) = trimmed.strip_prefix("trade_count:") {
            metrics.trade_count = value.trim().parse().ok()?;
        } else if let Some(value) = trimmed.strip_prefix("win_rate_pct:") {
            metrics.win_rate_pct = value.trim().parse().ok()?;
        } else if let Some(value) = trimmed.strip_prefix("profit_factor:") {
            metrics.profit_factor = value.trim().parse().ok()?;
        }
    }
    seen_any.then_some(metrics)
}

fn summarize_dispatch_totals(
    groups: &[AutoQuantPdaDispatchGroupResult],
) -> AutoQuantPdaDispatchTotals {
    let mut totals = AutoQuantPdaDispatchTotals::default();
    for group in groups {
        for result in &group.unit_results {
            totals.total_units += 1;
            match result.status.as_str() {
                "completed" => totals.completed_units += 1,
                "blocked" => totals.blocked_units += 1,
                _ => totals.failed_units += 1,
            }
        }
    }
    totals
}

fn persist_dispatch_artifact(
    state_dir: &str,
    artifact: &AutoQuantPdaUnitDispatchArtifact,
) -> Result<()> {
    let filename = format!(
        "auto_quant_pda_unit_dispatch.{}.json",
        artifact.generated_at.format("%Y%m%dT%H%M%S%.3fZ")
    );
    save_state(state_dir, &artifact.symbol, &filename, artifact)?;
    let path = artifact_state_path(state_dir, &artifact.symbol, &filename);
    append_artifact_ledger_entry(
        state_dir,
        &artifact.symbol,
        ArtifactLedgerEntry {
            entry_id: format!("ledger:{}", artifact.artifact_id),
            artifact_kind: "auto_quant_pda_unit_dispatch".to_string(),
            artifact_id: artifact.artifact_id.clone(),
            version: 1,
            generated_at: artifact.generated_at,
            symbol: artifact.symbol.clone(),
            source_phase: "auto_quant_pda_unit_dispatch".to_string(),
            source_run_id: Some(artifact.batch_artifact_id.clone()),
            path,
            status: "dispatch_completed".to_string(),
            promote_candidate: false,
            actionable: true,
            decision_hint: "auto_quant_unit_results_ready".to_string(),
            review_reason: format!(
                "completed={} blocked={} failed={}",
                artifact.totals.completed_units,
                artifact.totals.blocked_units,
                artifact.totals.failed_units
            ),
            review_rule_version: AUTO_QUANT_PDA_UNIT_DISPATCH_RULE_VERSION.to_string(),
            top_factor_name: None,
            top_factor_action: Some("review".to_string()),
            family_scores: std::collections::BTreeMap::new(),
            supersedes_artifact_id: None,
            quality_score: artifact.totals.completed_units.min(i32::MAX as usize) as i32,
            consumed_by_update_run_id: None,
            consumed_at: None,
            consumed_outcome: None,
            regraded_at: None,
            consumption_regrade_status: None,
            consumption_regrade_reason: None,
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn blocking_reason_for_profile_flags_external_surfaces() {
        let profile = AutoQuantConsumerEvidenceProfile {
            required_surfaces: vec!["greeks".to_string(), "volatility".to_string()],
            ..Default::default()
        };
        assert!(blocking_reason_for_profile(&profile)
            .unwrap()
            .contains("missing_external_evidence_provider"));
    }

    #[test]
    fn parse_run_tomac_aggregate_metrics_extracts_summary_block() {
        let stdout = r#"
---
strategy:         Demo
sharpe:           1.2500
sortino:          1.8000
calmar:           0.9000
total_profit_pct: 12.3400
max_drawdown_pct: -3.2100
trade_count:      42
win_rate_pct:     57.1400
profit_factor:    1.8700
"#;
        let metrics = parse_run_tomac_aggregate_metrics(stdout).unwrap();
        assert_eq!(metrics.trade_count, 42);
        assert_eq!(metrics.win_rate_pct, 57.14);
        assert_eq!(metrics.sharpe, 1.25);
    }

    #[test]
    fn load_target_batch_reads_latest_batch_artifact() {
        let dir = TempDir::new().unwrap();
        let artifact = AutoQuantPdaUnitBatchArtifact {
            artifact_id: "batch-1".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            objective: "expansion_manipulation".to_string(),
            combination_size: 1,
            max_parallel: 1,
            shared_workspace_root: "/tmp/shared".to_string(),
            selected_timeframes: vec!["15m".to_string()],
            selected_primitives: vec!["order_block".to_string()],
            consumer_evidence_profile: AutoQuantConsumerEvidenceProfile::default(),
            unit_jobs: Vec::new(),
            dispatch_groups: Vec::new(),
            notes: Vec::new(),
        };
        save_state(
            dir.path(),
            "NQ",
            "auto_quant_pda_unit_batch.test.json",
            &artifact,
        )
        .unwrap();
        let path = artifact_state_path(dir.path(), "NQ", "auto_quant_pda_unit_batch.test.json");
        append_artifact_ledger_entry(
            dir.path(),
            "NQ",
            ArtifactLedgerEntry {
                entry_id: "ledger:batch-1".to_string(),
                artifact_kind: "auto_quant_pda_unit_batch".to_string(),
                artifact_id: "batch-1".to_string(),
                version: 1,
                generated_at: artifact.generated_at,
                symbol: "NQ".to_string(),
                source_phase: "auto_quant_pda_unit_batch".to_string(),
                source_run_id: None,
                path,
                status: "batch_ready_for_dispatch".to_string(),
                promote_candidate: false,
                actionable: true,
                decision_hint: String::new(),
                review_reason: String::new(),
                review_rule_version: String::new(),
                top_factor_name: None,
                top_factor_action: None,
                family_scores: std::collections::BTreeMap::new(),
                supersedes_artifact_id: None,
                quality_score: 0,
                consumed_by_update_run_id: None,
                consumed_at: None,
                consumed_outcome: None,
                regraded_at: None,
                consumption_regrade_status: None,
                consumption_regrade_reason: None,
            },
        )
        .unwrap();

        let loaded = load_target_batch(dir.path().to_str().unwrap(), "NQ", None).unwrap();
        assert_eq!(loaded.artifact_id, "batch-1");
    }

    #[test]
    fn summarize_dispatch_totals_counts_status_buckets() {
        let groups = vec![AutoQuantPdaDispatchGroupResult {
            group_index: 0,
            unit_results: vec![
                AutoQuantPdaUnitResult {
                    status: "completed".to_string(),
                    ..Default::default()
                },
                AutoQuantPdaUnitResult {
                    status: "blocked".to_string(),
                    ..Default::default()
                },
                AutoQuantPdaUnitResult {
                    status: "failed".to_string(),
                    ..Default::default()
                },
            ],
        }];
        let totals = summarize_dispatch_totals(&groups);
        assert_eq!(totals.total_units, 3);
        assert_eq!(totals.completed_units, 1);
        assert_eq!(totals.blocked_units, 1);
        assert_eq!(totals.failed_units, 1);
    }

    #[test]
    fn parse_group_indices_defaults_to_all_groups() {
        let indices = parse_group_indices(None, 4).unwrap();
        assert_eq!(indices, vec![0, 1, 2, 3]);
    }

    #[test]
    fn unit_strategy_class_name_normalizes_identifier() {
        assert_eq!(
            unit_strategy_class_name("NQ__15m__long__order_block"),
            "NQ15mLongOrderBlock"
        );
    }

    #[test]
    fn render_strategy_source_includes_expected_entry_side() {
        let source = render_strategy_source(
            "NQ15mLongOrderBlock",
            &["order_block".to_string()],
            "15m",
            true,
            "Demo thesis",
        );
        assert!(source.contains("enter_long"));
        assert!(source.contains("Primitive sequence: order_block"));
        assert!(source.contains("dataframe[\"seq_step_0\"] = dataframe[\"sig_order_block\"]"));
    }

    #[test]
    fn render_strategy_source_preserves_multi_step_sequence_logic() {
        let source = render_strategy_source(
            "NQ5mLongMssFvg",
            &[
                "market_structure_shift".to_string(),
                "fair_value_gap".to_string(),
            ],
            "5m",
            true,
            "Demo thesis",
        );
        assert!(source.contains("Primitive sequence: market_structure_shift -> fair_value_gap"));
        assert!(source.contains("sig_market_structure_shift"));
        assert!(source.contains("sig_fair_value_gap"));
        assert!(source.contains("seq_step_1"));
        assert!(source.contains("rolling(8)"));
        assert!(source.contains("dataframe.loc[dataframe[\"seq_step_1\"] > 0, \"enter_long\"] = 1"));
    }
}
