"""
Crowded Trades — Market Clustering Measure

Paper: https://arxiv.org/abs/2002.03319v1
Implements: Trading overlap detection and herding bias quantification

Key results:
  - Market clustering has CAUSAL effect on price instability (especially positive tail)
  - Higher clustering → heavier tails, higher kurtosis
  - Effect stronger for positive tail (VLuck) than negative tail (VaR)
  - Clustering is persistent, driven by illiquidity and market cap

For ict-engine:
  - Herding bias verification for Ising overlay
  - Factor diversity monitoring
  - Cross-market crowding detection
"""

import numpy as np
from dataclasses import dataclass, field
from typing import Optional


@dataclass
class ClusteringResult:
    """Market clustering measurement result.
    
    §Abstract — "market clustering measure captures the degree of
    trading overlap among any two investors in that stock."
    """
    clustering_score: float     # 0-1, higher = more crowded
    n_agents: int
    n_assets: int
    mean_overlap: float         # average pairwise overlap
    max_overlap: float          # max pairwise overlap
    effective_diversity: float  # 1 / Herfindahl
    valid: bool


def compute_trading_overlap(
    trading_matrix: np.ndarray,
) -> np.ndarray:
    """§2 — Compute pairwise trading overlap.
    
    For each pair of agents (i, j), compute the fraction of assets
    they both trade (overlap / union).
    
    Args:
        trading_matrix: binary matrix (N_agents, N_assets),
                        1 if agent trades asset, 0 otherwise
    
    Returns:
        Overlap matrix (N_agents, N_agents), symmetric
    """
    n_agents = trading_matrix.shape[0]
    
    # Intersection: both agents trade the asset
    intersection = trading_matrix @ trading_matrix.T  # (N, N)
    
    # Union: |A| + |B| - |A∩B|
    trade_counts = trading_matrix.sum(axis=1)  # (N,)
    union = trade_counts[:, None] + trade_counts[None, :] - intersection
    
    # Overlap = intersection / union (Jaccard similarity)
    overlap = np.where(union > 0, intersection / union, 0.0)
    
    # Zero out diagonal
    np.fill_diagonal(overlap, 0.0)
    
    return overlap


def measure_market_clustering(
    trading_matrix: np.ndarray,
    weights: Optional[np.ndarray] = None,
) -> ClusteringResult:
    """§2 — Measure market clustering from trading data.
    
    "For each stock the clustering measure captures the degree of
    trading overlap among any two investors in that stock."
    
    Args:
        trading_matrix: binary (N_agents, N_assets) or weighted
        weights: optional per-agent weights (capital), shape (N_agents,)
    
    Returns:
        ClusteringResult with clustering metrics
    """
    n_agents, n_assets = trading_matrix.shape
    
    if n_agents < 2:
        return ClusteringResult(
            clustering_score=0.0, n_agents=n_agents, n_assets=n_assets,
            mean_overlap=0.0, max_overlap=0.0, effective_diversity=1.0,
            valid=False,
        )
    
    # §2 — Pairwise overlap
    overlap = compute_trading_overlap(trading_matrix)
    
    # Extract upper triangle (unique pairs)
    triu_idx = np.triu_indices(n_agents, k=1)
    pairwise_overlaps = overlap[triu_idx]
    
    # Weighted or unweighted mean
    if weights is not None:
        # Weight by product of agent weights
        W = weights[:, None] * weights[None, :]
        W_triu = W[triu_idx]
        if W_triu.sum() > 0:
            mean_overlap = np.average(pairwise_overlaps, weights=W_triu)
        else:
            mean_overlap = np.mean(pairwise_overlaps)
    else:
        mean_overlap = np.mean(pairwise_overlaps) if len(pairwise_overlaps) > 0 else 0.0
    
    max_overlap = float(np.max(pairwise_overlaps)) if len(pairwise_overlaps) > 0 else 0.0
    
    # Effective diversity (Herfindahl-based)
    if weights is not None and weights.sum() > 0:
        p = weights / weights.sum()
    else:
        trade_counts = trading_matrix.sum(axis=1).astype(float)
        p = trade_counts / trade_counts.sum() if trade_counts.sum() > 0 else np.ones(n_agents) / n_agents
    
    herfindahl = np.sum(p ** 2)
    effective_diversity = 1.0 / herfindahl if herfindahl > 0 else float(n_agents)
    
    return ClusteringResult(
        clustering_score=float(mean_overlap),
        n_agents=n_agents,
        n_assets=n_assets,
        mean_overlap=float(mean_overlap),
        max_overlap=max_overlap,
        effective_diversity=float(effective_diversity),
        valid=True,
    )


