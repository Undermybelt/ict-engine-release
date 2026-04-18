# Change surface

## Task objective
Assess and stage a safe path to add an ensemble voting execution layer after HMM + Bayesian filtering in ict-engine.

## Drift class
- workflow-surface-drift
- boundary-erosion
- main-file-bloat risk

## Editable paths for this task
- `docs/architecture-boundaries.md`
- `docs/change-surface.md`
- `docs/drift-ledger.md`
- `docs/paper-driven-typed-packets-design.md`
- `docs/typed-packets-paper-upgrade-plan.md`
- `src/domain/regime/*`
- `src/domain/belief/*`
- `src/state/*`
- `src/application/belief/*`
- `src/application/reflection/*`
- `src/reporting/belief/*`
- `src/factor_lab/factor_definition.rs`
- tests for the above surfaces

## Non-editable paths for this task
- `src/main.rs`
- runtime code paths
- Cargo dependencies
- persisted schema files beyond documentation inspection

## Constraints
- artifact-first only for this pass
- no speculative dependency install
- preserve current workflow/reporting/state contracts

## Verification
- confirm existing posterior/belief/state surfaces in repo
- confirm artifact ledger and reflection bundle integration points
- keep recommendations scoped to typed adapters rather than monolithic CLI edits

## Rollback trigger
If evidence shows the repo already has a different canonical extension seam, stop and revise this surface before code changes.
