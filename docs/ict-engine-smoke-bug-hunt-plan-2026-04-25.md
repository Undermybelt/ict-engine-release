# ICT Engine Smoke / Bug Hunt Plan

**Date:** 2026-04-25  
**Repo:** `/Users/thrill3r/projects-ict-engine/ict-engine`  
**Operator:** Cascade  
**Goal:** Run one focused smoke round against the current `ict-engine` CLI, separate regressions from known boundaries, and only patch code if a reproducible root-cause bug is confirmed.

## Scope

- Re-verify the Rust-only demo first-run path that was fixed earlier:
  - `cargo check`
  - `cargo build`
  - `./target/debug/ict-engine --help`
  - `./target/debug/ict-engine analyze --symbol DEMO --demo --state-dir <tmp> --human`
  - `./target/debug/ict-engine factor-pipeline-debug --symbol DEMO --data examples/demo/demo-15m.json --factor structure_ict --objective expansion_manipulation`
  - `./target/debug/ict-engine factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir <tmp> --backend native --human`
  - `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir <tmp> --human`
- Run one broader smoke path from `docs/smoke-acceptance.md` to probe `train -> analyze -> factor-research -> update -> workflow-status` using isolated `/tmp` state.
- Stop at external-install, real-market-data, or human-choice gates unless the step is already covered by a previously approved local-only path.

## Not In Scope

- Installing system packages.
- Starting Docker or mutating existing long-lived user state.
- Treating documented demo limits as bugs:
  - demo `backtest` needing more candles
  - Auto-Quant `run.py` needing at least one authored strategy file

## Trial State

- Rust-only regression path: `/tmp/ict-engine-bug-hunt-demo-20260425`
- Broader smoke path: `/tmp/ict-engine-bug-hunt-smoke-20260425`
- Any Auto-Quant trial, if needed: `/tmp/ict-engine-bug-hunt-aq-20260425`

## Success Criteria

- No regression in the post-fix Rust-only path:
  - `factor-research --backend native --human` stays concise and readable
  - `workflow-status --human` does not duplicate `Next step:`
  - demo analyze/research commands keep `/tmp` state and executable `Next` behavior where appropriate
- Either the broader smoke chain passes, or each failure is classified as one of:
  - code bug
  - documented design constraint
  - environment/configuration gate
- Every finding is recorded with exact commands and observed output.

## Execution Rules

- Prefer the built binary (`./target/debug/ict-engine`) after `cargo build` to avoid repeated Cargo noise.
- Keep all runtime writes under `/tmp`.
- If a failure looks like a bug, reproduce once before editing code.
- If the failure looks environmental, prove it with a minimal command and stop there.

## Planned Outputs

- `docs/ict-engine-smoke-bug-hunt-log-2026-04-25.md`
- Code changes only if a clear root-cause bug is confirmed and safely fixable within this session.
