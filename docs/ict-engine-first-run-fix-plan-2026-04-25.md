# ICT Engine First-Run Fix Plan

**Date:** 2026-04-25

**Goal:** Fix the highest-friction first-run issues found in `docs/ict-engine-first-run-trial-report-2026-04-25.md` without changing research behavior or writing repo-local runtime state.

**Scope:**
- Fix `factor-research --human` so the native backend emits a short human summary instead of a serialized report dump.
- Update README first-run guidance so Rust-only usage takes the native path and Auto-Quant dependency requirements are explicit.
- Let a single recorded historical data path continue without a redundant user-selection gate while preserving the multi-path gate.
- Fix Chinese mojibake in first-run research output without changing research behavior.
- Do not tune factors, change Auto-Quant readiness semantics, or write repo-local runtime state.

**Tasks:**
- [x] Add a focused failing test for factor-research human rendering.
- [x] Implement the smallest renderer change in `src/application/reporting/backtest_output.rs`.
- [x] Update README quick-start/demo wording.
- [x] Run targeted tests, format check, cargo check, and a real CLI smoke command with `/tmp` state.
- [x] Add single-path and multi-path tests for historical-data selection recommendations.
- [x] Let single recorded research paths produce executable next-step commands.
- [x] Add a focused regression test for Chinese prompt/output encoding.
- [x] Implement the smallest encoding fix.
- [x] Run targeted tests, format check, cargo check, and a real CLI smoke command with `/tmp` state.
