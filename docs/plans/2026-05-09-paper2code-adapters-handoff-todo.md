# Paper2Code Adapters Handoff TODO

> Live board for Slice 12: paper2code adapters.

**Goal:** wire paper-inspired modules as sidecar reports before any runtime/Rust integration.

**Scope:** sidecar only. No Rust runtime changes in this slice.

---

## Design Locks

- Zero config over JSONL market rows.
- Caller controls output path; no repo/state pollution.
- Adapters produce BBN/exec-tree hints, not hard trades.
- Existing dirty Rust files are untouched.
- Promotion still depends on downstream payoff/PBO/BBN evidence value gates.

## Current Slice

### Done

- [x] Failing tests written first: `scripts/research/tests/test_paper2code_adapters.py`
- [x] RED verified: missing `paper2code_adapters` import
- [x] Implemented `scripts/research/paper2code_adapters.py`
- [x] Adapter families:
  - `rammstein_ou_reversion`
  - `crowded_trades_pressure`
  - `kyle_liquidity_slippage`
  - `red_queens_friction`
- [x] Target tests passed:
  - `python3 -m unittest scripts/research/tests/test_paper2code_adapters.py -v` -> 3 OK

### Next

- [x] Run full research tests
  - `python3 -m unittest discover -s scripts/research/tests -p 'test_*.py'` -> 52 OK
- [x] Update master TODO
- [x] Commit only this slice
  - `11ac3c6 feat: add paper2code adapter reports`

## Output Contract

```text
paper2code_adapter_report.json
```

Fields:

```text
execution_hint
max_risk_score
adapters[].adapter_id
adapters[].paper_family
adapters[].edge_score
adapters[].risk_score
adapters[].bbn_evidence_hint
```

## CLI Floor

```bash
python3 scripts/research/paper2code_adapters.py \
  --rows-jsonl /tmp/ict-hl/NQ/market_rows.jsonl \
  --output-json /tmp/ict-hl/NQ/paper2code_adapter_report.json \
  --candidate-id NQ-paper-sidecar
```