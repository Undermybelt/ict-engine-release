# Regime Classifier R16 Handoff TODO

Live board for R16 analyze-live trace-only adapter wiring.

Goal: add the same optional regime consumer bundle trace surface to `analyze-live`, behind explicit flags, without changing decisions or invoking sidecars.

Scope: `analyze-live` CLI flag + read-only trace/report surface only. Do not stage unrelated drift.

---

## Routing / Process

- Primary route: `ict-engine-runtime`.
- Project router: `/Users/thrill3r/.hermes/routing/project-router.md` missing; no override.
- Repo entry map read: `AGENTS.md`.
- Domain reference read: `references/regime-classifier-sidecar-chain.md`.
- Process: verify dirty boundary -> narrow wiring -> cargo check -> CLI smoke -> commit only own files.

---

## Current Worktree Exclusions

Do not stage unrelated files unless explicitly asked:

- `src/auto_quant_command.rs`
- `src/validate_market_state_command.rs`
- `support/docs/plans/2026-05-09-regime-to-execution-mainline-audit-handoff-todo.md`

---

## Slice R16: Analyze-Live Trace-Only Wiring

### Done

- [x] Added explicit analyze-live flags:
  - `--regime-consumer-bundle <PATH>`
  - `--regime-consumer-bundle-strict`
- [x] Extended `AnalyzeLiveShellInput` and `AnalyzeLiveCommandInput`.
- [x] Loads `RegimeConsumerBundleAdapter` before live provider/network work when flag is present.
- [x] Appends read-only trace into `supporting.artifact_action_summary`.
- [x] Adds joined `regime_bundle_trace:*` line so compact outputs retain full trace context.
- [x] Does not alter execution branch, BBN posterior, path-ranker rows, or recommendation logic.
- [x] Does not invoke Python sidecars.
- [x] Missing/invalid strict fails early before live fetches.

### Verification

- [x] `cargo check` -> OK.
- [x] `cargo test --test regime_consumer_bundle_adapter` -> 8 passed.
- [x] Regression smoke for `analyze` valid bundle still emits:
  - `regime_bundle_status=loaded`
  - `regime_decision_state=single_label_99`
  - `regime_execution_tree_hint=accept_regime`
- [x] `analyze-live --regime-consumer-bundle missing --regime-consumer-bundle-strict` exits nonzero and contains `missing`.
- [x] `analyze-live --help` exposes `--regime-consumer-bundle`.

### Consumer Contract

- Default analyze-live behavior unchanged when flag is absent.
- Flag present + valid bundle -> trace/report context only.
- Flag present + missing/invalid + non-strict -> neutral trace/no decision change after provider work proceeds.
- Flag present + missing/invalid + strict -> early error before live data fetch.

---

## Immediate Next Slice: R17 BBN Soft-Evidence Adapter Spec/Tests

### Goal

Define and test a pure mapping from `consumer_hints.bbn_evidence_hint` into neutral/soft evidence records, still no posterior mutation unless later explicitly wired.

### Acceptance

- [ ] Add pure mapper function on adapter or BBN side.
- [ ] Tests for loaded/missing/invalid bundles.
- [ ] `single_label_99` strongest, `single_label_95` moderate, `label_set/transitional/unknown_abstain` neutral/guardrail.
- [ ] No mainline inference mutation.

---

## Commit Plan

Stage only R16 files, not unrelated drift.

Suggested add:

```bash
git add \
  support/docs/plans/2026-05-09-regime-classifier-r16-handoff-todo.md \
  src/main.rs \
  src/analyze_live_command.rs
```

Suggested commit:

```bash
git commit -m "feat: expose regime bundle trace on analyze-live"
```
