//! Parser for `run_ibkr.log` (Auto-Quant's per-strategy --- block log).
//!
//! Used as a redundant cross-check during `auto-quant-results-import`:
//! the manifest produced by `export_strategy_library.py` is the
//! canonical source of metrics, but this parser lets us verify that
//! the manifest matches what was actually emitted by the most recent
//! `run_ibkr.py` invocation. It mirrors the producer-side parser in
//! `Auto-Quant/export_strategy_library.py:parse_log` so block format
//! drift on either side is loud, not silent.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::manifest::{StrategyLibraryEntry, StrategyLibraryValidationMetrics};

/// One `---` block lifted from a `run_ibkr.log`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunIbkrLogBlock {
    pub strategy: String,
    pub commit: String,
    pub config: String,
    pub timerange: String,
    pub pairs: Vec<String>,
    pub auto_quant_meta: Option<serde_json::Value>,
    /// "ok" | "error" — derived from a `status: ERROR` line in the block.
    pub status: String,
    pub error_type: String,
    pub error_msg: String,
    pub aggregate: StrategyLibraryValidationMetrics,
    pub per_pair: BTreeMap<String, StrategyLibraryValidationMetrics>,
}

/// Parse the entire log into a list of `RunIbkrLogBlock`s, in source order.
pub fn parse_run_ibkr_log<P: AsRef<Path>>(path: P) -> Result<Vec<RunIbkrLogBlock>> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read run_ibkr log '{}'", path.display()))?;
    Ok(parse_run_ibkr_log_text(&text))
}

/// In-memory variant; useful for unit tests.
pub fn parse_run_ibkr_log_text(text: &str) -> Vec<RunIbkrLogBlock> {
    let mut blocks: Vec<RunIbkrLogBlock> = Vec::new();
    let mut current: Option<RunIbkrLogBlock> = None;
    let mut in_per_pair = false;

    for raw in text.lines() {
        if raw.trim() == "---" {
            if let Some(block) = current.take() {
                blocks.push(block);
            }
            current = Some(RunIbkrLogBlock {
                status: "ok".to_string(),
                ..Default::default()
            });
            in_per_pair = false;
            continue;
        }
        let Some(block) = current.as_mut() else {
            continue;
        };

        if raw.starts_with("per_pair:") {
            in_per_pair = true;
            continue;
        }

        if in_per_pair {
            if let Some((pair, metrics)) = parse_per_pair_line(raw) {
                block.per_pair.insert(pair, metrics);
                continue;
            }
            // Non-matching line ends the per_pair section.
            in_per_pair = false;
        }

        if let Some((key, value)) = parse_kv(raw) {
            apply_kv(block, &key, value.trim());
        }
    }
    if let Some(block) = current.take() {
        blocks.push(block);
    }
    blocks
}

fn parse_kv(line: &str) -> Option<(String, &str)> {
    let mut chars = line.chars();
    let mut key = String::new();
    while let Some(ch) = chars.clone().next() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            key.push(ch);
            chars.next();
        } else {
            break;
        }
    }
    if key.is_empty() {
        return None;
    }
    let rest = chars.as_str();
    rest.strip_prefix(':').map(|tail| (key, tail))
}

fn apply_kv(block: &mut RunIbkrLogBlock, key: &str, value: &str) {
    match key {
        "strategy" => block.strategy = value.to_string(),
        "commit" => block.commit = value.to_string(),
        "config" => block.config = value.to_string(),
        "timerange" => block.timerange = value.to_string(),
        "pairs" => {
            block.pairs = value
                .split(',')
                .filter(|p| !p.is_empty())
                .map(|p| p.trim().to_string())
                .collect();
        }
        "auto_quant_meta" => {
            block.auto_quant_meta = serde_json::from_str(value).ok();
        }
        "status" => {
            block.status = if value.eq_ignore_ascii_case("ERROR") {
                "error".to_string()
            } else {
                "ok".to_string()
            };
        }
        "error_type" => block.error_type = value.to_string(),
        "error_msg" => block.error_msg = value.to_string(),
        "sharpe" => {
            if let Ok(v) = value.parse() {
                block.aggregate.sharpe = v;
            }
        }
        "sortino" => {
            if let Ok(v) = value.parse() {
                block.aggregate.sortino = v;
            }
        }
        "calmar" => {
            if let Ok(v) = value.parse() {
                block.aggregate.calmar = v;
            }
        }
        "total_profit_pct" => {
            if let Ok(v) = value.parse() {
                block.aggregate.total_profit_pct = v;
            }
        }
        "max_drawdown_pct" => {
            if let Ok(v) = value.parse() {
                block.aggregate.max_drawdown_pct = v;
            }
        }
        "trade_count" => {
            if let Ok(v) = value.parse() {
                block.aggregate.trade_count = v;
            }
        }
        "win_rate_pct" => {
            if let Ok(v) = value.parse() {
                block.aggregate.win_rate_pct = v;
            }
        }
        "profit_factor" => {
            if let Ok(v) = value.parse() {
                block.aggregate.profit_factor = v;
            }
        }
        _ => {}
    }
}

