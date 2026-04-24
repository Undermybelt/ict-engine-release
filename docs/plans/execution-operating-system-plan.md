# ict-engine：Execution Operating System — 完整 4 Sprint 计划

> 合并源：`execution-first-4-sprint-plan.md` + `execution-paper-notes-and-plan-update.md` + `execution-first-4-sprint-next-steps.md`
> 核心原则：Execution Operating Surface + Prediction as Input（非 Prediction System + Execution Features）

## 指导论文

| 论文 | 直链 | 核心主张 |
|------|------|---------|
| Who Profits from Prediction? | SSRN 6191618 | 赚钱核心是 Execution，不是 Information |
| HMM Ensemble Voting | AIMS Press PDF | 多模型 HMM 混合投票优于单 HMM |
| Ising 相变金融模型 | arXiv 2504.19050 | Ising model 建模 herd / phase transition |
| Bayesian Decision Tree | arXiv 2309.15312 | 执行树升级为贝叶斯决策 |

## 全局优先级（不可违反）

```
execution feasibility > execution attribution > prediction value > regime explanation
```

## 现有组件清单

HMM + BBN/loopybayesnet + CatBoost/XGBoost voting + PD Array 检测 + MECE regimes + artifact ledger + reflection_bundle + hard gate

## 物理学第一性原理注入点

| 模型 | 金融含义 | 对 execution 的作用 | 对 regime 的作用（副作用） |
|------|---------|-------------------|------------------------|
| Ornstein-Uhlenbeck | 蹦床 / 均值回复 | 过伸后能否以可接受代价回归成交 | 弹性 regime 识别 |
| Ising Model | 相变 / herding | 拥挤/同向 herd 是否让执行边际恶化 | 相变 regime 检测 |
| Pythagorean Geometry | 趋势线过伸 | 当前离可做区有多远 | 过伸 regime 识别 |

**约束**：物理学层第一优先服务 execution feasibility；regime 提升是副作用，不是主目标。所有 OU/Ising/Pythagorean 输出必须可落入 execution artifact lineage。

---

## Step 0：Phase A 收尾（≈1 周，Sprint 1 前置）

> 只有 Phase A 稳定，Sprint 2 的物理层才不会把 repo 拉回 prediction-first。

### 0.1 main.rs 执行特征计算抽离

**现状**：`main.rs` 约 L15589–15620 内联计算 `aggression_bias` / `completion_pressure` / `liquidity_absorption_bias` / `prediction_score`。

**做法**：
- 新增 `src/application/execution/inputs.rs`
- `ExecutionInputSources` 聚合上游对象引用
- `derive_execution_inputs() -> ExecutionInputSnapshot`
- `main.rs` 只保留 `let snap = derive_execution_inputs(&sources);`

**验收**：`grep execution 计算字段 main.rs` 只剩消费路径。

### 0.2 surfaces.rs → reflection.rs + why_* 字段

**现状**：`surfaces.rs` 命名不一致，缺 `why_execution_dominates` / `why_prediction_is_demoted`。

**做法**：
- 重命名 → `reflection.rs`
- `ReflectionBundle` 加两条 `Option<String>`
- `build_explanation()` 根据 `execution_edge_share vs prediction_edge_share` 自动生成

**验收**：execution-dominant 场景 100% 附带 `why_*`。

### 0.3 execution_readiness 门限集中

**现状**：`0.65` / `0.45` 在 4 处硬编码。

**做法**：
- `src/domain/execution/mod.rs` 新增 `EXECUTION_GATE_READY: f64 = 0.65` / `EXECUTION_GATE_OBSERVE: f64 = 0.45`
- `classify_execution_gate(readiness) -> &'static str`

**验收**：`rg '0\.65|0\.45' src/domain/execution src/application/execution` 只剩常量定义处。

### 0.4 artifact.rs 接真实 OU 拟合

**现状**：`estimate_ou_execution_metrics` 已实现但未被消费。

**做法**：
- `ExecutionArtifactBuildContext` 传入 `prices` / `timestamps` 切片
- 优先走真实 OU 拟合，失败回退 `ltf_features.ou_*`

**验收**：真实 analyze 一次，`execution_artifact.json` 的 `ou_metrics` 来自真实拟合。

