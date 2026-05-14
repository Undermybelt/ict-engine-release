# Regime Classifier R15 Handoff TODO

Live board for R15 explicit-flag adapter trace wiring.

Goal: wire the optional regime consumer bundle adapter into one narrow read-only analyze report path behind explicit flags, without changing trading decisions.

Scope: `analyze` CLI flag + read-only trace/report surface only. Do not stage unrelated drift.

---

## Routing / Process

- Primary route: `ict-engine-runtime`.
- Project router: `/Users/thrill3r/.hermes/routing/project-router.md` missing; no override.
- Repo entry map read: `AGENTS.md`.
- Domain reference read: `references/regime-classifier-sidecar-chain.md`.
- Process: verify dirty boundary -> narrow wiring -> targeted tests -> cargo check -> CLI smoke -> commit only own files.

---

## Current Worktree Exclusions

Do not stage unrelated files unless explicitly asked:

- `src/auto_quant_command.rs`
- `src/validate_market_state_command.rs`
- `support/docs/plans/2026-05-09-regime-to-execution-mainline-audit-handoff-todo.md`

---

## Slice R15: Analyze Trace-Only Wiring

### Done

- [x] Confirmed prior dirty `main.rs` / `analyze_output.rs` / `execution_tree.rs` changes were gone before editing.
- [x] Added explicit analyze flags:
  - `--regime-consumer-bundle <PATH>`
  - `--regime-consumer-bundle-strict`
- [x] Extended `analyze_command` signature with optional bundle path + strict mode.
- [x] Loads `RegimeConsumerBundleAdapter` only when flag is present.
- [x] Appends read-only trace into `supporting.artifact_action_summary`.
- [x] Adds joined `regime_bundle_trace:*` line so compact outputs retain full trace context.
- [x] Does not alter execution branch, BBN posterior, path-ranker rows, or recommendation logic.
- [x] Does not invoke Python sidecars.
- [x] Missing/invalid non-strict remains adapter-neutral.
- [x] Missing/invalid strict fails early through adapter error.

### Verification

- [x] `cargo test --test regime_consumer_bundle_adapter` -> 8 passed.
- [x] `cargo check` -> OK.
- [x] CLI smoke valid bundle:
  - `cargo run -- analyze --symbol DEMO --demo --state-dir /tmp/... --compact --regime-consumer-bundle /tmp/.../regime_consumer_bundle.json`
  - Output contains:
    - `regime_bundle_status=loaded`
    - `regime_decision_state=single_label_99`
    - `regime_execution_tree_hint=accept_regime`
- [x] CLI smoke strict missing bundle:
  - `--regime-consumer-bundle /tmp/missing.json --regime-consumer-bundle-strict`
  - exit code nonzero; stderr contains `missing`.

### Consumer Contract

- Default analyze behavior unchanged when flag is absent.
- Flag present + valid bundle -> trace/report context only.
- Flag present + missing/invalid + non-strict -> neutral trace/no decision change.
- Flag present + missing/invalid + strict -> early error.

---

## Immediate Next Slice: R16 Analyze-Live Trace Wiring or BBN Soft-Evidence Adapter

### Option A: Analyze-live trace-only wiring

- Add same optional flags to `analyze-live`.
- Append trace/report context only.
- No decision behavior changes.

### Option B: BBN soft-evidence adapter spec/tests

- Define explicit mapping from `bbn_evidence_hint` to neutral/soft nodes.
- Tests first.
- No posterior mutation until gates are explicit.

---

## Commit Plan

Stage only R15 files, not unrelated drift.

Suggested add:

```bash
git add \
  support/docs/plans/2026-05-09-regime-classifier-r15-handoff-todo.md \
  src/main.rs \
  src/analyze_command.rs
```

Suggested commit:

```bash
git commit -m "feat: expose regime bundle trace on analyze"
```
