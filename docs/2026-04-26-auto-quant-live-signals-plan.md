# Auto-Quant → ict-engine Live Signals (Phase 2)

Date: 2026-04-26
Status: shipping
Phase: 2 of 3 (live signal channel)

## Context

Phase 1 (offline prior init, plan
`@/Users/thrill3r/projects-ict-engine/ict-engine/docs/2026-04-26-auto-quant-bbn-prior-init-plan.md`)
landed the path that turns validated Auto-Quant strategy backtests
into Beta-Binomial pseudo-counts on the `trade_outcome` CPT. Phase 1
runs **once** at strategy adoption.

Phase 2 covers the **live channel**: while a strategy is active, its
factor evaluations on each new bar must reach ict-engine's Stage D
(pre-Bayes filter) as `FactorContribution[]` so the live decision
loop sees them. The plan doc for Phase 1 explicitly listed Phase 2
as out-of-scope.

## Architectural decision: Redis-stream handoff (not PyO3)

The Phase 1 plan suggested a PyO3 in-process adapter. We **do not**
adopt PyO3. Reasons:

1. ict-engine has no Python ABI today and adding one couples ict-engine
   to a specific CPython version, packaging story, and venv layout.
2. The existing IBKR live-data bridge
   (`@/Users/thrill3r/Auto-Quant/scripts/ibkr_bridge/bridge.py`)
   already uses Redis Streams as the cross-process fan-out fabric,
   with multiple Auto-Quant + ict-engine consumers reading from it.
   A second producer / second stream is the matching, low-coupling
   shape.
3. Redis Streams give us back-pressure, replay-from-id, and natural
   crash safety (ict-engine restarts re-read from the last applied
   id). PyO3 would have to recreate all of this.
4. `redis = "0.27"` is a small, audited dep with a synchronous API
   that fits ict-engine's blocking-by-default style. Adding it costs
   far less than maintaining a PyO3 surface.

So the wire is: **Auto-Quant publisher → Redis Stream → ict-engine
consumer → JSONL state file → existing Stage D consumers**.

## Two channels into the BBN (refresher)

| Channel | Source | When | Effect |
|---|---|---|---|
| Prior init (Phase 1, shipped) | Auto-Quant validated backtests | Once at adoption | Beta-Binomial pseudo-counts on `trade_outcome` |
| **Live signal (Phase 2 — this doc)** | Auto-Quant strategy on each bar | Continuously | `FactorContribution[]` written to a state JSONL, consumable by Stage D |
| Posterior update (Phase 3) | Real trades | After each fill | `apply_feedback_to_trade_outcome_network` |

## Wire format

Stream key: `auto_quant:factor_signals:<symbol>` (lowercase symbol).

Each XADD entry has a single field `payload` whose value is a UTF-8
JSON document:

```json
{
  "schema_version": "1.0",
  "symbol": "NQ",
  "timestamp_ms": 1745678901234,
  "auto_quant_run_id": "live:NQ:MyBreakoutICT:20260426T120000Z",
  "strategy_name": "MyBreakoutICT",
  "strategy_mutation_id": "mb-001",
  "bar_close_ts_ms": 1745678900000,
  "contributions": [
    {
      "factor_name": "ict_breakout_5m",
      "category": "breakout",
      "direction": "Bull",
      "value": 0.42,
      "confidence": 0.71,
      "weighted_score": 0.30,
      "uncertainty_contribution": 0.08,
      "explanation": "BOS confirmed above 5m HH; FVG aligned"
    }
  ]
}
```

Field rules:

- `schema_version` must equal `"1.0"`. Consumers reject other values.
- `direction` is one of `"Bull" | "Bear" | "Neutral"` (matches
  `crate::types::Direction` debug repr).
- `category` is the FactorCategory string used in ict-engine's
  factor_lab. Unknown values pass through as `"external"`.
- `value`, `confidence`, `weighted_score`, `uncertainty_contribution`
  are finite f64s. NaN / inf are rejected.
- `contributions` is non-empty.

A separate sidecar key `auto_quant:factor_signals_status:<symbol>`
(Redis HASH) carries publisher liveness:

```
last_publish_ts_ms = <int>
last_publish_id    = <stream id>
publisher_state    = "running" | "stopped" | "error: <msg>"
publisher_pid      = <int>
```

ict-engine never writes to this hash; it is read-only diagnostics.

## Auto-Quant — publisher

New top-level script
`@/Users/thrill3r/Auto-Quant/auto_quant_live_signal_publisher.py`.

Usage:

```
python auto_quant_live_signal_publisher.py \
    --symbol NQ \
    --strategy user_data/strategies_ibkr/MyBreakoutICT.py \
    --bar-size 5min \
    --redis-url redis://localhost:6379 \
    [--max-iter N]              # bound emissions for tests
    [--selftest]                # in-process fixture run, no Redis
```

The publisher:

1. Imports the strategy module, reads its `AUTO_QUANT_META`.
2. Asserts the strategy module exposes `live_factor_contributions`
   — a callable
   `(df: pd.DataFrame, latest_index: int) -> list[dict]` matching
   the wire schema.
