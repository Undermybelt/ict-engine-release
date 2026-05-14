# ICT Engine 外部项目融合优化计划（审校版）

> 以仓库现状为真：先吸收已验证的外部模式，再决定是否做架构级接入。

---

## 一、审核结论

### 1.1 当前文档的主要问题

- 把 **已存在能力** 写成了缺失能力：
  - `factor-autoresearch` 已有 session / attempt / live snapshot / final summary 持久化
  - `factor-autoresearch-status` 已能汇总 `effective_status`、`interrupted`、`decision_counts`、`cluster_scoreboard`、`best_attempt`
  - `state` / `state*` 已被 `.gitignore` 忽略，默认不会被 `git reset --hard` 回滚
- 把 **候选方案** 写成了已设计好的 CLI：
  - `factor-autoresearch --output-format tsv` 尚不存在
  - `factor-autoresearch-status --retrospective` 尚不存在
- 混淆了 **Auto-Quant 指标** 与 **ict-engine 指标**：
  - Auto-Quant 以 `commit / sharpe / max_dd / status` 为主轴
  - ict-engine 当前更直接可用的是 `score_before / score_after / score_delta / aggregate_return / failure_tags / branch_summary`

### 1.2 重新校准后的结论

| 外部来源 | 真正可吸收的价值 | 与 repo 现状的关系 | 结论 |
|------|------|------|------|
| Auto-Quant | 研究账本、回顾、异常优化防护 | 与 `factor-autoresearch` 高度相邻 | **优先吸收** |
| backtrader | 组合/多资产回测表达能力 | 当前 `FactorBacktestEngine` 仍是单资产主路径 | **观察，暂缓大改** |
| AITD | LLM 决策叙事与资产筛选概念 | repo 里暂无稳定决策 contract 可直接挂接 | **仅保留概念，不立刻接入** |

---

## 二、repo reality：当前已经存在什么

### 2.1 已落地能力

- `factor-autoresearch`
  - 多轮 keep/discard 迭代
  - `--resume-latest`
  - `--session-id`
  - `--max-cluster-fail-streak`
- 状态产物
  - `factor_autoresearch_attempts.json`
  - `factor_autoresearch_sessions.json`
  - `factor_autoresearch_live.json`
  - `factor_autoresearch_final.json`
- 真相读取面
  - `factor-autoresearch-status --latest-only`
  - 输出 `effective_status`、`interrupted`、`decision_counts`、`failure_tag_counts`、`cluster_scoreboard`、`best_attempt`
- 持久化边界
  - 默认 `state` / `state*` 在 `.gitignore` 中
  - JSON 状态文件已经天然独立于被追踪代码文件

### 2.2 真实缺口

1. **人类友好的实验账本缺口**
   - 当前 JSON 适合程序消费，不适合快速 `tail` / `grep`
2. **研究回顾缺口**
   - 当前有原始 attempt/session truth，但没有自动生成 retrospective
3. **异常收益/目标漂移防护缺口**
   - 当前没有显式的 suspicious jump / gaming warning 层
4. **外部集成优先级治理缺口**
   - 旧文档把 Auto-Quant、backtrader、AITD 并列推进，实际应该分级

---

## 三、整合策略：先吸收模式，不做 1:1 移植

### 3.1 Auto-Quant：立即吸收

**建议吸收点**：

1. **追加型实验账本**
   - 在保留 JSON source of truth 的前提下，新增衍生 `experiments.tsv`
   - 重点是“便于扫描”，不是替代 JSON

2. **研究回顾输出**
   - 从已有 attempts / sessions / live / final summary 派生 Markdown retrospective
   - 回顾至少覆盖：
     - keep/discard 比例
     - score 轨迹
     - failure tag 聚类
     - cluster jump 表现
     - best attempt 与 interrupted session

3. **异常优化预警**
   - 不是照搬 “Sharpe jump”
   - 应基于 repo 当前指标定义：
     - `score_delta` 异常跳升
     - `aggregate_return` 与高分不一致
     - regression / failure tag 恶化但被 keep
     - cluster 表现过度集中

**不建议吸收点**：

