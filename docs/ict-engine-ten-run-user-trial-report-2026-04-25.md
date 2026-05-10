# ICT Engine 十次用户向试跑报告

**日期:** 2026-04-25  
**仓库:** `/Users/thrill3r/projects-ict-engine/ict-engine`  
**执行视角:** 作为第一次/早期使用者逐步试跑  
**运行状态目录:** `/tmp/ict-engine-ten-run-user-trial-20260425/state`

## 结论先说

当前仓的用户体验存在一个 **P0 级问题**：源码在当前状态下 `cargo build` 直接失败。

这意味着：

- 新用户按 README 或常规 Rust 习惯从源码启动时，会在第一步被挡住。
- 我后续能继续观察 CLI，只是因为本机刚好残留了一个旧的 `./target/debug/ict-engine` 二进制。
- 因此，后续第 2-10 次运行反映的是 **“坏源码 + 旧二进制兜底”** 的降级体验，不代表 fresh clone 用户真的能走到那里。

## 试跑方法

- 在仓根目录逐步运行 10 条命令。
- 尽量贴近用户会尝试的路径：构建、help、demo analyze、research、workflow、backtest、auto-quant status。
- 所有运行时写入都放到 `/tmp`。
- 不改代码，不安装系统依赖，不接真实数据。

## 十次实际运行

| # | 命令 | 结果 | 用户视角结论 |
|---|---|---|---|
| 1 | `cargo build` | **失败**，约 86.6s | 首个阻塞。等待很久后才报编译错误，fresh user 无法继续。 |
| 2 | `./target/debug/ict-engine --help` | 成功 | 仅因本机残留旧二进制。命令面完整，但这不是 fresh build 成功后的正常路径。 |
| 3 | `./target/debug/ict-engine analyze --help` | 成功 | 帮助面还算清楚。 |
| 4 | `./target/debug/ict-engine factor-research --help` | 成功 | 帮助面明确写了默认 backend 是 `auto-quant`。 |
| 5 | `./target/debug/ict-engine analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-ten-run-user-trial-20260425/state --human` | 成功，约 0.14s | 可读性不错，但 `Next` 用了 `ict-engine ... --state-dir <local-path>`，不可直接复制执行。 |
| 6 | `./target/debug/ict-engine factor-pipeline-debug --symbol DEMO --data examples/demo/demo-15m.json --factor structure_ict --objective expansion_manipulation` | 成功，约 0.01s | 信息很强，但默认输出非常厚，首屏不自解释。 |
| 7 | `./target/debug/ict-engine factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-ten-run-user-trial-20260425/state --backend native --human` | 成功，约 0.20s | 现在人类输出已明显改善，但 `Next` 仍不可直接执行。 |
| 8 | `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-ten-run-user-trial-20260425/state --human` | 成功，约 0.14s | 有价值，但 headline 冗长，且同一屏里出现潜在矛盾信号。 |
| 9 | `./target/debug/ict-engine backtest --symbol DEMO --data examples/demo/demo-15m.json --human --state-dir /tmp/ict-engine-ten-run-user-trial-20260425/state` | **失败**，约 0.07s | 这是合理边界，错误信息清楚。 |
| 10 | `./target/debug/ict-engine auto-quant-status --state-dir /tmp/ict-engine-ten-run-user-trial-20260425/state` | 成功，约 0.04s | 状态完整，但对首次用户过于底层、过于路径导向。 |

## 第 1 次运行的关键阻塞

`cargo build` 当前失败，输出中可见的主要错误集中在 `src/application/reporting/analyze_output.rs`，包括未解析到的函数/符号，例如：

- `build_analyze_live_output_envelope`
- `build_analyze_output_envelope`
- `trim_analyze_output_workflow_snapshot_ledgers`

在真正的用户视角里，这个问题比任何 help、文案、默认值都更优先，因为它直接让 fresh start 失效。

## 我遇到的主要不便

### 1. Fresh start 直接坏掉

这是最大的摩擦。

用户正常预期是：

```bash
cargo build
./target/debug/ict-engine --help
```

但当前第一步就失败，且失败前有较长等待与不少 warning 噪音，用户要花时间从编译日志里捞出真正的错误。

### 2. CLI 还能跑，只是因为本机有陈旧二进制

这会带来一个很不健康的体验分叉：

- 新用户：卡死在构建
- 老用户：可能误以为一切正常，因为旧二进制还能运行

这会掩盖源码与实际运行体已经分叉的事实。

### 3. `Next` 命令不可直接执行

在第 5、7、8 次运行里，`--human` 输出都给了类似：

```text
Next: ict-engine factor-research ... --state-dir <local-path> ...
```

这里有两个问题：

- `ict-engine` 假设二进制已经在 `PATH` 里，但当前用户实际是在 repo 内直接跑 `./target/debug/ict-engine`
- `--state-dir <local-path>` 被占位符替换后，命令变成了**不可复制执行**的半成品

这对 agent-first / command-following 场景尤其差，因为看起来像“下一步已经准备好了”，实际并不能直接跑。

### 4. 路径处理前后不一致

- `--human` 面为了脱敏，把真实路径变成 `<local-path>`
- 但 `auto-quant-status` 的 JSON 又完整打印了大量绝对路径

结果是：

