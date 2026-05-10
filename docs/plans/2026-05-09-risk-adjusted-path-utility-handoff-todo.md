# Risk-Adjusted Path Utility Handoff TODO

> Live board for Slice 10: risk-adjusted path utility.

**Goal:** make path-ranker targets model trade utility, not raw PnL only.

**Scope:** sidecar target exporter only. No Rust runtime changes in this slice.

---

## Design Locks

- Keep existing payoff gate behavior: `probe/promote` export targets; `reject` goes failure memory only.
- Add utility columns without removing raw labels.
- Utility formula is explicit and hot-pluggable later:
  - `risk_adjusted_path_utility = realized_R - mae_penalty - time_penalty + regime_confidence_bonus - slippage_penalty`
  - `mae_penalty = abs(min(0, mae))`
  - `time_penalty = max(0, time_to_hit) * 0.01`
  - `regime_confidence_bonus = clamp(regime_confidence, 0, 1) * 0.10`
  - `slippage_penalty = abs(slippage_R)`

## Current Slice

### Done

- [x] Failing tests written first in `scripts/research/tests/test_payoff_to_path_ranker_target.py`
- [x] RED verified: missing utility fields and helper
- [x] Implemented utility columns in `scripts/research/payoff_to_path_ranker_target.py`
- [x] Target tests passed:
  - `python3 -m unittest scripts/research/tests/test_payoff_to_path_ranker_target.py -v` -> 3 OK

### Next

- [x] Run full research tests
  - `python3 -m unittest discover -s scripts/research/tests -p 'test_*.py'` -> 46 OK
- [x] Update master TODO
- [x] Commit only this slice
  - `b41f850 feat: add risk adjusted path utility`

## Added Target Fields

```text
risk_adjusted_path_utility
mae_penalty
time_penalty
regime_confidence_bonus
slippage_penalty
```

## Consumer Meaning

- Ranker may train on `risk_adjusted_path_utility` when present.
- Raw `realized_R` remains for audit and fallback.
- High-confidence regime adds only a small bonus; drawdown/time/slippage still dominate.