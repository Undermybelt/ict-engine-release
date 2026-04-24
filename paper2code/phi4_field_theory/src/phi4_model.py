"""
φ⁴ Field Theory — Continuous Alternative to Ising

Paper: https://arxiv.org/abs/2512.17225v1
Implements: Continuous field theory for financial time series

Key contributions:
  - φ⁴ is continuous → avoids Ising's discretization error
  - Can reproduce kurtosis (Ising cannot with binarized data)
  - Inhomogeneous couplings w_ij model cross-asset correlations
  - Explicit symmetry-breaking models external market forces
  - 2008 crisis kurtosis reproduction far exceeds Ising

For ict-engine:
  - Sprint 4+ upgrade path from Ising binary spins to continuous fields
  - Kurtosis estimation as tail risk indicator
  - Cross-asset coupling matrix for SMT
"""

import numpy as np
from dataclasses import dataclass


@dataclass
class Phi4State:
    """§Abstract — φ⁴ field theory state.
    
    Each stock price change is mapped to a field φ_i.
    Hamiltonian: H = -Σ w_ij φ_i φ_j - Σ h_i φ_i + Σ (φ_i² - 1)²
    
    The (φ²-1)² term forces fields toward ±1 (like Ising) but
    allows continuous intermediate values.
    """
    fields: np.ndarray          # φ_i values, continuous in ℝ
    couplings: np.ndarray       # w_ij matrix (symmetric)
    external_fields: np.ndarray # h_i (symmetry-breaking)
    temperature: float          # T (controls fluctuations)


def phi4_hamiltonian(state: Phi4State) -> float:
    """§Eq — φ⁴ Hamiltonian.
    
    H = -Σ_{i<j} w_ij φ_i φ_j - Σ h_i φ_i + λ Σ (φ_i² - 1)²
    
    The last term is the φ⁴ potential: forces |φ| toward 1
    but allows continuous values (unlike Ising which is discrete).
    """
    phi = state.fields
    w = state.couplings
    h = state.external_fields
    
    # Interaction term: -Σ w_ij φ_i φ_j
    interaction = -0.5 * phi @ w @ phi
    
    # External field term: -Σ h_i φ_i
    external = -np.sum(h * phi)
    
    # φ⁴ potential: λ Σ (φ² - 1)² — forces toward ±1 but continuous
    lam = 1.0  # [UNSPECIFIED] — quartic coupling
    potential = lam * np.sum((phi**2 - 1)**2)
    
    return float(interaction + external + potential)


def estimate_phi4_from_returns(
    returns: np.ndarray,
    n_steps: int = 1000,
    lr: float = 0.01,
) -> Phi4State:
    """§Abstract — Estimate φ⁴ parameters from return data.
    
    "We use a φ⁴ quantum field theory with inhomogeneous couplings
    and explicit symmetry-breaking to model an ensemble of financial
    time series"
    
    Simple gradient descent to fit w_ij and h_i to observed returns.
    
    Args:
        returns: (T, N) array of asset returns
        n_steps: optimization steps
        lr: learning rate
    
    Returns:
        Fitted Phi4State
    """
    T, N = returns.shape
    
    # Normalize returns to [-1, 1] range for φ fields
    scale = np.abs(returns).max() + 1e-10
    normalized = np.clip(returns / scale, -0.99, 0.99)
    
    # Initialize couplings from return correlations
    corr = np.corrcoef(returns.T)
    np.fill_diagonal(corr, 0)
    couplings = corr * 0.5
    
    # Initialize external fields from mean returns
    external = np.mean(normalized, axis=0)
    
    # Fit via simple gradient descent on reconstruction loss
    for step in range(n_steps):
        # Pick a random time step
        t = np.random.randint(T)
        phi = normalized[t]
        
        # Gradient of H w.r.t. w_ij: dH/dw_ij = -φ_i φ_j
        grad_w = -np.outer(phi, phi)
        # Gradient w.r.t. h_i: dH/dh_i = -φ_i
        grad_h = -phi
        
        # Update (gradient descent on energy)
        couplings += lr * grad_w * 0.001  # small step
        external += lr * grad_h * 0.001
        
        # Symmetrize couplings
        couplings = (couplings + couplings.T) / 2
        np.fill_diagonal(couplings, 0)
    
    # Final state: use last observed returns as fields
    final_fields = normalized[-1]
    
    return Phi4State(
        fields=final_fields,
        couplings=couplings,
        external_fields=external,
        temperature=1.0,
    )


