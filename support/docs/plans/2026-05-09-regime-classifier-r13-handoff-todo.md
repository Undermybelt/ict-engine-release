# Regime Classifier R13 Handoff TODO

Live board for R13 optional adapter tests / stub design.

Goal: create a Rust adapter module with disabled/no-op default and unit tests, without wiring it into mainline command behavior.

Scope: isolated adapter module + tests + docs. Do not touch unrelated Rust drift.

---

## Routing / Process

- Primary route: `ict-engine-runtime`.
- Project router: `/Users/thrill3r/.hermes/routing/project-router.md` missing; no override.
- Repo entry map read: `AGENTS.md`.
- Domain reference read: `references/regime-classifier-sidecar-chain.md`.
- Process: TDD RED -> GREEN -> targeted Rust test -> cargo check -> commit only own files.

---

## Current Worktree Exclusions

Do not stage unrelated files unless explicitly asked:

- `src/auto_quant_command.rs`
- `src/validate_market_state_command.rs`
- `src/main.rs`
- `support/docs/plans/2026-05-09-regime-to-execution-mainline-audit-handoff-todo.md`

---

## Slice R13: Optional Adapter Stub

### Done

- [x] Wrote failing tests first: `tests/regime_consumer_bundle_adapter.rs`.
- [x] RED verified: unresolved import `consumer_bundle_adapter`.
- [x] Added adapter module: `src/application/regime/consumer_bundle_adapter.rs`.
- [x] Registered module in `src/application/regime/mod.rs`.
- [x] Disabled/no-op default implemented.
- [x] Unit coverage:
  - unset path -> disabled/no-op
  - valid bundle -> loaded known fields
  - missing non-strict -> neutral no-op
  - missing strict -> error
  - invalid schema non-strict -> neutral no-op
  - invalid schema strict -> error
- [x] No mainline command behavior wired.
- [x] No sidecar execution from Rust.
- [x] Adapter reads only explicit path.

### Verification

- [x] RED:
  - `cargo test --test regime_consumer_bundle_adapter` -> unresolved import.
- [x] Target GREEN:
  - `cargo test --test regime_consumer_bundle_adapter` -> 6 passed.
- [x] `cargo check` -> OK.

### Consumer Contract

- `RegimeConsumerBundleAdapter::load_optional(None, false)` returns disabled no-op.
- `load_optional(Some(path), false)` never fails for missing/invalid bundle; returns neutral adapter.
- `load_optional(Some(path), true)` fails on missing/invalid bundle.
- Valid bundle maps only known fields.
- No commands consume this adapter yet.

---

## Immediate Next Slice: R14 Wire Optional Adapter into One Read-Only Consumer Trace

### Goal

If desired, wire adapter into one narrow read-only trace/report path behind explicit flag, without changing execution behavior.

### Acceptance

- [ ] Add CLI flag only to selected command.
- [ ] Load adapter once.
- [ ] Print/report adapter status.
- [ ] Do not alter decisions yet.
- [ ] No sidecar execution from Rust.

---

## Commit Plan

Stage only R13 files, not unrelated Rust drift.

Suggested add:

```bash
git add \
  support/docs/plans/2026-05-09-regime-classifier-r13-handoff-todo.md \
  src/application/regime/consumer_bundle_adapter.rs \
  src/application/regime/mod.rs \
  tests/regime_consumer_bundle_adapter.rs
```

Suggested commit:

```bash
git commit -m "feat: add regime consumer bundle adapter stub"
```