### 0.5 workflow.rs 去启发式

**现状**：`apply_*_run_execution_fields` 用 `0.58`/`0.62`/`0.54` 等硬编码推导。

**做法**：
- `RunRecord` 扩展 `execution_artifact_id` / `execution_edge_share` / `execution_readiness` / `execution_gate_status`
- 改为直接读 record 字段

**验收**：同一 run 的 `workflow_snapshot` 与 `execution_artifact.json` 四字段字节级一致。

---

## Sprint 1（2-3 周）：Execution-First 基础强化

### 目标

把 Execution Score 显式加入 ICTFeatures 和 EntryExecution struct；在 voting layer 后新增 ExecutionArtifact；用 OU 过程计算 `overextension_distance` 和 `reversion_speed`。

### 具体怎么做

#### 1.1 新增 execution 领域模块

```
src/domain/execution/mod.rs       — 领域 trait + 常量
src/domain/execution/types.rs     — ExecutionFeatures / ExecutionArtifact
src/domain/execution/ou.rs        — OU 参数估计（已有）
src/domain/execution/score.rs     — Execution Score 计算
src/application/execution/mod.rs  — 应用层编排
src/application/execution/artifact.rs — artifact 构建 + 持久化（已有）
src/application/execution/reflection.rs — reflection 集成（已有，重命名中）
```

#### 1.2 扩展 ICTFeatures / EntryExecution

```rust
// src/domain/execution/types.rs

pub struct ExecutionFeatures {
    pub execution_score: f64,           // 综合执行评分
    pub aggression_bias: f64,           // 执行激进度
    pub completion_pressure: f64,       // 完成压力
    pub liquidity_absorption_bias: f64, // 流动性吸收偏向
    pub overextension_distance: Option<f64>, // OU 过伸距离
    pub reversion_speed: Option<f64>,   // OU 回复速度
    pub evidence_quality: f64,          // 证据质量
}

pub struct EntryExecution {
    pub regime_gate_passed: bool,
    pub execution_features: ExecutionFeatures,
    pub execution_note: String,
}
```

接入点：`config.rs` / `pipeline_types.rs` / `trade_plan.rs` / `analyze_output.rs`

#### 1.3 ExecutionArtifact（voting layer 后）

```rust
pub struct ExecutionArtifact {
    pub artifact_id: String,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub execution_score: f64,
    pub prediction_score: f64,
    pub execution_edge_share: f64,  // "Execution 73%"
    pub prediction_edge_share: f64, // "Prediction 27%"
    pub overextension_distance: Option<f64>,
    pub reversion_speed: Option<f64>,
    pub hard_gate_status: String,
    pub provenance: RunProvenance,
}
```

集成点：CatBoost/XGBoost voting 之后 → reflection_bundle 生成之前 → workflow_snapshot 写盘之前

持久化：`state/<SYMBOL>/execution_artifact.json` + `artifact_ledger.json` (kind=`execution_artifact`)

#### 1.4 OU 过程注入 execution 弹性度量

```rust
// src/domain/execution/ou.rs

pub struct OuExecutionMetrics {
    pub mean_level: f64,
    pub theta: f64,              // 回复强度
    pub sigma: f64,              // 波动率
    pub overextension_distance: f64,
    pub reversion_speed: f64,
}

pub fn estimate_ou_execution_metrics(
    prices: &[f64],
    timestamps: &[DateTime<Utc>],
) -> Option<OuExecutionMetrics>;
```

约束：不取代 HMM，只作为 execution tree / artifact 的额外特征。hard gate 可设：overextension 过大 + reversion_speed 过弱 → 降级 execution readiness。

#### 1.5 reflection_bundle 强制拆分 execution vs prediction

```rust
// ReflectionBundle 新增字段
pub execution_edge_share: f64,      // "Execution edge 73%"
pub prediction_edge_share: f64,     // "Prediction edge 27%"
pub execution_readiness: f64,
pub execution_summary: String,      // 为什么 execution 占主导
pub prediction_summary: String,     // 为什么 prediction 被降级
pub why_execution_dominates: Option<String>,
pub why_prediction_is_demoted: Option<String>,
```

