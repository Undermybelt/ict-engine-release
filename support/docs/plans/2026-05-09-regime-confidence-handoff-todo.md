# Regime Confidence Handoff TODO

> Live board for Slice 7: regime 95% confidence report.

**Goal:** make “95% regime confidence” operational and machine-consumable before BBN/path-ranker/execution-tree consumes regime evidence.

**Scope:** sidecar support/scripts/tests/docs only. Ignore unrelated worktree drift.

---

## Design Locks

- Zero config defaults work with only `rows.jsonl + candidate_id + output_json`.
- Hot-plug thresholds via CLI/profile later.
- No heavy dependencies; no sklearn/MAPIE required in first slice.
- Consumer artifact is compact JSON.
- `confidence_95` means:
  - singleton conformal-style set
  - rolling coverage >= 0.93
  - ECE <= 0.05
  - bootstrap CI width <= 0.25
  - transition probability <= 0.2
  - flip rate <= 0.2

## Current Slice

### Done

- [x] Master TODO created: `support/docs/plans/2026-05-09-heuristic-learning-execution-todo.md`
- [x] Failing tests written first: `support/scripts/research/tests/test_regime_confidence_report.py`
- [x] RED verified: missing `regime_confidence_report` import
- [x] Implemented `support/scripts/research/regime_confidence_report.py`
- [x] CLI supports:
  - `--rows-jsonl`
  - `--output-json`
  - `--candidate-id`
  - confidence thresholds

### Next

- [x] Run target tests
  - `python3 -m unittest support/scripts/research/tests/test_regime_confidence_report.py` -> 3 OK
- [x] Run full research tests
  - `python3 -m unittest discover -s support/scripts/research/tests -p 'test_*.py'` -> 39 OK
- [x] Update master TODO status
- [ ] Commit only this slice

## Input Contract

Rows JSONL:

```json
{"timestamp":"t0","truth":"trend","posterior":{"trend":0.96,"range":0.04},"transition_prob":0.05}
```

## Output Contract

```text
regime_confidence_report.json
```

Fields:

```text
confidence_95
conformal_set_size
rolling_coverage
calibration_ece
bootstrap_ci_width
transition_prob
flip_rate
regime_confidence_gate
```

## CLI Floor

```bash
python3 support/scripts/research/regime_confidence_report.py \
  --rows-jsonl /tmp/ict-hl/NQ/regime_rows.jsonl \
  --output-json /tmp/ict-hl/NQ/regime_confidence_report.json \
  --candidate-id nq-regime-v1
```
