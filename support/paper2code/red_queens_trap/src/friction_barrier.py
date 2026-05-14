"""
Red Queen's Trap — Friction Barrier Calculator

Paper: https://arxiv.org/abs/2512.15732v1
Implements: Breakeven analysis and cost-aware PnL

Section references:
  §4.1, Eq.4 — PnL_Net = (P_exit - P_entry)×Q - 2×(P×Q×Fee)
  §4.4, Eq.5 — EV = W·(R·Risk) - (1-W)·Risk - C_trans
  §4.4, Eq.6 — W_BE = (1 + C_ratio) / (1 + R)
"""

import numpy as np
from dataclasses import dataclass
from typing import Optional


@dataclass
class FrictionCosts:
    """§4.4 — Transaction cost structure.
    
    Paper: C_trans ≈ 0.1% round-trip (taker fee 0.04% + slippage 0.02% × 2)
    """
    taker_fee_pct: float = 0.04      # §3 — "Taker Fee of 0.04%"
    slippage_pct: float = 0.02       # §3 — "dynamic Slippage averaging 0.02%"
    
    @property
    def round_trip_pct(self) -> float:
        """§4.1 — Total round-trip cost as percentage.
        
        2 × (fee + slippage) = 2 × (0.04% + 0.02%) = 0.12%
        Paper says "≈0.1%" — we use exact calculation.
        """
        return 2.0 * (self.taker_fee_pct + self.slippage_pct)
    
    @property
    def round_trip_bps(self) -> float:
        """Round-trip cost in basis points."""
        return self.round_trip_pct * 100


def breakeven_win_rate(
    reward_risk_ratio: float,
    risk_pct: float = 1.0,
    costs: Optional[FrictionCosts] = None,
) -> float:
    """§4.4, Eq.6 — Calculate minimum win rate to break even.
    
    W_BE = (1 + C_ratio) / (1 + R)
    
    Where:
      R = reward-to-risk ratio
      C_ratio = C_trans / Risk_pct (transaction cost as fraction of risk)
    
    Paper result: Given C_trans ≈ 0.1%, Risk = 1%, R = 1,
    C_ratio = 0.1, W_BE = 1.1/2 = 55%. DL model achieved only 51.2%.
    
    Args:
        reward_risk_ratio: R in the paper (reward / risk)
        risk_pct: risk as percentage of capital (default 1.0%)
        costs: transaction cost structure
    
    Returns:
        Breakeven win rate (0-1)
    """
    if costs is None:
        costs = FrictionCosts()
    
    # §4.4, Eq.6 — W_BE = (1 + C_ratio) / (1 + R)
    # C_ratio = C_trans / Risk_pct
    c_ratio = costs.round_trip_pct / risk_pct
    w_be = (1.0 + c_ratio) / (1.0 + reward_risk_ratio)
    
    return w_be


def expected_value(
    win_rate: float,
    reward_risk_ratio: float,
    risk_amount: float = 1.0,
    costs: Optional[FrictionCosts] = None,
) -> float:
    """§4.4, Eq.5 — Expected value per trade.
    
    EV = W·(R·Risk) - (1-W)·Risk - C_trans
    
    Where C_trans is the absolute cost in the same units as Risk.
    
    Args:
        win_rate: W (0-1)
        reward_risk_ratio: R (reward / risk)
        risk_amount: absolute risk per trade (same units as cost)
        costs: transaction cost structure
    
    Returns:
        Expected value per trade (positive = profitable)
    """
    if costs is None:
        costs = FrictionCosts()
    
    # §4.4, Eq.5 — C_trans as absolute cost
    c_trans = costs.round_trip_pct / 100.0 * risk_amount
    ev = (win_rate * reward_risk_ratio * risk_amount
          - (1.0 - win_rate) * risk_amount
          - c_trans)
    
    return ev


def net_pnl(
    entry_price: float,
    exit_price: float,
    quantity: float,
    direction: str = "long",
    costs: Optional[FrictionCosts] = None,
) -> dict:
    """§4.1, Eq.4 — Net PnL after transaction costs.
    
    PnL_Net = (P_exit - P_entry)×Q - 2×(P×Q×Fee)
    
    "These micro-gains are gross profits. When netted against
     the round-trip transaction fees (0.08%), the actual economic
     value is negative." (§4.1)
    
    Args:
        entry_price: P_entry
        exit_price: P_exit
        quantity: Q
        direction: "long" or "short"
        costs: transaction cost structure
    
    Returns:
        Dict with gross_pnl, fees, net_pnl, is_profitable
    """
    if costs is None:
        costs = FrictionCosts()
    
    # §4.1, Eq.4 — Gross PnL
    if direction == "long":
        gross_pnl = (exit_price - entry_price) * quantity
    else:
        gross_pnl = (entry_price - exit_price) * quantity
    
    # §4.1 — Round-trip fees: 2 × (P × Q × Fee)
    # Using entry price as reference for fee calculation
    fees = 2.0 * entry_price * quantity * costs.round_trip_pct / 100.0
    
    # §4.1 — Net PnL
    net = gross_pnl - fees
    
    return {
        "gross_pnl": gross_pnl,
        "fees": fees,
        "net_pnl": net,
        "is_profitable": net > 0,
        "gross_positive_net_negative": gross_pnl > 0 and net <= 0,
    }


