# Research Gaps & Paper Synthesis

> 评估日期：2026-04-20
> 范围：ict-engine docs/ 下全部指导性文档 + sprint 计划

---

## 一、文档已做到位的

1. 分层架构清晰（domain / factor_lab / application / state / cli）
2. execution-first 优先级排序有论文支撑（SSRN "Who Profits from Prediction?"）
3. integrity rules 严格（isolated state、provenance、comparability）
4. 物理学层定位正确：先服务 execution feasibility，再服务 regime explanation
5. 代码-文档有对应关系，artifact ledger / reflection_bundle 落地

---

## 二、核心不足（按影响排序）

### A. 缺 risk management 层 [严重]

文档完全没有覆盖：
- 最大回撤约束
- Kelly / 半 Kelly 仓位 sizing
- 连续亏损 streak 管理
- slippage / cost 建模（OU 弹性 ≠ 交易成本）
- VaR / CVaR 作为 gate 输入

**根因**：execution-first 框架正确，但 execution 的 "cost side" 只有 OU 弹性，没有 market impact 结构。

### B. regime detection 的统计假设未被质疑 [严重]

- HMM 假设马尔可夫性，但市场 regime 切换常有长记忆效应
- MECE recovery > 95% 阈值缺乏理论支撑
- 没有讨论 regime persistence / regime duration distribution
- HMM 固有的切换滞后性没有对策

### C. 缺 walk-forward / out-of-sample 框架 [严重]

- factor mutation 和 autoresearch 全部在同一批数据上跑
- 没有 time-series-aware 的 walk-forward split
- 没有 out-of-sample 验证协议
- 容易过拟合到 NQ 清洗数据

### D. 市场微观结构基础太薄 [中等]

- execution feasibility 的理论基础只有 OU process
- 没有引用 Glosten-Milgrom、Kyle lambda、Amihud illiquidity 等经典模型
- OU process 的使用缺乏与 market impact models 的关联

### E. 没有对抗性 / 压力测试思想 [中等]

- 全部文档假设"模型是对的，参数要调"
- 没有"如果模型错了"的 fallback
- 没有 adversarial regime 假设
- 没有 stress testing protocol

### F. 跨市场信号的理论基础弱 [中等]

- `cross_market_smt` 只是作为一个 factor，缺乏因果性论证
- 没有讨论 lead-lag 关系的统计检验
- 没有 cointegration / Granger causality 验证

### G. Execution Tree 太理想化 [低-中]

- Bayesian Decision Tree 在 non-stationary 金融数据上容易过拟合
- SHAP 在 tree ensemble 上的 theoretical guarantees 在 non-stationary 数据上不成立
- 没有讨论模型不确定性传播

---

## 三、论文补强矩阵

### 3.1 RAmmStein — OU + Regime + Execution [最相关]

**arXiv**: 2602.19419v2
**PDF**: https://arxiv.org/pdf/2602.19419v2
**标题**: RAmmStein: Regime Adaptation in Mean-reverting Markets with Stein Thresholds

**核心贡献**：
1. 把 concentrated liquidity 管理形式化为 impulse control problem
2. 推导 HJB-QVI（Hamilton-Jacobi-Bellman quasi-variational inequality）
3. 用 OU 的 θ（均值回归速度）作为 regime signal
4. DDQN 学习 state space 中 action/inaction 的分界

**对 ict-engine 的直接启示**：
- execution tree 可以借鉴其 "regime-aware laziness" 概念——不是每个 regime 都该做
- OU θ 作为 regime signal 的用法可直接迁移到 `ExecutionPhysicsOverlay`
- HJB-QVI 的 free boundary 概念可以替代硬编码的 `0.65/0.45` 门限
- "67% 减少再平衡频率，净 ROI 提升 26%"——说明 execution quality 比 active% 重要

**可迁移的关键结构**：
- State vector: `[δ_p, d_edge, θ, δ_μ, σ̃, ϕ_a]` → 对应你的 `ExecutionFeatures`
- Decision boundary: Q(action=1) - Q(action=0) vs θ → 可替代 hard gate
- Regime-dependent action threshold: 高 θ → 更宽 inaction zone

### 3.2 Red Queen's Trap — 进化计算在 HFT 中的系统性失败 [关键警告]

**arXiv**: 2512.15732v1
**PDF**: https://arxiv.org/pdf/2512.15732v1
**标题**: The Red Queen's Trap: Limits of Deep Evolution in High-Frequency Trading

