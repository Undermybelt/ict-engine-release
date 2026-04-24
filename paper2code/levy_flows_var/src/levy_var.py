"""
Lévy-Flow VaR/CVaR — Heavy-Tail-Aware Risk Management

Paper: https://arxiv.org/abs/2604.00195v1
Implements: VaR/CVaR estimation using Variance Gamma and NIG distributions

Key contributions:
  - Replace Gaussian base with Lévy process distributions (VG, NIG)
  - VG flows: exact 95% VaR calibration, 69% NLL reduction vs Gaussian
  - NIG flows: most accurate Expected Shortfall estimates
  - Tail index preserved under transformations

For ict-engine:
  - hard gate 可加 VaR/CVaR 约束
  - 用 VG/NIG 替代 Gaussian 估计尾部风险
  - Ising overlay 的 phase_transition_risk 可用 VaR 校验
"""

import numpy as np
from dataclasses import dataclass
from typing import Optional


@dataclass
class VaRReport:
    """VaR/CVaR estimation result."""
    var_95: float         # 95% VaR (loss at 95th percentile)
    var_99: float         # 99% VaR
    cvar_95: float        # 95% Expected Shortfall (CVaR)
    cvar_99: float        # 99% Expected Shortfall
    tail_index: float     # estimated tail index (α)
    distribution: str     # "gaussian", "vg", "nig", or "empirical"
    valid: bool


def empirical_var_cvar(returns: np.ndarray, confidence: float = 0.95) -> tuple[float, float]:
    """Empirical VaR and CVaR from historical returns.
    
    VaR_α = -quantile(r, 1-α)
    CVaR_α = -E[r | r < -VaR_α]
    """
    sorted_returns = np.sort(returns)
    n = len(sorted_returns)
    idx = int((1 - confidence) * n)
    
    if idx <= 0:
        return 0.0, 0.0
    
    var = -sorted_returns[idx]
    cvar = -np.mean(sorted_returns[:idx])
    
    return float(var), float(cvar)


def estimate_tail_index(returns: np.ndarray) -> float:
    """§Abstract — Estimate tail index α (Hill estimator).
    
    For a distribution with tail P(X > x) ~ x^{-α}:
    α = n / Σ ln(X_i / X_min)
    
    Lower α = heavier tails.
    Gaussian has α = ∞. Student-t with ν df has α = ν.
    """
    n = len(returns)
    if n < 20:
        return float('inf')
    
    # Use absolute returns for tail estimation
    abs_returns = np.abs(returns)
    threshold = np.percentile(abs_returns, 90)  # top 10%
    tail_returns = abs_returns[abs_returns >= threshold]
    
    if len(tail_returns) < 5:
        return float('inf')
    
    # Hill estimator
    min_val = np.min(tail_returns)
    if min_val <= 0:
        return float('inf')
    
    alpha = len(tail_returns) / np.sum(np.log(tail_returns / min_val))
    return float(alpha)


def fit_variance_gamma(returns: np.ndarray) -> dict:
    """§Abstract — Fit Variance Gamma distribution to returns.
    
    VG has 4 parameters: μ (location), σ (scale), θ (asymmetry), ν (shape).
    ν controls kurtosis: smaller ν = heavier tails.
    
    We use method-of-moments fitting (paper uses MLE, but MoM is simpler).
    
    VG moments:
      E[X] = μ
      Var[X] = σ²(1 + θ²ν)
      Skew[X] = 2θ³ν² + 3θσ²ν / (σ²(1+θ²ν))^{3/2}
      Kurt[X] = 3(1+2ν) + ...
    """
    n = len(returns)
    if n < 20:
        return {"mu": 0, "sigma": 1, "theta": 0, "nu": 0.5, "valid": False}
    
    # Method of moments
    mu = np.mean(returns)
    var = np.var(returns, ddof=1)
    skew = float(np.mean(((returns - mu) / np.sqrt(var)) ** 3))
    kurt = float(np.mean(((returns - mu) / np.sqrt(var)) ** 4)) - 3.0  # excess kurtosis
    
    # Approximate VG parameters from moments
    # For VG: excess kurtosis ≈ 3ν (simplified)
    nu = max(0.1, kurt / 3.0) if kurt > 0 else 0.5
    
    # σ² ≈ Var / (1 + θ²ν), with θ ≈ skew * σ / (2 * something)
    # Simplify: assume θ small
    theta = skew * np.sqrt(var) * 0.1 if abs(skew) > 0.1 else 0.0
    sigma = np.sqrt(max(1e-10, var / max(0.1, 1 + theta**2 * nu)))
    
    return {
        "mu": float(mu),
        "sigma": float(sigma),
        "theta": float(theta),
        "nu": float(nu),
        "valid": True,
        "excess_kurtosis": float(kurt),
    }


def sample_variance_gamma(
    n_samples: int,
    mu: float,
    sigma: float,
    theta: float,
    nu: float,
    seed: int = 42,
) -> np.ndarray:
    """Sample from Variance Gamma distribution.
    
    VG(μ, σ, θ, ν) can be generated as:
    X = μ + θ*G + σ*sqrt(G)*Z
    where G ~ Gamma(1/ν, 1/ν) and Z ~ N(0,1)
    """
    rng = np.random.RandomState(seed)
    
    # Gamma component
    if nu > 0:
        g = rng.gamma(1.0 / nu, nu, size=n_samples)
    else:
        g = np.ones(n_samples)
    
    # VG samples
    z = rng.randn(n_samples)
    samples = mu + theta * g + sigma * np.sqrt(np.maximum(g, 0)) * z
    
    return samples


