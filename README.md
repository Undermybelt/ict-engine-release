# ICT Engine

Agent-first market-structure research from a clean terminal.

`ict-engine` is a Rust CLI for turning market data into inspectable trading
evidence: structure, technical context, SMT confirmation, regime probabilities,
policy/training surfaces, and an execution-tree readback that humans and agents
can both use.

It is not a black-box signal seller. It is a workbench for asking:

- What does the current market state look like?
- Which evidence is strong, weak, missing, or stale?
- Why is the system observing, blocking, or allowing an execution path?
- What should a human or agent inspect next?

The core CLI runs with Rust only. Python, Auto-Quant, richer providers, and
trainer artifacts are optional hot-plug surfaces.

## First Run

Clone, build, and get a useful answer without writing state into the repo:

```bash
cargo check
cargo run -- --help
cargo run -- analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-first-run --human
```

Expected shape:

```text
Structure: ...
Technicals: ...
SMT: ...
Regime: ... posterior_probabilities=range=... stress=... transition=... trend=...
Plan: action=observe ...
```

Then inspect the workflow state that an agent would use:

```bash
cargo run -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --refresh --agent
cargo run -- pre-bayes-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --refresh --output-format json
cargo run -- policy-training-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --output-format agent
```

Use `/tmp/...` state directories for trials. Reuse a state directory only when
you intentionally want cumulative learning and artifact history.

## What You Get

| Surface | What it answers |
|---|---|
| `provider-status` | Which data/provider lanes are ready, optional, or blocked |
| `analyze` | Human-readable structure, technicals, SMT, regime, and plan |
| `workflow-status` | Agent-readable current state and next action |
| `pre-bayes-status` | Evidence quality, soft labels, and posterior inputs |
| `policy-training-status` | Whether training/admission surfaces have useful data |
| `factor-candidate-packs` | Curated reusable factor candidates |
| `regime-confidence-assets` | Preserved high-confidence regime/source evidence |

Default behavior is consumer-safe:

- no private provider profile is required;
- no maintainer-local dataset is reused by default;
- public/no-config provider behavior falls back to Yahoo/yfinance-compatible
  paths where live data is needed;
- richer providers such as IBKR, TradingView/MCP, crypto adapters, and local
  trainer artifacts are opt-in.

## Output Modes

Most user-facing commands support these surfaces:

| Mode | Best for |
|---|---|
| `--human` | compact terminal readback for a human operator |
| `--agent` | structured state and routing for agents |
| `--compact` | low-token summaries |
| `--output-format json` | archival/debug output |

Examples:

```bash
cargo run -- provider-status --compact
cargo run -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --human
cargo run -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --refresh --agent
```

Agent consumers should prefer structured fields such as `decision_summary`,
`next_step`, `posterior_probabilities`, and artifact ledger entries over display
strings.

## Common Workflows

Analyze cleaned multi-timeframe data:

```bash
cargo run -- analyze \
  --symbol <SYM> \
  --data-htf <1d.json> \
  --data-mtf <1h.json> \
  --data-ltf <15m.json> \
  --state-dir /tmp/ict-engine-analyze \
  --human
```

Diagnose a factor or gate:

```bash
cargo run -- factor-pipeline-debug \
  --symbol <SYM> \
  --data <cleaned-15m.json> \
  --factor structure_ict \
  --objective expansion_manipulation
```

Run native factor research:

```bash
cargo run -- factor-research \
  --symbol <SYM> \
  --data <cleaned-15m.json> \
  --objective expansion_manipulation \
  --state-dir /tmp/ict-engine-native-research \
  --backend native \
  --human
```

Inspect curated candidates:

```bash
cargo run -- factor-candidate-packs --symbol FACTOR_CANDIDATES --state-dir /tmp/ict-engine-candidates
cargo run -- factor-candidate-admission-targets --symbol FACTOR_CANDIDATES --state-dir /tmp/ict-engine-candidates
cargo run -- regime-confidence-assets --symbol REGIME_CONFIDENCE_ASSETS --state-dir /tmp/ict-engine-regime-assets
```

These commands expose reusable artifacts for inspection. They do not promote a
candidate into live execution by themselves.

## Optional Research Helpers

Python wrappers are intentionally conservative. They print configuration by
default and only run backends when you pass `--run`.

```bash
python3 scripts/search_local.py --show-config
python3 scripts/search_cluster.py --show-config
python3 scripts/evaluate_bottleneck.py --show-config
```

Outside a maintainer workstation, pass explicit data roots. Do not rely on
recorded local paths.

## Contributor Gate

Before sending a PR or preparing a release candidate:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test
```

Then smoke the consumer path:

```bash
cargo run -- provider-status --compact
cargo run -- analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-first-run --human
cargo run -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --refresh --agent
```

For release candidates, use a clean sanitized export. Do not publish the broad
dirty research working tree.

## Repository Map

| Path | Purpose |
|---|---|
| `src/` | Rust CLI, analysis, orchestration, provider, and training surfaces |
| `examples/` | public demo/provider/factor candidate examples |
| `config/` | small public fixture/config surfaces |
| `scripts/` | optional Python research wrappers and helpers |
| `docs/README.md` | documentation trust map and folder policy |
| `docs/audits/release-signoff.md` | current release readiness record |
| `docs/release-mirror-runbook.md` | private release mirror flow |
| `AGENT.md` | operating contract for AI agents working in this repo |

## Release Policy

The development checkout is allowed to contain research history and local
experiments. The release mirror must not.

Release rules:

- publish only a clean, verified export slice;
- exclude generated provider caches, Auto-Quant workspaces, local state, and
  maintainer-local paths;
- keep default outputs free of private keys, tokens, account ids, and absolute
  local paths;
- refresh `docs/audits/release-signoff.md` and
  `docs/release-notes-draft.md` before publishing;
- follow `docs/release-mirror-runbook.md` for mirror tag and GitHub release
  creation.

## FAQ

### Is this usable without Python?

Yes. The core CLI and demo path are Rust-only. Python is for optional research
and provider/helper workflows.

### Can I feed raw CSV into `factor-research`?

No. Use cleaned JSON candles.

### Can a command make a strategy trade-ready by itself?

No. Candidate and regime-asset commands expose evidence and training/admission
surfaces. Runtime execution remains fail-closed until the required artifacts and
gates are explicitly present.

### What should agents read first?

Read `AGENT.md`, then use `provider-status`, `workflow-status`, `analyze`,
`pre-bayes-status`, and `policy-training-status` with an explicit `/tmp`
`--state-dir`.

## 中文简介

`ict-engine` 是一个 Rust CLI 研究工作台，用来把市场数据整理成可审计的结构证据、SMT 确认、regime 后验概率、训练/准入状态和执行树读回。

首跑：

```bash
cargo check
cargo run -- analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-first-run --human
```

原则：默认开箱可跑，状态显式，输出短而可读，agent 面使用结构化字段，发布只走干净 export。

## License

MIT. See `LICENSE`.
