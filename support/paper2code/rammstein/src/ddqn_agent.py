"""
RAmmStein — DDQN Agent

Paper: https://arxiv.org/abs/2602.19419v2
Implements: Double DQN for HJB-QVI approximation

Section references:
  §V — DDQN as HJB Solver
  §VI-A — Network architecture (2-layer, 128 hidden)
  §VI-A — Hyperparameters (lr=1e-4, γ=0.99, batch=64)
  §V, Eq.22 — Bellman equation connection to HJB
"""

import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F
from collections import deque
from dataclasses import dataclass
from typing import Optional


@dataclass
class DDQNConfig:
    """§VI-A — DDQN hyperparameters.
    
    All values from paper Table II or flagged [UNSPECIFIED].
    """
    state_dim: int = 8              # §IV-B — 8-dim state vector
    n_actions: int = 2              # §IV-C — {wait=0, rebalance=1}
    hidden_dim: int = 128           # §VI-A — "2-layer, 128 hidden"
    n_layers: int = 2               # §VI-A — "2-layer"
    lr: float = 1e-4                # §VI-A
    gamma: float = 0.99             # §VI-A — discount factor
    batch_size: int = 64            # §VI-A
    buffer_size: int = 100000       # §VI-A
    target_update_freq: int = 1000  # §VI-A — "every 1000 steps"
    epsilon_start: float = 1.0      # §VI-A
    epsilon_end: float = 0.01       # §VI-A
    epsilon_decay_steps: int = 50000  # §VI-A — "over 50k steps"
    seed: int = 42


class QNetwork(nn.Module):
    """§VI-A — Q-network: 2-layer MLP with 128 hidden units.
    
    "We use a 2-layer neural network with 128 hidden units
     and ReLU activation." (§VI-A)
    [UNSPECIFIED] — activation function not explicitly named, using ReLU.
    """
    
    def __init__(self, config: DDQNConfig):
        super().__init__()
        layers = []
        in_dim = config.state_dim
        for i in range(config.n_layers):
            out_dim = config.hidden_dim
            layers.append(nn.Linear(in_dim, out_dim))
            # [UNSPECIFIED] — activation not specified, using ReLU
            layers.append(nn.ReLU())
            in_dim = out_dim
        layers.append(nn.Linear(in_dim, config.n_actions))
        self.net = nn.Sequential(*layers)
        
        # [UNSPECIFIED] — weight init not specified, using default
    
    def forward(self, state: torch.Tensor) -> torch.Tensor:
        """Forward pass.
        
        Args:
            state: (batch, state_dim)
        Returns:
            q_values: (batch, n_actions)
        """
        return self.net(state)


class ReplayBuffer:
    """§VI-A — Experience replay buffer.
    
    "We maintain a replay buffer of 100,000 transitions." (§VI-A)
    """
    
    def __init__(self, capacity: int = 100000):
        self.buffer = deque(maxlen=capacity)
    
    def push(self, state, action, reward, next_state, done):
        self.buffer.append((state, action, reward, next_state, done))
    
    def sample(self, batch_size: int):
        indices = np.random.choice(len(self.buffer), batch_size, replace=False)
        batch = [self.buffer[i] for i in indices]
        states, actions, rewards, next_states, dones = zip(*batch)
        return (
            np.array(states, dtype=np.float32),
            np.array(actions, dtype=np.int64),
            np.array(rewards, dtype=np.float32),
            np.array(next_states, dtype=np.float32),
            np.array(dones, dtype=np.float32),
        )
    
    def __len__(self):
        return len(self.buffer)


