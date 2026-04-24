# The Well 物理模拟启示 → ict-engine 优化 Plan

> 来源 insight doc: `docs/the_well-physics-sim-ict-insights.md`
> 对齐主计划: `docs/plans/execution-operating-system-plan.md`
> 硬性原则: execution feasibility > execution attribution > prediction value > regime explanation
> 日期: 2026-04-21

## 0. 立场（在开干之前必须先锁死）

The Well 是物理场代理模型 (surrogate model) 的数据+网络集合。它的价值**不是**给 ict-engine 再加一套神经算子，而是给**execution-first 物理层**多补四件现成的数学工具：

- 频域学习 (FFT + softshrink)
- 轴向注意力 (axial attention)
- 张量分解 (Tucker)
- 分段 rollout 评估 (VRMSE + 窗口)

**任何提议如果不能映射到 `ExecutionFeatures` / `ExecutionPhysicsOverlay` / `ExecutionArtifact` / `hard gate` / `artifact_ledger` 之一，直接剔除**。不给 ict-engine 增加一个"只看起来像 paper"的孤立 demo。这是跨 Sprint 约束 §2 的底线。

## 1. 可迁移 insight × ict-engine 落点表

| Well 概念 | 数学本质 | ict-engine 落点 | 首要服务对象 | Sprint 挂载 |
|-----------|---------|----------------|-------------|-------------|
| FNO 频域模式 | `rfftn` 主模 + 相位 | `ExecutionFeatures.cycle_phase_alignment` / `dominant_cycle_energy` | execution feasibility (节奏性能否承接) | Sprint 2 physics overlay |
| AFNO softshrink | `F.softshrink(x, λ)` 稀疏阈值 | `factor_lab::metrics::sparse_select` + MECE recovery 因子剪枝 | factor ranking 显著性 | Sprint 3 recovery loop |
| TFNO Tucker 分解 | 高阶张量低秩核心 | `factor_lab::research::tucker_core` (factor × regime × timeframe) | 可比较性 + 参数效率 | Sprint 3/4 |
| AViT 轴向注意力 | 沿轴拆分 Q/K/V | MTF 聚合层 `application/orchestration/axial_pool.rs` | ExecutionTree 输入效率 | Sprint 4 |
| VRMSE + 窗口评估 | `(6:12)` / `(13:30)` rollout | `MeceRecoveryReport` 扩窗 + `execution_readiness_drift` | hard gate 覆盖 horizon | Sprint 3 |
| HDF5 分层存储 | trajectory → step → coord | `state/<SYMBOL>/artifact_ledger` 升级为 `.h5` (可选) | artifact 规模治理 | post-Sprint 4 (仅在体积到阈值) |
| 复数 MLP | 保留相位 | 频域 overlay 保留 `Complex64` 输出 | 周期性相位 attribution | Sprint 2 |
| 块对角权重 | 分块独立线性 | 多 symbol / timeframe 分块 scorer | 未到规模，**暂不做** | — (后置) |

上表第 8 行和 HDF5 明确**延后**：ict-engine 当前 symbol / artifact 量级不够大，先堆这两个属于过早优化。

## 2. 模块级优化提案

### 2.1 频域 execution 特征 (FNO-inspired, Sprint 2 增量)

**动机**：OU 告诉你"过伸能不能回"；但 OU 只是一阶均值回复。真实市场里，均值回复是否**节奏可承接**还受周期结构控制。FFT 主模 + 能量谱可以补这一层。

**新增**：

```
src/math/spectral.rs                    — 纯数学：rfft, power_spectrum, dominant_mode, softshrink
src/domain/execution/spectral.rs        — SpectralExecutionMetrics
src/application/belief/spectral_overlay.rs — 叠到 execution feasibility
```

```rust
// src/domain/execution/spectral.rs
pub struct SpectralExecutionMetrics {
    pub dominant_cycle_energy: f64,       // 主模能量占比 ∈ [0, 1]
    pub dominant_cycle_period_bars: f64,  // 主周期长度 (bars)
    pub cycle_phase_alignment: f64,       // 当前价 vs 主周期相位 ∈ [-1, 1]
    pub spectral_entropy: f64,            // 频谱熵 — 越低越"节奏清楚"
    pub high_freq_noise_ratio: f64,       // softshrink 前后能量比
}

pub fn estimate_spectral_execution_metrics(
    prices: &[f64],
    sparsity_lambda: f64,
) -> Option<SpectralExecutionMetrics>;
```

