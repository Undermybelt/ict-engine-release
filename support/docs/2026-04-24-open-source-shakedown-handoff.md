# Open-Source Shakedown Handoff · 2026-04-24

This note is for the next human contributor or agent picking up the open-source shakedown work on `ict-engine`. It assumes you have read `README.md` and `support/docs/agent-first-runbook.md`.

Its only job is to tell you, in order:

1. what the current baseline is
2. what was just closed
3. what is explicitly not closed and should be picked up next
4. which existing documents to read before doing that next step

## Current baseline

Branch: `green-baseline`

Head commits (most recent first):

- `16c6e54 chore: align release metadata and contributor baseline with CI truth`
- `88e388c feat: land autoresearch derived-surface contract and retrospective`

Verified green on HEAD:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`

Worktree: clean (no tracked modifications, no unexpected untracked files).

CI truth (`.github/workflows/ci.yml`):

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`

All three are hard gates. `README.md` now states this truthfully; treat it as the single source of truth for the contributor pre-PR checklist.

## What was closed in this shakedown pass

### Autoresearch derived surfaces (commit `88e388c`)

- introduced `src/application/factor_lifecycle/autoresearch_surface.rs` as the module that owns `experiments.tsv` and `factor_autoresearch_retrospective.md` as derived artifacts
- wired derived-surface sync into the `factor-autoresearch` dispatch with a warning-only failure path, so a sync failure never breaks the canonical autoresearch run
- moved `factor-autoresearch-status` aggregation out of `main.rs` into a reusable pure builder and routed the CLI through it
- sanitized retrospective markdown free-text fields and fixed multi-session retrospective header semantics
- added the derived-surface contract (`support/docs/autoresearch-derived-surfaces-contract.md`) and state-transition notes (`support/docs/autoresearch-state-transitions.md`)
- captured the derived-surfaces review in `support/docs/audits/2026-04-23-autoresearch-derived-surfaces-review.md` and the external pattern intake under `support/docs/external/`

### Release metadata and contributor baseline drift (commit `16c6e54`)

- `Cargo.toml` `repository` now points at `https://github.com/Undermybelt/ict-engine-release`, which matches the actual release transport described in `support/docs/release-mirror-runbook.md`
- `README.md` contributor baseline now matches `.github/workflows/ci.yml` instead of calling clippy advisory
- recorded the versioned plan at `support/docs/plans/2026-04-23-open-source-shakedown-plan.md` and the follow-up audit at `support/docs/audits/2026-04-24-open-source-shakedown-follow-up.md`

### Older audit items that were verified as no longer live

These are called out here so you don't re-open them without cause:

- `analyze` output-format conflict guard (human vs explicit json) is already in place
- `factor-autoresearch-status` empty-state now returns `status = "no_autoresearch_state"` with empty sessions / attempts / null live snapshot
- `research-verdict` with zero research runs correctly returns `stop_or_continue = "bootstrap_required"` and `current_bottleneck = "no_research_runs"`
- human next-step rendering no longer leaks raw `ask-user:` syntax on already-covered paths
- `factor-backtest --human` is no longer a single serialized dump

## What is explicitly not closed

### 1. Isolated stateful smoke run under `/tmp`

Not completed in this pass. The commands were prepared but the run itself was cancelled.

Recommended next run (do this first):

```bash
TMP_STATE="$(mktemp -d /tmp/ict-engine-shakedown.XXXXXX)"
echo "STATE_DIR=$TMP_STATE"

cargo run --quiet -- analyze --symbol DEMO --demo --human --state-dir "$TMP_STATE" \
  > "$TMP_STATE/analyze-human.txt"

cargo run --quiet -- analyze --symbol DEMO --demo --agent --state-dir "$TMP_STATE" \
  > "$TMP_STATE/analyze-agent.json"

cargo run --quiet -- workflow-status --symbol DEMO --state-dir "$TMP_STATE" --agent \
  > "$TMP_STATE/workflow-agent.json"
```

What to check on the output:

- `analyze --human` head reads like natural language, no raw `ask-user:` tokens
- `analyze --agent` JSON has `next_step` and `next_command` fields populated with sensible values
- `workflow-status --agent` JSON has `next_step` and `current_focus_phase` populated
- `$TMP_STATE` is the only place anything was written; the repo `state/` is untouched

If any of those fail, the correct fix site is almost always in:

- `src/application/reporting/analyze_output.rs`
- `src/application/reporting/human_report.rs`
- `src/application/orchestration/workflow_status.rs`

Not in the autoresearch surface.

### 2. Dirty tree re-growth risk

The tree is clean as of this handoff. Do not let it grow broad again before the next release checkpoint. Prefer small, single-topic commits that each pass the three CI gates locally.

### 3. Public wrapper data-root assumption must not regress

This is now an explicit invariant:

- public experiment wrappers must not assume a local Tomac cleaned-data layout exists
- wrappers should expose resolved config first
- wrappers should require explicit data readiness before `--run`
- when in doubt, prefer `--show-config` and explicit `--data-root` over hidden local path guessing

Current expected behavior:

- `python3 support/scripts/search_local.py --show-config`
- `python3 support/scripts/search_cluster.py --show-config`
- `python3 support/scripts/evaluate_bottleneck.py --show-config`
- `--run` refuses execution when `cleaned_data_ready=false`

Do not undo this by reintroducing "best effort" local path guessing as silent runtime behavior.

### 4. Hermes routing convention mismatch

The repo has `.hermes/`, but the routing files expected by the user-level AGENTS.md convention (`.hermes/routing/skill-router`, `project-router`, etc.) are not present in this worktree.

This is a project-process gap, not a code bug. It only matters if you are following the routing convention from outside this repo. Do not try to fix it in `ict-engine` source. If it needs fixing, it should be fixed in the user-level Hermes setup, not here.

### 5. Release to the private mirror

The two commits on `green-baseline` have not been pushed to `Undermybelt/ict-engine-release`. Follow `support/docs/release-mirror-runbook.md` when you are ready. Do not push source commits to a public source repo as part of a release flow.

## Suggested order for the next session

1. If you touch public experiment wrappers, preserve the `--show-config` + explicit data-readiness gate behavior.
2. If any CLI surface regresses, fix it at the rendering layer, not in unrelated research/autoresearch state code.
3. Keep `support/docs/release-notes-draft.md` and `support/docs/release-mirror-runbook.md` aligned with actual release transport.
4. Treat source repo history hygiene as a protected invariant now that oversized generated artifacts were removed.

## Reading list (in order)

- `README.md`
- `support/docs/agent-first-runbook.md`
- `support/docs/first-run.md`
- `support/docs/smoke-acceptance.md`
- `support/docs/plans/2026-04-23-open-source-shakedown-plan.md`
- `support/docs/audits/2026-04-24-open-source-shakedown-follow-up.md`
- `support/docs/autoresearch-derived-surfaces-contract.md`
- `support/docs/autoresearch-state-transitions.md`
- `support/docs/release-mirror-runbook.md`
- `support/docs/release-notes-draft.md`

## Ground rules for the next pass

- Do not weaken or delete existing tests to make a new change pass.
- Do not introduce new clippy warnings; CI is strict.
- Prefer minimal upstream fixes over downstream workarounds.
- If you touch a user-visible surface (CLI flag, JSON field, human text), update the corresponding doc in the same commit.
- State writes belong under a caller-provided `--state-dir`. Do not write into the repo `state/` directory from tests or smoke runs.
- Do not reintroduce public wrapper assumptions that only work on the maintainer's workstation layout.
