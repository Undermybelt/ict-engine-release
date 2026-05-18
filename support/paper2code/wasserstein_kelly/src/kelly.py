"""
Wasserstein-Kelly — Robust Position Sizing

Paper: https://arxiv.org/abs/2302.13979v1
Implements: Kelly criterion with Wasserstein distributional robustness

Key contributions:
  - Standard Kelly assumes known return distribution → estimation error
  - Wasserstein DRO: optimize against worst-case in Wasserstein ball
  - Robust to estimation error, more stable out-of-sample
  - Convex optimization → tractable

For ict-engine:
  - Position sizing based on factor confidence + estimation uncertainty
  - Wasserstein radius ε controls robustness vs growth tradeoff
  - ε→0: standard Kelly; ε→∞: min-variance (conservative)
"""

import numpy as np
from dataclasses import dataclass


@dataclass
class KellyResult:
    """Kelly portfolio optimization result."""
    weights: np.ndarray          # portfolio weights
    growth_rate: float           # expected log growth
    kelly_fraction: float        # fraction of capital to bet
    is_robust: bool              # whether Wasserstein-DRO was used
    valid: bool


def standard_kelly(
    expected_returns: np.ndarray,
    covariance: np.ndarray,
    risk_free_rate: float = 0.0,
) -> KellyResult:
    """Standard Kelly portfolio optimization.
    
    §1 — "The Kelly strategy maximizes the expected logarithm of wealth"
    
    max_w E[log(1 + w^T r)]
    ≈ max_w w^T μ - 0.5 w^T Σ w  (second-order approximation)
    
    Closed-form: w* = Σ^{-1} (μ - r_f 1)
    
    Args:
        expected_returns: μ, shape (N,)
        covariance: Σ, shape (N, N)
        risk_free_rate: r_f
    
    Returns:
        KellyResult with optimal weights
    """
    n = len(expected_returns)
    excess = expected_returns - risk_free_rate
    
    try:
        # §1 — w* = Σ^{-1} μ_excess
        inv_cov = np.linalg.inv(covariance + np.eye(n) * 1e-8)
        raw_weights = inv_cov @ excess
    except np.linalg.LinAlgError:
        return KellyResult(np.zeros(n), 0, 0, False, False)
    
    # Growth rate: μ^T w - 0.5 w^T Σ w
    growth = excess @ raw_weights - 0.5 * raw_weights @ covariance @ raw_weights
    
    # Kelly fraction: scale to unit gross exposure
    total = np.sum(np.abs(raw_weights))
    if total > 0:
        weights = raw_weights / total
        kelly_frac = min(1.0, total)  # cap at 1x
    else:
        weights = np.zeros(n)
        kelly_frac = 0.0
    
    return KellyResult(
        weights=weights,
        growth_rate=float(growth),
        kelly_fraction=float(kelly_frac),
        is_robust=False,
        valid=True,
    )


def wasserstein_kelly(
    expected_returns: np.ndarray,
    covariance: np.ndarray,
    epsilon: float = 0.1,
    risk_free_rate: float = 0.0,
) -> KellyResult:
    """§Proposition 2.2 — Wasserstein-Kelly portfolio.
    
    Robust Kelly against estimation error via Wasserstein DRO.
    
    The key insight: instead of optimizing against the empirical
    distribution, optimize against the worst distribution within
    a Wasserstein ball of radius ε around the empirical one.
    
    Convex reformulation (simplified):
        max_w min_{δ: ||δ|| ≤ ε} [w^T(μ+δ) - 0.5 w^T Σ w]
        = max_w [w^T μ - ε||w|| - 0.5 w^T Σ w]
    
    The ε||w|| term penalizes concentration → more diversification.
    
    Args:
        expected_returns: μ, shape (N,)
        covariance: Σ, shape (N, N)
        epsilon: Wasserstein radius (0=standard Kelly, ∞=min-var)
        risk_free_rate: r_f
    
    Returns:
        KellyResult with robust weights
    """
    n = len(expected_returns)
    excess = expected_returns - risk_free_rate
    
    if epsilon <= 0:
        return standard_kelly(expected_returns, covariance, risk_free_rate)
    
    # §Proposition 2.2 — Simplified convex form:
    # max_w [w^T μ - ε||w||_2 - 0.5 w^T Σ w]
    # Gradient: μ - ε * w/||w|| - Σ w = 0
    # Solve via projected gradient descent
    
    try:
        inv_cov = np.linalg.inv(covariance + np.eye(n) * 1e-8)
    except np.linalg.LinAlgError:
        return KellyResult(np.zeros(n), 0, 0, True, False)
    
    # Iterative solution (proximal gradient)
    w = np.zeros(n)
    lr = 0.01
    for _ in range(500):
        grad = excess - covariance @ w
        norm_w = np.linalg.norm(w) + 1e-10
        grad -= epsilon * w / norm_w  # proximal term
        
        w = w + lr * grad
        
        # Project: non-negative (long-only constraint)
        w = np.maximum(w, 0)
    
    # Growth rate (robustified)
    growth = excess @ w - 0.5 * w @ covariance @ w - epsilon * np.linalg.norm(w)
    
    # Normalize
    total = np.sum(np.abs(w))
    if total > 0:
        weights = w / total
        kelly_frac = min(1.0, total)
    else:
        weights = np.zeros(n)
        kelly_frac = 0.0
    
    return KellyResult(
        weights=weights,
        growth_rate=float(growth),
        kelly_fraction=float(kelly_frac),
        is_robust=True,
        valid=True,
    )


