# `main.rs` Guardrails

## Rule

Do not add new business shells, helper chains, or report/orchestration bodies to [`src/main.rs`](../src/main.rs).

`main.rs` is for:

- CLI argument definitions
- command selection
- thin adapters that call library APIs
- final output emission when that emission is already library-backed

## Forbidden Additions

Do not put these in `main.rs`:

- new report shell structs
- compare/report/result DTOs
- payload builder helpers
- human/json render helpers
- factor-lifecycle helper chains
- pre-bayes policy/summary helper chains
- multi-timeframe aggregation helpers
- backtest finalize/report assembly helpers
- SOP report builders
- large command orchestration bodies when a library runner can own them

## Required Placement

Put new code here instead:

- `src/application/reporting/` for output builders, renderers, compare surfaces
- `src/application/backtest/` for backtest/report/finalize/request/result logic
- `src/application/factor_lifecycle/` for mutation, expansion, lifecycle scoring/evaluation logic
- `src/application/belief/` for pre-bayes, market-profile, belief-side helpers
- `src/application/data_sources/` for cleaned-data/snapshot/source pipelines and SOP report builders
- `src/application/regime/` for regime/native-frame/multi-timeframe training helpers
- shared shell modules when the type must cross binary/library boundaries

## Review Gate

Any PR that adds a nontrivial helper or shell to `main.rs` must justify why it cannot live in a library module.

Default reviewer answer should be: move it out.

## Test Rule

When extracting a helper from `main.rs`:

1. add or redirect a test to the library API first
2. make it fail cleanly if the library API does not exist yet
3. move implementation
4. delete the local duplicate from `main.rs`
5. re-run:
   - `cargo fmt --check`
   - `cargo check --all-targets`
   - `cargo test`
   - `cargo clippy --all-targets -- -D warnings`

## Success Condition

If a future diff makes `main.rs` longer, the author should be able to explain why that growth is unavoidable entrypoint code rather than misplaced library logic.
