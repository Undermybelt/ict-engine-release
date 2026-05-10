# User-First Closed-Loop Trial TODO

> **For Hermes/Codex:** this is an execution TODO for a real user-style trial. Do not change code in this round. Work from the user/agent/operator surface only. Log every bug, blocker, redundancy, misleading prompt, or missing guidance into `docs/bug/0.1.0/2026-05-05-user-first-run-provider-to-auto-quant-bug-log.md`.

**Goal:** evaluate whether a first-time agent, on a freshly cloned local checkout, is guided end-to-end from the first CLI entry surface to provider onboarding, data acquisition, replay/backtest/live routing, strategy clarification, factor iteration, Auto-Quant assistance, and evidence flow into BBN / regime / execution-tree surfaces.

**Architecture:** treat this repo as a user-facing research CLI, not as a codebase to improve during the run. Use explicit `/tmp/...` state dirs to avoid polluting the checkout. Prefer the real command surfaces (`--help`, `workflow-status`, `human-next`, `provider-status`, `auto-quant-status`, `factor-research`, `factor-backtest`, `analyze`) over internal assumptions.

**Tech Stack:** Rust CLI, repo docs under `docs/`, local optional providers (TradingView MCP, IBKR), public-market providers (Yahoo Finance, Bybit, Binance, Kraken), external managed integration (`Auto-Quant`).

---

## Execution Rules

- [x] Do not modify code, tests, config defaults, or repo structure during this trial.
- [x] Prefer isolated `/tmp/...` `--state-dir` values for every real run.
- [x] Assume the user is new to the app and has not provided any provider credential unless explicitly stated.
- [x] Evaluate what the app *guides* the user/agent to do, not what the repo *could* theoretically do if we already knew the internals.
- [x] Every time guidance is missing, stale, misleading, redundant, blocked, or over-assumes maintainer knowledge, append it to the bug log markdown.

## Trial Scope

- [x] First entry and top-level help surface.
- [x] Provider onboarding guidance:
  - TradingView MCP
  - IBKR client / bridge
  - Yahoo Finance free fallback
  - crypto public data path via Bybit / Binance / Kraken
- [x] Decision routing guidance:
  - replay / review historical market behavior
  - backtest / evaluate strategy and factors
  - live / real-time or paper/live connected flow
- [x] Strategy clarification loop:
  - ask the owner what they actually want
  - if strategy is underspecified, identify whether the app guides factor iteration
- [x] Auto-Quant-assisted closed loop:
  - can the app guide the user/agent into using Auto-Quant as an external iterative engine
- [x] Evidence closure:
  - factor scoring / filtering
  - BBN evidence
  - regime labeling / scoring
  - execution tree

## Current Todo Board

### Done

- [x] Create the execution TODO markdown.
- [x] Create the bug log markdown.

### Next

- [x] Verify the no-pollution baseline:
  - use `cargo check`
  - use CLI help only
  - choose explicit `/tmp/ict-engine-user-first-run-...` state dirs
- [x] Read only the user/operator docs needed for the trial:
  - `README.md`
  - `docs/first-run.md`
  - `docs/agent-first-runbook.md`
  - `docs/auto-quant-ictengine-integration-guide.md`
  - `docs/2026-04-26-ibkr-live-data-bridge-plan.md`
  - `docs/2026-04-26-multi-exchange-data-source-integration-plan.md`
- [x] Run the first-entry command sequence from a clean-user perspective:
  - `./target/debug/ict-engine --help`
  - `./target/debug/ict-engine workflow-status --help`
  - `./target/debug/ict-engine provider-status --help`
  - if `human-next` is not a top-level subcommand, verify the equivalent guidance through `workflow-status --human` and any surfaced phase/status route instead
- [x] Check whether the app guides provider setup before any strategy flow:
  - does it explain TradingView MCP when market-data or chart-linked workflows are relevant
  - does it explain IBKR install / enable / bridge start when live or broker-linked data is needed
  - does it clearly say Yahoo Finance is the free fallback when no credential/provider is available
  - does it route crypto users toward Bybit / Binance public data and Kraken where appropriate
- [x] Check whether the app asks the user/owner which mode they want:
  - replay / review
  - backtest
  - live
- [x] For each mode, verify what the app asks next:
  - data source
  - symbol / market
  - provider
  - strategy
  - whether credentials or login are required
- [x] If the owner provides no strategy:
  - verify whether the app guides factor research / factor iteration / Auto-Quant loop instead of pretending strategy is already known
- [x] Verify whether the app explains the downstream closed loop in user terms:
  - factors -> factor scores / filtering
  - belief evidence / BBN nodes
  - regime classification / regime scoring
  - execution tree
