# ict-engine 模块化重构计划：以领域边界、纯计算、显式路由替代漂移拼接

> For Hermes: 只抽取用户给出的架构原则，不照搬示例项目/语言/交易领域名词。使用正约束措辞。优先真实模块提取、单一所有权、显式数据流、可测试边界。让 `main.rs` 持续收缩为 dispatch/orchestration only。

**Goal:** 以可迁移的架构原则重构 ict-engine：把“数据组装 / 领域状态 / 纯因子计算 / BBN 证据构建 / 策略分流与提交”拆开，消灭隐形漏水、静默过滤、跨层混算、main.rs 持续膨胀。

**Architecture:** 建立四层主干：规范领域模型 -> 防腐/装配层 -> 纯计算引擎 -> 策略/路由层。任何跨模块传递的核心对象，都用强类型结构承载；任何“是否通过/是否进入下一阶段”的判定，只在显式策略层完成；任何“算不全”的对象都保留壳体与缺失原因，而非中途消失。

**Tech Stack:** 维持 Rust 现栈；使用现有 `state/*`、`bbn/*`、`factor_lab/*`、`ict/*` 子树；通过结构体、枚举、模块导出面收敛、针对性测试推进。

---

## 从用户 prompt 抽取出的可迁移原则

1. 强类型领域模型先行。
2. 上游输入先过防腐层，再进入领域。
3. 纯计算只负责算，不负责判死刑。
4. 路由/策略层统一做分流、阈值、提拔、提交前判断。
5. API/CLI/状态输出，只序列化结构体，不手搓散乱映射。
6. 缺字段、弱证据、低质量，不得静默消失；要显式留下壳体、标签、原因。
7. 正约束：要求模块“产出完整对象、保留原始真相、显式打标签、单一职责”，而非鼓励“尽早过滤、就地跳过、边算边判”。

---

## 对 ict-engine 的直接映射

Python 示例中的：
- Domain Models → ict-engine 的状态/快照/证据/报告结构体
- Anti-Corruption Layer → 各类 parse/build/assemble helper
- Pure Calculation Engine → factor / ict / bbn inference 的纯函数面
- Policy & Router → pre-bayes gate / workflow phase / promotion / execution candidate 相关逻辑

换言之：
不是照搬 `OrderBook`、`GeminiContract`；
而是把 ict-engine 里的 `FrameFeatures`、`FactorDiagnostics`、`PreBayesEvidenceFilter`、`ExpansionBbnSupport`、`ExecutionCandidateArtifact` 等，收束成真正的 canonical domain surface。

---

## 当前问题定义

从本仓已见症状看，漂移主要来自：

1. `main.rs` 同时承担：
   - 输入解析
   - 多时间框装配
   - factor 运行
   - pre-bayes evidence build
   - timed PDA 摘要拼装
   - BBN 推断
   - workflow/status 组装
   - prompt/context 构造
   - artifact 决策与历史持久化

2. 同一类概念在多处重复出现，形成“隐形漏水”：
   - 先 build filter
   - 再局部补 timed PDA 字段
   - 再另一路把 evidence 单独转换
   - 再由别处决定 gate/status
   于是很容易出现：字段新增后，某条链透传了，另一条链没透传。

3. 纯计算与策略判定混杂：
   - 某函数既算分布，又顺手决定是否 observe_only
   - 某报告 builder 既做 evidence 汇总，又做 promotion hint
   - 某状态结构既是领域对象，又夹杂展示/命令推荐语义

4. 输出对象过晚成型：
   - 许多信息先以局部变量漂浮
   - 临近 CLI/API 输出时才被拼成报告结构
   - 中途任何一步少接一根线，就会“看似编译过，实际字段失真”

---

## 重构总目标

### 目标 A：领域模型单一所有权

为关键阶段建立 canonical 结构体，且每类结构体有唯一建造入口。

候选核心模型：
- `TimedPdaSummary` / 复用 `ICTStructureSummary`
- `PreBayesEvidenceFilter`
- `PreBayesEvidencePacket`（建议新增）
- `ExpansionFactorPipelineCore`（建议新增）
- `WorkflowRoutingDecision`（建议新增）
- `ExecutionCandidateArtifact`
- `PendingUpdateArtifact`

原则：
- 一类对象，一个 canonical builder。
- 字段新增后，先改 canonical builder，再让下游只消费此对象。

### 目标 B：防腐/装配层显式化

把上游杂乱输入统一装配后，再交给纯引擎。

这里的“上游”包括：
- candle/native frames
- multi_timeframe summary
- factor output diagnostics
- timed PDA states
- market-specific overrides
- prior history / policy history / learning state

建议新增装配层对象：
- `FactorPipelineInputs`
- `PreBayesAssemblyInputs`
- `WorkflowAssemblyInputs`

其职责：
- 收齐原材料
- 做无业务倾向的标准化
- 不做 pass/skip 决策

### 目标 C：纯计算引擎只产完整壳体

