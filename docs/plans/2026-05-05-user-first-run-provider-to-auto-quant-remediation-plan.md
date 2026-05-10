# ICT Engine User-First Provider To Auto-Quant Execution Board

> Authoritative execution board for closing the first-run provider-to-auto-quant UX gaps found in the 2026-05-05 user-view trial. Keep this file as the live reconciliation board for this workstream instead of creating parallel remediation docs.

**Goal:** make a first-time `ict-engine` user or agent able to start from the CLI, choose a sensible provider path, understand replay/backtest/live as distinct routes, enter factor iteration or Auto-Quant when strategy is underspecified, and stay guided through factor/regime/evidence/execution-tree closure.

**Source Docs:**
- `docs/plans/2026-05-05-user-first-run-provider-to-auto-quant-todo.md`
- `docs/bug/0.1.0/2026-05-05-user-first-run-provider-to-auto-quant-bug-log.md`
- `docs/bug/0.1.0/2026-05-05-user-first-run-provider-to-auto-quant-trial-report.md`

**Do Not Reopen By Default:** provider implementation scope expansion, new market-data backends, factor-model math changes, BBN scoring redesign, or broader architecture cleanup outside the first-run path.

---

## Hard Constraints

- Keep `workflow-status` as the primary first-run control plane instead of inventing a parallel onboarding command.
- Preserve zero-config and low-pollution behavior. All validation and trial paths must keep explicit `/tmp/...` state-dir usage viable.
- Do not leak maintainer-local profiles, paths, or state assumptions into default user-facing surfaces.
- Keep consumer surfaces ontology-light and token-friendly.
- Human, compact, and agent outputs must remain contract-consistent across native and Auto-Quant paths.
- Prefer extraction and reuse of existing repo surfaces over adding new one-off wrappers.
- For the current Auto-Quant/provider closure scope, treat the target provider set as:
  - `ibkr`
  - `tradingview_mcp`
  - `yfinance`
  - public crypto providers (`binance_public`, `bybit_public`, `kraken_public`, related crypto runtime helpers)
  - prediction-market providers (`polymarket_public`)
- Treat `openbb`, `openalice`, and `nofx` as reference-only historical baselines from earlier comparison work, not as the current target provider set for this repo.
- Do not treat built-in `entry_model` registry members as providers.
  - If `provider-status` still exposes an `entry_model` domain, treat that as terminology residue to remove, not as a legitimate provider category.

## Current Diagnosis

The trial findings collapse into four root problems:

1. The CLI has no real first-run router.
   - `--help` is a flat command list.
   - `workflow-status` empty state dead-ends.

2. Provider surfaces expose machine state, not user choice.
   - users cannot tell free/public/login-required paths apart
   - crypto vs tradfi guidance is implicit

3. Output contracts diverge by path.
   - native `factor-research --human` is readable
   - Auto-Quant `factor-research --human` prints JSON handoff blobs
   - `workflow-status` loses the handoff afterward

4. Evidence artifacts are not translated into “what to do next”.
   - factor/regime/execution-tree/BBN outputs exist
   - human surfaces do not route users into them clearly

## Fix Order

Land in this order:

1. First-run router in `workflow-status`
2. Provider semantic layer in `provider-status`
3. Native and Auto-Quant output-contract unification
4. Live/backtest first-run path repair
5. Evidence-loop human guidance

Reason:

- `1` and `3` remove the biggest dead ends first
- `2` makes provider decisions legible
- `4` and `5` are lower-risk once routing and output contracts are stable

## Current Todo Board

### Done

- [x] User-view trial completed from real CLI surfaces with isolated `/tmp/...` state dirs.
- [x] Bugs, blockers, and misleading steps logged into the repo bug log.
- [x] Root-cause grouping established:
  - entry/router failure
  - provider semantic failure
  - output-contract failure
  - evidence-loop translation failure
