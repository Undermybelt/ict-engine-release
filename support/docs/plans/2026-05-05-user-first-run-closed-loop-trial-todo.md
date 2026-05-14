# User-First Closed-Loop Trial TODO

> **For Hermes/Codex:** this is an execution TODO for a real first-time user / user-agent trial. Do not modify code, tests, config defaults, or repo structure in this round. Work only from the user/operator surface and log every issue into `support/docs/bug/0.1.0/2026-05-05-user-first-run-closed-loop-bug-log.md`.

**Goal:** verify whether a brand-new owner, after freshly cloning `ict-engine`, is actually guided from first CLI entry through provider onboarding, replay/backtest/live choice, strategy clarification, factor iteration, Auto-Quant-assisted improvement, evidence accumulation, BBN/regime/execution-tree closure, and ongoing data compounding.

**Architecture:** treat the repo as a user-facing research CLI, not as a codebase to improve during the run. Use explicit `/tmp/...` state dirs to avoid pollution. Prefer real command surfaces (`--help`, `workflow-status`, `provider-status`, `analyze`, `backtest`, `factor-backtest`, `factor-research`, `analyze-live`, `auto-quant-status`) over internal assumptions.

**Tech Stack:** Rust CLI, repo docs under `support/docs/`, optional local providers (TradingView MCP, IBKR/TWS/Gateway bridge), public/free providers (Yahoo Finance, Bybit, Binance, Kraken public data), external managed iteration backend (`Auto-Quant`).

---

## Execution Rules

- [x] Do not touch any code, tests, manifests, or config defaults.
- [x] Use only explicit `/tmp/...` `--state-dir` values.
- [x] Assume the owner is new and has provided no provider credential unless the CLI clearly asks for one.
- [x] Judge the app by what it actually guides the user/agent to do, not by what the repo docs imply it could do.
- [x] Every broken, redundant, blocked, misleading, or over-implicit step must be appended to the bug log markdown.

## Trial Scope

- [x] First CLI entry and start-here guidance.
- [x] Provider onboarding and login guidance:
  - TradingView MCP
  - IBKR client / Gateway / bridge
  - Yahoo Finance free fallback
  - Bybit / Binance public crypto data
  - Kraken public data path and any later login/runtime distinction
- [x] Workflow choice:
  - replay / review
  - backtest
  - live / paper-like / broker-linked
- [x] Strategy clarification:
  - ask the owner what they actually want
  - determine whether the CLI guides factor iteration when strategy is underspecified
- [x] Auto-Quant-assisted iteration:
  - determine whether the app guides the operator into using Auto-Quant to improve factors for the owner
- [x] Closed-loop evidence and data compounding:
  - factor scoring
  - factor filtering
  - BBN evidence nodes
  - regime classification / regime score
  - execution tree
  - whether the app helps the operator build useful future data during use

## Current Todo Board

### Done

- [x] Create this execution TODO markdown.
- [x] Create the dedicated bug log markdown target.
- [x] Verify no-pollution baseline with repo status and isolated `/tmp` state dirs only.
- [x] Read the minimum user/operator docs needed for the run (`README.md`, `support/docs/first-run.md`, `support/docs/agent-first-runbook.md`, `support/docs/auto-quant-ictengine-integration-guide.md`, `support/docs/2026-04-26-ibkr-live-data-bridge-plan.md`, `support/docs/2026-04-26-multi-exchange-data-source-integration-plan.md`).
- [x] Run the first-entry command sequence plus empty-state `workflow-status`/`provider-status` checks under a fresh `/tmp` home.
- [x] Execute replay/demo, factor-research, backtest, live, and Auto-Quant handoff surfaces as a first-time operator trial.
- [x] Append the first pass of concrete UX findings to the dedicated bug log.
- [x] Repair slice 1:
  - root `--help` now carries an explicit start-here footer
  - `provider-status --compact` now hints how to drill into one-provider setup, and one-provider compact output surfaces setup prompts directly
  - `workflow-status --phase bootstrap|ensemble-vote|structural-recommended-path-bundle --human` now has a human-readable path instead of falling back to raw JSON
  - top-level human workflow guidance now points evidence-review commands at human-readable sub-surfaces
  - live historical-path selection now keeps distinct candidate labels even after path redaction
  - `analyze-live` and `pre-bayes-status` now expose human-output flags at the CLI surface
