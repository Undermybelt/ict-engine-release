# External adapter contract

Status: draft

Purpose
- Define a read-only, agent-first adapter contract for external market-data tools.
- Keep `ict-engine` in research/data mode.
- Prevent drift into broker-shell behavior.

Scope
Allowed:
- market data reads
- historical OHLC fetch
- order book snapshots
- ticker/trade snapshots
- replay/snapshot ingestion
- sim/paper-oriented read paths

Forbidden:
- live order execution
- account auth flows inside adapter contract
- withdrawals, transfers, staking
- broker-side account mutation

Core subprocess contract
- stdout: machine-readable JSON only
- stderr: diagnostics only
- exit code 0: success
- non-zero exit: failure
- failure stdout should emit a JSON error envelope when possible

Required success envelope
```json
{
  "ok": true,
  "provider": "example",
  "operation": "ohlc.fetch",
  "data": {}
}
```

Required error envelope
```json
{
  "ok": false,
  "provider": "example",
  "operation": "ohlc.fetch",
  "error": {
    "category": "rate_limit",
    "message": "human readable detail",
    "retryable": true
  }
}
```

Rules
- Agent routing must use `error.category`, not free-text message parsing.
- Adapter contracts must be read-only by default.
- Any side-effecting adapter operation requires separate explicit design approval and is out of scope for `ict-engine`.
- Prefer snapshot/replay symmetry: a live-read adapter and a replay adapter should normalize to the same internal surface.

Suggested operations
- `ticker.fetch`
- `ohlc.fetch`
- `orderbook.fetch`
- `trades.fetch`
- `status.check`
- `snapshot.load`

Suggested metadata per operation
- `provider`
- `operation`
- `auth_required`
- `dangerous`
- `supports_json_stdout`
- `rate_limit_class`
- `asset_classes`

Design boundary
- Adapter layer returns normalized envelopes.
- `factor_lab` consumes normalized market data only.
- `application` handles routing, error classification, and workflow packaging.
- `state` persists provenance, failures, and adapter audit trails.
