# ICT Engine - ICT Expansion Trading Engine

Agent-first Rust CLI for ICT-style market analysis, probabilistic trade reasoning, factor research, feedback learning, and workflow tracking.

English first. 中文在后。

## Quick start

```bash
cargo check
cargo build
./target/debug/ict-engine --help
./target/debug/ict-engine analyze --help
./target/debug/ict-engine factor-research --help
```

If you only want the core CLI, Rust is enough. For a first run that stays Rust-only, use the native research backend shown below. Python scripts and Auto-Quant are optional research helpers.

## Contributor baseline

Before sending a PR, please run locally:

- `cargo check --all-targets`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`

All three must be green. CI (`.github/workflows/ci.yml`) runs these on every push.

## Common workflows

### Manage Auto-Quant dependency

```bash
cargo run -- auto-quant-status --state-dir /tmp/ict-engine-auto-quant
cargo run -- auto-quant-bootstrap --state-dir /tmp/ict-engine-auto-quant
cargo run -- auto-quant-update --state-dir /tmp/ict-engine-auto-quant
```

These commands manage the local, pinned Auto-Quant checkout used by the integration work.

For the Auto-Quant review loop:

```bash
cargo run -- factor-research --symbol DEMO --data examples/demo/demo-15m.json --backend auto-quant --state-dir /tmp/ict-engine-auto-quant
cargo run -- auto-quant-adoption-review --symbol DEMO --state-dir /tmp/ict-engine-auto-quant
cargo run -- auto-quant-adoption-decision --symbol DEMO --state-dir /tmp/ict-engine-auto-quant --decision adopt --rationale "approved for next bridge step"
```

### Analyze market data

```bash
cargo run -- analyze \
  --symbol <SYM> \
  --data-htf <1d.json> \
  --data-mtf <1h.json> \
  --data-ltf <15m.json> \
  --state-dir /tmp/ict-engine-analyze \
  --human
```

Human output starts with a trading-desk style summary:

```text
<SYM> | Bull bias | Entry: medium | Gate: observe_only | Quality: 0.244
Action: TUNE structure_ict
Next: ict-engine factor-research --symbol <SYM> --data <15m.json> --state-dir /tmp/ict-engine-analyze
```

### Demo smoke run

```bash
cargo run -- analyze \
  --symbol DEMO \
  --demo \
  --state-dir /tmp/ict-engine-first-run-native \
  --human

cargo run -- factor-pipeline-debug \
  --symbol DEMO \
  --data examples/demo/demo-15m.json \
  --factor structure_ict \
  --objective expansion_manipulation

cargo run -- factor-research \
  --symbol DEMO \
  --data examples/demo/demo-15m.json \
  --state-dir /tmp/ict-engine-first-run-native \
  --backend native \
  --human
```

Equivalent explicit-path form:

```bash
cargo run -- analyze \
  --symbol DEMO \
  --data-htf examples/demo/demo-15m.json \
  --data-mtf examples/demo/demo-15m.json \
  --data-ltf examples/demo/demo-15m.json \
  --state-dir /tmp/ict-engine-first-run-native \
  --human
```

If you omit `--state-dir`, the CLI defaults to repo-local `state/`.

This synthetic dataset is for first-run CLI verification only.
It ships with about 52 candles, so it is intentionally too small for `backtest`, which needs at least 71.

### Diagnose why a factor or gate did not pass

```bash
cargo run -- factor-pipeline-debug \
  --symbol <SYM> \
  --data <cleaned-15m.json> \
  --factor structure_ict \
  --objective expansion_manipulation
```

Read the key fields first:
- `evidence_quality_score`
- `gating_status`
- `bridge_gap`
- `paired_market_quality_report`
- `frame_physics_trace`
- `recommended_actions`

### Run factor research

Rust-only first run:

Use this path when you want the no-pollution in-process Rust path and do not want to bootstrap Auto-Quant on first run.

```bash
cargo run -- factor-research \
  --symbol <SYM> \
  --data <cleaned-15m.json> \
  --objective expansion_manipulation \
  --state-dir /tmp/ict-engine-first-run-native \
  --backend native \
  --human
```

Auto-Quant path:

```bash
cargo run -- factor-research \
  --symbol <SYM> \
  --data <cleaned-15m.json> \
  --objective expansion_manipulation \
  --state-dir /tmp/ict-engine-auto-quant \
  --backend auto-quant
```

### Market-data harness

The public `market-data-harness` path is provider-neutral by default. It no longer fills gaps from repo-owned market presets.

Preferred usage is an explicit request document:

```bash
cargo run -- market-data-harness \
  --action plan \
  --request-json examples/provider_requests/explicit-yfinance-request.json
```

Lightweight CLI shorthand is available for simple providers:

```bash
cargo run -- market-data-harness \
  --action plan \
  --market caller-request \
  --role etf_reference \
  --provider etf_reference=yfinance \
  --symbol-spec etf_reference=SPY
