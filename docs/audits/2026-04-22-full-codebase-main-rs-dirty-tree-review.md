# ict-engine 全仓审查：main.rs、dirty tree 与发布体验

> 日期：2026-04-22
> 审查范围：`/Users/thrill3r/projects-ict-engine/ict-engine`
> 重点：`src/main.rs` 臃肿优化、未提交改动是否还需修改、贡献者试运行、发布后人类用户与用户 agent 体验、功能缺口与具体解决方案。

## 结论

当前 dirty tree 的功能方向基本可继续推进：`cargo check --all-targets`、`cargo test`、`cargo clippy --all-targets -- -D warnings` 都通过，旧审计里的若干高风险体验问题已经修掉。

但当前工作树还不能直接作为“可发布/可合入”状态。主要阻塞项有三个：

- `cargo fmt --check` 失败，而新增的 `.github/workflows/ci.yml` 把 fmt 作为硬门禁；如果直接开 PR，CI 会红。
- 多个 artifact ledger writer 仍把 `path` 写成硬编码 `state/<SYMBOL>/...`，实际用户传了 `--state-dir /tmp/...` 时，agent 会拿到错误 artifact 路径。
- agent JSON 输出会把 `next_step.deferred_command` 和 `next_command` 内的本地路径递归 redacted 成 `<local-path>`，导致用户 agent 不能直接执行下一步命令。

`src/main.rs` 的问题不是单纯“行数多”，而是边界不对：它同时承担 CLI schema、dispatch、报告 DTO、状态编排、artifact ledger 视图、SOP、factor research/backtest 编排、输出 redaction、workflow snapshot 聚合和 160 个 bin 级测试。它占 `src/` Rust LOC 的约 36%，仍是后续贡献冲突和行为漂移的最大来源。

## 验证记录

### 静态和测试

| 命令 | 结果 | 说明 |
| --- | --- | --- |
| `cargo check --all-targets` | 通过 | dev target 和 tests target 均可编译 |
| `cargo test` | 通过 | lib 355 tests、bin 160 tests、integration/doc tests 全绿 |
| `cargo clippy --all-targets -- -D warnings` | 通过 | 说明 README 里 “clippy advisory” 已偏保守；当前 dirty tree 可过严格 clippy |
| `cargo fmt --check` | 失败 | `src/main.rs`、`src/application/orchestration/workflow_status.rs`、`src/application/release_closure/mod.rs` 需要 rustfmt |
| `git diff --check` | 通过 | 没有 whitespace error |

### CLI 试运行

| 场景 | 命令摘要 | 结果 |
| --- | --- | --- |
| 贡献者 help | `cargo run -- --help` | 通过，命令列表可见 |
| 环境变量发现 | `cargo run -- env` | 通过，能显示 `ICT_ENGINE_STATE_DIR` 等变量 |
| analyze human demo | `cargo run -- analyze --symbol DEMO --demo --human --state-dir /tmp/...` | 通过，human 输出已不再泄露原始 `ask-user:` wire 协议 |
| analyze agent demo | `cargo run -- analyze --symbol DEMO --demo --agent --state-dir /tmp/...` | 通过但有 agent 可执行命令被 redacted 的问题 |
| 输出 flag 冲突 | `analyze --human --output-format json` | 正确失败：`do not combine --output-format with --compact/--agent/--human` |
| workflow empty human | `workflow-status --symbol DEMO --state-dir /tmp/... --human` | 通过，空状态输出语义清楚 |
| workflow after analyze agent | `workflow-status --agent` | 通过但 `next_step.deferred_command` 被替换为 `<local-path>` |
| artifact latest only | `artifact-status --latest-only` | 输出成功，但暴露硬编码 `state/...` artifact path bug |
| autoresearch empty | `factor-autoresearch-status --latest-only` | 通过，空状态为 `no_autoresearch_state` |
| factor-backtest human | `factor-backtest --human` | 通过，已是可读多行摘要 |
| backtest demo | `backtest --human` with demo fixture | 正确失败，提示 `got 52, require at least 71`；README 已说明 demo fixture 不适合 `backtest` |
| factor-pipeline-debug | `factor-pipeline-debug ...` | 通过，但输出约 811 行 JSON；适合 debug，不适合作为 agent 默认低 token surface |
| repo 外 first run | binary from `/tmp/ict-engine-warning-run` | 会打印 `auto-creating state dir at ./state` warning，符合 docs |