- 人类摘要面过度脱敏，牺牲了可执行性
- 机器状态面又过度底层，首轮用户看起来负担很重

当前的路径策略不统一。

### 5. `workflow-status --human` 的 headline 太吵

第 8 次运行首行是：

```text
DEMO | research | pass_neutralized | pda_cluster=unavailable | duration=unavailable | remaining_bars=unavailable | spectral_entropy=unavailable | sparsity=unavailable
```

问题在于：

- 作为人类摘要，字段太多
- 多个 `unavailable` 会淹没真正重要的状态
- 第一眼很难知道用户该关心什么

相比之下，`analyze --human` 的摘要就更聚焦。

### 6. `workflow-status --human` 同屏信号有潜在冲突

第 8 次运行里同时出现：

- headline: `pass_neutralized`
- `Latest: research | gate=pre_bayes_gate_unavailable`

对普通用户来说，这看起来像“到底 gate 是过了还是 unavailable”。

即使底层语义可能分别来自不同层或不同 phase，这种呈现方式仍会造成混淆。

### 7. `factor-pipeline-debug` 很强，但首屏不可扫读

这条命令的内容很丰富，适合深度排障。

但用户首次运行时会遇到：

- JSON 非常厚
- 关键字段没有被置顶强调
- 要靠 README 才知道该先看 `evidence_quality_score`、`gating_status`、`bridge_gap`

也就是说，它是个很好的“原始诊断面”，但不是很好的“第一屏诊断面”。

### 8. `factor-research --help` 暗示默认还是 `auto-quant`

第 4 次运行里，help 明确写着：

- `--backend <BACKEND>`
- `Research backend: auto-quant (default) or native [default: auto-quant]`

这意味着首次用户若没仔细读更细的 onboarding，很容易理解为：

- 研究功能的正常默认路径就是 `auto-quant`

但从首轮体验来说，最稳的路径其实还是 `native`。默认值和最顺手路径之间仍存在张力。

### 9. `auto-quant-status` 对首次用户过于底层

第 10 次运行成功了，但输出像一个内部状态对象，包含：

- managed dir
- workspace 多个绝对路径
- dependency status
- required files
- recommended next command meta

对于已经在排查依赖的高级用户，这很好。

但对第一次点开它的用户，信息负担偏重；而且它直接把用户引向一个外部依赖 bootstrap 流程，在当前仓 fresh build 已坏的前提下，会进一步放大不确定性。

## 我认为“不合理输出”的地方

### 1. `Next` 看起来像“可执行下一步”，但实际不是

这是目前最明显的不合理输出之一。

如果一个命令显示 `Next:`，用户自然会理解为：

- 可以复制
- 可以马上跑
- 和当前运行方式兼容

但当前输出既不保留真实 `state-dir`，又默认用户已经把 `ict-engine` 放进 PATH，这会让“下一步”失去它应有的产品意义。

### 2. `workflow-status --human` 里过多 `unavailable`

对人类摘要面来说，一长串 unavailable 更像内部诊断泄漏，不像面向用户的“当前该看什么”。

### 3. 同屏 gate 信号不一致感

即使底层并不冲突，用户看到 `pass_neutralized` 和 `pre_bayes_gate_unavailable` 同时出现，直觉上就会认为输出没有对齐。

### 4. Fresh build 失败前 warning 太多

从用户体验上，warning 在错误前堆很多行，会弱化真正阻塞点。

这不是最核心的功能 bug，但它会放大“第一次编译就出问题”时的挫败感。

## 合理边界，不应误报为 bug

以下两点我认为是**合理失败**，文档也基本能解释：

- `backtest` 用 demo 数据失败：`got 52, require at least 71`
- `factor-pipeline-debug` 默认很重：它本来就是偏诊断面的命令

它们不是本次最核心的问题。

## 做得好的地方

也有几条体验是明显好的：

- `analyze --demo --human` 很像一个真正可用的首屏摘要
- `factor-research --backend native --human` 已经比之前自然很多
- `backtest` 边界报错足够直白
- 顶层 help 的命令覆盖面完整

换句话说，产品表层已经有一些很好的用户面，但目前被 **fresh build 失败** 和 **next-step 可执行性不足** 这两类问题压住了。

## 建议优先级

### P0

- 修复当前源码编译失败，恢复 fresh build。
- 在 fresh build 恢复前，不要把 repo 描述成可直接从源码按常规路径启动。

### P1

- 让 `Next:` 真正变成可执行命令：
  - 保留可运行路径，或
  - 明确区分“示意命令”和“可复制命令”
- 统一路径策略：
  - human 面不要把命令关键参数脱敏到不可执行
  - machine 面也不要在首次体验里默认灌太多路径细节
- 压缩 `workflow-status --human` 首屏，只保留最关键 2-4 个信号。

### P2

- 给 `factor-pipeline-debug` 一个更薄的首屏摘要，先置顶：
  - `evidence_quality_score`
  - `gating_status`
  - `bridge_gap`
  - `pipeline_verdict`
- 重新评估 `factor-research` 默认 backend 与首轮 onboarding 的一致性。

## 一句话总结

如果把这十次试跑当成真实用户体验，那么当前最准确的判断是：

**`ict-engine` 的人类摘要面已经有一些不错的雏形，但当前仓在 fresh build 上是坏的，而且“下一步命令”仍未达到真正可复制执行的产品标准。**
