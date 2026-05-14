# Auto-Quant PDA Unit Dispatch Plan

Date: 2026-04-28
Status: implementation-active
Scope: direct CLI dispatch of `auto-quant-pda-unit-batch` groups into external Auto-Quant execution plus result collection

Boundary note:
- This dispatch path is internal/experimental tooling.
- Consumer-facing flow should move to generic `agent-material-dispatch`.

## Goal

Add a CLI command that:

1. reads a previously generated PDA unit batch
2. executes each unit through an isolated Auto-Quant external run
3. runs groups in parallel
4. collects per-unit results back into `ict-engine` state

## Supported now

- unit scope based on cleaned candle data
- per-unit strategy scaffold generation
- per-unit AQ workspace materialization
- `run_tomac.py` execution for non-crypto synthetic pair backtests
- result parsing from the runner's stdout blocks

## Explicitly blocked now

If a unit requires any of these evidence surfaces and no external evidence artifact is supplied:

- `greeks`
- `open_interest`
- `implied_volatility`
- `options_chain`
- `cross_market`

then dispatch must mark the unit as blocked rather than fabricating those fields locally.

## Output

Dispatch must persist:

- one dispatch artifact
- one per-unit result record
- one per-group summary

Each unit result should include:

- `unit_id`
- `status`
- `workspace_root`
- `stdout_log_path`
- parsed aggregate metrics when available
- blocking reason when unavailable
