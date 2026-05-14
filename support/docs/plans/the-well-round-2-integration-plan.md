# Round 2: The Well Integration & Surfacing Plan

> 上一轮交付: commit `683f18b` — "feat: land the_well-inspired execution optimizations (2.1-2.5)"
> 上一轮 plan doc: `support/docs/plans/the-well-inspired-execution-optimizations.md`
> 主计划: `support/docs/plans/execution-operating-system-plan.md`
> 日期: 2026-04-21

---

## 第 1 部分 — Round 1 Log (what shipped)

### 提交摘要

| 项 | 值 |
|---|---|
| commit | `683f18b` on branch `green-baseline` |
| 文件 | 35 changed, +3676 / −30 |
| 新模块 | 8 (spectral / sparse / tucker / axial_pool / rollout_segments + 2 overlay + 1 artifact) |
| 新集成测试 | 7 文件, 32 个 test |
| 测试覆盖 | lib 335 passed / integration 186 passed / 0 failed |
| 新依赖 | 0 (radix-2 FFT + Jacobi 特征分解自研) |

### 交付模块清单（下游可用 API）

**数学层 (pure math, no domain deps)**
- `math::spectral::{rfft_one_sided, softshrink_bins, dominant_mode, normalized_spectral_entropy, dominant_energy_ratio, dominant_phase_alignment, high_frequency_noise_ratio, Complex, DominantMode}`

**领域层 (domain structs + estimators)**
- `domain::execution::{SpectralExecutionMetrics, estimate_spectral_execution_metrics, SPECTRAL_ENTROPY_CHAOS_CAP, DOMINANT_ENERGY_FLOOR, SPECTRAL_READINESS_PENALTY, apply_spectral_execution_penalty}`
- `domain::regime::{RolloutSegment, compute_rollout_segments, default_segment_bounds, classify_mece_recovery_segments_gate, MECE_SEGMENT_SHORT_FLOOR/MEDIUM/LONG_FLOOR, MECE_SEGMENT_DRIFT_FLOOR}`
- `domain::regime::{classify_mece_recovery_combined_gate}` (accuracy × sparsity × segments 三重短路)

**因子层**
- `factor_lab::{SparseSelection, sparse_select_by_softshrink, adaptive_lambda, sparsity_ratio_within_bounds, MECE_SPARSITY_LOWER_BOUND/UPPER_BOUND}`
- `factor_lab::{TuckerCore, fit_tucker_core}`
- `factor_lab::{FactorTuckerCoreArtifact, build_factor_tucker_core_artifact, persist_factor_tucker_core_artifact, tucker_attribution_confidence_is_high}`

**应用层 (overlays + orchestration)**
- `application::belief::{SpectralOverlayState, apply_spectral_overlay}`
- `application::orchestration::{AxialPoolConfig, AxialAttentionTrace, axial_pool_mtf_features, axial_branch_gate_triggers_observe, AXIAL_TIMEFRAME_ENTROPY_CAP}`

**artifact / ledger 变更**
- `execution_artifact.json` — schema 扩，`dominant_cycle_energy` / `cycle_phase_alignment` / `spectral_entropy` / `spectral_metrics` 可选字段；ledger version 1→2，rule version `execution-artifact-v2`
- `mece_recovery_artifact.json` — schema 扩，`sparsity_ratio` / `pruned_factor_trail` / `segments`；ledger version 1→2，rule version `mece-recovery-artifact-v2`
- 新 artifact kind `factor_tucker_core`，落 `state/<SYMBOL>/factor_tucker_core.json`

### 跨 Sprint 约束自检 (Round 1)

| 约束 | 状态 |
|---|---|
| §1 Execution 第一类 artifact | ✅ 五层全部入 artifact / ledger / reflection |
| §2 物理学层五件齐全 | ✅ spectral 有 trait + state + tests + compare + hard gate |
| §3 不允许"只加模型不加 comparability" | ✅ 每层都有 fixture / hash / regression 覆盖 |
| §4 main.rs 只做 thin orchestration | ✅ main.rs 零改动 |
| §5 execution gate 先行 | ✅ spectral penalty + sparsity + segment 三个 gate 已接 |

---

## 第 2 部分 — Round 1 没做完的事（integration gap）

Round 1 交付的是**能力层**，还未串到**运行时**。下列几处是链路断点：

