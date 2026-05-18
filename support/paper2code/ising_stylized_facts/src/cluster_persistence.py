"""
Ising Stylized Facts — Cluster Persistence Metrics

Paper: https://arxiv.org/abs/2512.17925v1
Implements: Spin cluster analysis for financial regime detection

Key results:
  - Cluster morphology and persistence explain volatility clustering
  - Cluster reorganization → intermittent volatility + heavy tails
  - Absolute overlap |S(t)+S(t+Δt)| quantifies state correlation
  - Critical point → maximum cluster size → regime transition

For ict-engine:
  - Ising overlay validation: cluster persistence ≠ regime persistence
  - Phase transition risk calibration using cluster size distribution
  - Volatility regime detection via cluster dynamics
"""

import numpy as np
from dataclasses import dataclass


@dataclass
class ClusterStats:
    """§Abstract — Cluster morphology statistics."""
    mean_cluster_size: float
    max_cluster_size: float
    n_clusters: int
    cluster_size_std: float
    giant_component_frac: float   # largest cluster / N
    persistence: float            # fraction of spins unchanged between t and t+1
    absolute_overlap: float       # §Abstract: 1/2P Σ|S_i(t)+S_i(t+Δt)|
    valid: bool


def find_clusters(spins: np.ndarray) -> list[set]:
    """Find connected clusters of same-sign spins (1D or 2D).
    
    §Abstract — "formation of spin clusters"
    For 1D: contiguous same-sign segments.
    For 2D: BFS-connected components.
    """
    if spins.ndim == 1:
        return _find_clusters_1d(spins)
    elif spins.ndim == 2:
        return _find_clusters_2d(spins)
    return []


def _find_clusters_1d(spins: np.ndarray) -> list[set]:
    """1D cluster detection: contiguous same-sign segments."""
    clusters = []
    current = set()
    current_sign = 0
    
    for i, s in enumerate(spins):
        if s == current_sign and s != 0:
            current.add(i)
        else:
            if current:
                clusters.append(current)
            current = {i} if s != 0 else set()
            current_sign = int(np.sign(s))
    
    if current:
        clusters.append(current)
    
    return clusters


def _find_clusters_2d(spins: np.ndarray) -> list[set]:
    """2D cluster detection: BFS-connected same-sign components."""
    rows, cols = spins.shape
    visited = np.zeros_like(spins, dtype=bool)
    clusters = []
    
    for r in range(rows):
        for c in range(cols):
            if visited[r, c] or spins[r, c] == 0:
                continue
            
            # BFS
            sign = spins[r, c]
            cluster = set()
            queue = [(r, c)]
            visited[r, c] = True
            
            while queue:
                cr, cc = queue.pop(0)
                cluster.add((cr, cc))
                
                for dr, dc in [(-1,0),(1,0),(0,-1),(0,1)]:
                    nr, nc = cr+dr, cc+dc
                    if 0 <= nr < rows and 0 <= nc < cols and not visited[nr, nc]:
                        if spins[nr, nc] == sign:
                            visited[nr, nc] = True
                            queue.append((nr, nc))
            
            clusters.append(cluster)
    
    return clusters


def cluster_persistence_metrics(
    spins_t: np.ndarray,
    spins_t1: np.ndarray,
) -> ClusterStats:
    """§Abstract — Compute cluster persistence and overlap metrics.
    
    "we analyze the formation of spin clusters, their temporal persistence,
    and the morphological evolution of the system"
    
    Args:
        spins_t: spin configuration at time t, shape (N,) or (H, W)
        spins_t1: spin configuration at time t+Δt, same shape
    
    Returns:
        ClusterStats with persistence and morphology metrics
    """
    flat_t = spins_t.flatten()
    flat_t1 = spins_t1.flatten()
    n = len(flat_t)
    
    if n == 0:
        return ClusterStats(0,0,0,0,0,0,0,False)
    
    # §Abstract — Persistence: fraction of spins unchanged
    persistence = np.mean(flat_t == flat_t1)
    
    # §Abstract — Absolute overlap: 1/2P Σ|S_i(t)+S_i(t+Δt)|
    # When both spins same sign: |1+1|=2 or |-1-1|=2 → contributes 1 after /2
    # When opposite: |1+(-1)|=0 → contributes 0
    # When one is 0: |1+0|=1 → contributes 0.5
    absolute_overlap = np.mean(np.abs(flat_t + flat_t1)) / 2.0
    
    # Cluster morphology at time t
    clusters = find_clusters(spins_t)
    n_clusters = len(clusters)
    
    if n_clusters == 0:
        return ClusterStats(0, 0, 0, 0, 0, float(persistence), float(absolute_overlap), True)
    
    sizes = [len(c) for c in clusters]
    mean_size = np.mean(sizes)
    max_size = np.max(sizes)
    size_std = np.std(sizes)
    giant_frac = max_size / n
    
    return ClusterStats(
        mean_cluster_size=float(mean_size),
        max_cluster_size=float(max_size),
        n_clusters=n_clusters,
        cluster_size_std=float(size_std),
        giant_component_frac=float(giant_frac),
        persistence=float(persistence),
        absolute_overlap=float(absolute_overlap),
        valid=True,
    )


