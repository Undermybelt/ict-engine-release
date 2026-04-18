# ICT Factor Mutation Optimization Plan

> 基于 state100 batch2 六轮实验结果 + 仓内已有 cluster 机制的分析。
> 目标：提升 `structure_ict` 因子 mutation 的 acceptance rate 和 score_delta。

## 1. 现状诊断

### 1.1 实验结果（state100/batch2, 6 runs）

| run | lookback | expansion_threshold | sweep_weight | score_delta | failure_tags |
|-----|----------|---------------------|--------------|-------------|--------------|
| 1   | 6.0      | 0.97                | 0.98         | -0.095      | composite_regressed |
| 2   | 7.0      | 1.04                | 1.06         | -0.067      | composite_regressed, bridge_gap_too_small, pre_bayes_gate_regressed |
| 3   | 8.0      | 1.11                | 1.14         | -0.013      | composite_regressed |
| 4   | 9.0      | 1.18                | 1.22         | -0.044      | composite_regressed, bridge_gap_too_small, pre_bayes_gate_regressed |
| 5   | 10.0     | 1.25                | 1.30         | -0.002      | composite_regressed |
| 6   | 11.0     | 1.32                | 1.38         | -0.024      | composite_regressed, bridge_gap_too_small, pre_bayes_gate_regressed |

### 1.2 关键发现

- **全部 rejected**，score_delta 全负。
- **run5 最接近中性**（delta=-0.002），参数：lookback=10, expansion=1.25, sweep_weight=1.30, unconfirmed=0.55。
- **偶数轮额外惩罚**：`evaluate_expansion_preview=true` 触发 bridge_gap + pre_bayes gate，增加 rejection 概率。
- **baseline top 排名恒定**：`[structure_ict, trend_momentum, volatility_mean_reversion]`，mutation 未能打破排名。
- **modulo 线性扫面太窄**：参数单调递增，未探索非线性组合或反向区域。
- **composite_score / factor_scores 未回填**：引擎只给 delta 和排名，缺乏细粒度归因。

### 1.3 根因

1. 参数扫面是 modulo 线性递增，只覆盖一个方向，错过局部最优盆地。
2. `evaluate_expansion_preview` 开启时 gate 过严，在探索阶段不宜同时开启。
3. 缺少 cluster jump 机制 — 仓内已有 4 个 cluster preset 但 batch 脚本未使用。
4. 单次 batch 无 baseline 锚定 — 每轮独立跑，无法做差分对比。

## 2. 优化方案

### Phase 1: 局部精搜（围绕 run5 最优点）

目标：在 run5 参数附近做 ±小步长网格搜索。

```
中心点（run5）:
  lookback=10, expansion_threshold=1.25, sweep_atr_multiplier=1.05
  sweep_weight=1.30, unconfirmed_sweep_weight=0.55
  opposing_sweep_penalty=1.20, post_sweep_displacement_weight=1.25
  sweep_recency_bars=8, sweep_return_bars=7

搜索策略:
  - 每参数 ±2 步，步长取 step_size_hints 的 50%
  - evaluate_expansion_preview=false（去掉额外 gate）
  - 预计 ~50 轮
```

### Phase 2: Cluster Jump 路径

仓内已有 4 个 cluster preset（`src/main.rs` L4797-4820），应直接用 autoresearch 的 cluster 机制：

| cluster | 核心参数偏移 | ICT 概念映射 |
|---------|-------------|-------------|
| `displacement_fvg_cluster` | displacement_weight↑, sweep_weight↓, unconfirmed↓ | FVG + displacement 质量 |
| `mss_bos_cluster` | lookback↑, expansion↑, return_bars↑, penalty↑ | BOS/CHoCH 结构确认 |
| `premium_discount_ote_cluster` | lookback↑↑, expansion↓, recency↑, return↑ | OTE 区间 + dealing range |
| `smt_cluster` | lookback↑↑↑, atr_mult↓, sweep_weight↓, penalty↑ | SMT divergence 跨品种 |

用法：
```bash
cargo run -- factor-autoresearch \
  --symbol NQ \
  --data ... --data-1m ... --data-5m ... --data-15m ... --data-1h ... --data-4h ... --data-1d ... \
  --objective expansion_manipulation \
  --ensemble \
  --state-dir state_cluster_sweep \
  --iterations 25 \
  --cluster-jump-enabled
```

每个 cluster 跑 25 轮，共 100 轮，覆盖 4 个 ICT 概念族。

### Phase 3: 细粒度归因补全

当前 `factor_mutation_evaluation` 不回填 `composite_score` / `factor_scores`。需要在引擎侧补：

位置：`src/main.rs` 中 `baseline_factor_mutation_metrics` 返回的 `FactorMutationMetricSet`。

改动：
1. 在 `metrics_before` / `metrics_after` 中填入 `composite_score` 和 per-factor scores。
2. 输出 JSON 增加 `mutation_spec_echo` 字段，回显实际注入的参数（便于离线分析）。

### Phase 4: 扩展 ICT 因子面（中期）

基于 ICT 概念研究，当前 `structure_ict` 只覆盖 sweep 相关参数。可扩展的因子面：

| 新参数/子因子 | ICT 概念 | 工程映射 |
|--------------|---------|---------|
| `fvg_min_gap_atr` | Fair Value Gap 最小宽度 | 3K gap / ATR 阈值 |
| `ob_displacement_min` | Order Block 位移确认 | 最后反向 K 后的 body/ATR |
| `bos_follow_through` | BOS 跟进力度 | break 后 N 根 K 的方向一致性 |
| `kill_zone_weight` | Kill Zone 时段加权 | London/NY open 时段内信号加权 |
| `inducement_depth` | Inducement 诱导深度 | 内部/外部流动性层级 |
| `breaker_retest_horizon` | Breaker Block 回测窗口 | 失效 OB 翻转后的 retest 反应 |

这些需要在 `FactorRegistry` 中注册新参数，并在 `structure_ict` 因子计算逻辑中消费。

## 3. 执行优先级

1. **Phase 1**（立即可做）：修 batch 脚本，围绕 run5 精搜 50 轮
2. **Phase 2**（立即可做）：用仓内 cluster jump 跑 4×25 轮
3. **Phase 3**（1-2h 改动）：补全 composite_score 回填 + spec echo
4. **Phase 4**（中期）：扩展因子参数面，需要改 factor 计算逻辑

## 4. 成功标准

- Phase 1+2 后至少 1 个 mutation accepted（score_delta > 0）
- Phase 3 后每轮输出包含可对比的 composite_score
- Phase 4 后 `structure_ict` 参数面从 9 个扩展到 15+

## Appendix: 参数默认值参考

从 `src/factor_lab/factor_definition.rs` 和 run5 推断的 baseline 附近值：

```
lookback:                       ~10
expansion_threshold:            ~1.25
sweep_atr_multiplier:           ~1.05
sweep_weight:                   ~1.30
unconfirmed_sweep_weight:       ~0.55
opposing_sweep_penalty:          ~1.20
post_sweep_displacement_weight: ~1.25
sweep_recency_bars:             ~8
sweep_return_bars:              ~7
```
