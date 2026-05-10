# ICT Engine Ten-Run User Trial Plan

**Date:** 2026-04-25  
**Repo:** `/Users/thrill3r/projects-ict-engine/ict-engine`  
**Operator:** Cascade acting as a first-run / early-use user

## Goal

Run `ict-engine` ten times from a realistic user perspective, capture friction and unreasonable output surfaces, and write a candid findings document under `docs/`.

## Scope

- Use the built binary when possible to reduce Cargo noise.
- Keep all runtime writes under isolated `/tmp` state directories.
- Favor commands that a new or early user would actually try from the current README/docs surface.
- Record both successful and failing runs if the failure is part of the user experience.
- Do not change runtime code during the trial.

## Out of Scope

- Installing system packages.
- Starting Docker.
- Using real private user data.
- Editing implementation before the ten-run observations are written down.

## Trial State

- Main isolated state: `/tmp/ict-engine-ten-run-user-trial-20260425`
- Additional isolated states may be used if a command naturally belongs in a separate branch.

## Planned Ten Runs

1. `cargo build`
2. `./target/debug/ict-engine --help`
3. `./target/debug/ict-engine analyze --help`
4. `./target/debug/ict-engine factor-research --help`
5. `./target/debug/ict-engine analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-ten-run-user-trial-20260425 --human`
6. `./target/debug/ict-engine factor-pipeline-debug --symbol DEMO --data examples/demo/demo-15m.json --factor structure_ict --objective expansion_manipulation`
7. `./target/debug/ict-engine factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-ten-run-user-trial-20260425 --backend native --human`
8. `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-ten-run-user-trial-20260425 --human`
9. `./target/debug/ict-engine backtest --symbol DEMO --data examples/demo/demo-15m.json --human --state-dir /tmp/ict-engine-ten-run-user-trial-20260425`
10. `./target/debug/ict-engine auto-quant-status --state-dir /tmp/ict-engine-ten-run-user-trial-20260425`

## Evaluation Focus

For each run, capture:

- command purpose from a user perspective
- whether the output is understandable on first read
- hidden prerequisites or surprising defaults
- awkward wording, over-verbose output, or misleading next steps
- whether the result helps the user decide the next action

## Success Criteria

- Ten runs are completed and logged.
- Findings distinguish between:
  - expected design boundaries
  - usability friction
  - unreasonable or misleading output
- A new report is written to `docs/ict-engine-ten-run-user-trial-report-2026-04-25.md`.
