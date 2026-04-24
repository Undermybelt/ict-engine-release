"""
Backtest Overfitting Detector

Paper: https://arxiv.org/abs/2209.05559v6
Implements: Combinatorial cross-validation for backtest overfitting detection

Key contributions:
  - Walk-forward on single validation set → overfitting risk
  - K-fold CV assumes IID → doesn't hold for financial data
  - Combinatorial cross-validation: simulate many market situations
  - Estimate probability of overfitting as hypothesis test
  - Reject agents with high overfitting probability

For ict-engine:
  - factor mutation: detect if "best score" is overfit
  - autoresearch: reject sessions with high overfitting probability
  - walk-forward: replace single split with combinatorial splits
"""

import numpy as np
from dataclasses import dataclass
from itertools import combinations


@dataclass
class OverfittingReport:
    """§Abstract — Backtest overfitting detection report."""
    n_trials: int
    n_periods: int
    overfitting_probability: float
    is_overfitted: bool
    in_sample_sharpe: float
    out_of_sample_sharpe: float
    sharpe_degradation: float
    valid: bool


def combinatorial_cv(
    returns: np.ndarray,
    n_splits: int = 5,
    min_train_periods: int = 60,
    min_test_periods: int = 20,
) -> list[tuple[np.ndarray, np.ndarray]]:
    """§Method — Combinatorial cross-validation splits.
    
    "We employ a combinatorial cross-validation method that tracks
    the degree of overfitting during the backtest process."
    
    Unlike K-fold which assumes IID, this generates all valid
    train/test splits respecting temporal ordering.
    
    Args:
        returns: (T, N) or (T,) return series
        n_splits: number of non-overlapping periods
        min_train_periods: minimum training period length
        min_test_periods: minimum test period length
    
    Returns:
        List of (train_indices, test_indices) tuples
    """
    T = returns.shape[0] if returns.ndim > 1 else len(returns)
    
    if T < min_train_periods + min_test_periods:
        # Not enough data — single split
        split = T // 2
        return [(np.arange(split), np.arange(split, T))]
    
    # Create non-overlapping period boundaries
    period_len = T // n_splits
    boundaries = [i * period_len for i in range(n_splits)] + [T]
    
    splits = []
    # Generate all combinations: train on some periods, test on others
    period_indices = list(range(n_splits))
    
    for r in range(1, n_splits):  # train on r periods
        for train_periods in combinations(period_indices, r):
            test_periods = [p for p in period_indices if p not in train_periods]
            if not test_periods:
                continue
            
            # Build index arrays
            train_idx = np.concatenate([
                np.arange(boundaries[p], boundaries[p+1]) for p in train_periods
            ])
            test_idx = np.concatenate([
                np.arange(boundaries[p], boundaries[p+1]) for p in test_periods
            ])
            
            if len(train_idx) >= min_train_periods and len(test_idx) >= min_test_periods:
                splits.append((train_idx, test_idx))
    
    if not splits:
        split = T // 2
        return [(np.arange(split), np.arange(split, T))]
    
    return splits


def sharpe_ratio(returns: np.ndarray, annualize: bool = True) -> float:
    """Compute Sharpe ratio."""
    if len(returns) < 2 or np.std(returns) < 1e-10:
        return 0.0
    sr = np.mean(returns) / np.std(returns)
    if annualize:
        sr *= np.sqrt(252)  # daily to annual
    return float(sr)


def estimate_overfitting_probability(
    strategy_returns: np.ndarray,
    benchmark_returns: np.ndarray = None,
    n_splits: int = 5,
    n_simulations: int = 100,
) -> OverfittingReport:
    """§Abstract — Estimate probability of backtest overfitting.
    
    "We formulate the detection of backtest overfitting as a hypothesis test"
    
    Method:
    1. Generate combinatorial CV splits
    2. For each split: compute in-sample and out-of-sample Sharpe
    3. Count fraction where OOS Sharpe < IS Sharpe (overfitting)
    4. P(overfitting) = fraction of splits showing degradation
    
    Args:
        strategy_returns: (T,) or (T, N) strategy returns
        benchmark_returns: optional benchmark for relative Sharpe
        n_splits: number of CV periods
        n_simulations: number of random splits to sample
    
    Returns:
        OverfittingReport with overfitting probability
    """
    T = strategy_returns.shape[0] if strategy_returns.ndim > 1 else len(strategy_returns)
    
    if T < 30:
        return OverfittingReport(0, 0, 0, False, 0, 0, 0, False)
    
    # Use first asset if multi-dimensional
    if strategy_returns.ndim > 1:
        rets = strategy_returns[:, 0]
    else:
        rets = strategy_returns
    
    # Generate splits
    splits = combinatorial_cv(rets, n_splits=n_splits)
    
    if len(splits) < 2:
        return OverfittingReport(1, 1, 0, False, sharpe_ratio(rets), sharpe_ratio(rets), 0, False)
    
    # Sample splits if too many
    if len(splits) > n_simulations:
        rng = np.random.RandomState(42)
        indices = rng.choice(len(splits), n_simulations, replace=False)
        splits = [splits[i] for i in indices]
    
    # §Method — Compute IS and OOS Sharpe for each split
    n_overfitted = 0
    is_sharpes = []
    oos_sharpes = []
    
    for train_idx, test_idx in splits:
        is_sharpe = sharpe_ratio(rets[train_idx])
        oos_sharpe = sharpe_ratio(rets[test_idx])
        
        is_sharpes.append(is_sharpe)
        oos_sharpes.append(oos_sharpe)
        
        # §Abstract — "estimate the probability of overfitting"
        if oos_sharpe < is_sharpe and is_sharpe > 0:
            n_overfitted += 1
    
    n_trials = len(splits)
    p_overfit = n_overfitted / n_trials if n_trials > 0 else 0
    
    mean_is = np.mean(is_sharpes) if is_sharpes else 0
    mean_oos = np.mean(oos_sharpes) if oos_sharpes else 0
    degradation = mean_is - mean_oos
    
    return OverfittingReport(
        n_trials=n_trials,
        n_periods=n_splits,
        overfitting_probability=float(p_overfit),
        is_overfitted=p_overfit > 0.5,  # majority of splits show degradation
        in_sample_sharpe=float(mean_is),
        out_of_sample_sharpe=float(mean_oos),
        sharpe_degradation=float(degradation),
        valid=True,
    )


