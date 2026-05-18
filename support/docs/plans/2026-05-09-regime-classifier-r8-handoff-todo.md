# Regime Classifier R8 Handoff TODO

Live board for R8 transition governor sidecar.

Goal: turn R5/R6/R7 regime signals into an execution-tree-compatible guardrail hint with hysteresis, transition hazard, and non-trade-usable abstain paths.

Scope: sidecar support/scripts/tests/docs only. Do not touch unrelated Rust drift.

---

## Routing / Process

- Primary route: `ict-engine-runtime`.
- Process skill: `test-driven-dev`.
- Completion gate: `verification-before-completion`.
- Project router: `/Users/thrill3r/.hermes/routing/project-router.md` missing; no override.
- Repo entry map read: `AGENTS.md`.
- Domain reference read: `references/regime-classifier-sidecar-chain.md`.

---

## Current Worktree Exclusions

Do not stage unrelated files unless explicitly asked:

- `src/application/orchestration/execution_tree.rs`
- `src/application/reflection/execution_tree_bundle.rs`
- `src/auto_quant_command.rs`
- `src/validate_market_state_command.rs`
- `support/docs/plans/2026-05-09-regime-to-execution-mainline-audit-handoff-todo.md`

---

## Slice R8: Transition Governor

### Done

- [x] Wrote failing tests first: `support/scripts/research/tests/test_regime_transition_governor.py`.
- [x] RED verified: `ModuleNotFoundError: No module named 'regime_transition_governor'`.
- [x] Implemented: `support/scripts/research/regime_transition_governor.py`.
- [x] Zero-config pure-Python fallback.
- [x] Reads `regime_expert_scores.jsonl`.
- [x] Reads `regime_conformal_calibration_report.json`.
- [x] Reads `regime_distributional_agreement_report.json`.
- [x] Optional `--hmm-report` and `--drift-rows` accepted; missing optional files do not fail.
- [x] Emits `regime_transition_governor_report.json`.
- [x] Enforces minimum duration / hysteresis via `--min-duration`.
- [x] Detects flip-flop labels.
- [x] Emits `transition_hazard`.
- [x] Emits `guardrail_reasons`.
- [x] Emits execution-tree-compatible hint:
  - `accept_regime`
  - `transition_guardrail`
  - `unknown_abstain`
- [x] Emits BBN-compatible evidence hint:
  - `regime_transition_hazard`
  - `regime_governor_hint`
  - `regime_governor_reasons`
- [x] Keeps broad/noisy states non-trade-usable until confidence + distributional gates agree.

### Verification

- [x] Target tests:
  - `python3 -m unittest support/scripts/research/tests/test_regime_transition_governor.py -v` -> 4 OK.
- [x] Full research suite after R8:
  - `python3 -m unittest discover -s support/scripts/research/tests -p 'test_*.py'` -> 79 OK.
- [x] CLI smoke with generated R2/R3/R5/R6/R7 artifacts and R8 report:
  - R2 ontology -> R3 features + auxiliary VRP/NQ -> R5 trainer -> R6 conformal -> R7 agreement -> R8 governor.
  - Output: `current_label=primary::TrendExpansion`, `execution_tree_hint=accept_regime`, `transition_hazard=0.0`, `reasons=[]`.

### CLI Floor

```bash
python3 support/scripts/research/regime_transition_governor.py \
  --scores /tmp/ict-regime/regime_expert_scores.jsonl \
  --conformal-report /tmp/ict-regime/regime_conformal_calibration_report.json \
  --distributional-report /tmp/ict-regime/regime_distributional_agreement_report.json \
  --label-prefix primary:: \
  --min-duration 3 \
  --output-json /tmp/ict-regime/regime_transition_governor_report.json
```

### Consumer Contract

- Required inputs:
  - R5 scores JSONL.
  - R6 conformal calibration JSON.
  - R7 distributional agreement JSON.
- Optional controls:
  - `--label-prefix` to opt into a regime family / subset.
  - `--min-duration` for hysteresis.
  - `--hmm-report` for transition persistence.
  - `--drift-rows` for external drift flags.
- User can opt in by invoking the script; main runtime remains unchanged.
- User can ignore this sidecar with zero impact.
- Outputs are explicit paths only; no repo-root state writes.

---

## Immediate Next Slice: R9 High-Confidence Decision Aggregator

### Create

- [ ] `support/scripts/research/regime_high_confidence_decision.py`
- [ ] `support/scripts/research/tests/test_regime_high_confidence_decision.py`

### Inputs

- [ ] R5 scores.
- [ ] R6 conformal report.
- [ ] R7 distributional agreement report.
- [ ] R8 transition governor report.

### Outputs

- [ ] `regime_high_confidence_decision.json`.

### Acceptance

- [ ] Emits states:
  - `single_label_95`
  - `single_label_99`
  - `label_set`
  - `transitional`
  - `unknown_abstain`
- [ ] Emits final `trade_usable` boolean.
- [ ] Emits final label or label set.
- [ ] Preserves rejection / abstain reasons compactly.
- [ ] Produces path-ranker / BBN / execution-tree-ready fields.

---

## Commit Plan

Stage only R8 sidecar files and docs, not unrelated Rust drift.

Suggested add:

```bash
git add \
  support/docs/plans/2026-05-09-regime-classifier-r7-handoff-todo.md \
  support/docs/plans/2026-05-09-regime-classifier-r8-handoff-todo.md \
  support/docs/plans/2026-05-09-regime-classifier-research-and-99-confidence-todo.md \
  support/scripts/research/regime_transition_governor.py \
  support/scripts/research/tests/test_regime_transition_governor.py
```

Suggested commit:

```bash
git commit -m "feat: add regime transition governor sidecar"
```
