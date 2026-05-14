"""
RAmmStein — State Vector Builder

Paper: https://arxiv.org/abs/2602.19419v2
Implements: 8-dimensional state vector for execution decisions

Section references:
  §IV-B — State Representation
  §IV-B — "agent observes an 8-dimensional state vector"
"""

import numpy as np
from dataclasses import dataclass
from .ou_estimator import OUParams


@dataclass
class ExecutionState:
    """§IV-B — 8-dimensional state vector.
    
    Components (from paper):
      δ_p:     normalized price deviation (S/c - 1)
      d_edge:  distance to nearest range edge [-1, 1]
      θ:       Stein Signal (mean-reversion speed) [0, 1]
      δ_μ:     mean deviation ((μ-S)/S)
      σ̃:       normalized sigma (clipped at 0.1)
      ϕ_a:     active fraction (time in range)
      (2 more: [UNSPECIFIED] — paper lists 8 but only 6 are explicitly named)
    """
    delta_p: float       # §IV-B — normalized price deviation
    d_edge: float        # §IV-B — distance to edge [-1, 1]
    theta: float         # §IV-B — Stein Signal [0, 1]
    delta_mu: float      # §IV-B — mean deviation
    sigma_norm: float    # §IV-B — σ̃ clipped at 0.1
    active_frac: float   # §IV-B — active fraction
    # [UNSPECIFIED] — paper says 8-dim but only 6 named; padding with 2 derived
    momentum: float      # short-term price momentum
    vol_ratio: float     # recent vol / long-term vol

    def to_array(self) -> np.ndarray:
        """Convert to numpy array for NN input."""
        return np.array([
            self.delta_p,
            self.d_edge,
            self.theta,
            self.delta_mu,
            self.sigma_norm,
            self.active_frac,
            self.momentum,
            self.vol_ratio,
        ], dtype=np.float32)


def build_state(
    current_price: float,
    position_center: float,
    range_lower: float,
    range_upper: float,
    ou_params: OUParams,
    recent_prices: np.ndarray,
    active_steps: int,
    total_steps: int,
) -> ExecutionState:
    """§IV-B — Build 8-dim state vector from market state.
    
    Args:
        current_price: current market price S_t
        position_center: LP position center c
        range_lower: lower bound of LP range
        range_upper: upper bound of LP range
        ou_params: estimated OU parameters (θ, μ, σ)
        recent_prices: recent price history for momentum/vol
        active_steps: steps price was inside range
        total_steps: total steps observed
    
    Returns:
        ExecutionState with all 8 components
    """
    # §IV-B — δ_p = S_t / c - 1
    delta_p = (current_price / position_center) - 1.0 if position_center > 0 else 0.0
    delta_p = np.clip(delta_p, -0.1, 0.1)  # [UNSPECIFIED] clipping

    # §IV-B — d_edge: distance to nearest edge, normalized to [-1, 1]
    range_width = range_upper - range_lower
    if range_width > 0:
        dist_to_upper = range_upper - current_price
        dist_to_lower = current_price - range_lower
        nearest_dist = min(dist_to_upper, dist_to_lower)
        d_edge = 2.0 * nearest_dist / range_width - 1.0  # [-1, 1]
    else:
        d_edge = 0.0

    # §IV-B — θ: Stein Signal, already clipped to [0,1] by estimator
    theta = ou_params.theta if ou_params.valid else 0.0

    # §IV-B — δ_μ = (μ - S_t) / S_t
    delta_mu = (ou_params.mu - current_price) / current_price if current_price > 0 and ou_params.valid else 0.0

    # §IV-B — σ̃: normalized sigma, clipped at 0.1
    sigma_norm = min(ou_params.sigma / current_price if current_price > 0 else 0.0, 0.1) if ou_params.valid else 0.0

    # §IV-B — ϕ_a: active fraction
    active_frac = active_steps / total_steps if total_steps > 0 else 1.0

    # [UNSPECIFIED] — momentum: 5-bar return
    if len(recent_prices) >= 5:
        momentum = (recent_prices[-1] / recent_prices[-5]) - 1.0 if recent_prices[-5] > 0 else 0.0
    else:
        momentum = 0.0

    # [UNSPECIFIED] — vol ratio: short/long realized vol
    if len(recent_prices) >= 20:
        short_vol = np.std(np.diff(recent_prices[-10:])) if len(recent_prices) >= 10 else 0.0
        long_vol = np.std(np.diff(recent_prices[-20:]))
        vol_ratio = short_vol / long_vol if long_vol > 1e-10 else 1.0
    else:
        vol_ratio = 1.0

    return ExecutionState(
        delta_p=float(delta_p),
        d_edge=float(d_edge),
        theta=float(theta),
        delta_mu=float(delta_mu),
        sigma_norm=float(sigma_norm),
        active_frac=float(active_frac),
        momentum=float(momentum),
        vol_ratio=float(vol_ratio),
    )


def state_to_execution_features(state: ExecutionState) -> dict:
    """Map RAmmStein state to ict-engine ExecutionFeatures.
    
    This bridges the paper's state vector to ict-engine's domain types.
    """
    return {
        # OU-derived execution signals
        "ou_theta": state.theta,
        "ou_overextension": abs(state.delta_mu),
        "ou_reversion_speed": state.theta,
        # Regime signal
        "regime_laziness_score": state.theta * state.active_frac,
        # Execution feasibility
        "edge_proximity": max(0, state.d_edge),  # positive = inside range
        "momentum_bias": state.momentum,
        "volatility_regime": state.vol_ratio,
    }