- commit 驱动的一轮一提交工作流
- “只有一个可编辑策略文件”的极简架构
- 直接把 `results.tsv` 作为唯一真相

### 3.2 backtrader：先列为观察项

**可以保留的方向**：

- 多资产组合回测
- 组合级指标
- 仓位 / 权重层

**但当前不应直接立项为 P0 的原因**：

- `FactorBacktestEngine` 当前主路径仍是单资产 candle 序列
- repo 已有多时间框架与 paired-data 面，不等于缺乏所有“多输入能力”
- 一旦上组合层，会连带改变：
  - backtest schema
  - metrics surface
  - state artifacts
  - research / release verdict logic

**结论**：

- backtrader 价值存在
- 但应在 Auto-Quant 的 P0 / P1 完成后，再以单独设计文档推进

### 3.3 AITD：仅保留概念，不直接接入

**目前只保留两个可参考概念**：

- LLM 生成“研究建议 / 解释”
- 动态资产池作为未来输入层

**当前不直接推进的原因**：

- repo 里尚无稳定、收敛的 LLM 决策 contract
- 若直接接，会把“研究执行”和“叙事生成”混在一起
- 没有明确的 external adapter / prompt contract / failure handling surface

**结论**：

- 先不把 AITD 写进近期开发表
- 等 Auto-Quant 回顾层成熟后，再评估是否给 retrospective 增加 LLM 摘要层

---

## 四、优先级重排后的实施计划

### Phase 1：Auto-Quant 账本导出（P0，1 周）

目标：
- 在不改变现有 JSON 真相层的前提下，补一个可快速浏览的实验账本

建议字段：
- `timestamp`
- `session_id`
- `attempt_id`
- `base_factor`
- `mutation_id`
- `decision_status`
- `score_before`
- `score_after`
- `score_delta`
- `aggregate_return_after`
- `top_factor`
- `failure_tags`
- `branch_summary`

验收：
- 能从现有 `factor_autoresearch_attempts.json` 无损导出
- 与 `factor-autoresearch-status` 的 decision 计数对齐
- 明确标注“derived artifact, not source of truth”

### Phase 2：Auto-Quant retrospective（P1，1-2 周）

目标：
- 从既有 state 生成可读回顾，而不是依赖人工翻 JSON

输出至少包含：
- session 概览
- keep / discard 比例
- best attempt
- cluster scoreboard
- 高频 failure tags
- interrupted / completed 判定
- 下一轮建议关注方向

验收：
- 同一 session 的回顾结论可回溯到 attempts / status JSON
- 不新增新的 truth file，只新增派生报告

### Phase 3：异常优化预警（P1，1-2 周）

目标：
- 在 autoresearch 里增加“可疑提升”标记，但先不自动回滚

初版只做 warning，不做 hard gate：
- suspicious score jump
- score 与 `aggregate_return` 不一致
- keep decision 与 regression / failure 信息冲突
- cluster 过拟合式集中

验收：
- warning 能落到 session / attempt summary
- 不改变现有 keep / discard 主流程
- 误报可通过阈值调整

### Phase 4：portfolio / LLM 层再评估（P2，后置）

进入条件：
- Auto-Quant P0 / P1 落地后仍存在明确研究瓶颈
- 已能证明单资产研究面不足以回答现有问题

---

## 五、明确非目标

- 不做 Auto-Quant 回测器的 1:1 移植
- 不把 `experiments.tsv` 设为唯一真相源
- 不在当前阶段引入新的虚构 CLI 选项
- 不把 backtrader 多资产层与 AITD LLM 层并行推进
- 不把 commit / worktree 管理写成当前 repo 的既有能力

---

## 六、文档落地约束

后续凡是写到外部集成，必须满足：

1. **先写落点**
   - 具体落到哪个命令、状态文件、脚本或模块
2. **区分已存在与候选**
   - 已存在：直接写真实命令 / 真实文件
   - 候选：必须标注 `candidate`
3. **JSON truth 不变**
   - `factor_autoresearch_attempts.json` 等仍是权威产物
4. **验收以现有状态面为准**
   - 先看 `factor-autoresearch-status`
   - 再看衍生 TSV / retrospective

---

## 七、下一步

