# ATLAS-GIC → ICT-Engine 整合计划

> 来源：https://github.com/chrisworsey55/atlas-gic (944 stars, General Intelligence Capital)
> 状态：设计阶段，未开始实现
> 依赖：factor-autoresearch-minimal-loop (v0 已设计), autoresearch-derived-surfaces (v1 已设计)

## ATLAS-GIC 核心概念

ATLAS-GIC 将 Karpathy 的 autoresearch 循环应用于交易 agent：
- **prompt 即权重**：agent 的 prompt 文件就是被优化的参数
- **Sharpe 即损失函数**：用滚动 Sharpe ratio 替代 validation loss
- **git commit/revert**：改进保留、不改进回滚
- **达尔文权重**：top quartile ×1.05, bottom ×0.95, 范围 [0.3, 2.5]
- **JANUS 元层**：多 cohort 加权，weight differential 作为 emergent regime detector
- **PRISM (All Seasons)**：按 regime 分别训练不同 cohort，同一起点不同进化路径
- **Agent Spawning**：知识盲区出现 3+ 次 → 自动生成新 specialist agent
- **Soros 反身性**：price→fundamentals, P&L→behavior, narrative→flows, market→policy

## ICT-Engine 现有锚点

| ATLAS-GIC 概念 | ICT-Engine 现有对应 | 差距 |
|---------------|-------------------|------|
| Autoresearch loop | `factor-autoresearch` CLI + session/attempt JSON | 已有 v0 设计，缺达尔文权重 |
| Darwinian weights | 固定 factor weights in registry | 无自动调整机制 |
| Multi-layer agents | BBN topology (7 nodes) | 单层，无分层 debate |
| JANUS meta-layer | Pre-Bayes evidence filtering | 无 cohort 加权 |
| PRISM regime training | Regime-aware factor pipeline | 无分 cohort 训练 |
| Agent spawning | Factor mutation + discovery | 被动 mutation，非主动 spawning |
| Soros reflexivity | Ising model for market dynamics | 有物理模型，无反身性循环 |
| Rolling Sharpe scoring | `FactorMutationEvaluation` metrics | 有 aggregate_return，缺 rolling Sharpe |

## 整合方案

### 1. 达尔文因子权重（Darwinian Factor Weights）

**概念**：ATLAS-GIC 的每个 agent 有 Darwinian weight [0.3, 2.5]，每日按表现自动调整。映射到 ict-engine：每个 factor family 有 Darwinian weight，按 rolling Sharpe 自动调整。

**实现**：
- 新增 `FactorDarwinianWeight` struct in `src/state/types.rs`
  - `factor_family: String`
  - `weight: f64` (范围 [0.3, 2.5])
  - `rolling_sharpe: f64`
  - `last_updated: DateTime<Utc>`
  - `history: Vec<WeightUpdate>`
- 新增 `factor_darwinian_weights.json` state 文件
- 更新规则：每日 after factor-research run
  ```rust
  if factor in top_quartile_sharpe {
      weight = min(2.5, weight * 1.05)
  } else if factor in bottom_quartile_sharpe {
      weight = max(0.3, weight * 0.95)
  }
  ```
- BBN 的 `factor_alignment` node 使用 Darwinian weight 加权

**CLI**：
- `factor-darwinian-status` — 查看当前权重、历史变化、quartile 排名
- `factor-darwinian-reset` — 重置到 1.0

**文件变更**：
- `src/state/types.rs` — 新 struct + constant
- `src/state/persistence.rs` — load/save helpers
- `src/main.rs` — 新 command
- `src/bbn/trading/topology.rs` — weight integration

### 2. JANUS 元层：Regime-Weighted Factor Blending

**概念**：ATLAS-GIC 的 JANUS 层坐在多个 cohort 之上，按 rolling accuracy 动态加权。weight differential 作为 emergent regime detector（short-window outperform → NOVEL_REGIME）。

**映射到 ict-engine**：
- 不同 factor family 视为不同 "cohort"
- 按 regime context 动态调整 factor family 权重
- weight differential 作为 regime 检测信号

**实现**：
- 新增 `FactorJanusLayer` in `src/application/janus/mod.rs`
  - `cohort_weights: BTreeMap<String, f64>` — factor family → weight
  - `rolling_window: usize` — 30 days default
  - `regime_threshold: f64` — 0.15 for regime detection
- 每日 cycle：
  1. 计算每个 factor family 的 rolling hit_rate + Sharpe
  2. Softmax + constraints (min 0.2, max 0.8)
  3. Blend recommendations across families
  4. Weight differential → regime signal
- Regime signal 映射：
  - `NOVEL_REGIME` → 短期 factor outperform → 市场处于非常规状态
  - `HISTORICAL_REGIME` → 长期 factor outperform → 市场处于经典模式
  - `MIXED` → 均衡

**CLI**：
- `factor-janus-status` — cohort weights, regime signal, blended recommendations
- `factor-janus-run` — 执行一次 JANUS cycle

**文件变更**：
- 新 `src/application/janus/mod.rs`
- `src/state/types.rs` — JanusState, JanusCohortMetrics
- `src/state/persistence.rs` — janus state helpers
- `src/main.rs` — janus commands

### 3. PRISM：Regime-Specific Factor Training

**概念**：ATLAS-GIC 训练 5 个 cohort，分别在 Bull/Low Vol, Crisis, Rate Tightening, Recovery, Euphoria 下进化。同一起点不同进化路径，发现 convergent 和 divergent evolution。

**映射到 ict-engine**：
- 按 regime context 运行独立的 factor-autoresearch session
- 比较不同 regime 下同一 factor 的进化路径
- 发现 convergent rules（所有 regime 都学到的 meta-rules）

