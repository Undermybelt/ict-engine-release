# ICT Engine — Agent Entry Map

This file is the first thing any AI agent should read when entering this repo.
It is the shared operating contract for Codex, Claude, and other agents. It also
maps the factor landscape so agents cannot claim "no usable factors exist."

## Agent Mission

Your job is to make `ict-engine` useful from a clean checkout, not to preserve
maintainer-local habits.

When a user asks for help, optimize for:
- a zero-config command they can run now;
- a short human explanation in the user's language;
- a structured agent surface when automation is needed;
- explicit state/artifact paths;
- no private paths, keys, datasets, or generated workspaces in public output.

Default to proving behavior with the real CLI. Do not answer release, readiness,
provider, or workflow questions from memory when a local command can check it.

## User Service Contract

For a new or confused user, guide them through this path first:

```bash
cargo run --quiet -- provider-status --compact
cargo run --quiet -- analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-first-run --human
cargo run --quiet -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --refresh --agent
```

Then explain:
- what evidence exists;
- what is missing;
- whether the system is observing, blocked, or ready for a next inspection step;
- the exact next command to run.

Never imply a strategy is trade-ready because a demo, candidate inventory, or
training surface exists. Candidate packs and regime-confidence assets are
inspection/admission surfaces until the runtime gates explicitly promote them.

## Agent Operating Checklist

Before changing files:
- read this file and the active handoff/plan named by the task;
- run `git status --short` and preserve unrelated dirty work;
- claim the work in the same authoritative markdown when the lane requires it;
- prefer `/tmp/...` for generated state and smoke output.

Before saying a gate passes:
- run the exact command in the current turn;
- inspect exit status and output;
- record evidence paths in the relevant handoff or release doc.

Before release:
- use a clean sanitized export, not the broad dirty working tree;
- rerun fmt, Clippy, full tests, and zero-config smoke from that export;
- scan smoke output for private paths and secret-like strings;
- publish only after an explicit operator instruction for the exact slice,
  tag, push, and GitHub release.

## User Language Contract

CLI, `--human`, `--agent`, and compact machine surfaces may use stable
agent-friendly English field labels such as `Structure`, `Technicals`, `SMT`,
`Regime`, and `Plan`. Those labels are the machine/agent contract.

When explaining output to a human operator, answer in the user's language and
translate the meaning, not the field contract. If the user writes Chinese,
respond in Chinese unless they explicitly ask otherwise.

## Current Release Gate

Do not publish, tag, push a release mirror, or tell the operator that the release
is ready until the closed-loop path below has fresh evidence in
`support/docs/plans/2026-05-12-hotplug-personal-data-release-handoff-todo.md`.

Required before release:
- zero-config first run works for an open-source consumer;
- no private API key, token, local file path, or maintainer-only dataset is
  required or leaked by default;
- no-config provider behavior falls back to Yahoo/yfinance-compatible defaults;
- explicitly configured richer/realtime providers are preferred only when the
  user opts in with `--profile`, provider config, env vars, or an explicit
  request;
- current/in-progress regime posterior probabilities are visible to users and
  agents, not just a completed regime label;
- the chain can be inspected through provider data, Pre-Bayes/filter, BBN,
  structural path-ranker/CatBoost artifacts, execution tree, and feedback/update
  learning;
- TimesFM remains optional hot-plug evidence only. Do not make it required, and
  do not keep it in the decision path unless it improves calibrated posterior
  quality in fresh evidence.

## Zero-Config Consumer Start

Use commands from a fresh shell and an explicit `/tmp` state directory when
checking consumer readiness:

```bash
cargo run --quiet -- provider-status --compact
cargo run --quiet -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --human
cargo run --quiet -- analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-first-run --human
cargo run --quiet -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --refresh --agent
cargo run --quiet -- pre-bayes-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --refresh --output-format json
cargo run --quiet -- policy-training-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --output-format agent
```

Rules:
- `--demo` is for first-run closure and smoke evidence; it is not trading proof.
- Use `--state-dir /tmp/...` for trials unless the operator explicitly asks for
  repo-local state.
- Keep `--human`, `--compact`, or `--agent` outputs token-friendly. Avoid
  `--inline-ledger` unless debugging a specific ledger issue.
- If a command fails or a surface is missing, record the exact command and
  blocker in the handoff TODO before changing code.

## Provider And Privacy Contract

Public surfaces must stay generic. The CLI must not assume the maintainer's
markets, paths, broker account, provider keys, strategy folder, or private data.

Provider policy:
- Default/no config: use public Yahoo/yfinance-compatible behavior where live
  data is needed and demo fixtures where the command is explicitly a demo.
- Configured provider: if the user provides an opt-in `--profile`, provider
  config, env var, local credential file, or explicit provider request, prefer
  the richer or timelier provider for the requested role.
- Missing provider: route agents through `provider-status --agent` and surface
  install/config guidance via workflow status or `human-next` text. Do not invent
  provider setup text in isolated command branches.
- Private profiles: examples under `support/examples/provider_profiles/` are opt-in
  references, not default runtime inputs.