**核心公式**（直接引用 insight doc 的 AFNO 块）：

```rust
let x_freq = rfft(&prices);                // Complex64
let x_sparse = softshrink(&x_freq, lambda);// 过滤高频噪声
let dominant = argmax_energy(&x_sparse);
let phase_align = phase_alignment(prices.last(), &dominant);
```

**挂 execution 的方式**：
- 扩 `ExecutionFeatures` 加三个 `Option<f64>`: `dominant_cycle_energy` / `cycle_phase_alignment` / `spectral_entropy`
- `ExecutionPhysicsOverlay` 增 `pub spectral: Option<SpectralExecutionMetrics>`
- 拼入 `build_execution_physics_overlay()` 的现有 `pipeline_builder` / `policy_engine` / `trace_builder` 三消费点，**不重算**

**hard gate 接法**（满足约束 §2 "物理学层必须有 hard gate 作用点"）：
- `spectral_entropy > θ_chaos` **且** `dominant_cycle_energy < θ_energy` → 降级 `execution_readiness`（叠在现有降级路径上，不新开 gate 字段）
- 阈值常量进 `src/domain/execution/gates.rs`

**artifact / comparability**：
- `execution_artifact.json` 新增 `spectral.*` 四字段
- `artifact_ledger.json` kind=`execution_artifact` 的 schema 增版本号
- `reflection_bundle` 在 `why_execution_dominates` 里允许引用 "cycle_phase_alignment=+0.78"

**测试** (跨 Sprint 约束 §3 "不允许只加模型不加 regression")：
- `tests/spectral_smoke.rs` — 纯周期信号 → `dominant_cycle_energy > 0.9`；白噪声 → `spectral_entropy` 上界
- `tests/spectral_overlay_gate.rs` — 高熵 + 低主模能量 → readiness 降级路径打开
- fixture hash 锁定（与 Sprint 3 `viterbi_output_hash` 同模式）

**量化目标**：
- Phase A 已有 symbol 全量 re-analyze 一次，`execution_artifact.json` 100% 带 spectral 字段；真实拟合占比 ≥ 90%（与 OU 真实拟合 KPI 同模板）

### 2.2 Softshrink 稀疏因子筛选 (AFNO-inspired, Sprint 3 增量)

**动机**：当前 `factor_lab/research.rs` 的 MECE recovery 搜索是基于 IC / gain 排序的阈值裁剪。问题：阈值硬编码、裁剪不带可比较的"稀疏度量"。softshrink 本质是**可微分的 L1 收缩**，把微弱因子自动推到 0，保留显著因子，副产物是一条全局 sparsity trace 可进 artifact。

**新增**：

```
src/factor_lab/sparse.rs
```

```rust
pub struct SparseSelection {
    pub kept_factors: Vec<String>,
    pub pruned_factors: Vec<(String, f64)>, // (name, pre_shrink_weight)
    pub sparsity_ratio: f64,                // kept / total
    pub lambda: f64,                        // softshrink 阈值
}

pub fn sparse_select_by_softshrink(
    weights: &BTreeMap<String, f64>,
    lambda: f64,
) -> SparseSelection;
```

**挂进 MECE recovery loop**：
- `search_factors_for_mece_recovery` 每轮在 `registry` 计算 IC / gain 后，先走 `sparse_select_by_softshrink` 再进 HMM Viterbi 评估
- `MeceRecoveryReport` 扩：
  ```rust
  pub sparsity_ratio: f64,
  pub pruned_factor_trail: Vec<(String, f64)>,
  ```
- `mece_recovery_artifact` 带同名字段入 ledger

**hard gate 接法**：
- `sparsity_ratio < 0.1` (剪得过狠) **或** `sparsity_ratio > 0.9` (几乎没剪) → 视为搜索异常，**不许** promote（同 `accuracy < 0.95` 的阻断方式）

**double constraint 与 execution 一致性**（对齐主计划 Sprint 3 §3.2 的强约束）：
- softshrink 后的 kept factor set 必须在 `execution_validity_histogram` 里落入 `execution_ready ≥ 50%`
- 否则即使 accuracy 过 0.95 也**block**（即不允许"稀疏选出一批因子但 execution 全堵"）

