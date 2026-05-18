# Auto-Quant → ict-engine Real Trade Outcomes (Phase 3)

Date: 2026-04-26
Status: shipping
Phase: 3 of 3 (real-trade feedback channel)

## Context

Phase 1 (offline prior init via Beta-Binomial pseudo-counts) and
Phase 2 (live factor-signal Redis stream) are landed. Phase 3 closes
the loop: when an Auto-Quant strategy actually trades — backtest,
dry-run, or live — the realised win/loss outcomes must flow back
into ict-engine's `trade_outcome` CPT via the existing
`apply_feedback_to_trade_outcome_network` path.

Today the only path that does this is `ict-engine update`, which
takes **one** trade at a time on the CLI. That's fine for hand
operation; it's not viable for batch ingestion of a backtest's
`trades.json` (hundreds of rows) or for daily reconciliation of a
dry-run database.

Phase 3 ships two pieces:

1. **ict-engine consumer**: `ict-engine auto-quant-ingest-real-trades`
   — reads a JSONL artifact, builds `FeedbackRecord`s, applies them
   to the trading network, persists the merged `LearningState`, and
   emits a content-hash-keyed ledger entry that prevents accidental
   double-ingestion.

2. **Auto-Quant exporter**:
   `auto_quant_export_real_trades.py` — turns a FreqTrade backtest
   results JSON into the ingest-ready JSONL artifact, with the
   strategy's `AUTO_QUANT_META` block flattened in for provenance.

## Wire format (JSONL artifact)

One JSON object per line. UTF-8. The producer is
`auto_quant_export_real_trades.py`; the consumer is the new
ict-engine command.

```json
{
  "schema_version": "1.0",
  "symbol": "NQ",
  "trade_id": "MyBreakoutICT:NQ:2026-04-23T13:45:00Z",
  "strategy_name": "MyBreakoutICT",
  "strategy_mutation_id": "mb-001",
  "auto_quant_run_id": "backtest:NQ:MyBreakoutICT:20260424T0930Z",
  "open_ts_ms": 1745423100000,
  "close_ts_ms": 1745427900000,
  "direction": "Bull",
  "pnl": 0.0123,
  "realized_outcome": "win",
  "regime_at_entry": "expansion",
  "entry_signal": "strong_buy",
  "factors_used": [
    {
      "factor_name": "ict_breakout_5m",
      "category": "breakout",
      "direction": "Bull",
      "value": 0.41,
      "confidence": 0.7,
      "weighted_score": 0.29,
      "uncertainty_contribution": 0.08
    }
  ],
  "model_probabilities_before_trade": {
    "selected_direction": "Bull",
    "selected_probability": 0.62,
    "long_score": 0.31,
    "short_score": -0.05,
    "win_prob_long": 0.66,
    "win_prob_short": 0.42,
    "uncertainty": 0.18
  }
}
```

Field rules:

- `schema_version` must equal `"1.0"`.
- `direction` ∈ `{"Bull", "Bear", "Neutral"}`. Maps to
  `Direction` in ict-engine.
- `realized_outcome` ∈ `{"win", "loss", "breakeven"}`. The ingest
  command additionally accepts a missing field if `pnl` is
  present and derives the label via `trade_outcome_label_from_pnl`.
- `regime_at_entry` is normalised via `normalize_regime_label`;
  unknown values default to `manipulation_expansion`.
- `entry_signal` is normalised via `normalize_entry_quality_label`.
- `factors_used` may be empty. If empty, `factor_alignment` and
  `factor_uncertainty` fall back to neutral labels (which produces
  a defensible-but-uninformative CPT row update — better than
  refusing data).
- All floats must be finite. NaN / inf are rejected.

## ict-engine — `auto-quant-ingest-real-trades`

```
ict-engine auto-quant-ingest-real-trades \
    --symbol NQ \
    --state-dir state \
    --trades <path to .jsonl> \
    [--source <label>]   # default: "auto_quant_real_trades"
    [--dry-run]          # parse + summarise, do not mutate BBN/learning state
    [--force]            # override the same-content-hash guard
```

Behaviour:

1. Load and validate the JSONL file. Lines that fail validation are
   counted and surfaced; the run continues on the rest unless
   **all** lines fail (in which case exit non-zero, no state change).
2. Compute `content_hash = sha256(canonical_jsonl_bytes)`.
3. Look up the artifact ledger for the symbol. If a previous
   `auto_quant_real_trades_ingested` entry with the same
   `content_hash` exists and `--force` is **not** set, refuse with
   a clear message that names the prior `artifact_id`. This guard
   is the symmetric twin of the Phase 1 prior-init same-library
   guard: ingesting the same backtest twice doubles the effective
   evidence, which silently corrupts the CPT.
