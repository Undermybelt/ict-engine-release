"""
RAmmStein — Environment Simulator

Paper: https://arxiv.org/abs/2602.19419v2
Implements: AMM liquidity provision environment for DDQN training

Section references:
  §IV-C — Environment dynamics
  §IV-C — Reward = fee_accrued - rebalance_cost
  §III-E, Eq.10 — OU price process
"""

import numpy as np
from dataclasses import dataclass
from typing import Optional
from .ou_estimator import OUParams, estimate_ou_mle
from .state_builder import ExecutionState, build_state


@dataclass
class EnvConfig:
    """Environment configuration.
    
    §IV-C — fee and cost structure.
    [UNSPECIFIED] — exact bps values not stated in paper.
    """
    rebalance_cost_bps: float = 30    # [UNSPECIFIED] — typical DEX cost
    fee_rate_bps: float = 3           # [UNSPECIFIED] — typical Uniswap fee
    ou_window: int = 200              # [UNSPECIFIED] — rolling window
    max_steps: int = 500              # [UNSPECIFIED]


class AMMEnvironment:
    """§IV-C — AMM liquidity provision environment.
    
    Simulates:
    - OU price process (§III-E, Eq.10)
    - Fee accrual when price is in range
    - Rebalance cost when action=1
    - Position state tracking
    """
    
    def __init__(
        self,
        prices: np.ndarray,
        config: EnvConfig,
    ):
        """
        Args:
            prices: historical price series for simulation
            config: environment configuration
        """
        self.prices = prices
        self.config = config
        self.reset()
    
    def reset(self) -> np.ndarray:
        """Reset environment to initial state.
        
        §IV-B — Initialize position at first price.
        """
        self.step_idx = self.config.ou_window  # start after warmup
        self.position_center = self.prices[self.step_idx]
        
        # §IV-B — Range width: user-defined λ
        # [UNSPECIFIED] — using ±2% as default
        spread = self.position_center * 0.02
        self.range_lower = self.position_center - spread
        self.range_upper = self.position_center + spread
        
        self.active_steps = 0
        self.total_steps = 0
        self.total_fees = 0.0
        self.rebalance_count = 0
        
        return self._get_state()
    
    def _get_state(self) -> np.ndarray:
        """Build current state vector (§IV-B)."""
        current_price = self.prices[self.step_idx]
        
        # §III-E — Estimate OU params from recent window
        start = max(0, self.step_idx - self.config.ou_window)
        recent = self.prices[start : self.step_idx + 1]
        ou_params = estimate_ou_mle(recent)
        
        state = build_state(
            current_price=current_price,
            position_center=self.position_center,
            range_lower=self.range_lower,
            range_upper=self.range_upper,
            ou_params=ou_params,
            recent_prices=recent,
            active_steps=self.active_steps,
            total_steps=max(1, self.total_steps),
        )
        return state.to_array()
    
    def step(self, action: int) -> tuple[np.ndarray, float, bool, dict]:
        """§IV-C — Execute one step.
        
        Actions:
          0 = wait (continuation region)
          1 = rebalance (exercise region)
        
        §IV-C — Reward:
          r = fee_accrued - rebalance_cost
        
        Args:
            action: 0 (wait) or 1 (rebalance)
        
        Returns:
            (next_state, reward, done, info)
        """
        current_price = self.prices[self.step_idx]
        
        # Check if price is in range
        in_range = self.range_lower <= current_price <= self.range_upper
        
        # §IV-C — Fee accrual (only when in range)
        if in_range:
            fee = current_price * self.config.fee_rate_bps / 10000.0
            self.active_steps += 1
        else:
            fee = 0.0
        self.total_steps += 1
        self.total_fees += fee
        
        # §IV-C — Rebalance cost
        rebalance_cost = 0.0
        if action == 1:
            rebalance_cost = current_price * self.config.rebalance_cost_bps / 10000.0
            # §IV-C — Reset position center to current price
            self.position_center = current_price
            spread = current_price * 0.02  # [UNSPECIFIED]
            self.range_lower = current_price - spread
            self.range_upper = current_price + spread
            self.rebalance_count += 1
        
        # §IV-C — Reward = fee - cost
        reward = fee - rebalance_cost
        
        # Advance
        self.step_idx += 1
        done = self.step_idx >= len(self.prices) - 1
        
        next_state = self._get_state() if not done else np.zeros(8, dtype=np.float32)
        
        info = {
            "fee": fee,
            "rebalance_cost": rebalance_cost,
            "in_range": in_range,
            "total_fees": self.total_fees,
            "rebalance_count": self.rebalance_count,
            "active_frac": self.active_steps / max(1, self.total_steps),
        }
        
        return next_state, reward, done, info
    
    @property
    def active_fraction(self) -> float:
        """§VIII-A — Active percentage of time."""
        return self.active_steps / max(1, self.total_steps)


def simulate_ou_process(
    n_steps: int,
    theta: float,
    mu: float,
    sigma: float,
    s0: float,
    dt: float = 1.0,
    seed: int = 42,
) -> np.ndarray:
    """§III-E, Eq.10 — Simulate OU price process.
    
    dS = θ(μ - S)dt + σdWt
    
    Euler-Maruyama discretization:
        S[t+1] = S[t] + θ(μ - S[t])dt + σ√dt * Z[t]
    
    Args:
        n_steps: number of simulation steps
        theta: mean-reversion speed
        mu: long-term mean
        sigma: diffusion coefficient
        s0: initial price
        dt: time step
        seed: random seed
    
    Returns:
        Simulated price series, shape (n_steps + 1,)
    """
    rng = np.random.RandomState(seed)
    prices = np.zeros(n_steps + 1)
    prices[0] = s0
    
    sqrt_dt = np.sqrt(dt)
    for t in range(n_steps):
        dW = rng.randn() * sqrt_dt
        prices[t + 1] = prices[t] + theta * (mu - prices[t]) * dt + sigma * dW
    
    return prices
