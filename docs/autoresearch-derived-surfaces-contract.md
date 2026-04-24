# Autoresearch Derived Surfaces Contract

Goal: define what is canonical truth, what is derived convenience output, and how humans or agents should read and maintain the `factor-autoresearch` surfaces.

This document is intentionally narrower than `docs/research-system-map.md`. It only covers the autoresearch session files and the new human-readable derived outputs.

## 1. Canonical vs derived

The autoresearch surfaces split into two classes:

| Class | Surface | Path | Trust level | Primary use |
|---|---|---|---|---|
| canonical | attempts | `<state_dir>/<SYMBOL>/factor_autoresearch_attempts.json` | authoritative | per-attempt truth |
| canonical | sessions | `<state_dir>/<SYMBOL>/factor_autoresearch_sessions.json` | authoritative | session summary truth |
| canonical | live snapshot | `<state_dir>/<SYMBOL>/factor_autoresearch_live.json` | authoritative | running/interrupted detection |
| canonical | final summary | `<state_dir>/<SYMBOL>/factor_autoresearch_final.json` | authoritative when present | completed-session summary |
| derived | status surface | CLI: `factor-autoresearch-status` | derived from canonical state | machine-readable rollup |
| derived | experiment ledger | `<state_dir>/<SYMBOL>/experiments.tsv` | convenience only | grep/diff/tail-friendly scan |
| derived | retrospective | `<state_dir>/<SYMBOL>/factor_autoresearch_retrospective.md` | convenience only | human-readable session recap |

Core rule:

- If a canonical JSON artifact and a derived surface disagree, canonical JSON wins.

## 2. What each canonical file means

### `factor_autoresearch_attempts.json`

This is the finest-grained source of truth.

Each record represents one attempt and carries:

- candidate mutation spec
- evaluation
- keep/discard decision
- score delta
- branch summary

If you need to reconstruct the session history exactly, start here.

### `factor_autoresearch_sessions.json`

This is the session-level rollup.

Use it for:

- session identity
- status at the session level
- kept/discarded counts
- baseline mutation lineage

### `factor_autoresearch_live.json`

This is the running-state snapshot.

Use it for:

- whether a run is currently active
- interrupted vs fresh-running inference
- current iteration / latest attempt / current candidate spec

### `factor_autoresearch_final.json`

This is the completed-session summary artifact.

Use it when:

- the session finished normally
- you want the last completed summary quickly

Do not assume a session is reliably completed unless the final artifact and the rest of the state agree.

## 3. What the derived files are for

### `factor-autoresearch-status`

This is the main machine-facing rollup.

Use it first when you want:

- effective session status
- interrupted/completed classification
- decision counts
- cluster scoreboard
- best attempt

It is the preferred starting point for agents because it is thinner than reading all raw JSON by hand.

### `experiments.tsv`

This is a convenience ledger generated from canonical attempt truth.

Use it for:

- quick terminal inspection
- `grep`
- `tail`
- `diff`
- lightweight spreadsheet import

Contract:

- one attempt maps to one row
- row order should follow chronological attempt order
- text fields may be flattened/sanitized for TSV safety
- this file must remain re-generatable from canonical JSON

Do not use it for:

- resume logic
- authoritative counting
- mutation replay
- anything that cannot tolerate lossy text flattening

### `factor_autoresearch_retrospective.md`

This is a convenience summary for humans.

Use it for:

- reading the session arc quickly
- spotting concentration in failure tags
- seeing the score trajectory
- checking the best attempt and next-focus hints

Contract:

- it is derived from the aggregated status surface and canonical JSON
- it is narrative convenience, not authoritative truth
- every claim in it should be traceable back to canonical artifacts

Do not use it for:

- downstream automation that requires precise field semantics
- authoritative completion detection
- canonical persistence

## 4. Correct read order

### If you want current session truth

1. `cargo run -- factor-autoresearch-status --symbol <SYM> --state-dir <dir> --latest-only`
2. `<state_dir>/<SYM>/factor_autoresearch_final.json`
3. `<state_dir>/<SYM>/factor_autoresearch_attempts.json`

Then, only if you want convenience surfaces:

4. `<state_dir>/<SYM>/experiments.tsv`
5. `<state_dir>/<SYM>/factor_autoresearch_retrospective.md`

### If you want exact machine truth

Stop at the canonical JSON layer.

Do not begin with:

- `experiments.tsv`
- retrospective markdown

### If you want a quick human scan

Read:

1. `factor-autoresearch-status --latest-only`
2. retrospective markdown
3. experiments TSV

Then fall back to canonical JSON for any claim that matters.

## 5. Sync and regeneration rules

Derived surfaces should follow these rules:

- never become the only copy of information
- never be manually edited as if they were canonical state
- be safe to delete and regenerate from canonical JSON
- stay small enough to inspect without loading full raw state

Contributor rule:

- if a new field matters for exact replay or authoritative evaluation, add it to canonical JSON first
- only then expose it through status / TSV / retrospective

## 6. Integrity rules

### Source-of-truth rule

- `factor_autoresearch_attempts.json` remains the ultimate per-attempt truth
- `factor_autoresearch_sessions.json`, `factor_autoresearch_live.json`, and `factor_autoresearch_final.json` remain canonical session truth
- derived outputs must never silently replace these files in documentation or tooling

### Drift rule

If a user or agent sees disagreement:

1. trust canonical JSON
2. regenerate the derived surfaces
3. only then investigate rendering or sync bugs

### Scope rule

`experiments.tsv` and retrospective are session-analysis aids, not a new execution engine.

That means:

- no resume semantics from TSV
- no keep/discard authority from Markdown
- no CLI contract that implies TSV or retrospective are canonical

### Session-scope rule

Retrospective content should remain clearly scoped.

If future work introduces multi-session retrospective views, do not reuse a single-session heading or summary shape without explicitly labeling it as a multi-session aggregate.

## 7. Contributor checklist for future changes

When extending autoresearch derived surfaces:

1. add or update the canonical field in typed state first
2. update the status rollup if the field belongs in the machine-facing summary
3. update TSV or retrospective only if the field improves human/operator utility
4. keep row/section semantics stable and documented
5. add focused tests for any new rendering or aggregation rule
6. update this contract if the trust boundary changes

## 8. Non-goals

This contract does not define:

- optimizer heuristics
- keep/discard policy
- warning heuristics for suspicious uplift
- future CLI flags that do not exist yet

Those belong in implementation plans or feature-specific docs.
