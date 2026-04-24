"""
Red Queen's Trap — Defensive Safety Module for ict-engine

Paper: https://arxiv.org/abs/2512.15732v1
Implements: Failure mode detection from "Galaxy Empire" post-mortem

Modules:
  friction_barrier:  Breakeven win rate, cost-aware PnL, friction analysis
  survivor_bias:     Detect survivor bias in mutation/evolution results
  mode_collapse:     Detect population homogeneity / factor convergence
  capital_decay:     Track cumulative capital decay in autoresearch
"""

from .friction_barrier import (
    FrictionCosts,
    breakeven_win_rate,
    expected_value,
    net_pnl,
    friction_sensitivity_analysis,
)
from .survivor_bias import (
    SurvivorBiasReport,
    detect_survivor_bias,
    apply_to_factor_mutation,
)
from .mode_collapse import (
    ModeCollapseReport,
    detect_mode_collapse,
    apply_to_factor_diversity,
)
from .capital_decay import (
    CapitalDecayReport,
    track_capital_decay,
    apply_to_autoresearch,
)

__all__ = [
    # Friction
    "FrictionCosts", "breakeven_win_rate", "expected_value", "net_pnl",
    "friction_sensitivity_analysis",
    # Survivor bias
    "SurvivorBiasReport", "detect_survivor_bias", "apply_to_factor_mutation",
    # Mode collapse
    "ModeCollapseReport", "detect_mode_collapse", "apply_to_factor_diversity",
    # Capital decay
    "CapitalDecayReport", "track_capital_decay", "apply_to_autoresearch",
]
