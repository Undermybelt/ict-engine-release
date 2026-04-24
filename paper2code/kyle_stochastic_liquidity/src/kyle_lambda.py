"""
Kyle's Model with Stochastic Liquidity — Execution Cost Structure

Paper: https://arxiv.org/abs/2204.11069v1
Implements: Kyle's Lambda estimation and market depth dynamics

Key results:
  - dP = λ_t dY_t (price impact = lambda × order flow)
  - Kyle's Lambda is a submartingale (increasing on average)
  - Market depth (1/λ) is also a submartingale
  - Log-returns are Gaussian even with stochastic volatility

For ict-engine:
  - λ_t as time-varying execution cost
  - Market depth as execution feasibility signal
  - Submartingale property → execution cost tends to increase over time
"""

import numpy as np
from dataclasses import dataclass
from typing import Optional


@dataclass
class KyleLambdaEstimate:
    """Kyle's Lambda estimation result.
    
    λ = Cov(ΔP, Q) / Var(Q)
    where ΔP = price change, Q = signed order flow
    
    §1 — "Kyle's Lambda is the sensitivity of the price to the total demand"
    """
    lambda_t: float         # Kyle's Lambda (price impact per unit flow)
    market_depth: float     # 1/λ (how much flow before price moves 1 unit)
    r_squared: float        # regression quality
    n_observations: int
    valid: bool


def estimate_kyle_lambda(
    price_changes: np.ndarray,
    order_flow: np.ndarray,
    min_samples: int = 20,
) -> KyleLambdaEstimate:
    """Estimate Kyle's Lambda from price changes and order flow.
    
    §1 — "dP_t = λ_t dY_t"
    λ = Cov(ΔP, Q) / Var(Q)
    
    This is the standard microstructure regression:
        ΔP_t = α + λ * Q_t + ε_t
    
    Args:
        price_changes: ΔP_t, shape (N,)
        order_flow: signed order flow Q_t, shape (N,)
        min_samples: minimum observations for valid estimate
    
    Returns:
        KyleLambdaEstimate with λ and market depth
    """
    n = len(price_changes)
    if n < min_samples or n != len(order_flow):
        return KyleLambdaEstimate(
            lambda_t=0.0, market_depth=0.0, r_squared=0.0,
            n_observations=n, valid=False,
        )
    
    # §1 — OLS: ΔP = α + λ*Q + ε
    X = np.column_stack([np.ones(n), order_flow])
    try:
        beta = np.linalg.lstsq(X, price_changes, rcond=None)[0]
    except np.linalg.LinAlgError:
        return KyleLambdaEstimate(
            lambda_t=0.0, market_depth=0.0, r_squared=0.0,
            n_observations=n, valid=False,
        )
    
    alpha, lam = beta[0], beta[1]
    
    # R-squared
    predicted = alpha + lam * order_flow
    ss_res = np.sum((price_changes - predicted) ** 2)
    ss_tot = np.sum((price_changes - np.mean(price_changes)) ** 2)
    r_sq = 1.0 - ss_res / ss_tot if ss_tot > 1e-12 else 0.0
    
    # §1 — Market depth = 1/λ
    if abs(lam) > 1e-10:
        depth = 1.0 / abs(lam)
    else:
        depth = float('inf')
    
    return KyleLambdaEstimate(
        lambda_t=float(abs(lam)),
        market_depth=float(depth),
        r_squared=float(r_sq),
        n_observations=n,
        valid=abs(lam) > 1e-10 and r_sq > 0.01,
    )


def rolling_kyle_lambda(
    price_changes: np.ndarray,
    order_flow: np.ndarray,
    window: int = 100,
) -> tuple[np.ndarray, np.ndarray]:
    """Rolling Kyle's Lambda estimation.
    
    §Corollary — "Kyle's Lambda is a submartingale"
    Track λ over time to detect increasing execution costs.
    
    Args:
        price_changes: full series of ΔP
        order_flow: full series of signed order flow
        window: rolling window size
    
    Returns:
        (lambda_series, depth_series)
    """
    n = len(price_changes)
    if n < window:
        est = estimate_kyle_lambda(price_changes, order_flow)
        return np.full(n, est.lambda_t), np.full(n, est.market_depth)
    
    lambdas = np.zeros(n - window + 1)
    depths = np.zeros(n - window + 1)
    
    for i in range(n - window + 1):
        est = estimate_kyle_lambda(
            price_changes[i:i+window],
            order_flow[i:i+window],
        )
        lambdas[i] = est.lambda_t if est.valid else np.nan
        depths[i] = est.market_depth if est.valid else np.nan
    
    # Forward-fill NaN
    for i in range(1, len(lambdas)):
        if np.isnan(lambdas[i]):
            lambdas[i] = lambdas[i-1]
            depths[i] = depths[i-1]
    
    return lambdas, depths