### 能得到什么明确优势

| 维度 | 提升 |
|------|------|
| comparability | execution quality 首次成为跨 run 可比较字段 |
| provenance | 每次 run 可回答"edge 来自 execution 还是 prediction" |
| reflection_bundle 可审计性 | 从叙述解释变成结构化 execution attribution |
| agent safety | human-next triage 不再被高方向概率误导；execution weak → hard gate 直接阻断 |
| contributor workflow | 改特征/模型/gate 必须面对 execution artifact 回归测试 |
| 量化目标 | reflection_bundle 100% 输出 execution/prediction edge split；artifact_ledger 可回放比较 |

---

## Sprint 2（2-3 周）：物理学模型注入

### 目标

在 PDArray 检测里新增 Pythagorean 投影 + Ising mean-field 层；在 HMM posterior 后增加 OU filter，作为执行树的额外特征。

**第一优先**：物理学层服务 execution feasibility。regime 提升是副作用。

### 具体怎么做

#### 2.1 Pythagorean 几何距离

```
src/math/geometry.rs                — 纯数学：Point2, segment_length, orthogonal_distance
src/ict/pythagorean_extension.rs    — domain 包装
```

```rust
pub struct PythagoreanExtensionMetrics {
    pub trendline_distance: f64,       // 沿趋势线距离
    pub orthogonal_extension: f64,     // 正交偏移
    pub normalized_overstretch: f64,   // 归一化过伸
}

pub fn measure_pythagorean_extension(
    anchor_a: (f64, f64),
    anchor_b: (f64, f64),
    current: (f64, f64),
) -> PythagoreanExtensionMetrics;
```

集成：
- `PDArrayType::detect` → `normalized_overstretch` 入 trace
- `FrameFeatures` → `pythagorean_overstretch: Option<f64>`
- `factor_lab/factor_definition.rs` → 新 execution-sensitive factor `overextension_bias`

验收：3 case 测试（对齐 / 正交 / 过伸边界）；`FrameFeatures` 序列化含新字段。

#### 2.2 Ising mean-field 层

```
src/domain/regime/ising.rs              — IsingState + estimate_ising_state
src/application/belief/ising_overlay.rs — 叠加到 HMM posterior
```

```rust
pub struct IsingState {
    pub magnetization: f64,          // 同向信号强度
    pub coupling_strength: f64,      // 参与者耦合度
    pub phase_transition_risk: f64,  // 相变风险
    pub herding_bias: f64,           // herd 偏向
}

pub fn estimate_ising_state(
    aligned_signals: &[f64],
    participation_weights: &[f64],
) -> Option<IsingState>;
```

约束：
- 不替换 BBN posterior，在 HMM posterior 后叠加
- 仅在 `execution_readiness >= EXECUTION_GATE_OBSERVE` 时允许影响 execution tree branch
- 否则仅作为观察面

验收：`herd_high_phase_risk` / `herd_low_phase_calm` 两条 fixture 测试。

#### 2.3 OU overlay（服务 execution feasibility）

```
src/application/belief/ou_overlay.rs
```

```rust
pub fn apply_ou_overlay(pipeline_state, ou_metrics);
```

关键约束：第一优先 execution feasibility。regime 提升仅在 `execution_readiness >= EXECUTION_GATE_READY` 时允许影响 regime posterior。

验收：单独 toggle ou_overlay 不改 regime posterior；联合通过 execution gate 时才允许影响。

#### 2.4 ExecutionPhysicsOverlay 汇总

```
src/application/execution/physics.rs
```

```rust
pub struct ExecutionPhysicsOverlay {
    pub ou: Option<OuExecutionMetrics>,
    pub ising: Option<IsingState>,
    pub pythagorean: Option<PythagoreanExtensionMetrics>,
}

pub fn build_execution_physics_overlay(
    candles: &[Candle],
    frame_features: &FrameFeatures,
) -> ExecutionPhysicsOverlay;
```

集成点：`pipeline_builder.rs` / `policy_engine.rs` / `trace_builder.rs` 各消费一次，不再重算。

#### 2.5 factor research 记录 physics feature lineage

