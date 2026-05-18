# Heuristic Payoff Handoff TODO

> Live board for the current self-iteration slice. Update after every meaningful change.

**Goal:** make the payoff truth layer zero-config, consumer-usable, token-friendly, non-polluting, and hot-pluggable before regime / BBN / path-ranker work.

**Scope:** support/scripts/research sidecar only unless a runtime integration defect is proven.

---

## Current State

- [x] First slice committed: `89a0007 feat: add heuristic payoff labeling tools`
- [x] `labeling_triple_barrier.py` exists
- [x] `factor_payoff_shape_report.py` exists
- [x] Unit tests exist for both
- [x] Current worktree has unrelated pre-existing Rust/docs dirty files; do not touch them

## This Slice

### Done

- [x] Added zero-config pipeline test first: `test_heuristic_payoff_pipeline.py`
- [x] Verified RED: missing `heuristic_payoff_pipeline` import fails
- [x] Added `support/scripts/research/heuristic_payoff_pipeline.py`
- [x] Pipeline writes isolated artifacts under caller-selected output dir:
  - `labels.jsonl`
  - `payoff_report.json`
  - `handoff_summary.json`
- [x] Default profile includes user-specific auxiliary fields:
  - `qqq_hv_level`
  - `nq_vs_200d_pct`
  - `vix3m_level`
  - `qqq_hv_pct_rank_252`
  - `vvix_over_vix`
- [x] User can opt into overrides via `--profile-json`
- [x] Added PSR/DSR fields to payoff report after RED test:
  - `psr`
  - `dsr`
  - `deflated_sharpe_benchmark`
  - `effective_trials`
  - `effective_sample_size`

### Next

- [x] Run target test green
  - `python3 -m unittest support/scripts/research/tests/test_heuristic_payoff_pipeline.py` -> 2 OK
- [x] Run full `support/scripts/research/tests` regression
  - `python3 -m unittest discover -s support/scripts/research/tests -p 'test_*.py'` -> 30 OK
- [x] Add/adjust docs if needed
  - this handoff board updated in place
- [ ] Commit only this slice if clean

## Design Locks

- Zero config default must work with only CSV + symbol + candidate id + output dir.
- No repo root state writes.
- Heavy deps forbidden in this slice.
- User profile is optional and hot-pluggable via JSON.
- Consumer output must be compact JSON and JSONL.
- Rejected payoff should stop chain before regime/BBN/path-ranker.

## CLI Floor

```bash
python3 support/scripts/research/heuristic_payoff_pipeline.py \
  --input-csv /tmp/events.csv \
  --output-dir /tmp/ict-hl/NQ/demo/payoff \
  --symbol NQ \
  --candidate-id demo
```

Optional profile:

```bash
python3 support/scripts/research/heuristic_payoff_pipeline.py \
  --input-csv /tmp/events.csv \
  --output-dir /tmp/ict-hl/NQ/demo/payoff \
  --symbol NQ \
  --candidate-id demo \
  --profile-json /tmp/tomac-nq-vrp-profile.json
```

## Open Questions

- Whether later to expose this via Rust CLI wrapper. Not needed for sidecar MVP.
- Whether to add DSR/PBO now or next slice. Current slice only creates handoff-ready payoff truth artifacts.
