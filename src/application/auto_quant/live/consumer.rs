//! Redis-stream consumer for Auto-Quant live factor signals.
//!
//! The hot path is intentionally stream-source-agnostic: the
//! consumer accepts any `StreamSource` implementation, which keeps
//! the unit tests Redis-free. The real Redis impl
//! ([`RealRedisSource`]) is built on top of the synchronous
//! `redis = 0.27` client.

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use chrono::{SecondsFormat, Utc};
use redis::{Commands, Value};

use super::persistence::{
    append_envelope, cursor_path, jsonl_path, read_cursor, write_cursor, LiveSignalsCursor,
};
use super::wire::{LiveFactorSignalEnvelope, ENVELOPE_FIELD, STREAM_KEY_PREFIX};

/// One Redis stream entry: an XADD id plus the JSON payload string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamEntry {
    pub id: String,
    pub payload: String,
}

/// Pluggable stream source so tests can drive the consumer without
/// a real Redis. Implementations are expected to block up to
/// `block_ms` milliseconds when no entries are available, then
/// return an empty `Vec`.
pub trait StreamSource {
    fn xread_block(
        &mut self,
        stream_key: &str,
        last_id: &str,
        block_ms: u64,
    ) -> Result<Vec<StreamEntry>>;
}

/// Operator-facing input for the consumer session.
#[derive(Debug, Clone)]
pub struct ConsumeLiveSignalsInput {
    pub symbol: String,
    pub state_dir: PathBuf,
    pub redis_url: String,
    /// Optional cap on XREAD iterations (one iteration may yield 0..N
    /// entries). `None` means run until shutdown.
    pub max_iterations: Option<u32>,
    /// Block timeout per XREAD call in milliseconds.
    pub block_ms: u64,
    /// Initial cursor used when no cursor file exists yet.
    /// `"$"` means "start from future entries", which is the safe
    /// default after long downtime; `"0"` replays everything still
    /// in the stream.
    pub initial_id: String,
}

/// Outcome of a single consumer session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumeLiveSignalsOutcome {
    pub stream_key: String,
    pub envelopes_applied: u32,
    pub envelopes_dropped: u32,
    pub iterations: u32,
    pub started_at: String,
    pub ended_at: String,
    pub cursor_start_id: String,
    pub cursor_end_id: String,
}

/// Drive the consumer loop using `source` for IO.
///
/// Stops when `max_iterations` is exhausted or `source` returns an
/// error. Persists a JSONL line per accepted contribution and an
/// atomic cursor update per accepted envelope. Drops envelopes that
/// fail wire-format validation; dropped envelopes do **not** advance
/// the cursor so a fixed publisher can replay them.
pub fn consume_live_signals<S: StreamSource>(
    input: &ConsumeLiveSignalsInput,
    source: &mut S,
) -> Result<ConsumeLiveSignalsOutcome> {
    let started_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);

    let stream_key = build_stream_key(&input.symbol);

    let initial_cursor = read_cursor(&input.state_dir, &input.symbol)?;
    let cursor_start_id = initial_cursor
        .as_ref()
        .map(|c| c.last_id.clone())
        .unwrap_or_else(|| input.initial_id.clone());

    let mut last_id = cursor_start_id.clone();
    let mut envelopes_applied: u32 = 0;
    let mut envelopes_dropped: u32 = 0;
    let mut iterations: u32 = 0;

    loop {
        if let Some(max) = input.max_iterations {
            if iterations >= max {
                break;
            }
        }
        iterations += 1;

        let entries = source.xread_block(&stream_key, &last_id, input.block_ms)?;
        if entries.is_empty() {
            // BLOCK timeout with no entries — still counts as one
            // iteration so `--max-iter` makes test runs deterministic.
            continue;
        }

        for entry in entries {
            match LiveFactorSignalEnvelope::from_json(&entry.payload) {
                Ok(envelope) => {
                    append_envelope(&input.state_dir, &input.symbol, &entry.id, &envelope)?;
                    last_id = entry.id.clone();
                    write_cursor(
                        &input.state_dir,
                        &input.symbol,
                        &LiveSignalsCursor {
                            stream_key: stream_key.clone(),
                            last_id: last_id.clone(),
                            updated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true),
                        },
                    )?;
                    envelopes_applied += 1;
                }
                Err(e) => {
                    log::warn!(
                        "auto-quant live-signal envelope rejected (stream_id={}): {}",
                        entry.id,
                        e
                    );
                    envelopes_dropped += 1;
                    // Do NOT advance last_id on a dropped envelope so
                    // a replay after a publisher fix is possible.
                }
            }
        }
    }

    let ended_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);

    Ok(ConsumeLiveSignalsOutcome {
        stream_key,
        envelopes_applied,
        envelopes_dropped,
        iterations,
        started_at,
        ended_at,
        cursor_start_id,
        cursor_end_id: last_id,
    })
}