```

For `ibkr` contracts or multi-role requests, use `--request-json` or `--request-stdin`.

Auto-Quant notes:
- first run may bootstrap a pinned dependency checkout under your chosen `--state-dir`
- `uv` is required for the helper scripts
- `prepare.py` may require `TA-Lib` (`brew install ta-lib`) unless you use the documented container fallback

### Read current research truth

```bash
cargo run -- factor-autoresearch-status --symbol <SYM> --state-dir <dir> --latest-only
python3 scripts/research_verdict.py <state-or-result-dir>
```

Auto-Quant integration note:
- `factor-research` and `factor-autoresearch` now default to `--backend auto-quant`
- pass `--backend native` if you explicitly want the Rust-only in-process path

## Output modes

`analyze`, `backtest`, `factor-backtest`, `factor-research`, and `workflow-status` support four output surfaces:

```bash
cargo run -- analyze --symbol <SYM> --data-htf <1d.json> --data-mtf <1h.json> --data-ltf <15m.json> --state-dir /tmp/ict-engine-output-modes --output-format json
cargo run -- analyze --symbol <SYM> --data-htf <1d.json> --data-mtf <1h.json> --data-ltf <15m.json> --state-dir /tmp/ict-engine-output-modes --compact
cargo run -- analyze --symbol <SYM> --data-htf <1d.json> --data-mtf <1h.json> --data-ltf <15m.json> --state-dir /tmp/ict-engine-output-modes --agent
cargo run -- analyze --symbol <SYM> --data-htf <1d.json> --data-mtf <1h.json> --data-ltf <15m.json> --state-dir /tmp/ict-engine-output-modes --human

