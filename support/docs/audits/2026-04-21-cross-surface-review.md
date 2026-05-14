# ict-engine 跨面审计报告 · 2026-04-21

> **Update 2026-04-21 (shakedown remediation 收尾):**
> - `cargo test` 当前全绿（lib + bin + integration 全部通过）。
> - `.github/workflows/ci.yml` 已存在，仓库有了 CI 基线。
> - P3-4 / P0-C3 `duration_sizing_scale` 回归在本地已修，对应字段现在由 `parse_duration_sizing_scale` 从 `artifact_action_summary` 正确填充。
> - `cargo clippy --all-targets --no-deps` 仍非 0 warning（余留少量 test-only lint），当前视为 advisory 而非 hard gate；别据此宣布仓库“已 clippy-clean”。
> - 下述 P0/P1/P2/P3 细节仍有效，只是上述基线项已搬到 "done" 栏，这里保留原文作为上下文。

> 作者: 本轮 session (Claude Opus 4.7)
> 审计范围: 除 codex 当前占用的三块（factor-backtest / factor-research / backtest 的 human output、那三个命令最终 JSON schema 测试、compare → analyze/reflection/agent 下游）之外的全部代码
> 对照: `green-baseline` 分支 HEAD (Round 1 + Round 2 已合入)
> 目标: 从开源贡献者、终端用户、AI agent 三个视角找到真实问题并给出可落地方案

---

## 0. 方法与信号源

**静态扫描**
- `cargo clippy --all-targets --no-deps` → 干净（4 个风格 warning，全部在 test 代码）
- `cargo test` → lib 347 passed / integration 185 passed / 1 bin 测试 `test_run_factor_backtest_persists_backtest_run_and_agent_bundle` 先前就 fail（duration_sizing_scale 非期望值）
- src/ 非 test 代码里 `.unwrap()` / `.expect()` / `panic!()` / `todo!()` 共 678 处
- 其中 main.rs 单文件占 327 处 `.unwrap()`
- `TODO` / `FIXME` / `XXX` / `HACK` 注释数: **0** — 纪律好，但也意味着"未完成的东西直接留在代码里而不是写在注释里"
- src/ 总 LOC: 76,084 行（main.rs 单文件占 约 25k 行，超总量 1/3）
- 整仓 Rust 文件里 `std::env::var` 直接读的环境变量: 6 个独立 key（下列表）

**文件产物**
- 磁盘上有 23 个 `state_*/` 目录（已 gitignore），合计 **约 5 GB** 未清理；其中 3 个 > 500 MB
- `tests/` 下 13 个 integration 文件 + 1 个 eml_poc，共 1704 行
- `.gitignore` 合理（覆盖 `/state*/`、`__pycache__`、`*.log`、`target/`）
- README.md 存在；CI 配置 `.github/workflows/`: **不存在**（仓库没 CI）

**审计样本的侵入深度**
- 读了: `main.rs` CLI struct、`state/types.rs`、`agent/prompts.rs`、`execution_focus.rs`、`workflow.rs`、`rollout_segments.rs`、所有 `tests/*.rs` 的 LOC 统计
- 跑了: `cargo check --all-targets`（绿）、`cargo clippy`（4 warning）、Round 2 `round2_smoke_replay.sh`（真实产出 artifact）
- 没跑: 真实 `analyze --demo` / `factor-research` 全链路（时间成本太高 + 会碰 codex 的 human output 线）

---

## 1. 发现按严重度排列

### P0 — 会直接伤到贡献者或用户

#### P0-1 · `.unwrap()` 在 CLI 命令主路径上泛滥
- **位置**: 整个 `src/main.rs`（327 处）、`src/application/` 子模块（约 200+ 处）
- **现象**: 用户只要文件路径错 / JSON 损坏 / state 文件半写入，就会拿到 Rust panic backtrace，而不是可读错误
- **影响**: 开源贡献者第一次跑 `cargo run -- analyze --symbol NQ --data foo.json`（foo.json 不存在）直接看到 thread panicked；终端用户碰到 stale state 文件也会 panic
- **建议**:
  - 做一次"panic on boundary" 审查：CLI 入口、文件读写、JSON 反序列化 — 这三条路径零 `.unwrap()`
  - 统一采用 `anyhow::Context` 模式：`fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?`
  - 新增 `tests/cli_error_surface.rs` — 断言常见误操作（文件缺失 / JSON 坏 / symbol 空）退出码与 stderr 前缀

#### P0-2 · `default_value = "state"` 遍布 20 个子命令
- **位置**: `src/main.rs:724, 727, 788, 807, 822, 867, 933, 943, 1026, ...`（每个子命令都有）
- **现象**: 所有命令都默认把 artifact 写到当前工作目录下的 `./state/` 子目录
- **影响**:
  - 用户在项目 A 的目录跑 `ict-engine analyze ...` 会默默建 `./state/`，并把 A 的 artifact 污染到 A
  - 在非 git 项目里跑会散落一堆 artifact
  - 仓库自身有 `.gitignore /state*/` 所以本仓库没事，但分发给用户的二进制没有这层保护
- **建议**:
  - 读 `ICT_ENGINE_STATE_DIR` env var 优先于 `"state"` 默认值
  - 若当前目录既不是 `cargo` 项目也没有 `.ict-engine` 标记文件，first-run 时 stderr 打印一行 warning: `"auto-creating state dir at ./state/ — set --state-dir or ICT_ENGINE_STATE_DIR to customize"`
  - 文档 README 顶部加一节 "state directory lifecycle"