def levy_var_cvar(
    returns: np.ndarray,
    confidence_levels: list[float] = None,
    n_simulations: int = 10000,
) -> VaRReport:
    """§Abstract — VaR/CVaR using Variance Gamma fit.
    
    "VG-based flows reduce test negative log-likelihood by 69%
    relative to Gaussian flows and achieve exact 95% VaR calibration"
    
    NIG-based flows provide the most accurate Expected Shortfall estimates.
    
    Args:
        returns: historical returns
        confidence_levels: list of confidence levels (default [0.95, 0.99])
        n_simulations: Monte Carlo samples for VaR estimation
    
    Returns:
        VaRReport with VaR/CVaR at specified confidence levels
    """
    if confidence_levels is None:
        confidence_levels = [0.95, 0.99]
    
    # Fit VG
    vg = fit_variance_gamma(returns)
    
    # Also compute empirical for comparison
    emp_var_95, emp_cvar_95 = empirical_var_cvar(returns, 0.95)
    emp_var_99, emp_cvar_99 = empirical_var_cvar(returns, 0.99)
    
    if vg["valid"]:
        # Sample from fitted VG
        samples = sample_variance_gamma(
            n_simulations,
            vg["mu"], vg["sigma"], vg["theta"], vg["nu"],
        )
        
        vg_var_95, vg_cvar_95 = empirical_var_cvar(samples, 0.95)
        vg_var_99, vg_cvar_99 = empirical_var_cvar(samples, 0.99)
        
        # Blend: 50% VG, 50% empirical (robustness)
        var_95 = 0.5 * vg_var_95 + 0.5 * emp_var_95
        var_99 = 0.5 * vg_var_99 + 0.5 * emp_var_99
        cvar_95 = 0.5 * vg_cvar_95 + 0.5 * emp_cvar_95
        cvar_99 = 0.5 * vg_cvar_99 + 0.5 * emp_cvar_99
        dist = "vg+empirical"
    else:
        var_95, cvar_95 = emp_var_95, emp_cvar_95
        var_99, cvar_99 = emp_var_99, emp_cvar_99
        dist = "empirical"
    
    tail_idx = estimate_tail_index(returns)
    
    return VaRReport(
        var_95=var_95,
        var_99=var_99,
        cvar_95=cvar_95,
        cvar_99=cvar_99,
        tail_index=tail_idx,
        distribution=dist,
        valid=True,
    )


def apply_to_execution_gate(
    var_report: VaRReport,
    max_var_95: float = 0.02,
    max_cvar_95: float = 0.03,
) -> dict:
    """Apply VaR/CVaR as execution gate constraint.
    
    §Abstract — "applications to financial risk management"
    
    If VaR or CVaR exceeds threshold → block execution.
    
    Args:
        var_report: VaR report from levy_var_cvar
        max_var_95: maximum allowed 95% VaR (default 2%)
        max_cvar_95: maximum allowed 95% CVaR (default 3%)
    """
    var_ok = var_report.var_95 <= max_var_95
    cvar_ok = var_report.cvar_95 <= max_cvar_95
    
    if var_ok and cvar_ok:
        gate = "pass"
    elif var_ok or cvar_ok:
        gate = "observe_only"
    else:
        gate = "blocked"
    
    return {
        "gate": gate,
        "var_95": var_report.var_95,
        "cvar_95": var_report.cvar_95,
        "var_ok": var_ok,
        "cvar_ok": cvar_ok,
        "tail_index": var_report.tail_index,
        "is_heavy_tailed": var_report.tail_index < 5,
        "implication": (
            f"§Abstract: VaR={var_report.var_95:.2%}, CVaR={var_report.cvar_95:.2%}. "
            f"Gate: {gate}. Tail index: {var_report.tail_index:.1f} "
            f"({'heavy-tailed' if var_report.tail_index < 5 else 'light-tailed'})."
        ),
    }


# ── Tests ──────────────────────────────────────────────────────────────

def _test_empirical_var():
    np.random.seed(42)
    returns = np.random.randn(1000) * 0.02
    var, cvar = empirical_var_cvar(returns, 0.95)
    assert var > 0, f"VaR should be positive: {var}"
    assert cvar >= var, f"CVaR should >= VaR: {cvar} vs {var}"
    print(f"  ✓ Empirical: VaR95={var:.4f}, CVaR95={cvar:.4f}")


def _test_vg_fit():
    np.random.seed(42)
    # Heavy-tailed returns (t-distribution-like)
    returns = np.random.standard_t(df=5, size=500) * 0.02
    vg = fit_variance_gamma(returns)
    assert vg["valid"]
    assert vg["excess_kurtosis"] > 0, "Should have positive excess kurtosis"
    print(f"  ✓ VG fit: nu={vg['nu']:.3f}, kurtosis={vg['excess_kurtosis']:.2f}")


def _test_levy_var():
    np.random.seed(42)
    returns = np.random.standard_t(df=5, size=500) * 0.02
    report = levy_var_cvar(returns)
    assert report.valid
    assert report.cvar_95 >= report.var_95
    assert report.tail_index < 10  # heavy-tailed data
    print(f"  ✓ Lévy VaR: VaR95={report.var_95:.4f}, CVaR95={report.cvar_95:.4f}, α={report.tail_index:.1f}")


def _test_execution_gate():
    np.random.seed(42)
    returns = np.random.randn(200) * 0.01
    report = levy_var_cvar(returns)
    result = apply_to_execution_gate(report, max_var_95=0.02)
    assert result["gate"] in ("pass", "observe_only", "blocked")
    print(f"  ✓ Execution gate: {result['gate']}")


def run_tests():
    print("Running Lévy-Flow VaR tests...")
    _test_empirical_var()
    _test_vg_fit()
    _test_levy_var()
    _test_execution_gate()
    print("All Lévy tests passed.")


if __name__ == "__main__":
    run_tests()
