# ICT Engine 首轮从零试用报告

**日期:** 2026-04-25  
**试用位置:** `/Users/thrill3r/projects-ict-engine/ict-engine`  
**环境:** macOS, `cargo 1.94.1`, `rustc 1.94.1`  
**试用目标:** 站在第一次接触项目的使用者角度，按 README 的入口完成一轮可复现使用，并记录真实摩擦点。

## 总体结论

ICT Engine 的 Rust-only demo 链路可以跑通：`cargo check` 通过，`analyze --demo --human` 能给出交易桌风格摘要，`factor-pipeline-debug` 能输出足够完整的诊断证据，`--backend native` 的 `factor-research` 能在不安装 Python 依赖的情况下生成研究状态。

但“从零开始”的默认路径不够顺：README 说 Rust 足够跑 core CLI，可 `factor-research` 默认走 `auto-quant`，首次会自动准备外部依赖并停在数据准备状态；继续按推荐命令执行时又被 `TA-Lib` 缺失卡住。另一个明显问题是 `factor-research --backend native --human` 实际仍输出巨大 JSON 字符串，并且其中的中文提示出现 mojibake，这会让新用户误判 `--human` 是否可用。

## 实际执行记录

| 步骤 | 命令 | 结果 | 体验记录 |
|---|---|---|---|
| 冷启动帮助 | `cargo run -- --help` | 成功，约 1m28s | 首次运行会触发完整编译，等待感明显；命令列表完整。 |
| 构建检查 | `cargo check` | 成功，约 16s | README 的第一个 baseline 成立。 |
| 子命令帮助 | `cargo run -- analyze --help` / `cargo run -- factor-research --help` | 成功 | 若并行执行，会出现 Cargo package/artifact lock 等待；建议文档按串行写。 |
| Demo 分析 | `cargo run -- analyze --symbol DEMO --demo --human` | 成功，约 1.66s | 输出简洁，可读性好。给出 `Bull bias`、`Gate: pass_hard`、`Quality: 0.880` 和下一步。 |
| 默认研究 | `cargo run -- factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-first-run-20260425` | 成功返回 handoff JSON，约 8.68s | 默认 `auto-quant` 没直接完成研究，而是返回 `dependency_ready_data_missing`，推荐先跑 Auto-Quant prepare。 |
| 推荐 prepare | `uv run /tmp/ict-engine-first-run-20260425/.deps/auto-quant/prepare.py` | 失败，约 0.28s | 阻塞于 `ERROR: TA-Lib is not installed.`，提示 `brew install ta-lib` 或 Docker fallback。 |
| Native 研究 | `cargo run -- factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-first-run-native-20260425 --backend native --human` | 成功，约 1.50s | 研究本身跑通：5 个 factor，best factor 为 `trend_momentum`，生成 46 条反馈。但 `--human` 输出不是人类摘要，而是超长 JSON 字符串。 |
| Pipeline debug | `cargo run -- factor-pipeline-debug --symbol DEMO --data examples/demo/demo-15m.json --factor structure_ict --objective expansion_manipulation` | 成功，约 1.33s | 诊断面很强，关键字段包括 `evidence_quality_score=0.6173`、`gating_status=pass_neutralized`、`bridge_gap=0.0216`、`pipeline_verdict=pre_bayes_pass_but_bridge_needs_confirmation`。 |
| Backtest 边界 | `cargo run -- backtest --symbol DEMO --data examples/demo/demo-15m.json --human --state-dir /tmp/ict-engine-first-run-backtest-20260425` | 预期失败，约 1.42s | 错误清楚：`got 52, require at least 71`。这与 README 对 demo 数据不足以 backtest 的说明一致。 |
| Workflow 状态 | `cargo run -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-first-run-native-20260425 --human` | 成功 | 显示 `action_blocked`，原因是 `user_selected_historical_data_missing`，下一步要求用户提供历史数据路径。 |

## 首次上手路径

实际最顺的新用户路径不是默认 `factor-research`，而是：