Privacy rules:
- Never commit real API keys, tokens, account ids, secrets, private broker
  output, or absolute maintainer data paths.
- Do not expose `/Users/...`, local Downloads paths, or profile-specific paths in
  default human/agent output.
- Redacted path hints may appear only in explicit opt-in profile surfaces.
- Generated dependency workspaces, Auto-Quant clones, large experiment state, and
  provider caches do not belong in a public release.

## Closed-Loop Contract

The intended runtime order is:

1. Provider/data surface: `provider-status`, `market-data-harness`,
   `analyze-live`, or `analyze --demo`.
2. Regime posterior: `analyze` persists the canonical structural posterior into
   workflow state; `workflow-status --refresh --agent` must expose active regime,
   confidence, and probability distribution.
3. Pre-Bayes/filter: `pre-bayes-status --refresh` exposes evidence quality,
   filtered labels, soft evidence, and policy/bridge status.
4. BBN: belief evidence from the Pre-Bayes filter feeds the trading network and
   persisted workflow snapshot.
5. Structural path-ranker/CatBoost: export or register ranked structural-path
   targets with `export-structural-path-ranking-target`,
   `register-structural-path-ranking-trainer-artifact`,
   `apply-structural-path-ranking-external-scores`, and
   `enable-structural-path-ranking-runtime`.
6. Execution tree: `analyze` persists `execution_tree_trace.json` and execution
   tree artifacts; `workflow-status` must show whether the path-ranker score was
   visible or actually used.
7. Feedback/update: use `update --feedback-file` or explicit realized outcome
   fields to feed posterior context and structural feedback into learning state.
8. Training/refinement: `policy-training-status` and structural path-ranking
   exports are the read-only inspection points for whether outcomes can train or
   refine the network.

Do not collapse this chain into a chat-only claim. For any release or promotion
claim, provide commands and artifacts proving the relevant links ran.

## Regime Posterior Requirement

Agents must expose live/incomplete regime belief as a probability distribution.
Acceptable agent-facing fields include maps such as:

```text
trend=0.42 range=0.31 transition=0.19 other=0.08
```

If the regime is not fully formed, do not force a single label. Report the
candidate distribution, uncertainty, source evidence, and whether it is suitable
for execution, observation, or training only. Aggregate Sharpe is not enough;
regime accuracy and regime-conditioned win rate matter for promotion.

## TimesFM Policy

TimesFM is an optional forecast bridge (`src/python_bridge/timesfm.rs` and
`support/scripts/timesfm_forecast.py`). Treat it as hot-pluggable evidence:
- do not require Python or TimesFM for zero-config Rust CLI use;
- do not block consumer workflow if TimesFM is absent;
- only wire TimesFM into posterior or execution decisions when fresh validation
  shows calibrated posterior quality improves;
- if it does not improve the posterior, leave it disabled or remove that edge in
  a separate, evidenced slice.

## Agent Work Discipline

- Read and update
  `support/docs/plans/2026-05-12-hotplug-personal-data-release-handoff-todo.md` during
  this release/closed-loop lane.
- For Board A/B regime-confidence or profitability-factor work, start from the
  compact current-state docs, not the oversized historical logs:
  `support/docs/plans/2026-05-12-board-a-regime-state-current.md`,
  `support/docs/plans/2026-05-12-board-b-profit-factor-current.md`, and
  `support/docs/plans/2026-05-12-board-ab-cleanup-retention-plan.md`. Open the old
  May 10 append-only logs only for targeted evidence lookup by heading, root id,
  or exact artifact reference.
- Do not append routine coordination/readback rows to the old May 10 Board A/B
  logs. New status belongs in the compact current-state docs; detailed evidence
  belongs in compact run-root packets under `support/docs/experiments/.../materials`,
  `summaries`, and `checks`.
- For Board A/B high-concurrency work, do not use repo markdown as a lock table
  or scratchpad. Start claims belong outside the repo, for example under
  `/tmp/ict-engine-agent-claims/board-a/` or `/tmp/ict-engine-agent-claims/board-b/`.
  The compact current-state docs should receive only terminal decisions with
  evidence paths.
- Docs are not runtime inputs. Rust, Python, shell, provider, Auto-Quant,
  training, and workflow code must not import, parse, grep, or depend on
  `support/docs/plans/*.md`. Promote any needed rule into typed config, command flags,
  schemas, fixtures, or tests before code consumes it.
- Preserve unrelated dirty worktree changes. Stage only files touched for the
  current coherent slice.
- Prefer `/tmp/...` for smoke state and generated artifacts.
- Use `rg`/`rg --files` for discovery and small, focused tests for verification.
- Public CLI/workflow surfaces must remain ontology-free and consumer-usable.
- If provider evidence is missing, enumerate Yahoo/yfinance, IBKR,
  TradingView/MCP, Kraken/public crypto, repo-local fixtures, and Auto-Quant
  artifacts before calling the lane data-blocked.
- Commit only verified, relevant slices. Do not release without a clean export
  and explicit operator confirmation.

## Factor Traceability

### Code-Level Factor Categories (Rust enum `FactorCategory`)

