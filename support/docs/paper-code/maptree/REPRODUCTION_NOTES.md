# Reproduction Notes — 模糊性审计

论文：MAPTree: Beating "Optimal" Decision Trees with Bayesian Decision Trees (arXiv 2309.15312)

## 已提取公式（via browser MathML OCR）

| 公式 | LaTeX | 来源 |
|------|-------|------|
| BCART 后验 | P(T\|Y,X) ∝ P(Y\|X,T) · P(T\|X) | §3.2 |
| 边际似然 | P(Y\|X,T) = ∏_{l∈T_leaves} B(c^1_l+ρ^1, c^0_l+ρ^0) / B(ρ^1, ρ^0) | §3.2 |
| 叶节点似然 | ℓ_leaf(c^1,c^0) := B(c^1+ρ^1, c^0+ρ^0) / B(ρ^1, ρ^0) | §3.2 |
| 分裂概率 | p_split(d) = α(1+d)^{-β} | §3.2 |
| 叶节点先验 | p_leaf(d,I) = 1 (V=∅), = 1-p_split(d) (V≠∅) | §3.2 |
| 内节点先验 | p_inner(d,I) = 0 (V=∅), = p_split(d)/\|V\| (V≠∅) | §3.2 |
| OR 启发式 | h(o) = -max{log ℓ_leaf, log p_split + log ℓ_leaf(c^1,0) + log ℓ_leaf(0,c^0)} | §5 |
| AND 启发式 | h(a) = h(o_{f=0}) + h(o_{f=1}) | §5 |

## 已提取算法伪码（Algorithm 1-5）

| 算法 | 功能 | 来源 |
|------|------|------|
| Algorithm 1 MAPTree | 主搜索循环：LB/UB 初始化 → 展开 → 更新 bounds | §5 |
| Algorithm 2 getSolution | 重建 MAP 树：比较叶 vs 分裂的 UB | §5 |
| Algorithm 3 findNodeToExpand | 选 UB-LB 最大的未展开节点 | §5 |
| Algorithm 4 updateLowerBounds | 底部向上传播 LB | §5 |
| Algorithm 5 updateUpperBounds | 底部向上传播 UB | §5 |

## SPECIFIED（论文明确给出）

| 细节 | 来源 | 值 |
|------|------|-----|
| BCART 后验公式 | §3.2 | P(T\|Y,X) ∝ P(Y\|X,T)·P(T\|X) |
| Beta-Binomial 共轭 | §3.2 | θ_l ~ Beta(ρ^1, ρ^0) |
| 边际似然公式 | §3.2 | B(c^1+ρ^1, c^0+ρ^0)/B(ρ^1, ρ^0) |
| 树先验结构 | §3.2 | p_split(d) = α(1+d)^{-β} |
| AND/OR 图等价 | §Theorem 1 | BCART MAP = AND/OR search |
| MAPTree 算法 | §Algorithm 1-5 | 完整伪码 |
| OR 启发式 | §5 | h(o) 公式 |
| AND 启发式 | §5 | h(a) = h(o_0)+h(o_1) |
| 实现语言 | Contributions | C++ + Python |
| 官方代码 | Abstract | github.com/ThrunGroup/maptree |
| 评估数据集 | Abstract | 16 real-world datasets |

## UNSPECIFIED（论文未明确）

| 细节 | 我们的选择 | 替代方案 |
|------|-----------|---------|
| ρ^1, ρ^0 值 | 1.0, 1.0（均匀先验） | 0.5 (Jeffreys), 信息先验 |
| α, β 值 | 0.95, 1.0 | 其他衰减参数 |
| 最大深度 | 10 | 无限制, 数据相关 |
| 最小叶节点 | 5 | 1, 10 |
| cost 函数定义 | = -log posterior | 其他成本度量 |
| 最大迭代数 | 5000 | 直到收敛 |

## PARTIALLY_SPECIFIED（提及但模糊）

| 细节 | 论文说的 | 问题 |
|------|---------|------|
| "heavily optimized" C++ | Contributions | bitset hashing, caching 详见 Appendix C |
| optimality certificate | Abstract | UB=LB 时证书成立 |
| synthetic dataset | §6.2 | 未给完整生成参数 |
| Theorem 2-6 | §5.1 | 公式已提取但证明在 Appendix A |