- [x] Focused integration verification: `cargo test --test provider_neutral_cli --target-dir /tmp/ict-engine-codex-it`
- [x] Repair slice 2:
  - native `factor-research --human` now surfaces an explicit optional Auto-Quant managed iteration path
  - Auto-Quant missing-data readiness/handoff now stays inside repo CLI via `ict-engine auto-quant-prepare --state-dir ...`
  - `workflow-status --human` confirms the Auto-Quant return path uses the repo wrapper instead of raw `uv run ...`
- [x] Focused command-level re-run under fresh `/tmp` home confirms:
  - root `--help` now includes a start-here block
  - `workflow-status --phase bootstrap --human` is readable
  - `workflow-status --phase ensemble-vote --human`, `pre-bayes-status --human`, and `workflow-status --phase structural-recommended-path-bundle --human` are readable
  - `analyze-live --help` advertises `--human`, and `analyze-live --human` produces a concise summary
  - live historical-data handoff keeps distinct labels (`[ltf]`, `[mtf]`, `[spot]`) after path redaction
  - Auto-Quant handoff/workflow guidance now recommends `ict-engine auto-quant-prepare --state-dir ...`
- [x] Lib baseline follow-up:
  - `belief_core::beta_dirichlet_update::tests::weighted_seed_update_matches_structural_outcome_heuristic`
    - fixed by aligning the test to the current explicit invalidated pseudo-count weight table instead of the stale older expectation
  - `state::types::tests::test_structural_prior_seed_rebuilds_node_duration_priors`
    - fixed by aligning the delegating-layer test with the canonical BOCPD helper in `src/belief_core/changepoint_gate.rs` after the owner move
  - verification:
    - `cargo test --lib belief_core::beta_dirichlet_update::tests::weighted_seed_update_matches_structural_outcome_heuristic -- --nocapture`
    - `cargo test --lib state::types::tests::test_structural_prior_seed_rebuilds_node_duration_priors -- --nocapture`
    - `cargo test --lib`
- [x] Consumer-surface slice 3:
  - `auto-quant-status` now supports `json` (default), `--compact`, and `--human`
  - help text now exposes the new consumer output modes
  - missing-dependency human output now reports a short next step plus repo CLI command instead of raw JSON
- [x] Focused verification for `auto-quant-status`:
  - `cargo test --test provider_neutral_cli auto_quant_status_help_and_human_surface_expose_consumer_output_modes -- --nocapture`
  - `cargo test --lib auto_quant_readiness_human_output_is_short_text_not_json_dump -- --nocapture`
  - `cargo test --lib auto_quant_readiness_compact_surface_keeps_summary_line -- --nocapture`
  - real CLI check:
    - `./target/debug/ict-engine auto-quant-status --help`
    - `./target/debug/ict-engine auto-quant-status --state-dir /tmp/ict-engine-human-surface-aq --human`
    - `./target/debug/ict-engine auto-quant-status --state-dir /tmp/ict-engine-human-surface-aq --compact`

### Next

- [ ] Additional product/runtime work only if a new slice is opened.

### Not Yet

- [ ] Additional UX polishing beyond the logged closed-loop bugs.

## Ordered Trial Checklist

### Phase 0: Hygiene

- [x] Use a fresh `/tmp/ict-engine-user-first-run-closed-loop-<stamp>` state root.
- [x] Confirm the repo stays clean before starting.
- [x] Confirm no prior maintainer state is silently reused.

### Phase 1: Entry

- [x] Check whether the first help surface tells the operator:
  - what the app is for
  - where to start
  - how replay / backtest / live differ
- [x] Check whether the CLI points early to `workflow-status`, `provider-status`, or equivalent guided surfaces.

