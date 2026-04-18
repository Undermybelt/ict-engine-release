# ict-engine smoke / acceptance

目的：
- 快速验证本仓核心 CLI 是否可用
- 验证 analyze -> factor-research -> update -> workflow-status 主链可跑通
- 验证历史数据复用已改为“先问用户选数据”的硬门禁

## 1. 最小前置

在仓根执行：

```bash
cargo fmt --all
cargo check
```

若仅做冒烟，不必先跑全量测试；若改了 workflow / state / command surface，建议加：

```bash
cargo test
```

## 2. 准备 smoke 数据

可用任意满足最小 bar 数要求的数据。
当前经验：少于 29 根会报：

```text
need at least 29 candles to build features
```

本地可直接生成三份假数据：
- `/tmp/ict-engine-smoke-htf.json`
- `/tmp/ict-engine-smoke-mtf.json`
- `/tmp/ict-engine-smoke-ltf.json`

建议每份 >= 80 bars。

## 3. train

```bash
cargo run -- train \
  --symbol NQ \
  --data /tmp/ict-engine-smoke-ltf.json \
  --epochs 1 \
  --state-dir /tmp/ict-engine-smoke-state
```

期待：
- 命令成功退出
- 输出包含 `workflow_phase=train`
- state 下生成 `NQ/hmm_params.json`

## 4. analyze

```bash
cargo run -- analyze \
  --symbol NQ \
  --data-htf /tmp/ict-engine-smoke-htf.json \
  --data-mtf /tmp/ict-engine-smoke-mtf.json \
  --data-ltf /tmp/ict-engine-smoke-ltf.json \
  --state-dir /tmp/ict-engine-smoke-state
```

期待：
- 输出含：
  - `report`
  - `compact_report`
  - `agent_report`
  - `human_report`
- state 下生成：
  - `pending_update_feedback.json`
  - `execution_candidate.json`
- `recommended_commands.research` 与 `recommended_commands.backtest` 不应直接 ready

## 5. 历史数据复用硬门禁

当前规则：
- 只要推荐命令要复用历史数据去跑 `factor-research` / `factor-backtest`
- 即使系统已记录路径，也必须先问用户选数据
- 因此推荐命令应表现为：
  - `ready = false`
  - `missing_inputs` 包含 `user_selected_historical_data`
  - `user_data_selection_required = true`
  - `user_data_selection_prompt` 非空
  - `recorded_data_paths` 列出已知路径

可直接检查 analyze 输出：

```bash
cargo run -- analyze \
  --symbol NQ \
  --data-htf /tmp/ict-engine-smoke-htf.json \
  --data-mtf /tmp/ict-engine-smoke-mtf.json \
  --data-ltf /tmp/ict-engine-smoke-ltf.json \
  --state-dir /tmp/ict-engine-smoke-state \
  > /tmp/ict-engine-analyze-out.json
```

再看：

```bash
python - <<'PY'
import json
obj = json.load(open('/tmp/ict-engine-analyze-out.json'))
rc = obj['report']['supporting']['recommended_commands']
print(json.dumps({
  'research_ready': rc['research']['ready'],
  'research_missing_inputs': rc['research']['missing_inputs'],
  'research_user_data_selection_required': rc['research']['user_data_selection_required'],
  'research_user_data_selection_prompt': rc['research']['user_data_selection_prompt'],
  'research_recorded_data_paths': rc['research']['recorded_data_paths'],
  'backtest_ready': rc['backtest']['ready'],
  'backtest_missing_inputs': rc['backtest']['missing_inputs'],
}, indent=2))
PY
```

期待：
- `research_ready = false`
- `backtest_ready = false`
- 两者 `missing_inputs` 含 `user_selected_historical_data`

## 6. factor-research

即便系统推荐会先问用户，冒烟验证时可手工指定已知数据路径运行：

```bash
cargo run -- factor-research \
  --symbol NQ \
  --data /tmp/ict-engine-smoke-ltf.json \
  --objective generic \
  --state-dir /tmp/ict-engine-smoke-state
```

期待：
- 输出含：
  - `report`
  - `reflection_bundle`
  - `factor_lifecycle`
- `dataset_comparability.comparison_class` 不应再是假性 `different_data_fingerprint`
- 若与 analyze 用同一份数据，当前预期通常为：
  - `same_data_different_config`

说明：
- research 的 data fingerprint 已与 analyze 对齐
- 因此同数据只会反映真实“配置差异”，不再误判为“数据不同”

## 7. update

```bash
cargo run -- update \
  --symbol NQ \
  --outcome win \
  --entry-signal medium \
  --state-dir /tmp/ict-engine-smoke-state
```

期待：
- 输出含：
  - `report`
  - `reflection_bundle`
- consumed artifact 被标记为已消费
- `update_runs.json` 新增一笔

## 8. workflow-status

```bash
cargo run -- workflow-status \
  --symbol NQ \
  --state-dir /tmp/ict-engine-smoke-state
```

期待：
- 能返回完整 snapshot
- `latest_train` / `latest_analyze` / `latest_research` / `latest_update` 均可见
- `current_focus_phase` 与最近动作一致

## 9. 快速验收点

若以下都成立，可视为本轮主链通过：
- `cargo check` 通过
- `train` 成功
- `analyze` 成功
- `factor-research` 成功
- `update` 成功
- `workflow-status` 成功
- `update` 输出含 `reflection_bundle`
- research/backtest 推荐命令进入“先问用户选数据”的硬门禁
- research 与 analyze 同数据时，不再误报 `different_data_fingerprint`

## 10. 已知设计约束

1. 当前“用户选数据”门禁落在推荐/工作流层，不是 CLI 参数级交互器。
2. 若要把门禁再升一级，可后续增加显式确认参数，例如：
   - `--approved-data-path`
   - `--data-choice-note`
3. 目前 repo 仍以 `README.md` 作为概览，以本文件承载执行级 smoke/acceptance 流。