### Gap A：axial_pool 未接入 ExecutionTreeInput

**现状**：`axial_pool_mtf_features` / `AxialAttentionTrace` 已实现并有独立测试，但 `ExecutionTreeInput` 的 `physics_overlay` / `hmm_posterior` 仍是标量字段拼接，未过轴向聚合层。

**影响**：Sprint 4 §4.2 要求 `reflection_bundle.execution_shap_top_k` 与 `axial_attention_trace` 交叉验证 — 现在 SHAP 层完全看不到 axial 贡献。

### Gap B：tucker core 没有 upstream caller

**现状**：`fit_tucker_core` + persist 实现完整，但**没有任何一个 CLI 子命令 / 工作流会调用它**。测试里是手造 tensor 跑的。

**影响**：Round 1 的验收目标之一是"至少 1 份真实 symbol 的 `factor_tucker_core.json` 入 ledger" — 暂未达成。

### Gap C：spectral overlay 未进入 SHAP 归因

**现状**：`StructuralExecutionShap::attributions` 列出 `execution_readiness / prediction_vote_score / execution_score / evidence_quality / ising / pythagorean / mece_recovery / branch_probability`。**没有 spectral 条目**。

**影响**：`why_execution_dominates` 字符串里 spectral 已进，但 SHAP 顶端归因里没有 — attribution lineage 不一致。

### Gap D：`--execution-focus` 默认面未交付

**现状**：主计划 Sprint 4 §4.3 把 `--execution-focus` 升为默认面。Round 1 没动这个。

**影响**：CLI 的终端段落还是"方向如何"先行，没变"能否做"先行。

### Gap E：spectral / segments / sparsity 未进 workflow_status 人类视图

**现状**：`application/orchestration/workflow_status.rs` 的 `summary_line` 已扩到 `pda_cluster | duration | remaining_bars`（Round 1 外的未提交改动），但不含 spectral / sparsity / segments 摘要。

**影响**：workflow_status 终端面无法快速读到"spectral_entropy 过高 → 本次 readiness 打折" 这类 attribution。

### Gap F：未在真实 analyze 回放上验证数值

**现状**：所有测试都是合成 fixture（正弦 / LCG 噪声 / 人工 tensor）。**没有**在真实 symbol 全量 re-analyze 过一次。

**影响**：Round 1 验收条件 "所有 symbol re-analyze 后 `execution_artifact.json` 100% 带 spectral 字段" 未验收。

---

## 第 3 部分 — Round 2 Scope（本轮做什么）

Round 2 目标：**让 Round 1 的五层从"能力"变成"链路"**。新增代码体量远小于 Round 1，重点在接线和验收。

### 3.1 Axial pool → ExecutionTreeInput 接线（关 Gap A）

**做什么**：
- 在 `ExecutionTreeInput` 上新增 `axial_trace: Option<&'a AxialAttentionTrace>` 字段（可选，保持向后兼容）
- `DefaultExecutionTreeScorer::score` 消费 `axial_trace.force_observe`：若为 true 且当前 branch 是 `fill_viable` / `wait_for_reversion` → 强制 `observe`
- `ExecutionTreeOutput` 扩 `axial_attention_trace: Vec<(String, f64)>` 持久化进 `execution_tree_trace.json`

**不做**：不构造 tensor 自己跑 axial；tensor 输入由**调用方**（Sprint 4 workflow builder）准备。本轮只做 scorer 这一半，tensor 构造留 Gap I（下轮）。

**测试**：
- `tests/execution_tree_axial_gate.rs` — 提供 force_observe=true 的 trace，断言 fill_viable 被降级为 observe
- 负测：force_observe=false → 原行为不变

**预估**：2-3 天

### 3.2 Tucker caller 接入 factor_lab research（关 Gap B）

**做什么**：
- `factor_lab::research::FactorLab` 加一个可选方法 `compute_factor_tucker_core(&self, candles, regime_labels, timeframe_labels) -> Option<TuckerCore>`
- 内部构造 `[nf × nr × nt]` 张量：每个 cell = 该因子在该 regime × 该 timeframe 下的归一化信号均值
- 暴露 CLI 钩子（哪个子命令待定 — 优先复用 `factor-autoresearch` 或 `analyze`，不单独新开子命令）
- 在该子命令里调用 `build_factor_tucker_core_artifact` + `persist_factor_tucker_core_artifact`