1. 两份文档已完成审校并对齐到 repo reality
2. 具体代码设计已落到：`support/docs/plans/2026-04-23-autoresearch-derived-surfaces-design.md`
3. 若进入实现，优先做：
   - TSV 导出
   - retrospective 导出
   - warning-only 的异常提升检测
4. backtrader 与 AITD 暂停在“观察 / 概念”层，不进入并行开发
5. 后续新增外部集成内容时，继续遵守本文件的“真实锚点优先”约束

---

## 八、结论

这轮外部融合的**唯一近程高价值方向**是 Auto-Quant 的研究工作流模式。
backtrader 与 AITD 仍有参考价值，但目前都不应和 Auto-Quant 放在同一优先级推进。

---

*审校时间：2026-04-22*
*版本：reviewed / reality-aligned*

=====
https://github.com/joshyattridge/smart-money-concepts
https://github.com/ranaroussi/quantstats
结论
这俩里，quantstats 比 smart-money-concepts 更值得考虑，但前提是你把它放在可选的派生报告层，不是核心运行时。

smart-money-concepts
有参考价值
不值得直接集成为主依赖
最适合当：算法对照物 / 回归校验器 / 缺口启发源
quantstats
有实际价值
但只适合：离线分析 / HTML tearsheet / 风险可视化
不适合直接进入 ict-engine 的核心回测指标语义
我看到的 repo reality
从 README.md、src/ict/*.rs、src/application/backtest/*、src/backtest/*、support/docs/external-integration-plan.md 看，ict-engine 现在是：

Rust-first 的 ICT 交易分析/研究 CLI
已经有一套自己的 ICT 结构识别
已经有自己的 回测与结果汇总
当前更缺的是 人类友好的派生表面，不是“完全没有指标/结构识别”
已经存在的 ICT 能力
你这仓里已经有这些模块：

FVG
src/ict/fvg.rs
Swing High/Low
src/ict/swing.rs
BOS / CHoCH
src/ict/bos_choch.rs
Order Block
src/ict/ob.rs
Liquidity / Sweep
src/ict/liquidity.rs
Premium / Discount
src/bayesian/premium_discount.rs
更多 PDA/ICT 概念位
EqualHighsLows
OptimalTradeEntry
Ndog
Nwog
OpenRangeGap
SwingFailurePattern
这些在 src/types.rs 和 src/ict/pda_state.rs 里都有体现
已经存在的回测/指标能力
你也已经有：

交易级回测
src/backtest/engine.rs
基础指标
sharpe
max_drawdown
win_rate
profit_factor
在 src/backtest/metrics.rs
运行期报告 / equity curve / recent trades / regime metrics
src/application/backtest/runtime_report.rs
src/backtest_report_shell.rs
而且当前文档明确写了近程重点是：

Auto-Quant 风格的派生账本
retrospective
warning-only integrity layer
也就是：现在 P0/P1 不是再接一个外部分析引擎，而是把现有 truth surface 做成人能读的派生产物。

对 smart-money-concepts 的判断
为什么它“有点用”
它 README 里的能力包括：

fvg
swing_highs_lows
bos_choch
ob
liquidity
previous_high_low
sessions
retracements
其中前五项，你仓里基本都已经有了。

它真正可能补到你的点，主要是这几个：

previous_high_low
我没在仓里看到同名公开 surface
sessions / kill zone 风格输出
你仓里有 session 时区逻辑，见 src/data/loader.rs
但不像 smc 那样直接做成一个通用 indicator surface
retracements
这块我没看到现成显式实现
某些定义细节可拿来对照
比如 join_consecutive FVG
mitigation index
它的 OHLCV 输出字段设计
为什么它“不值得直接集成”
核心原因是 重复度高 + 语义层级偏低 + Python 形态不贴合。

[重复度高]
你已经有 FVG / OB / BOS / CHoCH / liquidity / swing
[层级偏低]
smc 更像一组 DataFrame indicators
你的 repo 已经在做：
多时间框架
概率推理
policy / factor research
PDA state lifecycle
所以它更像“底层 indicator 包”，不是能直接补上中高层架构的东西
[接入形态不贴合]
smc 期待 pandas DataFrame
你主路径是 Rust CLI
目前 Python 在这个 repo 里只是 辅助脚本 和 单独 bridge
例如 src/python_bridge/timesfm.rs 是明确的外挂桥，不是主路径
一个很关键的细节
你仓里有些更高级概念虽然“有名字”，但看实现上未必是 dedicated detector。

比如在 src/ict/pda_state.rs 里：

OpenRangeGap
Ndog
Nwog
SwingFailurePattern
这些是从 fvgs.first() 派生 band 再封装成状态的，不像 smc 那样是独立 indicator 定义。

这意味着：

如果你想把这些概念做得更“定义驱动”
smc 可以当 参考规范/对照实现来源
但这仍然更像：

adapt / borrow ideas
不是 adopt as dependency
我的结论
结论标签：adapt
推荐用法：
作为对照实现
作为测试 oracle
作为缺失 surface 的启发
不推荐：
直接把它接进主 runtime
为它引入一整套 pandas-first 依赖流
对 quantstats 的判断
为什么它有用
quantstats 真正强的是：

现成的 performance/risk stats
plots
HTML tearsheet
Monte Carlo
CVaR / tail / ulcer / rolling stats
而你当前 repo 在报告层明显还比较“工程态”：

JSON payload 很完整
human output 有
但没有成熟的 HTML / 图形化报告层
所以它能补的不是“回测会不会跑”，而是：

结果更容易看
风险画像更完整
对外展示更成熟
为什么不能直接塞进核心指标面
它 README 自己写得很清楚：

QuantStats analyzes return series
not discrete trade data
这跟你现在 repo 的主语义有个明显错位：

你很多指标是交易级
TradeRecord
win_rate(trades)
profit_factor(trades)
runtime_report 里的 equity_curve
也是从 trade pnl 序列 累乘出来的
不是天然的日频/周频 mark-to-market return series
这会导致一个问题：

同名指标语义不完全一致
比如：

你现在的 win_rate = 交易胜率
QuantStats 的 win_rate = 正收益周期占比
如果你不先做一层时间序列 returns adapter，直接接进去，用户很容易被误导。

接入成本也不低
quantstats 需要的 Python 侧比较重：

Python >= 3.10
pandas
numpy
scipy
matplotlib
seaborn
tabulate
yfinance
optional plotly
而你的 repo 当前：

没有根级 pyproject.toml
没有成体系的 Python 环境管理
Python 主要还是辅助脚本，不是主运行时
所以它更适合作为：

可选脚本
导出后生成报告
研究笔记本/HTML 报告器
不适合作为：

主 CLI 的硬依赖
回测真相层的一部分
我的结论
结论标签：adopt as optional derived surface
推荐用法：
从 BacktestReport 或 state artifact 导出一份 time-indexed return series
再用单独 Python 脚本生成：
metrics
plots
html tearsheet
不推荐：
直接拿它替换你现有的 backtest metrics
直接把 trade-level PnL 当成它的 period return 输入
优先级建议
如果按“现在对 ict-engine 有没有用”排序，我会这样排：

[第一] quantstats
但只限 可选报告层
[第二] smart-money-concepts
但只限 参考/校验/局部移植
[更高优先级] 都低于你当前文档里的 Auto-Quant 派生表面工作
也就是 experiments.tsv
retrospective
suspicious uplift warnings
我给你的实操建议
[如果你想马上用一个]
先用 quantstats
但做成 support/scripts/export_quantstats_report.py 这种外围工具
[如果你想提升 ICT 定义严谨度]
再拿 smart-money-concepts 做对照
优先看：
retracements
previous_high_low
sessions
FVG / OB / liquidity 的边界定义
[如果你只想避免分心]
这俩现在都不是 P0
先做你文档里已经明确的 derived surfaces 更值
一句话版
smart-money-concepts：有用，但更多是“借算法定义/做对照”，不值得直接接进主系统。
quantstats：有用，而且比前者更实用，但应该只放在离线报告层，不要污染核心回测语义。
状态
已完成对 ict-engine 与这两个外部仓的调研对比，未修改代码。如果你要，我下一步可以直接帮你把这份结论落成一份 repo 内的 support/docs/ 评估文档，或者进一步给出一个最小接入方案。