## P0/P1 发现

### P0-1. 新增 CI 会因 rustfmt 失败

证据：

- `.github/workflows/ci.yml` 新增了 `cargo fmt --check`。
- 本地 `cargo fmt --check` 当前失败，diff 涉及：
  - `src/main.rs`
  - `src/application/orchestration/workflow_status.rs`
  - `src/application/release_closure/mod.rs`

影响：

- 贡献者按 README 提交 PR 后，CI 会在 fmt 阶段失败。
- 这不是功能 bug，但会直接阻断开源协作和发布流水线。

解决方案：

1. 在单独提交里运行 `cargo fmt`，不要和功能逻辑混在一起。
2. 重新跑：

```bash
cargo fmt --check
cargo check --all-targets
cargo test
cargo clippy --all-targets -- -D warnings
```

3. 如果暂时不想格式化全仓，就不要把 `cargo fmt --check` 放进新增 CI；但更推荐格式化，因为当前 clippy 和测试已经绿。

### P0-2. Artifact ledger 的 `path` 仍有硬编码 `state/...`

复现：

```bash
cargo run -- analyze --symbol DEMO --demo --agent --state-dir /tmp/ict-engine-audit-20260422-agent
cargo run -- artifact-status --symbol DEMO --state-dir /tmp/ict-engine-audit-20260422-agent --latest-only
```

观察：

- `pending_update`、`execution_candidate`、`ensemble_vote` 的路径使用了真实 `/tmp/...`。
- `execution_tree_artifact` 输出 `path: "state/DEMO/execution_tree_trace.json"`。
- `mece_recovery_artifact` 输出 `path: "state/DEMO/mece_recovery_artifact.json"`。

代码根因：

- `src/application/orchestration/execution_tree.rs:461` 使用 `Path::new("state")...`。
- `src/application/regime/persistence.rs:94` 使用 `Path::new("state")...`。
- `src/application/execution/persistence.rs:48` 使用 `Path::new("state")...`。
- `src/pda_sequence/persistence.rs:55` 和 `src/factor_lab/tucker_persistence.rs:105` 也有同类模式，虽本次 smoke 未触发，但属于同一缺陷族。

影响：

- 用户 agent 根据 ledger 读取 artifact 会读错文件。
- release 用户自定义 `--state-dir` 后，状态 truth 与 ledger truth 不一致。
- `artifact-status --latest-only` 作为“真相面”时会传播错误路径。

解决方案：

1. 在 `src/state/persistence.rs` 或新模块 `src/state/artifact_paths.rs` 加统一 helper：

```rust
pub fn artifact_path<P: AsRef<std::path::Path>>(
    state_dir: P,
    symbol: &str,
    file: &str,
) -> String {
    state_dir
        .as_ref()
        .join(symbol)
        .join(file)
        .to_string_lossy()
        .to_string()
}
```

2. 替换所有 `Path::new("state").join(...)` ledger path 写法。
3. 给每个 writer 加 tempdir 测试，断言 ledger `path` 以 tempdir 开头：
   - `persist_execution_tree_artifact`
   - `persist_mece_recovery_artifact`
   - `persist_execution_artifact`
   - `persist_pda_sequence_analysis`
   - `persist_factor_tucker_core`
4. 加一个集成测试：`analyze --state-dir <temp>` 后 `artifact-status --latest-only` 的每个 `path` 都存在或明确标记为 virtual/non-file。

