"""
Red Queen's Trap — Mode Collapse Monitor

Paper: https://arxiv.org/abs/2512.15732v1
Implements: Detection of population homogeneity / portfolio concentration

Section references:
  §4.3 — Mode Collapse and Systemic Beta
  §4.3 — "Phenotypic Convergence: functionally distinct agents held nearly identical portfolios"
  §4.3 — "Endogenous Risk in heterogeneous agent models"
"""

import numpy as np
from dataclasses import dataclass
from typing import Optional


@dataclass
class ModeCollapseReport:
    """§4.3 — Mode collapse detection report."""
    
    # Portfolio concentration
    mean_correlation: float         # avg pairwise correlation of holdings
    max_correlation: float          # max pairwise correlation
    effective_n_strategies: float   # 1 / sum(p_i^2) — effective diversity
    
    # Direction alignment
    long_fraction: float            # fraction of population going long
    short_fraction: float           # fraction going short
    direction_entropy: float        # entropy of long/short distribution
    
    # Risk concentration
    correlated_drawdown_risk: float # P(all agents drawdown simultaneously)
    
    # Warnings
    warnings: list[str] = None
    
    def __post_init__(self):
        if self.warnings is None:
            self.warnings = []
    
    @property
    def is_collapsed(self) -> bool:
        """§4.3 — True if population has converged to single strategy."""
        return self.effective_n_strategies < 2.0 or self.mean_correlation > 0.8


def detect_mode_collapse(
    holdings: np.ndarray,
    directions: Optional[np.ndarray] = None,
    returns_history: Optional[np.ndarray] = None,
) -> ModeCollapseReport:
    """§4.3 — Detect mode collapse in a population of strategies.
    
    Paper: "Despite initializing the population with diverse archetypes,
    the surviving population exhibited severe Phenotypic Convergence."
    
    This maps directly to ict-engine:
    - If all factors point the same direction → mode collapse
    - If factor correlation is high → diversity is illusory
    - If autoresearch converges to same parameter set → local optimum
    
    Args:
        holdings: per-agent position weights, shape (N_agents, N_assets)
        directions: per-agent direction (1=long, -1=short, 0=neutral), shape (N_agents,)
        returns_history: historical returns for correlation, shape (T, N_agents)
    
    Returns:
        ModeCollapseReport with concentration metrics
    """
    n_agents = holdings.shape[0]
    
    # §4.3 — Pairwise correlation of holdings
    if n_agents > 1 and holdings.shape[1] > 1:
        corr_matrix = np.corrcoef(holdings)
        # Extract upper triangle (excluding diagonal)
        triu_idx = np.triu_indices(n_agents, k=1)
        pairwise_corrs = corr_matrix[triu_idx]
        mean_corr = np.nanmean(pairwise_corrs)
        max_corr = np.nanmax(pairwise_corrs) if len(pairwise_corrs) > 0 else 0.0
    else:
        mean_corr = 0.0
        max_corr = 0.0
    
    # §4.3 — Effective number of strategies (Herfindahl-based)
    # p_i = fraction of total capital held by agent i
    total_capital = np.abs(holdings).sum(axis=1)
    if total_capital.sum() > 0:
        p = total_capital / total_capital.sum()
        effective_n = 1.0 / np.sum(p ** 2)
    else:
        effective_n = float(n_agents)
    
    # Direction alignment
    if directions is not None:
        long_frac = np.mean(directions > 0)
        short_frac = np.mean(directions < 0)
        neutral_frac = np.mean(directions == 0)
        
        # Direction entropy
        probs = np.array([long_frac, short_frac, max(0.001, neutral_frac)])
        probs = probs / probs.sum()
        dir_entropy = -np.sum(probs * np.log2(probs + 1e-10))
    else:
        long_frac = 0.5
        short_frac = 0.5
        dir_entropy = 1.0
    
    # Correlated drawdown risk: P(all agents down simultaneously)
    if returns_history is not None and returns_history.shape[1] > 1:
        # Count periods where >80% of agents had negative returns
        neg_returns = returns_history < 0
        frac_neg = neg_returns.mean(axis=1)
        correlated_dd = np.mean(frac_neg > 0.8)
    else:
        correlated_dd = 0.0
    
    # Generate warnings
    warnings = []
    if mean_corr > 0.7:
        warnings.append(
            f"§4.3 — High mean correlation ({mean_corr:.2f}). "
            "Agents may have converged to same portfolio (mode collapse)."
        )
    if effective_n < 2.0:
        warnings.append(
            f"§4.3 — Effective strategies = {effective_n:.1f} (from {n_agents} agents). "
            "Population has collapsed. Endogenous risk is high."
        )
    if long_frac > 0.8 or short_frac > 0.8:
        dominant = "long" if long_frac > short_frac else "short"
        warnings.append(
            f"§4.3 — {dominant} fraction = {max(long_frac, short_frac):.0%}. "
            "Population is directionally one-sided. A mean-reversion shock will cascade."
        )
    if correlated_dd > 0.2:
        warnings.append(
            f"§4.3 — Correlated drawdown in {correlated_dd:.0%} of periods. "
            "Liquidation cascade risk is elevated."
        )
    
    return ModeCollapseReport(
        mean_correlation=float(mean_corr),
        max_correlation=float(max_corr),
        effective_n_strategies=float(effective_n),
        long_fraction=float(long_frac),
        short_fraction=float(short_frac),
        direction_entropy=float(dir_entropy),
        correlated_drawdown_risk=float(correlated_dd),
        warnings=warnings,
    )


