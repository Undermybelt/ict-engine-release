//! JSONL → FeedbackRecord ingest path with content-hash idempotency.
//!
//! The ledger artifact_kind is `auto_quant_real_trades_ingested`.
//! Every ingest run pins `source_run_id = content_hash`, which is
//! the SipHash of the source JSONL bytes. A second run against the
//! same file is rejected unless `--force` is set, so accidental
//! double-ingestion (which would silently double the effective
//! evidence weight on the trade_outcome CPT) cannot happen.

use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;

use crate::application::backtest::apply_feedback_to_trade_outcome_network;
use crate::bbn::trading::persistence::load_or_init_trading_network;
use crate::config::compute_hash;
use crate::state::{
    append_artifact_ledger_entry, append_learning_feedback_batch, load_state_or_default,
    save_state, ArtifactLedgerEntry, ARTIFACT_LEDGER_FILE, BBN_STATE_FILE,
};

use super::wire::RealTradeRecord;

/// Ledger artifact_kind for this ingest path.
pub const ARTIFACT_KIND_REAL_TRADES: &str = "auto_quant_real_trades_ingested";

/// Rule version recorded on every real-trades ledger entry. Bump on
/// any change to the wire schema, idempotency key, or evidence
/// computation.
pub const REAL_TRADES_RULE_VERSION: &str = "auto-quant-real-trades-v1";

/// Operator-facing input for `ingest_real_trades`.
#[derive(Debug, Clone)]
pub struct IngestRealTradesInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
    pub trades_path: &'a str,
    pub source: &'a str,
    pub dry_run: bool,
    pub force: bool,
}

/// Outcome of an ingest run, suitable for serialising to stdout
/// alongside the ledger artifact id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestRealTradesOutcome {
    pub artifact_id: String,
    pub status: &'static str,
    pub trades_total: u32,
    pub trades_applied: u32,
    pub trades_invalid: u32,
    pub feedback_records_inserted: u32,
    pub content_hash: String,
    pub previous_artifact_id: Option<String>,
}

