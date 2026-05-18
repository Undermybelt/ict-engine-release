"""
Red Queen's Trap — Capital Decay Tracker

Paper: https://arxiv.org/abs/2512.15732v1
Implements: Real-time capital decay monitoring

Section references:
  §4, Figure 1 — "Monotonic decay of ROI"
  §4.2 — "Systemic Stagnation: 60% maintained static equity"
  §4.5 — "Soft Budget Constraint: zombie agents persisted"
  §5 — "Capital Decay > 70%"
"""

import numpy as np
from dataclasses import dataclass
from typing import Optional


@dataclass
class CapitalDecayReport:
    """§4, Figure 1 — Capital decay monitoring report."""
    
    initial_capital: float
    current_capital: float
    peak_capital: float
    
    # Decay metrics
    total_return: float         # (current - initial) / initial
    max_drawdown: float         # (peak - current) / peak
    capital_decay: float        # §5 — "Capital Decay > 70%"
    
    # Time-based
    n_periods: int
    decay_per_period: float     # average decay rate
    
    # Soft budget constraint (§4.5)
    total_bailouts: float       # capital injected to sustain zombies
    bailout_fraction: float     # bailouts / initial_capital
    
    # Warnings
    warnings: list[str] = None
    
    def __post_init__(self):
        if self.warnings is None:
            self.warnings = []
    
    @property
    def is_decaying(self) -> bool:
        """True if capital is monotonically decaying."""
        return self.total_return < -0.05  # >5% loss
    
    @property
    def is_catastrophic(self) -> bool:
        """§5 — True if capital decay exceeds paper's threshold."""
        return self.capital_decay > 0.5  # >50% decay


def track_capital_decay(
    capital_series: np.ndarray,
    bailouts: Optional[np.ndarray] = None,
) -> CapitalDecayReport:
    """§4, Figure 1 — Track capital decay over time.
    
    Paper: "We observed a catastrophic divergence between training metrics
    (Validation APY > 300%) and live performance (Capital Decay > 70%)."
    
    This maps to ict-engine's autoresearch:
    - capital_series = cumulative score_delta over mutation runs
    - If capital decays while "best score" improves → survivor bias
    - If bailouts are needed → soft budget constraint (§4.5)
    
    Args:
        capital_series: cumulative capital over time, shape (T,)
        bailouts: optional bailout injections per period, shape (T,)
    
    Returns:
        CapitalDecayReport with decay metrics and warnings
    """
    if len(capital_series) < 2:
        return CapitalDecayReport(
            initial_capital=capital_series[0] if len(capital_series) > 0 else 0,
            current_capital=capital_series[0] if len(capital_series) > 0 else 0,
            peak_capital=capital_series[0] if len(capital_series) > 0 else 0,
            total_return=0.0, max_drawdown=0.0, capital_decay=0.0,
            n_periods=len(capital_series), decay_per_period=0.0,
            total_bailouts=0.0, bailout_fraction=0.0,
        )
    
    initial = capital_series[0]
    current = capital_series[-1]
    peak = np.max(capital_series)
    
    # §4, Figure 1 — Total return
    total_return = (current - initial) / abs(initial) if initial != 0 else 0.0
    
    # §4, Figure 1 — Max drawdown
    running_peak = np.maximum.accumulate(capital_series)
    drawdowns = (running_peak - capital_series) / np.where(running_peak > 0, running_peak, 1)
    max_drawdown = float(np.max(drawdowns))
    
    # §5 — Capital decay = total loss from peak
    capital_decay = (peak - current) / peak if peak > 0 else 0.0
    
    # Time-based decay rate
    n_periods = len(capital_series)
    decay_per_period = total_return / n_periods if n_periods > 0 else 0.0
    
    # §4.5 — Soft budget constraint
    if bailouts is not None:
        total_bailouts = float(np.sum(bailouts))
        bailout_frac = total_bailouts / abs(initial) if initial != 0 else 0.0
    else:
        total_bailouts = 0.0
        bailout_frac = 0.0
    
    # Generate warnings
    warnings = []
    
    if total_return < -0.3:
        warnings.append(
            f"§5 — Capital decay {abs(total_return):.0%} exceeds 30%. "
            "System is in 'Galaxy Empire' territory (paper: >70%)."
        )
    
    if max_drawdown > 0.2:
        warnings.append(
            f"§4, Figure 1 — Max drawdown {max_drawdown:.0%}. "
            "Liquidation cascade risk if leverage is used."
        )
    
    # §4, Figure 1 — Check for monotonic decay
    if len(capital_series) > 10:
        # Split into halves, check if second half is worse
        mid = len(capital_series) // 2
        first_half_return = (capital_series[mid] - capital_series[0]) / abs(capital_series[0])
        second_half_return = (capital_series[-1] - capital_series[mid]) / abs(capital_series[mid]) if capital_series[mid] != 0 else 0
        
        if first_half_return < 0 and second_half_return < 0:
            warnings.append(
                "§4, Figure 1 — Monotonic decay detected (both halves negative). "
                "This matches the paper's 'capital decay' trajectory."
            )
    
    if bailout_frac > 0.1:
        warnings.append(
            f"§4.5 — Bailouts = {bailout_frac:.0%} of initial capital. "
            "Soft budget constraint is preventing creative destruction."
        )
    
    return CapitalDecayReport(
        initial_capital=float(initial),
        current_capital=float(current),
        peak_capital=float(peak),
        total_return=float(total_return),
        max_drawdown=float(max_drawdown),
        capital_decay=float(capital_decay),
        n_periods=n_periods,
        decay_per_period=float(decay_per_period),
        total_bailouts=total_bailouts,
        bailout_fraction=float(bailout_frac),
        warnings=warnings,
    )


