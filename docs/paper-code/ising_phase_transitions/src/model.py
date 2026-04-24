# §Bornholdt model — Ising 模型核心
# 论文：Phase Transitions in Financial Markets Using the Ising Model (arXiv 2504.19050)
#
# §Bornholdt model, Eq. 1:
#   S_i(t+1) = +1  with  p = 1/(1 + exp(-2β h_i(t)))
#   S_i(t+1) = -1  with  1-p
#
# §Bornholdt model, Eq. 2 (local field):
#   h_i(t) = Σ_j J_ij S_j - α S_i |(1/N) Σ_j S_j|
#
# 第一项 = herding（邻居耦合）
# 第二项 = minority game（全局磁化惩罚）

import numpy as np
from dataclasses import dataclass
from typing import Optional


@dataclass
class IsingConfig:
    """§Bornholdt model 参数"""
    # [SPECIFIED] §Bornholdt model: "α=10" (minority effect)
    alpha: float = 10.0
    # [SPECIFIED] §Bornholdt model: "β=1.7" (inverse temperature)
    beta: float = 1.7
    # [SPECIFIED] §Monte Carlo simulation: "32×32 agents"
    lattice_size: int = 32
    # [SPECIFIED] §Monte Carlo simulation: "1,000,000 iterations"
    n_iterations: int = 1_000_000
    # [SPECIFIED] §Monte Carlo simulation: "warm-up period of approximately t<100,000"
    warmup: int = 100_000
    # [SPECIFIED] §Monte Carlo simulation: "Δt=100"
    delta_t: int = 100
    # [SPECIFIED] §Monte Carlo simulation: "two-dimensional lattice... square with periodic boundary conditions"
    periodic_bc: bool = True


class IsingLattice:
    """§Bornholdt model: 2D 晶格 + 周期边界
    
    §Bornholdt model: "spin states (+1,-1) can be interpreted as representing
    a buyer (+1) and a seller (-1) in the financial market"
    """

    def __init__(self, config: IsingConfig):
        self.config = config
        L = config.lattice_size
        self.N = L * L  # 总 agent 数
        # §Monte Carlo simulation: "Starting from a random spin configuration"
        self.spins = np.random.choice([-1, 1], size=(L, L)).astype(np.int8)
        # §Bornholdt model: J_ij 邻居耦合矩阵
        # [UNSPECIFIED] 论文未明确 J_ij 的具体值，只说"neighbour interaction"
        # Using: J_ij=1 for nearest neighbors, 0 otherwise (标准 Ising 做法)
        # Alternatives: 随机耦合、距离衰减耦合
        self.J_neighbors = self._build_neighbor_coupling()

    def _build_neighbor_coupling(self) -> np.ndarray:
        """构建最近邻耦合权重（周期边界）
        
        §Bornholdt model: "interaction with the j-th neighbour"
        [UNSPECIFIED] 论文只提"neighbour interaction"未给 J_ij 值
        Using: 标准 Ising 最近邻 J=1
        """
        L = self.config.lattice_size
        # 对于 2D 方格，每个 agent 有 4 个最近邻
        # 返回每个位置的 4 个邻居坐标 (up, down, left, right)
        neighbors = np.zeros((L, L, 4, 2), dtype=np.int32)
        for i in range(L):
            for j in range(L):
                neighbors[i, j] = [
                    ((i - 1) % L, j),           # up
                    ((i + 1) % L, j),           # down
                    (i, (j - 1) % L),           # left
                    (i, (j + 1) % L),           # right
                ]
        return neighbors

    def magnetization(self) -> float:
        """§Bornholdt model: M(t) = (1/N) Σ_j S_j (全局磁化)"""
        return self.spins.sum() / self.N

    def local_field(self) -> np.ndarray:
        """§Bornholdt model, Eq. 2:
        h_i(t) = Σ_j J_ij S_j - α S_i |M(t)|
        
        第一项：herding — 邻居 spin 之和
        第二项：minority game — 与全局磁化方向相反时更有利
        """
        L = self.config.lattice_size
        alpha = self.config.alpha
        M = abs(self.magnetization())

        # 邻居贡献（herding term）
        h = np.zeros((L, L), dtype=np.float64)
        for di, dj in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
            h += np.roll(np.roll(self.spins, di, axis=0), dj, axis=1)

        # Minority game term: -α S_i |M|
        h -= alpha * self.spins * M

        return h

    def flip_probability(self, h: np.ndarray) -> np.ndarray:
        """§Bornholdt model, Eq. 1:
        P(S_i = +1) = 1/(1 + exp(-2β h_i))
        """
        beta = self.config.beta
        return 1.0 / (1.0 + np.exp(-2.0 * beta * h))

    def metropolis_step(self) -> int:
        """单步 Metropolis-Hastings 更新
        
        §Monte Carlo simulation: "we apply the Bornholdt update rule to compute
        the local field for each agent. Subsequently, we calculate the probability
        of the agent's state S(t) based on this local field"
        
        返回翻转的 spin 数量
        """
        h = self.local_field()
        p = self.flip_probability(h)
        # 生成随机数决定是否翻转
        r = np.random.random((self.config.lattice_size, self.config.lattice_size))
        # S_i(t+1) = +1 if r < p, else -1
        new_spins = np.where(r < p, 1, -1).astype(np.int8)
        flips = np.sum(new_spins != self.spins)
        self.spins = new_spins
        return flips

    def compute_returns(self, magnetization_series: np.ndarray) -> np.ndarray:
        """§Monte Carlo simulation: "logarithmic return r_Δt(t) = ln(P_t) - ln(P_{t-Δt})"
        
        用磁化强度 M(t) 代理价格 P(t)（论文隐含映射）
        [UNSPECIFIED] 论文未明确 M→P 的映射方式
        Using: P(t) = |M(t)| + ε (避免 log(0))
        Alternatives: cumsum of M, M^2 等
        """
        # 避免 log(0)
        prices = np.abs(magnetization_series) + 1e-10
        delta_t = self.config.delta_t
        if len(prices) <= delta_t:
            return np.array([])
        returns = np.log(prices[delta_t:]) - np.log(prices[:-delta_t])
        return returns


def simulate(config: Optional[IsingConfig] = None) -> dict:
    """运行完整 Bornholdt Ising 模拟
    
    §Monte Carlo simulation: 完整流程
    返回：magnetization_series, returns, config
    """
    if config is None:
        config = IsingConfig()

    lattice = IsingLattice(config)
    total_steps = config.n_iterations

    # 记录磁化强度
    mag_series = np.zeros(total_steps)

    for t in range(total_steps):
        lattice.metropolis_step()
        mag_series[t] = lattice.magnetization()

    # 去掉 warmup
    mag_after_warmup = mag_series[config.warmup:]
    returns = lattice.compute_returns(mag_after_warmup)

    return {
        "magnetization_series": mag_after_warmup,
        "returns": returns,
        "config": config,
        "final_spins": lattice.spins.copy(),
    }


if __name__ == "__main__":
    print("Running Bornholdt Ising simulation (§Monte Carlo)...")
    print("  Lattice: 32×32 | Iterations: 1,000,000 | Warmup: 100,000")
    print("  α=10 (minority) | β=1.7 (inverse temp) | Δt=100")

    config = IsingConfig()
    result = simulate(config)

    print(f"\nDone. {len(result['returns'])} returns computed.")
    print(f"Mean magnetization: {result['magnetization_series'].mean():.4f}")
    print(f"Return std: {result['returns'].std():.6f}")
    print("\nRun evaluate.py for stylized facts analysis.")
