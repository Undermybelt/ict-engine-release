# 2026-04-25 auto-quant parallel try report

## Scope

本报告记录 2026-04-25 对 `ict-engine` 中 `auto-quant` 相关入口的并发试跑结果。

相关计划见：

- `docs/2026-04-25-auto-quant-parallel-try-plan.md`

输入数据复用上一轮 live analyze 产物：

- LTF: `/tmp/ict-engine-live-validate-20260425/NQ/analyze_live_20260425T100923_ltf.json`
- paired spot: `/tmp/ict-engine-live-validate-20260425/NQ/analyze_live_20260425T100923_spot.json`

## Executive summary

结论分三层：

### 1. `ict-engine` -> `auto-quant` 集成已验证成立

`ict-engine` 当前已经能把 `auto-quant` 相关流程推进到以下阶段：

- dependency status
- bootstrap managed checkout
- readiness surface
- data prepare
- factor-research / factor-autoresearch handoff artifact persistence
- artifact ledger registration

### 2. 并发策略可行

本次采用：

- **shared managed checkout**
  - `/tmp/ict-engine-auto-quant-shared`
- **isolated state dirs**
  - `/tmp/ict-engine-aq-r-expansion`
  - `/tmp/ict-engine-aq-r-generic`
  - `/tmp/ict-engine-aq-ar-expansion`
  - `/tmp/ict-engine-aq-ar-generic`
  - 以及对应 `-ready` 复跑目录

实践证明这种方式可行：

- shared checkout 只需 bootstrap / prepare 一次
- 多条 `ict-engine` research / autoresearch 入口可并发扇出
- 每条线通过独立 `state-dir` 避免 handoff/ledger 冲突

### 3. 真正的当前阻塞点不是依赖，也不是数据，而是“没有策略”

`prepare.py` 成功后：

- `auto-quant-status` 进入 `dependency_ready_data_ready`
- 但 `uv run run.py` 仍失败
- 失败原因是：`user_data/strategies` 下没有可运行策略文件，仅有模板

所以当前边界非常清楚：

**`ict-engine` 已经把 Auto-Quant 集成推进到 external run ready；真正未闭环的是策略生成 / 注入，不是 bootstrap、依赖或历史数据准备。**

## What was tried

本次把“都试试”落实为 4 条并发线：

### Research

- `factor-research --backend auto-quant --objective expansion_manipulation`
- `factor-research --backend auto-quant --objective generic`

### Autoresearch

- `factor-autoresearch --backend auto-quant --objective expansion_manipulation`
- `factor-autoresearch --backend auto-quant --objective generic`

说明：代码中 `ResearchObjectiveMode` 当前只有两个目标：

- `generic`
- `expansion_manipulation`

## Execution log

## Phase 1: initial readiness

首先检查：

```bash
./target/debug/ict-engine auto-quant-status --state-dir /tmp/ict-engine-live-validate-20260425
```

结果：

- `status = missing_dependency`
- `bootstrap_needed = true`
- 推荐命令：`ict-engine auto-quant-bootstrap --state-dir ...`

## Phase 2: bootstrap shared checkout

执行：

```bash
ICT_ENGINE_AUTO_QUANT_DIR=/tmp/ict-engine-auto-quant-shared \
./target/debug/ict-engine auto-quant-bootstrap --state-dir /tmp/ict-engine-aq-bootstrap
```

结果：

- bootstrap 成功
- shared repo URL:
  - `https://github.com/TraderAlice/Auto-Quant.git`
- 当前 commit:
  - `d143ee67871be35fcadfe8a010020b483f230469`
- required files 均存在：
  - `README.md`
  - `program.md`
  - `prepare.py`
  - `run.py`
  - `versions/README.md`

## Phase 3: four parallel handoff runs before prepare

4 条线全部成功进入 `auto-quant` handoff integration。

共同结果特征：

- dependency 已健康
- `data_ready = false`
- readiness 为：`dependency_ready_data_missing`
- 统一推荐下一步：
  - `uv run /tmp/ict-engine-auto-quant-shared/prepare.py`

说明：

在这一步，`ict-engine` 不是直接跑完 Auto-Quant 本体，而是：

- 构建 handoff payload
- 持久化到 state
- 给出下一步 external workspace command

## Phase 4: run shared `prepare.py`

执行：

```bash
uv run prepare.py
```

结果：

- `uv` 可用：`uv 0.6.14`
- 依赖安装成功
- `prepare.py` 成功下载 5 个交易对 × 3 个周期，共 15 份 feather 数据
- 最终输出：`Ready.`

目标交易对：

- `BTC/USDT`
- `ETH/USDT`
- `SOL/USDT`
- `BNB/USDT`
- `AVAX/USDT`

目标周期：

- `1h`
- `4h`
- `1d`

实际落盘目录：

- `/tmp/ict-engine-auto-quant-shared/user_data/data`

已确认存在 15 份数据文件，例如：

- `BTC_USDT-1h.feather`
- `BTC_USDT-4h.feather`
- `BTC_USDT-1d.feather`
- `ETH_USDT-1h.feather`
- `AVAX_USDT-1d.feather`

## Phase 5: readiness after prepare

再次执行：

```bash
ICT_ENGINE_AUTO_QUANT_DIR=/tmp/ict-engine-auto-quant-shared \
./target/debug/ict-engine auto-quant-status --state-dir /tmp/ict-engine-aq-r-expansion
```