关键纯函数只做：
- 输入强类型
- 输出强类型
- 不做跨阶段过滤
- 算不全则显式给 `Option`/reason/tag

例如：
- `build_pre_bayes_evidence_filter(...) -> PreBayesEvidenceFilter`
- `build_pre_bayes_evidence_packet(...) -> PreBayesEvidencePacket`
- `infer_trade_support_from_packet(...) -> BbnInferenceSnapshot`
- `build_expansion_pipeline_core(...) -> ExpansionFactorPipelineCore`

铁律：
- 不在纯计算层决定“是否值得提交实战”
- 不在纯计算层夹带推荐命令
- 不在纯计算层静默抛弃低质量对象

### 目标 D：策略/路由层统一判定

所有“是否进入下一阶段”的逻辑，统一放到 router/policy 模块：
- pre-bayes gate 映射
- observe_only / pass_neutralized / pass_hard
- promote / hold / rollback / submit
- best-near-miss 类提拔逻辑

建议新增：
- `WorkflowRouteLabel`
- `WorkflowRouteReason`
- `route_pipeline_core_to_workflow(...)`
- `route_artifacts_for_update(...)`

这样：
- 纯对象先完整产出
- 再统一分流
- 每个分流结果都有原因码

---

## 建议模块边界

### 1. `src/ict/` 继续只管 ICT 结构事实

保留职责：
- timed PDA states
- liquidity pools / FVG / OB / CISD / sweeps
- 结构摘要

正约束：
- 只产结构事实
- 不产 workflow 决策
- 不产 CLI 展示语句

### 2. `src/factor_lab/` 只管因子信号与诊断

保留职责：
- factor engine
- diagnostics
- factor contributions
- backtest support

正约束：
- 只产因子级 signal/diagnostics
- 不直接写 pre-bayes gate
- 不直接写 execution candidate 决策

### 3. `src/bbn/` 只管 evidence/inference/topology

应收拢职责：
- canonical evidence packet -> evidence map
- entry_quality/trade_outcome inference
- network topology

建议新增/提炼：
- `bbn::evidence::PreBayesEvidencePacket`
- `bbn::trading::update::trade_evidence_from_packet(...)`

正约束：
- 只接受标准化 packet
- 不依赖 main.rs 局部变量拼装
- 不关心最终 CLI 报告布局

### 4. `src/state/` 只管持久化结构与历史记录

保留职责：
- persistence
- run records
- snapshots
- artifacts

正约束：
- state types 反映 canonical domain/result objects
- 不承担临时拼装逻辑

### 5. `src/workflow/` 或 `src/router/`（建议新增）

新增职责：
- pre-bayes gate routing
- promotion / rollback / update readiness
- analyze/update/train/research handoff 决策

这是本次最关键新增层。

### 6. `src/main.rs`

最终仅保留：
- CLI args parse
- 调 orchestration/service
- 输出序列化结果

正约束：
- main.rs 不再直接拼接大对象内部字段
- main.rs 不再同时做计算与路由
- main.rs 不再持有大量阶段性局部变量

---

## 推荐新增 canonical 对象

### `PreBayesEvidencePacket`

目的：把“filter + timed PDA + canonical assignments + raw traces”收成一个对象，成为 BBN 输入唯一载体。

建议字段：
- `filter: PreBayesEvidenceFilter`
- `evidence_assignments: BTreeMap<String, String>`
- `timed_pda_summary: ICTStructureSummary`
- `raw_market_regime_trace: FactorPipelineLabelSource`
- `raw_liquidity_context_trace: FactorPipelineLabelSource`
- `raw_multi_timeframe_resonance_trace: FactorPipelineLabelSource`
- `missing_inputs: Vec<String>`
- `assembly_notes: Vec<String>`

价值：
- 避免“filter 一部分字段在这补，另一部分 evidence 在别处算”
- BBN 层只吃 packet

### `ExpansionFactorPipelineCore`

目的：把“factor 输出 + packet + inference snapshot”封成单一中间结果。

建议字段：
- `factor_name`
- `latest_signal`
- `probability_support`
- `pre_bayes_packet`
- `entry_quality_snapshot`
- `trade_outcome_snapshot`
- `selected_direction`
- `selected_win_probability`
- `completeness_status`
- `missing_fields`

价值：
- 先形成完整壳体，再由 workflow/router 决定是否 promote/submit/observe

### `WorkflowRoutingDecision`

建议字段：
- `route_label`
- `reason_codes`
- `promotion_status`
- `execution_readiness`
- `recommended_next_stage`

价值：
- 所有判定集中，便于 prior/postmortem 复盘

---

## 明确禁止的旧模式

1. 在 `main.rs` 中对领域对象字段逐个临时补线。
2. 某个 builder 先产半成品，另一路函数再“顺手补齐”。
3. 在 evidence build 过程中顺便做 workflow 分流。
4. 用散落的 `let xxx = ...` 横跨数百行后，最后一次性拼报告。
5. 字段算不出来就直接不进对象。
6. 新增字段时，只改最终输出结构，不改 canonical builder。

