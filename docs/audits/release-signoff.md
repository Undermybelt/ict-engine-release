# Release signoff

Date: final signoff after release-hygiene, help audit, agent surface polish, paired-data quality carry-through, release-closure surfaces, and release-closure deepening.

## Final verdict

Ready to tag and release from a private release mirror.

No blocking release issues found in the codebase.

## Important release routing decision

Status note (2026-04-24)

The historical oversized-artifact blocker on the source repo has now been cleared by history rewrite.
Normal pushes to the source repo are available again.

Mirror release remains a valid release transport surface for clean tree-state publishing, but it is no longer required by an unsolved source-repo history blocker.

## Signoff checklist

### Build and test
- [x] `cargo check`
- [x] `cargo test`
- [x] worktree clean before signoff

### CLI quality
- [x] root help exposes `--version`
- [x] mechanical help audit passes
- [x] all current subcommands have option descriptions
- [x] new release-facing subcommands covered by audit

### Portability
- [x] no local path dependency in `Cargo.toml`
- [x] no release-blocking absolute-path hardcoding in source command paths
- [x] demo mode exists for first-run verification

### Output surfaces
- [x] `analyze --agent` exposes structured `next_step`
- [x] `workflow-status --agent` exposes structured `next_step`
- [x] `analyze --human` provides readable summary output
- [x] `workflow-status --human` provides readable summary output
- [x] local-path redaction exists and is regression-tested

### Release closure / closed loop
- [x] `research-verdict` exists
- [x] `research-verdict` emits contamination signal
- [x] `evidence-quality-breakdown` exists
- [x] paired-market quality report is preserved in debug path

### Repo hygiene
- [x] runtime artifacts ignored
- [x] no tracked `state*`, `__pycache__`, `.DS_Store`, or `tmp_cycle_seed_spec.json`
- [x] LICENSE present
- [x] `Cargo.toml` has `license`, `repository`, `authors`

## Commands executed for signoff

```bash
cargo check
cargo test
python3 scripts/help_audit.py
cargo run --quiet -- research-verdict --symbol DEMO --state-dir state
cargo run --quiet -- evidence-quality-breakdown --symbol DEMO --state-dir state
cargo run --quiet -- workflow-status --symbol DEMO --state-dir state --agent
cargo run --quiet -- analyze --symbol DEMO --demo --agent
```

## Decisive outcomes

### Mechanical help audit
- status: `pass`
- root version flag: present
- audited subcommands: `22`
- commands with missing option descriptions: `0`

### `research-verdict` smoke
- emitted compact closure verdict successfully
- emitted contamination fields successfully

### `evidence-quality-breakdown` smoke
- emitted component breakdown successfully
- emitted hard/neutralized gaps successfully

### `workflow-status --agent` smoke
- emitted structured `next_step`
- emitted `user_input_required=true` when historical-data selection gate is active

### `analyze --agent` smoke
- emitted `decision_hint_raw`
- emitted `decision_summary`
- emitted structured `next_step`

## Source repo history status

Previously oversized historical state artifacts blocked normal source-repo pushes.
That blocker was cleared on `2026-04-24` by removing generated `state*` artifacts from history.

## Residual non-blocking debt

Status note (2026-04-24)

Item 4 below is stale after the `main.rs` runtime-hotspot extraction line landed in commits `8ce1024` and `3e45254`.
Current post-`main.rs` debt inventory now lives in `docs/plans/2026-04-24-post-main-debt-inventory.md`.

1. public experiment wrappers are still preview-grade rather than a stable packaged interface
2. some experiment flows still assume a Tomac-style cleaned-data layout unless env vars are overridden

## Release recommendation

Proceed with source-repo development truth plus mirror tag/release for `v0.1.0`.
