# ict-engine：Execution-First 4 Sprint 下一步完整计划

## 定位
- 本文件是对以下两份文档的**可执行派生**：
  - `support/docs/execution-first-4-sprint-plan.md`（原始 4 Sprint 蓝图）
  - `support/docs/execution-paper-notes-and-plan-update.md`（论文读后对优先级的修正：Phase A→B→C→D）
- 它不重写原计划，只回答三件事：
  - 当下两份文档与代码之间的差距在哪里？
  - 下一步按什么顺序推进？
  - 每一步的验收条件怎么写？
- 目标不变：把 `ict-engine` 从"有状态的预测研究系统"推进到 `stateful execution research OS`。
- 优先级排序（沿用更新版）：`execution feasibility` → `execution attribution` → `prediction value` → `regime explanation`。

---

## 当前代码面快照（2026-04-20）

### 已落地（Phase A 基础骨架）
- `src/domain/execution/{mod,types,ou,score}.rs` 全在。
- `ExecutionFeatures` / `ExecutionArtifact` / `OuExecutionMetrics` 结构齐备、`Serialize`/`Deserialize` 就绪。
- `src/domain/execution/ou.rs` 新增 `estimate_ou_execution_metrics(prices, timestamps) -> Option<OuExecutionMetrics>`：AR(1) → OU 真实拟合，含 4 条单元测试（短样本/常数/趋势/AR(1) 恢复）。
- `src/application/execution/{mod,artifact,persistence,surfaces,workflow}.rs` 已接上：
  - `build_execution_artifact` 能产出 artifact
  - `persist_execution_artifact` 写 `state/<SYMBOL>/execution_artifact.json` 与 `artifact_ledger.json`（kind=`execution_artifact`）
  - `apply_execution_artifact_to_reflection_bundle` 把三条 execution 字段挂到 reflection
  - `apply_*_run_execution_fields` 在 analyze/research/backtest/update 四阶段回灌 `WorkflowPhaseSnapshot`
- `ReflectionBundle` 已暴露：`execution_edge_share` / `prediction_edge_share` / `execution_readiness` / `execution_summary` / `prediction_summary`。
- `workflow_status --agent` 能读到 `execution_readiness` 与 `execution_gate`。
- `main.rs` 已调用 `build_execution_artifact(...)` 并挂到 AnalyzeReport。

### 尚未闭环（Phase A 收尾清单）
1. `main.rs` 约 `15589–15620` 行仍在本地计算 `aggression_bias` / `completion_pressure` / `liquidity_absorption_bias` / `prediction_score` 共 ~30 行，违反"`main.rs` 只做 command wiring"约束。
2. `application/execution/surfaces.rs` 实际承担了原计划点名的 `reflection.rs` 职责，命名与 4 Sprint 蓝图不一致；且只挂了 `execution_summary` / `prediction_summary`，**缺** `why_execution_dominates` / `why_prediction_is_demoted` 两条显式解释字段。
3. `execution_readiness` 门限 `0.65` / `0.45` 在 `artifact.rs`、`workflow.rs`（两处）、`persistence.rs` 共 4 处硬编码重复，无单点配置。
4. 新 `estimate_ou_execution_metrics` 已存在但尚未被 `artifact.rs` 消费；`build_execution_artifact` 仍只接收 `ltf_features.ou_*` 预算值并走 `build_ou_execution_metrics` 打包。
5. `workflow.rs` 中 `apply_research/backtest/update_run_execution_fields` 使用硬编码启发式（`0.58`/`0.62`/`0.54`/`0.30` …）推导 `execution_edge_share` / `execution_readiness`，未走真实 `ExecutionArtifact` 字段。

### 未落地（Sprint 2/3/4 模块骨架）
- Sprint 2 物理层
  - `src/math/geometry.rs`：**缺**（仅有 `src/math.rs` 单文件）
  - `src/ict/pythagorean_extension.rs`：**缺**
  - `src/domain/regime/ising.rs`：**缺**
  - `src/application/belief/ou_overlay.rs`：**缺**
  - `src/application/belief/ising_overlay.rs`：**缺**
  - `src/application/execution/physics.rs`（`ExecutionPhysicsOverlay` 汇总）：**缺**
- Sprint 3 MECE
  - `src/domain/regime/mece_labeler.rs`：**缺**
  - `src/application/regime/recovery.rs`：**缺**
  - `support/scripts/mece_label_bootstrap.py`：**缺**（仅作离线辅助）
  - `tests/mece_recovery.rs` / `tests/hmm_recovery_regression.rs`：**缺**