cargo run -- workflow-status --symbol <SYM> --state-dir /tmp/ict-engine-output-modes --output-format json
cargo run -- workflow-status --symbol <SYM> --state-dir /tmp/ict-engine-output-modes --compact
cargo run -- workflow-status --symbol <SYM> --state-dir /tmp/ict-engine-output-modes --agent
cargo run -- workflow-status --symbol <SYM> --state-dir /tmp/ict-engine-output-modes --human
```

Use:
- `json` for full archival/debug output (default when no flag is passed)
- `compact` for low-token summary
- `agent` for next-step automation surface
- `human` for release-style readable summary

Notes:
- `--compact`, `--agent`, and `--human` are sugar for `--output-format <mode>`. Do not combine them with `--output-format`.
- There is no `--json` alias; JSON is the default, so `workflow-status --output-format json` is the explicit form and plain `workflow-status` already prints JSON.
- For no-pollution trials, prefer an explicit `--state-dir /tmp/...` instead of relying on the default repo-local `state/`.
- `backtest` requires roughly 70+ candles (warmup + hold bars). The bundled `examples/demo/demo-15m.json` (~52 candles) is sized for `analyze`/`factor-backtest` and will error out from `backtest`. Point `--data` at a larger cleaned dataset when running `backtest`.

Agent consumers should prefer:
- `decision_summary` over `decision_hint_raw`
- `next_step` for routing and gating
- `next_command` only as a display/backward-compatibility string

`analyze --agent` keeps: direction, entry state, pre-Bayes gate, next command, machine `decision_hint_raw`, human `decision_summary`, structured `next_step`, top evidence, top risks, and top next actions.

`workflow-status --agent` is thinner than `--compact`. It keeps: focus, block state, next command, top disagreement, top actionable artifact, and ensemble headline.

`workflow-status --human` prints concise terminal lines, for example:

```text
<SYM> | analyze | action_blocked
Block: user_selected_historical_data_missing
Latest: analyze | direction=Bull entry=medium gate=observe_only quality=0.244
Next: Ask the user to choose the historical dataset...
```

## Public script families

| Script | Use when | Backend |
|---|---|---|
| `scripts/search_local.py` | isolated local parameter search | `scripts/archive/factor_local_search_v2d.py` |
| `scripts/search_cluster.py` | cluster jump exploration | `scripts/archive/factor_cluster_jump_v2.py` |
| `scripts/evaluate_bottleneck.py` | evidence/gate/shrink/bridge bottleneck experiments | `scripts/archive/pre_bayes_policy_tuning.py` |
| `scripts/research_verdict.py` | summarize existing state/result dirs | existing artifacts |

Rules:
- default = print help only
- `--run` = execute backend
- `--target` = show backend path
- `--backend-help` = show non-executing backend summary
- `--show-config` = print resolved repo/data/bin paths and `cleaned_data_ready`
- wrappers refuse `--run` when the resolved cleaned-data root is not ready

Important:
- public wrappers must not assume the maintainer's local Tomac cleaned-data layout exists on another machine
- inspect `--show-config` first
- pass `--data-root /path/to/ict-cleaned-mtf` explicitly when you want real execution outside the author's workstation

## State truth

Research state usually lives under:

```text
<state_dir>/<SYMBOL>/
```

Important files:
- `factor_autoresearch_attempts.json`
- `factor_autoresearch_sessions.json`
- `factor_autoresearch_live.json`
- `factor_autoresearch_final.json`
- `factor_mutation_runs.json`
- `research_runs.json`
- `workflow_snapshot.json`
- `artifact_ledger.json`

Derived autoresearch surfaces:
- `experiments.tsv` — grep/diff-friendly ledger derived from autoresearch attempts
- `factor_autoresearch_retrospective.md` — human-readable recap derived from autoresearch status/canonical state

Trust rule:
- if a derived surface disagrees with canonical JSON, canonical JSON wins
- `docs/autoresearch-derived-surfaces-contract.md` defines the boundary in detail

Runtime state directories are ignored by git via `state*/`.

Query the ledger via `ict-engine artifact-status`. Filter flags have the following semantics:
- `--latest-only` keeps the latest row **per `artifact_kind`** (one entry per kind, chosen by `generated_at` then `version`), not a single global latest row. Combine with `--kind <name>` to reduce to one row for a specific kind.
- `--recent-n <N>` keeps the N most recent rows across all kinds.
- `--actionable-only` / `--rule-break-only` / `--consumed-only` are additive filters.

State precedence:
- `--state-dir` overrides `ICT_ENGINE_STATE_DIR`
- if neither is set, `ict-engine` uses `./state`
- shared state is for intentional cumulative loops only; use isolated state for fair comparison

Trust rule:
- if a derived surface disagrees with canonical JSON, canonical JSON wins
- `factor-autoresearch-status` is the preferred read surface for autoresearch session truth
- `experiments.tsv` and retrospective markdown are convenience surfaces only

State defaults and environment knobs:
- `ICT_ENGINE_STATE_DIR` overrides the default `./state` location
- `ict-engine env` prints the currently effective ICT-related environment settings
- `docs/environment-variables.md` documents supported variables
- `docs/state-directory-lifecycle.md` documents cleanup and lifecycle guidance

## Historical data reuse rule

If an agent wants to reuse historical data for `factor-research` or `factor-backtest`, it must ask the user which dataset to use, even when the system has recorded previous paths.

The workflow gate may surface:
- `action_blocked`
- `user_selected_historical_data_missing`
- candidate historical data paths

## FAQ

### Which command should I trust before giving a conclusion?

Use:

```bash
cargo run -- factor-autoresearch-status --symbol <SYM> --state-dir <dir> --latest-only
```

Then inspect the corresponding JSON artifacts.

### Why do the Python scripts not expose a full public CLI?

They are public wrappers over archived experiment backends. The wrappers are stable; the archived backends are still research-grade.

### Can I assume the wrappers will find usable data on a fresh machine?

No.
Wrappers now expose `--show-config` and require explicit cleaned-data readiness before `--run`.
If `cleaned_data_ready=false`, treat that as a setup error and pass `--data-root` explicitly.

### Can `--backend-help` show every backend argument?

No. It shows a non-executing summary. Archived backends do not yet expose a stable public argparse surface.

### Where do long-run outputs go?

Usually into repo-local state dirs such as `state_*` or `<state_dir>/<SYMBOL>/`, depending on the command/backend.

### What is the most common user mistake?

Using the wrong input surface:
- raw CSV instead of cleaned JSON candles
- `factor_mutation_runs.json` as `--mutation-spec`
- shared state dirs for fair comparison experiments

## Public docs

- `docs/ict-engine-docs-catalog-2026-04-25.md` — trust map for canonical, historical, and retained negative-example docs
- `docs/first-run.md`
- `docs/research-system-map.md`
- `docs/autoresearch-derived-surfaces-contract.md`
- `docs/autoresearch-state-transitions.md`
- `docs/objective-scoring-map.md`
- `docs/smoke-acceptance.md`

## Internal release/agent docs

- `docs/agent-first-runbook.md`
- `docs/auto-quant-ictengine-integration-guide.md`
- `docs/release-notes-draft.md`
- `docs/release-mirror-runbook.md` — **authoritative release procedure**
- `docs/external/external-patterns-synthesis-2026-04-23.md` — consolidated external pattern absorb/reject matrix

### Publishing policy (post-v0.0.1)

- The source-repo oversized-history blocker has been cleared; normal source-repo pushes are available again.
- The private release mirror `Undermybelt/ict-engine-release` remains the preferred clean tree-state release transport.
- Treat the source repo as development truth and the mirror repo as the curated release surface.
- See `docs/release-mirror-runbook.md` for the full flow and version-bump rules.

## 中文简介

`ict-engine` 是面向 ICT 市场结构、概率交易证据、因子研究、回灌学习、agent 可读状态的 Rust CLI 研究系统。

首跑先看 help，不要直接长跑：

```bash
cargo check
cargo run -- --help
cargo run -- analyze --help
```

若要读最新研究真相，优先：

```bash
cargo run -- factor-autoresearch-status --symbol <SYM> --state-dir <dir> --latest-only
```

## License

MIT. See `LICENSE`.