结果变为：

- `status = dependency_ready_data_ready`
- `healthy = true`
- `data_ready = true`
- 推荐下一步：
  - `uv run /tmp/ict-engine-auto-quant-shared/run.py`

这说明 `ict-engine` 对 Auto-Quant 的 readiness 判断链路已经被成功推进到“可执行研究”状态。

## Phase 6: try `run.py`

执行：

```bash
uv run run.py
```

结果失败，但失败原因非常具体：

```text
ERROR: no strategies found in /private/tmp/ict-engine-auto-quant-shared/user_data/strategies.
Create at least one `.py` file under user_data/strategies/ (see user_data/strategies/_template.py.example for the skeleton).
```

随后检查目录确认：

- `user_data/strategies` 下仅有：
  - `_template.py.example`

这证明：

- `run.py` 已可进入 runtime
- 失败不是因为缺依赖
- 失败不是因为缺数据
- 失败点是**没有实际策略文件**

## Phase 7: rerun the 4 handoff lines after prepare

在 `data_ready = true` 之后，对 4 条线再次复跑。

结论：

- 4 条线全部成功进入 `ready` 分支
- `readiness.status = dependency_ready_data_ready`
- `recommended_next_command = uv run /tmp/ict-engine-auto-quant-shared/run.py`
- 对应 artifact ledger 状态为：`ready_for_external_run`

## Artifact evidence

## Shared Auto-Quant workspace

- checkout root:
  - `/tmp/ict-engine-auto-quant-shared`
- data dir:
  - `/tmp/ict-engine-auto-quant-shared/user_data/data`
- strategies dir:
  - `/tmp/ict-engine-auto-quant-shared/user_data/strategies`

## Ready handoff artifacts

### Research / expansion_manipulation

- handoff file:
  - `/tmp/ict-engine-aq-r-expansion-ready/NQ/auto_quant_handoff.factor_research.json`
- ledger file:
  - `/tmp/ict-engine-aq-r-expansion-ready/NQ/artifact_ledger.json`
- ledger status:
  - `ready_for_external_run`

### Research / generic

- handoff file:
  - `/tmp/ict-engine-aq-r-generic-ready/NQ/auto_quant_handoff.factor_research.json`

### Autoresearch / expansion_manipulation

- handoff file:
  - `/tmp/ict-engine-aq-ar-expansion-ready/NQ/auto_quant_handoff.factor_autoresearch.json`
- ledger file:
  - `/tmp/ict-engine-aq-ar-expansion-ready/NQ/artifact_ledger.json`
- ledger status:
  - `ready_for_external_run`

### Autoresearch / generic

- handoff file:
  - `/tmp/ict-engine-aq-ar-generic-ready/NQ/auto_quant_handoff.factor_autoresearch.json`

## What the handoff payload now proves

ready 阶段的 handoff payload 已经明确包含：

- `backend = auto-quant`
- `objective`
- `data_path`
- `paired_data_path`
- `workspace.run_script`
- `readiness.status = dependency_ready_data_ready`
- `suggested_commands` 包含：
  - `python3 .../program.md`
  - `uv run .../run.py`

这说明 `ict-engine` 在 Auto-Quant 集成上的职责已经非常明确：

- 维护 dependency status / readiness
- 管理 handoff artifact
- 把 live/analyze 侧的数据路径交给 external Auto-Quant workspace
- 作为 control plane 记录可审计状态

## Important boundary

本次实验确认：

### 已验证成功的部分

- `ict-engine auto-quant-status`
- `ict-engine auto-quant-bootstrap`
- `factor-research --backend auto-quant`
- `factor-autoresearch --backend auto-quant`
- shared Auto-Quant dependency bootstrap
- `uv run prepare.py`
- readiness 由 `data_missing` 推进到 `data_ready`
- 4 条 handoff 线全部进入 `ready_for_external_run`

### 当前尚未闭环的部分

- strategy generation / materialization
- Auto-Quant backtest loop 的真实策略评估输出
- candidate package 回灌 `ict-engine` state

## Small artifact consistency note

观察到一个轻微一致性问题：

- stdout 打印的 payload 中包含 `handoff_artifact_path`
- 但持久化后的 handoff JSON 文件里该字段仍为空字符串

不过这不影响主要事实判断，因为：

- ledger 已记录真实文件路径
- handoff 文件本身已被正确落盘
- readiness / workspace / objective / data_path 等关键信息完整存在

## Final answer

如果把你的要求理解为：

**“Auto-Quant 能并发尝试的入口都去试一遍，并把能推进到哪一层跑清楚。”**

那么这次已经试清楚了：

- **都试了**
- **并发策略可行**
- **shared checkout + isolated state-dir 是正确打法**
- **`ict-engine` 已经把 Auto-Quant 集成推进到 ready_for_external_run**
- **真正缺的是策略文件，不是依赖、不是数据、也不是 handoff 逻辑**

## Recommended next step

如果还要继续把闭环再往前推，最小下一步不是再跑 `prepare.py`，而是：

1. 往 `/tmp/ict-engine-auto-quant-shared/user_data/strategies/` 放入至少一个真实策略 `.py`
2. 再执行：

```bash
uv run /tmp/ict-engine-auto-quant-shared/run.py
```

只有这样才能真正看到 Auto-Quant backtest loop 的产出，并继续验证 candidate package 是否能回流到 `ict-engine`。
