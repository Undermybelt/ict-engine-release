# Factor Formula Library Handoff TODO

> Live board for Slice 11: formula seed library.

**Goal:** give the self-iteration loop a hot-pluggable factor seed pool instead of inventing formulas from scratch each run.

**Scope:** sidecar only. No Rust runtime changes in this slice.

---

## Design Locks

- Zero config exports a usable formula library.
- Caller controls output paths; no repo/state pollution.
- Seeds carry source, family, expression, required fields, default params, allowed regimes, mutation hints.
- CLI family filters support focused searches.
- Seeds are skeletons, not production claims; payoff/BBN/PBO gates still decide promotion.

## Current Slice

### Done

- [x] Failing tests written first: `support/scripts/research/tests/test_factor_formula_library.py`
- [x] RED verified: missing `factor_formula_library` import
- [x] Implemented `support/scripts/research/factor_formula_library.py`
- [x] Included seed families:
  - momentum
  - volatility_breakout
  - mean_reversion
  - liquidity
  - options_vrp
  - structure_ict
  - crowding
- [x] Target tests passed:
  - `python3 -m unittest support/scripts/research/tests/test_factor_formula_library.py -v` -> 3 OK

### Next

- [x] Run full research tests
  - `python3 -m unittest discover -s support/scripts/research/tests -p 'test_*.py'` -> 49 OK
- [x] Update master TODO
- [x] Commit only this slice
  - `2672288 feat: add factor formula seed library`

## Output Contract

```text
factor_formula_library.json
factor_formula_library.jsonl
```

Seed fields:

```text
seed_id
family
source
expression
required_fields
default_params
allowed_regimes
mutation_hints
hotplug_ready
```

## CLI Floor

```bash
python3 support/scripts/research/factor_formula_library.py \
  --output-json /tmp/ict-hl/formula_library.json \
  --output-jsonl /tmp/ict-hl/formula_library.jsonl
```

Family-filtered:

```bash
python3 support/scripts/research/factor_formula_library.py \
  --output-json /tmp/ict-hl/mean_reversion_library.json \
  --family mean_reversion
```