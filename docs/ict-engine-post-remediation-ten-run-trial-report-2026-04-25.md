# ICT Engine Post-Remediation Ten-Run Trial Report

**Date:** 2026-04-25  
**Repo:** `/Users/thrill3r/projects-ict-engine/ict-engine`  
**Runtime state:** `/tmp/ict-engine-post-remediation-ten-run-20260425b` during the formal ten-run execution  
**Goal:** Re-run the user-facing first-run / early-use path after the reporting-surface fixes and record whether the repaired build and human `Next:` outputs hold up in practice without repo-local runtime state.

## Scope

- Use the built binary where possible.
- Keep runtime artifacts under `/tmp` only.
- Revisit the same command family that previously exposed the build break and non-executable human `Next:` issues.
- Treat expected product boundaries separately from true UX regressions.

## Ten Runs

| Run | Command | Result | User-facing observation |
|---|---|---|---|
| 1 | `cargo build` | Passed in about 15s. | The P0 build break is gone. Fresh build is usable again. |
| 2 | `target/debug/ict-engine --help` | Passed. | Top-level command surface is coherent and includes `analyze`, `factor-research`, `workflow-status`, and Auto-Quant lifecycle commands. |
| 3 | `target/debug/ict-engine analyze --help` | Passed. | `--demo`, `--state-dir`, `--human`, and `--inline-ledger` are discoverable. Default `state` path is still visible, so docs/examples must keep steering users to explicit `/tmp` state for no-pollution trials. |
| 4 | `target/debug/ict-engine factor-research --help` | Passed. | Help is understandable, but `--backend <BACKEND>` still defaults to `auto-quant`, which remains a surprising first-run default for users who only want the Rust-only path. |
| 5 | `target/debug/ict-engine analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-post-remediation-ten-run-20260425b --human` | Passed. | Human output is short and readable. It reported `Bull bias`, `Entry: medium`, `Gate: pass_hard`, `Quality: 0.880`, `Action: REPLACE volatility_mean_reversion`, and a direct executable `Next:` command pointing to native factor research. |
| 6 | `target/debug/ict-engine factor-pipeline-debug --symbol DEMO --data examples/demo/demo-15m.json --factor structure_ict --objective expansion_manipulation` | Passed. | Still intentionally verbose. Key fields were mechanical and useful: `evidence_quality_score=0.6173`, `gating_status=pass_neutralized`, `selected_entry_quality=medium`, `bridge_gap=0.0216`, `pipeline_verdict=pre_bayes_pass_but_bridge_needs_confirmation`. Good diagnostics, not a first-screen surface. |
| 7 | `target/debug/ict-engine factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-post-remediation-ten-run-20260425b --backend native --human` | Passed. | Human output stays concise instead of dumping serialized JSON. Best factor was `trend_momentum`; feedback summary was `generated=46 applied=46`; `Next:` remained directly executable and preserved `--backend native`. |
| 8 | `target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-post-remediation-ten-run-20260425b --human` | Passed. | Human summary is compact. `Block: none`. `Next:` is now a direct command and no longer duplicates `Next step:`. This confirms the P1 workflow-status surface is fixed. |
| 9 | `target/debug/ict-engine backtest --symbol DEMO --data examples/demo/demo-15m.json --human --state-dir /tmp/ict-engine-post-remediation-ten-run-20260425b` | Expected failure. | Still fails with `need more candles for backtest: got 52, require at least 71`. This is a demo-data boundary, not a regression introduced by the reporting fixes. |
| 10 | `target/debug/ict-engine auto-quant-status --state-dir /tmp/ict-engine-post-remediation-ten-run-20260425b` | Passed. | Status cleanly reports `missing_dependency`, `bootstrap_needed=true`, and the executable next command `ict-engine auto-quant-bootstrap --state-dir /tmp/ict-engine-post-remediation-ten-run-20260425b`. The recommendation is actionable and no longer muddied by path placeholder issues. |

## Confirmed Improvements

- `cargo build` succeeds again on the current source tree.
- `analyze --human` now emits a short human report instead of a broken or misleading follow-up surface.
- `factor-research --human` keeps `Next:` directly executable and preserves the intended `--backend native` path.
- `workflow-status --human` no longer renders `Next: Next step: ...` and now presents a directly runnable command.
- Human-readable `Next:` surfaces no longer collapse executable commands into `<local-path>` placeholders.

## Remaining Friction

- `factor-research --help` still defaults to `auto-quant`, which remains a surprising default for first-run users who are not yet prepared for dependency management.
- `factor-pipeline-debug` is still very large and mechanical. This is appropriate for a diagnostic command, but it is not beginner-friendly and should stay off the onboarding path.
- Demo `backtest` still hits the known 52-candle boundary. The error is clear, but onboarding docs should continue to avoid implying the bundled demo data is sufficient for backtesting.
- `analyze --help` and `factor-research --help` still advertise `[default: state]` for `--state-dir`. That is technically correct, but from a no-pollution perspective it remains safer for docs to always show explicit `/tmp/...` examples.

## User Judgment

The repaired surfaces now behave like a usable CLI again.

The two previously critical issues are resolved at the point of use:

- the binary builds
- the human `Next:` lines are actionable instead of misleading

What remains is mostly product-boundary or onboarding polish, not breakage.

## Pollution Guard

- The formal ten-run execution used `/tmp/ict-engine-post-remediation-ten-run-20260425b` only.
- No deliberate repo-local runtime state was created for this trial.
- No system packages were installed.
- No Docker services were started.
- No private user datasets were used.

## Recommendation

For the current remediation scope, the report-driven fixes are successful.

If follow-up polishing is wanted, the highest-value next UX tasks are:

1. make the Rust-only first path even more explicit in `factor-research --help` / docs
2. keep steering users toward explicit `/tmp` state-dir examples
3. optionally expose the demo backtest candle-count boundary more proactively in docs or help text