- [x] Verify whether the app helps accumulate useful data during use rather than requiring all evidence to pre-exist.
- [x] Record every broken, redundant, blocked, misleading, or overly implicit step into the bug log.

### Not Yet

- [x] Run a clean first-time replay flow from user entry to historical data acquisition.
- [x] Run a clean first-time backtest flow from user entry to factor/strategy clarification.
- [x] Run a clean first-time live flow from user entry to provider/broker login guidance.
- [x] Evaluate whether Auto-Quant handoff is discoverable without maintainer knowledge.
- [x] Write the user-trial report summarizing what the app actually guided well vs poorly.

## Ordered Trial Checklist

### Phase 0: Hygiene And Baseline

- [x] Use a fresh `/tmp/ict-engine-user-first-run-<timestamp>` state dir.
- [x] Confirm the repo stays clean before starting the trial.
- [x] Confirm no prior state is being silently reused unless the CLI explicitly says so.

### Phase 1: Entry Experience

- [x] Check whether the first help surface tells the user:
  - what the app is for
  - where to start
  - what commands map to analysis / replay / backtest / live
- [x] Check whether the app points to `workflow-status`, `human-next`, or an equivalent guided surface early.

### Phase 2: Provider Guidance

- [x] Determine whether a first-time user is guided to connect TradingView MCP when relevant.
- [x] Determine whether a first-time user is guided to connect IBKR, including:
  - install/start TWS or Gateway
  - enable the local bridge/consumer path
  - understand that credentials stay local
- [x] Determine whether missing provider credentials trigger a useful fallback:
  - Yahoo Finance for traditional free historical data
  - Bybit / Binance public crypto data
  - Kraken public crypto/forex/tokenized-asset data where applicable
  - if a later path needs Kraken credentialed/runtime features, verify that the app says so explicitly instead of implying public data is blocked
- [x] Determine whether the app over-assumes the user wants traditional finance when the user may want crypto.

### Phase 3: Workflow Choice

- [x] Verify whether the app asks the owner what they want to do first:
  - replay
  - backtest
  - live
- [x] Verify whether the follow-up questions are mode-appropriate.

### Phase 4: Replay / Backtest Data Path

- [x] If replay/backtest is chosen, verify whether the app guides:
  - provider choice
  - historical data acquisition
  - symbol / instrument choice
  - timeframe choice
  - explicit use of already-integrated providers
- [x] Verify whether the app asks what strategy or thesis to evaluate.
- [x] If strategy is underspecified, verify whether the app routes into factor iteration instead of failing silently.

### Phase 5: Live Path

- [x] If live is chosen, verify whether the app distinguishes:
  - paper-like live observation
  - actual broker-linked live path
  - data-only live feed
- [x] Verify whether it requests the minimum provider/broker setup needed, without assuming credentials already exist.

### Phase 6: Closed-Loop Evidence

- [x] Verify whether the app exposes or clearly explains:
  - factor scoring
  - factor filtering
  - entry into BBN evidence nodes
  - regime classification and regime scoring
  - execution tree
- [x] Verify whether the app makes it clear that useful data must be accumulated over use and iteration.
- [x] Verify whether Auto-Quant is discoverable as the external iterative engine that helps produce stronger factors for the owner.

### Phase 7: Bug Logging

- [x] For every failure, record:
  - exact command
  - expected user guidance
  - actual guidance
  - why it blocks or degrades the closed loop
  - whether there was a safe fallback

## Bug Log Target

- [x] Write all issues into:
  - `docs/bug/0.1.0/2026-05-05-user-first-run-provider-to-auto-quant-bug-log.md`

## Trial Report Target

- [x] Write the final user-view trial report into:
  - `docs/bug/0.1.0/2026-05-05-user-first-run-provider-to-auto-quant-trial-report.md`

## Minimum Trial Commands

Run these as the first user-style probes:

```bash
cargo check
./target/debug/ict-engine --help
./target/debug/ict-engine workflow-status --help
./target/debug/ict-engine provider-status --help
./target/debug/ict-engine auto-quant-status --state-dir /tmp/ict-engine-user-first-run-aq
./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-user-first-run-demo --human
./target/debug/ict-engine analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-user-first-run-demo --human
./target/debug/ict-engine factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-user-first-run-demo --backend native --human
```

## Trial Success Standard

- [ ] A first-time user/agent can tell where to start.
- [ ] Missing-provider cases produce guidance, not guesswork.
- [ ] Crypto and traditional-finance users are both guided to sensible data sources.
- [ ] Replay/backtest/live paths are discoverable and distinct.
- [ ] Strategy-underdefined cases route into factor iteration / Auto-Quant rather than dead-end.
- [x] Every major friction or bug is logged in the bug markdown.
