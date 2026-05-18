[中文 README](README.zh-CN.md)

# 🦊 ICT Engine ʕ•ᴥ•ʔ

> *Agent-first market-structure research from a clean terminal.*
> *把市场数据熬成可审计证据的 Rust 工作台喵~*

```text
       ╱|、
      (˚ˎ 。7   "What does the market look like now?
       |、˜〵    Which evidence can I trust? What should I check next?"
       じしˍ,)ノ   —— ict-engine answers those three.
```

It is **not** a black-box signal seller  ヽ(`Д´)ﾉ
It **is** a workbench ٩(◕‿◕)۶ that answers:

- 🔍 **What does the current market state look like?**
- 🪶 **Which evidence is strong, weak, missing, or stale?**
- 🚦 **Why is the system observing, blocking, or allowing an execution path?**
- 🧭 **What should a human or agent inspect next?**

The core CLI runs with **Rust only** ✨ Python, Auto-Quant, richer providers, and trainer artifacts are all **optional hot-plug surfaces** (｡•̀ᴗ-)✧

---

## 🧬 The Closed-Loop Pipeline At A Glance

```text
              ICT Engine closed loop ʕっ•ᴥ•ʔっ

       ╭──────────────────╮
       │ ① data provider  │ ◀── Yahoo · IBKR · TV/MCP · local fixture
       ╰─────────┬────────╯
                 │ cleaned candles
                 ▼
       ╭──────────────────╮
       │ ② regime posterior│ ◀── trend / range / transition probs
       ╰─────────┬────────╯
                 ▼
       ╭──────────────────╮
       │ ③ pre-bayes filter│ ◀── evidence quality / soft labels
       ╰─────────┬────────╯
                 ▼
       ╭──────────────────╮
       │ ④ BBN belief net │
       ╰─────────┬────────╯
                 ▼
       ╭──────────────────╮
       │ ⑤ PathRanker     │ ◀── CatBoost ranks structural paths
       ╰─────────┬────────╯
                 ▼
       ╭──────────────────╮
       │ ⑥ execution tree │ ◀── observe / block / allow
       ╰─────────┬────────╯
                 ▼
       ╭──────────────────╮
       │ ⑦ feedback       │ ◀── realized outcomes
       ╰─────────┬────────╯
                 ▼
       ╭──────────────────╮
       │ ⑧ training/refine│ ──╮
       ╰──────────────────╯   │
                 ▲            │
                 ╰─── ↻ loop ─╯  (back into posterior context)
```

> (´｡• ω •｡`) **Map tip**: every node in this chain can be inspected by a CLI command on its own, and each has an `--agent` surface. Consumers should read structured fields, **not** grep display text~

---

## 🚀 First Run In 30 Seconds (｡•̀ᴗ-)✧

> ٩(•́へ•́٩) **Iron rule**: use a `/tmp` state directory for your first run. **Do not dirty the repo!**

```bash
cargo check
cargo run -- --help
cargo run -- analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-first-run --human
```

Expected shape (｡◕‿◕｡):

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

> (｡♥‿♥｡) **Heads up**: keep trials inside `/tmp/...`. Reuse the same `--state-dir` only when you **intentionally** want cumulative learning and artifact history!

---

## 🧭 Command Journey: How A User Walks Through The Demo

```text
              USER ʕ•ᴥ•ʔ
                │
                │  Q: which providers are ready / blocked?
                ▼
       ╭─────────────────────╮
       │   provider-status   │   --compact
       ╰──────────┬──────────╯
                  │  Q: what does the market look like?
                  ▼
       ╭─────────────────────╮
       │       analyze       │   --human  → Structure/Tech/SMT/Regime/Plan
       ╰──────────┬──────────╯
                  │  Q: what should I do next?
                  ▼
       ╭─────────────────────╮
       │   workflow-status   │   --refresh --agent  → next_step
       ╰──────────┬──────────╯
                  │  Q: is the evidence clean or dirty?
                  ▼
       ╭─────────────────────╮
       │  pre-bayes-status   │   --refresh → evidence quality
       ╰──────────┬──────────╯
                  │  Q: enough data on the training surface?
                  ▼
       ╭──────────────────────────╮
       │  policy-training-status  │   → admission ready?
       ╰──────────────────────────╯
```

> (｡◕‿‿◕｡) This is the consumer-safe path: no private profile required, zero-config by default, falls back to Yahoo/yfinance-compatible providers when live data is needed.

---

## 🍭 What You Get (All The Readback Surfaces)

| Surface | What it answers | For |
|---|---|---|
| `provider-status` | which data/provider lanes are ready · optional · blocked | 🧍🤖 both |
| `analyze` | Structure / Technicals / SMT / Regime / Plan readback | 🧍 human |
| `workflow-status` | current state + next action | 🤖 agent |
| `pre-bayes-status` | evidence quality / soft labels / posterior inputs | 🤖 agent |
| `policy-training-status` | whether training/admission surfaces have useful data | 🤖 agent |
| `factor-candidate-packs` | curated reusable factor candidates | 🧍🤖 |
| `regime-confidence-assets` | preserved high-confidence regime/source evidence | 🧍🤖 |

