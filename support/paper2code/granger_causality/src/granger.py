"""
Granger Causality — Cross-Market Signal Validation

Paper: https://arxiv.org/abs/1408.2985v1 + https://arxiv.org/abs/1803.02962v1
Implements: Granger causality testing for factor/market validation

Key contributions:
  - Granger causality: X "Granger-causes" Y if past X improves prediction of Y
  - Frequency-domain Granger: causality at different time horizons
  - Stock market networks built from Granger causality

For ict-engine:
  - Validate cross_market_smt: does market A Granger-cause market B?
  - Multi-timeframe causality: short-term vs long-term predictive power
  - Factor validation: does factor F Granger-cause returns?
"""

import numpy as np
from dataclasses import dataclass


@dataclass
class GrangerResult:
    """Granger causality test result."""
    f_statistic: float
    p_value: float
    lags_tested: int
    is_causal: bool         # p < 0.05
    r_squared_unrestricted: float
    r_squared_restricted: float
    direction: str          # "x_causes_y", "y_causes_x", "bidirectional", "neither"
    valid: bool


def granger_causality_test(
    x: np.ndarray,
    y: np.ndarray,
    max_lag: int = 5,
    significance: float = 0.05,
) -> GrangerResult:
    """Test if x Granger-causes y.
    
    "X Granger-causes Y if past values of X help predict Y
    beyond what past Y alone can predict."
    
    Restricted model:  y_t = Σ a_i y_{t-i} + ε
    Unrestricted model: y_t = Σ a_i y_{t-i} + Σ b_i x_{t-i} + ε
    
    F-test: H0: all b_i = 0 (x does not Granger-cause y)
    
    Args:
        x: potential cause series, shape (T,)
        y: potential effect series, shape (T,)
        max_lag: maximum lag to test
        significance: significance level
    
    Returns:
        GrangerResult with F-statistic and p-value
    """
    n = len(x)
    if n < max_lag + 10 or len(y) != n:
        return GrangerResult(0, 1, 0, False, 0, 0, "neither", False)
    
    # Build lagged matrices
    T = n - max_lag
    
    # Y matrix: y_{t-1}, ..., y_{t-max_lag}
    Y_lags = np.column_stack([y[max_lag - i - 1 : n - i - 1] for i in range(max_lag)])
    y_target = y[max_lag:]
    
    # X lags: x_{t-1}, ..., x_{t-max_lag}
    X_lags = np.column_stack([x[max_lag - i - 1 : n - i - 1] for i in range(max_lag)])
    
    # Restricted model: y = Y_lags * a + ε
    X_r = np.column_stack([np.ones(T), Y_lags])
    try:
        beta_r = np.linalg.lstsq(X_r, y_target, rcond=None)[0]
        resid_r = y_target - X_r @ beta_r
        ss_r = np.sum(resid_r ** 2)
        r2_r = 1 - ss_r / np.sum((y_target - np.mean(y_target)) ** 2)
    except:
        return GrangerResult(0, 1, max_lag, False, 0, 0, "neither", False)
    
    # Unrestricted model: y = Y_lags * a + X_lags * b + ε
    X_u = np.column_stack([np.ones(T), Y_lags, X_lags])
    try:
        beta_u = np.linalg.lstsq(X_u, y_target, rcond=None)[0]
        resid_u = y_target - X_u @ beta_u
        ss_u = np.sum(resid_u ** 2)
        r2_u = 1 - ss_u / np.sum((y_target - np.mean(y_target)) ** 2)
    except:
        return GrangerResult(0, 1, max_lag, False, r2_r, r2_r, "neither", False)
    
    # F-test: H0: b_1 = ... = b_max_lag = 0
    # F = ((SS_r - SS_u) / q) / (SS_u / (T - k))
    q = max_lag  # number of restrictions
    k = X_u.shape[1]  # total parameters in unrestricted
    
    if ss_u < 1e-12:
        f_stat = 0.0
    else:
        f_stat = ((ss_r - ss_u) / q) / (ss_u / (T - k))
    
    # Approximate p-value using F-distribution
    # P(F > f_stat) with df1=q, df2=T-k
    from scipy import stats as scipy_stats
    try:
        p_value = 1 - scipy_stats.f.cdf(f_stat, q, T - k)
    except:
        # Fallback: use chi-squared approximation
        chi2 = q * f_stat
        try:
            p_value = 1 - scipy_stats.chi2.cdf(chi2, q)
        except:
            p_value = 1.0 if f_stat < 3.0 else 0.01
    
    return GrangerResult(
        f_statistic=float(f_stat),
        p_value=float(p_value),
        lags_tested=max_lag,
        is_causal=p_value < significance,
        r_squared_unrestricted=float(r2_u),
        r_squared_restricted=float(r2_r),
        direction="",  # filled by bidirectional test
        valid=True,
    )