```bash
cargo check
cargo run -- analyze --symbol DEMO --demo --human
cargo run -- factor-pipeline-debug \
  --symbol DEMO \
  --data examples/demo/demo-15m.json \
  --factor structure_ict \
  --objective expansion_manipulation
cargo run -- factor-research \
  --symbol DEMO \
  --data examples/demo/demo-15m.json \
  --state-dir /tmp/ict-engine-first-run-native \
  --backend native
```

如果目标是“只试 core CLI”，建议 README 把 `--backend native` 放进首轮 demo。Auto-Quant 可以作为第二段“完整研究后端”路径。

## 主要摩擦点

1. **README 的 core CLI 预期和默认 backend 有冲突。** README 写“Rust is enough”，但 `factor-research` 默认 `auto-quant`，会进入外部依赖和数据准备流程。
2. **Auto-Quant 首轮依赖链太早暴露。** 默认链路会拉到 `/tmp/.../.deps/auto-quant` 并推荐 `uv run prepare.py`，但缺 `TA-Lib` 时立即失败。对只想体验 ICT Engine 的用户，这是不必要的第一阻塞。
3. **`--human` 在 `factor-research --backend native` 上表现不符合预期。** 输出是 `Factor research summary: {巨大 JSON}`，不是类似 `analyze --human` 的摘要。
4. **输出中存在中文 mojibake。** Native research 的 JSON 里，中文块名显示为 `åºæ¬...` 这类乱码，影响可信度。
5. **下一步指令有时不适合自动执行。** `analyze --demo --human` 已记录 demo path，却仍提示“Ask the user which dataset to use”。这对 agent-first 流程偏保守，但在 demo smoke 中会打断闭环。
6. **Cargo 并行帮助命令会等待 lock。** 这不是 bug，但 README 的 quick start 若被 agent/用户并行执行，会产生无意义等待日志。

## 做得好的地方

- 顶层 `--help` 覆盖面足，命令命名清楚。
- `analyze --demo --human` 是目前最佳首屏体验：短、可读、有 action 和 next。
- `factor-pipeline-debug` 的诊断字段完整，适合排查 gate、bridge、evidence quality。
- Backtest 对 demo 数据不足的失败信息明确，且 README 事先说明了原因。
- `workflow-status --human` 能把状态压成几行，适合 agent 或人工接着处理。

## 建议改进优先级

**P0: 修正首轮 README 路径。** 把“Rust-only first run”明确为 `analyze --demo --human` + `factor-pipeline-debug` + `factor-research --backend native`。把 Auto-Quant 放到独立章节，说明需要 `uv`、`TA-Lib`、可能的 Docker fallback。

**P1: 修 `factor-research --human`。** 让它输出类似：

```text
DEMO | research | best=trend_momentum | action=Observe | comparable=false
Scores: trend_momentum=0.492 structure_ict=0.492 volatility_mean_reversion=0.052
Block: user_selected_historical_data_missing
Next: choose dataset or rerun with explicit --data and --state-dir
```

**P1: 处理编码问题。** 检查 `agent_prompts.workflow` 或输出序列化路径，确保中文提示按 UTF-8 正常显示；如果 CLI 默认英文优先，也可以把内部 prompt 保持英文，避免 mixed-language 输出。

**P2: 给 demo 模式放宽“ask user dataset”阻塞。** 对 `--demo` 或显式 `examples/demo/demo-15m.json`，可以把 next step 改成可执行命令，并把“生产数据需用户确认”作为 risk note。

**P2: 建议先 build 后跑帮助。** Quick start 可改为：

```bash
cargo build
./target/debug/ict-engine --help
./target/debug/ict-engine analyze --help
```

这样避免每条 `cargo run` 都混入构建日志，也减少并行 lock 噪音。

## 可复现性说明

本次试用只写入项目 docs 文件和 `/tmp/ict-engine-first-run-*` 临时 state，没有修改运行代码。`/tmp/ict-engine-first-run-20260425` 中的 Auto-Quant 依赖由默认 backend 流程创建；`/tmp/ict-engine-first-run-native-20260425` 中保存 native research 的试跑状态。