**测试**：
- `tests/sparse_select.rs` — 3 case: 全零、均匀、尖峰
- `tests/mece_recovery_sparse_gate.rs` — accuracy 过关但 execution_histogram 偏 blocked → hard gate 阻断

### 2.3 Tucker 因子张量分解 (TFNO-inspired, Sprint 3/4 交接)

**动机**：factor × regime × timeframe 天然是三阶张量。当前 `factors/regime_conditional.rs` + `weight_updater.rs` 是"按 regime 维度切片再独立更新"，等价于假设三维之间完全独立。Tucker 分解给一个**共享 execution 核心张量 + 三组因子矩阵**的显式低秩结构，更省参数且给出**可解释的因子-regime-timeframe 共基**。

**新增**：

```
src/factor_lab/tucker.rs
```

```rust
pub struct TuckerCore {
    pub core: Vec<Vec<Vec<f64>>>,          // shape: [rf, rr, rt]
    pub factor_loadings: Vec<Vec<f64>>,    // [n_factor × rf]
    pub regime_loadings: Vec<Vec<f64>>,    // [n_regime × rr]
    pub timeframe_loadings: Vec<Vec<f64>>, // [n_tf × rt]
    pub reconstruction_error: f64,
    pub rank_triplet: (usize, usize, usize),
}

pub fn fit_tucker_core(
    tensor: &Array3<f64>,
    ranks: (usize, usize, usize),
) -> Option<TuckerCore>;
```

实现注意：Rust 生态可选 `ndarray` + HOSVD (高阶 SVD) 作为 warm start，迭代用 ALS (alternating least squares)。**不引入 Python bridge**，保留 Rust 单一执行面。

**挂进 artifact**：
- 新 artifact kind: `factor_tucker_core`
- `state/<SYMBOL>/factor_tucker_core.json` 落盘
- 作为 MECE recovery artifact 的 sibling 而非替代

**execution 角度的价值**：
- 核心张量 `[rf, rr, rt]` 的主元素对应"哪一类因子 × 哪一类 regime × 哪一类 timeframe 同时高"
- 写入 reflection_bundle `execution_summary` 的证据条：e.g. "当前 regime 在 core[2,1,0] 载荷 0.78 的对角上，对应 PDArray 主因子族 × expansion × HTF"
- **仅作为 execution attribution 的补充证据**，不影响 gate 决策本身

**hard gate 接法**（间接）：
- `reconstruction_error > θ` → 不阻断 promotion，但在 reflection_bundle 标 `execution_attribution_confidence = low`
- Sprint 4 SHAP attribution 强制绑定时，low confidence 触发 top_k 从 5 提到 10（换更多证据线）

**测试**：
- `tests/tucker_smoke.rs` — 合成张量 (rank 2) → 恢复误差 < 1e-6
- `tests/tucker_artifact_lineage.rs` — tucker_core 版本切换 → artifact_ledger 有 diff entry

**量化目标**：
- Sprint 3 结束时至少有 1 份真实 symbol 的 `factor_tucker_core.json`
- Sprint 4 `--execution-focus` 输出面可 opt-in 显示 tucker 载荷 top-3

### 2.4 轴向注意力 MTF 聚合 (AViT-inspired, Sprint 4 增量)

**动机**：Sprint 4 的 `ExecutionTreeScorer` 输入里有 `ExecutionFeatures` + `ExecutionPhysicsOverlay` + HMM posterior + MECE recovery confidence + prediction vote score，**再加**多 timeframe。如果无脑拼接维度二次膨胀。AViT 的轴向注意力是 O(n√n) 而非 O(n²)，直接省下来。

**新增**：

```
src/application/orchestration/axial_pool.rs
```

```rust
pub struct AxialPoolConfig {
    pub timeframe_axis_weights: Vec<f64>,  // per-tf 相对重要度
    pub feature_axis_weights: Vec<f64>,    // per-feature
    pub softmax_temperature: f64,
}

pub fn axial_pool_mtf_features(
    tensor: &Array3<f64>,     // [timeframe × feature × time]
    config: &AxialPoolConfig,
) -> Array2<f64>;             // [feature × time]
```

