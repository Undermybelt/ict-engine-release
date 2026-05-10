# Payoff to Path-Ranker Handoff TODO

> Live board for the current payoff -> regime/BBN/path-ranker integration slice.

**Goal:** connect payoff truth (`reject/probe/promote`, `dsr`, `psr`) to path-ranker target export while keeping regime/BBN consumption gated. Rejected candidates must become failure memory only.

**Scope:** sidecar scripts and tests. Do not touch dirty Rust runtime files unless a sidecar cannot satisfy consumer needs.

---

## Design Locks

- Zero config: labels + payoff report + output dir + symbol are enough.
- No pollution: write only inside caller-selected output dir.
- Token friendly: compact JSON/CSV/JSONL artifacts.
- Hot-plug: user can choose auxiliary fields to carry into path-ranker rows.
- BBN/regime rule: only `probe` and `promote` are consumable.
- Reject rule: no path-ranker target CSV; write `failure_memory.jsonl` only.
- User-specific default fields:
  - `qqq_hv_level`
  - `nq_vs_200d_pct`
  - `vix3m_level`
  - `qqq_hv_pct_rank_252`
  - `vvix_over_vix`

## Current Slice

### Done

- [x] Wrote failing tests first: `scripts/research/tests/test_payoff_to_path_ranker_target.py`
- [x] Verified RED: missing `payoff_to_path_ranker_target` import
- [x] Implemented exporter script: `scripts/research/payoff_to_path_ranker_target.py`
- [x] Exporter writes path-ranker target rows for `probe/promote`
- [x] Exporter writes failure memory only for `reject`
- [x] Exporter writes `bbn_gate.json` so regime/BBN consumers can obey the gate
- [x] Wired exporter into `heuristic_payoff_pipeline.py`
- [x] Target tests passed:
  - `python3 -m unittest scripts/research/tests/test_payoff_to_path_ranker_target.py scripts/research/tests/test_heuristic_payoff_pipeline.py` -> 4 OK

### Next

- [x] Run full research tests
  - `python3 -m unittest discover -s scripts/research/tests -p 'test_*.py'` -> 33 OK
- [ ] Commit only this slice

## Artifact Contract

Input:

```text
labels.jsonl
payoff_report.json
```

Outputs for `probe/promote`:

```text
path_ranker_target.csv
path_ranker_target.jsonl
bbn_gate.json
path_ranker_handoff_summary.json
```

Outputs for `reject`:

```text
failure_memory.jsonl
bbn_gate.json
path_ranker_handoff_summary.json
```

## CLI Floor

```bash
python3 scripts/research/payoff_to_path_ranker_target.py \
  --labels-jsonl /tmp/labels.jsonl \
  --payoff-report-json /tmp/payoff_report.json \
  --output-dir /tmp/ict-hl/NQ/demo/path_ranker \
  --symbol NQ
```
