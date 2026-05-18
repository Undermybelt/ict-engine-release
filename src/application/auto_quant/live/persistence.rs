//! State-directory persistence for the live-signals consumer.
//!
//! Two files per `<state_dir>/<symbol>`:
//!
//! * `auto_quant_live_factor_contributions.jsonl` — append-only log,
//!   one JSON object per `LiveFactorContribution`, with envelope
//!   metadata flattened in alongside so each line is self-contained.
//! * `auto_quant_live_signals_cursor.json` — last applied stream id
//!   so the consumer can resume after a restart without re-applying
//!   the backlog.
//!
//! The cursor write is atomic (write-to-tmp + fsync + rename) so a
//! crash never leaves a partially-written cursor on disk.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use super::wire::LiveFactorSignalEnvelope;

const JSONL_FILENAME: &str = "auto_quant_live_factor_contributions.jsonl";
const CURSOR_FILENAME: &str = "auto_quant_live_signals_cursor.json";

/// Resolve the JSONL path inside `<state_dir>/<symbol>/`.
pub fn jsonl_path(state_dir: impl AsRef<Path>, symbol: &str) -> PathBuf {
    state_dir.as_ref().join(symbol).join(JSONL_FILENAME)
}

/// Resolve the cursor path inside `<state_dir>/<symbol>/`.
pub fn cursor_path(state_dir: impl AsRef<Path>, symbol: &str) -> PathBuf {
    state_dir.as_ref().join(symbol).join(CURSOR_FILENAME)
}

/// Persisted shape of the resume cursor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LiveSignalsCursor {
    pub stream_key: String,
    pub last_id: String,
    pub updated_at: String,
}

/// Read the cursor file. Returns `Ok(None)` when the file is absent
/// (a fresh consumer with no prior state).
pub fn read_cursor(state_dir: impl AsRef<Path>, symbol: &str) -> Result<Option<LiveSignalsCursor>> {
    let path = cursor_path(state_dir, symbol);
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| anyhow!("failed to read cursor at {}: {e}", path.display()))?;
    let parsed: LiveSignalsCursor = serde_json::from_str(&raw)
        .map_err(|e| anyhow!("failed to parse cursor at {}: {e}", path.display()))?;
    Ok(Some(parsed))
}