实现：三轴分别 softmax 权重求和，**不做 NN 训练**，权重来自 `factor_tucker_core` 的 loading（把 2.3 的产物喂回这里形成内环）或 fallback 均匀。

**挂法**：
- `ExecutionTreeInput` 在构造前先过 `axial_pool_mtf_features`
- `ExecutionTreeOutput` 新增 `axial_attention_trace: Vec<(String, f64)>` (top-k 贡献维度)
- 写入 `execution_tree_trace.json`

**hard gate 接法**：
- 轴向权重分布熵过高（无主导维度）→ `branch = "observe"` 强制，而非 `"fill_viable"`

**与 SHAP 的关系**（对齐 Sprint 4 §4.2）：
- `axial_attention_trace` 是 MTF 层的 attribution，SHAP 是 voting 层的 attribution
- 两者都进 reflection_bundle，互为交叉验证

**测试**：
- `tests/axial_pool.rs` — 单主导轴 → trace top1 权重 > 0.6
- `tests/execution_tree_axial_gate.rs` — 高熵 → observe 分支强制

### 2.5 分段 rollout 评估 (VRMSE-inspired, Sprint 3 扩展)

**动机**：主计划 Sprint 3 的 MECE recovery 用 single-step accuracy。The Well 的 `(6:12)` / `(13:30)` 窗口模式揭示一个事实：**短期 vs 中期 recovery 稳定性不同**。对 execution 有直接意义 — 短期 execution_ready 但中期漂走 block 的 run 是真实隐患。

**扩展 `MeceRecoveryReport`**：

```rust
pub struct RolloutSegment {
    pub horizon_bars: (usize, usize),    // e.g. (1, 5)
    pub accuracy: f64,
    pub execution_readiness_mean: f64,
    pub execution_readiness_drift: f64,  // readiness 斜率
}

// 新字段
pub segments: Vec<RolloutSegment>,       // 至少 3 段: short/medium/long
```

**强约束**：
- 段 `(1, 5)` 必须 ≥ 0.95（原 threshold）
- 段 `(6, 20)` ≥ 0.85
- 段 `(21, 50)` ≥ 0.75
- `execution_readiness_drift` 每段斜率不得 < -0.03 / bar

任何一段 break → hard gate 阻断 promotion。

**对应 test**：
- `tests/mece_recovery_rollout_segments.rs`
- fixture hash 加 segment 维度

### 2.6 延后的（显式不做）

