# Crowded Trades — Market Clustering & Herding Detection

Paper: [arXiv:2002.03319v1](https://arxiv.org/abs/2002.03319v1)

## What this implements

Herding bias detection based on the crowded trades paper:

1. **Trading Overlap** — Jaccard similarity between agent trading patterns
2. **Market Clustering** — mean pairwise overlap (§2)
3. **Herding Bias from Ising** — verify Ising magnetization against crowding
4. **ICT Ising Overlay Integration** — double crowding risk detection

## Key insight for ict-engine

§Abstract: "market clustering has a CAUSAL effect on the properties
of the tails of the stock return distribution, particularly the POSITIVE tail"

This means:
- Ising `herding_bias` can be verified with empirical crowding data
- Crowded LONG → positive tail risk (flash crash up)
- Crowded SHORT → negative tail risk (only in turmoil)
- Factor agreement + Ising agreement = double crowding = high risk

## Integration

```python
from crowded_trades.src import herding_bias_from_ising, apply_to_ict_ising_overlay

# Verify Ising herding
result = herding_bias_from_ising(spin_series)
if result["herding_detected"]:
    print(f"Herding: {result['implication']}")

# Apply to ict-engine Ising overlay
risk = apply_to_ict_ising_overlay(
    ising_magnetization=0.8,
    ising_coupling=1.5,
    factor_directions=np.array([1, 1, 1, 1, -0.5]),
)
print(f"Risk: {risk['risk_level']} — {risk['action']}")
```

## Tests: 4/4 passed