/// Compose the canonical stream key. Symbol case is preserved on
/// the output side because Redis keys are byte-strings; we only
/// downcase to keep the key shape consistent across calls.
pub fn build_stream_key(symbol: &str) -> String {
    format!("{STREAM_KEY_PREFIX}:{}", symbol.to_lowercase())
}

/// Real Redis stream source built on the synchronous `redis = 0.27`
/// client.
pub struct RealRedisSource {
    conn: redis::Connection,
}

impl RealRedisSource {
    /// Connect to Redis and return a ready stream source.
    pub fn connect(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| anyhow!("failed to open redis client at {redis_url}: {e}"))?;
        let conn = client
            .get_connection()
            .map_err(|e| anyhow!("failed to connect to redis at {redis_url}: {e}"))?;
        Ok(Self { conn })
    }
}

impl StreamSource for RealRedisSource {
    fn xread_block(
        &mut self,
        stream_key: &str,
        last_id: &str,
        block_ms: u64,
    ) -> Result<Vec<StreamEntry>> {
        let opts = redis::streams::StreamReadOptions::default()
            .block(block_ms as usize)
            .count(64);
        let reply: redis::streams::StreamReadReply = self
            .conn
            .xread_options(&[stream_key], &[last_id], &opts)
            .or_else(|err| {
                // redis-rs surfaces BLOCK timeout as a Nil reply,
                // which deserialises to an error inside the typed
                // reply path. Treat that as "no entries".
                if err.kind() == redis::ErrorKind::TypeError {
                    Ok(redis::streams::StreamReadReply { keys: Vec::new() })
                } else {
                    Err(err)
                }
            })
            .map_err(|e| anyhow!("redis xread on '{stream_key}' failed: {e}"))?;

        let mut out = Vec::new();
        for stream in reply.keys {
            for entry in stream.ids {
                let payload = match entry.map.get(ENVELOPE_FIELD) {
                    Some(Value::BulkString(bytes)) => String::from_utf8_lossy(bytes).into_owned(),
                    Some(Value::SimpleString(s)) => s.clone(),
                    _ => {
                        log::warn!(
                            "auto-quant live-signal stream entry {} missing '{}' field",
                            entry.id,
                            ENVELOPE_FIELD
                        );
                        continue;
                    }
                };
                out.push(StreamEntry {
                    id: entry.id,
                    payload,
                });
            }
        }
        Ok(out)
    }
}

/// Helper exposed for the ledger artifact path: the JSONL file the
/// consumer writes contributions into.
pub fn consumer_jsonl_path(state_dir: &std::path::Path, symbol: &str) -> PathBuf {
    jsonl_path(state_dir, symbol)
}

