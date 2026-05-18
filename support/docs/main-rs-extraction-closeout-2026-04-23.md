# `main.rs` Extraction Closeout

Date: 2026-04-23

Status note (2026-04-24)

This closeout is no longer the current source of truth for structural debt status.
It is superseded by `support/docs/plans/2026-04-24-one-shot-structural-debt-closure-plan.md`.

The statement below that the extraction debt was "substantially closed" became stale after later fact-based review and subsequent extraction work. Keep this file as historical context, not as the current debt assessment.

## Scope

This closeout covers the long-running extraction of report shells, helper chains, and command/report orchestration glue from [`src/main.rs`](../src/main.rs) into library modules under [`src/application/`](../src/application/) and a small number of shared shell modules.

The goal of this effort was not to make `main.rs` disappear. The goal was to reduce `main.rs` to:

- CLI parsing
- thin command adapters
- wiring of already-owned library surfaces

and to remove from it:

- report shell definitions
- payload/render glue
- backtest/report/finalize helpers
- factor-lifecycle helper chains
- pre-bayes and multi-timeframe helper chains
- SOP report shell assembly

## What Moved

Representative extracted surfaces now live here:

- `src/backtest_report_shell.rs`
- `src/application/reporting/backtest_output.rs`
- `src/application/backtest/*`
- `src/application/belief/pre_bayes_summary.rs`
- `src/application/belief/market_profiles.rs`
- `src/application/data_sources/clean_futures.rs`
- `src/application/data_sources/sop_reports.rs`
- `src/application/factor_lifecycle/expansion_scoring.rs`
- `src/application/factor_lifecycle/expansion_regressions.rs`
- `src/application/factor_lifecycle/expansion_objective.rs`
- `src/application/factor_lifecycle/expansion_evaluation.rs`
- `src/application/factor_lifecycle/mutation_preferences.rs`
- `src/application/factor_lifecycle/mutation_routing.rs`
- `src/application/factor_lifecycle/mutation_spec.rs`
- `src/application/factor_lifecycle/mutation_summary.rs`
- `src/application/factor_lifecycle/mutation_templates.rs`
- `src/application/regime/multi_timeframe_training.rs`
- `src/application/regime/native_frame_aggregation.rs`
- `src/application/regime/native_frame_analysis.rs`
- `src/application/multi_timeframe_inputs.rs`

## End State

The extraction debt this effort targeted is considered substantially closed.

`main.rs` still contains command entrypoints and some orchestration wrappers, but the previously obvious monolith debt is no longer concentrated there as report shells and helper chains.

What remains in `main.rs` is now mostly acceptable entrypoint code:

- CLI command parsing and dispatch
- thin wrappers that call library runners/builders
- a small number of command-specific flows that are still easier to understand in the entry binary

## Verification

The extraction line was re-verified with:

- `cargo fmt --check`
- `cargo check --all-targets`
- `cargo test`
- `cargo clippy --all-targets -- -D warnings`

## Non-Goals

This closeout does not claim that all business logic in the repository is perfectly modular.

It specifically claims:

- the obvious `main.rs` shell/helper/orchestration debt targeted by this extraction is no longer the dominant structural problem
- new work should not regress `main.rs` back into a monolith
