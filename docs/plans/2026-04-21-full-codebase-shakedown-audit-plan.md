# Full Codebase Shakedown Audit Plan

## Goal

Produce a repo-local audit in `docs/` that reviews the whole `ict-engine` codebase from three perspectives:

- open-source contributor first run
- released human CLI user
- released agent consumer

The audit must include:

- real command simulations
- 10+ usage scenarios
- bugs and UX failures found
- feature/documentation/testing gaps
- concrete fixes

## Method

1. Capture current repo state and existing docs/audits.
2. Run static verification (`cargo check`, `cargo test`, targeted smoke/help flows).
3. Simulate 10+ realistic scenarios across contributor, user, and agent paths.
4. Cross-check command surfaces against README/docs and persisted state expectations.
5. Record findings with severity, reproduction, impact, and fix guidance.
6. Save final audit in `docs/`.

## Constraints

- Do not revert unrelated user changes in the dirty worktree.
- Do not change product/module logic during the audit.
- Any code edits during this session must be limited to documentation/analysis support unless a separate follow-up task is requested.