### P0-3. Agent surface 的可执行命令被 redaction 破坏

复现：

```bash
cargo run -- analyze --symbol DEMO --demo --agent --state-dir /tmp/ict-engine-audit-20260422-agent
cargo run -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-audit-20260422-agent --agent
```

观察：

`next_step.deferred_command` 输出为：

```text
ict-engine factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir <local-path>
```

根因：

- `src/application/reporting/analyze_output.rs:635-638` 对 `json`、`compact`、`agent`、`human` 都使用 `print_redacted_json` / `redact_local_paths`。
- `src/main.rs:2361-2423` 还有一份重复 redaction 实现。
- `src/main.rs:2442-2460` 的 workflow agent 输出先构建 structured `next_step`，再递归 redacts 所有字符串字段。

影响：

- 人类用户可以理解 `<local-path>`，但不能复制执行。
- 用户 agent 更严重：它会把 `<local-path>` 当参数执行，下一步必失败。
- 新增 `RecommendedNextCommandMeta` 的意义被削弱，因为输出层最后仍把 machine field 改坏。

解决方案：

1. 把输出字段分成 machine 与 display 两类：

```rust
pub struct RenderedNextStep {
    pub action_type: String,
    pub user_input_required: bool,
    pub prompt: Option<String>,
    pub executable_command: Option<String>,
    pub display_command: Option<String>,
}
```

2. `agent` 和 `json` 输出保留 `executable_command` 原值，只 redacts `display_command`、human lines、log-like review text。
3. `compact` 可以默认 redacted，但必须清楚标注它不是执行 surface。
4. 增加回归测试：
   - `analyze --agent` 的 `next_step.deferred_command` 不含 `<local-path>`。
   - `workflow-status --agent` 的 `next_step.deferred_command` 不含 `<local-path>`。
   - `--human` 输出仍可 redacts 隐私路径，或增加 `--no-redact-paths` 让本地用户可复制命令。

### P1-1. `RecommendedNextCommandMeta` 已加字段，但未成为权威协议

证据：

- `src/state/types.rs:1915` 新增了 `RecommendedNextCommandMeta`。
- `src/application/reporting/agent_report.rs:29-58` 仍手写 parse `ask-user:`。
- `src/application/release_closure/mod.rs:52-79` 也手写一份 `workflow_next_step_view` parser。
- `src/application/orchestration/workflow_status.rs:715-830` 的 agent view 仍从 raw string 推导 `next_step`。

影响：

- 三份解析逻辑未来会漂移。
- agent 要同时理解 raw command、meta、next_step，协议没有唯一真相。
- 本次 redaction bug 就是因为 typed metadata 没有被输出层当成 machine contract 保护。

解决方案：

1. 把 `recommended_next_command_meta(raw)` 升级为唯一 parser。
2. 用一个 typed `NextStep` builder 生成 analyze agent report、workflow agent report、release closure view。
3. raw `recommended_next_command: String` 仅作为 backward-compatible display 字段。
4. agent docs 明确：优先读 `next_step.executable_command` 或 `recommended_next_command_meta.executable_command`，不要执行 raw `next_command`。

### P1-2. `src/main.rs` 仍是系统耦合中心

事实：

- `src/main.rs` 27,740 行。
- `src/` Rust 总行数约 77,079 行，`main.rs` 占约 36%。
- `main.rs` 有 313 个顶层 `fn`、76 个顶层 `struct`、3 个顶层 `enum`。
- 生产代码到 `line 21185`，`#[cfg(test)] mod tests` 从 `line 21186` 开始，bin 内测试有 160 个。
- 好消息：生产区没有 `.unwrap()` / `.expect()` / `panic!()` / `todo!()` / `unimplemented!()`，旧审计里的 CLI panic 风险已明显下降。

问题不是“main.rs 长”本身，而是职责边界错误：

