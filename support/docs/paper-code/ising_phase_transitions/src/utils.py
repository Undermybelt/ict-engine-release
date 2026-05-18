# 晶格工具函数
# §Bornholdt model + §Monte Carlo simulation

import numpy as np


def random_lattice(size: int) -> np.ndarray:
    """§Monte Carlo simulation: "Starting from a random spin configuration" """
    return np.random.choice([-1, 1], size=(size, size)).astype(np.int8)


def all_up_lattice(size: int) -> np.ndarray:
    """全 +1 晶格（全买）"""
    return np.ones((size, size), dtype=np.int8)


def all_down_lattice(size: int) -> np.ndarray:
    """全 -1 晶格（全卖）"""
    return -np.ones((size, size), dtype=np.int8)


def checkerboard_lattice(size: int) -> np.ndarray:
    """棋盘格（反铁磁序）— §Bornholdt model Fig. 2"""
    lattice = np.zeros((size, size), dtype=np.int8)
    for i in range(size):
        for j in range(size):
            lattice[i, j] = 1 if (i + j) % 2 == 0 else -1
    return lattice


def compute_local_energy(spins: np.ndarray, J: float = 1.0) -> float:
    """计算晶格总能量（最近邻耦合）
    
    E = -J Σ_{<i,j>} S_i S_j
    [UNSPECIFIED] 论文未定义能量函数
    Using: 标准 Ising 能量定义
    """
    L = spins.shape[0]
    energy = 0.0
    for di, dj in [(0, 1), (1, 0)]:  # 只算右和下，避免重复
        energy -= J * (spins * np.roll(np.roll(spins, -di, axis=0), -dj, axis=1)).sum()
    return float(energy)


def compute_binder_cumulant(magnetization_series: np.ndarray, n_bins: int = 10) -> float:
    """计算 Binder cumulant U = 1 - <M^4>/(3<M^2>^2)
    
    用于检测相变点
    [UNSPECIFIED] 论文未使用 Binder cumulant
    Using: 标准统计物理方法
    """
    M2 = (magnetization_series ** 2).mean()
    M4 = (magnetization_series ** 4).mean()
    if M2 < 1e-10:
        return 0.0
    return 1.0 - M4 / (3.0 * M2 ** 2)


def visualize_lattice(spins: np.ndarray, title: str = "") -> None:
    """可视化晶格状态（买=红，卖=蓝）"""
    try:
        import matplotlib.pyplot as plt
        plt.figure(figsize=(6, 6))
        plt.imshow(spins, cmap='RdBu', vmin=-1, vmax=1, interpolation='nearest')
        plt.colorbar(label='Spin (+1=buy, -1=sell)')
        plt.title(title or f"M={spins.sum()/spins.size:.3f}")
        plt.tight_layout()
        plt.savefig('lattice_snapshot.png', dpi=150)
        plt.close()
        print("Saved: lattice_snapshot.png")
    except ImportError:
        print("matplotlib not available, skipping visualization")
