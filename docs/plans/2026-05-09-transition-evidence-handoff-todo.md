# Transition Evidence Aggregator Handoff TODO

> Live board for Slice 8: transition evidence aggregator.

**Goal:** convert regime confidence + drift/changepoint rows into execution-tree-ready transition evidence.

**Scope:** sidecar only. No Rust runtime changes in this slice.

---

## Design Locks

- Zero config with `regime_confidence_report.json` only.
- Optional drift rows via JSONL.
- No repo pollution; caller chooses output path.
- Consumer-friendly JSON fields for execution tree / BBN.
- Hot-plug thresholds via CLI.

## Current Slice

### Done

- [x] Failing tests written first: `scripts/research/tests/test_transition_evidence_aggregator.py`
- [x] RED verified: missing `transition_evidence_aggregator` import
- [x] Implemented `scripts/research/transition_evidence_aggregator.py`
- [x] CLI supports:
  - `--regime-report-json`
  - `--drift-jsonl`
  - `--output-json`
  - transition thresholds

### Next

- [x] Run target tests
  - `python3 -m unittest scripts/research/tests/test_transition_evidence_aggregator.py` -> 3 OK
- [x] Run full research tests
  - `python3 -m unittest discover -s scripts/research/tests -p 'test_*.py'` -> 42 OK
- [x] Update master TODO
- [ ] Commit only this slice

## Output Contract

```text
transition_evidence.json
```

Fields:

```text
transition_alert_95
transition_hazard
drift_flags
execution_tree_block_hint
regime_confidence_gate
regime_confidence_95
```

## CLI Floor

```bash
python3 scripts/research/transition_evidence_aggregator.py \
  --regime-report-json /tmp/ict-hl/NQ/regime_confidence_report.json \
  --drift-jsonl /tmp/ict-hl/NQ/drift.jsonl \
  --output-json /tmp/ict-hl/NQ/transition_evidence.json
```