/// Parse a per_pair indented line of the form
/// `  PAIR: sharpe=… trades=… profit_pct=… dd_pct=… wr=… pf=…`
fn parse_per_pair_line(raw: &str) -> Option<(String, StrategyLibraryValidationMetrics)> {
    let trimmed = raw.trim_start();
    let (pair, rest) = trimmed.split_once(':')?;
    let pair = pair.trim().to_string();
    if pair.is_empty() {
        return None;
    }
    let mut metrics = StrategyLibraryValidationMetrics::default();
    let mut found_any = false;
    for token in rest.split_whitespace() {
        let Some((field, value)) = token.split_once('=') else {
            continue;
        };
        let parsed = value.parse::<f64>().ok();
        let parsed_int = value.parse::<u32>().ok();
        match (field, parsed, parsed_int) {
            ("sharpe", Some(v), _) => {
                metrics.sharpe = v;
                found_any = true;
            }
            ("trades", _, Some(v)) => {
                metrics.trade_count = v;
                found_any = true;
            }
            ("profit_pct", Some(v), _) => {
                metrics.total_profit_pct = v;
                found_any = true;
            }
            ("dd_pct", Some(v), _) => {
                metrics.max_drawdown_pct = v;
                found_any = true;
            }
            ("wr", Some(v), _) => {
                metrics.win_rate_pct = v;
                found_any = true;
            }
            ("pf", Some(v), _) => {
                metrics.profit_factor = v;
                found_any = true;
            }
            _ => {}
        }
    }
    if found_any {
        Some((pair, metrics))
    } else {
        None
    }
}

/// Single drift point between the manifest produced by
/// `export_strategy_library.py` and the canonical `run_ibkr.log`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestLogMismatch {
    pub strategy: String,
    pub field: String,
    pub manifest_value: serde_json::Value,
    pub log_value: serde_json::Value,
}

/// Aggregate cross-check report. Informational; never causes import to fail.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ManifestLogCrossCheck {
    pub n_blocks_in_log: usize,
    pub n_strategies_in_manifest: usize,
    pub matched: usize,
    pub mismatches: Vec<ManifestLogMismatch>,
    /// Names present in the manifest but absent from the log (likely
    /// the manifest was rebuilt from an older log or includes legacy
    /// strategies that were not re-run).
    pub manifest_only: Vec<String>,
    /// Names present in the log but absent from the manifest (likely
    /// the export step skipped a block — e.g. the source `.py` file
    /// is missing or its docstring is malformed).
    pub log_only: Vec<String>,
}

impl ManifestLogCrossCheck {
    /// `true` iff every manifest strategy lined up with a log block
    /// on status, trade_count, and win_rate_pct.
    pub fn is_clean(&self) -> bool {
        self.mismatches.is_empty() && self.manifest_only.is_empty() && self.log_only.is_empty()
    }
}

const METRIC_F64_TOLERANCE: f64 = 1.0e-6;

fn block_has_cross_check_metrics(block: &RunIbkrLogBlock) -> bool {
    let metrics = &block.aggregate;
    metrics.trade_count > 0
        || metrics.win_rate_pct.abs() > METRIC_F64_TOLERANCE
        || metrics.sharpe.abs() > METRIC_F64_TOLERANCE
        || metrics.sortino.abs() > METRIC_F64_TOLERANCE
        || metrics.calmar.abs() > METRIC_F64_TOLERANCE
        || metrics.profit_factor.abs() > METRIC_F64_TOLERANCE
        || metrics.max_drawdown_pct.abs() > METRIC_F64_TOLERANCE
        || metrics.total_profit_pct.abs() > METRIC_F64_TOLERANCE
}