- [x] Key owner files identified:
  - `src/application/orchestration/workflow_status.rs`
  - `src/application/provider_catalog.rs`
  - `src/factor_research_command.rs`
  - `src/application/auto_quant/command_entry.rs`
  - `src/analyze_live_command.rs`
  - `src/application/reporting/backtest_output.rs`
  - `src/application/reporting/analyze_output.rs`
- [x] **Workstream 1: First-Run Router**
  - empty-state `workflow-status` now routes users into a real start-here flow instead of `No actionable command available.`
  - human output now shows:
    - provider-first next step via `ict-engine provider-status --compact`
    - explicit replay / factors-backtest / live-bootstrap routes
  - agent output now exposes `first_run_router`, a bootstrap command, and a non-null `recommended_next_step`
  - default opt-in local profile hints are hidden on first-run workflow surfaces unless the user explicitly selects `--profile`
- [x] **Workstream 2: Provider Semantics**
  - `provider-status` surfaces now carry:
    - `user_access`
    - `market_fit`
    - `fallback_priority`
    - `summary`
  - `yfinance` is now explicitly exposed as the free tradfi fallback
  - `bybit_public` / `binance_public` / `kraken_public` are now explicitly exposed as public no-login crypto paths
  - `ibkr` / `tradingview_mcp` are now explicitly exposed as setup-required paths
  - default compact/agent outputs stop leaking maintainer-local opt-in profiles
- [x] Workstream 1/2 verification evidence captured:
  - `cargo test provider_catalog --lib -- --nocapture`
  - `cargo test empty_state_uses_explicit_no_state_contract --lib -- --nocapture`
  - `cargo test agent_bootstrap_profile_can_reuse_profile_path_hint_without_detected_root --lib -- --nocapture`
  - `cargo test --test provider_neutral_cli -- --nocapture`
  - live CLI spot checks:
    - `./target/debug/ict-engine provider-status --compact`
    - `./target/debug/ict-engine provider-status --provider yfinance --agent`
    - `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-regress-demo-ws1b --human`
    - `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-regress-demo-ws1c --agent`
- [x] **Workstream 3: Auto-Quant Contract Parity**
  - `factor-research --backend auto-quant` now receives the resolved output mode instead of hard-printing the raw handoff JSON
  - Auto-Quant handoff output now exposes:
    - `recommended_next_command`
    - `recommended_next_step`
    - `review_command`
    - `workflow_status_command`
    - `human_output`
  - human output is now short text with the prepare/review/workflow loop instead of a raw JSON blob
  - `workflow-status` now prefers `auto_quant_handoff_candidate` over the generic first-run router when no phase history exists yet
  - agent output now surfaces `auto_quant_handoff` and routes `recommended_next_step` to the handoff's next command
- [x] Workstream 3 verification evidence captured:
  - `cargo test auto_quant_handoff_human_output_is_short_text_not_json_dump --lib -- --nocapture`
  - `cargo test workflow_status_routes_auto_quant_handoff_candidate_before_first_run_router --lib -- --nocapture`
  - live CLI spot checks:
    - `./target/debug/ict-engine factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-aq-ws3-live2 --backend auto-quant --human`
    - `./target/debug/ict-engine factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-aq-ws3-live2-agent --backend auto-quant --agent`
    - `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-aq-ws3-live2 --human`
    - `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-aq-ws3-live2-agent --agent`
- [x] **Workstream 4: Live / Backtest Entry Repair**
  - `analyze-live` now reuses catalog live defaults when the symbol already maps to a known market key such as `NQ`
  - missing-default cases now return guided errors with:
    - explicit `--futures-symbol` / `--spot-symbol` / `--options-symbol` hints
    - provider-first guidance via `ict-engine provider-status --domain live_runtime --agent`
    - concrete market-key examples (`NQ`, `ES`, `GC`, `CL`)
  - `backtest` short-history failure now routes the user toward:
    - `factor-backtest`
    - `analyze --demo`
    - fetching a longer dataset via the provider route
