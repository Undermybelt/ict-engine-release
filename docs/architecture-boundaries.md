# Architecture boundaries

## Context
ict-engine has a large `src/main.rs`, growing workflow/reporting surfaces, and multiple persisted artifacts. New execution/voting work must not widen cross-layer drift.

## Layers
1. domain
   - regime / belief / strategy semantics
   - stable typed packets
2. factor_lab
   - factor ranking, research, backtest mechanics
3. application
   - adapters, reporting surfaces, reflection bundles, decision helpers
4. state
   - persisted records, workflow snapshot, artifact ledger, provenance/comparability
5. cli orchestration (`src/main.rs`)
   - parse args, call domain/application/state helpers, emit output

## Boundary rules
- `main.rs` may wire outputs, but new ensemble logic should not live there.
- New regime posterior/voting types belong in `domain/*` or `application/*`, not inline in CLI.
- New persisted audit artifacts must be defined in `state/types.rs` and persisted via `state/persistence.rs`.
- External adapter / exchange / market-data integrations should prefer a JSON-first subprocess contract with stable error categories (`api`, `auth`, `network`, `rate_limit`, `validation`, `config`, `io`, `parse`) and keep paper/sim paths first-class before any live path.
- For `ict-engine`, external adapter scope is read-only market data, snapshot/replay, and sim-oriented research support. No live order execution, withdrawals, transfers, staking, or broker-shell command surfaces belong in the default adapter boundary.
- Cross-source workflow intake should prefer minimal primitive modules, explicit data->analysis->report separation, reusable SOP/skill crystallization, and typed workflow surfaces; avoid importing speculative theory stacks directly into trading inference.
- Reflection/reporting extensions should go through `application/reflection/*` or other application adapters first.
- If adding CatBoost/XGBoost later, keep them behind executor traits/adapters so command surfaces depend on stable typed outputs, not crate-specific APIs.

## Allowed adapter bridges
- domain -> application
- factor_lab -> application
- state -> application
- cli -> application/state/domain

## Forbidden drift
- no new giant logic blocks in `main.rs`
- no executor-specific structs embedded directly into unrelated run records without a dedicated audit artifact
- no provenance/fingerprint logic hidden in ad hoc JSON blobs

## Ensemble extension target shape
- posterior artifact: typed, comparable, provenance-aware
- executor outputs: typed leaf/action/confidence/explanation packets
- voting aggregation: typed ensemble decision + audit snapshot
- CLI integration: helper/adapters emit versioned artifacts and attach summarized views to existing surfaces
