# Factor Artifact Naming Contract

## Purpose

This contract exists to stop one recurring failure mode:

- a board contains real historical execution evidence
- but the current workspace does not expose a directly reusable input
- and the two states get confused as "missing evidence"

The factor-iteration lane therefore uses four distinct artifact layers.

## Canonical Layers

1. `archive_reference`
   - Meaning: a repo doc may explain where a hypothesis came from, but it is
     not product/runtime truth.
   - Not sufficient by itself to rebuild, promote, or ship a candidate pack.
   - Must never be the consumer-facing dependency for a factor or strategy.

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
- `archive_evidence_status`
- `curation_decision`
- `pack_build_reason`

### `evidence_status`

- `buildable`
  - current workspace has a directly reusable input
- `missing_reusable_artifact`
  - no machine-consumable input is present; do not promote from prose
- `deferred`
  - the lane is intentionally blocked by a named prerequisite

### `artifact_kind`

- `freqtrade_backtest_zip`
- `strategy_library_json`
- `regime_benchmark_json`
- `candidate_pack_dir`
- `candidate_placeholder`
- `regime_gate_placeholder`

### `archive_evidence_status`

- `archive_reference_only`
  - historical prose may help explain provenance, but runtime artifacts decide
    usefulness

### `curation_decision`

- `promote_to_candidate_pack`
  - build a reusable candidate pack from current machine-consumable input
- `promote_to_regime_artifact_bundle`
  - build a reusable regime bundle from current machine-consumable input
- `needs_named_prerequisite`
  - blocked by an explicit missing profile or input
- `discard_until_reusable_artifact`
  - keep out of the active loop until a real reusable artifact exists

## Naming Rules

- Reserve `state` for runtime or temp execution state only.
- Use `profile` for explicit opt-in overlays that inject personal or environment-specific evidence.
- Use `candidate` for reusable logical entries in the factor registry.
- Use `pack` only for emitted white-box factor artifact outputs.

## Interpretation Rules

- `archive_reference` does not imply `buildable`.
- `buildable` does not imply `promotable`.
- `candidate_pack_dir` is the preferred repo-local promotion input: it carries
  the distilled useful result without requiring raw Auto-Quant workspaces,
  private backtest zips, or historical board prose.
- `temp_state_dir` does not become canonical merely because it is the newest artifact.
- A lane may be `archive_reference_only + deferred` without being lost or invalid.
- A lane with only prose history must be classified as
  `discard_until_reusable_artifact`, not treated as an active candidate.
- A regime-only lane may stay zero-config by default while exposing
  `artifact_kind=regime_benchmark_json`; concrete benchmark paths should arrive
  only through an explicit opt-in profile or a future shared bundle.
- `reusable_input` means "machine-consumable and readable now", not merely
  "filesystem path exists". A corrupt `freqtrade_backtest_zip` must stay
  non-buildable and be surfaced as `invalid_artifact:...`.