def herding_bias_from_ising(
    spin_series: np.ndarray,
) -> dict:
    """§Abstract — Verify Ising herding bias against crowding evidence.
    
    Maps the crowded trades paper's clustering concept to ict-engine's
    Ising overlay:
    - Ising magnetization ≈ directional agreement
    - Ising coupling ≈ trading overlap
    - High clustering + high magnetization = herding confirmed
    
    Args:
        spin_series: agent spin values (-1 or +1), shape (N_agents,) or (T, N_agents)
    
    Returns:
        Dict with herding bias diagnostics
    """
    if spin_series.ndim == 1:
        spins = spin_series.reshape(1, -1)
    else:
        spins = spin_series
    
    T, N = spins.shape
    
    # Magnetization: mean spin (§Abstract — "degree of trading overlap")
    magnetization = np.mean(spins, axis=1)  # (T,)
    
    # Direction agreement: fraction of agents with same sign as majority
    majority_sign = np.sign(magnetization)
    agreement = np.mean(
        spins == majority_sign[:, None], axis=1
    )  # (T,)
    
    # Pairwise correlation of spins (as proxy for clustering)
    if N > 1:
        spin_corr = np.corrcoef(spins.T)  # (N, N)
        triu_idx = np.triu_indices(N, k=1)
        mean_spin_corr = np.nanmean(spin_corr[triu_idx])
    else:
        mean_spin_corr = 0.0
    
    # §Abstract — "causal effect on positive tail"
    # High positive magnetization = crowded long = positive tail risk
    # High negative magnetization = crowded short = negative tail risk
    pos_magnetization = np.mean(magnetization[magnetization > 0]) if np.any(magnetization > 0) else 0.0
    neg_magnetization = np.mean(magnetization[magnetization < 0]) if np.any(magnetization < 0) else 0.0
    
    return {
        "mean_magnetization": float(np.mean(magnetization)),
        "mean_agreement": float(np.mean(agreement)),
        "mean_spin_correlation": float(mean_spin_corr),
        "positive_crowding": float(pos_magnetization),
        "negative_crowding": float(abs(neg_magnetization)),
        "herding_detected": float(np.mean(agreement)) > 0.7,
        "tail_risk_asymmetric": abs(pos_magnetization) > abs(neg_magnetization) * 1.5
                              or abs(neg_magnetization) > abs(pos_magnetization) * 1.5,
        "implication": (
            "§Abstract: Crowded LONG positions → positive tail risk elevated. "
            if pos_magnetization > 0.7 else
            "§Abstract: Crowded SHORT positions → negative tail risk elevated. "
            if abs(neg_magnetization) > 0.7 else
            "Balanced positioning. No significant crowding detected."
        ),
    }


def apply_to_ict_ising_overlay(
    ising_magnetization: float,
    ising_coupling: float,
    factor_directions: np.ndarray,
) -> dict:
    """Apply crowded trades findings to ict-engine Ising overlay.
    
    §Abstract — "market clustering has a causal effect on the properties
    of the tails of the stock return distribution, particularly the
    positive tail"
    
    Mapping:
    - ising_magnetization → directional crowding
    - ising_coupling → trading overlap intensity
    - factor_directions → verify if factors also agree (double crowding)
    
    Args:
        ising_magnetization: current Ising magnetization (-1 to 1)
        ising_coupling: current coupling strength
        factor_directions: directional bias of each factor (-1 to 1)
    
    Returns:
        Dict with crowding risk assessment for ict-engine
    """
    # Factor agreement
    if len(factor_directions) > 0:
        factor_agreement = np.mean(np.sign(factor_directions) == np.sign(factor_directions[0]))
    else:
        factor_agreement = 0.0
    
    # Combined crowding signal
    # Both Ising and factors agreeing = double crowding risk
    ising_crowded = abs(ising_magnetization) > 0.6
    factors_crowded = factor_agreement > 0.8
    
    # §Abstract — Positive tail risk (crowded long)
    # §Abstract — Negative tail risk (crowded short) — but only in turmoil
    direction = "long" if ising_magnetization > 0 else "short"
    
    risk_level = "low"
    if ising_crowded and factors_crowded:
        risk_level = "high"  # double crowding
    elif ising_crowded or factors_crowded:
        risk_level = "medium"
    
    # Recommended action
    if risk_level == "high":
        action = (
            f"§Abstract: {direction.upper()} crowding detected (Ising + factors). "
            f"{'Positive' if direction == 'long' else 'Negative'} tail risk elevated. "
            "Consider reducing position size or widening stop-loss."
        )
    elif risk_level == "medium":
        action = f"Moderate {direction} crowding. Monitor for escalation."
    else:
        action = "No significant crowding. Normal execution."
    
    return {
        "ising_magnetization": ising_magnetization,
        "ising_coupling": ising_coupling,
        "factor_agreement": factor_agreement,
        "crowding_direction": direction,
        "risk_level": risk_level,
        "double_crowding": ising_crowded and factors_crowded,
        "action": action,
    }


# ── Tests ──────────────────────────────────────────────────────────────

def _test_overlap_identical():
    """All agents trade same assets → overlap = 1."""
    matrix = np.ones((5, 3))
    result = measure_market_clustering(matrix)
    assert result.clustering_score > 0.99
    print(f"  ✓ Identical trading: clustering={result.clustering_score:.3f}")


def _test_overlap_disjoint():
    """Each agent trades different assets → overlap ≈ 0."""
    matrix = np.eye(5)  # 5 agents, 5 assets, each trades one unique
    result = measure_market_clustering(matrix)
    assert result.clustering_score < 0.01
    print(f"  ✓ Disjoint trading: clustering={result.clustering_score:.3f}")


def _test_herding_bias():
    """Test herding detection from spin series."""
    np.random.seed(42)
    # All spins +1 (herding)
    spins = np.ones((10, 50))
    result = herding_bias_from_ising(spins)
    assert result["herding_detected"]
    assert result["mean_agreement"] == 1.0
    print(f"  ✓ Herding detected: agreement={result['mean_agreement']:.0%}")


def _test_ict_integration():
    """Test Ising overlay integration."""
    result = apply_to_ict_ising_overlay(
        ising_magnetization=0.8,
        ising_coupling=1.5,
        factor_directions=np.array([1, 1, 1, 1, -0.5]),
    )
    assert result["risk_level"] in ("medium", "high")
    print(f"  ✓ ICT integration: risk={result['risk_level']}, direction={result['crowding_direction']}")


def run_tests():
    print("Running crowded trades tests...")
    _test_overlap_identical()
    _test_overlap_disjoint()
    _test_herding_bias()
    _test_ict_integration()
    print("All crowded trades tests passed.")


if __name__ == "__main__":
    run_tests()
