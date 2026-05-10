# Provider-Neutral CLI Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove repo-owned market defaults from the public CLI so the default product surface carries no built-in ES/NQ/YM/GC/CL stance and only consumes external agent/provider configuration for multi-market, live, and companion-symbol orchestration.

**Architecture:** Keep the single-symbol core generic, but make all provider orchestration explicit-request driven. Repo-owned market packs become examples/fixtures instead of runtime truth. Public commands stop inferring spot/options/related markets from repo config; callers must pass a request document or explicit symbol/provider overrides. Hardcoded market-family heuristics in the public runtime are either removed or demoted behind optional external policy input.

**Tech Stack:** Rust, Clap, serde/serde_json, existing provider fetchers, Cargo test, Cargo clippy, existing help audit script.

---

## Why This Plan Exists

当前仓库已经有更合理的分工：

- 需求由外部 agent 持有
- provider 偏好由外部 agent / operator 配置
- CLI 的职责应是执行显式请求，而不是替用户预设“默认市场宇宙”

但现在 public CLI 仍残留明显 repo 立场：

1. `src/main.rs`
   - 多个 `--symbol` / `--market` help 直接举 `NQ, ES, GC` 作为默认示例
   - `market-data-harness` 仍以 `--market <MARKET>` + repo preset 自动推导为主路径
2. `config/market_data_harness_presets.json`
   - 把 `NQ/ES/YM/GC/CL` 写成 repo 默认 market pack
3. `config/market_relationships.json`
   - 把 related futures / ETF / CFD / crypto / vol proxy 写成 repo 默认 companion universe
4. `src/market_catalog/mod.rs`
   - 将上述 repo 配置提升为运行时默认 truth
5. `src/application/data_sources/live_defaults.rs`
   - 默认推断 live futures / spot / options symbols
6. `src/analyze_live_command.rs`
   - 缺省时直接回退到 repo 推断 symbol 关系
7. `src/application/data_sources/control_matrix_runtime.rs`
   - 用 repo 内 provider 默认值和 market key 推导 companion fetch
8. `src/analyze/smt_correlation_section.rs`
   - 默认读取 repo companion universe
9. `src/main.rs`
   - `duration_sizing_scale()` 仍按 `NQ/CL/GC` 分支，属于 repo-owned market behavior heuristic

这与“CLI 默认中立，只接受外部 agent/provider 配置”的目标相反。

## Non-Goals

- 不删除核心单市场能力：
  - `analyze`
  - `factor-research`
  - `factor-pipeline-debug`
  - `backtest`
- 不删除现有 provider fetcher：
  - `yfinance`
  - `tradingview_mcp`
  - `ibkr`
- 不重写 Auto-Quant / Tomac 的只读外部素材接入逻辑
- 不阻止 repo 继续保留 NQ/ES/YM/GC/CL 相关 example 或 fixture
- 不要求一次性清理所有历史文档，只要求 public/runtime truth 改正

## Success Criteria

- `ict-engine --help` 与各子命令帮助不再把 repo 偏好市场写成默认示例
- `market-data-harness` 的默认用法不再依赖 repo preset market catalog
- live / companion / paired-market 相关流程：
  - 要么由显式外部 request/config 成功驱动
  - 要么在缺配置时返回明确错误
  - 不能再静默回退到 repo 内置市场关系
- 删除或移动 repo 内 market preset/relationship 文件后，核心 CLI 不应因默认依赖它们而失效
- NQ/ES/YM/GC/CL 仅保留在 example、fixture、测试样例或项目内部研究资料中，不再作为 product default

## Plan Status Note

本计划应视为对以下文档的方向性修正：

- [2026-04-27-market-data-harness-refactor-plan.md](/Users/thrill3r/projects-ict-engine/ict-engine/docs/plans/2026-04-27-market-data-harness-refactor-plan.md)
- [2026-04-27-canonical-market-catalog-debt-closure-plan.md](/Users/thrill3r/projects-ict-engine/ict-engine/docs/plans/2026-04-27-canonical-market-catalog-debt-closure-plan.md)

上述两份文档把 repo market config 提升成了默认运行时 truth；本计划要求把它降级为 optional example/fixture。

## File Structure

- Modify: `src/main.rs`
- Modify: `README.md`
- Modify: `scripts/help_audit.py`
- Modify: `src/application/data_sources/harness.rs`
- Modify: `src/application/data_sources/command_entry.rs`
- Modify: `src/application/data_sources/control_matrix_runtime.rs`
- Modify: `src/application/data_sources/live_defaults.rs`
- Modify: `src/application/data_sources/options_summary.rs`
- Modify: `src/analyze_live_command.rs`
- Modify: `src/analyze/smt_correlation_section.rs`
- Modify: `src/application/orchestration/workflow_status.rs`
- Modify: `src/market_catalog/mod.rs`
- Modify or Move: `config/market_data_harness_presets.json`
- Modify or Move: `config/market_relationships.json`
- Create: `examples/provider_requests/`
- Create: `tests/provider_neutral_cli.rs`
- Create: `docs/external/provider-request-contract.md`

### Task 1: Neutralize public CLI language

**Files:**
- Modify: `src/main.rs`
- Modify: `README.md`
- Modify: `scripts/help_audit.py`

- [ ] Replace `e.g. NQ, ES, GC`-style help text with neutral wording such as `Instrument identifier supplied by the caller`.
- [ ] Change `market-data-harness --help` so the primary path is explicit request input, not repo market-key lookup.
- [ ] Update README examples to prefer `<SYMBOL>`, `<INSTRUMENT>`, `<REQUEST_JSON>`, `<STATE_DIR>`, and sample files under `examples/` instead of maintainer-preferred futures symbols.
- [ ] Extend `scripts/help_audit.py` so it fails when help output contains repo-owned market exemplars in public-facing flag descriptions.