#### P0-3 · `recommended_next_command` 在 14+ 个 struct 上都是 `String`（无 schema）
- **位置**: `src/state/types.rs:1178, 1231, 1485, 1618, 1724, 1956, 2138, 2269, 2345`（+5 处 default）
- **现象**: 字段类型是 `String`，内容由各处 format! 手工拼接，没有集中定义的枚举或 builder
- **影响**:
  - AI agent 读到的命令字符串可能包含不存在的 flag（如历史遗留的 `--ensemble-vote` 已删除）
  - 自引用风险: recommendation 里包含当前命令自己 → 死循环
  - 字符串里混入 `"ask-user: ..."` 前缀（ledger entry review_reason 里见过）— agent 可能直接当命令跑
- **建议**:
  - 新增 `src/agent/next_command.rs`：
    ```rust
    pub enum NextCommand {
        IctEngine { subcommand: IctEngineSubcommand, args: BTreeMap<String, String> },
        AskUser { question: String, paths: Vec<String> },
        NoOp,
    }
    pub fn render_next_command(cmd: &NextCommand) -> String;
    pub fn parse_next_command(raw: &str) -> Option<NextCommand>;
    ```
  - 所有 `recommended_next_command: String` 保留 wire 字段（向后兼容），新增伴生字段 `next_command_kind: NextCommandKind`（enum），agent 优先读 kind
  - `tests/next_command_stability.rs` 断言不同 snapshot 输入产同等字符串（determinism）

---

### P1 — 影响上手与回归稳定性

#### P1-1 · 678 处 unwrap/expect 没有"哪条是必定 infallible / 哪条是 latent bug"的区分
- **位置**: 分布在 `src/application/*`、`src/domain/*`、`src/math/*`
- **现象**: 有些 unwrap 是安全的（比如刚构造的 Array 立即访问），有些是危险的（JSON parse / 浮点 NaN 比较）
- **影响**: 无法做自动化"危险 panic"扫描；contributor PR 审查没标准
- **建议**:
  - 引入 lint: `#![deny(clippy::unwrap_used)]` 到 `src/lib.rs`（不是 bin）+ 对每处合法 unwrap 加 `#[allow(clippy::unwrap_used)]` + 一行 safety 注释
  - 用 clippy 的 `clippy::expect_used` 作为"已标注"白名单 — expect 必须带原因
  - 可以分两轮推进: 先让 `src/math/` / `src/domain/` 清零，之后 `src/application/`

#### P1-2 · ~5 GB 的 `state_*/` 散落目录没有生命周期工具
- **位置**: 仓库根下 23 个 `state_*/` 目录，3 个超 500MB
- **现象**: 全部 gitignored，但没有清理 / 归档 / 复用工具；新开发者 clone 下来后本地跑几次就掉进去
- **影响**:
  - contributor 本地磁盘压力
  - 不同 state 目录之间 state schema 可能漂移 — 老的 `state_expansion_preview/` 还在 v1 schema，新代码假设 v2 schema 读
  - 回放测试没有基线（哪个目录是"干净的 smoke 基线"？）
- **建议**:
  - 新增 `ict-engine state status` CLI 子命令：列出 state_dir 下 artifact 版本分布
  - 新增 `ict-engine state archive --keep-last-n 3 <dir>` — 保留最新 N 个 run，其它打 tar + 删掉
  - 在 README "State directory lifecycle" 一节指定：`state_autoresearch_smoke` 是小基线，其他 `state_*/` 是 session 产物，可删
  - `support/scripts/state_cleanup.sh` — 交互式清理（不自动执行）

#### P1-3 · 环境变量无集中文档
- **位置**: 6 个 key 散落:
  - `ICT_ENGINE_STAGED_ORCHESTRATION` (`stage_runner.rs:19`)
  - `ICT_ENGINE_BELIEF_PRIMARY` (`bbn/engine/registry.rs:25`)
  - `ICT_ENGINE_FAMILY_HISTORY_WINDOW` (`config.rs:1157`)
  - `ICT_ENGINE_TOMAC_ROOT` (`main.rs:13397`)
  - `ICT_EXECUTION_FOCUS` (`reporting/execution_focus.rs:43`) — Round 2 新增
  - `HOME`（间接用，用于路径发现）
- **现象**: README 没提；`ict-engine --help` 也不提；agent 无法从 CLI 出发知道它们的存在
- **影响**: 新用户 / 贡献者无法发现，行为像"魔法"
- **建议**:
  - 新建 `support/docs/environment-variables.md` — 每个变量一行（默认 / 作用 / 示例）
  - CLI 新增 `ict-engine env` 子命令列出当前有效的 ICT_* 变量值
  - README 加 "Configuration & Environment" 一节

#### P1-4 · 20+ 子命令各自 --help，但没有全局 "choose the right command" 向导
- **位置**: `src/main.rs` 整个 `enum Commands`
- **现象**: 命令列表：`analyze, analyze-live, train, backtest, update, factor-research, factor-mutation-status, factor-autoresearch, factor-autoresearch-status, research-verdict, evidence-quality-breakdown, factor-backtest, clean-futures, futures-sop, expansion-sop, factor-pipeline-debug, workflow-status, pre-bayes-status, pre-bayes-diff, artifact-lineage, artifact-status, artifact-diff` — 22 个子命令
- **影响**: 新用户懵：我应该从哪开始？ analyze-live 和 analyze 有什么区别？ factor-research 和 factor-autoresearch 为什么分开？
- **建议**:
  - `ict-engine tour` 子命令：打印 user-journey 图，按阶段（clean → research → analyze → backtest → update → status）
  - README 加一张 "command map" ASCII 图（或 Mermaid）
  - Deprecate 建议: `futures-sop` 和 `expansion-sop` 能不能合并？它们都是 SOP 包装 — 单独列两个 SOP 混淆用户

#### P1-5 · `--output-format` + `--human` / `--agent` / `--compact` alias 重复
- **位置**: 每个支持输出格式的命令（analyze / workflow-status）都有:
  ```
  --output-format  (default "json")
  --human / --agent / --compact  (alias flags, all bool)
  ```
