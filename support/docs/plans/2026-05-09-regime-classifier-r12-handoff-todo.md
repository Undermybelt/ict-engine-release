# Regime Classifier R12 Handoff TODO

Live board for R12 mainline optional adapter spec.

Goal: define a non-invasive adapter contract for mainline consumers to read `regime_consumer_bundle.json` if explicitly provided, without changing default runtime behavior.

Scope: support/docs/spec only. No Rust implementation in this slice.

---

## Routing / Process

- Primary route: `ict-engine-runtime`.
- Project router: `/Users/thrill3r/.hermes/routing/project-router.md` missing; no override.
- Repo entry map read: `AGENTS.md`.
- Domain reference read: `references/regime-classifier-sidecar-chain.md`.
- Process: spec first, docs-only, verify no Rust touched by this slice.

---

## Current Worktree Exclusions

Do not stage unrelated files unless explicitly asked:

- `src/auto_quant_command.rs`
- `src/validate_market_state_command.rs`
- `support/docs/plans/2026-05-09-regime-to-execution-mainline-audit-handoff-todo.md`

---

## Slice R12: Mainline Optional Adapter Spec

### Done

- [x] Added spec: `support/docs/regime-consumer-bundle-mainline-adapter-spec.md`.
- [x] Defined CLI/config field names:
  - `--regime-consumer-bundle <PATH>`
  - `--regime-consumer-bundle-strict`
  - config key `regime_consumer_bundle.{path,strict,enabled}`
- [x] Defined precedence:
  - CLI path
  - config path
  - unset no-op
- [x] Defined execution tree / BBN / path-ranker mapping table.
- [x] Preserved zero-config default behavior.
- [x] Defined rollback/no-op behavior when bundle path is missing or invalid.
- [x] Defined strict vs non-strict behavior.
- [x] Defined safety/no-pollution rules.
- [x] Added future Rust shape and validation checklist.

### Verification

- [x] Docs-only slice; no Rust files touched by R12.
- [ ] Check staged files before commit.

### Consumer Contract

- Mainline remains unchanged unless a future implementation explicitly adds adapter support.
- Sidecar remains opt-in.
- Missing/invalid bundle in non-strict mode is neutral evidence, not failure.
- Strict mode may fail early before mutation.

---

## Immediate Next Slice: R13 Optional Adapter Tests / Stub Design

### Goal

If implementation is desired, create tests/stub around optional adapter loading without wiring it into runtime behavior yet.

### Acceptance

- [ ] Add adapter module with disabled/no-op default.
- [ ] Unit tests for unset, valid, missing non-strict, missing strict, invalid schema.
- [ ] No mainline command behavior changed unless flag is wired explicitly.
- [ ] No sidecar execution from Rust.

---

## Commit Plan

Stage only R12 docs, not unrelated Rust drift.

Suggested add:

```bash
git add \
  support/docs/plans/2026-05-09-regime-classifier-r12-handoff-todo.md \
  support/docs/regime-consumer-bundle-mainline-adapter-spec.md
```

Suggested commit:

```bash
git commit -m "docs: specify regime consumer bundle adapter"
```
