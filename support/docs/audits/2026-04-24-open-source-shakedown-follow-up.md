# ict-engine Open-Source / User / Agent Shakedown Follow-Up · 2026-04-24

## Scope

Follow-up verification focused on the current dirty tree after the autoresearch derived-surface fixes.

This pass prioritized:

- open-source contributor trust
- release/user-facing metadata correctness
- agent empty-state contract quality
- current CI truth versus docs truth

## Method

### Static and read-only verification

- repo docs review: `README.md`, `support/docs/first-run.md`, `support/docs/smoke-acceptance.md`, `support/docs/agent-first-runbook.md`, `support/docs/release-mirror-runbook.md`
- existing audits review, especially `support/docs/audits/2026-04-21-full-codebase-shakedown.md`
- worktree inspection via `git status --short`
- CLI/help audit via `python3 support/scripts/help_audit.py`
- exact CI gate verification via:
  - `cargo fmt --check`
  - `cargo check`
  - `cargo clippy --all-targets -- -D warnings`
- targeted regression checks for touched surfaces

### Limited runtime verification

- `cargo run -- factor-autoresearch-status --symbol DEMO --state-dir /tmp/ict-engine-empty-status --latest-only`
- `cargo run -- research-verdict --symbol DEMO --state-dir /tmp/ict-engine-empty-status`

A broader stateful smoke run under `/tmp` was prepared but not completed in this pass.

## Verified Current Facts

### Contributor baseline

- root CLI help renders correctly
- `python3 support/scripts/help_audit.py` passes with no missing help descriptions
- current tree now passes strict clippy again
- current tree passes `cargo check`
- current tree passes `cargo fmt --check`

### Agent empty-state surfaces

`factor-autoresearch-status` empty-state contract is now explicit and semantically clean:

- `status = "no_autoresearch_state"`
- `live_snapshot = null`
- `sessions = []`
- `attempts = []`

`research-verdict` no-history contract is now guarded correctly:

- `current_bottleneck = "no_research_runs"`
- `stop_or_continue = "bootstrap_required"`
- `recommended_next_experiment` points to bootstrapping research

### Older whole-repo audit items that are no longer live

The following older findings were re-checked and do not appear to be live regressions in the current tree:

- output-format conflict guard for `analyze`
- human next-step leakage of raw `ask-user:` syntax on already-covered human render paths
- `factor-autoresearch-status` placeholder empty-state object
- `research-verdict` returning `continue` with zero research runs
- `factor-backtest --human` being a single serialized dump

## Confirmed Live Issues In This Pass

### 1. CI/doc contract drift on clippy gate

Observed before the fix in this pass:

- `.github/workflows/ci.yml` enforced `cargo clippy --all-targets -- -D warnings`
- `README.md` still described clippy as advisory and not a merge gate

Impact:

- contributor expectations were wrong
- local pre-PR checklist did not match actual CI behavior

Resolution in this pass:

- fixed the immediate clippy failures introduced on the autoresearch surface
- updated `README.md` so the contributor baseline matches current CI

### 2. Release-facing package metadata drift

Observed before the fix in this pass:

- `Cargo.toml` pointed `repository` at `https://github.com/thrill3r/ict-engine`
- release docs say outward publishing happens through `Undermybelt/ict-engine-release`

Impact:

- package metadata did not describe the actual release transport surface
- contributors and release consumers could be routed to the wrong repo URL

Resolution in this pass:

- updated `Cargo.toml` repository metadata to `https://github.com/Undermybelt/ict-engine-release`

### 3. Current dirty tree remains broad

Observed:

- multiple tracked files are already modified outside the narrow fix scope
- multiple docs and autoresearch files remain untracked in the worktree

Impact:

- merge/conflict risk remains high
- review clarity is lower than it should be for release-polish work

Resolution in this pass:

- not fixed directly; this is a branch hygiene / change-management issue, not a one-file bug

## Code Changes Made In This Pass

- `src/application/factor_lifecycle/autoresearch_surface.rs`
  - fixed strict clippy failures on the retrospective markdown path and median helper
- `src/main.rs`
  - removed duplicated `#[cfg(test)]` attributes
- `Cargo.toml`
  - aligned release-facing repository metadata with the documented mirror flow
- `README.md`
  - aligned contributor checklist with actual CI gates
- `support/docs/plans/2026-04-23-open-source-shakedown-plan.md`
  - added the versioned execution plan for this shakedown

## Verification Results After Fixes

Verified green in this pass:

- `cargo test retrospective_markdown_sanitizes_free_text_fields -- --nocapture`
- `cargo test retrospective_multi_session_surface_uses_multi_session_header -- --nocapture`
- `cargo test test_output_format_resolve_rejects_human_and_explicit_json_mix -- --nocapture`
- `cargo check --message-format short`
- `cargo fmt --check`
- `cargo clippy --all-targets --no-deps -- -D warnings`
- `cargo clippy --all-targets -- -D warnings`

## Deferred / Not Fully Closed

### 1. Full stateful contributor smoke run

Not completed in this pass:

- `analyze --demo --human`
- `analyze --demo --agent`
- `workflow-status --agent`

Reason:

- the isolated `/tmp` smoke run was prepared but not completed in-session

### 2. Dirty-tree management

Still open:

- worktree breadth is larger than the minimal fix scope
- unrelated pending files should be triaged before a release/signoff step

### 3. Hermes routing convention mismatch

Observed:

- repo contains `.hermes/`
- expected `.hermes/routing/*` files were not present in this worktree

This is noted as a project-process gap, not treated as a code bug in this pass.

## Bottom Line

This pass did not reveal a fresh broad product failure.

Instead, it found and closed a smaller but real set of release-polish issues:

- strict clippy gate drift reintroduced by recent code changes
- contributor checklist drift versus CI truth
- package metadata drift versus documented release flow

The repo is in a materially better contributor/release state after these fixes, but a final signoff should still include one isolated `/tmp` stateful smoke run and a deliberate review of the remaining dirty tree.
