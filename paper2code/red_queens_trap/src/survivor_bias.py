"""
Red Queen's Trap — Survivor Bias Detector

Paper: https://arxiv.org/abs/2512.15732v1
Implements: Detection of survivor bias in evolutionary selection

Section references:
  §4.2 — The "Stagnation-Starvation" Loop
  §4.2 — "60% of the population maintained static equity with zero contribution"
  §4.5 — Soft Budget Constraint: "Zombie agents persisted"
"""

import numpy as np
from dataclasses import dataclass, field
from typing import Optional


@dataclass
class SurvivorBiasReport:
    """§4.2 — Report on survivor bias in a population of strategies."""
    
    # Population statistics
    total_agents: int
    active_agents: int          # agents that actually traded
    stagnant_agents: int        # agents with zero trades
    zombie_agents: int          # agents bailed out (§4.5)
    
    # Performance metrics
    survivor_mean_return: float     # mean return of survivors only
    full_population_mean: float     # mean return including dead agents
    survivor_bias_gap: float        # difference (positive = overestimate)
    
    # Diversity metrics
    unique_strategies: int
    phenotypic_entropy: float       # §4.3 — lower = more collapsed
    
    # Warnings
    warnings: list[str] = field(default_factory=list)
    
    @property
    def stagnation_rate(self) -> float:
        """§4.2 — Fraction of population that never traded."""
        return self.stagnant_agents / max(1, self.total_agents)
    
    @property
    def is_biased(self) -> bool:
        """True if survivor bias is significant."""
        return self.survivor_bias_gap > 0.01  # >1% overestimate


def detect_survivor_bias(
    returns: np.ndarray,
    is_alive: np.ndarray,
    is_active: np.ndarray,
    strategies: Optional[np.ndarray] = None,
) -> SurvivorBiasReport:
    """§4.2 — Detect survivor bias in a population of strategies.
    
    Paper: "60% of the population maintained static equity with zero
    contribution until their lifespan expired."
    
    The key insight: if you only look at "surviving" agents, you
    overestimate system performance because dead agents (who lost money)
    are excluded from the sample.
    
    This is directly applicable to ict-engine's factor mutation:
    if autoresearch only reports "best attempts" without counting
    dead/failed mutations, the acceptance rate is overestimated.
    
    Args:
        returns: per-agent returns, shape (N,)
        is_alive: whether agent is still alive, shape (N,)
        is_active: whether agent made at least one trade, shape (N,)
        strategies: optional strategy labels for diversity analysis, shape (N,)
    
    Returns:
        SurvivorBiasReport with bias quantification
    """
    n = len(returns)
    
    # §4.2 — Count stagnant agents (alive but never traded)
    stagnant = np.sum(is_alive & ~is_active)
    active = np.sum(is_active)
    dead = np.sum(~is_alive)
    
    # §4.5 — Zombie detection: alive but negative return
    zombies = np.sum(is_alive & (returns < 0))
    
    # Survivor bias: compare survivor-only mean vs full population
    survivor_mask = is_alive & is_active
    if survivor_mask.any():
        survivor_mean = returns[survivor_mask].mean()
    else:
        survivor_mean = 0.0
    
    full_mean = returns.mean()
    bias_gap = survivor_mean - full_mean
    
    # §4.3 — Phenotypic diversity (strategy entropy)
    if strategies is not None:
        unique = len(np.unique(strategies))
        _, counts = np.unique(strategies, return_counts=True)
        probs = counts / counts.sum()
        entropy = -np.sum(probs * np.log2(probs + 1e-10))
    else:
        unique = 1
        entropy = 0.0
    
    # Generate warnings
    warnings = []
    if stagnant / max(1, n) > 0.3:
        warnings.append(
            f"§4.2 — {stagnant/n:.0%} agents stagnant (paper: 60%). "
            "Evolution may be selecting for inaction, not alpha."
        )
    if bias_gap > 0.02:
        warnings.append(
            f"§4.2 — Survivor bias overestimates returns by {bias_gap:.1%}. "
            "Include dead agents in performance reporting."
        )
    if zombies / max(1, n) > 0.1:
        warnings.append(
            f"§4.5 — {zombies} zombie agents detected (alive but losing). "
            "Soft budget constraint may be preventing creative destruction."
        )
    if entropy < 1.0 and unique > 1:
        warnings.append(
            f"§4.3 — Low phenotypic entropy ({entropy:.2f}). "
            "Population may be collapsing to single strategy (mode collapse)."
        )
    
    return SurvivorBiasReport(
        total_agents=n,
        active_agents=int(active),
        stagnant_agents=int(stagnant),
        zombie_agents=int(zombies),
        survivor_mean_return=float(survivor_mean),
        full_population_mean=float(full_mean),
        survivor_bias_gap=float(bias_gap),
        unique_strategies=int(unique),
        phenotypic_entropy=float(entropy),
        warnings=warnings,
    )