### Task 2: Make market-data-harness explicit-request only

**Files:**
- Modify: `src/application/data_sources/harness.rs`
- Modify: `src/application/data_sources/command_entry.rs`
- Modify: `src/main.rs`
- Create: `docs/external/provider-request-contract.md`

- [ ] Redefine the harness contract around caller-supplied request payloads instead of repo preset lookup.
- [ ] Keep `market_key` only as an opaque caller label for state/reporting if still needed; it must no longer imply repo-known companion symbols.
- [ ] Require one of:
  - `--request-json <path>`
  - `--request-stdin`
  - full explicit CLI overrides that populate the same request schema without repo defaults
- [ ] Expand the request schema so it can carry:
  - primary instrument id
  - related roles
  - symbol specs per role
  - provider preferences per role
  - optional volatility proxy symbol
  - optional live futures / spot / options identities
- [ ] If the request omits a required symbol spec or provider mapping, fail with an actionable validation error instead of consulting repo preset config.

### Task 3: Remove repo-default market catalog from runtime control flow

**Files:**
- Modify: `src/market_catalog/mod.rs`
- Modify: `src/application/data_sources/live_defaults.rs`
- Modify: `src/application/data_sources/options_summary.rs`
- Modify: `src/analyze_live_command.rs`
- Modify: `src/application/orchestration/workflow_status.rs`
- Modify: `src/analyze/smt_correlation_section.rs`

- [ ] Stop loading repo market catalog as an unconditional runtime dependency for live/default/paired-market inference.
- [ ] Change live analysis so missing `futures_symbol` / `spot_symbol` / `options_symbol` produces a clear “external config required” error unless the caller supplied all needed identifiers.
- [ ] Change SMT correlation companion lists to come from explicit request/config payloads rather than repo relationship tables.
- [ ] Change options-volatility proxy fallback to use caller-supplied proxy identity instead of repo-owned `options_volatility_proxy`.
- [ ] Remove `build_inferable_live_defaults_map()` from public bootstrap surfaces, or mark it example-only if a backward-compat bridge is temporarily required.

### Task 4: Demote repo market packs to examples/fixtures

**Files:**
- Modify or Move: `config/market_data_harness_presets.json`
- Modify or Move: `config/market_relationships.json`
- Create: `examples/provider_requests/`
- Modify: `src/market_catalog/mod.rs`

- [ ] Move current NQ/ES/YM/GC/CL mappings out of default runtime config and into explicit examples or fixtures.
- [ ] Rename the concept in docs from `canonical market catalog` to `sample market packs` or equivalent language that does not imply product truth.
- [ ] Ensure tests that need the old mappings load them from fixture/example paths explicitly.
- [ ] If temporary backward compatibility is required, gate it behind an explicit file path or env var such as `ICT_ENGINE_MARKET_PACK_PATH`, never implicit repo root auto-load.

### Task 5: Remove repo-owned market behavior heuristics from public runtime

**Files:**
- Modify: `src/main.rs`
- Modify: `src/application/backtest/command_entry.rs`
- Modify: `src/application/backtest/finalized_run.rs`
- Modify: `src/factor_research_runtime.rs`
- Create: `tests/provider_neutral_cli.rs`

- [ ] Replace `duration_sizing_scale()` market-specific branches with one of two outcomes:
  - a generic rule that depends only on structural inputs
  - an optional externally supplied execution-policy config
- [ ] Audit other symbol-conditional logic that changes runtime behavior for `NQ/CL/GC/...` and either generalize it or move it behind explicit config.
- [ ] Keep tests, but rewrite them around neutral fixture inputs and config-driven behavior rather than hardcoded repo market identities.

### Task 6: Verification and migration

**Files:**
- Modify: `README.md`
- Modify: `docs/external/provider-request-contract.md`
- Create: `tests/provider_neutral_cli.rs`

- [ ] Add tests that prove public CLI no longer leaks repo market examples in help text.
- [ ] Add tests that `market-data-harness` rejects incomplete requests instead of filling gaps from repo presets.
- [ ] Add tests that live analysis fails clearly without external companion config.
- [ ] Run:
  - `cargo test --all -- --nocapture`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `python3 scripts/help_audit.py`
- [ ] Manually verify that a sample example request under `examples/provider_requests/` still drives the harness successfully.

## Recommended Execution Order

1. Task 1
2. Task 2
3. Task 3
4. Task 4
5. Task 5
6. Task 6

先切 public wording 与 request contract，再拔 runtime 默认推断；否则旧 catalog 仍会继续污染行为边界。

## Risks

- `analyze-live` 目前依赖 repo 默认推断，直接移除后会暴露调用方缺配置的问题
- 若一次性删除 catalog 文件，部分测试和 bootstrap surfaces 会先坏掉
- 历史文档仍会出现 NQ/ES/YM/GC/CL；需要区分“历史/example”与“public default”

## Mitigations

- 先把 request schema 与 example 请求文件补齐，再切 runtime
- 为旧路径提供短期明确报错，不做静默兼容
- 对仍需保留的 repo 内部样例统一迁移到 `examples/provider_requests/` 和测试 fixture

## Acceptance Gate

只有在以下条件同时满足后，才算这个计划完成：

- public help 文案不再携带 repo 偏好市场示例
- runtime 默认路径不再读取 repo market catalog 进行 symbol/provider 推断
- multi-market / live / companion 流程只由外部 request/provider config 驱动
- repo 自有 NQ/ES/YM/GC/CL 材料只作为 example/fixture 存在