- Sprint 4 Execution Tree
  - `src/application/orchestration/execution_tree.rs`：**缺**
  - `src/application/reporting/execution_focus.rs`：**缺**
  - CLI flag `--execution-focus`：**未注册**
  - `ExecutionShapAttribution` / `execution_tree_artifact`：**缺**

---

## 执行顺序

### Step 0：Phase A 收尾（≈ 1 周）
> 所有 Step 1 之前的前置。只有 Phase A 稳定，Sprint 2 的物理层才不会把 repo 拉回 prediction-first。

#### 0.1 把 `main.rs` 的 execution 输入计算抽到模块
- 新增 `src/application/execution/inputs.rs`
  - `pub struct ExecutionInputSources<'a> { ... }`：聚合 `pre_bayes_evidence_filter` / `selected_entry_quality_distribution` / `pre_bayes_entry_quality_bridge` / `decision` 等上游对象引用。
  - `pub struct ExecutionInputSnapshot { aggression_bias, completion_pressure, liquidity_absorption_bias, evidence_quality, prediction_score }`
  - `pub fn derive_execution_inputs(src: &ExecutionInputSources) -> ExecutionInputSnapshot`
- `main.rs`:
  - 删除 `15589–15620` 行的内联计算
  - 仅保留 `let snap = derive_execution_inputs(&sources); let artifact = build_execution_artifact_from_snapshot(symbol, &snap, ...);`
- 验收：
  - `grep -n 'aggression_bias\|completion_pressure\|liquidity_absorption_bias' src/main.rs` 只剩 artifact 消费路径，不再出现计算。
  - `cargo check` 通过；已有 analyze 集成测试输出的 artifact 字段字节级不变。

#### 0.2 `surfaces.rs` → `reflection.rs` 命名对齐 + 补 `why_*` 字段
- 重命名 `src/application/execution/surfaces.rs` → `src/application/execution/reflection.rs`；更新 `mod.rs` 的 `pub mod` 与 `pub use`。
- `ReflectionBundle` 加两条 `Option<String>`：
  - `why_execution_dominates`
  - `why_prediction_is_demoted`
- 在 `reflection.rs` 新增：
  - `fn build_execution_explanation(artifact: &ExecutionArtifact) -> (Option<String>, Option<String>)`
  - 当 `execution_edge_share > prediction_edge_share` 时：
    - `why_execution_dominates`: 依据 `execution_score` / `evidence_quality` / `overextension_distance` / `reversion_speed` 四条来源拼 1-2 句叙述。
    - `why_prediction_is_demoted`: 依据 `prediction_score` 与 `prediction_edge_share` 的缺口来源拼 1 句。
  - 反之不填，两字段保持 `None`。
- 验收：
  - `reflection_bundle.json` 在 execution-dominant 场景下 100% 附带 `why_*`。
  - 旧 `apply_execution_artifact_to_reflection_bundle` 测试不破；新增 1 条 execution-dominant + 1 条 prediction-dominant 的回归测试。

#### 0.3 `execution_readiness` 门限集中
- 在 `src/domain/execution/mod.rs`（或新 `gates.rs`）新增：
  - `pub const EXECUTION_GATE_READY: f64 = 0.65;`
  - `pub const EXECUTION_GATE_OBSERVE: f64 = 0.45;`
  - `pub fn classify_execution_gate(readiness: f64) -> &'static str`（返回 `execution_ready` / `execution_observe_only` / `execution_blocked`）
- `artifact.rs`、`persistence.rs`、`workflow.rs` 全部换用该常量与函数。
- 验收：`rg -n '0\.65|0\.45' src/domain/execution src/application/execution` 只剩常量定义处。

#### 0.4 把 `artifact.rs` 切到真实 OU 拟合
- `application/execution/artifact.rs` 新增：
  - `pub struct ExecutionArtifactBuildContext<'a> { prices: Option<&'a [f64]>, timestamps: Option<&'a [DateTime<Utc>]>, fallback_ou: Option<&'a LtfOuFallback> }`
  - `build_execution_artifact(...)` 内部优先走 `estimate_ou_execution_metrics`；失败（返回 `None`）才回退到 `build_ou_execution_metrics(ltf_features.ou_*)`。