def apply_to_factor_mutation(
    mutation_scores_before: np.ndarray,
    mutation_scores_after: np.ndarray,
    mutation_accepted: np.ndarray,
) -> dict:
    """Apply survivor bias detection to ict-engine factor mutation results.
    
    Maps Red Queen's Trap failure modes to factor mutation:
    - "survivors" = accepted mutations
    - "dead" = rejected mutations  
    - "zombies" = accepted but score_delta near zero
    - "stagnant" = mutations that didn't change ranking
    
    Args:
        mutation_scores_before: composite scores before mutation, shape (N,)
        mutation_scores_after: composite scores after mutation, shape (N,)
        mutation_accepted: whether each mutation was accepted, shape (N,)
    
    Returns:
        Dict with bias analysis for factor mutation
    """
    n = len(mutation_scores_before)
    deltas = mutation_scores_after - mutation_scores_before
    
    # Map to Red Queen's Trap framework
    is_alive = mutation_accepted
    is_active = np.abs(deltas) > 0.001  # non-trivial change
    returns = deltas
    
    report = detect_survivor_bias(returns, is_alive, is_active)
    
    # Additional factor-mutation-specific checks
    if report.is_biased:
        report.warnings.append(
            "Factor mutation acceptance rate may be inflated by survivor bias. "
            "Report full population statistics, not just accepted mutations."
        )
    
    # §4.2 — Check for "Time-is-Life" equivalent: are we selecting for
    # frequent mutations rather than profitable ones?
    if report.active_agents > 0:
        avg_delta_accepted = deltas[mutation_accepted].mean() if mutation_accepted.any() else 0
        avg_delta_rejected = deltas[~mutation_accepted].mean() if (~mutation_accepted).any() else 0
        
        if avg_delta_accepted < 0.005 and avg_delta_accepted > 0:
            report.warnings.append(
                f"Accepted mutations have marginal improvement (avg delta={avg_delta_accepted:.4f}). "
                "May be selecting for 'frequent changers' not 'profitable changers' (§4.2)."
            )
    
    return {
        "report": report,
        "n_mutations": n,
        "n_accepted": int(mutation_accepted.sum()),
        "n_rejected": int((~mutation_accepted).sum()),
        "avg_delta_accepted": float(deltas[mutation_accepted].mean()) if mutation_accepted.any() else 0,
        "avg_delta_all": float(deltas.mean()),
        "survivor_bias_pct": float(report.survivor_bias_gap * 100),
    }


# ── Tests ──────────────────────────────────────────────────────────────

def _test_stagnation_detection():
    """§4.2 — Detect 60% stagnation rate."""
    np.random.seed(42)
    n = 500
    returns = np.random.randn(n) * 0.01
    is_alive = np.ones(n, dtype=bool)
    is_active = np.zeros(n, dtype=bool)
    is_active[:200] = True  # only 40% active (60% stagnant)
    
    report = detect_survivor_bias(returns, is_alive, is_active)
    assert report.stagnation_rate > 0.5, f"Should detect >50% stagnation: {report.stagnation_rate}"
    assert len(report.warnings) > 0, "Should generate warnings"
    print(f"  ✓ Stagnation: {report.stagnation_rate:.0%} (paper: 60%)")


def _test_survivor_bias_overestimate():
    """§4.2 — Survivor-only mean overestimates full population."""
    np.random.seed(42)
    n = 100
    returns = np.concatenate([
        np.random.randn(50) * 0.02 + 0.01,   # survivors: positive
        np.random.randn(50) * 0.02 - 0.05,   # dead: negative
    ])
    is_alive = np.concatenate([np.ones(50), np.zeros(50)]).astype(bool)
    is_active = np.ones(n, dtype=bool)
    
    report = detect_survivor_bias(returns, is_alive, is_active)
    assert report.survivor_mean_return > report.full_population_mean
    assert report.is_biased
    print(f"  ✓ Survivor bias: {report.survivor_bias_gap:.1%} overestimate")


def _test_factor_mutation_integration():
    """Test applying to factor mutation results."""
    np.random.seed(42)
    n = 50
    scores_before = np.random.uniform(0.4, 0.7, n)
    scores_after = scores_before + np.random.randn(n) * 0.02
    accepted = scores_after > scores_before
    
    result = apply_to_factor_mutation(scores_before, scores_after, accepted)
    assert result["n_mutations"] == n
    assert result["survivor_bias_pct"] >= 0
    print(f"  ✓ Factor mutation: {result['n_accepted']}/{n} accepted, bias={result['survivor_bias_pct']:.2f}%")


def run_tests():
    print("Running survivor bias tests...")
    _test_stagnation_detection()
    _test_survivor_bias_overestimate()
    _test_factor_mutation_integration()
    print("All survivor bias tests passed.")


if __name__ == "__main__":
    run_tests()