fn metric_selection_score(
    manifest: &StrategyLibraryValidationMetrics,
    block: &RunIbkrLogBlock,
) -> f64 {
    let mut score = 0.0;
    if manifest.trade_count != block.aggregate.trade_count {
        score += 10_000.0
            + (i64::from(manifest.trade_count) - i64::from(block.aggregate.trade_count)).abs()
                as f64;
    }
    for (manifest_value, log_value) in [
        (manifest.win_rate_pct, block.aggregate.win_rate_pct),
        (manifest.sharpe, block.aggregate.sharpe),
        (manifest.sortino, block.aggregate.sortino),
        (manifest.calmar, block.aggregate.calmar),
        (manifest.profit_factor, block.aggregate.profit_factor),
        (manifest.max_drawdown_pct, block.aggregate.max_drawdown_pct),
        (manifest.total_profit_pct, block.aggregate.total_profit_pct),
    ] {
        let delta = (manifest_value - log_value).abs();
        if delta > METRIC_F64_TOLERANCE {
            score += 1_000.0 + delta;
        }
    }
    score
}

fn log_block_selection_score(entry: &StrategyLibraryEntry, block: &RunIbkrLogBlock) -> f64 {
    let mut score = 0.0;
    if !entry.status.is_empty() && entry.status != block.status {
        score += 1_000_000.0;
    }
    if !entry.timerange.is_empty() && entry.timerange != block.timerange {
        score += if block.timerange.is_empty() {
            50_000.0
        } else {
            100_000.0
        };
    }
    if let Some(metrics) = &entry.validation_metrics {
        score += metric_selection_score(metrics, block);
    } else if !block_has_cross_check_metrics(block) {
        score += 1_000.0;
    }
    score
}

fn select_log_block_for_entry<'a>(
    entry: &StrategyLibraryEntry,
    candidates: &[&'a RunIbkrLogBlock],
) -> Option<&'a RunIbkrLogBlock> {
    candidates.iter().copied().min_by(|a, b| {
        log_block_selection_score(entry, a).total_cmp(&log_block_selection_score(entry, b))
    })
}