/// Helper exposed for the ledger artifact path: the cursor file the
/// consumer maintains.
pub fn consumer_cursor_path(state_dir: &std::path::Path, symbol: &str) -> PathBuf {
    cursor_path(state_dir, symbol)
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::super::persistence::jsonl_path;
    use super::super::wire::{LiveFactorContribution, SCHEMA_VERSION};
    use super::*;

    /// Test double: returns a queued sequence of XREAD batches, then
    /// empty Vecs forever.
    #[derive(Default)]
    struct FakeSource {
        batches: VecDeque<Vec<StreamEntry>>,
    }

    impl StreamSource for FakeSource {
        fn xread_block(
            &mut self,
            _stream_key: &str,
            _last_id: &str,
            _block_ms: u64,
        ) -> Result<Vec<StreamEntry>> {
            Ok(self.batches.pop_front().unwrap_or_default())
        }
    }

    fn make_envelope_json(stream_name: &str) -> String {
        let env = LiveFactorSignalEnvelope {
            schema_version: SCHEMA_VERSION.into(),
            symbol: "NQ".into(),
            timestamp_ms: 1_745_678_901_234,
            auto_quant_run_id: format!("run:{stream_name}"),
            strategy_name: "S".into(),
            strategy_mutation_id: "m-1".into(),
            bar_close_ts_ms: 1_745_678_900_000,
            contributions: vec![LiveFactorContribution {
                factor_name: "f1".into(),
                category: "c".into(),
                direction: "Bull".into(),
                value: 0.1,
                confidence: 0.5,
                weighted_score: 0.05,
                uncertainty_contribution: 0.02,
                explanation: "".into(),
            }],
        };
        env.to_json().unwrap()
    }

    fn input_for(state_dir: &std::path::Path, max_iter: u32) -> ConsumeLiveSignalsInput {
        ConsumeLiveSignalsInput {
            symbol: "NQ".into(),
            state_dir: state_dir.to_path_buf(),
            redis_url: "redis://localhost:6379".into(),
            max_iterations: Some(max_iter),
            block_ms: 0,
            initial_id: "$".into(),
        }
    }

    #[test]
    fn applies_envelope_appends_jsonl_and_advances_cursor() {
        let dir = tempfile::tempdir().unwrap();
        let mut src = FakeSource::default();
        src.batches.push_back(vec![StreamEntry {
            id: "1745678901234-0".into(),
            payload: make_envelope_json("a"),
        }]);

        let input = input_for(dir.path(), 1);
        let outcome = consume_live_signals(&input, &mut src).unwrap();

        assert_eq!(outcome.envelopes_applied, 1);
        assert_eq!(outcome.envelopes_dropped, 0);
        assert_eq!(outcome.cursor_end_id, "1745678901234-0");
        assert_eq!(outcome.cursor_start_id, "$");
        assert_eq!(outcome.stream_key, "auto_quant:factor_signals:nq");

        let raw = std::fs::read_to_string(jsonl_path(dir.path(), "NQ")).unwrap();
        assert_eq!(raw.trim().lines().count(), 1);

        let cur = read_cursor(dir.path(), "NQ").unwrap().unwrap();
        assert_eq!(cur.last_id, "1745678901234-0");
        assert_eq!(cur.stream_key, "auto_quant:factor_signals:nq");
    }

    #[test]
    fn invalid_envelope_drops_without_advancing_cursor() {
        let dir = tempfile::tempdir().unwrap();
        let mut src = FakeSource::default();
        src.batches.push_back(vec![StreamEntry {
            id: "1-0".into(),
            payload: r#"{"schema_version":"9.9","symbol":"NQ","timestamp_ms":0,"auto_quant_run_id":"x","strategy_name":"y","bar_close_ts_ms":0,"contributions":[{"factor_name":"f","category":"c","direction":"Bull","value":0.0,"confidence":0.0,"weighted_score":0.0,"uncertainty_contribution":0.0}]}"#.into(),
        }]);

        let input = input_for(dir.path(), 1);
        let outcome = consume_live_signals(&input, &mut src).unwrap();

        assert_eq!(outcome.envelopes_applied, 0);
        assert_eq!(outcome.envelopes_dropped, 1);
        assert_eq!(outcome.cursor_end_id, "$");
        // No JSONL file should exist because every envelope failed.
        assert!(!jsonl_path(dir.path(), "NQ").exists());
        // No cursor file either.
        assert!(read_cursor(dir.path(), "NQ").unwrap().is_none());
    }

    #[test]
    fn resumes_from_persisted_cursor_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let cursor = LiveSignalsCursor {
            stream_key: "auto_quant:factor_signals:nq".into(),
            last_id: "1745678900000-0".into(),
            updated_at: "2026-04-26T12:00:00Z".into(),
        };
        write_cursor(dir.path(), "NQ", &cursor).unwrap();

        let mut src = FakeSource::default();
        // Empty batch: source returns nothing, simulating BLOCK timeout.
        src.batches.push_back(vec![]);

        let input = input_for(dir.path(), 1);
        let outcome = consume_live_signals(&input, &mut src).unwrap();

        assert_eq!(outcome.cursor_start_id, "1745678900000-0");
        assert_eq!(outcome.cursor_end_id, "1745678900000-0");
        assert_eq!(outcome.envelopes_applied, 0);
    }

    #[test]
    fn build_stream_key_lowercases_symbol() {
        assert_eq!(build_stream_key("NQ"), "auto_quant:factor_signals:nq");
        assert_eq!(build_stream_key("es"), "auto_quant:factor_signals:es");
    }

    #[test]
    fn iterations_count_matches_xread_calls_even_without_entries() {
        let dir = tempfile::tempdir().unwrap();
        let mut src = FakeSource::default();
        // Two empty batches.
        src.batches.push_back(vec![]);
        src.batches.push_back(vec![]);

        let input = input_for(dir.path(), 2);
        let outcome = consume_live_signals(&input, &mut src).unwrap();
        assert_eq!(outcome.iterations, 2);
    }
}