3. Loops, XREAD-ing from `ibkr:bars:<symbol>:<bar_size>` with
   `BLOCK=2000`. For each new bar batch it builds a DataFrame
   identical in shape to FreqTrade's, calls
   `live_factor_contributions`, validates the dicts, wraps them in
   the envelope, and XADDs to `auto_quant:factor_signals:<symbol>`.
4. Updates the status hash on each iteration. On any unhandled
   exception the publisher writes `publisher_state = "error: …"`
   and exits non-zero.

`--selftest` skips Redis entirely. It feeds a fixed three-bar
DataFrame through the strategy, validates the output against the
wire schema, and prints `PASS`. CI runs this. The strategy template
(`_template.py.example`) is updated to ship a working
`live_factor_contributions` stub.

## ict-engine — consumer

New module `src/application/auto_quant/live/`:

- `mod.rs` — facade
- `wire.rs` — `LiveFactorSignalEnvelope`, `LiveFactorContribution`,
  serde + validation
- `consumer.rs` — sync Redis client, XREAD loop, last-id resume
- `persistence.rs` — JSONL append at
  `<state_dir>/<symbol>/auto_quant_live_factor_contributions.jsonl`
  + ledger artifact

Resume semantics: ict-engine persists the last applied stream id at
`<state_dir>/<symbol>/auto_quant_live_signals_cursor.json`. On
startup the consumer reads that cursor and XREADs from it. If the
cursor file is missing it starts from `$` (only future entries),
not from `0`, to avoid re-applying a backlog after a multi-day
ict-engine outage.

Each successfully-parsed envelope:

1. Appends one JSON line per `LiveFactorContribution` to the JSONL
   path, with the wire envelope's metadata copied alongside the
   contribution (so each line is self-contained for downstream
   readers).
2. Updates the cursor file atomically (write tmp, fsync, rename).

Invalid envelopes (bad schema_version, NaN floats, empty
contributions) are logged with their stream id and **dropped** —
they do not advance the cursor (so a fixed publisher can replay
them).

## CLI

```
ict-engine auto-quant-consume-live-signals \
    --symbol NQ \
    --state-dir state \
    --redis-url redis://localhost:6379 \
    [--max-iter N]      # bound XREAD iterations; default = unbounded
    [--block-ms 2000]   # XREAD BLOCK; default 2000
    [--start-from $]    # initial cursor when no file present;
                        # accepts "$" or a stream id
```

`--max-iter` and `--start-from` exist for tests + first runs. In
production the operator sets neither.

## Ledger

New artifact kind `auto_quant_live_signals_ingested` written **once
per consumer session that actually processed at least one envelope**
on graceful shutdown (Ctrl-C, `--max-iter` reached, fatal Redis
error). It records:

- `symbol`
- `redis_url` (sanitised — host/port only, password stripped)
- `stream_key`
- `cursor_start_id` / `cursor_end_id`
- `envelopes_applied`
- `envelopes_dropped`
- `started_at` / `ended_at` (RFC 3339)

This is **operational telemetry, not state**. Re-running the
consumer creates a fresh entry; there is no "applied / superseded"
state machine because the JSONL is append-only and the cursor is
authoritative for resume semantics.

## Tests

Shipped:

- Unit (wire): schema_version mismatch rejected, NaN/inf rejected,
  empty contributions rejected, round-trip JSON.
- Unit (persistence): JSONL append is one-line-per-contribution,
  cursor write is atomic (tmp + rename), cursor read is forgiving
  on missing file.
- Unit (consumer, no Redis): a synthetic XREAD response feeds the
  parser path end-to-end.
- Integration (in-process): a fake Redis trait impl feeds 3
  envelopes; assertions on JSONL file shape, cursor advance, ledger
  artifact.
- Auto-Quant `--selftest`: DataFrame fixture → publisher → wire
  envelope → matches schema.

Not shipped (deferred to Phase 3 / ops):

- A real-Redis end-to-end test in CI (the in-process fake covers
  the same parse + persist code paths today; a Redis container in
  CI is operational scope).
- Stage D integration. The JSONL is the contract; Stage D consumers
  read it via a separate follow-up wiring change. Phase 2 stops at
  delivering the JSONL safely.

## Out of scope (Phase 3 and beyond)

- Real-trade `FeedbackRecord` ingestion (Phase 3).
- Bidirectional handshake (publisher reading ict-engine's CPT state).
- Multi-strategy fan-in (today, one stream per symbol carries one
  strategy's contributions; we expand to multi-strategy in a later
  phase if it becomes a real need).
- Authentication / TLS for Redis. Today the bridge runs on
  `localhost`. Hardening is an ops task, not an integration task.

## Risks

| Risk | Mitigation |
|---|---|
| Publisher emits NaN values silently | Wire validation rejects them on both sides |
| Consumer falls behind and stream is trimmed | Stream uses `MAXLEN ~ 50_000`; cursor records last id; if cursor refers to a trimmed id, consumer logs a single drift warning and resumes from `$` |
| Redis password leaks into ledger | `redis_url` is sanitised before persist |
| Two consumers race on same JSONL | JSONL append uses `O_APPEND` writes; cursor file is per-symbol; running two consumers on one symbol is **not supported** and logged as an unsafe state via Redis status hash if detected |
