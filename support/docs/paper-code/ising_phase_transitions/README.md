# Ising Phase Transitions in Financial Markets

Paper: [arXiv 2504.19050](https://arxiv.org/abs/2504.19050)
Title: "Phase Transitions in Financial Markets Using the Ising Model: A Statistical Mechanics Perspective"
Author: Bruno Giorgio (2025)

## Core Contribution

用 Bornholdt Ising 模型模拟金融市场 agent 交互，复现 S&P 500 的统计特征（stylized facts）：
- 波动率聚集 (volatility clustering)
- 负偏度 (negative skewness)
- 厚尾 (heavy tails)
- 收益无自相关 / 绝对收益有自相关

## 模型概要

每个 agent = 一个 spin (±1)，买(+1) 卖(-1)。
局部场 h_i(t) = Σ J_ij S_j - α S_i |M(t)|，其中：
- J_ij：邻居耦合（herding）
- α S_i |M|：全局磁化惩罚（minority game）
- β：逆温度（控制波动率）

更新规则：P(S_i=+1) = 1/(1 + exp(-2β h_i))

## 参数（§Bornholdt model）

| 参数 | 值 | 含义 |
|------|-----|------|
| α | 10 | minority effect 强度 |
| β | 1.7 | 逆温度 |
| L | 32 | 晶格边长 (32×32 = 1024 agents) |
| iterations | 1,000,000 | Monte Carlo 步数 |
| Δt | 100 | 收益计算间隔 |
| warm-up | 100,000 | 预热期 |

## 快速开始

```bash
cd ising_phase_transitions
pip install numpy scipy matplotlib
python src/model.py          # 运行模拟
python src/evaluate.py       # 计算 stylized facts
python notebooks/walkthrough.py  # 可视化
```

## 文件结构

```
src/
  model.py       — §Bornholdt 模型核心：IsingLattice + BornholdtUpdate
  simulation.py  — §Monte Carlo 模拟引擎
  evaluate.py    — §Analysis of Results：stylized facts 检验
  data.py        — S&P 500 数据加载（如有）
  utils.py       — 晶格工具函数
configs/
  base.yaml      — 所有超参数（每个标注来源章节）
REPRODUCTION_NOTES.md — 模糊性审计
```
