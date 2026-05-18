# Board A/B Cleanup Retention Plan

Updated: `2026-05-12 22:43:33 +0800`

Goal: make the A/B documentation clean without losing experiment results,
creating dangling references, or leaving hidden runtime dependencies on archival
append-only logs.

## Decision

The old May 10 A/B logs are archival cleanup targets, not runtime or strategy
dependencies. Immediate deletion is still not safe because references must be
classified before removal.

Reasons:

- `support/docs/plans/2026-05-10-actionable-regime-confidence-todo.md` is about `52,375` lines / `6.8M`.
- `support/docs/plans/2026-05-10-regime-conditional-auto-quant-profitability-todo.md` is about `47,952` lines / `5.2M`.
- release handoff docs, historical packets, and many experiment
  support/scripts/checklists still hard-reference those exact paths.
- Current active roots and Cargo/Rust owners still exist, including `223253`, `223410`, provider-authority work, branch-path work, and SMT tests.
- Some historical scripts use the old board paths as evidence inputs. Deleting now would create dangling references and would make old audit packets harder to replay.

Therefore the cleanup starts with extraction and reference migration, not
deletion. Runtime fixes must be promoted into code, configs, state artifacts,
candidate packs, or admission targets before any prose is treated as support.

## Context Budget Policy

The current failure mode is not only disk size. It is repeated context replay of
two old append-only logs that now total more than 100,000 lines.

Default agent behavior:

- Read compact Board A/B current-state docs first.
- Use `ict-engine` runtime/state surfaces for decisions; use archival logs only
  for targeted historical lookup.
- Store new detailed evidence in run-root packets, not in board prose.
- Keep board updates as current gate decisions and artifact pointers.
- Treat another broad append-only audit as a regression unless it replaces
  context load with a compact packet.

## New Compact Authority Files

- Board A current state: `support/docs/plans/2026-05-12-board-a-regime-state-current.md`
- Board B current state: `support/docs/plans/2026-05-12-board-b-profit-factor-current.md`
- Factor candidate ingestion instructions: `support/docs/plans/2026-05-12-factor-candidate-ingestion-instructions.md`

These files are the current read targets. The old append-only logs remain
retained only as archival cleanup inputs until parity and reference migration
pass.

## Retention Classes

Keep:

- compact current A/B docs;
- compact evidence packets under each run root `materials/`, `summaries/`, `checks/`, and stable JSON/CSV/JSONL packets;
- provider authority manifests and per-regime stats;
- Pre-Bayes/filter policy artifacts, BBN likelihood/calibration artifacts, CatBoost/path-ranker artifacts, execution-tree traces, and feedback/update learning packets;
- active roots, active owner claims, and roots referenced as the only support for a live gate;
- artifacts needed to reproduce fail-closed claims, especially provider failure, branch-path loss, and insufficient sample-density evidence.

Eligible for later deletion after extraction:

- Auto-Quant workspaces under run roots;
- `.deps` directories;
- duplicated provider dumps already represented by a provider authority manifest;
- temporary `state_*` directories that are not referenced by compact packets;
- generated pycache/build scratch;
- exploratory roots whose negative evidence has been classified into market/factor, infrastructure, or chain-contract evidence.

Never delete in this slice:

- `141000` or any owner-protected root;
- active `223253`, `223410`, `223650`, active Cargo/Rust target output, or roots without terminal closeout;
- archival Board A/B logs before reference migration and parity;
- runtime source files or scripts modified by other agents.

## Dry-Run Deletion Rules

A candidate path may only appear in a real deletion command when all are true:

- `active_owner=false`
- `referenced_by_current_docs=false`
- `referenced_by_scripts=false`
- `extraction_packet_exists=true`
- `parity_replay_pass=true`
- `local_raw_dependency=false`
- `not_sole_evidence_for_live_gate=true`

Current deletion status:

- `deletion_allowed=false`
- `dry_run_allowed=true`
- `reference_migration_required=true`
- `parity_required=true`
- `old_board_archive_or_delete_allowed=false`

## Next Implementation Steps

1. Build `support/docs/experiments/actionable-regime-confidence/evidence-retention-ledger.csv`.
2. Add one row per run root with owner status, active/closed state, retention class, extracted packet path, bytes, and cleanup action.
3. Build a reference map for old A/B board paths across docs, scripts, and experiment packets.
4. Update stable docs and agent entrypoints to point agents to compact A/B docs while keeping old logs as archived source logs.
5. Generate a no-delete dry-run report listing delete/compress/keep decisions.
6. Run parity readbacks from compact docs plus retained packets.
7. Only after parity passes, replace old board paths with archive pointers or move old boards under an archive path in one reviewed slice.