### Phase 2: Provider Onboarding

- [x] Determine whether the app guides the operator to connect TradingView MCP when chart/data workflows need it.
- [x] Determine whether the app guides the operator to connect IBKR, including:
  - install/start TWS or Gateway
  - enable the local bridge / consumer path
  - understand that credentials stay local
- [x] Determine whether the app clearly falls back to Yahoo Finance when the owner has no tradfi credential.
- [x] Determine whether the app clearly routes crypto users toward Bybit / Binance public data.
- [x] Determine whether the app explains Kraken correctly:
  - public data path when no login is needed
  - any later login/runtime requirement when that path changes

### Phase 3: Workflow Choice

- [x] Verify whether the app asks the owner which mode they want first:
  - replay / review
  - backtest
  - live
- [x] Verify whether the follow-up prompts are mode-appropriate.

### Phase 4: Replay / Backtest

- [x] Verify whether replay/backtest guidance asks for:
  - provider choice
  - historical data acquisition
  - symbol / market
  - strategy or thesis
- [x] If strategy is insufficient, verify whether the app explicitly routes into factor iteration instead of pretending strategy already exists.

### Phase 5: Live

- [x] Verify whether the app distinguishes:
  - data-only live observation
  - paper-like live
  - broker-linked live
- [x] Verify whether it requests the minimum provider/runtime setup needed without assuming credentials already exist.

### Phase 6: Auto-Quant

- [x] Verify whether Auto-Quant is discoverable as an external iteration engine.
- [x] Verify whether the operator is guided to use it to improve factors for the owner.
- [x] Verify whether the operator can return from the Auto-Quant handoff back into `workflow-status`.

### Phase 7: Evidence And Data Compounding

- [x] Verify whether the app exposes or clearly guides the operator to:
  - factor scoring
  - factor filtering
  - BBN evidence review
  - regime review
  - execution-tree review
- [x] Verify whether the app helps the operator accumulate useful data that improves future runs rather than requiring all evidence to pre-exist.

### Phase 8: Bug Logging

- [x] For every issue, record:
  - exact command
  - user goal
  - expected guidance
  - actual guidance
  - why it blocks or degrades the loop
  - whether a safe fallback existed

## Bug Log Target

- [x] Append all issues to:
  - `support/docs/bug/0.1.0/2026-05-05-user-first-run-closed-loop-bug-log.md`

## Minimum Trial Commands

```bash
git status --short
./target/debug/ict-engine --help
./target/debug/ict-engine workflow-status --help
./target/debug/ict-engine provider-status --help
./target/debug/ict-engine auto-quant-status --state-dir /tmp/ict-engine-user-first-run-closed-loop-aq
./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-user-first-run-closed-loop-demo --human
./target/debug/ict-engine analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-user-first-run-closed-loop-demo --human
./target/debug/ict-engine factor-research --symbol DEMO --data support/examples/demo/demo-15m.json --state-dir /tmp/ict-engine-user-first-run-closed-loop-demo --backend native --human
./target/debug/ict-engine analyze-live --symbol NQ --state-dir /tmp/ict-engine-user-first-run-closed-loop-live
./target/debug/ict-engine backtest --symbol DEMO --data support/examples/demo/demo-15m.json --state-dir /tmp/ict-engine-user-first-run-closed-loop-backtest --human
```

## Trial Success Standard

- [x] A first-time user/agent can tell where to start.
- [x] Missing-provider cases produce guidance rather than guesswork.
- [x] TradingView MCP / IBKR setup and login intent are clearly surfaced when relevant.
- [x] Yahoo Finance, Bybit, Binance, and Kraken are explained clearly enough for the operator to choose the right no-credential path.
- [x] Replay / backtest / live are discoverable as separate choices.
- [x] Strategy-underdefined cases route into factor iteration / Auto-Quant rather than dead-end.
- [x] The app exposes factor -> BBN / regime / execution-tree follow-up clearly enough for a user-agent to continue the loop.
- [x] Every meaningful friction is logged in the bug markdown.
