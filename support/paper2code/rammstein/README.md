# RAmmStein — Regime-Aware Execution via OU + DDQN

Paper: [arXiv:2602.19419v2](https://arxiv.org/abs/2602.19419v2)
Authors: Pranay Anchuri et al.
Mode: minimal | Framework: numpy + pytorch

## What this implements

Three core components from the RAmmStein paper, adapted for ict-engine:

1. **OU Parameter Estimator** (`src/ou_estimator.py`)
   - MLE estimation of θ (mean-reversion speed), μ (equilibrium), σ (volatility)
   - Rolling window estimation for time-varying parameters
   - θ serves as the "Stein Signal" — regime indicator

2. **8-Dim State Vector** (`src/state_builder.py`)
   - Builds the execution decision state from market data + OU params
   - Maps to ict-engine's `ExecutionFeatures` via `state_to_execution_features()`

3. **DDQN Agent** (`src/ddqn_agent.py`)
   - Double DQN approximating the HJB-QVI solution
   - Learns decision boundary: Q(rebalance) - Q(wait) vs θ
   - Key insight: high θ → inaction zone (regime-aware laziness)

## File structure

```
rammstein/
├── configs/base.yaml          # All hyperparameters (cited to paper)
├── src/
│   ├── __init__.py            # Public API
│   ├── ou_estimator.py        # OU MLE estimation + unit tests
│   ├── state_builder.py       # 8-dim state vector + ict-engine bridge
│   ├── ddqn_agent.py          # DDQN with HJB reward structure
│   ├── environment.py         # AMM environment simulator
│   └── train.py               # Training loop + decision boundary eval
├── contribution.md            # Paper analysis
├── ambiguity_audit.md         # Specified vs unspecified items
├── REPRODUCTION_NOTES.md      # Implementation choices explained
├── requirements.txt           # numpy, torch, pyyaml
└── README.md                  # This file
```

## Quick start

```bash
pip install numpy torch pyyaml

# Test OU estimator (no torch needed)
python3 -c "
from src.ou_estimator import run_tests
run_tests()
"

# Full training (requires torch)
python3 -m src.train
```

## Key paper equations implemented

- **Eq.10**: dS = θ(μ-S)dt + σdWt — OU price process
- **Eq.22**: V(s) = f(s,c)Δt + e^{-ρΔt} E[V(S_{t+Δt}, c)] — Bellman-HJB connection

## Integration with ict-engine

```python
from rammstein.src import estimate_ou_mle, build_state, state_to_execution_features

# Estimate OU from recent prices
ou = estimate_ou_mle(recent_prices)

# Build execution state
state = build_state(
    current_price=S_t,
    position_center=c,
    range_lower=lower,
    range_upper=upper,
    ou_params=ou,
    recent_prices=history,
    active_steps=n_active,
    total_steps=n_total,
)

# Map to ict-engine ExecutionFeatures
features = state_to_execution_features(state)
# → {"ou_theta": ..., "ou_overextension": ..., "regime_laziness_score": ...}
```

## Unspecified choices

See `REPRODUCTION_NOTES.md` for all implementation decisions not stated in the paper.

Key ones:
- OU estimation window: 200 bars (paper says "rolling", no size given)
- State vector: 8-dim but only 6 named; we add momentum + vol_ratio
- Rebalance cost: 30 bps (typical DEX, not stated in paper)
- Training episodes: 1000 (not stated in paper)

## Citation

```bibtex
@article{anchuri2026rammstein,
  title={RAmmStein: Regime Adaptation in Mean-reverting Markets with Stein Thresholds},
  author={Anchuri, Pranay and others},
  journal={arXiv preprint arXiv:2602.19419},
  year={2026}
}
```
