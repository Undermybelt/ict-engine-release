# 2026-05-08 Whole Repo Limitations And Keywords

## Scope

- Repo audited from the current `green-baseline` working tree after structural cleanup work.
- Baseline gate used: `cargo clippy --all-targets -- -D warnings`
- Surface probes used:
  - `cargo run --quiet -- --help`
  - `cargo run --quiet -- workflow-status --symbol NEWSYM --state-dir /tmp/ict-engine-audit-smoke --human`
  - `cargo run --quiet -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-audit-smoke --agent`
  - `cargo run --quiet -- analyze --symbol DEMO --demo --human`
  - `cargo run --quiet -- factor-backtest --symbol DEMO --data examples/demo/demo-15m.json --human`
  - `cargo run --quiet -- factor-autoresearch-status --symbol DEMO --state-dir /tmp/ict-engine-audit-smoke`
  - `cargo run --quiet -- research-verdict --symbol DEMO --state-dir /tmp/ict-engine-audit-smoke`

## What Is Good Now

- Whole-repo `clippy -D warnings` baseline is clean on the current working tree.
- Public workflow/provider surfaces preserve zero-config default while supporting opt-in provider-profile reuse.
- First-run human and agent `workflow-status` surfaces now keep profile-aware provider/bootstrap/factor routes when the personal lane is selected.
- Empty autoresearch state now emits an explicit ask-user contract instead of a fake runnable command missing `--data`.
- Non-matching symbols no longer get an unrelated personal-lane hint on human first-run output.

## Remaining Limitations

- Historical-data bootstrap is still manual.
  - `factor-research`, `factor-backtest`, and `factor-autoresearch` still rely on the user or caller to provide a valid cleaned candle file path.
  - The repo gives good next-step hints, but it does not yet provide a fully zero-config historical data acquisition path for arbitrary symbols.

- Empty-state workflow routing is still generic rather than symbol-intent aware.
  - `workflow-status` can route users toward replay / factors / live, but it does not yet infer which route is most appropriate from symbol family, available local datasets, or recent intent.
  - This is functional, but not yet highly adaptive.

- Personal provider-profile reuse is still selector-based, not persisted workflow identity.
  - The repo now preserves selected profiles across many follow-up routes, but it still depends on explicit `--profile` threading or render-time promotion rather than a persisted workflow/session identity model.
  - That keeps the public default clean, but it also means deeper future features may need more explicit context plumbing.

- Research-state truth remains sparse on fresh state dirs.
  - `research-verdict` and `factor-autoresearch-status` are structurally useful, but on fresh state they mainly tell you there is no research state yet.
  - The repo is honest here, but the user experience is still “bootstrap required” rather than “guided warm start”.

- Auto-Quant integration is still handoff-centric.
  - The managed Auto-Quant path is much cleaner than before, but it is still fundamentally a handoff / coordination boundary.
  - That means strategy iteration, retrospective scoring, and candidate adoption still require explicit artifact flow and external execution steps.

- Provider readiness breadth is better surfaced than it is automatically resolved.
  - The system tells the user which providers/tracks are pending, but setup-required lanes like TradingView MCP, external runtimes, IBKR bridge, or options-enriched workflows still depend on external operator/runtime readiness.

- Symbol-profile matching is still contract-label based, not semantic/fuzzy.
  - The fix now avoids showing obviously unrelated personal lanes for non-matching symbols.
  - But matching is still based on declared `data_contracts.symbols`, not broader semantic symbol families, aliases, or market-group reasoning.

- `main.rs` / command surface breadth is still large.
  - The repo is structurally healthier than before, but the top-level command surface remains broad and advanced.
  - Discoverability is improved, yet still demands a technically strong caller to navigate many lanes well.

- Audit coverage is still sample-based, not exhaustive runtime proof.
  - Current evidence covers representative public surfaces and core structural gates.
  - It does not prove every command/phase/provider combination is perfect; especially deep cross-product combinations remain a future audit area.

## Pain Points

- New users still need to understand when they must provide data files versus when the repo can proceed zero-config.
- Advanced routes expose strong primitives, but some “recommended next” flows still assume the user understands the surrounding workflow vocabulary.
- Provider/profile behavior is now much better, but there is still conceptual weight around tracks, contracts, bootstrap, and external runtimes.
- Empty-state honesty is good, but the repo can still feel “capable but not fully self-starting” on brand-new symbols and fresh state dirs.
- Research and autoresearch surfaces can be correct while still feeling underpowered when no prior artifacts exist.
- Public surfaces remain clean, but deeper artifact and structural-belief lanes still require substantial mental model depth.

## Solution Direction Keywords

- zero-config market data bootstrap
- historical dataset auto-discovery
- symbol-to-dataset resolution
- workflow intent inference
- adaptive first-run routing
- profile/session context persistence
- opt-in context propagation
- provider capability negotiation
- external runtime orchestration
- artifact-first execution contracts
- progressive disclosure CLI UX
- self-describing recommended-next-command schema
- ask-user contract normalization
- warm-start research bootstrap
- cold-start structural priors
- semantic symbol matching
- market family ontology mapping
- options enrichment orchestration
- local-plus-remote data fusion
- policy correction calibration
- delayed reward replay validation
- structural path ranking calibration
- contextual bandit / off-policy evaluation
- doubly robust estimation
- conformal execution gating
- belief network evidence weighting
- HMM regime continuity
- changepoint-aware duration modeling
- multi-timeframe resonance filtering
- factor family breadth screening
- cross-market generalization
- trade-density-aware search
- white-box factor generation
- factor-to-evidence pipeline design
- factor filtering before belief node evidence
- CatBoost path ranking
- execution-tree action arbitration

## If You Reopen This Audit

- Re-run `cargo clippy --all-targets -- -D warnings` first.
- Re-run the exact CLI surface probes listed at the top of this document.
- Prioritize real user/agent misroutes before deeper internal refactors.
- Treat this document as a live limitations register, not a one-off report.