def kurtosis_from_phi4(state: Phi4State, n_samples: int = 10000) -> dict:
    """§Abstract — Estimate kurtosis from φ⁴ field configuration.
    
    "The φ⁴ quantum field theory is expressive enough to reproduce
    the higher-order statistics such as the market kurtosis, which
    can serve as an indicator of possible market shocks."
    
    Sample field configurations from the Boltzmann distribution
    and compute kurtosis.
    """
    N = len(state.fields)
    T = state.temperature
    
    # Metropolis sampling
    samples = np.zeros((n_samples, N))
    current = state.fields.copy()
    current_E = phi4_hamiltonian(state)
    
    for i in range(n_samples):
        # Propose: perturb one random field
        proposal = current.copy()
        j = np.random.randint(N)
        proposal[j] += np.random.randn() * 0.1
        proposal[j] = np.clip(proposal[j], -1.5, 1.5)
        
        # Compute energy change
        test_state = Phi4State(proposal, state.couplings, state.external_fields, T)
        new_E = phi4_hamiltonian(test_state)
        
        # Metropolis acceptance
        dE = new_E - current_E
        if dE < 0 or np.random.rand() < np.exp(-dE / T):
            current = proposal
            current_E = new_E
        
        samples[i] = current
    
    # Compute kurtosis of each field
    kurtoses = []
    for j in range(N):
        x = samples[:, j]
        if np.std(x) > 1e-10:
            k = float(np.mean((x - np.mean(x))**4) / np.std(x)**4) - 3.0  # excess kurtosis
            kurtoses.append(k)
    
    return {
        "mean_excess_kurtosis": float(np.mean(kurtoses)) if kurtoses else 0.0,
        "max_kurtosis": float(np.max(kurtoses)) if kurtoses else 0.0,
        "is_leptokurtic": (np.mean(kurtoses) > 0.5) if kurtoses else False,
        "n_assets": N,
        "implication": (
            "§Abstract: Leptokurtic (heavy tails). "
            "Kurtosis can serve as market shock indicator."
            if kurtoses and np.mean(kurtoses) > 0.5 else
            "Mesokurtic or platykurtic. No excess tail risk from φ⁴ model."
        ),
    }


def cross_asset_coupling_analysis(state: Phi4State) -> dict:
    """Analyze cross-asset coupling structure from φ⁴ model.
    
    §Abstract — "inhomogeneous couplings" model cross-asset correlations.
    For ict-engine: this replaces/augments the simple correlation matrix
    used in cross_market_smt.
    """
    w = state.couplings
    N = w.shape[0]
    
    # Coupling strength per asset (sum of absolute couplings)
    coupling_strength = np.sum(np.abs(w), axis=1)
    
    # Most coupled pairs
    triu = np.triu_indices(N, k=1)
    pair_couplings = [(triu[0][i], triu[1][i], w[triu[0][i], triu[1][i]])
                      for i in range(len(triu[0]))]
    pair_couplings.sort(key=lambda x: abs(x[2]), reverse=True)
    
    return {
        "mean_coupling_strength": float(np.mean(coupling_strength)),
        "max_coupling": float(np.max(np.abs(w[triu]))),
        "top_pairs": [(int(i), int(j), float(c)) for i, j, c in pair_couplings[:5]],
        "coupling_diversity": float(np.std(coupling_strength)),
    }


# ── Tests ──────────────────────────────────────────────────────────────

def _test_hamiltonian_known():
    """Test: simple 2-field system."""
    state = Phi4State(
        fields=np.array([1.0, -1.0]),
        couplings=np.array([[0, 0.5], [0.5, 0]]),
        external_fields=np.array([0.0, 0.0]),
        temperature=1.0,
    )
    H = phi4_hamiltonian(state)
    # H = -0.5*w*(1)(-1)*2 + λ*((1-1)²+((-1)²-1)²) = 0.5 + 0 = 0.5
    assert abs(H - 0.5) < 0.1, f"H={H}"
    print(f"  ✓ Hamiltonian: H={H:.3f}")


def _test_phi4_estimation():
    """Test: fit φ⁴ to synthetic correlated returns."""
    np.random.seed(42)
    N = 3
    T = 200
    # Correlated returns
    cov = np.array([[1, 0.5, 0.3], [0.5, 1, 0.4], [0.3, 0.4, 1]])
    returns = np.random.multivariate_normal(np.zeros(N), cov, T)
    
    state = estimate_phi4_from_returns(returns, n_steps=500)
    assert state.fields.shape == (N,)
    assert state.couplings.shape == (N, N)
    print(f"  ✓ φ⁴ estimation: fields={state.fields.round(3)}, couplings diag=0")


def _test_kurtosis():
    """Test: kurtosis estimation."""
    np.random.seed(42)
    state = Phi4State(
        fields=np.random.randn(5) * 0.5,
        couplings=np.random.randn(5, 5) * 0.1,
        external_fields=np.zeros(5),
        temperature=0.5,
    )
    result = kurtosis_from_phi4(state, n_samples=2000)
    assert "mean_excess_kurtosis" in result
    print(f"  ✓ Kurtosis: mean_excess={result['mean_excess_kurtosis']:.3f}")


def _test_coupling_analysis():
    """Test: cross-asset coupling analysis."""
    state = Phi4State(
        fields=np.zeros(4),
        couplings=np.array([[0, 0.8, 0.1, 0], [0.8, 0, 0.3, 0.2], [0.1, 0.3, 0, 0.5], [0, 0.2, 0.5, 0]]),
        external_fields=np.zeros(4),
        temperature=1.0,
    )
    result = cross_asset_coupling_analysis(state)
    assert result["top_pairs"][0][2] == 0.8  # strongest coupling
    print(f"  ✓ Coupling analysis: top pair={result['top_pairs'][0]}, diversity={result['coupling_diversity']:.3f}")


def run_tests():
    print("Running φ⁴ field theory tests...")
    _test_hamiltonian_known()
    _test_phi4_estimation()
    _test_kurtosis()
    _test_coupling_analysis()
    print("All φ⁴ tests passed.")


if __name__ == "__main__":
    run_tests()
