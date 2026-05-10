# Regime Classifier R6 Handoff TODO

Live board for R6 conformal calibration sidecar.

Goal: turn R5 expert scores into coverage-gated regime confidence artifacts, while keeping adoption optional, zero-config, token-friendly, and isolated to explicit output paths.

Scope: sidecar scripts/tests/docs only. Do not touch unrelated Rust drift.

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

- `src/auto_quant_command.rs`
- `src/main.rs`
- `src/validate_market_state_command.rs`
- `docs/plans/2026-05-09-regime-to-execution-mainline-audit-handoff-todo.md`

---

## Slice R6: Conformal Calibration Layer

### Done

- [x] Wrote failing tests first: `scripts/research/tests/test_regime_conformal_calibration_report.py`.
- [x] RED verified: `ModuleNotFoundError: No module named 'regime_conformal_calibration_report'`.
- [x] Implemented: `scripts/research/regime_conformal_calibration_report.py`.
- [x] Zero-config pure-Python fallback; no scipy/sklearn dependency.
- [x] Reads `regime_expert_scores.jsonl`.
- [x] Reads `regime_expert_training_report.json`.
- [x] Optional truth labels supported via `--truth`; missing truth does not fail.
- [x] Emits `regime_conformal_calibration_report.json`.
- [x] Supports target coverage defaults:
  - `0.95`
  - `0.99`
- [x] Supports custom repeated `--target-coverage` flags.
- [x] Supports optional hot-plug scope via `--label-prefix` (example: `primary::`, `volatility::`).
- [x] Emits class-conditional coverage when truth labels exist.
- [x] Emits singleton rate.
- [x] Emits max and average conformal set size.
- [x] Emits `confidence_95` / `confidence_99` only when overall coverage, singleton rate, and set-size gates pass.
- [x] Unknown/Neutral/Transitional labels remain `trade_usable=false` through label contracts.

### Verification

- [x] Target tests:
  - `python3 -m unittest scripts/research/tests/test_regime_conformal_calibration_report.py -v` -> 4 OK.
- [x] Full research suite after R6:
  - `python3 -m unittest discover -s scripts/research/tests -p 'test_*.py'` -> 72 OK.
- [x] CLI smoke with generated R2/R3/R5 artifacts and R6 report:
  - R2 ontology -> R3 features -> R5 trainer -> R6 conformal report.
  - `--label-prefix primary::` run completed with `row_count=4`, `singleton_rate=0.75`, `max_conformal_set_size=2`.

### CLI Floor

```bash
python3 scripts/research/regime_conformal_calibration_report.py \
  --scores /tmp/ict-regime/regime_expert_scores.jsonl \
  --training-report /tmp/ict-regime/regime_expert_training_report.json \
  --truth /tmp/ict-regime/regime_truth.jsonl \
  --label-prefix primary:: \
  --output-json /tmp/ict-regime/regime_conformal_calibration_report.json
```

### Consumer Contract

- Required inputs:
  - R5 scores JSONL.
  - R5 training report JSON.
- Optional inputs:
  - Truth labels JSONL keyed by `timestamp`.
  - Custom `--target-coverage` values.
  - Label scope via `--label-prefix`; this lets consumers adopt only selected regime families.
- User can opt in by invoking the script; main runtime remains unchanged.
- User can ignore this sidecar with zero impact.
- Outputs are explicit paths only; no repo-root state writes.

---

## Next Slice Completed: R7 Distributional Agreement

### Create

- [x] `scripts/research/regime_distributional_agreement_report.py`
- [x] `scripts/research/tests/test_regime_distributional_agreement_report.py`

### Inputs

- [x] `regime_features.csv` or JSONL.
- [x] `regime_expert_scores.jsonl`.
- [x] `regime_conformal_calibration_report.json`.

### Outputs

- [x] `regime_distributional_agreement_report.json`.

### Acceptance

- [x] Compares current feature window to each label archetype.
- [x] Uses pure-Python quantile/energy-distance proxy fallback.
- [x] Emits agreement/disagreement with classifier top label.
- [x] Emits `transitional_flag` for high-distance or mixed archetype cases.
- [x] User VRP/NQ fields remain visible in feature group summaries when present:
  - `qqq_hv_level`
  - `nq_vs_200d_pct`
  - `vix3m_level`
  - `qqq_hv_pct_rank_252`
  - `vvix_over_vix`
- [x] Target tests: `python3 -m unittest scripts/research/tests/test_regime_distributional_agreement_report.py -v` -> 3 OK.

---

## Commit Plan

Stage only R6 sidecar files and docs, not unrelated Rust drift.

Suggested add:

```bash
git add \
  docs/plans/2026-05-09-regime-classifier-r5-handoff-todo.md \
  docs/plans/2026-05-09-regime-classifier-r6-handoff-todo.md \
  docs/plans/2026-05-09-regime-classifier-research-and-99-confidence-todo.md \
  scripts/research/regime_conformal_calibration_report.py \
  scripts/research/tests/test_regime_conformal_calibration_report.py
```

Suggested commit:

```bash
git commit -m "feat: add regime conformal calibration sidecar"
```
