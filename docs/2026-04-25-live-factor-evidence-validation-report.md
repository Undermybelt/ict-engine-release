# 2026-04-25 live factor evidence validation report

## Scope

本报告验证 `ict-engine` 当前实现是否满足以下预期：

- 实时数据可进入主分析链路
- 因子与滤波结果作为**特征/证据**进入**已有** BBN 节点或已有策略树
- 因子不会新建 BBN / CatBoost 节点
- 运行后存在可审计产物

相关计划见：`docs/2026-04-25-live-factor-evidence-validation-plan.md`

## Executive conclusion

结论分三层：

### 1. 已验证成立

`ict-engine` 当前已经实现了下面这条链路：

`live data -> analyze-live -> FactorEngine -> pre-Bayes filter -> BBN existing nodes -> execution tree artifact -> ensemble vote artifact`

也就是说，你提出的核心预期里，**“因子/滤波结果作为特征或证据进入已有节点/策略树”这一点是成立的**。

### 2. 需要精确表述

当前实现里，因子结果**不是**“变成新的 BBN 节点”或“生成新的 CatBoost 树节点”。

更准确地说：

- 因子诊断先进入 `build_pre_bayes_evidence_filter(...)`
- 再被映射成已有 BBN 节点的 hard/soft evidence
- 同时 belief / pre-Bayes / PDA 等字段被整理为 `PolicyFeatureVector`
- 再进入 file-backed 的 `catboost_file` / `xgboost_file` policy executor surface

所以正确表述应是：

**因子与滤波结果被投影为已有 BBN 节点的证据，以及已有策略层的特征。**

### 3. 目前仍是占位/样例的部分

`CatBoostCompatiblePolicyEngine` 当前**不是训练好的真实 CatBoost runtime**，而是：

- sample JSON / placeholder 驱动
- schema-compatible
- file-backed policy executor
- 可产出决策与 split trace
- 但不等于真实训练树推理引擎

因此如果你的更高期望是“真实训练过的 CatBoost 模型树在生产推理”，**当前仓库还没有把这一层闭环到位**。

## Runtime validation

### Executed command

```bash
./target/debug/ict-engine analyze-live --symbol NQ --state-dir /tmp/ict-engine-live-validate-20260425
```

### Result

命令执行成功，返回 live analyze 输出，并在隔离 state 目录生成完整产物。

### Runtime evidence summary

本次运行的关键结果包括：

- 实时数据源成功接入
- `source_snapshot` 显示：
  - `futures_backend = openbb`
  - `aux_backend = openbb`
  - `futures_base_url = native://openbb`
  - `aux_base_url = native://openbb`
- 输出里出现：
  - `pre_bayes_quality_score = 0.637`
  - `gating_status = pass_neutralized`
  - `market_behavior_profile = index_beta_regime_sensitive`
- workflow 建议下一步为：
  - 先确认历史数据复用路径
  - 再进入 `factor-research`

## Persisted artifacts

本次运行在以下目录生成了审计产物：

`/tmp/ict-engine-live-validate-20260425/NQ`

关键文件已确认存在：

- `workflow_snapshot.json`
- `artifact_ledger.json`
- `execution_tree_trace.json`
- `ensemble_vote.json`
- `ensemble_vote_history.json`
- `execution_candidate.json`
- `pending_update_feedback.json`
- `analyze_live_20260425T100923_*.json` 多周期 live candles 快照

## Code-path verification

## Live data entry

`analyze-live` CLI 已存在并可执行。

主入口链路为：

- `main.rs`
- `analyze_live_command(...)`
- `build_live_data_source(...)`
- `build_analyze_report(...)`

`analyze_live_command(...)` 会拉取：

- futures: `1d`, `4h`, `1h`, `15m`, `5m`, `1m`
- spot candles
- options summary

然后把这些输入交给 `build_analyze_report(...)`。

## Factor -> pre-Bayes filter

在 `build_analyze_report(...)` 中，主链路明确包含：

- `FactorEngine::run(...)`
- `build_pre_bayes_evidence_filter(...)`
- `trade_evidence_from_pre_bayes_filter(...)`
- `infer_entry_quality(...)`
- `infer_trade_outcome(...)`

### What from factors enters the filter

`build_pre_bayes_evidence_filter(...)` 明确消费了 `factor_diagnostics` 的这些信息：