| 项 | 原因 |
|----|------|
| 训练真实 FNO/TFNO 网络 | ict-engine 是 stateful research OS，不是 surrogate model 训练平台；新增训练面破坏 Rust 单一执行面 |
| 块对角权重多 symbol scorer | 当前 symbol 覆盖量 < 50，过早分块；若 Sprint 4 后量级上 200+ 再议 |
| HDF5 作为 artifact_ledger 主存 | 当前 state/* 是 JSON + ledger 追加，并未到 HDF5 的规模拐点；**保留作为后续独立 research note** |
| 复数 MLP 主存格式 | 频域 overlay 内部用 `Complex64` 计算即可，不把 `Complex64` 暴露进 `ExecutionArtifact` schema (序列化成本大) |
| 整个 AViT 模型 | 只借"轴向聚合"这一个原语；训练 Transformer 不在 scope |

## 3. 与现有 4 Sprint 的对齐 (不新增 Sprint)

| Sprint | 原目标 | 本 plan 增量 | 是否破坏约束 |
|--------|-------|-------------|-------------|
| Step 0 | Phase A 收尾 | 无 | — |
| Sprint 1 | Execution-First 基础 | 无（执行层结构不动） | — |
| Sprint 2 | 物理学注入 | **+2.1 频域 overlay** 作为 OU/Ising/Pythagorean 的第四个 physics 成员 | 不破坏 (完全对齐 §2.4 `ExecutionPhysicsOverlay` 结构) |
| Sprint 3 | MECE recovery 闭环 | **+2.2 softshrink** + **+2.5 rollout 分段** + **+2.3 tucker 落地** | 不破坏 (强化 recovery double constraint) |
| Sprint 4 | Execution Tree 产品化 | **+2.4 axial pool** + **+2.3 tucker 进 SHAP 旁支** | 不破坏 (仍然 execution-first，SHAP 强绑不变) |

**关键**：四个 Sprint 的验收条件不松动，本 plan 只**增加**验收项，不替换。原 "accuracy >= 0.95" / "SHAP top_k" / "--execution-focus 默认面" 全部保留。

## 4. 跨 Sprint 约束自检

对每条约束逐条确认本 plan 是否合规：

1. **Execution 作为第一类 artifact** ✓
   - 2.1 进 `execution_artifact.json` + `artifact_ledger`
   - 2.2 进 `mece_recovery_artifact` 扩展字段
   - 2.3 新 kind `factor_tucker_core`
   - 2.4 进 `execution_tree_trace.json`
   - 2.5 进 `MeceRecoveryReport.segments`

2. **物理学层禁止孤立 demo (trait + state lineage + tests + compare surface + hard gate 五件齐全)** ✓
   - 2.1 频域 overlay 五件齐全
   - 2.2/2.4 非 physics 项不适用此约束，但同样带 artifact + test + gate

3. **不允许"只加模型不加 comparability"** ✓
   - 每节都明确列了 artifact / ledger / regression fixture hash

4. **main.rs 永远只做 thin orchestration** ✓
   - 所有新模块在 `src/math/` / `src/domain/execution/` / `src/application/execution/` / `src/application/belief/` / `src/factor_lab/` / `src/application/orchestration/`
   - main.rs 零新增

5. **execution gate 先行** ✓
   - 2.1 gate in Sprint 2（与主计划 §2.3 OU overlay 同期）
   - 2.2 gate in Sprint 3
   - 2.4 gate in Sprint 4

## 5. 执行顺序 & 预估

| 阶段 | 内容 | 预估 | 依赖 |
|------|------|------|------|
| S2-a | 2.1 spectral overlay 实现 + fixture | 4-5 天 | Sprint 2 physics 骨架 (已存在) |
| S2-b | 2.1 artifact/ledger/reflection 串接 | 2-3 天 | S2-a |
| S3-a | 2.2 softshrink + MECE loop 接入 | 3-4 天 | Sprint 3 recovery 骨架 |
| S3-b | 2.5 rollout 分段 + regression | 3 天 | S3-a |
| S3-c | 2.3 tucker 离线实现 + 单 symbol artifact | 5-6 天 | S3-a |
| S4-a | 2.4 axial_pool + execution_tree 接入 | 3-4 天 | S3-c (loadings 复用) |
| S4-b | 2.4 SHAP 交叉 + reflection 绑定 | 2-3 天 | S4-a |

合计 ≈ 22-28 天，**穿插**进主计划 Sprint 2-4，不单列 Sprint 5。

## 6. 成功判据

完成本 plan 后，ict-engine 应同时满足：

- `execution_artifact.json` 100% 覆盖 `spectral.*` 四字段 (2.1)
- MECE recovery 通过 sparsity + execution histogram + rollout segments 三重 hard gate (2.2 + 2.5)
- 至少 1 份真实 symbol 的 `factor_tucker_core.json` 入 ledger (2.3)
- `execution_tree_trace.json` 带 `axial_attention_trace` + 对应 branch 决定 (2.4)
- 所有新 fixture hash 锁定，CI 上有 regression 阻断
- `--execution-focus` 默认面终端段落在 "能否做 / 方向如何" 之外，**多出一句**"为什么 execution 这个结论可信"，证据可引用 spectral phase / tucker loading / axial trace / sparsity ratio

## 7. 明确拒绝的路径

- 引入 PyTorch / JAX / onnx 依赖跑 FNO/TFNO/AViT — 破坏 Rust 单一执行面
- 把 the_well 数据集本身作为 ict-engine 训练语料 — 物理场数据与金融 tick 不同分布，迁移学习无依据
- 用 FNO 直接做价格预测 — 违反 execution-first（prediction 降级原则）
- 新加独立子命令如 `physics-sim-demo` — 违反跨 Sprint 约束 §2
- 把 tucker core 写进 main.rs 的决策路径 — 违反约束 §4

## 8. 参考

- 本 plan 所有公式直接来源: `docs/the_well-physics-sim-ict-insights.md` §1-4
- 约束与目标来源: `docs/plans/execution-operating-system-plan.md`
- 原 paper: Ohana et al. NeurIPS 2024, arXiv:2412.00568
