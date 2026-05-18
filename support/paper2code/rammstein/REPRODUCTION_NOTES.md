# REPRODUCTION_NOTES — RAmmStein (2602.19419v2)

## Unspecified choices (with rationale)

### OU estimation window = 200
Paper says "rolling window" but never specifies size. We use 200 bars as a reasonable
balance between estimation stability and regime responsiveness. Sensitivity: smaller
windows (50-100) give noisier θ estimates; larger (500+) lag regime changes.

### DDQN activation = ReLU
Paper says "2-layer neural network with 128 hidden units and ReLU activation" (§VI-A).
Technically specified but easy to miss.

### Rebalance cost = 30 bps, Fee rate = 3 bps
Paper does not state exact cost values. Using typical Uniswap V3 values:
- 3 bps fee tier (most common for major pairs)
- ~30 bps total rebalancing cost (gas + slippage + swap fee)

### Range width = ±2%
Paper says "user-defined λ" (§IV-B). Using ±2% as a reasonable default for
futures-like instruments. For crypto, wider ranges (±5%) may be appropriate.

### State vector: 8 dimensions, only 6 named
Paper explicitly names: δ_p, d_edge, θ, δ_μ, σ̃, ϕ_a (6 components).
States "8-dimensional state vector" (§IV-B) but does not name the remaining 2.
We fill with momentum (5-bar return) and vol_ratio (short/long realized vol).

### θ clipping to [0, 1]
Paper §IV-B says "truncated to [0,1]" but does not specify the formula.
We use simple np.clip. Alternative: could normalize by a learned max_θ.

### Training episode length = 500 steps
Not specified. Using 500 as a balance between episode diversity and computational cost.

### Number of episodes = 1000
Not specified. Using 1000 as a reasonable budget. Paper's experiments likely used more.

## Key equations implemented

- Eq.10: dS = θ(μ-S)dt + σdWt — OU SDE
- Eq.22: V(s) = f(s,c)Δt + e^{-ρΔt} E[V(S_{t+Δt}, c)] — Bellman-HJB connection

## Integration notes for ict-engine

The `state_to_execution_features()` function maps RAmmStein's state vector to
ict-engine's `ExecutionFeatures`:
- `ou_theta` → execution feasibility signal
- `ou_overextension` → from δ_μ
- `regime_laziness_score` = θ × ϕ_a — key "regime-aware laziness" metric
- `edge_proximity` → execution readiness signal

## Known limitations

1. DDQN training on simulated OU data will not generalize to real market data
   without fine-tuning on historical prices.
2. The 2-action space (wait/rebalance) is specific to AMM LP management.
   For general trading, action space needs expansion.
3. OU assumption may not hold in trending markets — θ will be near 0 and
   the model provides no useful signal.
