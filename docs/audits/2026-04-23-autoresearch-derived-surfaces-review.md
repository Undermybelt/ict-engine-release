# ict-engine 审查：autoresearch derived surfaces

> 日期：2026-04-23
> 审查范围：`src/main.rs`、`src/application/factor_lifecycle/autoresearch_surface.rs`、`src/application/factor_lifecycle/mod.rs`、`src/state/persistence.rs`、`src/state/types.rs`
> 重点：`factor-autoresearch` 状态面抽取、`experiments.tsv` 派生账本、`factor_autoresearch_retrospective.md` 派生回顾

## 结论

这轮改动方向是对的：`factor-autoresearch-status` 的聚合逻辑已经从 `main.rs` 抽离，`experiments.tsv` 和 retrospective 也有对应的单元测试，`cargo check` 与目标测试都通过。

但当前 dirty tree 还不建议直接合入。主要有两个实现层面的可靠性问题，以及一个明确的格式门禁问题。

## 验证记录

- `cargo test autoresearch_surface -- --nocapture`：通过
- `cargo check`：通过
- `git diff --check`：通过
- `cargo fmt --check`：失败

## Findings

### P1-1. 派生产物写入失败会直接打断 canonical autoresearch 流程

**位置**
- `src/main.rs:7035-7036`
- `src/main.rs:7104-7105`

**问题**

当前流程在 canonical JSON 已经写入成功之后，立刻用 `?` 同步：

- `experiments.tsv`
- `factor_autoresearch_retrospective.md`

这会把“派生产物刷新失败”提升成“主命令失败”。

具体风险有两种：

1. `append_factor_autoresearch_attempt(...)` 已经把 attempt truth 写进 `factor_autoresearch_attempts.json`，但 `sync_factor_autoresearch_experiments_tsv(...)` 失败后，session 计数和后续迭代不会继续推进。
2. `save_factor_autoresearch_final_summary(...)` 已经写完 final summary，但 retrospective 写失败会让整个命令以错误退出，CLI 退出语义和 canonical state truth 不再一致。

这和本轮设计文档里“TSV / retrospective 是 derived artifact，不替代 JSON truth”的边界是冲突的。

**建议**

- 把两个 sync helper 改成 warning-only / best-effort，不要用 `?` 直接中断主流程。
- 或者把它们放到显式的 refresh/export 命令里，让 canonical persistence 和派生刷新解耦。

### P1-2. retrospective Markdown 直接插入未转义的自由文本，输出很容易被打坏

**位置**
- `src/application/factor_lifecycle/autoresearch_surface.rs:469-480`
- `src/application/factor_lifecycle/autoresearch_surface.rs:492-520`

**问题**

retrospective 渲染时直接把以下字段拼进 Markdown：

- `best_attempt.evaluation.reason`
- `best_attempt.candidate_mutation_spec.hypothesis`
- `best_attempt.branch_summary`
- `recommended_next_focus`

这些值都来自自由文本，尤其 hypothesis / reason 很可能是模型生成内容，天然可能带：

- 换行
- 反引号
- 竖线
- 多余空白

`experiments.tsv` 已经显式做了 `sanitize_tsv_text(...)`，但 retrospective 这里没有等价的 Markdown-safe 处理。结果会是：

- 列表被意外断开
- 表格被竖线污染
- 行内 code fence 被反引号打坏

**建议**

- 新增 `sanitize_markdown_text(...)` 或至少复用现有压平逻辑。
- 给 retrospective 渲染补一组包含换行、反引号、`|` 的单元测试。

### P2-1. retrospective helper 在未限定 session 时会生成“标题和内容不一致”的回顾

**位置**
- `src/application/factor_lifecycle/autoresearch_surface.rs:370-405`
- `src/application/factor_lifecycle/autoresearch_surface.rs:527-549`

**问题**

`build_factor_autoresearch_retrospective(...)` 会：

- 用 `surface.sessions.first()` 作为 header 的 `session_id` / `objective` / `base_factor`
- 但 `attempts_total`、`score_trajectory`、`top_failure_tags`、`cluster_scoreboard` 用的是 `surface` 里的全部 attempts

当前 `main.rs` 在完成态调用时传了 `Some(session_id)`，所以现有主路径是安全的。但 helper 本身接受 `session_filter: Option<&str>`，一旦有人用 `None` 调它，就会得到“看起来像单 session，实际上是多 session 聚合”的 retrospective。

**建议**

- 如果 retrospective 语义就是单 session，就把 helper 收紧成必传 `session_id`。
- 否则需要显式渲染 multi-session summary，而不是复用单 session 标题。

### P2-2. 当前 dirty tree 还没过 rustfmt 门禁

**位置**
- `src/application/factor_lifecycle/autoresearch_surface.rs`
- `src/application/factor_lifecycle/mod.rs`
- `src/state/persistence.rs`
- `src/main.rs`

**问题**

本地 `cargo fmt --check` 当前是红的。功能验证没问题，但如果把这批改动作为待合入候选，格式门禁还没满足。

**建议**

- 在逻辑确认后单独跑一次 `cargo fmt`。
- 然后重新验证：
  - `cargo fmt --check`
  - `cargo check`
  - `cargo test autoresearch_surface -- --nocapture`

## 正向观察

- `factor-autoresearch-status` 的聚合逻辑已经抽到独立模块，后续继续加 warning / report surface 会比继续堆在 `main.rs` 干净得多。
- 新增测试覆盖了这轮最关键的四块：
  - stale-running -> interrupted
  - cluster scoreboard 聚合
  - `experiments.tsv` 渲染与落盘
  - retrospective Markdown 落盘
- `experiments.tsv` 明确保持 derived artifact 语义，而不是再造一个 truth source，这个边界判断是对的。

## 建议的下一步

1. 先把 derived artifact sync 从主执行成功语义里解耦。
2. 再补 Markdown 文本清洗。
3. 最后跑 `cargo fmt`，把这轮 dirty tree 收到可合入状态。