- **现象**: 用户既可写 `--human` 也可写 `--output-format human`；两者同用的优先级隐式（靠 `resolve_output_format` 函数）
- **影响**: agent 写脚本时不知道哪个 authoritative；可能被两种写法混用
- **建议**:
  - 保留 `--human/--agent/--compact` 作为短 alias，明确声明为"sugar over --output-format"
  - `resolve_output_format` 冲突时（e.g. `--human --output-format agent`）应显式报错而不是默默选一个
  - `tests/output_format_resolution.rs` 覆盖 5 种组合

---

### P2 — 体验类 / 可扩展性

#### P2-1 · AgentPromptPack 版本号存在但无迁移路径
- **位置**: `src/agent/prompts.rs:8` `PROMPT_PACK_VERSION = "agent-prompts-v1"`
- **现象**: 字段存在，但没有 "v1 → v2" 的 migrate 函数，也没有反序列化时版本校验
- **影响**: 下次真的要改 prompt schema，老 state 读进来会默默拿到 v1 结构配 v2 调用方 — NPE 风险
- **建议**:
  - 将 `PROMPT_PACK_VERSION` 升为公开 enum `PromptPackVersion::V1`
  - `AgentPromptPack` deserialize 时做 `version` 字段匹配；不认识的版本返回 `Err("unsupported agent pack version")`
  - 把当前 v1 结构锁死，未来新字段只加到 v2，并写 `fn migrate_v1_to_v2()` 

#### P2-2 · artifact_ledger.json 只追加不压缩
- **位置**: `src/state/persistence.rs` `append_artifact_ledger_entry`
- **现象**: 每次 persist 都 append 一条；没有 rotate、compact、prune 逻辑
- **影响**: 长期跑的 state_dir 里 ledger 会膨胀到几十万条 entry，`workflow-status --refresh` 会越来越慢
- **建议**:
  - 新增 `ict-engine ledger compact --symbol NQ --keep-last-per-kind 100` — 按 kind 保留最近 N 条，旧条归档到 `artifact_ledger.archive.jsonl`
  - 定义 "hot ledger" 概念：最近 N 条 + 所有未 consumed 的 — 热数据走 `artifact_ledger.json`，冷数据走 archive
  - `tests/ledger_compaction.rs` 覆盖 hot/cold 划分语义

#### P2-3 · artifact kind 没有集中注册表
- **位置**: 字符串 `"execution_artifact"`, `"mece_recovery_artifact"`, `"factor_tucker_core"`, `"ensemble_vote"`, `"pda_sequence"`, `"pending_update"`, `...` 散落在多个 persist 函数里
- **现象**: 没有 `enum ArtifactKind`；新增 kind 靠约定；读端可能漏 match 某个 kind
- **影响**: 
  - artifact-status / artifact-lineage 命令的 `--kind` filter 没有自动补全候选
  - agent 不知道全集
- **建议**:
  - 新增 `src/state/artifact_kind.rs`:
    ```rust
    pub enum ArtifactKind {
        ExecutionArtifact,
        MeceRecoveryArtifact,
        FactorTuckerCore,
        EnsembleVote,
        PdaSequence,
        PendingUpdate,
        // ...
    }
    impl ArtifactKind {
        pub fn wire_name(&self) -> &'static str;
        pub fn all() -> &'static [ArtifactKind];
    }
    ```
  - `ict-engine artifact-kinds` 列出全集
  - persist 函数接 `ArtifactKind` 而非 `&str`

#### P2-4 · Round 2 `ICT_EXECUTION_FOCUS` env var 未文档化 + 未接入 workflow_status
- **位置**: `src/application/reporting/execution_focus.rs` 定义了 `execution_focus_enabled()` 但没被 `workflow_status.rs::build_human_workflow_status_view` 消费
- **现象**: env var 存在、测试通过、但真实触发路径为空（focus surface 写了没人调）
- **影响**: Round 2 §3.6 的验收条件（flag 存在且可用）只完成了一半 — build 函数可调但 CLI 不调
- **建议**: Round 3 工作（本审计外）把 `execution_focus_enabled()` 的调用接入 `workflow_status` 人类视图的顶部渲染

#### P2-5 · 测试覆盖缺口（避开 codex 的 f-b/f-r schema 区）
- **位置**: `tests/`
- **缺口**:
  - 没有 CLI `--help` 输出稳定性回归（frequently 漂移）
  - 没有"state dir schema migration"测试（v1 → v2 ExecutionArtifact 读回的兼容性）
  - 没有测试 `persist_execution_artifact` / `append_artifact_ledger_entry` 在磁盘满 / 权限拒绝时的错误路径
  - 没有 `ICT_EXECUTION_FOCUS` env var 切换后 workflow_status 输出 diff 的 snapshot 测试
  - 没有 fuzz/property 测试（如 `quickcheck`）对 spectral / tucker / sparse 的边界
  - Tucker driver 有单测但没测读真实 `state_autoresearch_smoke/NQ/learning_state.json` 的 snapshot
- **建议**: 开 `tests/cli_help_regression.rs` + `tests/state_schema_v1_compat.rs` + `tests/tucker_driver_real_state.rs`

---

### P3 — 技术债 / 机会

#### P3-1 · main.rs 单文件 25k 行
- **现象**: 所有子命令的 pre-dispatch 逻辑都挤在 main.rs
- **影响**: PR 冲突高发（Round 2 已经撞了两次）；新贡献者读不完
- **建议**: 主计划 §4 已说 "main.rs 只做 thin orchestration" — 当前还未收敛。拆分成 `src/bin/ict-engine/commands/{analyze,backtest,...}.rs`，每个子命令一个文件，main.rs 只做 clap 分发