- `main.rs` 调用处传入 `candles` / `timestamps` 切片。
- 验收：
  - 真实 analyze 一次，`execution_artifact.json` 的 `ou_metrics` 字段来自真实拟合。
  - 输入过短或非平稳时，字段回退到 `ltf_features.ou_*` 路径，不产生 `None`/空值。
  - 新增回归测试覆盖"真实拟合"与"回退"两条分支。

#### 0.5 `workflow.rs` 去启发式
- `AnalyzeRunRecord` / `BacktestRunRecord` / `ResearchRunRecord` / `UpdateRunRecord` 扩展（若未有）：
  - `execution_artifact_id: Option<String>`
  - `execution_edge_share: Option<f64>`
  - `execution_readiness: Option<f64>`
  - `execution_gate_status: Option<String>`
- `apply_*_run_execution_fields` 改为直接读 record 字段，**删除** `0.58`/`0.62`/`0.54` 等硬编码权重。
- 验收：同一次 run 的 `workflow_snapshot` 与对应 `execution_artifact.json` 四字段字节级一致。

---

### Step 1：Sprint 2 物理层骨架（≈ 2–3 周）
> 前置原则：`OU` / `Ising` / `Pythagorean` 三层**首先服务 execution feasibility**，regime 解释只能作为 side effect。任何一层都必须同时落到：trait + state lineage + tests + compare surface + hard gate 作用点。

#### 1.1 Pythagorean 几何
- `src/math/geometry.rs`（纯数学，不依赖任何 domain 类型）：
  - `pub struct Point2 { pub x: f64, pub y: f64 }`
  - `pub fn segment_length(a: Point2, b: Point2) -> f64`
  - `pub fn orthogonal_distance(anchor_a: Point2, anchor_b: Point2, point: Point2) -> f64`
  - 单元测试覆盖：对齐、正交、退化（同点）。
- `src/ict/pythagorean_extension.rs`：
  - `PythagoreanExtensionMetrics { trendline_distance, orthogonal_extension, normalized_overstretch }`
  - `fn measure_pythagorean_extension(anchor_a, anchor_b, current) -> PythagoreanExtensionMetrics`
- 集成：
  - `PDArrayType::detect`：将 `normalized_overstretch` 作为 secondary feature 入 trace。
  - `FrameFeatures`：暴露 `pythagorean_overstretch: Option<f64>`。
  - `factor_lab/factor_definition.rs`：新增一条 execution-sensitive factor `overextension_bias`。
- 验收：
  - `cargo test` 覆盖 3 case（对齐 / 正交 / 过伸边界）。
  - `FrameFeatures` 序列化含新字段；artifact ledger `feature_lineage` 能追 `pythagorean_overstretch`。

#### 1.2 Ising mean-field 层
- `src/domain/regime/ising.rs`：
  - `IsingState { magnetization, coupling_strength, phase_transition_risk, herding_bias }`
  - `estimate_ising_state(aligned_signals: &[f64], participation_weights: &[f64]) -> Option<IsingState>`
- `src/application/belief/ising_overlay.rs`：
  - `apply_ising_overlay(...)`：在 HMM posterior 之后叠加，不替换 BBN posterior。
  - 把 `phase_transition_risk` 写入 `BeliefAttribution`。
- 约束：`ising_overlay` 只在 `execution_readiness >= EXECUTION_GATE_OBSERVE` 时允许影响 execution tree branch，否则仅作为观察面。
- 验收：
  - 两条 fixture 测试：`herd_high_phase_risk` / `herd_low_phase_calm` 各一。
  - `belief` pipeline 序列化含 `ising_*` 字段。

#### 1.3 OU overlay（服务 execution feasibility）
- `src/application/belief/ou_overlay.rs`：
  - `apply_ou_overlay(pipeline_state, ou_metrics)`：把 `overextension_distance` / `reversion_speed_per_bar` 暴露到 `pipeline_types`。
- 关键约束：第一优先 execution feasibility；regime 提升仅在 `execution_readiness >= EXECUTION_GATE_READY` 时允许影响 regime posterior。
- 验收：
  - 单独 toggle `ou_overlay` 不改 regime posterior；联合通过 execution gate 时才允许影响。
  - 与 `estimate_ou_execution_metrics` 共享同一条 metrics 路径，无重复实现。

