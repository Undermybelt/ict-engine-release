# Kyle's Model with Stochastic Liquidity

Paper: [arXiv:2204.11069v1](https://arxiv.org/abs/2204.11069v1)

## What this implements

Execution cost structure from Kyle's microstructure model:

1. **Kyle's Lambda Estimator** — λ = Cov(ΔP, Q)/Var(Q)
2. **Market Depth** — 1/λ (how much flow before price moves)
3. **Submartingale Check** — λ tends to increase over time
4. **Execution Cost Calculator** — cost = λ × |order_size|

## Key insight for ict-engine

§Corollary: "Both Kyle's Lambda and its inverse (market depth) are submartingales."

This means:
- Execution costs tend to **increase** over time
- Market depth tends to **decrease** over time
- **Earlier execution is generally preferred** (costs will be higher later)

## Integration

```python
from kyle_stochastic_liquidity.src import estimate_kyle_lambda, execution_cost_from_kyle

# Estimate λ from recent data
est = estimate_kyle_lambda(price_changes, order_flow)
print(f"λ={est.lambda_t:.4f}, depth={est.market_depth:.0f}")

# Calculate cost for a specific order
cost = execution_cost_from_kyle(est.lambda_t, order_size=100, price=5000)
print(f"Impact: {cost['cost_bps']:.0f} bps")
```

## Tests: 4/4 passed