/// Read + validate the JSONL trades file, apply the implied
/// `FeedbackRecord`s through the trading-network update path, and
/// emit a ledger entry summarising the run.
pub fn ingest_real_trades(input: IngestRealTradesInput<'_>) -> Result<IngestRealTradesOutcome> {
    let raw = std::fs::read_to_string(input.trades_path).with_context(|| {
        format!(
            "reading real-trades JSONL artifact at '{}'",
            input.trades_path
        )
    })?;

    let content_hash = compute_hash(&[raw.as_str()]);

    let previous = find_existing_for_hash(input.state_dir, input.symbol, &content_hash)?;
    if !input.force {
        if let Some(prev_id) = &previous {
            bail!(
                "refusing to re-ingest the same JSONL: a prior \
                 auto_quant_real_trades_ingested entry with content_hash \
                 '{}' exists ({}); pass --force after rolling back the BBN \
                 to override",
                content_hash,
                prev_id
            );
        }
    }

    let timestamp = Utc::now();
    let artifact_id = format!(
        "auto_quant_real_trades_{}_{}",
        input.symbol,
        timestamp.format("%Y%m%dT%H%M%S%.9fZ")
    );

    let (records, invalid_count) = parse_jsonl(&raw, input.trades_path)?;
    let total: u32 = (records.len() + invalid_count) as u32;

    if records.is_empty() {
        // Same status whether or not invalid_count > 0: no records
        // mean no CPT mutation, by definition. The invalid count is
        // surfaced separately in `review_reason` for audit.
        let status = "no_op";
        write_ledger(
            input,
            &artifact_id,
            timestamp,
            status,
            total,
            0,
            invalid_count as u32,
            0,
            &content_hash,
        )?;
        return Ok(IngestRealTradesOutcome {
            artifact_id,
            status,
            trades_total: total,
            trades_applied: 0,
            trades_invalid: invalid_count as u32,
            feedback_records_inserted: 0,
            content_hash,
            previous_artifact_id: previous,
        });
    }

    if input.dry_run {
        let status = "dry_run_preview";
        write_ledger(
            input,
            &artifact_id,
            timestamp,
            status,
            total,
            records.len() as u32,
            invalid_count as u32,
            0,
            &content_hash,
        )?;
        return Ok(IngestRealTradesOutcome {
            artifact_id,
            status,
            trades_total: total,
            trades_applied: records.len() as u32,
            trades_invalid: invalid_count as u32,
            feedback_records_inserted: 0,
            content_hash,
            previous_artifact_id: previous,
        });
    }

    // Build feedback records, anchoring run_id on the ingest
    // artifact when the source did not carry one.
    let feedback_records = records
        .into_iter()
        .map(|r| {
            let mut fr = r.into_feedback_record(input.source);
            if fr.run_id.is_none() {
                fr.run_id = Some(artifact_id.clone());
            }
            fr
        })
        .collect::<Vec<_>>();
    let trades_applied = feedback_records.len() as u32;

    // Apply the CPT update first so the BBN snapshot is consistent
    // with what we report below. Fail-loudly: no partial mutation.
    let mut network = load_or_init_trading_network(input.symbol, input.state_dir)?;
    let updates_applied = apply_feedback_to_trade_outcome_network(&mut network, &feedback_records)?;

    save_state(input.state_dir, input.symbol, BBN_STATE_FILE, &network)?;

    let mut feedback_records_inserted: u32 = 0;
    if !feedback_records.is_empty() {
        let _learning_state = append_learning_feedback_batch(
            std::path::Path::new(input.state_dir),
            input.symbol,
            &feedback_records,
        )?;
        // We surface the CPT-evidence count rather than
        // `learning_state.feedback_history.len()`. The latter is the
        // running total across all symbols' history (deduped on
        // (symbol, timestamp, source, trade_id)), which is useful
        // for audit elsewhere but not the right granularity here.
        feedback_records_inserted = updates_applied.min(u32::MAX as usize) as u32;
    }

    let status = if trades_applied > 0 {
        "applied"
    } else {
        "no_op"
    };

    write_ledger(
        input,
        &artifact_id,
        timestamp,
        status,
        total,
        trades_applied,
        invalid_count as u32,
        feedback_records_inserted,
        &content_hash,
    )?;

    Ok(IngestRealTradesOutcome {
        artifact_id,
        status,
        trades_total: total,
        trades_applied,
        trades_invalid: invalid_count as u32,
        feedback_records_inserted,
        content_hash,
        previous_artifact_id: previous,
    })
}

fn parse_jsonl(raw: &str, path_label: &str) -> Result<(Vec<RealTradeRecord>, usize)> {
    let mut records = Vec::new();
    let mut invalid = 0usize;
    for (idx, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<RealTradeRecord>(trimmed) {
            Ok(record) => match record.validate() {
                Ok(_) => records.push(record),
                Err(e) => {
                    log::warn!(
                        "real-trades JSONL '{}' line {}: validation failed: {}",
                        path_label,
                        idx + 1,
                        e
                    );
                    invalid += 1;
                }
            },
            Err(e) => {
                log::warn!(
                    "real-trades JSONL '{}' line {}: parse failed: {}",
                    path_label,
                    idx + 1,
                    e
                );
                invalid += 1;
            }
        }
    }
    Ok((records, invalid))
}

fn find_existing_for_hash(
    state_dir: &str,
    symbol: &str,
    content_hash: &str,
) -> Result<Option<String>> {
    let ledger: Vec<ArtifactLedgerEntry> =
        load_state_or_default(state_dir, symbol, ARTIFACT_LEDGER_FILE)?;
    Ok(ledger
        .into_iter()
        .rev()
        .find(|e| {
            e.artifact_kind == ARTIFACT_KIND_REAL_TRADES
                && (e.status == "applied" || e.status == "dry_run_preview")
                && e.source_run_id.as_deref() == Some(content_hash)
        })
        .map(|e| e.artifact_id))
}

