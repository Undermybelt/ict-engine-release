# MAPTree: Bayesian Decision Trees

Paper: [arXiv 2309.15312](https://arxiv.org/abs/2309.15312)
Title: "MAPTree: Beating 'Optimal' Decision Trees with Bayesian Decision Trees"
Authors: Colin Sullivan, Mo Tiwari, Sebastian Thrun (2023)
Code: https://github.com/ThrunGroup/maptree

## Core Contribution

用 AND/OR 搜索找 BCART (Bayesian CART) 后验的 MAP (Maximum A Posteriori) 树。
比 sampling 方法更快，且提供最优性证书。

## 关键思想

1. **BCART 后验**：P(T|D) ∝ P(D|T)·P(T)
   - P(D|T) = 每个叶节点的边际似然（Beta-Binomial 共轭）
   - P(T) = 树结构先验（深度惩罚）

2. **AND/OR 图等价**：定理证明 BCART MAP 推断 = AND/OR 图搜索
   - AND 节点 = 分裂决策（选哪个特征/阈值）
   - OR 节点 = 不分裂（成为叶节点）

3. **MAPTree 算法**：分支定界搜索，剪枝掉后验低于当前最优的子树

## 实现说明

HTML 解析方程严重失真。实现基于：
- 论文 Abstract + Introduction + 核心定理描述
- GitHub 仓库 (ThrunGroup/maptree) 的 C++ 实现参考
- BCART 标准公式 (Chipman et al., 1998)

## 快速开始

```bash
cd maptree
pip install numpy scipy
python src/model.py           # MAPTree 分类器
python src/evaluate.py        # 对比 sklearn CART
```

## 文件结构

```
src/
  model.py       — BCART 后验 + MAPTree 搜索算法
  evaluate.py    — 对比实验
  utils.py       — 树结构工具
configs/
  base.yaml      — 超参数
REPRODUCTION_NOTES.md — 模糊性审计
```