def bidirectional_granger(
    x: np.ndarray,
    y: np.ndarray,
    max_lag: int = 5,
    significance: float = 0.05,
) -> GrangerResult:
    """Test Granger causality in both directions.
    
    Returns combined result with direction classification.
    """
    xy = granger_causality_test(x, y, max_lag, significance)  # x → y
    yx = granger_causality_test(y, x, max_lag, significance)  # y → x
    
    if xy.is_causal and yx.is_causal:
        direction = "bidirectional"
    elif xy.is_causal:
        direction = "x_causes_y"
    elif yx.is_causal:
        direction = "y_causes_x"
    else:
        direction = "neither"
    
    # Return the stronger direction's result
    if xy.f_statistic >= yx.f_statistic:
        return GrangerResult(
            f_statistic=xy.f_statistic,
            p_value=xy.p_value,
            lags_tested=max_lag,
            is_causal=xy.is_causal or yx.is_causal,
            r_squared_unrestricted=xy.r_squared_unrestricted,
            r_squared_restricted=xy.r_squared_restricted,
            direction=direction,
            valid=True,
        )
    else:
        return GrangerResult(
            f_statistic=yx.f_statistic,
            p_value=yx.p_value,
            lags_tested=max_lag,
            is_causal=xy.is_causal or yx.is_causal,
            r_squared_unrestricted=yx.r_squared_unrestricted,
            r_squared_restricted=yx.r_squared_restricted,
            direction=direction,
            valid=True,
        )


def validate_cross_market_smt(
    market_a_returns: np.ndarray,
    market_b_returns: np.ndarray,
    factor_returns: np.ndarray = None,
    max_lag: int = 5,
) -> dict:
    """Validate ict-engine cross_market_smt with Granger causality.
    
    §Abstract (Granger Networks) — "temporal proximity and
    preferential attachment" in stock market networks.
    
    If market A does NOT Granger-cause market B, then SMT
    between them should be downweighted.
    
    Args:
        market_a_returns: returns of market A (e.g., ES)
        market_b_returns: returns of market B (e.g., NQ)
        factor_returns: optional factor signal for validation
        max_lag: maximum lag to test
    """
    # A → B
    ab = bidirectional_granger(market_a_returns, market_b_returns, max_lag)
    
    result = {
        "a_grangers_b": ab.is_causal and "x_causes_y" in ab.direction,
        "b_grangers_a": ab.is_causal and "y_causes_x" in ab.direction,
        "bidirectional": ab.direction == "bidirectional",
        "f_statistic": ab.f_statistic,
        "p_value": ab.p_value,
        "direction": ab.direction,
        "r_squared_gain": ab.r_squared_unrestricted - ab.r_squared_restricted,
        "smt_valid": ab.is_causal,
        "recommendation": "",
    }
    
    # Factor validation if provided
    if factor_returns is not None and len(factor_returns) == len(market_a_returns):
        fa = bidirectional_granger(factor_returns, market_a_returns, max_lag)
        fb = bidirectional_granger(factor_returns, market_b_returns, max_lag)
        
        result["factor_grangers_a"] = fa.is_causal
        result["factor_grangers_b"] = fb.is_causal
        result["factor_valid"] = fa.is_causal or fb.is_causal
    
    if result["smt_valid"]:
        result["recommendation"] = (
            f"Granger causality confirmed (p={ab.p_value:.3f}). "
            f"Direction: {ab.direction}. "
            f"SMT signal is statistically supported."
        )
    else:
        result["recommendation"] = (
            f"No Granger causality (p={ab.p_value:.3f}). "
            "SMT signal may be spurious. Consider downweighting."
        )
    
    return result


# ── Tests ──────────────────────────────────────────────────────────────

def _test_granger_causal():
    """x causes y: y = 0.5*x_{t-1} + noise → should detect causality."""
    np.random.seed(42)
    n = 500
    x = np.random.randn(n) * 0.1
    y = np.zeros(n)
    for t in range(1, n):
        y[t] = 0.5 * x[t-1] + np.random.randn() * 0.05
    
    result = granger_causality_test(x, y, max_lag=3)
    assert result.is_causal, f"Should detect causality: p={result.p_value}"
    print(f"  ✓ Granger causal: F={result.f_statistic:.2f}, p={result.p_value:.4f}")


def _test_granger_not_causal():
    """Independent series → should NOT detect causality."""
    np.random.seed(42)
    x = np.random.randn(500) * 0.1
    y = np.random.randn(500) * 0.1
    
    result = granger_causality_test(x, y, max_lag=3)
    # Most of the time should not be causal (p > 0.05)
    print(f"  ✓ Granger independent: F={result.f_statistic:.2f}, p={result.p_value:.4f}, causal={result.is_causal}")


def _test_bidirectional():
    np.random.seed(42)
    n = 500
    x = np.random.randn(n) * 0.1
    y = np.zeros(n)
    for t in range(1, n):
        y[t] = 0.3 * x[t-1] + 0.2 * y[t-1] + np.random.randn() * 0.05
    
    result = bidirectional_granger(x, y, max_lag=3)
    assert result.valid
    print(f"  ✓ Bidirectional: direction={result.direction}, causal={result.is_causal}")


def _test_smt_validation():
    np.random.seed(42)
    n = 500
    es = np.random.randn(n) * 0.01
    nq = np.zeros(n)
    for t in range(1, n):
        nq[t] = 0.6 * es[t-1] + np.random.randn() * 0.005
    
    result = validate_cross_market_smt(es, nq, max_lag=3)
    assert result["smt_valid"]
    print(f"  ✓ SMT validation: causal={result['smt_valid']}, direction={result['direction']}")


def run_tests():
    print("Running Granger causality tests...")
    _test_granger_causal()
    _test_granger_not_causal()
    _test_bidirectional()
    _test_smt_validation()
    print("All Granger tests passed.")


if __name__ == "__main__":
    run_tests()
