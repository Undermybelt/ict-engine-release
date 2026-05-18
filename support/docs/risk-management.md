# Risk Management Layer

> 本文档补强 ict-engine 最大缺失：risk management。
> 基于 7 篇论文的实证结论，不是理论推导。

## 一、现有不足

ict-engine 的 hard gate 只看：
- `pre_bayes_gate_status`
- `evidence_quality_score`
- `bridge_gap`

缺少：
- 交易成本结构化（不只是 spread）
- 最大回撤约束
- 连续亏损 streak 管理
- 仓位 sizing
- 拥挤度风险
- 资本衰减监控

## 二、基于论文的 risk 层设计

### 2.1 摩擦壁垒检查（Red Queen's Trap §4.4）

每次 mutation 评估前，先算盈亏平衡胜率：

```
W_BE = (1 + C_ratio) / (1 + R)
```

- `C_ratio = round_trip_cost / risk_pct`（典型值：0.12% / 1% = 0.12）
- `R = reward_risk_ratio`
- 如果 `model_accuracy < W_BE`，无论 score_delta 多好，直接 reject

**为什么**：Red Queen's Trap 证明 51.2% 的方向准确率低于 55% 盈亏平衡线，
系统本质上是"把资本转移到交易所手续费收入"的机器。

### 2.2 成本感知 PnL（Red Queen's Trap §4.1）

每个 trade 的净 PnL：

```
PnL_Net = (P_exit - P_entry) × Q - 2 × (P × Q × Fee)
```

**关键**：只看毛利不看净利，会产生 "Fool's Gold" —— 毛利正但净利负。

ict-engine 的 `selected_win_probability` 应该用净 PnL 而不是毛 PnL 计算。

### 2.3 Kyle Lambda 执行成本（Kyle's Model §1）

执行成本不是常数，是市场状态的函数：

```
λ = Cov(ΔP, Q) / Var(Q)
Cost = λ × |order_size|
```

**Submartingale 性质**（§Corollary）：λ 随时间递增 → 执行成本递增 → 早做优于晚做。

ict-engine 的 `execution_readiness` 应该考虑：
- 当前 λ vs 历史 λ（成本是否在上升）
- 市场深度 1/λ（流动性是否在恶化）

### 2.4 存活偏差检测（Red Queen's Trap §4.2）

autoresearch 的 keep/discard 机制天然有存活偏差：
- 只报告 "best attempt" → 忽略 dead mutations
- 累积 score_delta 可能为负，但 "best" 看起来在改善

**检查项**：
- 报告全部 population 统计，不只是 accepted
- 检测 stagnant mutations（accepted 但 score_delta ≈ 0）
- 检测 zombie mutations（accepted 但负 delta）

### 2.5 模态崩溃监控（Red Queen's Trap §4.3）

如果所有 factor 同意一个方向 → 不是强信号，是模态崩溃。

**指标**：
- 因子方向一致率 > 80% → warning
- effective_n_strategies < 2 → warning
- 拥挤度（Crowded Trades §Abstract）→ 正尾风险

### 2.6 资本衰减跟踪（Red Queen's Trap §4, Figure 1）

autoresearch 的累积 score_delta 应该像资本曲线一样监控：
- 单调衰减 → system is decaying
- 最大回撤 > 20% → warning
- 需要 bailout（接受负 delta mutation）→ soft budget constraint

### 2.7 拥挤度风险（Crowded Trades §Abstract）

Ising `herding_bias` + 因子方向一致 = 双重拥挤：
- 拥挤 LONG → 正尾风险（flash crash up）
- 拥挤 SHORT → 负尾风险（仅在动荡期）
- 因果性已验证（控制了常见风险驱动因素后仍显著）

### 2.8 Cluster 持久性验证（Ising Stylized Facts §Abstract）

Ising overlay 的有效性应该用 cluster persistence 验证：
- cluster persistence ↔ realized vol 相关性
- cluster reorganization 率 → regime transition 预警
- giant component fraction → 市场集中度

### 2.9 Kurtosis 监控（φ⁴ Field Theory §Abstract）

Ising 二值化丢掉 kurtosis 信息：
- 超额 kurtosis > 0.5 → 重尾风险
- 可作为 Ising overlay 的补充指标
- 远期可升级为连续场论版本

## 三、集成优先级

### 立即可做（不影响现有架构）
1. `research-gaps-and-paper-synthesis.md` 已完成 ✓
2. paper2code 模块已写好、测试通过 ✓
3. 本文档已写好 ✓

### Sprint 2 内做
1. 在 `factor_mutation_evaluation` 入口加摩擦壁垒检查
2. 在 `autoresearch` 尾部加存活偏差 + 资本衰减报告
3. 在 `Ising overlay` 后加 cluster persistence 验证

### Sprint 3-4 做
1. Kyle lambda 集成到 `execution_readiness` 计算
2. 模态崩溃监控集成到 `factor ranking`
3. Kurtosis 监控集成到 `physics overlay`
4. φ⁴ 连续场论升级（替代 Ising 二值化）

## 四、论文证据链

| 风险层 | 论文 | 证据强度 |
|---|---|---|
| 摩擦壁垒 | Red Queen's Trap §4.4 | 实证：500 agent，51.2% < 55% |
| 成本感知 PnL | Red Queen's Trap §4.1 | 实证：Fool's Gold 现象 |
| Kyle lambda | Kyle's Model §Corollary | 理论+实证：submartingale |
| 存活偏差 | Red Queen's Trap §4.2 | 实证：60% stagnant |
| 模态崩溃 | Red Queen's Trap §4.3 | 实证：500 agent → 1 portfolio |
| 资本衰减 | Red Queen's Trap §5 | 实证：>70% decay |
| 拥挤度 | Crowded Trades §Abstract | 实证+因果：MiFID 数据 |
| Cluster persistence | Ising Stylized Facts §Abstract | 实证：S&P500 |
| Kurtosis | φ⁴ Field Theory §Abstract | 实证：2008 危机 |
