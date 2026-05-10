# Regime Classifier R10 Handoff TODO

Live board for R10 consumer bundle / manifest sidecar.

Goal: package R2-R9 artifacts into one token-friendly manifest that a human, BBN, path-ranker, or execution tree can consume without scanning every intermediate JSON.

Scope: sidecar scripts/tests/docs only. Do not touch unrelated Rust drift.

---

## Routing / Process

- Primary route: `ict-engine-runtime`.
- Project router: `/Users/thrill3r/.hermes/routing/project-router.md` missing; no override.
- Repo entry map read: `AGENTS.md`.
- Domain reference read: `references/regime-classifier-sidecar-chain.md`.
- Process: TDD RED -> GREEN -> full research suite -> CLI smoke -> commit only own files.

---

## Current Worktree Exclusions

Do not stage unrelated files unless explicitly asked:

- `src/application/orchestration/execution_tree.rs`
- `src/auto_quant_command.rs`
- `src/validate_market_state_command.rs`
- `docs/plans/2026-05-09-regime-to-execution-mainline-audit-handoff-todo.md`

---

## Slice R10: Consumer Bundle / Manifest

### Done

- [x] Wrote failing tests first: `scripts/research/tests/test_regime_consumer_bundle.py`.
- [x] RED verified: `ModuleNotFoundError: No module named 'regime_consumer_bundle'`.
- [x] Implemented: `scripts/research/regime_consumer_bundle.py`.
- [x] Zero-config pure-Python sidecar.
- [x] Reads any subset of R2-R9 artifact paths.
- [x] Emits compact `regime_consumer_bundle.json`.
- [x] Includes latest decision, artifact paths, schema versions, consumer hints, and missing-artifact list.
- [x] Hot-plug input via repeated `--include-artifact key=path`.
- [x] Optional auto-discovery via `--artifact-dir` using default artifact names.
- [x] Missing artifacts are reported, not fatal.
- [x] No runtime mutation.

### Verification

- [x] RED:
  - `python3 -m unittest scripts/research/tests/test_regime_consumer_bundle.py -v` -> missing module.
- [x] Target GREEN:
  - `python3 -m unittest scripts/research/tests/test_regime_consumer_bundle.py -v` -> 4 OK.
- [x] Full research suite:
  - `python3 -m unittest discover -s scripts/research/tests -p 'test_*.py'` -> 88 OK.
- [x] CLI smoke R2 -> R3(+aux) -> R5 -> R6 -> R7 -> R8 -> R9 -> R10.
  - Output: `artifact_count=7`, `missing_artifacts=[]`, `decision_state=single_label_99`, `trade_usable=true`, `final_label=primary::TrendExpansion`, `execution_tree_hint=accept_regime`.
  - Bundle size: 2963 bytes.

### CLI Floor

```bash
python3 scripts/research/regime_consumer_bundle.py \
  --artifact-dir /tmp/ict-regime \
  --output-json /tmp/ict-regime/regime_consumer_bundle.json
```

Hot-plug explicit mode:

```bash
python3 scripts/research/regime_consumer_bundle.py \
  --include-artifact decision=/tmp/ict-regime/regime_high_confidence_decision.json \
  --include-artifact transition_governor=/tmp/ict-regime/regime_transition_governor_report.json \
  --output-json /tmp/ict-regime/regime_consumer_bundle.json
```

### Consumer Contract

- Optional inputs:
  - `--artifact-dir` for default R2-R9 filenames.
  - repeated `--include-artifact key=path` for hot-plug user-selected artifacts.
- Consumer output fields:
  - `latest_decision`
  - `consumer_hints`
  - `artifacts`
  - `missing_artifacts`
  - `consumer_contract`
- User can opt in by invoking the script; main runtime remains unchanged.
- User can ignore this sidecar with zero impact.
- Outputs are explicit paths only; no repo-root state writes.

---

## Immediate Next Slice: R11 Sidecar Chain README / One-Command Runner

### Create

- [ ] `scripts/research/regime_sidecar_pipeline.py`
- [ ] `scripts/research/tests/test_regime_sidecar_pipeline.py`
- [ ] `docs/regime-classifier-sidecar-chain.md`

### Goal

Give consumers one zero-config command to run R2-R10 in a temp/output directory, with optional OHLCV/aux/truth inputs and no repo pollution.

### Acceptance

- [ ] One command can run R2-R10 when provided OHLCV.
- [ ] If no OHLCV is provided, emits a helpful input contract and exits nonzero without creating repo-root state.
- [ ] Supports `--output-dir`, `--label-prefix`, `--auxiliary-evidence`, `--truth`.
- [ ] Prints compact final decision and bundle path.
- [ ] Tests cover success and missing-input contract.

---

## Commit Plan

Stage only R10 sidecar files and docs, not unrelated Rust drift.

Suggested add:

```bash
git add \
  docs/plans/2026-05-09-regime-classifier-r10-handoff-todo.md \
  scripts/research/regime_consumer_bundle.py \
  scripts/research/tests/test_regime_consumer_bundle.py
```

Suggested commit:

```bash
git commit -m "feat: add regime consumer bundle sidecar"
```