/// Atomic-replace the cursor file. Always creates the parent dir.
pub fn write_cursor(
    state_dir: impl AsRef<Path>,
    symbol: &str,
    cursor: &LiveSignalsCursor,
) -> Result<()> {
    let path = cursor_path(state_dir.as_ref(), symbol);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| anyhow!("failed to create state dir {}: {e}", parent.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    let body = serde_json::to_string_pretty(cursor)
        .map_err(|e| anyhow!("failed to serialise cursor: {e}"))?;
    {
        let mut f = File::create(&tmp)
            .map_err(|e| anyhow!("failed to open tmp cursor at {}: {e}", tmp.display()))?;
        f.write_all(body.as_bytes())
            .map_err(|e| anyhow!("failed to write tmp cursor: {e}"))?;
        f.sync_all()
            .map_err(|e| anyhow!("failed to fsync tmp cursor: {e}"))?;
    }
    std::fs::rename(&tmp, &path)
        .map_err(|e| anyhow!("failed to rename tmp cursor to {}: {e}", path.display()))?;
    Ok(())
}

/// Append one JSON line per contribution to the JSONL log.
///
/// Each line carries envelope metadata alongside the contribution so
/// downstream readers do not need a join.
pub fn append_envelope(
    state_dir: impl AsRef<Path>,
    symbol: &str,
    stream_id: &str,
    envelope: &LiveFactorSignalEnvelope,
) -> Result<usize> {
    let path = jsonl_path(state_dir.as_ref(), symbol);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| anyhow!("failed to create state dir {}: {e}", parent.display()))?;
    }
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| anyhow!("failed to open jsonl at {}: {e}", path.display()))?;

    let mut written = 0usize;
    for c in &envelope.contributions {
        let line = serde_json::json!({
            "stream_id": stream_id,
            "schema_version": envelope.schema_version,
            "symbol": envelope.symbol,
            "timestamp_ms": envelope.timestamp_ms,
            "auto_quant_run_id": envelope.auto_quant_run_id,
            "strategy_name": envelope.strategy_name,
            "strategy_mutation_id": envelope.strategy_mutation_id,
            "bar_close_ts_ms": envelope.bar_close_ts_ms,
            "contribution": c,
        });
        let serialised = serde_json::to_string(&line)
            .map_err(|e| anyhow!("failed to serialise jsonl line: {e}"))?;
        writeln!(f, "{serialised}").map_err(|e| anyhow!("failed to write jsonl line: {e}"))?;
        written += 1;
    }
    f.sync_all()
        .map_err(|e| anyhow!("failed to fsync jsonl: {e}"))?;
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::super::wire::{LiveFactorContribution, SCHEMA_VERSION};
    use super::*;

    fn sample_envelope() -> LiveFactorSignalEnvelope {
        LiveFactorSignalEnvelope {
            schema_version: SCHEMA_VERSION.into(),
            symbol: "NQ".into(),
            timestamp_ms: 1_745_678_901_234,
            auto_quant_run_id: "run-1".into(),
            strategy_name: "S".into(),
            strategy_mutation_id: "m-1".into(),
            bar_close_ts_ms: 1_745_678_900_000,
            contributions: vec![
                LiveFactorContribution {
                    factor_name: "f1".into(),
                    category: "c".into(),
                    direction: "Bull".into(),
                    value: 0.1,
                    confidence: 0.5,
                    weighted_score: 0.05,
                    uncertainty_contribution: 0.02,
                    explanation: "".into(),
                },
                LiveFactorContribution {
                    factor_name: "f2".into(),
                    category: "c".into(),
                    direction: "Bear".into(),
                    value: -0.2,
                    confidence: 0.6,
                    weighted_score: -0.12,
                    uncertainty_contribution: 0.03,
                    explanation: "".into(),
                },
            ],
        }
    }

    #[test]
    fn read_cursor_returns_none_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let got = read_cursor(dir.path(), "NQ").unwrap();
        assert!(got.is_none());
    }

    #[test]
    fn write_then_read_cursor_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let cur = LiveSignalsCursor {
            stream_key: "auto_quant:factor_signals:nq".into(),
            last_id: "1745678901234-0".into(),
            updated_at: "2026-04-26T12:00:00Z".into(),
        };
        write_cursor(dir.path(), "NQ", &cur).unwrap();
        let got = read_cursor(dir.path(), "NQ").unwrap().unwrap();
        assert_eq!(got, cur);
    }

    #[test]
    fn write_cursor_overwrites_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let mut cur = LiveSignalsCursor {
            stream_key: "k".into(),
            last_id: "1-0".into(),
            updated_at: "t".into(),
        };
        write_cursor(dir.path(), "NQ", &cur).unwrap();
        cur.last_id = "2-0".into();
        write_cursor(dir.path(), "NQ", &cur).unwrap();
        let got = read_cursor(dir.path(), "NQ").unwrap().unwrap();
        assert_eq!(got.last_id, "2-0");
    }

    #[test]
    fn append_envelope_writes_one_line_per_contribution() {
        let dir = tempfile::tempdir().unwrap();
        let env = sample_envelope();
        let n = append_envelope(dir.path(), "NQ", "1-0", &env).unwrap();
        assert_eq!(n, 2);
        let raw = std::fs::read_to_string(jsonl_path(dir.path(), "NQ")).unwrap();
        let lines: Vec<&str> = raw.trim().split('\n').collect();
        assert_eq!(lines.len(), 2);
        for line in lines {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            assert_eq!(v["stream_id"].as_str(), Some("1-0"));
            assert_eq!(v["symbol"].as_str(), Some("NQ"));
            assert!(v["contribution"]["factor_name"].is_string());
        }
    }

    #[test]
    fn append_envelope_appends_does_not_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let env = sample_envelope();
        append_envelope(dir.path(), "NQ", "1-0", &env).unwrap();
        append_envelope(dir.path(), "NQ", "2-0", &env).unwrap();
        let raw = std::fs::read_to_string(jsonl_path(dir.path(), "NQ")).unwrap();
        let count = raw.trim().lines().count();
        assert_eq!(count, 4);
    }
}
