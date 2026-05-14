# BBN Evidence Value Report Handoff TODO

> Live board for Slice 9: BBN evidence value report.

**Goal:** only promote BBN evidence edges that reduce uncertainty, improve realized log-loss, and help resolve contradiction cases.

**Scope:** sidecar only. No Rust runtime changes in this slice.

---

## Design Locks

- Zero config from JSONL rows.
- Caller controls output path; no repo/state pollution.
- Edge-level accept/reject lists are machine-readable by BBN/path consumers.
- Hot-plug thresholds via CLI.
- Negative `posterior_entropy_delta` and `logloss_delta` are improvements.

## Current Slice

### Done

- [x] Failing tests written first: `support/scripts/research/tests/test_bbn_evidence_value_report.py`
- [x] RED verified: missing `bbn_evidence_value_report` import
- [x] Implemented `support/scripts/research/bbn_evidence_value_report.py`
- [x] CLI supports:
  - `--rows-jsonl`
  - `--output-json`
  - `--candidate-id`
  - value thresholds

### Next

- [x] Run target tests
  - `python3 -m unittest support/scripts/research/tests/test_bbn_evidence_value_report.py -v` -> 3 OK
- [x] Run full research tests
  - `python3 -m unittest discover -s support/scripts/research/tests -p 'test_*.py'` -> 45 OK
- [x] Update master TODO
- [x] Commit only this slice
  - `029800c feat: add bbn evidence value report`

## Input Contract

```jsonl
{"edge_id":"vrp_regime_to_fill_viable","prior_prob":0.55,"posterior_prob":0.82,"outcome":1,"contradiction":false}
```

Required fields:

```text
edge_id
prior_prob
posterior_prob
outcome
```

Optional fields:

```text
contradiction
```

## Output Contract

```text
bbn_evidence_value_report.json
```

Fields:

```text
posterior_entropy_delta
logloss_delta
contradiction_lift
accepted_edges
rejected_edges
edge_details
```

## CLI Floor

```bash
python3 support/scripts/research/bbn_evidence_value_report.py \
  --rows-jsonl /tmp/ict-hl/NQ/bbn_evidence_rows.jsonl \
  --output-json /tmp/ict-hl/NQ/bbn_evidence_value_report.json \
  --candidate-id NQ-vrp-bbn
```