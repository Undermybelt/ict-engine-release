# PB12 Runner Closure Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the PB(12) control-matrix runner for native `factor-research` without polluting the primary state surfaces.

**Architecture:** `factor_research_command` will execute the 12 PB(12) runs against isolated temporary state clones, collect run summaries, and persist only a dedicated PB12 sweep artifact plus ledger entry in the main state directory. Main `learning_state`, BBN state, workflow snapshot, and regular `research_runs.json` stay untouched by the sweep.

**Tech Stack:** Rust, serde/serde_json, existing state persistence helpers, existing artifact ledger, `tempfile`.

---

## File Map

- Modify: `src/application/backtest/control_matrix.rs`
- Modify: `src/application/backtest/command_entry.rs`
- Modify: `src/application/reporting/backtest_output.rs`
- Modify: `src/application/reporting/mod.rs`
- Modify: `src/main.rs`

## Task 1: Lock the non-pollution contract

**Files:**
- Modify: `src/application/backtest/command_entry.rs`

- [x] Add a failing unit test that runs `factor_research_command` with `control_matrix_pb12=true` via a stub runner and asserts:
  - the runner is invoked 12 times
  - each invocation uses a temporary state dir instead of the main one
  - the main state dir does not receive runner-owned files
  - a PB12 artifact history file plus ledger entry is created in the main state dir

- [x] Run the targeted test and confirm it fails for the expected reason: PB12 runner path not implemented.

## Task 2: Add PB12 sweep artifact/state surfaces

**Files:**
- Modify: `src/application/backtest/control_matrix.rs`

- [x] Define the PB12 sweep artifact types:
  - artifact header (`artifact_id`, `generated_at`, `symbol`, `sweep_id`, `research_objective`)
  - per-run summary (`run_number`, `run_label`, `baseline`, enabled/disabled toggles, best factor, aggregate return, feedback counts, comparability, next command)
  - top-runs and baseline summary projections

- [x] Add persistence helpers for:
  - appending PB12 artifacts to a dedicated history file
  - loading PB12 artifacts for tests/reporting
  - writing the corresponding `auto_quant_pb12_research_run` ledger entry with `source_run_id = sweep_id`

- [x] Add focused tests for artifact serialization/persistence and ledger emission.

## Task 3: Wire isolated PB12 execution into `factor_research_command`

**Files:**
- Modify: `src/application/backtest/command_entry.rs`
- Modify: `src/main.rs`

- [x] Extend the runner callback signature so `factor_research_command` can choose the per-run state directory.

- [x] Implement PB12 execution flow:
  - build `ControlMatrixPlan::pb12()`
  - clone the current symbol state into a fresh temp dir per run
  - execute one research run per PB12 row against the temp dir
  - convert each report into a PB12 run summary
  - rank top runs and persist the sweep artifact in the main state dir

- [x] Keep the non-control-matrix path byte-for-byte equivalent in behavior.

## Task 4: Add reporting for PB12 sweep output

**Files:**
- Modify: `src/application/reporting/backtest_output.rs`
- Modify: `src/application/reporting/mod.rs`

- [x] Add a PB12 output payload builder and human renderer that surfaces:
  - plan kind
  - run count
  - baseline run summary
  - top-N run summaries

- [x] Add unit tests for payload/human-output shape.

## Task 5: Verify with targeted and end-to-end coverage

**Files:**
- Modify: `src/main.rs`

- [x] Add one higher-level test that runs the real native PB12 path against sample candles and asserts:
  - the PB12 artifact history contains one sweep with 12 runs
  - the ledger contains `auto_quant_pb12_research_run`
  - main `research_runs.json` remains unmodified by the sweep

- [x] Run:
  - `cargo test test_factor_research_command_pb12_uses_isolated_state_and_persists_sweep_artifact -- --nocapture`
  - `cargo test test_control_matrix_artifact_persistence_writes_ledger_entry -- --nocapture`
  - `cargo test test_factor_research_command_pb12_executes_real_runner_without_polluting_primary_history -- --nocapture`
  - `cargo test test_factor_research_output_payload_includes_pb12_sweep_summary -- --nocapture`
  - `cargo fmt`

- [x] If targeted verification is green, run one broader regression slice covering touched modules only.

## Delivered Scope

- PB12 sweep executes in isolated temporary symbol-state clones and persists a dedicated `auto_quant_pb12_research_run` artifact/ledger entry.
- PB12 artifact now carries a minimal read-only discovery summary against the Auto-Quant strategy-library baseline when available.
- Follow-up operator-driven promotion CLI and P4 market-data ingestion are intentionally not part of this closure plan.
