# 2026-04-23 Open-Source Contributor / User / Agent Shakedown Plan

## Objective

Do a whole-repo shakedown from four perspectives:

- open-source contributor first run
- test / smoke / release operator
- released human CLI user
- released agent consumer

Then fix only the highest-value, reproducible problems with minimal scope expansion.

## Constraints

- repo artifacts are the source of truth
- prefer focused fixes over broad refactors
- keep changes reviewable in an already-dirty tree
- validate with the smallest meaningful command/test loop
- do not assume older audits are still fully current; re-verify before changing code

## Baseline Observations

- repo contains `.hermes/`, but no `.hermes/routing/*` files were present in this worktree
- `src/main.rs` remains the primary CLI entry and is still very large
- `README.md`, `support/docs/first-run.md`, `support/docs/smoke-acceptance.md`, `support/docs/agent-first-runbook.md`, and `support/docs/release-mirror-runbook.md` define the main contributor / release expectations
- current worktree is already dirty before this shakedown closes
- prior full-codebase audit exists at `support/docs/audits/2026-04-21-full-codebase-shakedown.md` and should be treated as a hypothesis list, not ground truth

## Phase Plan

### Phase 1 — Map the current product surface

- inventory CLI commands, output modes, state-dir behavior, and release/public docs
- inventory tests, scripts, and CI expectations
- inventory current dirty-tree scope and likely collision points

### Phase 2 — Reproduce contributor / user / agent issues

- verify safe first-run commands (`--help`, env, docs-aligned smoke surfaces)
- re-check previously reported UX / empty-state / output-symmetry issues
- identify which issues are still live in the current tree

### Phase 3 — Prioritize fix set

Prioritize problems that are:

1. reproducible now
2. user-visible or agent-contract-visible
3. cheap to fix without architecture drift
4. easy to lock with targeted tests

### Phase 4 — Implement and verify

- land minimal fixes only for issues confirmed in Phase 2
- add targeted regression tests where contract risk is high
- run focused tests plus `cargo check` / `cargo fmt --check`
- summarize remaining deferred risks explicitly

## Verification Targets

### Static

- `cargo check`
- `cargo fmt --check`
- selected unit tests around changed surfaces

### Safe command surfaces

- `cargo run -- --help`
- `python3 support/scripts/help_audit.py`
- doc-aligned `--help` / no-state commands

### Potentially stateful smoke checks

Use isolated temp state dirs when needed so repo state does not get polluted.

## Candidate Risk Buckets

- output-format contract mismatches
- human surface leaking machine / agent protocol text
- weak or misleading empty-state contracts
- release/docs mismatch with actual runnable surfaces
- dirty-tree collision and generated-artifact confusion
- tests/docs claiming stronger guarantees than the repo currently enforces

## Deliverables

- this plan
- a concise current-state audit summary in chat
- minimal validated code fixes for confirmed issues
- a list of deferred issues that were observed but intentionally not fixed in this pass