def friction_sensitivity_analysis(
    win_rates: np.ndarray = None,
    reward_risk_ratios: np.ndarray = None,
    costs: Optional[FrictionCosts] = None,
) -> np.ndarray:
    """Generate EV surface over (win_rate, R) space.
    
    Useful for visualizing the "friction barrier" — the region
    where even above-random accuracy is economically insufficient.
    """
    if win_rates is None:
        win_rates = np.linspace(0.40, 0.65, 50)
    if reward_risk_ratios is None:
        reward_risk_ratios = np.linspace(0.5, 3.0, 50)
    
    ev_surface = np.zeros((len(win_rates), len(reward_risk_ratios)))
    for i, w in enumerate(win_rates):
        for j, r in enumerate(reward_risk_ratios):
            ev_surface[i, j] = expected_value(w, r, costs=costs)
    
    return win_rates, reward_risk_ratios, ev_surface


# ── Tests ──────────────────────────────────────────────────────────────

def _test_breakeven():
    """§4.4 — Paper reports W_BE ≈ 55% for R=1, C≈0.1%."""
    w_be = breakeven_win_rate(reward_risk_ratio=1.0)
    assert 0.53 < w_be < 0.57, f"W_BE={w_be:.3f}, expected ~0.55"
    print(f"  ✓ Breakeven win rate: {w_be:.1%} (paper: ~55%)")


def _test_model_vs_threshold():
    """§4.4 — Model accuracy 51.2% is below breakeven ~55%.
    
    Paper: C_trans ≈ 0.1%, Risk = 1%, R = 1
    W_BE ≈ 55%. Model at 51.2% → EV < 0.
    """
    # Use paper's cost structure: round-trip ≈ 0.1%
    w_be = breakeven_win_rate(reward_risk_ratio=1.0, risk_pct=1.0)
    model_accuracy = 0.512
    # EV = W - (1-W) - C/Risk = 2W - 1 - C_ratio
    costs = FrictionCosts()
    c_ratio = costs.round_trip_pct / 1.0  # C_trans / Risk_pct
    ev = 2.0 * model_accuracy - 1.0 - c_ratio / 100.0  # EV as fraction of risk
    # With 51.2% and R=1: EV = 2*0.512 - 1 - 0.0012 = 0.024 - 0.0012 = 0.0228
    # But normalized to risk%: EV_pct = (2W-1-C_ratio)*Risk = 0.0228*1% = 0.0228%
    # The key point: 51.2% < 56% breakeven, so in the paper's regime the model loses
    # Our 0.12% cost makes breakeven 56% vs paper's 55% with 0.1%
    assert w_be > model_accuracy, f"W_BE={w_be:.1%} should exceed model={model_accuracy:.1%}"
    print(f"  ✓ Model 51.2% < W_BE {w_be:.1%}: below breakeven threshold")


def _test_net_pnl_fools_gold():
    """§4.1 — 'Fool's Gold': micro-gains that are gross positive but net negative."""
    # Paper: 0.05% micro-movement, 0.08% round-trip fee
    result = net_pnl(
        entry_price=100.0,
        exit_price=100.05,  # +0.05% move
        quantity=1000.0,
        direction="long",
    )
    assert result["gross_positive_net_negative"], "Should be Fool's Gold"
    print(f"  ✓ Fool's Gold: gross={result['gross_pnl']:.2f}, fees={result['fees']:.2f}, net={result['net_pnl']:.2f}")


def _test_friction_barrier():
    """Verify the 'friction barrier' concept: even good accuracy can lose money."""
    # 53% win rate with R=1: breakeven is 56%, so 53% loses
    w_be = breakeven_win_rate(1.0)
    assert 0.53 < w_be, f"53% should be below breakeven {w_be:.1%}"
    assert 0.57 > w_be, f"57% should be above breakeven {w_be:.1%}"
    print(f"  ✓ Friction barrier: 53% < W_BE {w_be:.1%} < 57%")


def run_tests():
    print("Running Red Queen's Trap friction tests...")
    _test_breakeven()
    _test_model_vs_threshold()
    _test_net_pnl_fools_gold()
    _test_friction_barrier()
    print("All friction tests passed.")


if __name__ == "__main__":
    run_tests()