#### 1.4 `ExecutionPhysicsOverlay` 汇总
- `src/application/execution/physics.rs`：
  - `struct ExecutionPhysicsOverlay { ou: Option<OuExecutionMetrics>, ising: Option<IsingState>, pythagorean: Option<PythagoreanExtensionMetrics> }`
  - `fn build_execution_physics_overlay(candles, frame_features) -> ExecutionPhysicsOverlay`
- 集成点：`pipeline_builder.rs`、`policy_engine.rs`、`trace_builder.rs` 各消费一次，不再重算。
- 验收：`artifact_ledger` 的每条 `execution_artifact` 都能追到 `ou_overextension_distance` / `ou_reversion_speed` / `ising_phase_transition_risk` / `pythagorean_overstretch` 四个字段 lineage。

---

### Step 2：Sprint 3 MECE + Recovery 闭环（≈ 2–3 周）

#### 2.1 手工 MECE labeler
- `src/domain/regime/mece_labeler.rs`：
  - `enum MeceRegimeLabel { Expansion, Manipulation, Reversion, Compression, TrendContinuation, Unknown }`
  - `fn manual_mece_labeler(candles, frame_features) -> Vec<MeceRegimeLabel>`
- labels 本身 versioned、写入 `artifact_ledger`、进入 `workflow_snapshot` 的 comparability surface。
- `support/scripts/mece_label_bootstrap.py` 只作离线辅助：**不允许**成为系统真相源。
- 验收：同一 `(candles, frame_features)` 输入下 Rust 版本与 Python 辅助脚本标签一致率 > 99%。

#### 2.2 因子搜索 → MECE recovery loop
- `src/application/regime/recovery.rs`：
  - `fn search_factors_for_mece_recovery(candles, labels, registry) -> Result<MeceRecoveryReport>`
  - `struct MeceRecoveryReport { accuracy, macro_f1, confusion_matrix, best_factor_set, provenance, execution_validity_histogram }`
- 并行记录 execution consistency：`execution_validity_histogram` 的 buckets 至少覆盖 `execution_ready` / `execution_observe_only` / `execution_blocked` 三类。**严禁**出现"regime 恢复 95% 但 execution artifact 持续 block"的假成功。
- 集成：`factor_lab/research.rs`、`state/research_runs.json`、`factor_autoresearch_live/final`。
- 验收：每次 research run 的报告同时含 `accuracy >= 0.95` 与 execution 分布；"恢复高但 execution 全 block" 自动降级为 observe-only。

#### 2.3 `mece_recovery_artifact`
- 新 artifact kind：`mece_recovery_artifact`。
- 字段：`accuracy` / `macro_f1` / `selected_factors` / `hmm_viterbi_hash` / `label_hash` / `provenance`。
- `hard_gate` 消费 `accuracy` 阈值 `0.95`；低于该阈值禁止 promote。
- 验收：有 fixture 能够触发一次 hard_gate 拦截并回放。

#### 2.4 恢复回归测试
- `tests/mece_recovery.rs`：固定 fixture 下 `accuracy >= 0.95`。
- `tests/hmm_recovery_regression.rs`：Viterbi 输出 hash 未经声明变更不得变化。
- 验收：CI 对 MECE recovery 回归有硬阻断；任何因子调整都必须过这两条测试。

---

### Step 3：Sprint 4 Execution Tree + 产品化（≈ 2–3 周）

#### 3.1 `ExecutionTreeScorer`
- `src/application/orchestration/execution_tree.rs`：
  - `trait ExecutionTreeScorer { fn score(&self, input: &ExecutionTreeInput<'_>) -> Result<ExecutionTreeOutput>; }`
  - `ExecutionTreeInput`: `ExecutionFeatures` + `ExecutionPhysicsOverlay` + `RegimeProbs` + `mece_recovery_confidence` + `prediction_vote_score`。
  - `ExecutionTreeOutput`: `execution_score` / `branch` / `execution_bias` / `gate_status`。
  - `DefaultExecutionTreeScorer` impl：Bayesian Decision Tree 风格，记录 branch probability / posterior uncertainty / split reason lineage。
- 产物：`state/<SYMBOL>/execution_tree_trace.json` + `execution_tree_artifact` kind。
- 集成点：CatBoost voting 之后、`reflection_bundle` 之前、`workflow_snapshot` 写盘之前。

#### 3.2 SHAP → `reflection_bundle`
- `struct ExecutionShapAttribution { feature, contribution, feature_value }`
- `reflection_bundle` 必带 `execution_shap_top_k`（默认 k=5）。
- 验收：CI 回归禁止 `reflection_bundle` 在 execution-tree 路径下丢失该字段。

