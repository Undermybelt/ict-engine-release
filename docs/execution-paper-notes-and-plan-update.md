# Execution 论文笔记与计划更新

论文对象
- 标题：`Who Profits from Prediction? Execution, Not Information`
- 直接 SSRN PDF 链接在当前环境下返回 `403`。
- 因此本笔记基于：
  - 论文标题与公开可得摘要/讨论线索
  - 论文主张的可验证架构含义
  - 与 `ict-engine` 当前代码面交叉比对后的工程判断

结论先行
- 对 `ict-engine` 最重要的不是“再多一个预测层”，而是把：
  - 决策时点
  - 执行可做性
  - alpha capture ratio
  - cost / delay / fill quality
  变成 first-class stateful artifacts。
- 预测仍重要，但在 research OS 中应降级为 `execution decision` 的输入，而非最终主语。
- 这与我们对 `BTC-Trading-Since-2020` 的桶化判断一致：账本原生 execution logic 往往比强行 ICT 化更稳。

---

## 读后感悟

### 1. 赚钱对象不是 forecast，而是 alpha capture
`ict-engine` 现在最强的是：
- HMM / BBN / pre-bayes / factor ranking / hard gate

但这些多数仍偏“是否有方向/状态优势”。

论文主张提醒我们：
- 真正该持久化和比较的对象，不是 `prediction quality`
- 而是 `captured alpha after execution constraints`

对应到本仓，应把以下问题显式化：
- 这个信号是否值得下单？
- 现在下单是否会把 alpha 交给 spread / impact / delay？
- 这个 setup 的 edge 有多少来自方向判断，有多少来自执行条件？

### 2. 方向正确但执行差，不应被判成“系统成功”
若系统只看：
- selected direction
- win probability
- promotion/rollback

则容易把“方向没错但根本不该做”的样本混入正反馈。

更新后的原则应是：
- `prediction_correct != execution_valid`
- execution weak 的 setup，即便方向强，也应降级为 observe / block

### 3. stateful research OS 的核心，是把 execution 变成 versioned evidence
论文启发下，`ict-engine` 不应只保存：
- analyze report
- research report
- reflection bundle

还应保存：
- execution edge share
- execution readiness
- execution drag / capture loss
- execution artifact lineage

否则它仍更像“会记忆的预测引擎”，而不是 execution operating system。

### 4. 物理学层不该只是解释 market state，也该解释 execution feasibility
OU / Ising / Pythagorean 过去更容易被理解为 regime / setup feature。

论文读后更合理的用法是：
- OU：过伸后还能否以可接受代价回归/回踩成交
- Ising：拥挤/同向 herd 是否让执行边际恶化
- Pythagorean：当前离可做区有多远，而不只是“离趋势线多远”

即：物理学层优先服务 execution feasibility，再服务 prediction。

### 5. `BTC-Trading-Since-2020` 的真正价值，不是教 ICT，而是教 execution abstraction
桶化结果：
- ICT wins 19
- native wins 54

这说明：
- 该仓不宜作为主 ICT 因子真相源
- 但它非常适合作为 execution-style factor 的弱监督来源

故本仓应吸收它的方式是：
- 生成 `execution-style priors`
- 生成 `fill/aggression/completion/concentration` 类因子
- 用来校验何时 prediction 该让位于 execution gate

---

## 对原 4 Sprint 计划的修正

### 总修正
原计划方向基本对，但仍偏“把 execution 加进现有预测系统”。

新原则：
- 不是 `Prediction System + Execution Features`
- 而是 `Execution Operating Surface + Prediction as Input`

### Sprint 1 修正
原先重点：
- 加 `ExecutionArtifact`
- 加 OU overextension

修正后应更硬：
- 先建立 `ExecutionFeatures` 与 `ExecutionArtifact`
- 同时在 reflection / report 中强制输出：
  - `execution_edge_share`
  - `prediction_edge_share`
  - `execution_readiness`
- 且 hard gate 先消费 execution_readiness，而非等待 Sprint 4 才接入

### Sprint 2 修正
原先重点：
- 物理学模型注入 regime / PDArray

修正后：
- physics overlay 第一优先服务 `execution feasibility`
- regime 提升是副作用，不是主目标
- 所有 OU / Ising / Pythagorean 输出必须可落入 execution artifact lineage

### Sprint 3 修正
原先重点：
- MECE recovery > 95%

修正后：
- MECE 仍重要
- 但恢复准确率不该成为唯一 truth target
- 还应增加：
  - execution-vs-prediction attribution consistency
  - recovery 高但 execution weak 的样本应被单列，不得自动 promotion

### Sprint 4 修正
原先重点：
- 全量 execution tree 产品化

修正后：
- execution tree 不只是模型集成层
- 它应成为新的主输出 surface
- `--execution-focus` 不应是额外 flag，而应成为默认可读面；保留非 execution 面为 secondary surface

---

## 更新后的实施顺序

### Phase A：先把 execution 做成 canonical surface
优先级高于 physics、MECE、新 voting。

先落：
- `src/domain/execution/*`
- `src/application/execution/*`
- `AnalyzeSupporting.execution_artifact`
- `ReflectionBundle.execution_edge_share/execution_readiness`

### Phase B：让现有 analyze / research / reflection 全部吃到 execution artifact
目标：
- 不改业务主逻辑过多
- 先把 execution attribution 面接好
- 让 repo 进入“可比较 execution artifact”的状态

### Phase C：再把 physics layer 接到 execution，而非先接 regime
- OU first for execution
- Ising second for crowding / execution risk
- Pythagorean third for overextension / feasible entry zone

### Phase D：最后再升级 voting 与 MECE 闭环
只有在 execution surface 先稳定后，这两者才不会继续把 repo 拉回 prediction-first。

---

## 当前代码实现策略

本轮之后开始实现时，遵循：
1. 不把新逻辑塞进 `main.rs`
2. 先做模块：
   - `src/domain/execution/*`
   - `src/application/execution/*`
3. 再做薄接线：
   - `analyze_report_shell.rs`
   - `application/reflection/*`
4. 只做一条最小闭环：
   - build execution artifact
   - attach to analyze report
   - attach to reflection bundle
   - `cargo check` / targeted tests

---

## 新版计划摘要

### Sprint 1（修订版）
- 目标：Execution surface first-class 化
- 验收：analyze/research/reflection 均出现 execution artifact / execution edge split

### Sprint 2（修订版）
- 目标：physics-for-execution，而非 physics-for-explanation
- 验收：OU/Ising/Pythagorean 进入 execution lineage

### Sprint 3（修订版）
- 目标：MECE + execution consistency 双约束
- 验收：恢复率与 execution validity 同时可审计

### Sprint 4（修订版）
- 目标：execution tree 成为主 workflow surface
- 验收：workflow / reflection / artifact ledger 默认执行 execution-first triage

---

## 最终判断
- 论文没有推翻 `ict-engine` 现有 HMM/BBN/factor 闭环。
- 它推翻的是系统的优先级排序。
- 新排序应为：
  1. execution feasibility
  2. execution attribution
  3. prediction value
  4. regime explanation

如果 `ict-engine` 按此更新，它才会从“有状态的预测研究系统”真正转成“stateful execution research OS”。
