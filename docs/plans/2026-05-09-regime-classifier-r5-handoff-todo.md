# Regime Classifier R5 Handoff TODO

Live board for R5 one-vs-rest regime expert trainer.

Goal: keep R2 ontology, R3 features, and R4 discovery usable by consumers through a zero-config, token-friendly, no-pollution sidecar that can be adopted or ignored.

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
- `src/validate_market_state_command.rs`

---

## Slice R5: One-vs-Rest Expert Training

### Done

- [x] Wrote failing tests first: `scripts/research/tests/test_regime_expert_trainer.py`.
- [x] RED verified: `ModuleNotFoundError: No module named 'regime_expert_trainer'`.
- [x] Implemented: `scripts/research/regime_expert_trainer.py`.
- [x] Zero-config pure-Python fallback; no sklearn dependency.
- [x] Loads R2 `regime_ontology_manifest.json`.
- [x] Reads R3 `regime_features.csv` or JSONL.
- [x] Optional R4 inputs are supported without requiring them:
  - `--cluster-report`
  - `--hmm-report`
- [x] Emits one expert summary per ontology label.
- [x] Emits `regime_expert_scores.jsonl`.
- [x] Emits `regime_expert_training_report.json`.
- [x] Precision-first threshold default: `0.8`.
- [x] Balanced fallback toggle: `--balanced-thresholds` uses `0.5`.
- [x] Unknown/Neutral/Transitional classes are forced to `decision=abstain`.
- [x] Report includes per-label:
  - `precision`
  - `recall`
  - `f1`
  - `brier_proxy`
  - `ece_proxy`
  - `support`
  - `threshold`
- [x] Scores include:
  - `timestamp`
  - `label_id`
  - `score`
  - `threshold`
  - `decision`
  - `abstain_reason`
- [x] Purged split / embargo-compatible interface exposed in report:
  - `purged_split_interface.enabled=true`
  - `embargo_bars`
  - `implementation=deterministic_fallback`
- [x] Ontology remains read-only.

### Verification

- [x] Target tests:
  - `python3 -m unittest scripts/research/tests/test_regime_expert_trainer.py -v` -> 4 OK.
- [x] Full research suite after R5:
  - `python3 -m unittest discover -s scripts/research/tests -p 'test_*.py'` -> 68 OK.
- [x] CLI smoke with generated R2/R3 artifacts:
  - R2 ontology -> R3 features -> R5 trainer.
  - Output: `expert_count=53`, `score_count=212`, `mode=pure_python_threshold_fallback`, `ontology_mutation=read_only`.

### CLI Floor

```bash
python3 scripts/research/regime_expert_trainer.py \
  --ontology /tmp/ict-regime/regime_ontology_manifest.json \
  --features /tmp/ict-regime/regime_features.csv \
  --cluster-report /tmp/ict-regime/cluster_regime_discovery_report.json \
  --hmm-report /tmp/ict-regime/hmm_regime_discovery_report.json \
  --output-scores /tmp/ict-regime/regime_expert_scores.jsonl \
  --output-report /tmp/ict-regime/regime_expert_training_report.json
```

### Consumer Contract

- Required inputs:
  - R2 ontology JSON.
  - R3 features CSV/JSONL.
- Optional inputs:
  - R4 cluster report.
  - R4 HMM report.
- User can opt in by invoking the script; main runtime remains unchanged.
- User can ignore this sidecar with zero impact.
- Outputs are explicit paths only; no repo-root state writes.

---

## Next Slice Completed: R6 Conformal Calibration

### Create

- [x] `scripts/research/regime_conformal_calibration_report.py`
- [x] `scripts/research/tests/test_regime_conformal_calibration_report.py`

### Inputs

- [x] `regime_expert_scores.jsonl`
- [x] `regime_expert_training_report.json`
- [x] Optional truth labels.

### Outputs

- [x] `regime_conformal_calibration_report.json`

### Acceptance

- [x] Supports target coverage `0.95` and `0.99`.
- [x] Emits class-conditional coverage.
- [x] Emits singleton rate.
- [x] Emits conformal set size.
- [x] Emits `confidence_95` / `confidence_99` only when coverage gates pass.
- [x] Unknown/abstain labels remain non-trade-usable.
- [x] Target tests: `python3 -m unittest scripts/research/tests/test_regime_conformal_calibration_report.py -v` -> 4 OK.

---

## Commit Plan

Stage only sidecar chain files, not unrelated Rust drift.

Suggested R5 add:

```bash
git add \
  docs/plans/2026-05-09-regime-classifier-r5-handoff-todo.md \
  scripts/research/regime_expert_trainer.py \
  scripts/research/tests/test_regime_expert_trainer.py
```

Suggested commit:

```bash
git commit -m "feat: add regime expert trainer sidecar"
```