| main.rs 区间 | 当前职责 | 应迁移到 |
| --- | --- | --- |
| `181-704` | output format、report DTO、command input DTO | `src/application/cli/types.rs`、`src/application/reporting/report_types.rs` |
| `707-1457` | Clap `Commands` schema | 暂留 main 或迁移 `src/cli/commands.rs` |
| `1458-1887` | 巨型 command dispatch | `src/application/cli/dispatch.rs`，main 只 parse + dispatch |
| `2155-2423` | analyze output + redaction | `src/application/reporting/`，复用 `output_foundation` |
| `2425-2535` | workflow output wrapper | `src/application/orchestration/workflow_status.rs` |
| `3098-3563` | artifact status/diff/lineage view | `src/application/artifacts/` 或 `src/state/artifact_ledger.rs` |
| `3660-6464` | clean-futures / futures-sop / expansion-sop | `src/application/sop/` |
| `6464-8532` | analyze persistence/artifact glue | `src/application/analyze_command/` |
| `8533-11380` | workflow snapshot build and conflict analysis | `src/application/orchestration/workflow_snapshot.rs` |
| `11466-12473` | train/backtest/update commands | `src/application/commands/{train,backtest,update}.rs` |
| `12474-15400` | factor research/autoresearch/factor backtest | `src/application/factor_commands/` |
| `15401-17660` | analyze report + probabilistic backtest builder | `src/application/analyze/` and `src/application/backtest/` |
| `17660-21185` | helper/reporting/agent prompt builders | split by domain: `agent`, `state`, `reporting`, `artifact_review` |
| `21186-27740` | 160 bin tests and fixtures | module tests near moved code + integration tests |

Recommended extraction plan:

1. Stage 0：先不动行为，跑 `cargo fmt`，固定 CI。
2. Stage 1：抽 redaction + next-step parser。这个阶段直接修 P0-3，并删除 `main.rs` 与 `output_foundation.rs` 的重复 redaction。
3. Stage 2：抽 artifact ledger view/writers。这个阶段修 P0-2，并让 path contract 有测试。
4. Stage 3：抽 output/report DTO。迁移 `AnalyzeReport`、`BacktestReport`、`UpdateReport` 和 emit 函数，main 只传 input。
5. Stage 4：抽 command handlers。每个 subcommand 一个 handler module，签名形如 `pub fn run(input: AnalyzeCommandInput) -> Result<()>`。
6. Stage 5：移动 tests。每次迁移一个功能簇，就把相关 tests 移到对应 module 或 `tests/cli_*`.

Guardrails:

- 一次只迁移一个 command family，不混 feature work。
- 每阶段跑 `cargo check --all-targets`、相关 targeted tests、最后跑 `cargo test`。
- 不改变 serialized field name；若必须改，先写 release note 和 compatibility test。

## Dirty Tree 审查

### 可保留方向

- `Cargo.toml` 开启 `clap/env` 合理；`ICT_ENGINE_STATE_DIR` 现在通过 clap env 生效。
- README 新增 contributor baseline、output mode、state env、ledger semantics，方向正确。
- 新增 `.github/workflows/ci.yml` 方向正确，但要先让 `cargo fmt --check` 绿。
- 新增 hybrid regime / Wasserstein / HSMM / governor / timeframe 模块有 integration test，`tests/regime_core_first_pass.rs` 覆盖基本行为。
- `factor-backtest --human` 已从旧审计中的巨大 dump 改成可读摘要。
- `factor-autoresearch-status` empty state 已从旧占位结构改成清楚的 `no_autoresearch_state`。
- `analyze --human --output-format json` 冲突已正确报错。

### 还得改的地方