- `alignment_label`
- `uncertainty_label`
- `long_support`
- `short_support`
- `uncertainty`

同时还结合：

- multi-timeframe evidence
- market override policy
- PDA sequence summary
- regime / liquidity labels

这说明**因子不是被丢掉，而是直接进入了 pre-Bayes 滤波层**。

## Filter -> existing BBN nodes

`build_pre_bayes_evidence_filter(...)` 会生成：

- `evidence_assignments`
- `gating_status`
- `uses_soft_evidence`
- 各节点 soft distribution

其 `evidence_assignments` 明确对应已有 BBN 节点：

- `market_regime`
- `liquidity_context`
- `factor_alignment`
- `factor_uncertainty`
- `multi_timeframe_resonance`

之后 `trade_evidence_from_pre_bayes_filter(...)` 会把这些 assignment / distribution 映射为 `Evidence`，写入已有网络节点：

- `market_regime`
- `liquidity_context`
- `factor_alignment`
- `factor_uncertainty`
- `multi_timeframe_resonance`
- `entry_quality`

其中：

- `entry_quality` 还会根据 `active_pda_count` / `inversed_pda_count` 再做已有节点证据映射
- `trade_outcome` 则通过已有网络 CPT 推断得到

因此在 BBN 这一层，**你的预期是完全成立的**：

**因子/滤波不是生成新节点，而是作为已有节点的证据进入网络。**

## Runtime proof from workflow snapshot

`workflow_snapshot.json` 中可直接看到这组证据：

- `pre_bayes_uses_soft_evidence = true`
- `pre_bayes_filtered_assignments` 包含：
  - `factor_alignment = mixed`
  - `factor_uncertainty = low`
  - `liquidity_context = favorable`
  - `market_regime = bull`
  - `multi_timeframe_resonance = aligned`
- `pre_bayes_soft_evidence` 里存在上述节点的软分布

这已经是运行态证据，不只是静态代码推断。

## Execution tree verification

`build_analyze_report(...)` 在 BBN / decision 之后还会继续构建：

- `derive_execution_inputs(...)`
- `build_execution_artifact_from_snapshot(...)`
- `ExecutionTreeInput`
- `DefaultExecutionTreeScorer.score(...)`
- `build_execution_tree_artifact(...)`
- `persist_execution_tree_artifact(...)`

本次运行已落盘：

`/tmp/ict-engine-live-validate-20260425/NQ/execution_tree_trace.json`

关键结果：

- `branch = transition_guardrail`
- `execution_bias = guarded`
- `gate_status = observe`
- `decision_hint = execution_guarded_due_to_high_transition_hazard`

### Important boundary

这里的 execution tree 是**已有执行树/规则树面**，已经真实落盘并参与决策面。

但它当前默认的 SHAP 解释是 `StructuralExecutionShap`，代码注释明确说明：

- 这是 structural attribution
- 不是来自真实 CatBoost/XGBoost Shapley value
- 是为了先满足 execution surface 的可解释性

所以：

- **execution tree 存在且已工作**
- **但它不是 CatBoost 训练树本体**

## CatBoost-compatible policy layer verification

## Static truth

`CatBoostCompatiblePolicyEngine` 当前默认加载：

- `src/application/orchestration/catboost_policy.sample.json`
- 若文件不可用则退回 placeholder

其静态事实包括：

- `engine_name() = "catboost-compatible-placeholder"`
- placeholder `notes` 含：`catboost_schema_only_no_runtime`
- sample JSON 中：`"trees": []`

因此这层目前是：

- **CatBoost-compatible**
- **file-backed**
- **sample / placeholder**
- **不是训练好的真实 CatBoost runtime model**

## Which features enter this layer

`policy_features_from_input(...)` 会把已有 belief / pre-Bayes 信息整理成 `PolicyFeatureVector`。

已确认进入该层的关键字段包括：

- **categorical / derived**
  - `factor_alignment`
  - `factor_uncertainty`
  - `gating_status`
  - `selected_entry_quality`
  - `recommended_command`
  - `selected_direction`
  - `setup_family`
  - `entry_style`
  - `risk_template`
  - `setup_quality`
  - `signal_bar_pattern`
  - `session_model`
  - `htf_rb_type`
  - `ltf_path_label`
  - `latest_break_type`
  - `pda_survival_regime`

