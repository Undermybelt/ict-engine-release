# Auto-Quant → ICT Engine 吸收计划（审校版）

> 不做 1:1 移植；围绕现有 `factor-autoresearch` 吸收实验账本、研究回顾与异常优化防护。

具体代码设计见：`docs/plans/2026-04-23-autoresearch-derived-surfaces-design.md`

---

## 一、审核摘要

### 1.1 上一版的失真点

- 把“直接移植核心逻辑”写成目标，但当前 repo 并不采用 commit/worktree 驱动的研究循环
- 把“跨 git 持久化”写成缺失项，但当前 `state` / `state*` 已被 `.gitignore` 忽略，JSON 状态天然独立于 tracked code
- 把不存在的 CLI 写成既定方案：
  - `factor-autoresearch --output-format tsv`
  - `factor-autoresearch-status --retrospective`
  - `factor-autoresearch --detect-gaming`
- 直接照搬 `commit / sharpe / max_dd` 账本字段，但当前 autoresearch 真正稳定可用的是：
  - `session_id`
  - `attempt_id`
  - `decision.status`
  - `score_before / score_after / score_delta`
  - `metrics_after.aggregate_return`
  - `failure_tags`
  - `branch_summary`

### 1.2 校准后的迁移结论

Auto-Quant 值得吸收的不是它的极简回测器或单文件架构，而是三件事：

1. **追加型实验账本**
2. **基于历史 state 的 retrospective**
3. **对“可疑提升”的元目标/目标完整性预警**

---

## 二、repo reality：当前可直接复用的锚点

### 2.1 命令与状态真相

| 面 | 当前锚点 | 说明 |
|---|---|---|
| 迭代执行 | `factor-autoresearch` | 已支持 keep/discard、多轮迭代、`--resume-latest`、`--session-id` |
| 真相读取 | `factor-autoresearch-status` | 已输出 session、attempts、`effective_status`、`interrupted`、`cluster_scoreboard`、`best_attempt` |
| 逐轮记录 | `factor_autoresearch_attempts.json` | 最细粒度的 attempt truth |
| 会话汇总 | `factor_autoresearch_sessions.json` | session 级 summary |
| 运行快照 | `factor_autoresearch_live.json` | 运行中 / 中断判定基础 |
| 完成产物 | `factor_autoresearch_final.json` | 正常完成后的 final summary |

### 2.2 当前真正缺失的能力

| 能力 | 当前状态 | 是否应做 |
|---|---|---|
| 人类友好的账本导出 | 缺失 | **是，P0** |
| retrospective 文档 | 缺失 | **是，P1** |
| 可疑提升 warning 层 | 缺失 | **是，P1** |
| 单文件极简架构 | 不存在 | **否，非目标** |
| commit 驱动实验循环 | 不存在 | **否，非目标** |

### 2.3 指标映射原则

如果要吸收 Auto-Quant 模式，必须先做指标翻译：

- Auto-Quant 的 `Sharpe` 不应直接等价为 ict-engine 的 `best_factor_composite_score`
- Auto-Quant 的 `max_dd` 当前不在 autoresearch attempt truth 中稳定暴露
- ict-engine 的初版账本应优先使用**现有稳定字段**，避免为迁移文档虚构 schema

---

## 三、吸收范围与阶段

### 3.1 Phase 1：实验账本导出（P0，1 周）

**目标**：
- 保留 JSON truth
- 增加适合 `tail` / `grep` / diff 的 `experiments.tsv`

**推荐字段**：

```
timestamp	session_id	attempt_id	base_factor	mutation_id	decision_status	score_before	score_after	score_delta	aggregate_return_after	top_factor	failure_tags	branch_summary
```

**设计原则**：
- `experiments.tsv` 是 **derived artifact**
- 不替代 `factor_autoresearch_attempts.json`
- 不要求引入 commit hash；若未来真做 git/worktree 集成，再增加 `source_revision` 可选列

**建议实现落点**：
- 首选：在 autoresearch attempt 追加后同步刷新 / 追加导出
- 备选：写成独立 exporter，从 attempts JSON 派生

**验收标准**：
- 行数可与 attempts 数对齐
- 字段可回溯到真实 JSON
- 不改变现有 stdout JSON surface

### 3.2 Phase 2：研究回顾导出（P1，1 周）

**目标**：
- 从现有 state 自动生成 Markdown retrospective

**最低内容要求**：
- session 概览
- `effective_status` / `interrupted`
- keep / discard 比例
- score 轨迹
- 高频 `failure_tags`
- `cluster_scoreboard`
- `best_attempt`
- 下一轮建议关注的 mutation 方向

**建议实现路线**：
- 优先复用 `factor-autoresearch-status` 已聚合的信息
- 优先扩展现有 script/reporting surface
- 不先发明新的主 CLI 选项

**验收标准**：
- 每个结论都能回链到 attempts / status JSON
- interrupted/completed 判定与 status 命令一致
- retrospective 是纯派生产物

### 3.3 Phase 3：目标完整性预警（P1，1-2 周）

