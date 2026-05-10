# 2026-04-25 auto-quant seeded strategy backtest report

## Scope

本报告记录一次从 `ict-engine` / `auto-quant` 集成继续向前推进的真实实验：

- 不停在 handoff / readiness
- 直接在 Auto-Quant workspace 里生成可回测策略
- 运行 `uv run run.py`
- 根据结果至少做一轮策略迭代

关联文档：

- `docs/2026-04-25-auto-quant-parallel-try-plan.md`
- `docs/2026-04-25-auto-quant-parallel-try-report.md`

## Process correction

前一阶段只验证到了：

- `ict-engine` 可以 bootstrap / prepare / handoff 到 Auto-Quant
- Auto-Quant workspace 可以进入 `ready_for_external_run`

但这还不够。

真正目标导向的下一步应该是：

- 由 agent 主动创建初始策略
- 使用 `run.py` 回测
- 再根据结果迭代

本报告就是对那一缺口的补做。

## Workspace and branch

本次实验工作区：

- `/tmp/ict-engine-auto-quant-shared`

本次实验分支：

- `autoresearch/apr25-cascade`

已初始化：

- `results.tsv`
- `user_data/strategies/*.py`

## Inputs used

策略研究入口仍复用上一轮 live analyze 的上下文，但 Auto-Quant 自身回测使用的是它的固定数据面：

- pairs:
  - `BTC/USDT`
  - `ETH/USDT`
  - `SOL/USDT`
  - `BNB/USDT`
  - `AVAX/USDT`
- timeframes:
  - `1h`
  - `4h`
  - `1d`
- timerange:
  - `20230101-20251231`

## Strategy design basis

## Historical repository evidence used

在写首版策略前，先读取了：

- `program.md`
- `README.md`
- `versions/0.1.0/retrospective.md`
- `versions/0.2.0/retrospective.md`
- `versions/0.3.0/retrospective.md`
- `versions/0.2.0/strategies/*.py`
- `versions/0.3.0/strategies/*.py`

从这些材料中提炼出的约束是：

- 不走 `exit_profit_only` / ROI clipping 这类已知 Goodhart 路线
- 优先复用 v0.3.0 已验证有效的 MTF / cross-pair 模式
- 不把 1h Donchian 弱 breakout、ADX 滞后确认这类已知弱结构硬塞回来
- 首轮先用已知正边际策略作为 seed，而不是从零发明高不确定方案

## External research used

本次还补了一轮公开研究检索。

主要采用了两条可落地启发：

- `arXiv:2106.08420` `Trend-Following Strategies via Dynamic Momentum Learning`
  - 启发：趋势速度在不同阶段可能需要动态切换，而不是固定单一速度
- `arXiv:2101.01006` `Design and analysis of momentum trading strategies`
  - 启发：趋势/动量策略的收益更依赖“让赢家运行”的结构，而不是过早剪掉收益分布

其中第一条被落实为一次实际策略迭代；结果见下文。

## Seeded strategies

本次实际创建了 3 个起始策略：

- `user_data/strategies/BTCLeaderBreakX.py`
- `user_data/strategies/MTFTrendStack.py`
- `user_data/strategies/VolBBSqueeze.py`

三者分别覆盖：

- breakout
- trend-following
- volatility

## Round 1 results

执行：

```bash
uv run run.py > run.log 2>&1
```

### BTCLeaderBreakX

- Sharpe: `1.0716`
- Profit: `120.67%`
- Max DD: `-8.8241%`
- Trades: `347`
- Profit factor: `1.8568`

按 per-pair 观察：

- 5 个交易对全部为正收益
- ETH / SOL / AVAX 更强
- BTC 自身 Sharpe 相对低，但仍为正

这是本次实验最重要的事实：

**只要 agent 主动 seed 策略并真正回测，Auto-Quant 当前工作区里已经能直接得到一个 clean-edge 非常强的候选策略。**

### MTFTrendStack

- Sharpe: `0.6128`
- Profit: `44.81%`
- Max DD: `-12.5262%`
- Trades: `410`
- Profit factor: `1.3363`

按 per-pair 观察：

- SOL 明显最强
- BTC 较弱甚至接近负边缘
- ETH / BNB / AVAX 贡献一般

结论：

- aggregate 是正边际
- 但 pair-robustness 不够好
- 更像“某些币种上的结构性趋势策略”，还不是全宇宙一致强策略

### VolBBSqueeze

Round 1 未得到有效指标，原因不是策略逻辑，而是运行时网络异常：

- `ccxt.base.errors.RequestTimeout`
- `binance GET https://api.binance.com/api/v3/exchangeInfo`
- 最终表现为 FreqTrade 的 `OperationalException`

因此 Round 1 时不能把 `VolBBSqueeze` 记作策略失败，只能记作环境噪声。

## Round 2 evolution

## Action taken

对 `MTFTrendStack` 做了一次基于外部研究的定向改动：

- 当 4h 趋势很强时，用更快的 `ema9` reclaim 入场
- 当 4h 趋势较中等时，用更慢的 `ema21` reclaim 降噪
- 同时加了一个轻量 `rsi` 上限，减少后段追高

这是把 `2106.08420` 的“动态动量速度”启发压进了可回测规则。

## Round 2 results

执行：

```bash
uv run run.py > run2.log 2>&1
```

### BTCLeaderBreakX

- Sharpe: `1.0716`
- 与 Round 1 一致
- 继续稳居最优策略