- [x] **Workstream 5: Evidence-Loop Translation**
  - post-research `workflow-status --human` now routes users into:
    - `workflow-status --phase ensemble-vote`
    - `pre-bayes-status`
    - `workflow-status --phase structural-recommended-path-bundle`
  - human output stops blindly echoing another `factor-research` command when review surfaces are more useful
  - agent output now exposes `evidence_review` and can route `recommended_next_step` through the evidence review router
- [x] Workstream 4/5 verification evidence captured:
  - `cargo test backtest_command_wraps_short_history_error_with_guidance --lib -- --nocapture`
  - `cargo test resolve_live_symbol_inputs_infers_catalog_defaults_for_nq --bin ict-engine -- --nocapture`
  - `cargo test resolve_live_symbol_inputs_guides_when_defaults_are_missing --bin ict-engine -- --nocapture`
  - `cargo test workflow_status_routes_research_users_into_evidence_review_before_rerun --lib -- --nocapture`
  - live CLI spot checks:
    - `./target/debug/ict-engine analyze-live --symbol NQ --state-dir /tmp/ict-engine-live-ws4`
    - `./target/debug/ict-engine backtest --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-backtest-ws4 --human`
    - `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-ws5-native --human`
- [x] Completion audit rerun against the real provider call matrix and provider→Auto-Quant bridge.
  - audit scope:
    - external/provider-runtime call surfaces only
    - `entry_model` domain treated as a naming bug/residue, not as a legitimate provider transport category
    - current target provider set only:
      - `ibkr`
      - `tradingview_mcp`
      - `yfinance`
      - public crypto / prediction-market providers
    - `openbb`, `openalice`, and `nofx` are not treated as current target providers for closure
  - internal-registry boundary confirmed with:
    - `./target/debug/ict-engine provider-status --domain entry_model --agent`
    - result:
      - `breaker_rb_long_v1` and `cisd_rb_long_v1` are `access_mode=internal_model_registry`
      - `selectable_by_user=false`
      - conclusion:
        - this is evidence of terminology drift in `provider-status`, not evidence of an extra provider family
  - market-data real-call evidence:
    - `./target/debug/ict-engine market-data-harness --action fetch --market NQ --interval 1d --role etf_reference --provider etf_reference=yfinance --symbol-spec etf_reference=QQQ`
      - passed
      - returned real `QQQ` daily candles
    - `./target/debug/ict-engine market-data-harness --action fetch --market NQ --interval 1d --role options_underlying --provider options_underlying=yfinance --symbol-spec options_underlying=QQQ`
      - failed
      - `yahoo crumb endpoint returned error`
    - `./target/debug/ict-engine market-data-harness --action fetch --market NQ --interval 1d --role options_underlying --provider options_underlying=tradingview_mcp --symbol-spec options_underlying=NASDAQ:QQQ`
      - failed as expected
      - `ICT_ENGINE_TVREMIX_MCP_API_KEY must be set for tradingview_mcp`
    - `printf '%s' '{"market_key":"NQ","interval":"1d","related_roles":["etf_reference"],"provider_preferences":{"etf_reference":"ibkr"},"symbol_overrides":{"etf_reference":{"display_symbol":"QQQ","ibkr":{"symbol":"QQQ","sec_type":"STK","exchange":"SMART","currency":"USD"}}}}' | ./target/debug/ict-engine market-data-harness --action fetch --request-stdin`
      - explicit `ibkr` `QQQ` contract
      - failed after reaching the real fetcher
      - `ibkr-historical requires the ibkr_bridge package`
      - underlying dependency gap: `No module named 'redis'`
  - public crypto / prediction-market external-fetch evidence:
    - `python3 scripts/auto_quant_external/fetch_external.py binance-kline --symbol BTCUSDT --interval 1h --start 2026-05-01 --end 2026-05-03 --output /tmp/ict-engine-provider-binance.csv`
      - passed
      - `49` rows
    - `python3 scripts/auto_quant_external/fetch_external.py bybit-kline --category linear --symbol BTCUSDT --interval 1h --start 2026-05-01 --end 2026-05-03 --output /tmp/ict-engine-provider-bybit.csv`
      - passed
      - `49` rows
    - `python3 scripts/auto_quant_external/fetch_external.py kraken-kline --market spot --pair XBTUSD --interval 1h --output /tmp/ict-engine-provider-kraken.csv`
      - passed
      - `721` rows
    - `python3 scripts/auto_quant_external/fetch_external.py polymarket-markets --limit 3 --format json --output /tmp/ict-engine-provider-polymarket.json`
      - passed
      - `3` rows
  - local-runtime real-call evidence:
    - `python3 -m scripts.ibkr_bridge.setup status`
      - callable
      - consent present
      - selected candidate: `IB Gateway paper (127.0.0.1:4002)`
      - still blocked by missing python `redis` package
    - `kraken auth show -o json`
      - passed
      - returned masked configured credentials
  - reference-only baseline note:
    - a one-off `openbb` bridge sanity check was run during the first draft of this audit
    - after user correction, that evidence is explicitly excluded from current target-provider closure because `openbb/openalice/nofx` are not the intended provider set for this repo