#### P3-2 · 没有 `CONTRIBUTING.md` / `CODE_OF_CONDUCT.md`
- **影响**: OSS 项目观感差；新贡献者不知道该怎么做 PR
- **建议**: 加 `CONTRIBUTING.md` 规范（跑 `cargo test`、跑 `support/scripts/round2_smoke_replay.sh`、不在 CLI path 上 unwrap、不改 main.rs 超 50 行）

#### P3-3 · 没有 CI
- **现象**: `.github/workflows/` 空
- **影响**: 每个 PR 合并前没有自动 cargo test / clippy；本仓库的 bin test `test_run_factor_backtest_persists_backtest_run_and_agent_bundle` 先前已 fail 就是这个原因
- **建议**: 加 `.github/workflows/ci.yml` — 运行 `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`；macOS + ubuntu matrix

#### P3-4 · 有一个持续 fail 的 bin 测试
- **位置**: `tests::test_run_factor_backtest_persists_backtest_run_and_agent_bundle` (src/main.rs:22220)
- **现象**: `assert_eq!(runs[0].duration_sizing_scale, Some(1.0))` 得到 `None`
- **影响**: `cargo test` 返回 non-zero；贡献者/CI 混乱
- **建议**: 这条可能属于 codex 的 f-b/f-r schema 领域（duration_sizing_scale 是 backtest 相关字段），**标记给 codex 收拾**

#### P3-5 · 3 个 clippy 风格 warning
- `clippy::field_reassign_with_default` @ src/main.rs:25563（测试代码里）
- `clippy::useless_vec` @ src/main.rs:21691（测试代码里）
- **建议**: `cargo clippy --fix --tests` 一行解决 — 可以在 CI 里加 `-D warnings` 强制

#### P3-6 · 同名方法 / builder 爆炸
- **现象**: `build_canonical_belief_snapshot` / `build_canonical_belief_snapshot_with_pda` / `build_canonical_belief_report` / `build_canonical_belief_report_with_pda` — 同一族 4 个函数，参数递增
- **影响**: 新入场者不知该用哪个；也是 Round 2 审计时看到的主要混乱源
- **建议**: 合并成一个 `build_canonical_belief_snapshot(input: BeliefSnapshotInput)` + `BeliefSnapshotInput::with_pda(packet)` builder 方法

---

## 2. 对 AI Agent 使用者的专项发现

ict-engine 既服务人类 CLI 用户，也服务 AI agent（它们读 state 并决定下一步）。AI agent 视角额外问题:

#### A-1 · agent 看到的 stdout/stderr 划分不清
- 默认 `env_logger::init()` 在 main.rs:1408 — 不显式设定 log level，log 会混进 stderr
- **建议**: `ict-engine` 在 agent 模式下强制 `RUST_LOG=error` 或重定向 `/dev/null`；或者加 `--quiet-log` flag

#### A-2 · agent_context_bundle_minimal vs full 的读取策略不明
- 两种 bundle 并存（`AgentContextBundle` / `AgentContextBundleMinimal`），`pda_cluster_label` 仅在 minimal 上
- **建议**: 文档化"什么时候用哪个"；agent 协议 spec 一份 `support/docs/agent-protocol.md`

#### A-3 · blocking_truth 的可机读等级混乱
- 硬阻断字符串在 workflow_status.rs:355 硬编码:
  ```rust
  let hard_block_statuses = ["blocked", "bridge_needs_confirmation", "validated_regressing", "credibility_gate_blocked"];
  ```
- **建议**: 提升为 `pub const HARD_BLOCK_STATUSES: &[&str]` + 文档

#### A-4 · reflection_bundle 的字段可选性 agent 读不出
- 很多字段 `Option<String>` + `#[serde(skip_serializing_if = "Option::is_none")]` — serialize 时消失，agent 不知道这些字段是"没有值"还是"本版本没这个字段"
- **建议**: schema 里加一个顶层 `present_fields: Vec<String>` 元字段，显式告诉 agent 哪些字段参与了这次输出

---

## 3. 功能查漏补缺

**CLI 侧应补**：
- `ict-engine init [dir]` — 初始化 state_dir + 写 `.ict-engine/` 标记
- `ict-engine doctor` — 检查 state_dir schema 版本、必需 artifact 齐全度、git 状态
- `ict-engine state archive / clean / size` — 生命周期管理
- `ict-engine artifact-kinds` — 枚举全集
- `ict-engine env` — 列出所有 ICT_* 变量及当前值
- `ict-engine tour` — user-journey 向导

**架构侧应补**：
- schema migration 层（v1→v2 ExecutionArtifact 升级路径）
- `NextCommand` enum（替代 `recommended_next_command: String`）
- `ArtifactKind` enum（替代字符串）
- 集中的 "HardBlockStatus" / "ExecutionBias" / "DecisionHint" 枚举

**文档侧应补**：
- `support/docs/environment-variables.md`
- `support/docs/agent-protocol.md`
- `support/docs/state-directory-lifecycle.md`
- `CONTRIBUTING.md`
- `CHANGELOG.md`（目前没有版本演进记录）

**CI 侧应补**：
- `.github/workflows/ci.yml`（fmt + clippy + test）
- `support/scripts/round2_smoke_replay.sh` 加入 CI 作为 smoke gate

---

## 4. 路线图建议（按优先级）

### Sprint A — 稳定性底座（约 5-7 天）
1. **P0-1**: 做一次 panic-on-boundary 审查 — CLI / IO / JSON 三路径清零 unwrap（≈ 100 处 unwrap 替换）
2. **P0-2**: `ICT_ENGINE_STATE_DIR` env var 支持 + first-run warning
3. **P3-3** + **P3-5**: 加 CI（fmt + clippy -D warnings + test）
4. **P3-4**: 让 codex 修 duration_sizing_scale 那条 bin 测试 / 或标 ignore

