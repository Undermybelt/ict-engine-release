# Anti-drift skill research notes

目标：为 Hermes 设计一个“避免项目架构/主文件/模块边界持续漂移”的本地 skill，吸收外部方法，不盲信外部 doctrine，不默认安装重依赖。

## 外部高价值参考（吸收，不照搬）

### 1. Import Linter
- 仓库：https://github.com/seddonym/import-linter
- 价值：把模块边界写成 contracts，并在 CI fail
- 吸收点：
  - 边界声明应是可执行规则，不是散文
  - 允许层/禁止跨层依赖应有机械检查
- 不照搬点：
  - Python import 语义不适合直接照抄到 Rust/main.rs-heavy 仓

### 2. ArchUnit
- 仓库：https://github.com/TNG/ArchUnit
- 价值：architecture as tests
- 吸收点：
  - 架构规则要写成测试/fitness functions
  - 防漂移不能靠 review 记忆
- 不照搬点：
  - JVM 生态假设不可直接移植

### 3. Nx enforce-module-boundaries
- 文档：https://nx.dev/features/enforce-module-boundaries
- 价值：tag + constraint + graph
- 吸收点：
  - 给模块/子系统打标签
  - 按标签定义依赖矩阵
- 不照搬点：
  - JS/TS monorepo 特定工具面

### 4. Dylint / Rust custom lint 路线
- 仓库：https://github.com/trailofbits/dylint
- 价值：Rust 原生可执行架构规则载体
- 吸收点：
  - 最终应把高价值 anti-drift 规则下沉为 lint/test/checker
- 不照搬点：
  - 先别一上来造复杂 lint；先用 repo artifact + tests + CI

### 5. guppy / cargo graph analysis 路线
- 仓库：https://github.com/guppy-rs/guppy
- 价值：Rust workspace 依赖图分析
- 吸收点：
  - anti-drift skill 应鼓励 crate/module graph 巡检与边界报告
- 不照搬点：
  - 不把外部库变成 skill 硬依赖

## 对 agent/AI coding 特别重要的方法
- plan-first, no code before boundary/risk review
- allowed change surface / patch budget
- contract tests / behavior tests > snapshot patches
- module ownership / local module readme / ADR references
- debug artifact before bugfix
- architecture fitness functions

## 推荐新 skill
- 名称：`architecture-drift-guard`

## 触发条件
- 大型 `main.rs` / God file 持续膨胀
- 反复出现 API drift / struct initializer drift / call-site drift
- 用户提到：漂移、架构腐蚀、边界不清、模块提取失败、越改越乱
- 需要在实现前先设边界、计划、验证与回退点

## 核心步骤（草案）
1. 先判 drift 类型
   - file truncation drift
   - API signature drift
   - struct initializer drift
   - boundary erosion
   - workflow/output surface drift
2. 产 repo artifact
   - `support/docs/architecture-boundaries.md`
   - `support/docs/drift-ledger.md`
   - `support/docs/change-surface.md`
3. 定 allowed change surface
   - 本次允许改哪些文件/模块
   - 禁止跨哪些边界
4. 建 fitness checks
   - compile/test search guard
   - forbidden dependency edges
   - required artifact updates
5. 强制 debug evidence
   - `DEBUG.md`
   - failing command
   - root cause note
6. 实施最小变更
7. 验证
   - fmt/check/test
   - search for banned patterns / old APIs
   - if needed graph/LOC report
8. 若形成稳定流程，则把检查下沉为脚本/test/lint

## 需要落地到 repo 的 artifacts
- `support/docs/architecture-boundaries.md`
- `support/docs/drift-ledger.md`
- `support/docs/change-surface.md`
- `DEBUG.md`
- optional: `support/scripts/check_architecture_drift.py`
- optional: `tests/architecture_drift_guards.rs`

## 与现有 skills 的差异
- 不同于 `systematic-debugging`
  - 那是查 bug 根因
  - 这个是管“边界与漂移治理”
- 不同于 `ict-engine-staged-mainrs-extraction-triage`
  - 那偏 main.rs 抽取顺序
  - 这个更广，管 anti-drift governance
- 不同于 `ddd-project-guardrails`
  - 那偏项目设计/DDD
  - 这个偏活体仓库防腐/防漂移/变更治理

## 安全/治理原则
- 不默认安装任何外部工具
- 先吸收方法，再决定是否引依赖
- 默认本地、只读、可审计
- 重依赖/联网/高权限执行必须单独审批
