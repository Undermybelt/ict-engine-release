# Regime Classifier R14 Handoff TODO

Live board for R14 read-only adapter trace surface.

Goal: add a read-only trace/report surface to the optional regime consumer bundle adapter without wiring it into mainline decisions or touching active dirty command/report files.

Scope: adapter module + adapter tests + docs. Do not stage unrelated Rust drift.

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

- `src/application/orchestration/execution_tree.rs`
- `src/application/reporting/analyze_output.rs`
- `src/auto_quant_command.rs`
- `src/main.rs`
- `src/validate_market_state_command.rs`
- `docs/plans/2026-05-09-regime-to-execution-mainline-audit-handoff-todo.md`

---

## Slice R14: Adapter Read-Only Trace Surface

### Done

- [x] Detected dirty mainline files; avoided wiring into command/report paths.
- [x] Wrote failing tests for `trace_entries` first.
- [x] RED verified: no method named `trace_entries`.
- [x] Added `RegimeConsumerBundleAdapter::trace_entries(path)`.
- [x] Added compact trace values:
  - `regime_bundle_status=disabled|loaded|missing|invalid`
  - `regime_bundle_path=<PATH>`
  - `regime_bundle_error=<COMPACT>`
  - `regime_decision_state=<STATE>`
  - `regime_trade_usable=<BOOL>`
  - `regime_final_label=<LABEL>`
  - `regime_execution_tree_hint=accept_regime|transition_guardrail|unknown_abstain`
- [x] Trace helper is read-only and side-effect free.
- [x] No mainline command behavior changed.
- [x] No sidecar execution from Rust.

### Verification

- [x] RED:
  - `cargo test --test regime_consumer_bundle_adapter` -> missing `trace_entries`.
- [x] Target GREEN:
  - `cargo test --test regime_consumer_bundle_adapter` -> 8 passed.
- [x] `cargo check` -> OK.

### Consumer Contract

- Future mainline wiring can append `adapter.trace_entries(path)` to trace/report surfaces.
- This helper does not alter execution branch, BBN posterior, path-ranker rows, or trade recommendation.
- Missing/invalid bundle remains visible as trace-only neutral evidence.

---

## Immediate Next Slice: R15 Wire Adapter Trace Behind Explicit Flag

### Goal

Wire adapter trace into one narrow read-only report path behind explicit flag, still not changing decisions.

### Acceptance

- [ ] Wait for dirty `main.rs` / `analyze_output.rs` ownership to clear or coordinate.
- [ ] Add explicit CLI flag only.
- [ ] Load adapter once.
- [ ] Append trace entries to report/trace.
- [ ] No decision behavior changes.
- [ ] No sidecar execution from Rust.

---

## Commit Plan

Stage only R14 files, not unrelated Rust drift.

Suggested add:

```bash
git add \
  docs/plans/2026-05-09-regime-classifier-r14-handoff-todo.md \
  src/application/regime/consumer_bundle_adapter.rs \
  tests/regime_consumer_bundle_adapter.rs
```

Suggested commit:

```bash
git commit -m "feat: add regime bundle adapter trace entries"
```