**Default behavior is consumer-safe (｡♥‿♥｡):**

- 🔓 no private provider profile required
- 🚫 no maintainer-local dataset reused by default
- 🌐 zero-config falls back to Yahoo/yfinance-compatible paths when live data is needed
- 🔌 IBKR / TradingView-MCP / crypto adapters / local trainer artifacts → **opt-in**

---

## 🎚️ Output Modes: One Command, Four Readbacks

| Mode | Best for | Face |
|---|---|---|
| `--human` | compact terminal readback for a human operator | (◕‿◕) |
| `--agent` | structured state and routing for agents | (¬‿¬) |
| `--compact` | low-token summaries (saves money + context) | (●'◡'●) |
| `--output-format json` | archival / debug | (｡•̀ᴗ-) |

```bash
cargo run -- provider-status --compact
cargo run -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --human
cargo run -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --refresh --agent
```

> ⚠️ (•́へ•́╮ ) Agent consumers: **prefer reading** `decision_summary` / `next_step` / `posterior_probabilities` / artifact ledger fields — **do not** parse display strings!

---

## 🛠️ Common Workflows

### 📊 Analyze cleaned multi-timeframe data

```bash
cargo run -- analyze \
  --symbol <SYM> \
  --data-htf <1d.json> \
  --data-mtf <1h.json> \
  --data-ltf <15m.json> \
  --state-dir /tmp/ict-engine-analyze \
  --human
```

### 🔧 Diagnose a factor or gate

```bash
cargo run -- factor-pipeline-debug \
  --symbol <SYM> \
  --data <cleaned-15m.json> \
  --factor structure_ict \
  --objective expansion_manipulation
```

### 🔬 Run native factor research

```bash
cargo run -- factor-research \
  --symbol <SYM> \
  --data <cleaned-15m.json> \
  --objective expansion_manipulation \
  --state-dir /tmp/ict-engine-native-research \
  --backend native \
  --human
```

### 🧺 Inspect curated candidates

```bash
cargo run -- factor-candidate-packs --symbol FACTOR_CANDIDATES --state-dir /tmp/ict-engine-candidates
cargo run -- factor-candidate-admission-targets --symbol FACTOR_CANDIDATES --state-dir /tmp/ict-engine-candidates
cargo run -- regime-confidence-assets --symbol REGIME_CONFIDENCE_ASSETS --state-dir /tmp/ict-engine-regime-assets
```

> (｡•́︿•̀｡) These commands **only** expose reusable artifacts for inspection. They **do not** promote a candidate into live execution on their own.

---

## 🌳 Factor Family Map (A~H)

`ict-engine` splits factors into 8 families (`FactorCategory` enum) — 5 full families + 3 stubs:

```text
                       🌳 FactorRegistry
                              │
            ╭─────────────────┼─────────────────╮
            ▼                 ▼                 ▼
        ✅ active          ✅ active         🟡 stub
            │                 │                 │
   ╭────────┴───────╮ ╭───────┴───────╮ ╭───────┴───────╮
   │ A structure    │ │ B trend       │ │ E crowding    │
   │   ICT          │ │   momentum    │ │   herding     │
   │ structure_ict  │ │ trend_mom...  │ │ crowd_herd... │
   ├────────────────┤ ├───────────────┤ ├───────────────┤
   │ C cross-mkt    │ │ D vol mean-   │ │ F spectral    │
   │   SMT          │ │   reversion   │ │   rhythm      │
   │ cross_smt      │ │ vol_meanrev   │ │ spectral_rhy  │
   ├────────────────┤ ╰───────────────╯ ├───────────────┤
   │ G options *    │                   │ H session     │
   │   hedging      │                   │   liquidity   │
   │ options_hedge  │                   │ session_liq   │
   ╰────────────────╯                   ╰───────────────╯
                 * G requires --auxiliary-evidence data
```

> (｀・ω・´) **Hot-plug convention**: external factor families go through the Auto-Quant backend (`--backend auto-quant`) — they **do not** need a `FactorCategory` enum variant! The Rust registry is a bootstrap seed, not the design boundary ٩(◕‿◕)۶

**Source locations**:

| Role | Path |
|---|---|
| Factor definitions + compute | `src/factor_lab/factor_definition.rs` |
| Factor registry | `src/factors/registry.rs` |
| Engine orchestration | `src/factor_lab/engine.rs` |
| Execution tree consumer | `src/application/orchestration/execution_tree.rs` |
| BBN evidence consumer | `src/bbn/evidence.rs` |
| HMM/regime consumer | `src/application/regime/` |