**实现**：
- 扩展 `factor-autoresearch` 支持 `--regime-context` 参数
- 每个 regime context 独立的 session ledger
- 新增 `factor-prism-compare` command：跨 regime 对比同一 factor 的进化
- Convergent rule detection：如果所有 regime 都收敛到类似策略，标记为 meta-rule

**CLI**：
- `factor-autoresearch --regime-context bull_low_vol --symbol BTCUSDT ...`
- `factor-prism-compare --symbol BTCUSDT` — 跨 regime 对比报告

**文件变更**：
- `src/main.rs` — regime-context parameter + prism-compare command
- `src/state/types.rs` — PrismRegime enum, PrismComparison

### 4. Factor Spawning：主动发现知识盲区

**概念**：ATLAS-GIC 在 debate 中检测到重复盲区 3+ 次 → 自动生成新 specialist agent。映射到 ict-engine：factor evaluation 中检测到重复 failure pattern → 自动生成新 factor mutation。

**实现**：
- 在 `factor-autoresearch` session 中追踪 failure_tags 频率
- 当同一 failure_tag 出现 3+ 次 → 自动生成针对性 mutation spec
- 新 factor 以 neutral weight (1.0) 加入 registry
- 如果新 factor 在 N 次 attempt 后仍低于 threshold → 标记为 "extinct"
- 如果达到 threshold → 标记为 "survived"，weight 从 1.0 开始达尔文调整

**CLI**：
- `factor-spawn-status` — 查看 spawned factors、survival 状态
- 自动集成到 `factor-autoresearch` loop 中

**文件变更**：
- `src/state/types.rs` — FactorSpawn, SpawnStatus
- `src/main.rs` — spawn status command
- autoresearch loop 增加 spawn detection

### 5. Soros 反身性循环 → Ising 增强

**概念**：ATLAS-GIC 建模 5 个反身性循环：price→fundamentals, P&L→behavior, narrative→flows, market→policy, reflexive reversal detection。

**映射到 ict-engine**：Ising model 已有 spin interaction，但缺少反身性反馈。增强 Ising model 加入 feedback loops。

**实现**：
- 扩展 `IsingState` 增加反身性 feedback fields
  - `price_fundamentals_feedback: f64` — 价格变动 → 基本面变化
  - `pnl_behavior_feedback: f64` — P&L → 行为变化（追涨杀跌）
  - `narrative_flows_feedback: f64` — 叙事 → 资金流
  - `reflexive_extreme_detected: bool` — 反身性极端信号
- 反身性 reversal detection：feedback loop 单向运行 5+ 轮 → 标记为反身性极端

**文件变更**：
- `src/factors/ising.rs` 或新建 `src/factors/reflexivity.rs`
- `src/state/types.rs` — ReflexivityState

### 6. Rolling Sharpe 评分增强

**概念**：ATLAS-GIC 的核心评分是 rolling Sharpe ratio，用 conviction-weighted returns 计算。

**映射到 ict-engine**：当前 `FactorMutationEvaluation` 有 `aggregate_return` 但缺少 rolling Sharpe。需要增加。

**实现**：
- 在 `FactorMutationMetricSet` 中增加：
  - `rolling_sharpe_60d: f64`
  - `rolling_sharpe_20d: f64`
  - `conviction_weighted_return: f64`
- Sharpe 计算：
  ```rust
  fn rolling_sharpe(returns: &[f64], lookback: usize) -> f64 {
      let window = &returns[returns.len().saturating_sub(lookback)..];
      let mean = window.iter().sum::<f64>() / window.len() as f64;
      let variance = window.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (window.len() - 1) as f64;
      let std_dev = variance.sqrt().max(0.0001);
      (mean / std_dev) * (252.0_f64).sqrt() // annualized
  }
  ```
- Autoresearch loop 的 keep/discard 改用 rolling_sharpe 改善作为主要判据

**文件变更**：
- `src/state/types.rs` — 增加 Sharpe fields
- factor-research evaluation 增加 Sharpe 计算

## 实现优先级

| 优先级 | 模块 | 复杂度 | 价值 | 前置依赖 |
|--------|------|--------|------|----------|
| P0 | Rolling Sharpe 评分 | 低 | 高 | 无 |
| P0 | Darwinian 因子权重 | 中 | 高 | Rolling Sharpe |
| P1 | JANUS 元层 | 中 | 高 | Darwinian weights |
| P1 | Factor Spawning | 中 | 中 | Autoresearch loop |
| P2 | PRISM regime training | 高 | 中 | Autoresearch + JANUS |
| P2 | Soros 反身性 | 高 | 中 | Ising model |

## 与现有设计的关系

- **factor-autoresearch-minimal-loop (v0)**：达尔文权重和 Rolling Sharpe 直接增强 v0 的 keep/discard 判据
- **autoresearch-derived-surfaces (v1)**：JANUS 和 PRISM 的输出可以作为新的 derived surface
- **BBN topology**：Darwinian weights 直接影响 `factor_alignment` node 的 evidence
- **Ising model**：反身性循环是 Ising spin interaction 的自然扩展

## 验证路径

1. Rolling Sharpe：单元测试 + 与 numpy 对照
2. Darwinian weights：模拟 30 天数据验证权重收敛
3. JANUS：两 cohort 对比测试
4. PRISM：同一 factor 在不同 regime 下的进化对比
5. Spawning：注入 failure_tag 频率测试 spawn 触发
6. 反身性：Ising simulation 中加入 feedback loop 验证极端检测
