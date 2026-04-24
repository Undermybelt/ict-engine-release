# Reproduction Notes — 模糊性审计

论文：Phase Transitions in Financial Markets Using the Ising Model (arXiv 2504.19050)

## SPECIFIED（论文明确给出）

| 细节 | 来源 | 值 |
|------|------|-----|
| 晶格尺寸 | §Monte Carlo simulation | 32×32 |
| 迭代次数 | §Monte Carlo simulation | 1,000,000 |
| 预热期 | §Monte Carlo simulation | t<100,000 |
| 收益间隔 Δt | §Monte Carlo simulation | 100 |
| α (minority effect) | §Bornholdt model | 10 |
| β (inverse temperature) | §Bornholdt model | 1.7 |
| 周期边界条件 | §Monte Carlo simulation | 是 |
| 更新规则 | §Bornholdt model Eq.1 | Boltzmann 概率 |
| 局部场定义 | §Bornholdt model Eq.2 | h_i = Σ J_ij S_j - α S_i \|M\| |
| 收益定义 | §Monte Carlo simulation | r = ln(P_t) - ln(P_{t-Δt}) |
| 自相关检验 lag | §Autocorrelation | τ={0,...,150} |
| η 经验范围 | §Autocorrelation | [0.2, 0.4] |
| Ising 模型 η | §Autocorrelation | ≈0.3 |
| S&P 500 数据范围 | §Monte Carlo simulation | 1982.02 - 2022.02 |

## UNSPECIFIED（论文未明确，需自行决定）

| 细节 | 我们的选择 | 替代方案 |
|------|-----------|---------|
| J_ij 耦合值 | J=1（最近邻） | 随机耦合、距离衰减 |
| M→P 映射 | P(t)=\|M(t)\|+ε | cumsum(M), M² 等 |
| 晶格拓扑 | 2D 方格 | Torus (§提到但用 2D) |
| 初始 spin 分布 | 均匀随机 | 全 +1、棋盘格 |
| 随机数生成器 | numpy MT19937 | PCG, xoshiro |
| 价格数据来源 | 模拟（无数据时） | yfinance, csv |
| 收益标准化方法 | z-score | 均值差、百分比 |
| 正态性检验样本 | 5000 降采样 | 全样本（可能过度拒绝） |

## PARTIALLY_SPECIFIED（论文提及但模糊）

| 细节 | 论文说的 | 问题 |
|------|---------|------|
| 3D Torus 表示 | §提到 Ruffini & Deco (2021) 的 Torus | 实际实现用 2D 方格 |
| "good simulation" 参数 | §说"empirically estimated" | 未给搜索过程 |
| 统计工具 | §说"StatTools library" | 未给版本/配置 |

## 已知偏差

1. **数据缺失**：论文用真实 S&P 500 数据，我们无此数据时用模拟替代
2. **规模简化**：demo 用 200k 迭代，论文用 1M
3. **耦合模型**：论文未明确 J_ij 值，我们用标准最近邻
