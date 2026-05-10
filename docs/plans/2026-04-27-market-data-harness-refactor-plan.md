# Market Data Harness Refactor Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace PB12's provider-specific hardcoded runtime mapping with a generic market-data harness that lets agents plan and fetch main/related symbols across providers, then feed the current factor runtime without polluting primary state.

**Architecture:** Add a generic request/plan/fetch harness under `src/application/data_sources/` with a versioned preset config, provider capability registry, and read-only normalized envelopes. PB12 runtime becomes a consumer of the harness, not a provider-specific implementation. New CLI commands expose harness planning/fetching so agents can operate it directly.

**Tech Stack:** Rust, serde/serde_json, existing `fetch_external.py`, existing `scripts/ibkr_bridge`, direct HTTP for TradingView MCP, existing Yahoo fetch code, Clap.

---

## Task 1: Define harness contract

**Files:**
- Create: `src/application/data_sources/harness.rs`
- Create: `config/market_data_harness_presets.json`
- Modify: `src/application/data_sources/mod.rs`

- [ ] Add typed request/plan/fetch/result structs for generic market-data harness operations.
- [ ] Add versioned preset config for known market mappings and provider symbol specs.
- [ ] Add pure unit tests for request resolution and provider task planning.

## Task 2: Add direct agent-operable commands

**Files:**
- Modify: `src/main.rs`
- Modify: `src/application/data_sources/command_entry.rs`

- [ ] Add `market-data-harness --action plan` command behavior that prints the resolved provider plan.
- [ ] Add `market-data-harness --action fetch` command behavior that executes the plan and prints normalized envelopes.
- [ ] Keep both commands read-only and side-effect-free except optional snapshot output.

## Task 3: Route PB12 through the harness

**Files:**
- Modify: `src/application/data_sources/control_matrix_runtime.rs`
- Modify: `src/application/backtest/command_entry.rs`
- Modify: `src/factor_research_runtime.rs`

- [ ] Replace provider-specific PB12 mapping logic with harness request construction.
- [ ] Convert harness results into the current `paired_candles_override` / `auxiliary_override` runtime bundle.
- [ ] Preserve PB12 isolated-state semantics and no-pollution boundary.

## Task 4: Verify and document

**Files:**
- Modify: `docs/2026-04-27-pda-factor-universe-plan.md`

- [ ] Add targeted tests for harness plan/fetch and PB12 integration.
- [ ] Run `cargo test --all -- --nocapture`.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Update the PDA universe plan status text to describe the new harness architecture.