### MTFTrendStack after dynamic-entry experiment

- Sharpe: `0.3662`
- Profit: `26.37%`
- Max DD: `-8.7392%`
- Trades: `332`
- Profit factor: `1.2437`

相对 Round 1：

- `0.6128 -> 0.3662`
- 退化明显
- BTC / ETH / BNB 进一步变弱
- SOL 仍有贡献，但无法弥补整体退化

结论：

**这条 literature-inspired 改动在当前 5-pair / 1h+4h+1d 设定上不成立，应回滚。**

这点很重要：

- 外部论文检索是有价值的
- 但启发不能直接当真理
- 必须经回测筛掉不适配当前宇宙的数据结构改动

### VolBBSqueeze

Round 2 成功运行，得到：

- Sharpe: `0.6979`
- Profit: `43.04%`
- Max DD: `-9.4025%`
- Trades: `299`
- Profit factor: `1.4622`

这直接证明：

- Round 1 的失败是网络噪声
- 不是策略逻辑有问题
- `VolBBSqueeze` 本身是一个健康的正边际 volatility 策略

## Round 3 rollback

由于动态改动明确拖累 `MTFTrendStack`，因此进行了选择性回滚：

- 恢复到原始 archived reclaim 逻辑
- 撤掉动态快慢入场实验

执行：

```bash
uv run run.py > run3.log 2>&1
```

结果：

- `BTCLeaderBreakX` 继续稳定输出 `1.0716`
- `VolBBSqueeze` 继续稳定输出 `0.6979`
- `MTFTrendStack` 本轮未完成验证，因为再次遇到 Binance `exchangeInfo` timeout

因此对 `MTFTrendStack` rollback 的判断依据主要来自：

- Round 1 的原始 baseline
- Round 2 的明确退化
- 以及当前文件内容已恢复为 baseline 版本

## Final portfolio status

截至本次实验结束，工作区里最有价值的策略状态是：

### Lead

- `BTCLeaderBreakX`
- Sharpe `1.0716`
- 5-pair 全部正收益
- 当前最强赚钱面

### Secondary

- `VolBBSqueeze`
- Sharpe `0.6979`
- 是健康的第二范式

### Conditional / needs more work

- `MTFTrendStack`
- baseline 有正边际
- literature-inspired 动态入场改动无效，已回滚
- 后续如果再研究，应该针对 pair asymmetry，而不是简单改 entry speed

## What this proves

本次实验明确证明了下面几件事：

### 1. Earlier stopping point was wrong

停在“没有策略文件”是不对的。

因为一旦 agent 真的去生成 seed strategies 并运行回测，系统立刻就能产出可用结果，甚至直接出现：

- Sharpe `1.0716`
- 全 5 pair 正收益

的强候选。

### 2. The missing piece was not the runtime

此前缺的不是：

- bootstrap
- prepare
- data
- handoff

缺的是：

- **agent 主动生成初始策略**
- **agent 主动执行回测并继续迭代**

### 3. The current handoff/program surface is under-specified for a control-plane integration

虽然 `program.md` 在 Auto-Quant 仓里已经写了“agent 应该创建 1-3 starting strategies”，但 `ict-engine` 侧当前 handoff surface 还不够强，至少没有把以下意图变成硬约束：

- 当 `user_data/strategies/` 为空时，agent 必须先 seed strategies
- 不能把“无策略文件”当成任务结束
- 应优先读取 archived winners / retrospectives，再决定 seed set
- 在有外部研究启发时，必须通过回测验证而不是口头采用

## Practical recommendation

如果后续要把这条链真正做成更强的“agentic auto research”体验，最小改动建议是：

### 1. Strengthen the handoff prompt

至少增加这些硬规则：

- no strategy files -> create 2-3 seeds immediately
- first seeds should preferentially come from archived winners or their minimal descendants
- first full run must happen before asking for more human guidance
- literature search can guide mutation, but every such mutation must be experimentally accepted or rejected

### 2. Preserve the current top seed set

当前最合理的 active set 是：

- `BTCLeaderBreakX`
- `MTFTrendStack` baseline
- `VolBBSqueeze`

### 3. Next research target

下一个最值得研究的不是再动 `BTCLeaderBreakX`，而是：

- 继续改善 `MTFTrendStack` 的 pair robustness
- 重点解决 BTC / ETH / BNB 的弱边际问题
- 避免再次使用“看起来聪明但实际拖 Sharpe”的动态速度切换

## Files produced or updated in this experiment

Auto-Quant workspace:

- `results.tsv`
- `user_data/strategies/BTCLeaderBreakX.py`
- `user_data/strategies/MTFTrendStack.py`
- `user_data/strategies/VolBBSqueeze.py`
- `run.log`
- `run2.log`
- `run3.log`

Repo docs:

- `docs/2026-04-25-auto-quant-seeded-strategy-backtest-report.md`

## Final answer

如果问题是：

**“agent 能不能别等人提醒，自己生成、回测、迭代出能赚钱的策略？”**

这次实验的答案是：

**能，而且应该这么做。**

本次已经实际产出：

- 一个强 lead strategy：`BTCLeaderBreakX`，Sharpe `1.0716`
- 一个稳健 secondary strategy：`VolBBSqueeze`，Sharpe `0.6979`
- 一次被实验否定并回滚的 literature-inspired 改动：`MTFTrendStack` dynamic entry

这比停在“没有策略文件”要接近真实目标得多。