def execution_cost_from_kyle(
    kyle_lambda: float,
    order_size: float,
    price: float,
) -> dict:
    """§1 — Calculate execution cost using Kyle's Lambda.
    
    Cost = λ * |Q| (price impact of the order)
    Cost_pct = Cost / P
    
    This provides a theoretically grounded execution cost estimate
    that varies with market conditions (λ is time-varying).
    
    Args:
        kyle_lambda: estimated λ
        order_size: signed order quantity
        price: current price
    
    Returns:
        Dict with cost breakdown
    """
    impact = kyle_lambda * abs(order_size)
    impact_pct = impact / price if price > 0 else 0.0
    
    # Market depth: how much flow before price moves 1%
    depth_for_1pct = (0.01 * price) / kyle_lambda if kyle_lambda > 0 else float('inf')
    
    return {
        "price_impact": impact,
        "impact_pct": impact_pct,
        "market_depth": 1.0 / kyle_lambda if kyle_lambda > 0 else float('inf'),
        "depth_for_1pct_move": depth_for_1pct,
        "cost_bps": impact_pct * 10000,
    }


def check_submartingale(lambda_series: np.ndarray) -> dict:
    """§Corollary — Check if Kyle's Lambda exhibits submartingale behavior.
    
    "Both Kyle's Lambda and its inverse (the market depth) are submartingales."
    
    A submartingale has E[X_{t+1} | F_t] ≥ X_t, i.e., tends to increase.
    For execution: λ increasing → cost increasing → feasibility decreasing.
    
    Args:
        lambda_series: time series of Kyle's Lambda
    
    Returns:
        Dict with submartingale diagnostics
    """
    n = len(lambda_series)
    if n < 10:
        return {"valid": False}
    
    # Compute increments
    diffs = np.diff(lambda_series)
    
    # Mean increment (positive = submartingale tendency)
    mean_increment = np.mean(diffs)
    
    # Fraction of positive increments
    positive_frac = np.mean(diffs > 0)
    
    # Trend test: regress λ on time
    t = np.arange(n)
    X = np.column_stack([np.ones(n), t])
    try:
        beta = np.linalg.lstsq(X, lambda_series, rcond=None)[0]
        trend_slope = beta[1]
    except:
        trend_slope = 0.0
    
    return {
        "valid": True,
        "mean_increment": float(mean_increment),
        "positive_increment_frac": float(positive_frac),
        "trend_slope": float(trend_slope),
        "is_submartingale": mean_increment > 0 and positive_frac > 0.5,
        "implication": (
            "Execution costs tend to increase over time. "
            "Earlier execution is generally preferred."
            if mean_increment > 0 else
            "Execution costs are stable or decreasing. "
            "Patience may be rewarded."
        ),
    }


# ── Tests ──────────────────────────────────────────────────────────────

def _test_kyle_lambda_known():
    """Test: recover known Kyle's Lambda from simulated data."""
    np.random.seed(42)
    n = 1000
    true_lambda = 0.5
    
    order_flow = np.random.randn(n) * 10.0
    noise = np.random.randn(n) * 0.1
    price_changes = true_lambda * order_flow + noise
    
    est = estimate_kyle_lambda(price_changes, order_flow)
    assert est.valid
    assert abs(est.lambda_t - true_lambda) < 0.1, f"λ={est.lambda_t:.3f} vs {true_lambda}"
    print(f"  ✓ Known λ: estimated={est.lambda_t:.3f} (true {true_lambda}), R²={est.r_squared:.3f}")


def _test_market_depth():
    """Test: market depth = 1/λ."""
    est = estimate_kyle_lambda(
        np.array([0.5, 1.0, 1.5, 2.0, 2.5]),
        np.array([1.0, 2.0, 3.0, 4.0, 5.0]),
    )
    if est.valid:
        assert abs(est.market_depth - 1.0/est.lambda_t) < 0.01
        print(f"  ✓ Market depth: {est.market_depth:.3f} = 1/{est.lambda_t:.3f}")


def _test_execution_cost():
    """Test: execution cost from Kyle's Lambda."""
    result = execution_cost_from_kyle(0.5, 100, 10000)
    assert result["price_impact"] == 50.0  # 0.5 * 100
    assert result["cost_bps"] == 50.0      # 50/10000 * 10000
    print(f"  ✓ Execution cost: impact={result['price_impact']:.0f}, bps={result['cost_bps']:.0f}")


def _test_submartingale():
    """Test: detect submartingale in trending λ series."""
    # Increasing lambda series (submartingale)
    lam = np.linspace(0.1, 0.5, 100) + np.random.randn(100) * 0.01
    result = check_submartingale(lam)
    assert result["is_submartingale"]
    print(f"  ✓ Submartingale detected: mean_inc={result['mean_increment']:.4f}, pos_frac={result['positive_increment_frac']:.2f}")


def run_tests():
    print("Running Kyle's Lambda tests...")
    _test_kyle_lambda_known()
    _test_market_depth()
    _test_execution_cost()
    _test_submartingale()
    print("All Kyle tests passed.")


if __name__ == "__main__":
    run_tests()