扩展 `objective_surfaces` / `artifact_ledger` / `reflection_bundle`：
- `ou_overextension_distance`
- `ou_reversion_speed`
- `ising_phase_transition_risk`
- `pythagorean_overstretch`

### 能得到什么明确优势

| 维度 | 提升 |
|------|------|
| regime 检测 | 不再只有统计概率，有物理解释面：弹性、相变、过伸 |
| factor ranking | 可区分"方向对但执行差"与"方向一般但 execution 可做" |
| forward-test 稳定性 | execution physics 比方向信号更平滑、更可迁移 |
| hard gate | 不是黑箱 veto，而是"过伸 + herd risk + mean reversion elasticity"三重证据 |
| artifact ledger | 每个 regime/factor 决策可追到哪种 physics overlay 推动 |
| 量化目标 | `factor-pipeline-debug` 加 physics trace；objective_surfaces 至少含 3 个 physics feature score |

---

## Sprint 3（2-3 周）：MECE Regimes + HMM 完美恢复闭环

### 目标

实现 `manual_mece_labeler` + 因子搜索循环，直到 HMM Viterbi 路径与 MECE 标签的恢复准确率 > 95%；锁定最优因子集存入 artifact ledger。

**修正**：恢复准确率不是唯一优化目标。必须并行记录 execution consistency。

### 具体怎么做

#### 3.1 manual_mece_labeler

```
src/domain/regime/mece_labeler.rs
src/application/regime/recovery.rs
scripts/mece_label_bootstrap.py（仅离线辅助，非真相源）
```

```rust
pub enum MeceRegimeLabel {
    Expansion,
    Manipulation,
    Reversion,
    Compression,
    TrendContinuation,
    Unknown,
}

pub fn manual_mece_labeler(
    candles: &[Candle],
    frame_features: &FrameFeatures,
) -> Vec<MeceRegimeLabel>;
```

标签本身 versioned → 写入 artifact_ledger → 进入 workflow_snapshot comparability surface。

#### 3.2 因子搜索 → MECE recovery loop

```rust
pub struct MeceRecoveryReport {
    pub accuracy: f64,
    pub macro_f1: f64,
    pub confusion_matrix: BTreeMap<String, BTreeMap<String, usize>>,
    pub best_factor_set: Vec<String>,
    pub execution_validity_histogram: BTreeMap<String, usize>, // execution 分布
    pub provenance: RunProvenance,
}

pub fn search_factors_for_mece_recovery(
    candles: &[Candle],
    labels: &[MeceRegimeLabel],
    registry: &FactorRegistry,
) -> Result<MeceRecoveryReport>;
```

**执行一致性双约束**：`execution_validity_histogram` 至少覆盖 `execution_ready` / `execution_observe_only` / `execution_blocked` 三类。严禁"regime 恢复 95% 但 execution 持续 block"的假成功。

集成：`factor_lab/research.rs` / `state/research_runs.json` / `factor_autoresearch_live/final`

#### 3.3 mece_recovery_artifact

```rust
pub struct MeceRecoveryArtifact {
    pub artifact_id: String,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub accuracy: f64,
    pub macro_f1: f64,
    pub selected_factors: Vec<String>,
    pub hmm_viterbi_hash: String,
    pub label_hash: String,
    pub execution_validity_summary: String,
    pub provenance: RunProvenance,
}
```

hard_gate 消费 `accuracy >= 0.95`；低于阈值禁止 promote。

#### 3.4 cargo test 级恢复验证

```rust
// tests/mece_recovery.rs
#[test]
fn hmm_viterbi_mece_recovery_stays_above_threshold() {
    let report = run_fixture_mece_recovery();
    assert!(report.accuracy >= 0.95);
}

// tests/hmm_recovery_regression.rs
#[test]
fn viterbi_output_hash_unchanged_without_declared_change() {
    let hash = compute_viterbi_fixture_hash();
    assert_eq!(hash, EXPECTED_HASH);
}
```

CI 对 MECE recovery 回归有硬阻断。

### 能得到什么明确优势