| Rust Enum Variant | snake_case key | Family (TODO doc) | Code Location | Status |
|---|---|---|---|---|
| `TrendMomentum` | `trend_momentum` | Family B | `src/factor_lab/factor_definition.rs:365` | active |
| `VolatilityMeanReversion` | `volatility_mean_reversion` | Family D (partial) | `src/factor_lab/factor_definition.rs:380` | active |
| `StructureIct` | `structure_ict` | Family A | `src/factor_lab/factor_definition.rs:396` | active |
| `CrossMarketSmt` | `cross_market_smt` | Family C | `src/factor_lab/factor_definition.rs:420` | active |
| `OptionsHedging` | `options_hedging` | Family G (partial) | `src/factor_lab/factor_definition.rs:431` | active |
| `CrowdingHerding` | `crowding_herding` | Family E | `src/factor_lab/factor_definition.rs` | active (compute stub) |
| `SpectralRhythm` | `spectral_rhythm` | Family F | `src/factor_lab/factor_definition.rs` | active (compute stub) |
| `SessionLiquidity` | `session_liquidity` | Family H | `src/factor_lab/factor_definition.rs` | active (compute stub) |

### Design-Level Factor Families (from execution-tree TODO)

| Family | Name | Mapped Category | TODO Section | Code Gap |
|---|---|---|---|---|
| A | Structure / Setup Quality | `StructureIct` | Family A | code covers ICT setups only; no crowding/setup-quality subfactors |
| B | Directionality / Persistence | `TrendMomentum` | Family B | code covers EMA+RSI+ADX; no continuation-failure/exhaustion subfactors |
| C | Cross-Market Confirmation | `CrossMarketSmt` | Family C | code covers SMT; no leader-laggard/correlation-consistency subfactors |
| D | Stretch / Reversion Feasibility | `VolatilityMeanReversion` | Family D | code covers Bollinger+ATR; no OU-reversion/exhaustion subfactors |
| E | Crowding / Herding Execution Risk | `CrowdingHerding` | Family E | compute stub exists; no subfactors beyond stub |
| F | Spectral Rhythm / Chaos | `SpectralRhythm` | Family F | compute stub exists; spectral_entropy in execution tree inputs but stub only |
| G | Options / Dealer Positioning | `OptionsHedging` | Family G | compute path exists but requires `--auxiliary-evidence` data |
| H | Session / Liquidity Window Quality | `SessionLiquidity` | Family H | compute stub exists; no subfactors beyond stub |

### Key Source Paths

- Factor definitions + compute: `src/factor_lab/factor_definition.rs`
- Factor registry (5 hardcoded factors): `src/factors/registry.rs`
- Factor engine (orchestration): `src/factor_lab/engine.rs`
- Factor lifecycle / autoresearch / mutation: `src/application/factor_lifecycle/`
- Regime-conditional evaluation: `src/factors/regime_conditional.rs`
- Execution tree (factor consumer): `src/application/orchestration/execution_tree.rs`
- BBN evidence (factor consumer): `src/bbn/evidence.rs`
- HMM/regime (factor consumer): `src/application/regime/`

### State Directories

Pattern: `state/<SYMBOL>/` for production, `state_<experiment>/` for isolated runs.
All state dirs are `/tmp/...` by default via `--state-dir` flag. Zero-config: `cargo run --quiet -- analyze --demo --human`.

### Why Agents Say "No Factors"

1. No AGENTS.md existed before this file — agents had no entry map
2. Factor families A-H are documented only in `support/docs/plans/2026-05-05-execution-tree-factor-auto-quant-todo.md` (5590 lines) — agents cannot scan a 436KB doc efficiently
3. Families E, F, H now have `FactorCategory` enum variants and compute stubs — previously missing
4. Factor compute paths are split across `factor_lab/` and `factors/` — grep for "factor" hits 20+ files with no index

## Hot-Plug Convention

External factor families (Auto-Quant workspace) do NOT need a `FactorCategory` enum variant
to be usable. The Auto-Quant backend (`--backend auto-quant`) authors strategies outside this repo.
The Rust registry is a bootstrap seed, not the design boundary.

To add a new family to the Rust registry:
1. Add variant to `FactorCategory` enum in `factor_definition.rs`
2. Add `fn <variant>() -> FactorDefinition` constructor
3. Register in `FactorRegistry::default()` in `factors/registry.rs`
4. Add compute path in `factor_definition.rs` `evaluate_*` methods
5. Update this AGENTS.md traceability table

## Architecture Rules

- Zero-config default: `ict-engine analyze --demo --human` works with no env vars
- Token-friendly: `--human` flag for compact desk-style output; `--compact` for machine output
- No pollution: all state dirs are explicit `--state-dir` or `/tmp/...`
- No debt: this file must stay current; stale entries must be pruned
- Auto-Quant isolation: Auto-Quant output always lands in `<state-dir>/auto-quant/` subdirectory, never in repo root. Override with `ICT_ENGINE_AUTO_QUANT_OUTPUT_DIR` env var for custom location. Hot-pluggable: user can disable any family via optional config; engine gracefully skips