- **numerical / boolean / ICT-derived**
  - `evidence_quality_score`
  - `risk_reward`
  - `kelly_fraction`
  - `overlap_ratio`
  - `displacement_strength`
  - `event_b_consecutive_count`
  - `event_a_sequence_stage`
  - `killswitch_completion`
  - `fvgs_open`
  - `order_blocks_nearby`
  - `pda_bull_count`
  - `liquidity_sweep_count`
  - 以及多种 PDA / ICT 相关布尔特征

这说明**pre-Bayes / belief / ICT 信息确实进入了已有策略树特征面**。

## Runtime proof from ensemble vote

本次运行已落盘：

`/tmp/ict-engine-live-validate-20260425/NQ/ensemble_vote.json`

其中 `executor_summaries` 明确出现：

- `executor=catboost_file ...`
- `executor=xgboost_file ...`

并且 `split_explanations` 包含：

- `gating_status=trend`
- `selected_entry_quality=medium`
- `factor_alignment=index_beta_regime_sensitive`
- `factor_uncertainty=low`

这证明当前策略层已经把 belief / pre-Bayes 派生特征送进了已有 policy executor surface。

## Important interpretation

这里需要非常谨慎地区分两件事：

### 成立的说法

- 因子/滤波结果进入了已有策略树特征层
- 当前系统能产出 `catboost_file` / `xgboost_file` 风格的 executor 决策
- ensemble vote 已经把这些 executor 结果纳入汇总

### 不应夸大的说法

- 这**不等于**系统已经跑了训练好的真实 CatBoost 模型树
- 这**也不等于** execution tree 本身就是 CatBoost 树

## Artifact ledger proof

`artifact_ledger.json` 中已记录以下关键 artifact kind：

- `execution_tree_artifact`
- `pending_update`
- `execution_candidate`
- `ensemble_vote`
- `mece_recovery_artifact`

这说明运行结果不是只打印到 stdout，而是已经进入可追踪的 artifact ledger。

## Answer to the original expectation

如果把你的预期精确定义为：

**“实时数据进入后，因子及其滤波结果作为特征/证据进入已有 BBN 节点或已有策略树，并产出可审计结果。”**

那么结论是：

**是的，当前仓库已经基本这样工作。**

但如果把预期定义为：

**“系统会把因子真正编译成新的 BBN 节点或真实训练 CatBoost 树节点。”**

那么结论是：

**不是。当前实现并没有这么做。**

## Current boundary summary

### Verified now

- `analyze-live` 可真实跑通
- live data 已进入主分析链路
- `FactorEngine` 已参与 live analyze
- factor diagnostics 已进入 pre-Bayes filter
- pre-Bayes filter 已映射到已有 BBN 节点
- execution tree artifact 已真实落盘
- ensemble vote artifact 已真实落盘
- `catboost_file` executor 已参与策略层汇总

### Not fully closed yet

- 真实训练 CatBoost 模型 runtime
- 非占位的树结构推理
- 真正来自模型的 SHAP / feature attribution
- “因子 -> 新增节点”的机制（当前也不建议按这个方向理解）

## Minimal next-step recommendation

如果你下一步想把这条链进一步从“兼容/占位”推到“更实战”，最小增量建议是：

### 1. 保持 BBN 侧不新增节点

继续沿用当前做法：

- 因子 -> `PreBayesEvidenceFilter`
- filter -> existing BBN node evidence

这条边界已经比较清晰，不建议改成“因子随意长新节点”。

### 2. 把 policy layer 从 sample JSON 升级为真实模型 artifact

优先替换：

- `catboost_policy.sample.json`
- `xgboost_policy.sample.json`

目标是：

- 保留同一套 `PolicyFeatureVector`
- 但接真实训练产物
- 让 `trees` / `leaf_outputs` 不再是空壳或样例

### 3. 区分 execution tree 与 CatBoost policy

建议文档上明确两层：

- **execution tree** = 执行可行性/风险分支面
- **CatBoost policy layer** = post-BBN policy action layer

避免后续把两者混成一个概念。

## Files produced by this task

- `docs/2026-04-25-live-factor-evidence-validation-plan.md`
- `docs/2026-04-25-live-factor-evidence-validation-report.md`

## Repro notes

本次验证使用独立 state 目录：

`/tmp/ict-engine-live-validate-20260425`

如需复验，可再次运行：

```bash
./target/debug/ict-engine analyze-live --symbol NQ --state-dir /tmp/ict-engine-live-validate-20260425
```