**目标**：
- 给 autoresearch 增加“可疑提升” warning 层
- 第一版只做 warning，不自动 rollback，不改 keep/discard 主决策

**首批候选 heuristics**：
- `score_delta` 异常跳升
- `score_after` 提升但 `aggregate_return` 不配套
- keep 了一个带显著 regression / `failure_tags` 恶化的 attempt
- cluster 表现过度集中，疑似模板投机
- 只在某个脆弱 surface 上抬分，但总体研究面没有同步改善

**验收标准**：
- warning 可写入 attempt / final summary 的派生输出
- 阈值可调
- 不影响已有 session 恢复语义

### 3.4 Phase 4：窄适配，而不是架构照搬（P2）

仅在前 3 个阶段跑通后，再评估是否需要：

- 更强的 human-readable ledger
- 更丰富的研究阶段自动分段
- LLM 对 retrospective 的摘要层

**不进入此阶段的内容**：
- `AutoResearch.rs` 单文件模式
- 不可编辑固定框架 + git reset 主流程
- 以 commit 为主轴的 keep/discard 实验管理

---

## 四、落点设计

### 4.1 产物落点

| 目标 | 建议落点 | 当前锚点 |
|---|---|---|
| TSV 账本 | `state_dir/<symbol>/experiments.tsv` | `factor_autoresearch_attempts.json` |
| retrospective | `state_dir/<symbol>/factor_autoresearch_retrospective.md` | `factor-autoresearch-status` 聚合输出 |
| warning surfaces | attempt summary / final summary 的派生字段 | `FactorAutoresearchAttempt` / `FactorAutoresearchSummary` |

### 4.2 模块落点

| 目标 | 建议位置 | 原因 |
|---|---|---|
| 账本生成 | `src/state/` 附近或相邻 exporter | 最接近 state schema |
| 回顾导出 | `scripts/` 或 reporting 层 | 先做派生，不污染主执行命令 |
| warning 判定 | `factor_autoresearch_command` 附近 | 最靠近 keep/discard 决策形成处 |

### 4.3 命名约束

后续文档必须区分：

- **已存在命令**
  - `factor-autoresearch`
  - `factor-autoresearch-status`
- **候选产物**
  - `experiments.tsv`
  - `factor_autoresearch_retrospective.md`
- **不得直接假定存在的 CLI**
  - `--output-format tsv`
  - `--retrospective`
  - `--detect-gaming`

---

## 五、实现约束

1. **JSON truth 优先**
   - `factor_autoresearch_attempts.json` 仍是最终真相
2. **不改主输出契约**
   - 第一阶段不要求改变 `factor-autoresearch` 当前 stdout JSON
3. **先派生，后内建**
   - 能从现有 state 派生的东西，先别加重主命令职责
4. **指标命名要忠实**
   - 没有 Sharpe，就不要把 composite score 直接写成 Sharpe
5. **warning-only first**
   - 先给 warning，不直接 auto rollback

---

## 六、验收标准

### 6.1 功能验收

1. **TSV 账本**
   - 能从真实 attempts 产物生成
   - 行数 / decision 计数与 status 输出一致
   - 支持快速人工浏览

2. **retrospective**
   - 能输出 session 级 Markdown 回顾
   - 包含 completed / interrupted 真相
   - 包含 best attempt / cluster / failure tag 统计

3. **目标完整性预警**
   - 可标出 suspicious attempt
   - 不直接覆盖 keep/discard
   - warning 来源字段可追溯

### 6.2 完整性验收

- 不把派生文件当 source of truth
- 不要求用户进入 commit-driven workflow
- 不新增与当前实现不符的伪 CLI 文案
- 文档中所有字段都能在 repo 现有 schema 中找到，或明确标注为 candidate

---

## 七、风险与缓解

### 风险 1：TSV 与 JSON 漂移

**缓解**：
- 以 attempts JSON 为唯一输入源生成 TSV
- 不在两个地方各自维护同一份事实

### 风险 2：warning 误报

**缓解**：
- 第一版只做 warning
- 阈值可配置
- 保留命中原因，便于复盘

### 风险 3：retrospective 叙事过度脑补

**缓解**：
- 每条结论尽量引用 attempt id / session id / count
- 先做统计回顾，再做摘要语言润色

---

## 八、时间表

| 阶段 | 任务 | 时间 |
|------|------|------|
| Phase 1 | `experiments.tsv` 派生账本 | 1 周 |
| Phase 2 | retrospective 导出 | 1 周 |
| Phase 3 | warning-only 目标完整性预警 | 1-2 周 |
| **总计** | | **3-4 周** |

---

## 九、总结

Auto-Quant 对 ict-engine 的高价值输入不是“极简四文件架构”，而是：

1. **实验账本**
2. **回顾文档**
3. **对可疑优化的防漂移意识**

ict-engine 已经具备 autoresearch 的核心 state 基座；当前要补的是 **human-readable derived surfaces**，而不是再造一套新的研究执行内核。

---

*审校时间：2026-04-22*
*版本：reviewed / reality-aligned*