---

## 正约束版实现策略

### Phase 1：先固化 canonical surface

任务：
1. 找出现在重复组装的核心对象。
2. 为每类对象指定唯一所有者模块。
3. 新增 `packet/core/route` 三层中间对象。
4. 先不追求大改功能，只先把数据流收束。

验收：
- timed PDA 五字段不再靠 main.rs 局部补线才能存在
- BBN 只从 canonical packet 取证据

### Phase 2：抽离 pure builders

任务：
1. 从 `main.rs` 提取：
   - `build_pre_bayes_evidence_packet`
   - `build_expansion_factor_pipeline_core`
   - `build_workflow_routing_decision`
2. 每个函数只接受强类型输入。
3. 每个函数只返回强类型输出。

验收：
- main.rs 每次减少一个大块逻辑
- cargo check/test 持续全绿

### Phase 3：集中 route / policy

任务：
1. 把 `observe_only/pass_*`、promotion、rollback、submission readiness 收拢到新模块。
2. 把“推荐命令 / 推荐动作”从纯计算对象剥离到 route output 或 presenter。

验收：
- 任何分流都能给 reason code
- prior 验证与事后反思有统一着陆点

### Phase 4：输出层瘦身

任务：
1. CLI/status/workflow 只消费 structured objects。
2. 消灭重复 JSON/summary 拼装路径。
3. presenter 只做展示，不反向影响 domain state。

验收：
- 输出结构变化时，不需要回头改纯计算逻辑

---

## 与用户 10 项要求的对齐

1. 轻量化
- 通过单一 canonical 对象减少重复拼装与重复转换。

2. agent 友好
- 模块边界清晰；每个 agent 只改一层。

3. token 友好
- packet/core/route 三层固定后，后续讨论可直接围绕对象名，而非重复贴长流程。

4. 引导友好
- presenter/router 分离后，可分别产内部调试面与人类友好面。

5. 有数据源
- 输入装配层显式承接 candles/native frames/history。

6. 有历史数据回测路径
- factor/backtest 与 packet/core 结构统一后，研究/回测/线上更易共用。

7. 有因子创建/回测/迭代/进化/提交实战能力
- workflow/router 层可把这些阶段性动作统一显式化。

8. 有实战判断时效性
- readiness 与 route label 集中后，可直接挂 live freshness checks。

9. 有 prior 验证与事后反思
- route reason codes + packet snapshot 自带 prior/postmortem 接口。

10. 最终走向无痛迭代 BBN
- BBN 只吃 canonical packet；新增证据字段只改 packet builder 与 BBN mapping。

---

## 测试策略

### 单测

1. `build_pre_bayes_evidence_packet`：
- 完整输入
- 缺一部分输入
- timed PDA 为空
- market override 存在/不存在

2. `trade_evidence_from_packet`：
- hard evidence
- soft evidence
- timed PDA summary 驱动 entry_quality

3. `build_expansion_factor_pipeline_core`：
- 完整对象必生成
- 缺字段时仍生成壳体，并显式 missing fields

4. `route_pipeline_core_to_workflow`：
- observe_only
- pass_neutralized
- pass_hard
- promotion/rollback reasons

### 回归测试

把这类历史惨案固化：
- 新字段已加入 filter，但未加入 evidence mapping
- 某条路径补了 timed PDA，另一条路径没补
- main.rs 重构后 imports/re-exports 漂移
- CLI 输出看似有值，state snapshot 却没持久化

---

## 预期首批文件改动方向

优先候选：
- `src/bbn/evidence.rs`
- `src/bbn/trading/update.rs`
- `src/state/types.rs`
- `src/main.rs`
- 新增 `src/workflow/` 或 `src/router/`

如果继续推进，首刀应是：
1. 提炼 `PreBayesEvidencePacket`
2. 让 `trade_evidence_from_pre_bayes_filter` 升级/替换为 packet-based surface
3. 把 `build_expansion_factor_pipeline_report*` 中重复的 timed PDA + filter + evidence 组装块抽走

---

## 实施纪律

1. 每次只抽一块 canonical builder。
2. 先让测试锁住，再移动代码。
3. 若新增字段，顺序固定：
   - state/domain struct
   - canonical builder
   - consumer mapping
   - presenter/output
   - tests
4. 每轮后执行：
   - `cargo fmt`
   - `cargo check`
   - targeted tests
   - `cargo test`


---

## 下一步执行建议

立刻按此计划开做，但不全盘乱拆。

最优起手式：
1. 新增 `PreBayesEvidencePacket`
2. 把现有两处 factor pipeline timed-PDA 组装重复块抽到 packet builder
3. 让 BBN inference 只消费 packet
4. 再抽 workflow router

一句话：
这次不是照搬 Python；是把它的“强模型 + 防腐层 + 纯计算 + 统一路由”翻译成 ict-engine 的 Rust 模块化路线。