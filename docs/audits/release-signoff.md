# Release signoff

Date: final signoff after release-hygiene, help audit, agent surface polish, paired-data quality carry-through, release-closure surfaces, and release-closure deepening.

## Final verdict

Ready to tag and release.

No blocking release issues found.

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

## Residual non-blocking debt

1. `research-verdict` is compact, not a full experiment analytics engine yet
2. contamination heuristics are useful but still conservative
3. `evidence-quality-breakdown` still infers some terms from persisted policy/filter state rather than storing every raw intermediate
4. `src/main.rs` remains a structural debt hotspot, with plan already documented in `docs/plans/main-rs-extraction-plan.md`

## Release recommendation

Proceed with tag and release.
