"""φ⁴ Field Theory — Continuous Alternative to Ising."""
from .phi4_model import (
    Phi4State, phi4_hamiltonian, estimate_phi4_from_returns,
    kurtosis_from_phi4, cross_asset_coupling_analysis,
)
__all__ = [
    "Phi4State", "phi4_hamiltonian", "estimate_phi4_from_returns",
    "kurtosis_from_phi4", "cross_asset_coupling_analysis",
]
