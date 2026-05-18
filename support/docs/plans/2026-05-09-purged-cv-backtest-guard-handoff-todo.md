# Purged CV Backtest Guard Handoff TODO

> Live board for current PBO / Purged CV / Embargo slice.

**Goal:** stop high-Sharpe hallucination caused by overlapping labels, leakage, and repeated trials before regime/BBN/path-ranker consumes the candidate.

**Scope:** sidecar scripts only. Do not touch unrelated Rust/runtime files or other agents' formatting drift.

---

## Design Locks

- Zero config: pipeline emits guard by default.
- Hot-plug: user can disable with `purged_cv_enabled: false` in profile JSON.
- No pollution: write only under caller-selected `--output-dir`.
- Consumer friendly: compact `purged_cv_guard.json`; payoff report also carries summary fields.
- Rejected/unsafe candidates should be visible before path-ranker/BBN use.
- Keep implementation dependency-free; no mlfinlab/heavy packages.

## Current Slice

### Done

- [x] Added failing tests first: `support/scripts/research/tests/test_purged_cv_backtest_guard.py`
- [x] Verified RED: missing `purged_cv_backtest_guard` import
- [x] Implemented `support/scripts/research/purged_cv_backtest_guard.py`
- [x] Added standalone CLI:
  - `--labels-jsonl`
  - `--output-json`
  - `--nb-trials`
  - `--embargo-bars`
  - `--fold-count`
- [x] Wired guard into `heuristic_payoff_pipeline.py`
- [x] Pipeline writes `purged_cv_guard.json`
- [x] Payoff report now carries:
  - `pbo`
  - `oos_sharpe_lcb`
  - `embargo_bars`
  - `leakage_flags`
  - `purged_cv_gate`

### Next

- [x] Run target tests
  - `python3 -m unittest support/scripts/research/tests/test_purged_cv_backtest_guard.py support/scripts/research/tests/test_heuristic_payoff_pipeline.py` -> 5 OK
- [x] Run full research tests
  - `python3 -m unittest discover -s support/scripts/research/tests -p 'test_*.py'` -> 36 OK
- [ ] Commit only this slice

## Artifact Contract

Input:

```text
labels.jsonl
```

Output:

```text
purged_cv_guard.json
```

Fields:

```text
pbo
oos_sharpe_lcb
oos_sharpe_mean
embargo_bars
leakage_flags
purged_cv_gate
folds[]
```

## CLI Floor

```bash
python3 support/scripts/research/purged_cv_backtest_guard.py \
  --labels-jsonl /tmp/ict-hl/NQ/demo/labels.jsonl \
  --output-json /tmp/ict-hl/NQ/demo/purged_cv_guard.json \
  --nb-trials 25 \
  --embargo-bars 1 \
  --fold-count 4
```