---

## 🐍 Optional Research Helpers (Python)

> Python wrappers **print configuration by default** — only run backends when you pass `--run` (｡•̀ᴗ-)✧

```bash
python3 support/scripts/search_local.py --show-config
python3 support/scripts/search_cluster.py --show-config
python3 support/scripts/evaluate_bottleneck.py --show-config
```

⚠️ (•́へ•́╮ ) Outside a maintainer workstation, **pass explicit data roots** — do not rely on recorded local paths!

---

## 🧹 Contributor Gate (Run These Before A PR)

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

> (◣_◢) Release candidates must use a **clean sanitized export** — do not publish the broad dirty research working tree!

---

## 🗺️ Repository Map

| Path | Purpose |
|---|---|
| `src/` | 🦀 Rust CLI, analysis, orchestration, provider, training surfaces |
| `support/examples/` | 📦 public demo / provider / factor-candidate examples |
| `config/` | ⚙️ small public fixture / config surfaces |
| `support/scripts/` | 🐍 optional Python research wrappers and helpers |
| `support/docs/README.md` | 📚 documentation trust map and folder policy |
| `support/docs/audits/release-signoff.md` | ✅ current release readiness record |
| `support/docs/release-mirror-runbook.md` | 🪞 private release mirror flow |
| `AGENT.md` | 🤖 operating contract for AI agents working in this repo (agent must-read) |

---

## 📦 Install And Package Policy

Licensed under **PolyForm Noncommercial License 1.0.0** — **not currently published as a public package-manager artifact**.

Supported local routes:

```bash
cargo install --path .
cargo run -- --help
```

| Channel | Status | Reason |
|---|---|---|
| ✅ Cargo local install (`cargo install --path .`) | Supported | local use does not redistribute the project |
| 🚫 crates.io | Blocked | public registry publication needs PolyForm channel review |
| 🚫 npm / npx | Blocked | `npx` normally executes packages from npm — needs channel review |
| 🚫 Homebrew public tap/core | Blocked | public formulae distribute source/binaries — needs review |
| 🟡 private local wrappers | Allowed for the copyright holder or authorized private users | improves local ergonomics, does not grant redistribution |

> (｡╯︵╰｡) If the project later needs public `npx`, Homebrew, crates.io, Docker, or binary release distribution, run a **separate packaging slice** that verifies the channel complies with PolyForm Noncommercial 1.0.0 and the project Required Notice.

**Policy references**:

- [Cargo manifest fields](https://doc.rust-lang.org/cargo/reference/manifest.html): `license` and `publish`
- [npm package metadata](https://docs.npmjs.com/cli/v10/configuring-npm/package-json): `license`, `UNLICENSED`, and `private`
- [Homebrew license guidelines](https://docs.brew.sh/License-Guidelines): public formulae need redistributable licensing

---

## 🚢 Release Policy

The development checkout is allowed to contain research history and local experiments. The release mirror **must not** (｡•̀ᴗ-)✧

- ✂️ publish only a clean, verified export slice
- 🚫 exclude generated provider caches, Auto-Quant workspaces, local state, and maintainer-local paths
- 🔐 default outputs **must not** include private keys, tokens, account ids, or absolute local paths
- 📝 refresh `support/docs/audits/release-signoff.md` and `support/docs/release-notes-draft.md` before publishing
- 🏷️ follow `support/docs/release-mirror-runbook.md` for mirror tag and GitHub release
- ❌ **no public package-manager artifacts** unless the license is changed or written permission grants that exact channel

---

## ❓ FAQ

**Q: Is this usable without Python?** (´｡• ᵕ •｡`)
A: Yes! The core CLI and demo path are Rust-only. Python is for optional research and provider/helper workflows.

**Q: Can I feed raw CSV into `factor-research`?** (｡╯︵╰｡)
A: No — use **cleaned JSON candles**.

**Q: Can a command make a strategy trade-ready by itself?** ╮(╯▽╰)╭
A: No. Candidate and regime-asset commands expose **evidence / training / admission** surfaces. Runtime execution remains **fail-closed** until the required artifacts and gates are explicitly present.

**Q: What should agents read first?** ʕ•ᴥ•ʔ📖
A: Read [`AGENT.md`](AGENT.md), then use `provider-status`, `workflow-status`, `analyze`, `pre-bayes-status`, and `policy-training-status` with an explicit `/tmp` `--state-dir`.

---

## 📜 License

This project uses the **PolyForm Noncommercial License 1.0.0**:

- ✅ Noncommercial use, modification, and distribution (under the license)
- ❌ Commercial use requires **separate permission**

See [`LICENSE`](LICENSE).

---

```text
                  ╱|、
                 (˚ˎ 。7    Thanks for reading to the end~
                  |、˜〵      Happy structuring! ʕ•ᴥ•ʔ ✨
                  じしˍ,)ノ
```