### Sprint B — 体验底座（约 5-7 天）
5. **P0-3**: `NextCommand` enum + 迁移策略
6. **P1-3**: 环境变量集中 doc + `ict-engine env` 命令
7. **P1-4**: `ict-engine tour` + README command-map
8. **P1-5**: output-format 冲突检测 + 测试

### Sprint C — 扩展性底座（约 7-10 天）
9. **P2-3**: `ArtifactKind` enum + 注册表
10. **P2-2**: ledger compaction 命令
11. **P1-2**: state dir 生命周期工具
12. **P3-1**: main.rs 拆分（最少把 artifact / workflow 两组命令挪走）
13. **P2-1**: AgentPromptPack migration 框架

### 跨 Sprint 持续
- **P1-1**: clippy::unwrap_used deny 推广（分模块做）
- **P2-5**: 测试覆盖补强（按模块）
- 文档补齐（CONTRIBUTING / CHANGELOG / agent-protocol）

---

## 5. 显式 out-of-scope（交给 codex）

本审计**不涉及**以下区域，codex 当前在改:
1. `src/main.rs` 11859-11900（backtest_command 的 human output）
2. `src/main.rs` 12700 附近（factor_research_command 的 human output）
3. `src/main.rs` 13806 附近（factor_backtest_command 的 human output）
4. `src/main.rs` 25605 / 25626 附近（最终 JSON schema 测试）
5. 任何 compare 线（backtest_compare / research_compare）下游到 analyze / reflection / agent_report 的扩展
6. `duration_sizing_scale` / `backtest_conformal_coverage_1sigma` / `backtest_trade_count` 相关的 AnalyzeRunRecord / ResearchRunRecord / BacktestRunRecord 字段

如果本审计里有建议触及了这六条，**以 codex 的版本为准**，审计建议作废。

---

## 6. 附录: 本次没做的深度探索

受时间与 session context 限制，以下没做，但对完整审计有用，列给后续轮次:
- 跑 `cargo run -- analyze --demo` 全链路实测 5 种 output_format（会撞 codex 区）
- 跑 `cargo run -- factor-autoresearch --iterations 3` 验证长链路 state_dir 增长
- 手读 `src/pda_sequence/`、`src/bvar/`、`src/gp/`、`src/hawkes/`、`src/kalman/`、`src/mcmc/`、`src/sv/` 模块 — 这些都是 "研究功能"，可能有 dead code
- 对照 `support/docs/plans/` 下 20+ 个 plan 文件，核对哪些 plan 的承诺还没落地（plan-reality diff）
- 跑一次 `cargo tree | wc -l` 看依赖总规模
- 审查 `support/paper2code/` 目录（仓库根里还有一个 Python 子项目，未进 Cargo workspace）

以上建议留给 Round 3+ 的审计轮次。

---

## 7. Codex Compare Surface Closure (commit `5a6a050`) 追加审计

> 原 §5 列为 out-of-scope 的 6 条被 codex 在 `5a6a050 feat: close compare surfaces across research and backtest` 里收口（+3258 / -136 across 9 files，main.rs 单独 +2391）。本节把这块纳入审计，方法与前 §1-§3 相同。
> 审计对齐: HEAD = `5a6a050`；参考 `support/docs/plans/2026-04-21-compare-surface-closure-plan.md`。

### 7.0 Codex 实际交付清单

| 产物 | 位置 | 说明 |
|---|---|---|
| `BacktestCompareReport` 结构 + 3 个 build 辅助 | `src/application/backtest/backtest_compare.rs`（新建 535 行） | `compare_backtest_results` / `build_shrink_on_off_comparison_summary` / `build_oos_quality_delta_surface` |
| 3 个 JSON payload builder | `src/main.rs` L16823-16885 | `build_backtest_output_payload` / `build_factor_backtest_output_payload` / `build_factor_research_output_payload` |
| 2 个 compare 摘要函数 | `src/main.rs` L16809-16821 | `human_backtest_compare_summary` / `human_research_compare_summary`，都 wrap 共享的 `human_compare_summary` |
| 3 个 human renderer | `src/main.rs` L16887-16945 | `render_backtest_human_output` / `render_factor_backtest_human_output` / `render_factor_research_human_output` |
| `ReflectionBundle.compare_summary: Option<String>` 新字段 | `src/application/reflection/adapter.rs` L35 | 让 compare 摘要可以进 reflection 面 |
| research 适配器填充 | `src/application/reflection/research_adapter.rs` L40 | `bundle.compare_summary = Some(compare_summary.to_string())` |
| 新增 schema 测试 | `src/main.rs` L25710-25892 | 5 个测试覆盖 3 个 payload + 2 个摘要 label |
| 计划 doc | `support/docs/plans/2026-04-21-compare-surface-closure-plan.md`（389 行） | 步步走的 agentic plan |

### 7.1 P0 — 发布就会被用户/贡献者骂

#### P0-C1 · `--output-format human` 对 backtest / factor-backtest / factor-research **根本没接到 CLI**
- **位置**: `src/main.rs` L11849-11855 / 12686-12693 / 13784-13791 三条分发点
- **现象**: 三个命令命令结构体（clap derive 在 L811-1110）没有 `output_format` / `--human` / `--agent` / `--compact` 任何一个 flag。payload 构建后无条件走 `println!("{}", serde_json::to_string_pretty(&payload)?)`
- **影响**: 计划 doc §Scope Notes 明确说"Current reality: backtest, factor-backtest, and factor-research do **not** accept OutputFormat::Human today"，codex 做的"human 渲染"实质是在 JSON payload 里塞一个叫 `human_output` 的字符串字段。用户只能 `jq -r '.human_output' <output.json>` 从 JSON 里扒出来 — 典型把"添加一个字段"当成"接入一个 flag"
- **建议** (具体 patch):
  1. 三个命令的 clap 结构 + 输入 struct 都加 `output_format: OutputFormat` + 三个 alias bool
  2. 各命令入口复用 `resolve_output_format()` 把 flag 合并成 `OutputFormat`
  3. 每条分发点 `match output_format` — Human 分支改为 `println!("{}", payload["human_output"].as_str().unwrap_or(""))`，Json/Compact/Agent 复用已有路径
  4. 新增 `tests/cli_backtest_human_flag.rs` + `tests/cli_factor_research_human_flag.rs` 断言 `--human` 退出码 0 并输出不以 `{` 开头的纯文本（用 `assert_cmd` 或 `std::process::Command`）

