# Workflow-status hard-block adapter extraction — 2026-04-15

## Goal
Move workflow-status shaping logic out of `src/main.rs` into typed application adapters, starting with hard-block / ensemble history surfaces.

## Why
- `main.rs` is still too heavy.
- hard-block aggregation and history filtering are application/reporting concerns, not CLI concerns.
- recent external intake and architecture boundaries now explicitly require:
  - minimal atomic modules
  - data -> analysis -> report separation
  - reusable workflow/SOP crystallization
  - typed workflow surfaces
  - no speculative-theory leakage into inference

## Scope for this extraction slice
1. Extract vote scorecard resolution helper from CLI into `application/orchestration/*`.
2. Extract ensemble-vote history shaping into typed adapter(s).
3. Extract hard-block-only / reason / limit filtering into typed adapter(s).
4. Keep CLI responsible only for:
   - parsing args
   - choosing adapter/phase
   - printing JSON
5. Extend adapter extraction to human surface, ensemble-vote single surface, and auxiliary artifact/report surfaces.

## Proposed target shape
- `src/application/orchestration/workflow_status.rs`
  - typed human workflow surface builder
  - typed ensemble-vote surface builder
  - typed ensemble-history response structs
  - typed auxiliary artifact/report surface helpers
  - helper to resolve scorecards
  - helper to build hard-block filtered rows

## Boundary rules for this slice
- No new large shaping blocks in `main.rs`.
- Keep persisted record definitions in `state/types.rs` only.
- CLI should not manually aggregate hard-block reason leaderboards once adapter exists.
- Adapter outputs should remain versionable and JSON-serializable.

## Immediate next implementation order
1. Introduce typed workflow-status adapter structs in application layer.
2. Port `resolved_vote_scorecards` first.
3. Port ensemble-vote-history aggregation second.
4. Port human workflow surface third.
5. Port ensemble-vote single surface fourth.
6. Port auxiliary artifact/report surface branches fifth.
7. Rewire CLI callsites.
8. Preserve current JSON shape with regression tests.

## Current completed slice
- scorecard resolution moved into adapter
- hard-block filtering moved into adapter
- ensemble-vote-history moved into adapter
- human workflow view moved into adapter
- ensemble-vote single surface moved into adapter
- sample human workflow fixture and nearby adapter tests moved next to adapter
- pending/execution/artifact summary+trend surface access now routed through adapter helper

## Remaining likely extraction targets
- artifact-consumed-gate
- artifact-factor-consumed-validation / leaderboard
- artifact-family-consumed-validation / leaderboard
- artifact-lineage-summaries
- artifact-decision-summary
- artifact-rule-break* surfaces
- artifact-impact* surfaces

## Non-goals
- Full `main.rs` breakup in one pass.
- Reworking persistence format.
- Redesigning report semantics.

## Verification
- Existing workflow-status tests keep passing.
- Existing ensemble/history tests keep passing.
- No JSON shape regression for current CLI surfaces.