#[allow(clippy::too_many_arguments)]
fn write_ledger(
    input: IngestRealTradesInput<'_>,
    artifact_id: &str,
    timestamp: chrono::DateTime<Utc>,
    status: &'static str,
    trades_total: u32,
    trades_applied: u32,
    trades_invalid: u32,
    feedback_records_inserted: u32,
    content_hash: &str,
) -> Result<()> {
    let path = std::path::Path::new(input.trades_path)
        .canonicalize()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| input.trades_path.to_string());

    let review_reason = format!(
        "ingested {trades_applied}/{trades_total} (invalid {trades_invalid}) \
         feedback_inserted={feedback_records_inserted} content_hash={content_hash} \
         dry_run={dry_run} force={force}",
        dry_run = input.dry_run,
        force = input.force,
    );

    append_artifact_ledger_entry(
        input.state_dir,
        input.symbol,
        ArtifactLedgerEntry {
            entry_id: format!("ledger:{}", artifact_id),
            artifact_kind: ARTIFACT_KIND_REAL_TRADES.to_string(),
            artifact_id: artifact_id.to_string(),
            version: 1,
            generated_at: timestamp,
            symbol: input.symbol.to_string(),
            source_phase: "auto_quant_real_trades".to_string(),
            source_run_id: Some(content_hash.to_string()),
            path,
            status: status.to_string(),
            promote_candidate: false,
            actionable: false,
            decision_hint: format!("ingested {trades_applied} trade(s)"),
            review_reason,
            review_rule_version: REAL_TRADES_RULE_VERSION.to_string(),
            quality_score: trades_applied.min(i32::MAX as u32) as i32,
            ..Default::default()
        },
    )
    .map_err(|e| anyhow!("failed to append real-trades ledger entry: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::auto_quant::real_trades::wire::SCHEMA_VERSION;

    fn good_jsonl_line() -> String {
        let s = format!(
            r#"{{"schema_version":"{}","symbol":"NQ","trade_id":"t-1","strategy_name":"S","strategy_mutation_id":"m-1","auto_quant_run_id":"run-1","open_ts_ms":1745423100000,"close_ts_ms":1745427900000,"direction":"Bull","pnl":0.0123,"realized_outcome":"win","regime_at_entry":"expansion","entry_signal":"strong_buy","factors_used":[]}}"#,
            SCHEMA_VERSION
        );
        s
    }

    fn write_jsonl(path: &std::path::Path, lines: &[String]) {
        std::fs::write(path, lines.join("\n")).unwrap();
    }

    #[test]
    fn no_op_when_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().to_str().unwrap();
        let trades = dir.path().join("trades.jsonl");
        std::fs::write(&trades, "").unwrap();

        let outcome = ingest_real_trades(IngestRealTradesInput {
            symbol: "NQ",
            state_dir,
            trades_path: trades.to_str().unwrap(),
            source: "auto_quant_real_trades",
            dry_run: false,
            force: false,
        })
        .unwrap();

        assert_eq!(outcome.status, "no_op");
        assert_eq!(outcome.trades_applied, 0);
        assert_eq!(outcome.trades_invalid, 0);
    }

    #[test]
    fn applied_status_when_one_valid_trade() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().to_str().unwrap();
        let trades = dir.path().join("trades.jsonl");
        write_jsonl(&trades, &[good_jsonl_line()]);

        let outcome = ingest_real_trades(IngestRealTradesInput {
            symbol: "NQ",
            state_dir,
            trades_path: trades.to_str().unwrap(),
            source: "auto_quant_real_trades",
            dry_run: false,
            force: false,
        })
        .unwrap();

        assert_eq!(outcome.status, "applied");
        assert_eq!(outcome.trades_applied, 1);
        assert_eq!(outcome.feedback_records_inserted, 1);
        assert!(outcome.previous_artifact_id.is_none());

        // Ledger entry pins content_hash as source_run_id.
        let ledger: Vec<ArtifactLedgerEntry> =
            load_state_or_default(state_dir, "NQ", ARTIFACT_LEDGER_FILE).unwrap();
        let entry = ledger
            .iter()
            .find(|e| e.artifact_kind == ARTIFACT_KIND_REAL_TRADES)
            .expect("real-trades ledger entry");
        assert_eq!(
            entry.source_run_id.as_deref(),
            Some(outcome.content_hash.as_str())
        );
        assert_eq!(entry.status, "applied");
    }

    #[test]
    fn second_run_with_same_content_refused_without_force() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().to_str().unwrap();
        let trades = dir.path().join("trades.jsonl");
        write_jsonl(&trades, &[good_jsonl_line()]);

        ingest_real_trades(IngestRealTradesInput {
            symbol: "NQ",
            state_dir,
            trades_path: trades.to_str().unwrap(),
            source: "auto_quant_real_trades",
            dry_run: false,
            force: false,
        })
        .unwrap();

        let err = ingest_real_trades(IngestRealTradesInput {
            symbol: "NQ",
            state_dir,
            trades_path: trades.to_str().unwrap(),
            source: "auto_quant_real_trades",
            dry_run: false,
            force: false,
        })
        .unwrap_err();
        assert!(
            err.to_string().contains("refusing to re-ingest"),
            "got {err}"
        );
    }

    #[test]
    fn force_allows_second_run_and_records_lineage() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().to_str().unwrap();
        let trades = dir.path().join("trades.jsonl");
        write_jsonl(&trades, &[good_jsonl_line()]);

        let first = ingest_real_trades(IngestRealTradesInput {
            symbol: "NQ",
            state_dir,
            trades_path: trades.to_str().unwrap(),
            source: "auto_quant_real_trades",
            dry_run: false,
            force: false,
        })
        .unwrap();

        let second = ingest_real_trades(IngestRealTradesInput {
            symbol: "NQ",
            state_dir,
            trades_path: trades.to_str().unwrap(),
            source: "auto_quant_real_trades",
            dry_run: false,
            force: true,
        })
        .unwrap();

        assert_eq!(first.content_hash, second.content_hash);
        assert_eq!(
            second.previous_artifact_id.as_deref(),
            Some(first.artifact_id.as_str())
        );
    }

    #[test]
    fn dry_run_does_not_persist_bbn() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().to_str().unwrap();
        let trades = dir.path().join("trades.jsonl");
        write_jsonl(&trades, &[good_jsonl_line()]);

        let outcome = ingest_real_trades(IngestRealTradesInput {
            symbol: "NQ",
            state_dir,
            trades_path: trades.to_str().unwrap(),
            source: "auto_quant_real_trades",
            dry_run: true,
            force: false,
        })
        .unwrap();

        assert_eq!(outcome.status, "dry_run_preview");
        assert_eq!(outcome.feedback_records_inserted, 0);
        // No BBN snapshot was written.
        assert!(
            !crate::state::state_exists(state_dir, "NQ", BBN_STATE_FILE),
            "dry-run must not write a BBN snapshot"
        );
    }

    #[test]
    fn invalid_lines_are_counted_and_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().to_str().unwrap();
        let trades = dir.path().join("trades.jsonl");
        write_jsonl(
            &trades,
            &[
                good_jsonl_line(),
                "this is not valid json".to_string(),
                r#"{"schema_version":"9.9","symbol":"NQ","trade_id":"t","strategy_name":"S","open_ts_ms":0,"close_ts_ms":0,"direction":"Bull","pnl":0.0}"#.to_string(),
            ],
        );

        let outcome = ingest_real_trades(IngestRealTradesInput {
            symbol: "NQ",
            state_dir,
            trades_path: trades.to_str().unwrap(),
            source: "auto_quant_real_trades",
            dry_run: false,
            force: false,
        })
        .unwrap();

        assert_eq!(outcome.trades_total, 3);
        assert_eq!(outcome.trades_applied, 1);
        assert_eq!(outcome.trades_invalid, 2);
    }
}