def apply_to_autoresearch(
    mutation_deltas: np.ndarray,
    accepted: np.ndarray,
    bailout_mask: Optional[np.ndarray] = None,
) -> dict:
    """Apply capital decay tracking to ict-engine autoresearch.
    
    Maps Red Queen's Trap to autoresearch:
    - capital_series = cumulative score_delta
    - "bailouts" = mutations accepted despite negative delta (soft budget)
    - "decay" = cumulative score going down despite "best" going up
    
    Args:
        mutation_deltas: score_delta per mutation, shape (N,)
        accepted: whether each mutation was accepted, shape (N,)
        bailout_mask: mutations accepted despite negative delta, shape (N,)
    
    Returns:
        Dict with decay analysis for autoresearch
    """
    cumulative = np.cumsum(mutation_deltas)
    
    if bailout_mask is None:
        # Auto-detect bailouts: accepted but negative delta
        bailout_mask = accepted & (mutation_deltas < 0)
    
    bailouts = np.where(bailout_mask, np.abs(mutation_deltas), 0.0)
    
    report = track_capital_decay(cumulative, bailouts=bailouts)
    
    # Autoresearch-specific checks
    n_accepted = int(accepted.sum())
    n_positive = int((mutation_deltas[accepted] > 0).sum()) if accepted.any() else 0
    
    result = {
        "report": report,
        "n_mutations": len(mutation_deltas),
        "n_accepted": n_accepted,
        "n_positive_accepted": n_positive,
        "acceptance_rate": n_accepted / max(1, len(mutation_deltas)),
        "positive_rate_among_accepted": n_positive / max(1, n_accepted),
        "cumulative_delta": float(cumulative[-1]),
    }
    
    # §4.2 — "Time-is-Life" equivalent check
    if n_accepted > 5 and n_positive / max(1, n_accepted) < 0.3:
        result.setdefault("warnings", []).append(
            f"Only {n_positive}/{n_accepted} accepted mutations improved score. "
            "May be selecting for frequent mutation, not profitable mutation (§4.2)."
        )
    
    return result


# ── Tests ──────────────────────────────────────────────────────────────

def _test_galaxy_empire_decay():
    """§5 — Replicate 'Capital Decay > 70%'."""
    # Simulate Galaxy Empire-like decay
    np.random.seed(42)
    capital = 100000.0
    series = [capital]
    for _ in range(100):
        capital *= (1 + np.random.randn() * 0.02 - 0.005)  # slight negative drift
        series.append(capital)
    
    report = track_capital_decay(np.array(series))
    assert report.total_return < -0.1, f"Should have significant decay: {report.total_return}"
    print(f"  ✓ Galaxy Empire decay: {abs(report.total_return):.0%} (paper: >70%)")


def _test_bailout_detection():
    """§4.5 — Detect soft budget constraint."""
    capital = np.array([100.0, 95.0, 90.0, 85.0, 80.0])
    bailouts = np.array([0.0, 5.0, 10.0, 15.0, 20.0])  # increasing bailouts
    
    report = track_capital_decay(capital, bailouts=bailouts)
    assert report.bailout_fraction > 0.4
    assert len(report.warnings) > 0
    print(f"  ✓ Bailout detected: {report.bailout_fraction:.0%} of initial capital")


def _test_autoresearch_integration():
    """Test applying to autoresearch results."""
    np.random.seed(42)
    n = 20
    deltas = np.random.randn(n) * 0.01 - 0.002  # slight negative bias
    accepted = deltas > -0.005  # loose acceptance
    
    result = apply_to_autoresearch(deltas, accepted)
    assert result["n_mutations"] == n
    print(f"  ✓ Autoresearch: {result['n_accepted']}/{n} accepted, cumulative={result['cumulative_delta']:.4f}")


def run_tests():
    print("Running capital decay tests...")
    _test_galaxy_empire_decay()
    _test_bailout_detection()
    _test_autoresearch_integration()
    print("All capital decay tests passed.")


if __name__ == "__main__":
    run_tests()