| 项 | 风险 | 处理建议 |
| --- | --- | --- |
| fmt 未通过 | CI blocker | 单独 `cargo fmt` 提交 |
| hard-coded ledger paths | agent 会读错 artifact | 建 helper，替换 5 处 writer，补 tempdir 测试 |
| agent machine fields 被 redacted | agent 下一步命令不可执行 | 分离 `executable_command` 与 `display_command` |
| `RecommendedNextCommandMeta` 未权威化 | 协议漂移 | 删除重复 parser，集中 next-step builder |
| untracked `paper2code/` 57 个文件 | release tarball 和源码边界变重 | 明确是 research sandbox；加 README、requirements policy、smoke tests，或移到 `docs/external/` / 独立仓 |
| missing `CONTRIBUTING.md` / `CODE_OF_CONDUCT.md` | 开源协作入口不完整 | 至少加 `CONTRIBUTING.md`，说明 verify commands、state-dir、main.rs 修改限制 |
| old audits 与当前状态混在 docs 根部 | 读者容易混淆 | 给旧审计加 superseded header，指向本文 |

### paper2code 专项

`paper2code/` 当前有 57 个未跟踪源码/文档文件，`find` 统计约 7,096 行。它包含 `bridge.py`，会动态加载多个 paper module，并依赖 numpy；子目录还带 `rammstein/requirements.txt`、`red_queens_trap/requirements.txt` 等独立运行时。

风险：

- 它不在 Rust test/CI 内。
- 它和 README 的“Python scripts are optional research helpers”关系不清楚；用户可能以为它是正式 runtime。
- 若打包发布，会把未验证 Python research sandbox 一起带出去。

建议：

1. 如果这是正式功能，补 `paper2code/README.md`、`python -m pytest` 或至少 import smoke，并在 CI 里单独可选跑。
2. 如果只是素材，移动到 `docs/external/` 或 `paper2code/README.md` 明确“not runtime, research notes only”。
3. 不要让 Rust CLI 的发布承诺依赖 `paper2code`，除非先有 dependency policy 和 reproducible environment。

## 开源贡献者视角

贡献者 first-run 体验比昨天审计更好：

- `cargo check --all-targets` 通过。
- `cargo test` 全绿。
- `cargo clippy --all-targets -- -D warnings` 全绿。
- README 现在写清楚 test baseline、output mode、state env、backtest demo fixture 限制。

还缺：

- `cargo fmt --check` 当前失败，和新 CI 冲突。
- 没有 `CONTRIBUTING.md`，贡献者不知道 main.rs 的修改边界、如何处理 state dirs、哪些 docs 是最新真相。
- `src/main.rs` 太大，新贡献者很难定位“某个命令应该改哪里”；即使 tests 全绿，review 成本仍高。

建议贡献者模拟测试包：

```bash
cargo fmt --check
cargo check --all-targets
cargo test
cargo clippy --all-targets -- -D warnings
cargo run -- --help
cargo run -- analyze --symbol DEMO --demo --human --state-dir /tmp/ict-engine-contrib-smoke
cargo run -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-contrib-smoke --agent
cargo run -- artifact-status --symbol DEMO --state-dir /tmp/ict-engine-contrib-smoke --latest-only
```

验收要求：

- fmt/check/test/clippy 全绿。
- `workflow-status --agent.next_step.deferred_command` 不含 `<local-path>`。
- `artifact-status.entries[*].path` 对真实文件 artifact 要么存在，要么有明确 `path_kind` 说明不是文件路径。

## 发布后用户与用户 agent 体验

### 人类用户

已改善：

- `--human` 输出更像 trading desk 摘要，不再直接泄露 `ask-user:`。
- `factor-backtest --human` 不再是单行巨大 JSON dump。
- 错误的 flag 组合会显式失败。
- repo 外首次创建默认 `./state` 会给 warning。

仍有问题：

- human 输出里若输入路径是绝对路径，会显示 `--data <local-path>`，隐私安全但不可复制执行。
- `factor-pipeline-debug` 输出太大，没有 `--compact` / `--agent`；用户只能人工滚动巨大 JSON。
- `backtest` demo fixture 不能跑是合理的，但 first-run docs 应给一个可跑的 backtest fixture 或明确“demo smoke 仅 analyze/factor-backtest”。