| 维度 | 提升 |
|------|------|
| HMM 状态 | 从"看起来合理"变成有显式恢复率约束的 state machine |
| comparability | 不同 run 按 label hash / viterbi hash 精确比较 |
| provenance | regime 改动可追到标签版、因子集版、模型版 |
| reflection_bundle | 输出"regime 解释是否落在已验证的 MECE 恢复区间内" |
| agent safety | 恢复率跌破阈值 → hard gate 禁止 promotion |
| contributor workflow | PR 不只过编译，还要过 regime recovery 回归 |
| 量化目标 | Viterbi vs MECE 恢复 > 95%；factor-autoresearch 从"盲调"升级为"围绕恢复率的可验证搜索" |

---

## Sprint 4（2-3 周）：Execution Tree + Full Workflow 产品化

### 目标

Execution Score、OU/Ising/Pythagorean 特征全部喂给 CatBoost/XGBoost voting；SHAP 解释强制绑定到 reflection_bundle；`--execution-focus` 升为默认面。

### 具体怎么做

#### 4.1 ExecutionTreeScorer

```
src/application/orchestration/execution_tree.rs
src/application/reporting/execution_focus.rs
```

```rust
pub struct ExecutionTreeInput<'a> {
    pub execution_features: &'a ExecutionFeatures,
    pub physics_overlay: &'a ExecutionPhysicsOverlay,
    pub hmm_posterior: &'a RegimeProbs,
    pub mece_recovery_confidence: Option<f64>,
    pub prediction_vote_score: f64,
}

pub struct ExecutionTreeOutput {
    pub execution_score: f64,
    pub branch: String,          // "fill_viable" / "wait_for_reversion" / "block_crowded"
    pub execution_bias: String,  // "aggressive" / "passive" / "skip"
    pub gate_status: String,     // "ready" / "observe" / "blocked"
}

pub trait ExecutionTreeScorer {
    fn score(&self, input: &ExecutionTreeInput<'_>) -> Result<ExecutionTreeOutput>;
}
```

`DefaultExecutionTreeScorer`：Bayesian Decision Tree 风格，记录 branch probability / posterior uncertainty / split reason lineage。

产物：`state/<SYMBOL>/execution_tree_trace.json` + `execution_tree_artifact` kind

集成点：CatBoost/XGBoost voting 之后 → reflection_bundle 之前 → workflow_snapshot 写盘之前

#### 4.2 SHAP → reflection_bundle 强制绑定

```rust
pub struct ExecutionShapAttribution {
    pub feature: String,
    pub contribution: f64,
    pub feature_value: String,
}
```

`reflection_bundle` 必带 `execution_shap_top_k`（默认 k=5）。

CI 回归禁止 reflection_bundle 在 execution-tree 路径下丢失该字段。

#### 4.3 --execution-focus 升为默认面

CLI 注册到 `analyze` / `workflow-status` 子命令。

语义：execution triage 是默认可读面；非 execution 面退为 secondary surface。

无 flag 时 `analyze` 的终端最终段落先回答"能否做"，再回答"方向如何"。

#### 4.4 Hard gate 升级为 execution-first

规则：
- prediction strong + execution weak → **block**
- prediction medium + execution strong → **observe / tune**，不拒绝

验收：`tests/hard_gate_execution_first.rs` 覆盖 2×2 全组合并有明确 `decision_hint`。

### 能得到什么明确优势

| 维度 | 提升 |
|------|------|
| 项目定位 | 从"概率 + 预测引擎"彻底升级为 Execution Operating System |
| 护城河 | stateful execution lineage / artifacts / hard gates / recovery loops，AI wrapper 不可复制 |
| stateful CLI | 用户直接问"这单能不能做"，不只看到涨跌偏向 |
| hard gate | 从单纯风险阻断变成 execution readiness governance |
| artifact ledger | execution tree / SHAP / physics overlay / MECE recovery 全部形成可追踪证据链 |
| contributor workflow | 新增 feature 不只提高 AUC，要通过全链路验证 |
| 量化目标 | --execution-focus 是默认面；reflection_bundle 100% 附带 execution attribution；promotion gate 以 execution score 为第一条件 |

---

## 跨 Sprint 固定约束