**验收**：对至少 1 个真实 symbol (e.g. `state_autoresearch_smoke` 里已有的 NQ 数据) 产出一份 `factor_tucker_core.json` 入 ledger。

**测试**：
- 单测：`FactorLab::compute_factor_tucker_core` 在 fixture 上返回非 None
- 集测：端到端跑一个 autoresearch smoke 路径，artifact_ledger 里能 grep 到 `factor_tucker_core` kind

**预估**：3-4 天

### 3.3 Spectral 进 SHAP 归因（关 Gap C）

**做什么**：
- `StructuralExecutionShap::attributions` 追加三条：
  - `spectral_entropy` contribution = `SPECTRAL_ENTROPY_CHAOS_CAP - entropy`（向上偏离 → 正贡献，negative 推向 block）
  - `dominant_cycle_energy` contribution = `energy - DOMINANT_ENERGY_FLOOR`
  - `cycle_phase_alignment` contribution = `alignment - 0.0`（相位对齐 >0 推向 fill）
- 仅当 `features.spectral_metrics.is_some()` 时才追加，保持 SHAP 长度稳定

**测试**：
- 扩 `application::reflection::execution_tree_bundle::tests` — 断言带 spectral features 的 input 产生包含 3 个 spectral SHAP 条目的 top_k

**预估**：1-2 天

### 3.4 workflow_status 终端面扩 spectral / sparsity / segments（关 Gap E）

**做什么**：
- `build_human_workflow_status_view` 的 `summary_line` 扩：`... | spectral_entropy={e} | sparsity={s} | segments_gate={promote|blocked}`
- 仅在对应 artifact 存在时显示；不存在时 `unavailable`
- 跟既有 `pda_cluster=` / `duration=` 同一 pattern

**注意**：这个文件已有未提交改动（pda_cluster / duration 已加），本轮做的是进一步扩展，commit 时明确区分

**测试**：
- `tests/` 现有的 `build_workflow_status_phase_value_matches_human_surface` 同类断言覆盖新字段

**预估**：1 天

### 3.5 真实 symbol 回放 & 验收签字（关 Gap F）

**做什么**：
- 写一个 `support/scripts/round2_smoke_replay.sh` —
  1. 对 `state_autoresearch_smoke` / `state100` 等已有 state dir 跑 `analyze` / `workflow-status`
  2. 断言 `execution_artifact.json` 带 spectral_metrics
  3. 断言 `mece_recovery_artifact.json` 带 sparsity_ratio / segments
  4. 断言 ledger 有 `factor_tucker_core` entry（前提 3.2 已落地）
- 脚本输出 pass/fail + diff

**不做**：不改 CI 流水线接入（保持与主计划步调）。脚本暴露即可，人工触发。

**预估**：1-2 天

### 3.6 `--execution-focus` 默认面最小可用版（关 Gap D — 部分）

**做什么**：
- `analyze` / `workflow-status` 增加 `--execution-focus` flag（默认 false，本轮不强制为默认值 — 只先做"可用"）
- flag 为 true 时，人类视图终端段落先渲染 "能否做"：`gate_status` + `why_execution_dominates` + `execution_shap_top_k` top-3
- 不删除原方向信息，仅换顺序

**不做**：**不把 flag 升为默认值**。Sprint 4 的 §4.3 目标是"默认面" — Round 2 只做铺路，避免在 pipeline 未 100% 铺满 spectral/axial 时就强改默认行为。

**预估**：2-3 天

**合计 Round 2 工作量估**：10-15 天。

---

## 第 4 部分 — Round 2 不做的事（显式列出）

- **不**接入真实 Python bridge 去跑 the_well 数据集（违背 Rust 单一执行面）
- **不**把 flag `--execution-focus` 升默认值（时机是 Sprint 4 完整落地后）
- **不**改 main.rs 除非是 thin command wiring（约束 §4）
- **不**引入 rustfft / ndarray-linalg 等 heavy 依赖
- **不**改 `ExecutionTreeOutput.branch` 新增枚举 — 仅复用 `observe` / `fill_viable` / `wait_for_reversion` / `block_crowded`
- **不**做 HDF5 持久化（规模未到拐点）
- **不**做多 symbol 块对角 scorer

---

## 第 5 部分 — Round 2 跨 Sprint 约束预检

逐条检查 Round 2 内容是否合规：