- [x] Neutralized legacy live-runtime naming without deleting the useful capability layer.
  - landed provider-neutral/shared market support in:
    - `src/data/realtime/market_support.rs`
  - live runtime surfaces now prefer:
    - `yfinance`
    - `external_http_runtime`
    - `crypto_public_runtime`
  - public `analyze-live --help` no longer defaults to `openbb`; it now defaults to `yfinance`
  - provider/workflow public summaries now stop treating `openbb/openalice/nofx` as the preferred visible names
  - compatibility note:
    - superseded by the 2026-05-06 follow-up below
  - fresh verification evidence:
    - `cargo check`
    - `cargo test provider_catalog --lib -- --nocapture`
    - `cargo test resolve_live_backend_base_url_uses_expected_sources --lib -- --nocapture`
    - `cargo test analyze_live_command_input_carries_backend_and_symbols --lib -- --nocapture`
    - `cargo run --quiet -- analyze-live --help`
    - `cargo run --quiet -- provider-status --compact`
    - `cargo run --quiet -- provider-status --provider yfinance --compact`
- [x] Cleared remaining live-runtime legacy aliases and physical filename shells.
  - removed legacy backend parser aliases from:
    - `src/data/realtime/live_data.rs`
    - `src/application/data_sources/live_defaults.rs`
  - renamed physical runtime source files to match current capability names:
    - `src/data/realtime/yfinance_runtime.rs`
    - `src/data/realtime/external_http_runtime.rs`
    - `src/data/realtime/crypto_public_runtime.rs`
  - removed `#[path = "..."]` shims from `src/data/realtime/mod.rs`
  - source-surface grep evidence:
    - `rg -n "openbb|openalice|nofx|OpenAlice|OpenBB|Nofx" src tests examples`
    - result: no hits

### Completion Audit 2026-05-06

- Verified-good external/provider-runtime calls in this environment:
  - `yfinance` candle fetch
  - `binance_public`
  - `bybit_public`
  - `kraken_public`
  - `polymarket_public`
  - `kraken_cli`
- Verified-real but currently blocked/degraded calls:
  - `yfinance` options summary
  - `tradingview_mcp`
  - `ibkr`
  - `ibkr_bridge`
- Unverified by design in this pass:
  - `entry_model` registry members as external providers
  - they are internal registry items, and the fact that they appear under `provider-status` should be treated as naming debt rather than provider coverage
  - `openbb`, `openalice`, `nofx`
  - they are reference-only historical baselines, not the current target provider set for this repo
- Deletion feasibility audit:
  - the earlier one-shot deletion blocker has been reduced to historical-doc residue, not runtime source residue
  - current runtime source uses:
    - `YahooFinanceProvider`
    - `ExternalHttpRuntimeProvider`
    - `CryptoPublicRuntimeProvider`
  - current runtime parser inputs are:
    - `yfinance`
    - `external_http` / `external_http_runtime`
    - `crypto_public` / `crypto_public_runtime`
  - old upstream project names remain only in historical docs/plans/audits that explain prior baselines and corrections