class DDQNAgent:
    """§V, §VI-A — Double DQN agent.
    
    §V, Eq.22: "V(s) = f(s,c)Δt + e^{-ρΔt} E[V(S_{t+Δt}, c)]"
    This is the Bellman equation that connects HJB to Q-learning.
    
    The DDQN decouples action selection (online net) from
    action evaluation (target net) to reduce overestimation.
    """
    
    def __init__(self, config: DDQNConfig):
        self.config = config
        
        # §VI-A — Online and target networks
        self.online_net = QNetwork(config)
        self.target_net = QNetwork(config)
        self.target_net.load_state_dict(self.online_net.state_dict())
        
        # §VI-A — Adam optimizer
        self.optimizer = torch.optim.Adam(self.online_net.parameters(), lr=config.lr)
        
        # §VI-A — Replay buffer
        self.buffer = ReplayBuffer(config.buffer_size)
        
        # ε-greedy schedule
        self.epsilon = config.epsilon_start
        self.epsilon_step = (config.epsilon_start - config.epsilon_end) / config.epsilon_decay_steps
        
        self.steps = 0
    
    def select_action(self, state: np.ndarray) -> int:
        """§VI-A — ε-greedy action selection.
        
        ε decays linearly from 1.0 to 0.01 over 50k steps.
        """
        if np.random.random() < self.epsilon:
            return np.random.randint(self.config.n_actions)
        
        with torch.no_grad():
            state_t = torch.FloatTensor(state).unsqueeze(0)
            q_values = self.online_net(state_t)
            return q_values.argmax(dim=1).item()
    
    def update_epsilon(self):
        """Linear ε decay."""
        if self.epsilon > self.config.epsilon_end:
            self.epsilon -= self.epsilon_step
            self.epsilon = max(self.epsilon, self.config.epsilon_end)
    
    def store_transition(self, state, action, reward, next_state, done):
        """Store transition in replay buffer."""
        self.buffer.push(state, action, reward, next_state, done)
    
    def train_step(self) -> Optional[float]:
        """§V — One DDQN training step.
        
        §V, Eq.22: Q(s,a) = r + γ * Q_target(s', argmax_a' Q_online(s',a'))
        
        The DDQN key: action selection uses online net,
        action evaluation uses target net.
        """
        if len(self.buffer) < self.config.batch_size:
            return None
        
        states, actions, rewards, next_states, dones = self.buffer.sample(self.config.batch_size)
        
        states_t = torch.FloatTensor(states)
        actions_t = torch.LongTensor(actions)
        rewards_t = torch.FloatTensor(rewards)
        next_states_t = torch.FloatTensor(next_states)
        dones_t = torch.FloatTensor(dones)
        
        # Current Q values: Q(s, a)
        current_q = self.online_net(states_t).gather(1, actions_t.unsqueeze(1)).squeeze(1)
        
        # §V — DDQN: action selection from online, evaluation from target
        with torch.no_grad():
            # argmax_a' Q_online(s', a')
            next_actions = self.online_net(next_states_t).argmax(dim=1)
            # Q_target(s', argmax_a' Q_online(s', a'))
            next_q = self.target_net(next_states_t).gather(1, next_actions.unsqueeze(1)).squeeze(1)
            # r + γ * Q_target * (1 - done)
            target_q = rewards_t + self.config.gamma * next_q * (1.0 - dones_t)
        
        # §VI-A — Smooth L1 loss (Huber)
        loss = F.smooth_l1_loss(current_q, target_q)
        
        self.optimizer.zero_grad()
        loss.backward()
        # [UNSPECIFIED] — gradient clipping not specified
        torch.nn.utils.clip_grad_norm_(self.online_net.parameters(), 1.0)
        self.optimizer.step()
        
        self.steps += 1
        self.update_epsilon()
        
        # §VI-A — Target network update every 1000 steps
        if self.steps % self.config.target_update_freq == 0:
            self.target_net.load_state_dict(self.online_net.state_dict())
        
        return loss.item()
    
    def get_q_diff(self, state: np.ndarray) -> float:
        """§VII-B — Q(action=1) - Q(action=0) for decision boundary.
        
        This is the key output for visualization: positive means
        "rebalance is preferred", negative means "wait is preferred".
        """
        with torch.no_grad():
            state_t = torch.FloatTensor(state).unsqueeze(0)
            q_values = self.online_net(state_t)
            return (q_values[0, 1] - q_values[0, 0]).item()