**核心发现**：
- 500 个 DRL+进化算法 agent，训练 APY>300%，实盘资本衰减>70%
- 三个致命 failure mode：
  1. **Aleatoric Uncertainty 过拟合**：在低熵时间序列上，模型拟合的是随机噪声而非信号
  2. **Survivor Bias**：进化选择在高方差下产生虚假优胜者
  3. **Microstructure friction 不可克服**：没有 order-flow 数据，模型复杂度越高越脆弱

**对 ict-engine 的直接警告**：
- factor mutation 如果只追求 score_delta 而不考虑 execution friction，会重蹈覆辙
- autoresearch 的 keep/discard 机制天然有 survivor bias 风险
- 增加模型复杂度（更多 factor、更复杂的 voting）在没有信息优势时加剧脆弱性

**建议行动**：
- 给 factor mutation 加 "learn from failures" 模块（参考 FactorEngine）
- 在 autoresearch 中加入 "capital decay" 回测，不仅看 score_delta
- execution friction 必须作为 mutation 的硬约束，不是软惩罚

### 3.3 Ising Model + Stylized Facts — cluster persistence 的微观机制

**arXiv**: 2512.17925v1
**PDF**: https://arxiv.org/pdf/2512.17925v1
**标题**: Stylized Facts and Their Microscopic Origins: Clustering, Persistence, and Stability in a 2D Ising Framework

**核心贡献**：
- 2D Ising 网络中 spin cluster 的形态和持久性可以解释金融市场的 stylized facts
- cluster persistence → 波动率聚集
- cluster reorganization → 间歇性和重尾

**对 ict-engine 的启示**：
- Ising overlay 的 `herding_bias` 可以用 cluster size distribution 校验
- `phase_transition_risk` 应该和 cluster persistence 挂钩
- 高 cluster persistence = regime 可能更持久 → 影响 MECE label 的时间窗口

### 3.4 φ⁴ Quantum Field Theory — 超越 Ising 的连续版本

**arXiv**: 2512.17225v1
**PDF**: https://arxiv.org/pdf/2512.17225v1
**标题**: Modelling financial time series with φ⁴ quantum field theory

**核心贡献**：
- φ⁴ 场论替代 Ising，连续场避免离散化失真
- 能捕捉 Ising 二值化丢掉的高阶统计量（kurtosis）
- 2008 危机期间 kurtosis 重现精度远超 Ising

**对 ict-engine 的长期启示**：
- 如果 Ising overlay 的二值 spin 假设在实盘中表现差，可考虑升级为连续场论版本
- 但这是 Sprint 4+ 的工作，当前先落地 Ising

### 3.5 FactorEngine — 因子挖掘 + learn from failures

**arXiv**: 2603.16365v2
**PDF**: https://arxiv.org/pdf/2603.16365v2
**标题**: FactorEngine: A Program-level Knowledge-Infused Factor Mining Framework

**核心贡献**：
- 把 factor 当作 Turing-complete code（不是固定表达式）
- LLM 指导方向搜索 + Bayesian 超参优化
- **经验知识库**：包括从失败中学习的轨迹

**对 ict-engine 的启示**：
- factor mutation 可以借鉴 "learn from failures" 模块
- 把 rejected mutation 的原因存入经验知识库
- 下一次 mutation 时查知识库避免重复失败模式

### 3.6 Kyle's Model with Stochastic Liquidity

**arXiv**: 2204.11069v1
**PDF**: https://arxiv.org/pdf/2204.11069v1

**核心贡献**：
- Kyle lambda 在 stochastic volatility 下的扩展
- 在 log-normal fundamental price 下，log-return 是 Gaussian 即使价格有随机波动率
- Kyle's Lambda 和 market depth 都是 submartingales

**对 ict-engine 的启示**：
- execution cost 不是常数，而是 volatility-dependent
- OU overextension 可以挂到 Kyle lambda 上
- `execution_readiness` 的计算应该考虑 market depth

### 3.7 Crowded Trades + Market Clustering

**arXiv**: 2002.03319v1
**PDF**: https://arxiv.org/pdf/2002.03319v1

**核心贡献**：
- 实证证明 crowded trades → 价格不稳定（特别是正尾）
- 因果性已验证（控制了常见风险驱动因素）

**对 ict-engine 的启示**：
- Ising `herding_bias` 可以用拥挤度指标直接校验
- `cross_market_smt` 的有效性可以用拥挤度条件化

### 3.8 Granger Causality Stock Market Networks

