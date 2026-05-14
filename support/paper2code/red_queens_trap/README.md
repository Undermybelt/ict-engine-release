# Red Queen's Trap — Defensive Safety Module for ict-engine

Paper: [arXiv:2512.15732v1](https://arxiv.org/abs/2512.15732v1)

## What this implements

Four defensive components extracted from the "Galaxy Empire" post-mortem
(500 DRL agents, training APY>300%, live capital decay>70%):

1. **Friction Barrier** (`src/friction_barrier.py`)
   - Breakeven win rate calculator (§4.4, Eq.5-6)
   - Cost-aware PnL: gross → net after fees (§4.1, Eq.4)
   - "Fool's Gold" detector: gross positive but net negative trades
   - Key result: 51.2% accuracy < 55% breakeven → system loses money

2. **Survivor Bias Detector** (`src/survivor_bias.py`)
   - Detects survivor bias in mutation/evolution results (§4.2)
   - Maps to ict-engine: accepted mutations vs dead mutations
   - Stagnation detection (paper: 60% agents never traded)
   - Zombie agent detection (§4.5 — soft budget constraint)

3. **Mode Collapse Monitor** (`src/mode_collapse.py`)
   - Detects population homogeneity (§4.3)
   - Maps to ict-engine: all factors agreeing = overconfidence
   - Effective N strategies (Herfindahl-based)
   - Direction alignment entropy
   - Correlated drawdown risk

4. **Capital Decay Tracker** (`src/capital_decay.py`)
   - Real-time capital decay monitoring (§4, Figure 1)
   - Maps to ict-engine: cumulative score_delta in autoresearch
   - Soft budget constraint detection (§4.5)
   - Bailout injection tracking

## Integration with ict-engine

```python
from red_queens_trap.src import (
    breakeven_win_rate,
    apply_to_factor_mutation,
    apply_to_factor_diversity,
    apply_to_autoresearch,
)

# 1. Before accepting a mutation: check friction barrier
w_be = breakeven_win_rate(reward_risk_ratio=1.5)
if model_accuracy < w_be:
    print(f"Below breakeven: {model_accuracy:.1%} < {w_be:.1%}")

# 2. After autoresearch batch: check survivor bias
result = apply_to_factor_mutation(scores_before, scores_after, accepted)
if result["report"].is_biased:
    print(f"Survivor bias: {result['survivor_bias_pct']:.1f}% overestimate")

# 3. After factor ranking: check mode collapse
diversity = apply_to_factor_diversity(factor_scores, factor_directions)
if not diversity["is_diverse"]:
    print("Factors have collapsed to single strategy")

# 4. After autoresearch session: check capital decay
decay = apply_to_autoresearch(mutation_deltas, accepted)
if decay["report"].is_catastrophic:
    print(f"Catastrophic decay: {abs(decay['report'].total_return):.0%}")
```

## File structure

```
red_queens_trap/
├── configs/base.yaml           # All thresholds (cited to paper)
├── src/
│   ├── __init__.py             # Public API
│   ├── friction_barrier.py     # Breakeven + cost-aware PnL + tests ✓
│   ├── survivor_bias.py        # Survivor bias detector + tests ✓
│   ├── mode_collapse.py        # Mode collapse monitor + tests ✓
│   └── capital_decay.py        # Capital decay tracker + tests ✓
├── contribution.md
├── requirements.txt
└── README.md
```

## Five failure modes from the paper

| # | Failure Mode | ict-engine Risk | This Module |
|---|---|---|---|
| 1 | Cost-Blind Hallucination (§4.1) | Factor mutation ignores friction | `friction_barrier.py` |
| 2 | Stagnation-Starvation Loop (§4.2) | Autoresearch selects inaction | `survivor_bias.py` |
| 3 | Mode Collapse / Systemic Beta (§4.3) | All factors agree = overconfidence | `mode_collapse.py` |
| 4 | Friction Barrier (§4.4) | Below-breakeven accuracy = losing | `friction_barrier.py` |
| 5 | Soft Budget Constraint (§4.5) | Keeping bad mutations alive | `capital_decay.py` |

## Key equations

- Eq.4: PnL_Net = (P_exit - P_entry)×Q - 2×(P×Q×Fee)
- Eq.5: EV = W·(R·Risk) - (1-W)·Risk - C_trans
- Eq.6: W_BE = (1 + C_ratio) / (1 + R)
- Eq.3: τ_{t+1} = τ_t - 1 + α·I(Profit_t > 0)
