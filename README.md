# ICT Engine - ICT Expansion Trading Engine

Agent-first Rust CLI for ICT-style market analysis, probabilistic trade reasoning, factor research, feedback learning, and workflow tracking.

English first. 中文在后。

## Quick start

```bash
cargo check
cargo run -- --help
cargo run -- analyze --help
cargo run -- factor-research --help
```

If you only want the core CLI, Rust is enough. Python scripts are optional research helpers.

## Common workflows

### Analyze market data

```bash
cargo run -- analyze \
  --symbol NQ \
  --data-htf <1d.json> \
  --data-mtf <1h.json> \
  --data-ltf <15m.json> \
  --human
```

Human output starts with a trading-desk style summary:

```text
NQ | Bull bias | Entry: medium | Gate: observe_only | Quality: 0.244
Action: TUNE structure_ict
Next: cargo run -- factor-research --symbol NQ --data <15m.json> --factor structure_ict
```

### Demo smoke run

```bash
cargo run -- analyze --symbol DEMO --demo --human

cargo run -- factor-pipeline-debug \
  --symbol DEMO \
  --data examples/demo/demo-15m.json \
  --factor structure_ict \
  --objective expansion_manipulation
```

Equivalent explicit-path form:

```bash
cargo run -- analyze \
  --symbol DEMO \
  --data-htf examples/demo/demo-15m.json \
  --data-mtf examples/demo/demo-15m.json \
  --data-ltf examples/demo/demo-15m.json \
  --human
```

This synthetic dataset is for first-run CLI verification only.

### Diagnose why a factor or gate did not pass

```bash
cargo run -- factor-pipeline-debug \
  --symbol NQ \
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

```bash
cargo run -- factor-research \
  --symbol NQ \
  --data <cleaned-15m.json> \
  --objective expansion_manipulation
```

### Read current research truth

```bash
cargo run -- factor-autoresearch-status --symbol <SYM> --state-dir <dir> --latest-only
python3 scripts/research_verdict.py <state-or-result-dir>
```

## Output modes

`analyze` and `workflow-status` support four output surfaces:

```bash
cargo run -- analyze --symbol NQ --data-htf <1d.json> --data-mtf <1h.json> --data-ltf <15m.json> --output-format json
cargo run -- analyze --symbol NQ --data-htf <1d.json> --data-mtf <1h.json> --data-ltf <15m.json> --compact
cargo run -- analyze --symbol NQ --data-htf <1d.json> --data-mtf <1h.json> --data-ltf <15m.json> --agent
cargo run -- analyze --symbol NQ --data-htf <1d.json> --data-mtf <1h.json> --data-ltf <15m.json> --human

cargo run -- workflow-status --symbol NQ --state-dir state --output-format json
cargo run -- workflow-status --symbol NQ --state-dir state --compact
cargo run -- workflow-status --symbol NQ --state-dir state --agent
cargo run -- workflow-status --symbol NQ --state-dir state --human
```

Use:
- `json` for full archival/debug output
- `compact` for low-token summary
- `agent` for next-step automation surface
- `human` for release-style readable summary

Agent consumers should prefer:
- `decision_summary` over `decision_hint_raw`
- `next_step` for routing and gating
- `next_command` only as a display/backward-compatibility string

`analyze --agent` keeps: direction, entry state, pre-Bayes gate, next command, machine `decision_hint_raw`, human `decision_summary`, structured `next_step`, top evidence, top risks, and top next actions.

`workflow-status --agent` is thinner than `--compact`. It keeps: focus, block state, next command, top disagreement, top actionable artifact, and ensemble headline.

`workflow-status --human` prints concise terminal lines, for example:

```text
NQ | analyze | action_blocked
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

Runtime state directories are ignored by git via `state*/`.

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

- `docs/first-run.md`
- `docs/research-system-map.md`
- `docs/objective-scoring-map.md`
- `docs/smoke-acceptance.md`

## Internal release/agent docs

- `docs/agent-first-runbook.md`
- `docs/release-notes-draft.md`

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