#### P0-C2 · `render_factor_backtest_human_output` / `render_factor_research_human_output` 实质是 JSON dump 套个前缀
- **位置**: `src/main.rs` L16919-16931 / L16933-16945
- **现象**:
  ```rust
  let mut lines = vec![format!(
      "Factor backtest summary: {}",
      serde_json::to_string(report).unwrap_or_else(|_| "unavailable".to_string())
  )];
  ```
  把整个 `FactorBacktestReport` / `ResearchReport` 序列化成单行 JSON，粘在 "Factor backtest summary:" 后面。`serde_json::to_string` 对 `impl Serialize` 的 report 返回几千个字符的单行 JSON
- **影响**: "Human" 渲染实际输出给用户的是一行几 KB 的 JSON。这不是 human surface，是 JSON-prefixed dump
- **建议** (具体 patch):
  1. 把 `render_factor_backtest_human_output` / `render_factor_research_human_output` 签名从 `report: &impl Serialize` 换成各自具体类型 `&FactorBacktestReport` / `&ResearchReport`（类型擦除是罪魁祸首）
  2. 手写 renderer 提炼关键字段（见 `backtest` 已有的正版示范 L16891-16912: 聚焦 `trades` / `total_return` / `spread_bps` / `comparable`）
  3. 对 factor-backtest 至少渲染：`symbol` / `factor_count` / `best_factor` / `aggregate_return` / `family_outcomes.len()` / top-3 factor scores
  4. 对 factor-research 至少渲染：`research_objective` / `best_factor` / `aggregate_return` / `feedback_records_generated/applied` / top-3 ranking
  5. 新增 snapshot 测试锁定 human 行的 shape（不锁精确数字，但锁字段名存在）

#### P0-C3 · 先前存在的 bin 测试 `test_run_factor_backtest_persists_backtest_run_and_agent_bundle` 仍然 fail
- **位置**: `src/main.rs` L22386 `assert_eq!(runs[0].duration_sizing_scale, Some(1.0))`
- **现象**: 现在 fail 行位置从 L22221 漂到 L22386（codex 增加了 2391 行），但内容不变：`left: None, right: Some(1.0)`。该断言期望 `BacktestRunRecord.duration_sizing_scale` 被 `parse_duration_sizing_scale(&report.artifact_action_summary)` 填充，但目前 artifact_action_summary 里没有 `duration_sizing_scale=X.XX` 那行，所以 parse 返回 None
- **影响**: `cargo test` 仍然 1 failed — 本次 compare 手术完整地包围了 duration_sizing_delta_surface 这块（backtest_compare.rs 多个 duration_sizing_direction 字符串），codex 应当顺手补这条 bin 测试
- **建议** (具体 patch):
  1. 在 `run_factor_backtest` 的某处（应当是生成 `artifact_action_summary` 的地方）往 summary 里 push 一条 `format!("duration_sizing_scale={:.2} remaining_expected_bars={:.3} market={} family={}", scale, remaining, market, family)` — 现在看样子 run_factor_backtest 不产这行所以测试挂
  2. 若该行本就应该由其它路径注入、只是在单测 fixture 里没注入，那就改断言为 `.is_none() || .is_some()` 容忍路径；或在测试里手动 push 那一行
  3. 比较干净的做法: 把 `duration_sizing_scale` 的 populate 挪到 `BacktestRunRecord::from_report(...)` 构造函数里，不依赖 `artifact_action_summary` 的字符串 parse

### 7.2 P1 — 影响可维护性

#### P1-C1 · `replacen("Compare:", "Backtest compare:", 1)` 是脆弱的字符串改装
- **位置**: `src/main.rs` L16813 / L16820
- **现象**: `human_backtest_compare_summary` / `human_research_compare_summary` 的唯一区别是 wrap `human_compare_summary` 后做一次 replacen 把前缀 "Compare:" 改写成 "Backtest compare:" / "Research compare:"
- **影响**: 如果哪天改 `human_compare_summary` 的前缀（比如改成 "compare:"、小写、或加时间戳），两个 wrapper 会静默变成 no-op；测试 `test_human_backtest_compare_summary_labels_backtest_surface` 会红，但正式消费者会拿到未分流的前缀
- **建议**: 把 `human_compare_summary` 抽成 `fn human_compare_summary_body(compare: &BacktestCompareReport) -> String`（返回不带前缀的正文），两个 wrapper 各自拼前缀:
  ```rust
  pub fn human_backtest_compare_summary(c: Option<&BacktestCompareReport>) -> Option<String> {
      c.map(|c| format!("Backtest compare: {}", human_compare_summary_body(c)))
  }
  ```
  一处改动，两面稳定

#### P1-C2 · `ReflectionBundle.compare_summary` 只在 research 路径被 set，backtest 路径没 set
- **位置**:
  - 写入: `src/application/reflection/research_adapter.rs` L40
  - 断言未写: `src/application/reflection/adapter.rs` L164 `assert!(bundle.compare_summary.is_none())`
