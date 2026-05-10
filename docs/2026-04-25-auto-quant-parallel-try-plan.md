# 2026-04-25 auto-quant parallel try plan

## Goal

基于刚生成的 live snapshots，验证 `ict-engine` 中 `auto-quant` 相关入口的可执行边界，并尽量并发试跑所有合理组合。

## Scope

当前仓库内 `auto-quant` 相关入口主要是：

- `factor-research --backend auto-quant`
- `factor-autoresearch --backend auto-quant`
- `auto-quant-status`
- `auto-quant-bootstrap`
- `auto-quant-update`

其中真正与研究入口直接相关的是前两个；它们当前在 `ict-engine` 中的职责是：

- 检查 / bootstrap managed Auto-Quant dependency
- 生成并持久化 handoff payload
- 给出下一步 `uv run prepare.py` / `uv run run.py` 的建议

## Practical interpretation of "都试试"

本次把“都试试”解释为 4 条并发线：

1. `factor-research` + `objective=expansion_manipulation`
2. `factor-research` + `objective=generic`
3. `factor-autoresearch` + `objective=expansion_manipulation`
4. `factor-autoresearch` + `objective=generic`

## Isolation strategy

### Shared managed dependency

为了避免重复 clone，多条并发线共享同一个 Auto-Quant checkout：

- `ICT_ENGINE_AUTO_QUANT_DIR=/tmp/ict-engine-auto-quant-shared`

### Isolated state dirs

为了避免 handoff 文件和 ledger 互相覆盖，每条并发线使用独立 state dir：

- `/tmp/ict-engine-aq-r-expansion`
- `/tmp/ict-engine-aq-r-generic`
- `/tmp/ict-engine-aq-ar-expansion`
- `/tmp/ict-engine-aq-ar-generic`

## Input data

复用上一轮 live analyze 的已落盘数据：

- LTF data:
  - `/tmp/ict-engine-live-validate-20260425/NQ/analyze_live_20260425T100923_ltf.json`
- Paired spot data:
  - `/tmp/ict-engine-live-validate-20260425/NQ/analyze_live_20260425T100923_spot.json`
- Optional native frames:
  - `m1`, `m5`, `mtf(15m)`, `h4`, `htf(1d)`

## Planned execution order

### Phase 1

单次执行：

- `auto-quant-status`
- `auto-quant-bootstrap`

目标：确保 shared managed dependency 存在且健康。

### Phase 2

在 dependency ready 后并发执行 4 条命令：

- `factor-research --backend auto-quant --objective expansion_manipulation`
- `factor-research --backend auto-quant --objective generic`
- `factor-autoresearch --backend auto-quant --objective expansion_manipulation`
- `factor-autoresearch --backend auto-quant --objective generic`

## Success criteria

至少确认：

1. shared Auto-Quant dependency bootstrap 成功，或明确失败原因。
2. 4 条命令都能进入 `auto-quant` handoff 路径并落盘 artifact；若失败，记录失败点。
3. 识别当前实现是否只到 handoff，不到真正 Auto-Quant run loop。
4. 把产物路径和结论补进 `docs` 报告。

## Expected boundary

预计当前 `ict-engine` 不会直接在内部跑完 Auto-Quant 研究本体，而是：

- 生成 handoff payload
- 将后续执行委托给 external Auto-Quant workspace

如果事实如此，需要在报告里明确写为“handoff integration verified, external runtime not fully exercised inside ict-engine”.
