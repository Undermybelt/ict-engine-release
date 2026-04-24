"""
RAmmStein — Regime-Aware Execution via OU + DDQN

Paper: https://arxiv.org/abs/2602.19419v2
Adapted for ict-engine integration.

Modules:
  ou_estimator:  OU parameter estimation (θ, μ, σ) via MLE
  state_builder: 8-dim state vector construction
  ddqn_agent:    Double DQN for HJB-QVI approximation
  environment:   AMM liquidity provision simulator
  train:         Training loop + decision boundary evaluation
"""

from .ou_estimator import OUParams, estimate_ou_mle, estimate_ou_rolling
from .state_builder import ExecutionState, build_state, state_to_execution_features
from .ddqn_agent import DDQNAgent, DDQNConfig, QNetwork
from .environment import AMMEnvironment, EnvConfig, simulate_ou_process
from .train import train, evaluate_decision_boundary, load_config

__all__ = [
    "OUParams", "estimate_ou_mle", "estimate_ou_rolling",
    "ExecutionState", "build_state", "state_to_execution_features",
    "DDQNAgent", "DDQNConfig", "QNetwork",
    "AMMEnvironment", "EnvConfig", "simulate_ou_process",
    "train", "evaluate_decision_boundary", "load_config",
]
