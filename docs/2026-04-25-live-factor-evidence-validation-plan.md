# 2026-04-25 live factor evidence validation plan

## Goal

验证 `ict-engine` 当前实现是否满足以下预期：

1. 实时数据可以进入 `analyze-live` 主链路。
2. 因子输出不会新建 BBN / CatBoost 节点。
3. 因子诊断与 pre-Bayes 滤波结果会作为特征/证据进入已有 BBN 节点或已有策略树。
4. 运行后会把关键产物持久化到 state，并能据此形成可审计结论。

## Validation standard

本次以“进入已有节点或策略树”为验收标准，不以“新增节点存在”为标准。

需要同时满足：

- 因子输出进入 `build_pre_bayes_evidence_filter(...)`
- `PreBayesEvidenceFilter` 被映射进 `trade_evidence_from_pre_bayes_filter(...)`
- BBN 使用已有节点承接这些证据，例如：
  - `market_regime`
  - `liquidity_context`
  - `factor_alignment`
  - `factor_uncertainty`
  - `multi_timeframe_resonance`
  - `entry_quality`
  - `trade_outcome`
- 策略层使用已有策略树/执行树/ensemble surface 承接这些结果，例如：
  - `ExecutionTreeArtifact`
  - `EnsembleVoteRecord`
  - `CatBoostCompatiblePolicyEngine`
- 若 CatBoost 只是文件回放/占位策略，而不是训练好的真实模型，需要在结论里明确写出。

## Planned evidence collection

### 1. Static code-path verification

确认以下链路存在且连通：

- `analyze-live` -> `analyze_live_command(...)`
- live backend -> `build_live_data_source(...)`
- `build_analyze_report(...)`
- `FactorEngine::run(...)`
- `build_pre_bayes_evidence_filter(...)`
- `trade_evidence_from_pre_bayes_filter(...)`
- `infer_entry_quality(...)` / `infer_trade_outcome(...)`
- `build_execution_tree_artifact(...)`
- `persist_execution_tree_artifact(...)`
- `build_stub_ensemble_vote_from_input(...)`
- `persist_ensemble_vote_record(...)`

### 2. Runtime verification

优先尝试最小侵入验证：

- 查看已有二进制/帮助输出，确认 `analyze-live` CLI 入口存在
- 选择独立 `--state-dir`，避免污染 repo 默认 `state/`
- 以默认 `openbb` live backend 为首选做一次真实运行
- 若 live 请求失败，记录失败位置与原因，不伪造“已验证成功”

### 3. Artifact verification

若运行成功，检查独立 state 目录中是否生成：

- `workflow_snapshot.json`
- `artifact_ledger.json`
- `execution_tree_trace.json`
- `ensemble_vote.json` / ensemble history
- `execution_candidate.json`
- `pending_update_artifact.json`
- live 持久化 candles 快照

## Expected risks

- `analyze-live` 依赖外部实时数据源，可能受网络、供应商接口、频率限制影响。
- `openbb` 路径实际实现更接近 Yahoo / Barchart 抓取与兼容层，不等于本地 OpenBB server。
- `CatBoostCompatiblePolicyEngine` 当前可能只是 schema/sample-policy 占位，不代表真实训练过的 CatBoost 模型。
- staged orchestration 受环境变量控制，默认不一定产出 staged artifacts。

## Deliverables

完成后在 `docs/` 写一份报告，至少包含：

1. 结论：当前实现是否符合“作为特征/证据进入已有节点或策略树”的预期。
2. 证据：代码路径、运行命令、关键输出、产物路径。
3. 边界：哪些部分已验证，哪些部分仍是占位或未闭环。
4. 建议：若链路不完整，给出最小补齐建议。