| 约束 | 3.1 axial | 3.2 tucker | 3.3 shap | 3.4 status | 3.5 replay | 3.6 focus |
|---|---|---|---|---|---|---|
| §1 第一类 artifact | ✅ 进 execution_tree_trace | ✅ 新 kind | — (attribution) | — (read-only view) | — (validator) | — (view) |
| §2 五件齐全 | ✅ (已建) | ✅ (已建) | — | — | — | — |
| §3 comparability | ✅ regression | ✅ end-to-end | ✅ inline | ✅ inline | ✅ asserts | ✅ inline |
| §4 main.rs thin | ✅ | ✅ (研究路径 + 少量 wiring) | ✅ | ⚠️ 可能需加 1 个 flag 注册 | — (script) | ⚠️ 加 flag + wiring |
| §5 gate 先行 | ✅ 接 observe gate | N/A (lineage) | N/A | — | ✅ 验证 gate | — |

⚠️ 标记的两条要特别注意：新 flag 注册只做 `clap` 结构里的 `--execution-focus: bool` 和一个 `if` 路由，不在 main.rs 写任何 execution 计算逻辑。

---

## 第 6 部分 — Round 2 执行顺序建议

```
Day 1-2    : 3.3 spectral→SHAP (最小，先确认 attribution 闭环)
Day 3-4    : 3.1 axial→ExecutionTreeInput
Day 5-7    : 3.2 tucker caller + 真实 symbol 验证
Day 8      : 3.4 workflow_status 摘要扩
Day 9-10   : 3.6 --execution-focus flag (非默认)
Day 11-12  : 3.5 回放脚本 + 全链路 smoke
Day 13-15  : buffer + 回归修复 + 下一轮 plan 起草
```

---

## 第 7 部分 — Round 2 验收

完成 Round 2 后，ict-engine 应同时满足：

- 对至少 1 个真实 symbol，全链路跑一次 analyze 后：
  - `execution_artifact.json` 带 `spectral_metrics.*` 四字段（Round 1 验收补签字）
  - `mece_recovery_artifact.json` 带 `sparsity_ratio > 0` + `segments.len() == 3`
  - `artifact_ledger.json` 含 `factor_tucker_core` entry（新）
  - `reflection_bundle.execution_shap_top_k` 含至少 1 条 spectral 归因
  - `execution_tree_trace.json` 含 `axial_attention_trace`（当有 tensor 输入）
- `--execution-focus` flag 存在且可用（但**不默认**）
- `support/scripts/round2_smoke_replay.sh` 可跑可过
- 全量 `cargo test` 仍然 0 failed

---

## 第 8 部分 — 与主计划的关系

| 主计划 Sprint | Round 1 | Round 2 | Sprint 剩余（Round 3+） |
|---|---|---|---|
| Step 0 Phase A | ✅ 已完 | — | — |
| Sprint 1 Execution | ✅ 已完 | — | — |
| Sprint 2 Physics | ✅ + spectral | — | — |
| Sprint 3 MECE recovery | 能力完 | **接入真实数据** (3.2, 3.5) | artifact replay 压测 |
| Sprint 4 Execution Tree | 骨架完 | **axial + SHAP + flag** (3.1, 3.3, 3.6) | `--execution-focus` 升默认 |

Round 2 ≈ 主计划 Sprint 3 后半 + Sprint 4 中段。Round 3 起开始正式的 Sprint 4 收官。

---

## 附录 — 如果卡住

**卡 3.2（tucker caller）**：如果 FactorLab 当前结构不方便嵌 tucker 计算，退而求其次 — 写 `src/factor_lab/tucker_driver.rs`，暴露一个 `build_factor_tensor_from_research_runs(state_dir) -> Array3<f64>`，在 `factor-autoresearch` 子命令结尾调用。**不**新开子命令。

**卡 3.1（axial 接线）**：如果 `ExecutionTreeInput` 生命周期改动扩散太大，采用 owned 而非 ref，`Option<AxialAttentionTrace>` 直接 clone 进去；性能在这层无关键路径。

**卡 3.6（flag 注册）**：如果发现 clap 侧要动的地方超过 20 行，先在 `reporting::execution_focus` 里暴露函数 + 在 workflow_status 里加一个 env var 开关（`ICT_EXECUTION_FOCUS=1`），clap flag 后面再补。