### Next

- [ ] Remove `entry_model` from provider terminology.
  - preferred outcome:
    - built-in model registry stops appearing as a `provider-status` domain
  - minimum acceptable fallback:
    - if temporary compatibility requires it to remain exposed, label it as internal registry residue instead of provider truth
- [x] Remove the remaining compatibility residue for `openbb/openalice/nofx` after downstream surfaces stabilize.
  - remove parser aliases once no first-party surface emits them
  - rename physical source filenames so internal grep no longer suggests the upstream project names
- [x] Replace `openbb` / `openalice` / `nofx` branding with provider-neutral naming before any deletion attempt.
  - target outcome:
    - consumer/user-facing surfaces expose capability/provider names such as `yfinance`, `ibkr`, `tradingview_mcp`, crypto public providers, and prediction-market providers
    - internal adapter/module names no longer leak upstream project branding
- [x] Extract provider-neutral shared market-evidence types out of `openalice`.
  - minimum extraction set:
    - `AuxiliaryMarketEvidence`
    - `OptionsChainSummary`
    - `SpotInstrumentKind`
    - `Quote`
- [x] Replace the current `OpenBBProvider`-backed `yfinance` implementation with a provider-neutral `yfinance` adapter boundary.
  - only after that is in place should `openbb` stop existing as a visible backend name
- [x] Remove or explicitly demote reference-only baseline providers from the current provider closure language and future audit scope.
  - `openbb`
  - `openalice`
  - `nofx`
- [ ] Split provider readiness by capability, not only by provider id.
  - `yfinance` candle fetch is proven here
  - `yfinance` options summary still fails on Yahoo crumb
  - the first-run surface should not imply those two capabilities are equally ready
- [ ] Close the `ibkr` / `ibkr_bridge` dependency gap.
  - current real blockers:
    - missing python `redis`
    - local runtime only partially available even though gateway paper `127.0.0.1:4002` is reachable
- [ ] Re-run `tradingview_mcp` after a real `ICT_ENGINE_TVREMIX_MCP_API_KEY` is supplied.
  - current result only proves the guard/error path
- [ ] Decide whether `kraken_cli` belongs in this first-run provider→Auto-Quant closure scope or should remain documented as a later wallet/runtime lane.

### Not Yet

- [ ] Broader command-surface cleanup for top-level `--help`.
  - This should wait until `workflow-status` is a credible first-run router; otherwise wording cleanup will drift.

- [ ] Provider-profile polish beyond first-run safety.
  - Do not spend this lane on richer profile UX until default surfaces stop leaking local profile assumptions.

- [ ] Broader Auto-Quant product-surface cleanup outside first-run.
  - Keep the current slice narrowly focused on `factor-research` handoff readability and workflow-status continuity.

- [ ] Deeper evidence-surface rearchitecture.
  - No new artifact families or belief-model expansions in this slice; just translate the existing ones into user-facing guidance.

## File Ownership By Workstream

### Workstream 1: First-Run Router

- `src/application/orchestration/workflow_status.rs`
- `src/status_command.rs`

Primary target:

- empty-state human and agent rendering
- bootstrap-phase reuse
- first-run next-step generation

### Workstream 2: Provider Semantics

- `src/application/provider_catalog.rs`

Primary target:

- provider item semantics
- compact and agent rendering
- workflow-provider-support inputs

### Workstream 3: Auto-Quant Contract Parity

- `src/factor_research_command.rs`
- `src/application/auto_quant/command_entry.rs`
- `src/application/auto_quant/readiness.rs`
- `src/application/reporting/backtest_output.rs`

Primary target:

- Auto-Quant `factor-research` output-mode handling
- handoff payload rendering
- workflow-status continuity after handoff

### Workstream 4: Live / Backtest Entry Repair

- `src/analyze_live_command.rs`
- `src/application/data_sources/live_defaults.rs`
- `src/application/backtest/command_entry.rs`

Primary target:

- inferred symbol defaults
- guided live errors
- backtest short-data recovery guidance