def regime_transition_indicator(
    spin_series: np.ndarray,
    window: int = 20,
) -> np.ndarray:
    """§Abstract — Detect regime transitions via cluster dynamics.
    
    "The critical structure of clusters and their reorganization over time
    thus provide a microscopic mechanism that gives rise to the
    intermittency and clustered volatility observed in prices"
    
    High cluster reorganization rate → regime transition imminent.
    
    Args:
        spin_series: (T, N) array of spin configurations over time
        window: rolling window for transition detection
    
    Returns:
        Transition indicator series, shape (T - window + 1,)
        Higher values = more cluster reorganization = regime instability
    """
    T = spin_series.shape[0]
    if T < window + 1:
        return np.array([0.0])
    
    transitions = np.zeros(T - window + 1)
    
    for i in range(T - window + 1):
        reorg_count = 0
        for j in range(window - 1):
            stats = cluster_persistence_metrics(
                spin_series[i + j], spin_series[i + j + 1]
            )
            if stats.valid:
                # Low persistence = high reorganization
                reorg_count += (1.0 - stats.persistence)
        
        transitions[i] = reorg_count / (window - 1)
    
    return transitions


def validate_ising_overlay(
    ising_magnetization_series: np.ndarray,
    realized_vol_series: np.ndarray,
) -> dict:
    """Validate ict-engine Ising overlay against stylized facts.
    
    §Abstract — If Ising is correct:
    - Cluster persistence should correlate with volatility clustering
    - Large cluster reorganization should precede vol spikes
    
    Args:
        ising_magnetization_series: |M(t)| over time
        realized_vol_series: realized volatility over time
    
    Returns:
        Dict with validation metrics
    """
    n = min(len(ising_magnetization_series), len(realized_vol_series))
    if n < 10:
        return {"valid": False, "reason": "insufficient data"}
    
    mag = ising_magnetization_series[:n]
    vol = realized_vol_series[:n]
    
    # Correlation between |magnetization| and volatility
    corr = np.corrcoef(np.abs(mag), vol)[0, 1]
    
    # Does high magnetization precede high vol?
    if n > 1:
        lead_corr = np.corrcoef(np.abs(mag[:-1]), vol[1:])[0, 1]
    else:
        lead_corr = 0.0
    
    return {
        "valid": True,
        "magnetization_vol_correlation": float(corr),
        "lead_correlation": float(lead_corr),
        "stylized_fact_consistent": abs(corr) > 0.2,
        "implication": (
            "Ising magnetization correlates with realized vol. "
            "Cluster dynamics capture volatility clustering."
            if abs(corr) > 0.2 else
            "Weak correlation. Ising overlay may not capture vol regime."
        ),
    }


# ── Tests ──────────────────────────────────────────────────────────────

def _test_cluster_detection_1d():
    spins = np.array([1, 1, -1, -1, -1, 1, 0, 1, 1])
    clusters = find_clusters(spins)
    sizes = sorted([len(c) for c in clusters], reverse=True)
    assert sizes == [3, 2, 2, 1]  # [-1,-1,-1], [1,1], [1,1], [1]
    print(f"  ✓ 1D clusters: sizes={sizes}")


def _test_persistence_identical():
    spins = np.ones(100)
    stats = cluster_persistence_metrics(spins, spins)
    assert stats.persistence == 1.0
    assert stats.absolute_overlap == 1.0
    print(f"  ✓ Identical config: persistence={stats.persistence:.2f}")


def _test_persistence_flipped():
    spins_t = np.ones(100)
    spins_t1 = -np.ones(100)
    stats = cluster_persistence_metrics(spins_t, spins_t1)
    assert stats.persistence == 0.0
    assert stats.absolute_overlap == 0.0
    print(f"  ✓ Flipped config: persistence={stats.persistence:.2f}")


def _test_transition_indicator():
    np.random.seed(42)
    # First half: all +1, second half: all -1 (regime change)
    first = np.ones((20, 50))
    second = -np.ones((20, 50))
    series = np.vstack([first, second])
    indicator = regime_transition_indicator(series, window=10)
    # Should spike near the transition point (index 20)
    assert indicator[15] > indicator[5], "Transition should increase reorg rate"
    print(f"  ✓ Transition indicator: peak at transition point")


def run_tests():
    print("Running Ising cluster persistence tests...")
    _test_cluster_detection_1d()
    _test_persistence_identical()
    _test_persistence_flipped()
    _test_transition_indicator()
    print("All Ising tests passed.")


if __name__ == "__main__":
    run_tests()
