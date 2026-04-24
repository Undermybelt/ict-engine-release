"""
RAmmStein — OU Parameter Estimator

Paper: https://arxiv.org/abs/2602.19419v2
Implements: Ornstein-Uhlenbeck process parameter estimation via MLE

Section references:
  §III-E, Eq.10 — dS = θ(μ-S)dt + σdWt
  §IV-B — θ as "Stein Signal", truncated to [0,1]
"""

import numpy as np
from dataclasses import dataclass
from typing import Optional


@dataclass
class OUParams:
    """§III-E — OU process parameters from Eq.10.
    
    dSt = θ(μ - St)dt + σdWt
    
    θ: mean-reversion speed (the "Stein Signal")
    μ: long-term mean / equilibrium price
    σ: diffusion / volatility
    """
    theta: float    # mean-reversion speed
    mu: float       # long-term mean
    sigma: float    # diffusion coefficient
    valid: bool     # whether estimation succeeded


def estimate_ou_mle(
    prices: np.ndarray,
    dt: float = 1.0,
    theta_clip: tuple[float, float] = (0.0, 1.0),
) -> OUParams:
    """§III-E — MLE estimation of OU parameters.
    
    Given a price series S[0..N], estimate θ, μ, σ from the discrete
    AR(1) representation of the OU process.
    
    The discrete form of dS = θ(μ-S)dt + σdW is:
        S[t+1] = a + b*S[t] + ε[t]
    where:
        b = exp(-θ*dt)  →  θ = -ln(b)/dt
        a = μ*(1-b)     →  μ = a/(1-b)
        σ² = Var(ε)*(2θ)/(1-exp(-2θ*dt))
    
    Args:
        prices: price series, shape (N,) where N >= 3
        dt: time step between observations (default 1.0)
        theta_clip: (min, max) for θ truncation — §IV-B says [0,1]
    
    Returns:
        OUParams with estimated θ, μ, σ and validity flag
    """
    n = len(prices)
    if n < 3:
        return OUParams(theta=0.0, mu=prices[-1] if n > 0 else 0.0, sigma=0.0, valid=False)
    
    # §III-E — AR(1) regression: S[t+1] = a + b*S[t] + ε
    s = prices[:-1]   # S[t]
    s_next = prices[1:]  # S[t+1]
    
    # OLS: [a, b] = (X^T X)^{-1} X^T y
    # X = [[1, S[0]], [1, S[1]], ...]
    X = np.column_stack([np.ones(n - 1), s])
    try:
        beta = np.linalg.lstsq(X, s_next, rcond=None)[0]
    except np.linalg.LinAlgError:
        return OUParams(theta=0.0, mu=prices[-1], sigma=0.0, valid=False)
    
    a_hat, b_hat = beta[0], beta[1]
    
    # §III-E — b must be in (0, 1) for mean-reversion
    if b_hat <= 0 or b_hat >= 1:
        # Not mean-reverting at this timescale
        return OUParams(theta=0.0, mu=prices.mean(), sigma=prices.std(), valid=False)
    
    # §III-E, Eq.10 — θ = -ln(b)/dt
    theta = -np.log(b_hat) / dt
    
    # §III-E — μ = a/(1-b)
    mu = a_hat / (1.0 - b_hat)
    
    # §III-E — σ from residual variance
    residuals = s_next - (a_hat + b_hat * s)
    residual_var = np.var(residuals, ddof=2)
    
    # σ² = Var(ε) * 2θ / (1 - exp(-2θ*dt))
    denom = 1.0 - np.exp(-2.0 * theta * dt)
    if denom > 1e-12:
        sigma = np.sqrt(residual_var * 2.0 * theta / denom)
    else:
        sigma = np.sqrt(residual_var)
    
    # §IV-B — θ truncated to [theta_clip_min, theta_clip_max]
    theta = np.clip(theta, theta_clip[0], theta_clip[1])
    
    return OUParams(
        theta=float(theta),
        mu=float(mu),
        sigma=float(sigma),
        valid=True,
    )


def estimate_ou_rolling(
    prices: np.ndarray,
    window: int = 200,
    dt: float = 1.0,
    theta_clip: tuple[float, float] = (0.0, 1.0),
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Rolling OU estimation over a price series.
    
    Args:
        prices: full price series
        window: estimation window size [UNSPECIFIED in paper]
        dt: time step
        theta_clip: θ clipping range
    
    Returns:
        (theta_series, mu_series, sigma_series) — each shape (N - window + 1,)
    """
    n = len(prices)
    if n < window:
        params = estimate_ou_mle(prices, dt, theta_clip)
        return (
            np.full(n, params.theta),
            np.full(n, params.mu),
            np.full(n, params.sigma),
        )
    
    thetas = np.zeros(n - window + 1)
    mus = np.zeros(n - window + 1)
    sigmas = np.zeros(n - window + 1)
    
    for i in range(n - window + 1):
        chunk = prices[i : i + window]
        params = estimate_ou_mle(chunk, dt, theta_clip)
        thetas[i] = params.theta
        mus[i] = params.mu
        sigmas[i] = params.sigma
    
    return thetas, mus, sigmas


# ── Unit tests ──────────────────────────────────────────────────────────

def _test_ou_known_params():
    """Test: recover known OU parameters from simulated data."""
    np.random.seed(42)
    true_theta, true_mu, true_sigma = 0.3, 100.0, 1.0
    dt = 1.0
    n = 5000
    
    # Simulate OU process
    prices = np.zeros(n)
    prices[0] = true_mu
    for t in range(1, n):
        dW = np.random.randn() * np.sqrt(dt)
        prices[t] = prices[t-1] + true_theta * (true_mu - prices[t-1]) * dt + true_sigma * dW
    
    est = estimate_ou_mle(prices, dt)
    assert est.valid, "Estimation should succeed on long series"
    assert abs(est.theta - true_theta) < 0.05, f"θ: {est.theta:.3f} vs {true_theta}"
    assert abs(est.mu - true_mu) < 1.0, f"μ: {est.mu:.1f} vs {true_mu}"
    assert abs(est.sigma - true_sigma) < 0.1, f"σ: {est.sigma:.3f} vs {true_sigma}"
    print(f"  ✓ Known params: θ={est.theta:.3f} (true {true_theta}), μ={est.mu:.1f} (true {true_mu}), σ={est.sigma:.3f} (true {true_sigma})")


def _test_ou_short_series():
    """Test: short series returns invalid."""
    est = estimate_ou_mle(np.array([1.0, 2.0]))
    assert not est.valid, "Should be invalid for n<3"
    print("  ✓ Short series correctly returns invalid")


def _test_ou_constant():
    """Test: constant prices → θ ≈ 0."""
    est = estimate_ou_mle(np.full(100, 50.0))
    # constant series → b ≈ 1 → θ ≈ 0
    assert est.theta < 0.1, f"θ should be near 0 for constant: {est.theta}"
    print(f"  ✓ Constant prices: θ={est.theta:.4f} (near 0)")


def _test_ou_trending():
    """Test: strong trend → θ clipped to 0 (not mean-reverting)."""
    prices = np.linspace(100, 200, 500)
    est = estimate_ou_mle(prices)
    # b > 1 for trending → invalid
    assert not est.valid or est.theta < 0.01
    print(f"  ✓ Trending prices: valid={est.valid}, θ={est.theta:.4f}")


def run_tests():
    print("Running OU estimator tests...")
    _test_ou_known_params()
    _test_ou_short_series()
    _test_ou_constant()
    _test_ou_trending()
    print("All OU tests passed.")


if __name__ == "__main__":
    run_tests()