1. **Execution 作为第一类 artifact**：所有 execution 对象必须进入 state/* / artifact_ledger.json / workflow_snapshot.json / reflection_bundle
2. **物理学层禁止孤立 demo**：trait + state lineage + tests + compare surface + hard gate 作用点五件齐全
3. **不允许"只加模型不加 comparability"**：每个新层必须回答——run 怎么比 / provenance 怎么写 / artifact 怎么存 / regression 怎么测
4. **main.rs 永久只做 command wiring + thin orchestration**：逻辑进 domain/execution/* / application/execution/* / domain/regime/* / application/belief/* / application/orchestration/*
5. **execution gate 先行**：Sprint 1 即接入 hard gate，不等 Sprint 4

---

## 验收总览

| 阶段 | 验收条件 |
|------|---------|
| Step 0 (Phase A 收尾) | main.rs 无 execution 计算；ReflectionBundle 含 why_* 字段；artifact_ledger 100% 有 execution_artifact；workflow_snapshot 与 artifact 字节级一致 |
| Sprint 1 | reflection_bundle 100% 输出 execution/prediction edge split；OU metrics 真实拟合；hard gate 消费 execution_readiness |
| Sprint 2 | FrameFeatures 含 ou_* / ising_* / pythagorean_* 字段；无物理层处于孤立 demo；artifact lineage 可追四个 physics 字段 |
| Sprint 3 | MECE recovery accuracy >= 0.95 作为 hard gate；recovery + execution consistency 双约束写入 artifact |
| Sprint 4 | --execution-focus 是默认面；reflection_bundle 必带 SHAP top-k；hard gate 以 execution_readiness 为首要条件 |

---

## 如果我是技术 co-founder

关键不在"多加几个模型"，在四件事同时成立：

**1. Execution 变成一等公民**
不再围绕"谁预测更准"内卷。围绕"何时可做、为何可做、做错时证据在哪"建立闭环。直接对齐论文结论：赚钱核心在 execution，不在 information。

**2. 物理学不是装饰，是 execution feasibility 的硬约束**
OU 告诉你"过伸了还能不能以可接受代价回踩成交"；Ising 告诉你"herd 拥挤是否让执行边际恶化"；Pythagorean 告诉你"离可做区多远"。这三者服务 execution，不服务 regime 叙事。

**3. 可验证性做到学术级**
MECE recovery 回归测试、Viterbi hash 锁定、execution artifact lineage、SHAP attribution 全部进 CI。任何改动能被审计、回放、比较。contributor PR 不只过编译，过全链路。

**4. 3 个月后，ict-engine 的护城河是 stateful execution lineage**
任何 AI wrapper、prompt copilot、GPT 套壳都复制不了的东西：
- artifact ledger 里的 execution 版本链
- hard gate 的 execution-first 治理
- MECE recovery 的可验证回归
- reflection_bundle 的结构化 attribution

这才是 stateful research OS，不是又一个 trading bot。

---

## 模块目录（最终版）

```
src/domain/execution/
  mod.rs          — trait + 常量 + classify_execution_gate
  types.rs        — ExecutionFeatures / ExecutionArtifact / OuExecutionMetrics
  ou.rs           — OU 参数估计
  score.rs        — Execution Score 计算

src/application/execution/
  mod.rs          — 编排
  artifact.rs     — artifact 构建 + 持久化
  reflection.rs   — reflection_bundle 集成 + why_* 生成
  physics.rs      — ExecutionPhysicsOverlay 汇总
  inputs.rs       — 从 main.rs 抽离的输入计算

src/math/
  geometry.rs     — Pythagorean 纯数学

src/ict/
  pythagorean_extension.rs — domain 包装

src/domain/regime/
  ising.rs        — IsingState + estimate_ising_state
  mece_labeler.rs — MECE 手工标注

src/application/belief/
  ising_overlay.rs — Ising → HMM posterior 叠加
  ou_overlay.rs   — OU → execution feasibility 叠加

src/application/regime/
  recovery.rs     — MECE recovery 搜索循环

src/application/orchestration/
  execution_tree.rs — ExecutionTreeScorer trait + impl

src/application/reporting/
  execution_focus.rs — --execution-focus 输出格式

tests/
  mece_recovery.rs
  hmm_recovery_regression.rs
  hard_gate_execution_first.rs

scripts/
  mece_label_bootstrap.py — 离线辅助，非真相源
```
