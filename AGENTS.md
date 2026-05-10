# ICT Engine — Agent Entry Map

This file is the first thing any AI agent should read when entering this repo.
It maps the factor landscape so agents cannot claim "no usable factors exist."

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
All state dirs are `/tmp/...` by default via `--state-dir` flag. Zero-config: `./target/debug/ict-engine analyze --demo --human`.

### Why Agents Say "No Factors"

1. No AGENTS.md existed before this file — agents had no entry map
2. Factor families A-H are documented only in `docs/plans/2026-05-05-execution-tree-factor-auto-quant-todo.md` (5590 lines) — agents cannot scan a 436KB doc efficiently
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