4. Build `FeedbackRecord`s; `enrich_feedback_record` fills in
   `run_id` (= the new ingest artifact id), `trade_id` (from the
   JSONL or auto-derived), `prompt_version`, and
   `factor_version`.
5. Apply via `apply_feedback_to_trade_outcome_network` (which uses
   the canonical `CPTUpdater::batch_update`, the same code path
   `update_command` exercises for one-at-a-time updates).
6. Persist:
   - `LearningState` via `append_learning_feedback_batch` (which
     deduplicates on `(symbol, timestamp, source, trade_id)`, so
     re-ingestion of partially-overlapping batches is safe).
   - Mutated trading network via `save_state(BBN_STATE_FILE, ..)`.
7. Emit a `auto_quant_real_trades_ingested` ledger entry with:
   - `artifact_id = auto_quant_real_trades_<symbol>_<ns_ts>`
   - `source_run_id = content_hash` (lineage / idempotency key)
   - `path` = absolute path of the source JSONL
   - `status = "applied" | "dry_run_preview" | "no_op"`
   - `quality_score = applied_count.min(i32::MAX)`
   - `review_reason` = human-readable summary
   - `review_rule_version = "auto-quant-real-trades-v1"`

## Rollback recipe

The trading network mutation is irreversible *in place*. To roll
back an ingestion run:

1. Delete `<state_dir>/<symbol>/bbn_network.json` and
   `<state_dir>/<symbol>/learning_state.json`.
2. Re-run `ict-engine auto-quant-prior-init …` to rebuild the
   prior from the strategy library (Phase 1 path).
3. Re-run `ict-engine auto-quant-ingest-real-trades` for every
   trades artifact you still want to apply, in the original order.
4. Optionally clean stale `auto_quant_real_trades_ingested`
   ledger entries — they remain on disk as audit history but no
   longer correspond to live state.

The ledger's `source_run_id = content_hash` makes this replay
deterministic: if you re-ingest the same JSONL, you get the same
new artifact_id but the same content hash, so the ordering audit
is preserved.

## Auto-Quant — `auto_quant_export_real_trades.py`

Single script under `Auto-Quant/`. Reads a FreqTrade backtest
results JSON (the file emitted by `freqtrade backtesting … --export
trades` at `user_data/backtest_results/*.json`) and writes a JSONL
file ready to feed to ict-engine.

```
python auto_quant_export_real_trades.py \
    --backtest-result user_data/backtest_results/foo.json \
    --strategy user_data/strategies_ibkr/MyBreakoutICT.py \
    --symbol NQ \
    --output state/NQ/realized_trades_<run_id>.jsonl \
    [--auto-quant-run-id <id>]   # default: derived from filename + utcnow
```

The strategy module is parsed for `AUTO_QUANT_META` so each row
carries `strategy_name` + `strategy_mutation_id` provenance. If
the strategy's meta block is invalid, the export refuses (no
silent provenance loss).

Self-test: `python auto_quant_export_real_trades.py --selftest`
runs a fixture FreqTrade-results dict through the conversion and
asserts the output JSONL is valid and round-trips. Suitable for CI.

## Tests

Shipped (ict-engine):

- Unit (wire): schema_version, direction, realized_outcome,
  NaN/inf, missing-fields rejection.
- Unit (ingest): empty file → no_op ledger; one-line file →
  applied ledger + LearningState updated; same-content-hash second
  run refused without --force, accepted with --force; --dry-run
  emits dry_run_preview ledger and does **not** mutate BBN.
- Integration: import strategy library (Phase 1) → prior-init →
  ingest a 3-row JSONL → assertions on the trade_outcome CPT row
  drift and ledger lineage.

Shipped (Auto-Quant):

- `--selftest`: fixture FreqTrade-results dict → expected JSONL
  shape; validation rejects NaN; round-trip JSON.

## Out of scope (post-Phase 3)

- Real-time feedback (websocket / Redis stream of fills). Today the
  cadence is "after a backtest" or "daily reconciliation", which a
  file-based JSONL handles fine.
- Cross-strategy aggregation: each ingestion run is single-strategy
  by construction (the Auto-Quant exporter picks one strategy
  module). Multi-strategy ingestion is just running the command
  once per strategy.
- Authentication / signing of the JSONL artifact. The content
  hash provides integrity, not authenticity. Out of scope until we
  have a multi-host story.

## Risks

| Risk | Mitigation |
|---|---|
| Same backtest applied twice silently doubles CPT evidence | content_hash guard in ledger; --force required to override |
| FreqTrade results schema change | exporter has `--selftest` + a single allow-list of fields; a schema bump fails loudly |
| Mid-batch crash leaves partial state | apply via `CPTUpdater::batch_update` (single transaction); JSONL parse + validation happens **before** any mutation |
| Operator forgets to roll back BBN before re-ingesting | ledger entry status remains visible; rollback recipe in this doc; --force is clearly destructive |