### 用户 agent

已改善：

- `next_step` 结构化程度更高。
- empty workflow/autoresearch state 有明确 contract。
- `recommended_next_command_meta` 已进入 state types。

仍有问题：

- agent 可执行 command 被 `<local-path>` 破坏。
- ledger path 可能错误。
- agent view 仍混合 raw `next_command`、`next_step`、`recommended_command`、`recommended_next_command_meta`，没有唯一权威字段。
- `artifact-status` 没有 `path_exists` / `path_kind`，agent 无法在不额外 stat 的情况下判断 ledger 条目是否可读。

## 功能查漏补缺

1. Artifact path contract

需要定义所有 ledger `path` 的语义：真实文件、相对 state-dir、repo-relative、virtual artifact。当前同一输出内混合 `/tmp/...` 与 `state/...`。

2. Output privacy policy

需要区分 human privacy 与 agent executability。建议：

- `human` 默认 redacted。
- `compact` 默认 redacted。
- `agent` 默认 raw executable fields + redacted display fields。
- `json` 默认 raw，可加 `--redact-paths`。

3. Command protocol schema

建议把 `ask-user:` string 降级为兼容层，新增：

```json
{
  "next_step": {
    "action_type": "ask_user_choose_historical_data",
    "user_input_required": true,
    "prompt": "...",
    "executable_command": "ict-engine factor-research ...",
    "blocked_reason": "user_selected_historical_data_missing"
  }
}
```

4. CLI help/output snapshot tests

已有 160 个 bin tests，但还缺真实 process-level snapshot：

- `ict-engine --help`
- `ict-engine analyze --help`
- `ict-engine analyze --agent`
- `ict-engine workflow-status --agent`
- `ict-engine artifact-status --latest-only`

5. State maintenance

已有 `scripts/state_cleanup.sh`，但还没有正式 CLI：

- `ict-engine state status --state-dir <dir>`
- `ict-engine state prune --keep-last-n <n>`
- `ict-engine artifact-status --validate-paths`

6. main.rs ownership rule

应写进 `CONTRIBUTING.md`：

- 正常 PR 不应新增 `src/main.rs` 大段逻辑。
- 新 command 必须有 handler module。
- 新 output surface 必须在 reporting/orchestration 模块有 snapshot test。

## 推荐执行顺序

### Batch 1：发布阻塞修复

1. `cargo fmt` 并提交格式化。
2. 修复 hard-coded ledger path。
3. 修复 agent executable command redaction。
4. 新增对应回归测试。
5. 跑完整验证：

```bash
cargo fmt --check
cargo check --all-targets
cargo test
cargo clippy --all-targets -- -D warnings
```

### Batch 2：协议收敛

1. 将 `RecommendedNextCommandMeta` 变成唯一 next command parser。
2. 统一 `AgentNextStep` / `workflow_next_step_view`。
3. agent docs 改为只推荐读取 `next_step.executable_command`。
4. 给 `artifact-status` 增加 `path_exists` 或 `path_kind`。

### Batch 3：main.rs 拆分

1. 先抽 `output_foundation` 和 workflow/analyze output wrapper，减少 redaction 漂移。
2. 再抽 artifact ledger view 与 workflow snapshot builder。
3. 再抽 command handlers。
4. 最后迁移 tests。

### Batch 4：开源包装

1. 新增 `CONTRIBUTING.md`。
2. 给旧审计加 superseded header。
3. 明确 `paper2code/` 的 release 边界。
4. 补 first-run backtest fixture 或 docs。

## 本次审查后的状态判断

可以继续在当前分支上做修复，不需要回滚 dirty tree。未提交改动中新增的 regime/PDA/agent-output 方向有测试支撑，整体不是坏方向。

但不要在修复前发布或合 PR。最短闭环是先修 fmt、ledger path、agent redaction 三项；这三项都小于大重构，但直接影响 CI、artifact truth 和用户 agent 可执行性。