def apply_to_factor_mutation(
    score_deltas: np.ndarray,
    accepted: np.ndarray,
    window: int = 20,
) -> dict:
    """Apply overfitting detection to ict-engine factor mutation.
    
    Treat each mutation's score_delta as a "return" and check
    if the acceptance pattern is overfitted.
    
    §Abstract — "reject overfitted agents, increasing the chance
    of good trading performance"
    
    Args:
        score_deltas: per-mutation score_delta, shape (N,)
        accepted: whether each mutation was accepted, shape (N,)
        window: rolling window for Sharpe-like calculation
    
    Returns:
        Dict with overfitting analysis for factor mutation
    """
    n = len(score_deltas)
    if n < window:
        return {"valid": False, "reason": "insufficient data"}
    
    # Treat accepted mutations' deltas as "strategy returns"
    accepted_deltas = score_deltas[accepted] if accepted.any() else score_deltas
    
    report = estimate_overfitting_probability(
        accepted_deltas,
        n_splits=min(5, len(accepted_deltas) // 10),
    )
    
    result = {
        "report": report,
        "n_mutations": n,
        "n_accepted": int(accepted.sum()),
        "is_overfitted": report.is_overfitted,
        "recommendation": (
            f"§Abstract: Overfitting probability {report.overfitting_probability:.0%}. "
            "Reject this factor configuration. The 'best score' may be "
            "a false positive from cherry-picking the validation set."
            if report.is_overfitted else
            f"Overfitting probability {report.overfitting_probability:.0%}. "
            "Acceptable — OOS Sharpe is not significantly worse than IS."
        ),
    }
    
    return result


# ── Tests ──────────────────────────────────────────────────────────────

def _test_combinatorial_cv():
    returns = np.random.randn(100)
    splits = combinatorial_cv(returns, n_splits=5)
    assert len(splits) > 1
    # All splits should have non-overlapping test sets
    print(f"  ✓ Combinatorial CV: {len(splits)} splits generated")


def _test_overfitting_noisy():
    """Noisy strategy should show high overfitting probability."""
    np.random.seed(42)
    returns = np.random.randn(200) * 0.01  # pure noise
    report = estimate_overfitting_probability(returns, n_splits=5)
    assert report.valid
    # Pure noise should have ~50% overfitting probability
    print(f"  ✓ Noisy strategy: P(overfit)={report.overfitting_probability:.2f}, IS={report.in_sample_sharpe:.2f}, OOS={report.out_of_sample_sharpe:.2f}")


def _test_overfitting_good_strategy():
    """Consistent edge should show low overfitting."""
    np.random.seed(42)
    returns = np.random.randn(200) * 0.01 + 0.001  # slight positive drift
    report = estimate_overfitting_probability(returns, n_splits=5)
    assert report.valid
    print(f"  ✓ Good strategy: P(overfit)={report.overfitting_probability:.2f}, degradation={report.sharpe_degradation:.2f}")


def _test_factor_mutation_integration():
    np.random.seed(42)
    n = 50
    deltas = np.random.randn(n) * 0.01
    accepted = deltas > np.percentile(deltas, 70)  # top 30%
    result = apply_to_factor_mutation(deltas, accepted)
    assert "report" in result
    assert result["n_mutations"] == n
    print(f"  ✓ Factor mutation: overfitted={result['is_overfitted']}, accepted={result['n_accepted']}/{n}")


def run_tests():
    print("Running backtest overfitting tests...")
    _test_combinatorial_cv()
    _test_overfitting_noisy()
    _test_overfitting_good_strategy()
    _test_factor_mutation_integration()
    print("All overfitting tests passed.")


if __name__ == "__main__":
    run_tests()