- **现象**: reflection bundle 有了 compare_summary 字段，但只有 research 的 adapter 填；backtest 的 adapter 从未赋值
- **影响**: agent 读 `reflection_bundle.compare_summary` 在 backtest run 上永远得 None，但在 research run 上有值 — 反射面语义不对称，下游 agent 容易误以为 backtest 没跑 compare
- **建议**:
  1. 在 `src/application/reflection/adapter.rs` 加对应 `apply_backtest_compare_summary_to_reflection_bundle(bundle: &mut ReflectionBundle, summary: Option<&str>)` 
  2. `backtest_command` 在构造 reflection bundle 时调用
  3. 更新 `adapter.rs:164` 测试从 `is_none()` 变成覆盖 "set via apply 函数"

#### P1-C3 · 3 个 payload builder + 3 个 renderer 都住在 main.rs
- **位置**: `src/main.rs` L16809-16945，共 137 行
- **现象**: 跨 Sprint 约束 §4 要求 "main.rs 永远只做 thin orchestration + command wiring"。Codex 的 plan 文档里写 "Keep compare generation centered in src/main.rs for now" — 自己先破例了
- **影响**: main.rs 本轮又加 2391 行，从 ~25k 逼近 ~27.4k 行；后续 PR 冲突面积增大
- **建议**:
  1. 建 `src/application/reporting/backtest_output.rs`，把 3 个 builder + 3 个 renderer + 2 个 summary helper 整体迁进去
  2. 在 `src/application/reporting/mod.rs` 新增 re-export
  3. main.rs 三条分发点只保留 `use crate::application::reporting::build_backtest_output_payload;` + `let payload = build_backtest_output_payload(...)`

#### P1-C4 · schema 测试用 `.get("xxx").is_some()` 不能排除 `Value::Null`
- **位置**: `src/main.rs` L25854-25855
  ```rust
  assert!(payload.get("compact_compare_report").is_some());
  assert!(payload.get("backtest_compare_report").is_some());
  ```
- **现象**: `serde_json::Value::get()` 对 key 存在但值为 null 的情况返回 `Some(&Value::Null)`，`.is_some()` 照样 true。所以即使 codex 某天让 `compact_compare_report` 写成 null 而不是对象，这条断言仍然 pass
- **建议**: 把断言改为 `assert!(payload.get("compact_compare_report").map_or(false, |v| !v.is_null()))` — 有语义地检查"非 null 且 key 存在"

### 7.3 P2 — 质量/维护

#### P2-C1 · 3 份 schema 测试 copy-paste，没参数化
- **位置**: `src/main.rs` L25766 / L25858 / L25877 — `test_backtest_output_payload_includes_human_compare_summary` / `test_factor_backtest_output_payload_includes_human_compare_summary` / `test_factor_research_output_payload_includes_human_compare_summary`
- **现象**: 三个测试除了调用不同 builder 几乎完全复制；fixture 构造（sample BacktestReport / ResearchReport）也各自复制
- **建议**: 用 `rstest::rstest` crate 或手写 `#[test]` 遍历 fixture: 一个 `[(label, payload_fn, expected_summary_key, expected_prefix), ...]` 数组，一个 helper 跑断言。修 bug 改一处

#### P2-C2 · 没有"compare 是 None"的测试
- **位置**: `src/main.rs` L25766-25892 所有 schema 测试都传 `Some(sample_compare_report(...))`
- **现象**: 没人测 `build_backtest_output_payload(..., None, ...)` 时 payload 里 `human_backtest_compare_summary` 是 null 还是 key 不存在
- **影响**: 下游 agent 可能依赖 "字段永远存在"，一旦某次 backtest 没跑 compare，字段缺失会触发 agent 端 None-check 路径未覆盖
- **建议**: 补 `test_backtest_output_payload_omits_compare_when_none` / `_factor_backtest_` / `_research_` 三条，断言 `payload["human_backtest_compare_summary"].is_null()`（或 key 不存在，取决于设计决定 — 建议 null 而非缺失，便于 schema 稳定）

#### P2-C3 · `ReflectionBundle` 的 `Option<String>` 字段数增到 12
- **位置**: `src/application/reflection/adapter.rs` L14-49
- **现象**: 新增 `compare_summary` 后，bundle 的 `Option<String>` 字段达 12 个 (`execution_*` / `why_*` / `*_summary` / `compare_summary` / `pda_*`)；其中 11 个带 `#[serde(skip_serializing_if = "Option::is_none")]`
- **影响**: 结合 §A-4（agent 分不清"没有值"与"本版本没此字段"），这次又扩了一个。agent 读 `.compare_summary` 拿不到时没法判断原因
- **建议**: 参考原审计 §A-4，给 ReflectionBundle 加 `#[serde]` rename + `present_fields` 元字段（见 §2 路线图 Sprint B 提过）

#### P2-C4 · plan doc 把"agentic sub-skill 指令"混进了功能描述
- **位置**: `support/docs/plans/2026-04-21-compare-surface-closure-plan.md` 开头
  > **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended)...
- **现象**: 功能 plan 的首段是给 agent 讲"怎么执行"的元说明，人类读者需要跳过
- **建议**: 把 agent-runtime 说明挪到 plan 末尾的 "Appendix: execution mode notes"；plan 正文开门见山讲架构

### 7.4 功能查漏补缺（原 §3 的本轮增量）

`--output-format` 在 backtest / factor-backtest / factor-research 上缺失 → 这是 P0-C1，上面已开建议，此处不重复。

新增的功能缺口：