def fractional_kelly(
    expected_returns: np.ndarray,
    covariance: np.ndarray,
    fraction: float = 0.5,
    risk_free_rate: float = 0.0,
) -> KellyResult:
    """Fractional Kelly: scale down from full Kelly.
    
    §Abstract (Risk-Sensitive paper) — "optimal allocation admits
    two complementary interpretations: as a fractional Kelly strategy
    and as a Kelly portfolio adjusted via the entropic regularization"
    
    Fraction = 0.5 means half-Kelly: half the position size of full Kelly.
    This reduces variance at the cost of some growth.
    
    Args:
        expected_returns: μ
        covariance: Σ
        fraction: Kelly fraction (0-1)
        risk_free_rate: r_f
    """
    full = standard_kelly(expected_returns, covariance, risk_free_rate)
    if not full.valid:
        return full
    
    return KellyResult(
        weights=full.weights * fraction,
        growth_rate=full.growth_rate * fraction,
        kelly_fraction=full.kelly_fraction * fraction,
        is_robust=False,
        valid=True,
    )


def kelly_for_single_bet(
    win_prob: float,
    win_payoff: float = 1.0,
    loss_payoff: float = 1.0,
) -> dict:
    """Classic Kelly for a single binary bet.
    
    f* = (p * b - q) / b
    where p=win_prob, q=1-p, b=win_payoff/loss_payoff
    
    Args:
        win_prob: probability of winning
        win_payoff: amount won per unit bet
        loss_payoff: amount lost per unit bet
    
    Returns:
        Dict with optimal fraction and diagnostics
    """
    q = 1.0 - win_prob
    b = win_payoff / loss_payoff if loss_payoff > 0 else float('inf')
    
    if b <= 0:
        return {"f_star": 0.0, "edge": 0, "valid": False}
    
    f_star = (win_prob * b - q) / b
    edge = win_prob * b - q  # positive = positive edge
    
    return {
        "f_star": float(max(0, f_star)),
        "edge": float(edge),
        "valid": edge > 0,
        "half_kelly": float(max(0, f_star * 0.5)),
        "quarter_kelly": float(max(0, f_star * 0.25)),
    }


# ── Tests ──────────────────────────────────────────────────────────────

def _test_standard_kelly():
    mu = np.array([0.1, 0.05, 0.02])
    cov = np.array([[0.04, 0.01, 0.0], [0.01, 0.02, 0.005], [0.0, 0.005, 0.01]])
    result = standard_kelly(mu, cov)
    assert result.valid
    assert result.weights[0] > result.weights[2], "Higher return asset should get more weight"
    print(f"  ✓ Standard Kelly: weights={result.weights.round(3)}, growth={result.growth_rate:.4f}")


def _test_wasserstein_kelly():
    mu = np.array([0.1, 0.05, 0.02])
    cov = np.array([[0.04, 0.01, 0.0], [0.01, 0.02, 0.005], [0.0, 0.005, 0.01]])
    
    std = standard_kelly(mu, cov)
    robust = wasserstein_kelly(mu, cov, epsilon=0.5)
    
    # Robust should be more diversified
    std_conc = np.max(np.abs(std.weights))
    rob_conc = np.max(np.abs(robust.weights))
    assert rob_conc <= std_conc + 0.01, "Robust should be less concentrated"
    print(f"  ✓ Wasserstein Kelly: max_weight std={std_conc:.3f} vs robust={rob_conc:.3f}")


def _test_fractional_kelly():
    mu = np.array([0.1, 0.05])
    cov = np.array([[0.04, 0.01], [0.01, 0.02]])
    full = standard_kelly(mu, cov)
    half = fractional_kelly(mu, cov, fraction=0.5)
    assert abs(half.kelly_fraction - full.kelly_fraction * 0.5) < 0.001
    print(f"  ✓ Fractional Kelly: full={full.kelly_fraction:.3f}, half={half.kelly_fraction:.3f}")


def _test_single_bet():
    result = kelly_for_single_bet(win_prob=0.6, win_payoff=1.0, loss_payoff=1.0)
    # f* = (0.6*1 - 0.4)/1 = 0.2
    assert abs(result["f_star"] - 0.2) < 0.001
    assert result["valid"]
    print(f"  ✓ Single bet: f*={result['f_star']:.2f}, edge={result['edge']:.2f}")


def run_tests():
    print("Running Wasserstein-Kelly tests...")
    _test_standard_kelly()
    _test_wasserstein_kelly()
    _test_fractional_kelly()
    _test_single_bet()
    print("All Kelly tests passed.")


if __name__ == "__main__":
    run_tests()