### Workstream 5: Evidence-Loop Translation

- `src/application/orchestration/workflow_status.rs`
- `src/application/reporting/analyze_output.rs`

Primary target:

- human explanation of actionable artifacts
- mapping factors/regime/evidence/execution surfaces into concrete next steps

## Verification Commands

Run these after each relevant slice, not only at the end:

```bash
cargo check
cargo test workflow_status -- --nocapture
cargo test provider_catalog -- --nocapture
cargo test factor_research -- --nocapture
cargo test auto_quant -- --nocapture
cargo test analyze_live -- --nocapture
```

Run these user-facing regressions before calling the board “closed”:

```bash
cargo run -- --help
cargo run -- provider-status --compact
cargo run -- provider-status --agent
cargo run -- provider-status --provider yfinance --agent
cargo run -- provider-status --provider bybit_public --agent
cargo run -- provider-status --provider binance_public --agent
cargo run -- provider-status --provider kraken_public --agent
cargo run -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-regress-demo --human
cargo run -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-regress-demo --agent
cargo run -- analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-regress-demo --human
cargo run -- factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-regress-demo --human
cargo run -- analyze-live --symbol NQ --state-dir /tmp/ict-engine-regress-live
cargo run -- backtest --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-regress-backtest --human
```

## Closure Standard

This board is not done until all of the following are true:

- a first-time user can tell where to start from CLI surfaces alone
- missing-provider cases produce actionable guidance rather than guesswork
- tradfi and crypto data paths are both explicit
- replay/backtest/live are discoverable as separate choices
- Auto-Quant `--human` output is actually human-readable
- `workflow-status` keeps the loop after Auto-Quant handoff
- human surfaces route users into factor/regime/evidence/execution-tree review instead of leaving those artifacts implicit

## Blocked

- `entry_model` terminology drift
  - current user correction: `entry_model` should not be called a provider
  - current repo evidence:
    - `./target/debug/ict-engine provider-status --domain entry_model --agent`
    - returned `breaker_rb_long_v1` and `cisd_rb_long_v1` under provider-shaped output even though both are built-in registry members
  - acceptable temporary state:
    - keep the audit scope excluding them from provider closure
    - but treat the naming itself as remediation debt, not as settled semantics
- Target-provider scope drift
  - current user correction: `openbb`, `openalice`, and `nofx` were earlier reference/baseline backends, not the intended provider set for this repo's current Auto-Quant closure
  - acceptable temporary state: keep historical mentions as implementation residue only, but do not count them in provider closure claims or future audit success/failure summaries
- `yfinance` options-capability parity
  - failing command:
    - `./target/debug/ict-engine market-data-harness --action fetch --market NQ --interval 1d --role options_underlying --provider options_underlying=yfinance --symbol-spec options_underlying=QQQ`
  - failing evidence:
    - `yahoo crumb endpoint returned error`
- `tradingview_mcp` real invocation
  - failing command:
    - `./target/debug/ict-engine market-data-harness --action fetch --market NQ --interval 1d --role options_underlying --provider options_underlying=tradingview_mcp --symbol-spec options_underlying=NASDAQ:QQQ`
  - failing evidence:
    - `ICT_ENGINE_TVREMIX_MCP_API_KEY must be set for tradingview_mcp`
- `ibkr` / `ibkr_bridge` real fetch/runtime closure
  - failing commands:
    - `printf '%s' '{"market_key":"NQ","interval":"1d","related_roles":["etf_reference"],"provider_preferences":{"etf_reference":"ibkr"},"symbol_overrides":{"etf_reference":{"display_symbol":"QQQ","ibkr":{"symbol":"QQQ","sec_type":"STK","exchange":"SMART","currency":"USD"}}}}' | ./target/debug/ict-engine market-data-harness --action fetch --request-stdin`
    - `python3 -m scripts.ibkr_bridge.setup status`
  - failing evidence:
    - `No module named 'redis'`
    - `ibkr historical fetch failed for 'QQQ'`