**arXiv**: 1408.2985v1
**PDF**: https://arxiv.org/pdf/1408.2985v1

**对 ict-engine 的启示**：
- `cross_market_smt` 应加 Granger causality 检验作为 evidence quality 输入
- 如果 Granger 检验不显著，SMT factor 应被降权

---

## 四、发散性跨领域思路

### 4.1 从 AMM/DeFi 到传统市场 execution
RAmmStein 的 "regime-aware laziness" 概念：最佳策略有时候是不行动。你的 execution tree 可以借鉴其 free boundary 概念替代硬编码门限。

### 4.2 从进化计算失败学到的
Red Queen's Trap 证明：在信息不对称下，增加模型复杂度加剧系统脆弱性。factor mutation 必须有 execution friction 硬约束。

### 4.3 从量子场论到 Ising 升级路径
φ⁴ 场论提供了一个渐进升级路径：先落地 Ising（Sprint 2），验证后考虑连续场论版本（Sprint 4+）。

### 4.4 从拥挤交易到 Ising herding 的因果验证
用实证数据建立 Ising herding_bias ↔ 价格不稳定的因果链，而不是假设它成立。

### 4.5 从 Kyle lambda 到 execution cost 结构化
execution cost 不是常数。OU overextension + Kyle lambda = 更现实的 execution feasibility 评估。

---

## 五、论文 PDF 汇总

| # | 论文 | arXiv ID | PDF 链接 | 相关度 |
|---|------|----------|----------|--------|
| 1 | RAmmStein (OU + regime + execution) | 2602.19419v2 | arxiv.org/pdf/2602.19419v2 | ★★★★★ |
| 2 | Red Queen's Trap (evolution failure) | 2512.15732v1 | arxiv.org/pdf/2512.15732v1 | ★★★★★ |
| 3 | FactorEngine (factor mining) | 2603.16365v2 | arxiv.org/pdf/2603.16365v2 | ★★★★ |
| 4 | Ising Stylized Facts (cluster persistence) | 2512.17925v1 | arxiv.org/pdf/2512.17925v1 | ★★★★ |
| 5 | φ⁴ field theory for finance | 2512.17225v1 | arxiv.org/pdf/2512.17225v1 | ★★★ |
| 6 | Kyle's Model + Stochastic Liquidity | 2204.11069v1 | arxiv.org/pdf/2204.11069v1 | ★★★★ |
| 7 | Crowded trades + clustering | 2002.03319v1 | arxiv.org/pdf/2002.03319v1 | ★★★ |
| 8 | Granger Causality Stock Networks | 1408.2985v1 | arxiv.org/pdf/1408.2985v1 | ★★★ |
| 9 | Phase Transitions (Ising in finance) | 2504.19050v1 | arxiv.org/pdf/2504.19050v1 | ★★★（已引用） |
| 10 | OOM-RL (market-driven alignment) | 2604.11477v1 | arxiv.org/pdf/2604.11477v1 | ★★★ |

已引用但可能未下载：
| HMM ensemble voting | aimspress | aimspress.com/data/article/preview/pdf/69045d2fba35de34708adb5d.pdf |
| Bayesian Decision Tree | 2309.15312 | arxiv.org/pdf/2309.15312 |

---

## 六、建议阅读顺序

1. **RAmmStein**（2602.19419）—— 直接可迁移到 Sprint 2
2. **Red Queen's Trap**（2512.15732）—— 给 factor mutation 加安全阀
3. **Kyle's Model**（2204.11069）—— execution cost 结构化
4. **FactorEngine**（2603.16365）—— learn from failures 模块
5. **Ising Stylized Facts**（2512.17925）—— Ising overlay 校验
6. **Crowded Trades**（2002.03319）—— herding_bias 因果验证
7. **φ⁴ field theory**（2512.17225）—— 远期升级路径

---

## 七、建议行动

### 立即可做（1-2 天）
1. 下载并精读 RAmmStein，提取其 state vector 和 decision boundary 设计
2. 给 `cross_market_smt` 加 Granger causality 检验参考
3. 在 docs/ 加 risk management 章节骨架

### Sprint 2 内做
1. 把 RAmmStein 的 "regime-aware laziness" 融入 execution tree
2. 用 Kyle lambda 框架结构化 execution cost
3. 给 factor mutation 加 "learn from failures" 约束

### Sprint 3-4 做
1. 用 crowded trades 数据校验 Ising herding_bias
2. 考虑 Ising 升级为连续场论版本
3. 建立 walk-forward 验证框架