- **compare_summary 的 cross-adapter 对称性**: reflection bundle 的 compare_summary 既然加了，就应在所有写 reflection bundle 的路径上都有 populate 机会；现在只有 research 写了。建一个 `apply_compare_summary_to_reflection_bundle(bundle, Option<&BacktestCompareReport>)` 通用 helper
- **compare 的"是否应做"信号**: 当 backtest 运行但历史记录不足（compare 不可生成），payload 里应有一个 `compare_available: bool` 或 `compare_blocked_reason: Option<String>` 字段；现在缺了这层，只能通过看 `backtest_compare_report` 是 null 推断
- **compare_summary 的 key field 缺 artifact_id 链**: 目前 human_compare_summary 是纯文本 "Compare: direction | risk | next"，没有指回的 `baseline_run_id` / `candidate_run_id`。若两个 run 的 compare 结果冲突，agent 无法定位是哪两个 run。建议在 `BacktestCompareReport` 加 `baseline_run_id: Option<String>` / `candidate_run_id: Option<String>` 并在 human summary 里 suffix `(baseline=X, candidate=Y)`
- **缺 compact_compare_report 的独立 CLI surface**: codex 把 compact_compare_report 塞进 payload，但没加独立命令 `ict-engine compare --baseline <id> --candidate <id>` 让用户直接查任意两个 run 的 compare。应当补一个
- **没有 `ict-engine compare --last 2` 快捷面**: 用户最想要的是"比较最近两次 run"，current 需要自己去 workflow_snapshot 扒两个 run_id

### 7.5 对原审计 §4 路线图的影响

- **Sprint A 稳定性底座** 追加:
  - P0-C1 `--output-format` 接入（约 1-2 天）
  - P0-C2 两个 factor render 函数重写（约 2-3 天）
  - P0-C3 duration_sizing_scale bin 测试修复（约 0.5 天）— **建议 codex 自己收拾**，属他 scope
- **Sprint B 体验底座** 追加:
  - P1-C1 replacen → 拼前缀重构（0.5 天）
  - P1-C2 backtest reflection bundle 补 compare_summary（1 天）
  - 新命令 `ict-engine compare --baseline/--candidate/--last N`（2 天）
- **Sprint C 扩展性底座** 追加:
  - P1-C3 3 个 builder + 3 个 renderer 出 main.rs（1-2 天）
  - P2-C3 ReflectionBundle 加 `present_fields` 元字段（配合原 §A-4）
  - P2-C1 schema 测试参数化（0.5 天）
  - P2-C2 None-compare 测试补齐（0.5 天）

### 7.6 codex 这批做对的地方（值得保留）

审计默认列问题，但也要记好的部分：

- **BacktestCompareReport 抽到独立模块**（`src/application/backtest/backtest_compare.rs`）— 比散在 main.rs 好
- **shrink / oos / duration 三个 delta surface 都有 unit test**（inline 在 backtest_compare.rs 里）— 覆盖细
- **schema 测试的前缀断言强绑定了输出 shape** — `assert!(summary.starts_with("Backtest compare:"))` 等，未来 codex 自己再改前缀时会立即红
- **`OutputFormat::parse` 识别 "json" / "compact" / "agent" / "human"**（从 `resolve_output_format` L2320 可见）— Output 枚举已有 `Human` 变体，只要接到 flag 就能用，P0-C1 的 fix 实际上是"连线"而不是"新建"
- **计划 doc 显式列了 Scope Notes 明示不接 analyze/report redesign** — 边界清晰，没乱扩张

### 7.7 更新后的 out-of-scope

§5 原列的 6 条已随 `5a6a050` 完成；本轮审计覆盖那 6 条。现在的 out-of-scope:

- **其他 codex 可能在跟进的 work-in-progress 分支**（不在 HEAD 上的）
- **未合入的 PR**（目前 HEAD 干净，无 PR）
- **`support/paper2code/` 子目录**（Python，未进 Cargo workspace，原审计 §6 已列）

---

## 8. 最终完整路线图（综合 §4 + §7.5）

按修订后的优先级重列，Sprint A-C 继续沿用：

### Sprint A 稳定性底座 (7-10 天)
1. [P0-1] CLI / IO / JSON 三路径 unwrap 清零（约 100 处）
2. [P0-2] `ICT_ENGINE_STATE_DIR` + first-run warning
3. [P0-C1] backtest / factor-backtest / factor-research 接 `--output-format` flag
4. [P0-C2] `render_factor_backtest_human_output` / `render_factor_research_human_output` 重写
5. [P0-C3] duration_sizing_scale bin 测试修复
6. [P3-3] + [P3-5] 加 CI（fmt + clippy -D + test）

### Sprint B 体验底座 (7-10 天)
7. [P0-3] `NextCommand` enum
8. [P1-3] 环境变量集中 doc + `ict-engine env`
9. [P1-4] `ict-engine tour` + command map
10. [P1-5] output-format 冲突测试补齐
11. [P1-C1] replacen → 拼前缀重构
12. [P1-C2] backtest reflection compare_summary 对称
13. [新] `ict-engine compare --baseline/--candidate/--last` 独立命令

### Sprint C 扩展性底座 (10-12 天)
14. [P2-3] `ArtifactKind` enum
15. [P2-2] ledger compaction 命令
16. [P1-2] state dir 生命周期工具
17. [P3-1] main.rs 拆分（优先把 P1-C3 的 6 个函数挪走）
18. [P2-1] AgentPromptPack migration 框架
19. [P1-C3] + [P2-C3] ReflectionBundle + payload builder 解 main.rs
20. [P2-C1] + [P2-C2] schema 测试参数化 + None-compare 补齐

### 持续跨 Sprint
- [P1-1] clippy::unwrap_used deny 推广
- [P2-5] 测试覆盖补强
- 文档: CONTRIBUTING / CHANGELOG / agent-protocol / environment-variables / state-directory-lifecycle / compare-surface-schema

---

## 9. 附录: 本次审计的 git 基线

- `5a6a050 feat: close compare surfaces across research and backtest` (HEAD)
- `2602cfa feat: land Round 2 integration layer` (Round 2)
- `683f18b feat: land the_well-inspired execution optimizations (2.1-2.5)` (Round 1)
- `e2ae793 feat: PDA FCGR + ensemble majority voting` (pre-Round-1)

所有 P0/P1/P2 编号在以上三层 commit 之间稳定，便于 grep 和 PR 关联。

