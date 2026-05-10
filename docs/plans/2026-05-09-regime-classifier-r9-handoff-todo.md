# Regime Classifier R9 Handoff TODO

Live board for R9 high-confidence decision aggregator sidecar.

Goal: aggregate R5/R6/R7/R8 into one compact consumer decision for BBN, path-ranker, and execution-tree consumers.

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
- `src/application/reflection/execution_tree_bundle.rs`
- `src/auto_quant_command.rs`
- `src/main.rs`
- `src/validate_market_state_command.rs`
- `docs/plans/2026-05-09-regime-to-execution-mainline-audit-handoff-todo.md`

---

## Slice R9: High-Confidence Decision Aggregator

### Done

- [x] Wrote failing tests first: `scripts/research/tests/test_regime_high_confidence_decision.py`.
- [x] RED verified: `ModuleNotFoundError: No module named 'regime_high_confidence_decision'`.
- [x] Implemented: `scripts/research/regime_high_confidence_decision.py`.
- [x] Zero-config pure-Python sidecar.
- [x] Reads R5 `regime_expert_scores.jsonl`.
- [x] Reads R6 `regime_conformal_calibration_report.json`.
- [x] Reads R7 `regime_distributional_agreement_report.json`.
- [x] Reads R8 `regime_transition_governor_report.json`.
- [x] Emits `regime_high_confidence_decision.json`.
- [x] Supports hot-plug consumer scope via `--label-prefix`.
- [x] Emits decision states:
  - `single_label_95`
  - `single_label_99`
  - `label_set`
  - `transitional`
  - `unknown_abstain`
- [x] Emits final `trade_usable` boolean.
- [x] Emits final label or label set.
- [x] Preserves rejection / abstain reasons compactly.
- [x] Emits path-ranker-ready context.
- [x] Emits BBN-ready evidence hint.
- [x] Emits execution-tree-ready hint.
- [x] Carries user-specific VRP/NQ context:
  - `qqq_hv_level`
  - `nq_vs_200d_pct`
  - `vix3m_level`
  - `qqq_hv_pct_rank_252`
  - `vvix_over_vix`

### Verification

- [x] RED:
  - `python3 -m unittest scripts/research/tests/test_regime_high_confidence_decision.py -v` -> missing module.
- [x] Target GREEN:
  - `python3 -m unittest scripts/research/tests/test_regime_high_confidence_decision.py -v` -> 5 OK.
- [x] Full research suite:
  - `python3 -m unittest discover -s scripts/research/tests -p 'test_*.py'` -> 84 OK.
- [x] CLI smoke R2 -> R3(+aux) -> R5 -> R6 -> R7 -> R8 -> R9.
  - Output: `decision_state=single_label_99`, `trade_usable=true`, `final_label=primary::TrendExpansion`, `execution_tree_hint=accept_regime`, `abstain_reasons=[]`.
  - User VRP/NQ keys present: `nq_vs_200d_pct`, `qqq_hv_level`, `qqq_hv_pct_rank_252`, `vix3m_level`, `vvix_over_vix`.

### CLI Floor

```bash
python3 scripts/research/regime_high_confidence_decision.py \
  --scores /tmp/ict-regime/regime_expert_scores.jsonl \
  --conformal-report /tmp/ict-regime/regime_conformal_calibration_report.json \
  --distributional-report /tmp/ict-regime/regime_distributional_agreement_report.json \
  --governor-report /tmp/ict-regime/regime_transition_governor_report.json \
  --label-prefix primary:: \
  --output-json /tmp/ict-regime/regime_high_confidence_decision.json
```

### Consumer Contract

- Required inputs:
  - R5 scores JSONL.
  - R6 conformal calibration JSON.
  - R7 distributional agreement JSON.
  - R8 transition governor JSON.
- Optional controls:
  - `--label-prefix` to opt into a regime family / subset.
- Consumer output fields:
  - `decision_state`
  - `trade_usable`
  - `final_label`
  - `label_set`
  - `abstain_reasons`
  - `execution_tree_hint`
  - `bbn_evidence_hint`
  - `path_ranker_context`
  - `user_vrp_nq_context`
- User can opt in by invoking the script; main runtime remains unchanged.
- User can ignore this sidecar with zero impact.
- Outputs are explicit paths only; no repo-root state writes.

---

## Immediate Next Slice: R10 Consumer Bundle / Manifest

### Create

- [ ] `scripts/research/regime_consumer_bundle.py`
- [ ] `scripts/research/tests/test_regime_consumer_bundle.py`

### Goal

Package R2-R9 artifacts into one token-friendly manifest that a human, BBN, path-ranker, or execution tree can consume without scanning every intermediate JSON.

### Acceptance

- [ ] Reads any subset of R2-R9 artifact paths.
- [ ] Emits compact `regime_consumer_bundle.json`.
- [ ] Includes latest decision, artifact paths, schema versions, consumer hints, and missing-artifact list.
- [ ] Zero-config: no optional dependencies.
- [ ] Hot-plug: supports `--include-artifact` repeated key/path pairs.
- [ ] No runtime mutation.

---

## Commit Plan

Stage only R9 sidecar files and docs, not unrelated Rust drift.

Suggested add:

```bash
git add \
  docs/plans/2026-05-09-regime-classifier-r9-handoff-todo.md \
  scripts/research/regime_high_confidence_decision.py \
  scripts/research/tests/test_regime_high_confidence_decision.py
```

Suggested commit:

```bash
git commit -m "feat: add high confidence regime decision sidecar"
```