def apply_to_factor_diversity(
    factor_scores: np.ndarray,
    factor_directions: np.ndarray,
) -> dict:
    """Apply mode collapse detection to ict-engine factor ranking.
    
    Maps Red Queen's Trap to factor diversity:
    - "agents" = factors
    - "holdings" = factor scores/weights
    - "directions" = factor directional bias (bull/bear/neutral)
    - "mode collapse" = all factors agreeing → overconfidence
    
    Args:
        factor_scores: per-factor scores, shape (N_factors,)
        factor_directions: per-factor direction (-1 to 1), shape (N_factors,)
    
    Returns:
        Dict with diversity analysis
    """
    n = len(factor_scores)
    
    # Build pseudo-holdings matrix (each factor as an "agent")
    # Use factor scores as position sizes, directions as sign
    holdings = np.abs(factor_scores).reshape(-1, 1) * np.sign(factor_directions).reshape(-1, 1)
    
    report = detect_mode_collapse(
        holdings=holdings,
        directions=np.sign(factor_directions),
    )
    
    # Factor-specific analysis
    score_std = np.std(factor_scores)
    direction_agreement = np.mean(np.sign(factor_directions) == np.sign(factor_directions[0]))
    
    result = {
        "report": report,
        "n_factors": n,
        "score_std": float(score_std),
        "direction_agreement": float(direction_agreement),
        "is_diverse": not report.is_collapsed and direction_agreement < 0.8,
    }
    
    if direction_agreement > 0.9:
        result.setdefault("warnings", []).append(
            "All factors agree on direction. This may be mode collapse, not strong signal."
        )
    
    return result


# ── Tests ──────────────────────────────────────────────────────────────

def _test_mode_collapse_detection():
    """§4.3 — Detect when all agents hold identical portfolios."""
    n_agents = 500
    n_assets = 5
    # All agents hold the same portfolio (mode collapse)
    base = np.random.randn(n_assets)
    holdings = np.tile(base, (n_agents, 1))
    
    report = detect_mode_collapse(holdings)
    assert report.mean_correlation > 0.99, f"Should be near 1: {report.mean_correlation}"
    assert report.is_collapsed
    print(f"  ✓ Mode collapse: corr={report.mean_correlation:.3f}, effective_n={report.effective_n_strategies:.1f}")


def _test_diverse_population():
    """Verify diverse population is NOT flagged as collapsed."""
    n_agents = 100
    n_assets = 10
    holdings = np.random.randn(n_agents, n_assets)
    
    report = detect_mode_collapse(holdings)
    assert not report.is_collapsed
    print(f"  ✓ Diverse: corr={report.mean_correlation:.3f}, effective_n={report.effective_n_strategies:.1f}")


def _test_direction_collapse():
    """§4.3 — Detect when 100% of population goes long."""
    n_agents = 500
    holdings = np.random.randn(n_agents, 3)
    directions = np.ones(n_agents)  # all long
    
    report = detect_mode_collapse(holdings, directions=directions)
    assert report.long_fraction == 1.0
    assert len(report.warnings) > 0
    print(f"  ✓ Direction collapse: {report.long_fraction:.0%} long, {len(report.warnings)} warnings")


def _test_factor_diversity():
    """Test applying to ict-engine factors."""
    # All factors agree (mode collapse in factor space)
    scores = np.array([0.8, 0.75, 0.7, 0.65, 0.6])
    directions = np.array([1, 1, 1, 1, 1])  # all bullish
    
    result = apply_to_factor_diversity(scores, directions)
    assert result["direction_agreement"] == 1.0
    print(f"  ✓ Factor diversity: agreement={result['direction_agreement']:.0%}, diverse={result['is_diverse']}")


def run_tests():
    print("Running mode collapse tests...")
    _test_mode_collapse_detection()
    _test_diverse_population()
    _test_direction_collapse()
    _test_factor_diversity()
    print("All mode collapse tests passed.")


if __name__ == "__main__":
    run_tests()