/// Cross-check a `StrategyLibraryManifest` against a list of log blocks
/// parsed from `run_ibkr.log`. Compares strategy presence, status, and
/// the headline numeric metrics that drive prior init
/// (`trade_count`, `win_rate_pct`). Other metrics (sharpe, sortino,
/// calmar, profit_factor, drawdown) are checked too but only when both
/// sides report a non-default value, since the producer may emit
/// zeros/defaults for an errored strategy.
pub fn cross_check_manifest_against_log(
    manifest: &super::manifest::StrategyLibraryManifest,
    blocks: &[RunIbkrLogBlock],
) -> ManifestLogCrossCheck {
    use serde_json::json;

    let mut report = ManifestLogCrossCheck {
        n_blocks_in_log: blocks.len(),
        n_strategies_in_manifest: manifest.strategies.len(),
        ..ManifestLogCrossCheck::default()
    };

    let mut log_index: BTreeMap<&str, Vec<&RunIbkrLogBlock>> = BTreeMap::new();
    for block in blocks {
        log_index
            .entry(block.strategy.as_str())
            .or_default()
            .push(block);
    }
    let mut manifest_names: BTreeMap<&str, ()> = BTreeMap::new();

    for entry in &manifest.strategies {
        manifest_names.insert(entry.name.as_str(), ());
        let Some(candidates) = log_index.get(entry.name.as_str()) else {
            report.manifest_only.push(entry.name.clone());
            continue;
        };
        let Some(block) = select_log_block_for_entry(entry, candidates) else {
            report.manifest_only.push(entry.name.clone());
            continue;
        };
        let mut local_mismatches = 0usize;

        if entry.status != block.status {
            report.mismatches.push(ManifestLogMismatch {
                strategy: entry.name.clone(),
                field: "status".to_string(),
                manifest_value: json!(entry.status),
                log_value: json!(block.status),
            });
            local_mismatches += 1;
        }

        if let Some(metrics) = &entry.validation_metrics {
            if metrics.trade_count != block.aggregate.trade_count {
                report.mismatches.push(ManifestLogMismatch {
                    strategy: entry.name.clone(),
                    field: "trade_count".to_string(),
                    manifest_value: json!(metrics.trade_count),
                    log_value: json!(block.aggregate.trade_count),
                });
                local_mismatches += 1;
            }
            if (metrics.win_rate_pct - block.aggregate.win_rate_pct).abs() > METRIC_F64_TOLERANCE {
                report.mismatches.push(ManifestLogMismatch {
                    strategy: entry.name.clone(),
                    field: "win_rate_pct".to_string(),
                    manifest_value: json!(metrics.win_rate_pct),
                    log_value: json!(block.aggregate.win_rate_pct),
                });
                local_mismatches += 1;
            }
            // Soft-fail metrics: only flag when both sides reported a
            // non-zero number, otherwise the comparison is meaningless
            // (default zeros from the export side, or errored block).
            for (label, m_val, l_val) in [
                ("sharpe", metrics.sharpe, block.aggregate.sharpe),
                ("sortino", metrics.sortino, block.aggregate.sortino),
                ("calmar", metrics.calmar, block.aggregate.calmar),
                (
                    "profit_factor",
                    metrics.profit_factor,
                    block.aggregate.profit_factor,
                ),
                (
                    "max_drawdown_pct",
                    metrics.max_drawdown_pct,
                    block.aggregate.max_drawdown_pct,
                ),
                (
                    "total_profit_pct",
                    metrics.total_profit_pct,
                    block.aggregate.total_profit_pct,
                ),
            ] {
                if m_val != 0.0 && l_val != 0.0 && (m_val - l_val).abs() > METRIC_F64_TOLERANCE {
                    report.mismatches.push(ManifestLogMismatch {
                        strategy: entry.name.clone(),
                        field: label.to_string(),
                        manifest_value: json!(m_val),
                        log_value: json!(l_val),
                    });
                    local_mismatches += 1;
                }
            }
        }

        if local_mismatches == 0 {
            report.matched += 1;
        }
    }

    for block in blocks {
        if !manifest_names.contains_key(block.strategy.as_str()) {
            report.log_only.push(block.strategy.clone());
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::auto_quant::results::manifest::{
        StrategyLibraryEntry, StrategyLibraryManifest, StrategyLibraryMetadata,
        StrategyLibraryValidationMetrics,
    };

    const SAMPLE_LOG: &str = "preamble line\n\
---\n\
strategy:         GoodStrat\n\
commit:           abc1234\n\
config:           config.ibkr.json\n\
timerange:        20240101-20240201\n\
pairs:            SPY/USD,QQQ/USD\n\
auto_quant_meta:  {\"strategy\":\"GoodStrat\",\"mutation_id\":\"mb-001\",\"base_factor\":\"f\",\"hypothesis\":\"h\",\"paradigm\":\"p\",\"expected_regime\":\"r\",\"factors_used\":[\"bos\"],\"parent\":\"root\",\"asset_class\":\"equities\",\"status\":\"active\",\"created\":\"\"}\n\
sharpe:           1.4200\n\
sortino:          2.1300\n\
calmar:           4.5000\n\
total_profit_pct: 12.3000\n\
max_drawdown_pct: -3.2000\n\
trade_count:      87\n\
win_rate_pct:     54.5000\n\
profit_factor:    1.8500\n\
per_pair:\n\
  SPY/USD: sharpe=1.5000 trades=50 profit_pct=15.00 dd_pct=-2.50 wr=58.0 pf=2.10\n\
  QQQ/USD: sharpe=1.3000 trades=37 profit_pct=8.50 dd_pct=-3.80 wr=49.0 pf=1.55\n\
\n\
---\n\
strategy:         BrokenStrat\n\
commit:           abc1234\n\
config:           config.ibkr.json\n\
auto_quant_meta:  {\"strategy\":\"BrokenStrat\",\"mutation_id\":\"mb-002\",\"base_factor\":\"f\",\"hypothesis\":\"h\",\"paradigm\":\"p\",\"expected_regime\":\"r\",\"factors_used\":[\"x\"],\"parent\":\"root\",\"asset_class\":\"equities\",\"status\":\"active\",\"created\":\"\"}\n\
status:           ERROR\n\
error_type:       ValueError\n\
error_msg:        boom\n\
traceback:\n\
  File \"x\", line 1\n";

    const DUPLICATE_SUMMARY_LOG: &str = "---\n\
strategy:         GoodStrat\n\
timerange_label:  full\n\
commit:           abc1234\n\
timerange:        20240101-20240201\n\
sharpe:           1.4200\n\
sortino:          2.1300\n\
calmar:           4.5000\n\
total_profit_pct: 12.3000\n\
max_drawdown_pct: -3.2000\n\
trade_count:      87\n\
win_rate_pct:     54.5000\n\
profit_factor:    1.8500\n\
\n\
---\n\
strategy:         GoodStrat\n\
timerange_label:  SUMMARY\n\
commit:           abc1234\n\
robust_sharpe:    1.4200   # min across declared timeranges\n";

    const DUPLICATE_TIMERANGE_LOG: &str = "---\n\
strategy:         RegimeAdaptiveBNB\n\
timerange_label:  bull_2021\n\
timerange:        20210101-20211231\n\
sharpe:           0.3226\n\
sortino:          0.3771\n\
calmar:           10.6619\n\
total_profit_pct: 8.1100\n\
max_drawdown_pct: -4.1163\n\
trade_count:      16\n\
win_rate_pct:     81.2500\n\
profit_factor:    2.4178\n\
\n\
---\n\
strategy:         RegimeAdaptiveBNB\n\
timerange_label:  full_5y\n\
timerange:        20210101-20251231\n\
sharpe:           0.1380\n\
sortino:          0.1543\n\
calmar:           3.6967\n\
total_profit_pct: 16.4100\n\
max_drawdown_pct: -4.6742\n\
trade_count:      115\n\
win_rate_pct:     69.5652\n\
profit_factor:    1.4262\n\
\n\
---\n\
strategy:         RegimeAdaptiveBNB\n\
timerange_label:  SUMMARY\n\
robust_sharpe:    0.0967   # min across declared timeranges\n";

    #[test]
    fn parses_two_blocks_with_status_split() {
        let blocks = parse_run_ibkr_log_text(SAMPLE_LOG);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].status, "ok");
        assert_eq!(blocks[1].status, "error");
    }

    #[test]
    fn extracts_aggregate_metrics() {
        let blocks = parse_run_ibkr_log_text(SAMPLE_LOG);
        let good = &blocks[0];
        assert_eq!(good.strategy, "GoodStrat");
        assert!((good.aggregate.sharpe - 1.42).abs() < 1e-9);
        assert_eq!(good.aggregate.trade_count, 87);
        assert!((good.aggregate.win_rate_pct - 54.5).abs() < 1e-9);
    }

    #[test]
    fn extracts_per_pair_metrics() {
        let blocks = parse_run_ibkr_log_text(SAMPLE_LOG);
        let good = &blocks[0];
        assert_eq!(good.per_pair.len(), 2);
        let spy = good.per_pair.get("SPY/USD").unwrap();
        assert_eq!(spy.trade_count, 50);
        assert!((spy.win_rate_pct - 58.0).abs() < 1e-9);
    }

    #[test]
    fn captures_error_type_and_msg() {
        let blocks = parse_run_ibkr_log_text(SAMPLE_LOG);
        let bad = &blocks[1];
        assert_eq!(bad.error_type, "ValueError");
        assert_eq!(bad.error_msg, "boom");
    }

    #[test]
    fn captures_auto_quant_meta_json() {
        let blocks = parse_run_ibkr_log_text(SAMPLE_LOG);
        let good = &blocks[0];
        let meta = good.auto_quant_meta.as_ref().unwrap();
        assert_eq!(
            meta.get("mutation_id").and_then(|v| v.as_str()),
            Some("mb-001")
        );
    }

    fn manifest_entry_mirroring_good_block() -> StrategyLibraryEntry {
        StrategyLibraryEntry {
            name: "GoodStrat".to_string(),
            file_path: "user_data/strategies_ibkr/GoodStrat.py".to_string(),
            metadata: StrategyLibraryMetadata {
                strategy: "GoodStrat".to_string(),
                mutation_id: "mb-001".to_string(),
                ..Default::default()
            },
            status: "ok".to_string(),
            validation_metrics: Some(StrategyLibraryValidationMetrics {
                sharpe: 1.42,
                sortino: 2.13,
                calmar: 4.5,
                total_profit_pct: 12.3,
                max_drawdown_pct: -3.2,
                trade_count: 87,
                win_rate_pct: 54.5,
                profit_factor: 1.85,
            }),
            ..Default::default()
        }
    }

    fn manifest_entry_mirroring_regime_full_block() -> StrategyLibraryEntry {
        StrategyLibraryEntry {
            name: "RegimeAdaptiveBNB".to_string(),
            file_path: "user_data/strategies_ibkr/RegimeAdaptiveBNB.py".to_string(),
            metadata: StrategyLibraryMetadata {
                strategy: "RegimeAdaptiveBNB".to_string(),
                mutation_id: "real-auto-quant-RegimeAdaptiveBNB".to_string(),
                ..Default::default()
            },
            status: "ok".to_string(),
            validation_metrics: Some(StrategyLibraryValidationMetrics {
                sharpe: 0.138,
                sortino: 0.1543,
                calmar: 3.6967,
                total_profit_pct: 16.41,
                max_drawdown_pct: -4.6742,
                trade_count: 115,
                win_rate_pct: 69.5652,
                profit_factor: 1.4262,
            }),
            timerange: "20210101-20251231".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn cross_check_is_clean_when_manifest_mirrors_log() {
        let blocks = parse_run_ibkr_log_text(SAMPLE_LOG);
        let manifest = StrategyLibraryManifest {
            manifest_version: "1.0".to_string(),
            strategies: vec![
                manifest_entry_mirroring_good_block(),
                StrategyLibraryEntry {
                    name: "BrokenStrat".to_string(),
                    status: "error".to_string(),
                    metadata: StrategyLibraryMetadata {
                        strategy: "BrokenStrat".to_string(),
                        mutation_id: "mb-002".to_string(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let report = cross_check_manifest_against_log(&manifest, &blocks);
        assert!(report.is_clean(), "{:?}", report);
        assert_eq!(report.matched, 2);
    }

    #[test]
    fn cross_check_flags_numeric_drift() {
        let blocks = parse_run_ibkr_log_text(SAMPLE_LOG);
        let mut entry = manifest_entry_mirroring_good_block();
        if let Some(m) = entry.validation_metrics.as_mut() {
            m.win_rate_pct = 99.0; // log says 54.5
            m.trade_count = 12; // log says 87
        }
        let manifest = StrategyLibraryManifest {
            manifest_version: "1.0".to_string(),
            strategies: vec![entry],
            ..Default::default()
        };
        let report = cross_check_manifest_against_log(&manifest, &blocks);
        assert!(!report.is_clean());
        let fields: Vec<&str> = report.mismatches.iter().map(|m| m.field.as_str()).collect();
        assert!(fields.contains(&"trade_count"));
        assert!(fields.contains(&"win_rate_pct"));
        // BrokenStrat is in the log but absent from the manifest: log_only.
        assert_eq!(report.log_only, vec!["BrokenStrat".to_string()]);
        assert_eq!(report.matched, 0);
    }

    #[test]
    fn cross_check_prefers_metric_block_when_summary_reuses_strategy_name() {
        let blocks = parse_run_ibkr_log_text(DUPLICATE_SUMMARY_LOG);
        let manifest = StrategyLibraryManifest {
            manifest_version: "1.0".to_string(),
            strategies: vec![manifest_entry_mirroring_good_block()],
            ..Default::default()
        };

        let report = cross_check_manifest_against_log(&manifest, &blocks);

        assert!(report.is_clean(), "{:?}", report);
        assert_eq!(report.matched, 1);
    }

    #[test]
    fn cross_check_prefers_manifest_matching_timerange_block() {
        let blocks = parse_run_ibkr_log_text(DUPLICATE_TIMERANGE_LOG);
        let manifest = StrategyLibraryManifest {
            manifest_version: "1.0".to_string(),
            strategies: vec![manifest_entry_mirroring_regime_full_block()],
            ..Default::default()
        };

        let report = cross_check_manifest_against_log(&manifest, &blocks);

        assert!(report.is_clean(), "{:?}", report);
        assert_eq!(report.matched, 1);
    }

    #[test]
    fn cross_check_reports_asymmetric_coverage() {
        let blocks = parse_run_ibkr_log_text(SAMPLE_LOG);
        let manifest = StrategyLibraryManifest {
            manifest_version: "1.0".to_string(),
            strategies: vec![StrategyLibraryEntry {
                name: "GhostStrat".to_string(),
                status: "ok".to_string(),
                metadata: StrategyLibraryMetadata {
                    strategy: "GhostStrat".to_string(),
                    mutation_id: "g-001".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            }],
            ..Default::default()
        };
        let report = cross_check_manifest_against_log(&manifest, &blocks);
        assert!(!report.is_clean());
        assert_eq!(report.manifest_only, vec!["GhostStrat".to_string()]);
        assert!(report.log_only.contains(&"GoodStrat".to_string()));
        assert!(report.log_only.contains(&"BrokenStrat".to_string()));
    }
}