#### 3.3 `--execution-focus` flag
- 注册到 `analyze` / `workflow-status` 子命令。
- 不作为可选开关：**默认就以 execution triage 为主面**，非 execution 面降为 secondary surface。
- 验收：
  - 无 flag 时 `analyze` 的终端最终段落先回答"能否做"，再回答"方向如何"。
  - `agent-first-runbook.md` 同步改写（文档任务）。

#### 3.4 Hard gate 升级为 execution-first
- 规则：
  - prediction strong + execution weak → **block**。
  - prediction medium + execution strong → **observe / tune**，不拒绝。
- 验收：`tests/hard_gate_execution_first.rs` 覆盖 2×2 全组合并有明确 decision_hint。

---

## 跨 Sprint 固定约束（再次提醒）
1. 任何新层都必须同时进：`state/*`、`artifact_ledger.json`、`workflow_snapshot.json`、`reflection_bundle`。
2. 物理学层禁止做成孤立 demo：trait + state lineage + tests + compare surface + hard gate 作用点五件齐全。
3. 禁止"只加模型，不加 comparability"：每个新层必须回答——上一 run 与本 run 怎么比 / provenance 怎么写 / artifact 怎么存 / regression 怎么测。
4. `main.rs` 永久只做 command wiring 与 thin orchestration；逻辑进：
   - `src/domain/execution/*`
   - `src/application/execution/*`
   - `src/domain/regime/*`
   - `src/application/belief/*`
   - `src/application/orchestration/*`

---

## 近 1 周 Quick Wins（按投入产出排序）
1. **Step 0.1** —— `main.rs` 抽离：最低成本、最高可读性收益，顺带把 Step 0.4 的注入口打开。
2. **Step 0.3** —— gate 门限集中：一次性消除 4 处硬编码，测试代价极小。
3. **Step 0.4** —— `artifact.rs` 接真实 OU：让已经写好的 `estimate_ou_execution_metrics` 产生端到端价值。
4. **Step 0.2** —— `reflection.rs` 重命名 + `why_*` 字段：使 `reflection_bundle` 真正具备 execution 解释性。
5. **Step 0.5** —— `workflow.rs` 去启发式：让 `workflow_snapshot` 字段与 `execution_artifact` 100% 对齐。

---

## 开放问题
- MECE label 的真 fixture 从哪里取？`BTC-Trading-Since-2020` 分桶结果能否作为弱监督起点？
- SHAP 计算走现有 Rust 实现还是 `python_bridge`？后者会引入 cross-runtime provenance 成本。
- `execution_tree` 是否允许在非默认 staged orchestration 路径下运行？默认 off vs on 影响现有 artifact 兼容性。
- `--execution-focus` 升格为默认面后，`agent-first-runbook.md` / `first-run.md` 的叙事顺序同步改写是否需要计入 Step 3？
- `workflow.rs` 的启发式在 Step 0.5 前仍会污染 `workflow_snapshot`。是否需要在 0.5 完成前给字段加一个 `"heuristic": true` 标识，避免误把这段数据当成 artifact 真值？

---

## 验收总览
- **Phase A 收尾**：
  - `main.rs` 不再出现 execution feature 计算；
  - `ReflectionBundle` 含 `why_execution_dominates` / `why_prediction_is_demoted` / 真实 OU 字段；
  - `artifact_ledger` 100% 出现 `execution_artifact`；
  - `workflow_snapshot` 的 execution 字段与 artifact 完全对齐，无硬编码启发式。
- **Sprint 2**：
  - `FrameFeatures` / `reflection_bundle` 各含 `ou_*` / `ising_*` / `pythagorean_*` 字段；
  - 无任何物理层处在孤立 demo 路径。
- **Sprint 3**：
  - MECE recovery `accuracy >= 0.95` 作为 hard gate；
  - recovery + execution consistency 双约束一并写入 `mece_recovery_artifact`。
- **Sprint 4**：
  - `--execution-focus` 是默认面；
  - `reflection_bundle` 必带 SHAP top-k；
  - `hard gate` 以 `execution_readiness` 为首要条件，`prediction_confidence` 降级为次要输入。

完成以上四层叠加后，`ict-engine` 才符合"stateful execution research OS"的定义，而不再是"会记忆的预测引擎"。
