# ICT Engine Post-Remediation Ten-Run Trial Plan

**Date:** 2026-04-25  
**Repo:** `/Users/thrill3r/projects-ict-engine/ict-engine`  
**Operator:** Cascade acting as a post-fix trial user

## Goal

Run `ict-engine` ten times after the reporting-surface remediation, validate that the repaired build and human `Next:` outputs hold up in realistic usage, and record any remaining user friction without introducing repo-local runtime pollution.

## Scope

- Use the already built binary when possible.
- Keep all runtime artifacts under isolated `/tmp` state directories.
- Revisit the first-run / early-use path that previously exposed the P0/P1 issues.
- Include both success paths and expected design-boundary failures if they are meaningful from a user perspective.
- Do not edit implementation during the ten-run execution phase.

## Out of Scope

- Installing system packages.
- Starting Docker.
- Using private user data.
- Auto-Quant strategy authoring beyond the existing product boundary.
- Repo-local ignored `state/` writes.

## Trial State

- Main isolated state: `/tmp/ict-engine-post-remediation-ten-run-20260425`
- Optional branch state: `/tmp/ict-engine-post-remediation-ten-run-20260425-aq`

## Planned Ten Runs

1. `cargo build`
2. `./target/debug/ict-engine --help`
3. `./target/debug/ict-engine analyze --help`
4. `./target/debug/ict-engine factor-research --help`
5. `./target/debug/ict-engine analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-post-remediation-ten-run-20260425 --human`
6. `./target/debug/ict-engine factor-pipeline-debug --symbol DEMO --data examples/demo/demo-15m.json --factor structure_ict --objective expansion_manipulation`
7. `./target/debug/ict-engine factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-post-remediation-ten-run-20260425 --backend native --human`
8. `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-post-remediation-ten-run-20260425 --human`
9. `./target/debug/ict-engine backtest --symbol DEMO --data examples/demo/demo-15m.json --human --state-dir /tmp/ict-engine-post-remediation-ten-run-20260425`
10. `./target/debug/ict-engine auto-quant-status --state-dir /tmp/ict-engine-post-remediation-ten-run-20260425`

## Evaluation Focus

For each run, capture:

- command purpose from a user perspective
- whether the output is understandable on first read
- whether `Next:` is directly actionable when shown in human-readable output
- hidden prerequisites or surprising defaults
- whether the command teaches the user what to do next without leaking wire protocol or broken placeholders

## Success Criteria

- Ten runs are completed and logged.
- The repaired build remains healthy.
- Human `Next:` surfaces are executable where appropriate.
- Findings distinguish between expected boundaries and remaining usability issues.
- A new report is written to `docs/ict-engine-post-remediation-ten-run-trial-report-2026-04-25.md`.
