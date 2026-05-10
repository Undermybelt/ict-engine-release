# Factor Artifact Naming Contract

## Purpose

This contract exists to stop one recurring failure mode:

- a board contains real historical execution evidence
- but the current workspace does not expose a directly reusable input
- and the two states get confused as "missing evidence"

The factor-iteration lane therefore uses four distinct artifact layers.

## Canonical Layers

1. `board_record`
   - Meaning: a repo doc records real historical execution evidence or conclusions.
   - Owner examples:
     - `docs/plans/2026-05-05-execution-tree-factor-auto-quant-todo.md`
     - `docs/plans/2026-05-08-factor-iteration-filter-belief-catboost-execution-tree-board.md`
   - Not sufficient by itself to rebuild a candidate pack.

2. `reusable_input`
   - Meaning: the current workspace exposes a machine-consumable input.
   - Allowed examples:
     - `freqtrade_backtest_zip`
     - `strategy_library.json`
     - `regime_benchmark_json`
   - This is the minimum layer needed for repeatable pack construction.

3. `candidate_pack`
   - Meaning: the white-box output contract produced by the factor-iteration helper layer.
   - Current family:
     - `factor_expression.json`
     - `factor_eval_grid_summary.json`
     - `transfer_score.json`

4. `temp_state_dir`
   - Meaning: ephemeral run-time or validation state under `/tmp/...`.
   - This may prove a run happened, but it is not a stable semantic owner.

## Required Registry Fields

Every factor candidate registry entry should carry:

- `candidate_id`
- `evidence_status`
- `artifact_kind`
- `board_evidence_status`
- `pack_build_reason`

### `evidence_status`

- `buildable`
  - current workspace has a directly reusable input
- `board_evidence_only`
  - historical board evidence exists, but no reusable input is present
- `deferred`
  - the lane is intentionally blocked by a named prerequisite

### `artifact_kind`

- `freqtrade_backtest_zip`
- `strategy_library_json`
- `regime_benchmark_json`
- `candidate_placeholder`
- `regime_gate_placeholder`

### `board_evidence_status`

- `board_recorded`
  - an authoritative repo document already records the lane

## Naming Rules

- Reserve `state` for runtime or temp execution state only.
- Use `profile` for explicit opt-in overlays that inject personal or environment-specific evidence.
- Use `candidate` for reusable logical entries in the factor registry.
- Use `pack` only for emitted white-box factor artifact outputs.

## Interpretation Rules

- `board_record` does not imply `buildable`.
- `buildable` does not imply `promotable`.
- `temp_state_dir` does not become canonical merely because it is the newest artifact.
- A lane may be `board_recorded + deferred` without being lost or invalid.
- A regime-only lane may stay zero-config by default while exposing
  `artifact_kind=regime_benchmark_json`; concrete benchmark paths should arrive
  only through an explicit opt-in profile or a future shared bundle.
- `reusable_input` means "machine-consumable and readable now", not merely
  "filesystem path exists". A corrupt `freqtrade_backtest_zip` must stay
  non-buildable and be surfaced as `invalid_artifact:...`.
