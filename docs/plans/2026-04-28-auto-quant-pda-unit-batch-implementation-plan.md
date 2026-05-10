# Auto-Quant PDA Unit Batch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an Auto-Quant-centered batch command that turns explicit PDA primitive sequences into isolated `setup × symbol × timeframe × direction` units and persists parallel-ready AQ handoff artifacts.

Boundary note:
- This plan remains valid for internal research tooling.
- It is no longer the target consumer-facing architecture; `agent-material-*` is the public protocol direction.

**Architecture:** Keep the existing shared managed Auto-Quant checkout, but generate isolated per-unit state dirs and a batch manifest above the existing handoff system. The CLI will produce natural-language unit briefs and ordered primitive sequences rather than treating repo `structure_ict` aggregation as the iteration truth.

**Tech Stack:** Rust CLI, existing Auto-Quant handoff persistence, `serde`, `clap`, isolated `/tmp`-style state dirs

---

### Task 1: Define PDA unit and batch types

**Files:**
- Create: `src/application/auto_quant/pda_unit_batch.rs`
- Modify: `src/application/auto_quant/mod.rs`
- Test: `src/application/auto_quant/pda_unit_batch.rs`

- [ ] **Step 1: Write failing tests for primitive parsing and ordered unit generation**

Target tests:

```rust
#[test]
fn parse_pda_primitive_kind_accepts_user_facing_names() {}

#[test]
fn build_unit_sequences_uses_ordered_permutations_for_combination_size_two() {}

#[test]
fn build_unit_jobs_expands_direction_timeframe_cross_product() {}
```

- [ ] **Step 2: Run the tests to verify they fail for missing types/functions**

Run: `cargo test parse_pda_primitive_kind_accepts_user_facing_names build_unit_sequences_uses_ordered_permutations_for_combination_size_two build_unit_jobs_expands_direction_timeframe_cross_product -- --nocapture`

Expected: FAIL because the new batch module does not exist yet.

- [ ] **Step 3: Implement the minimal types and builders**

Add:
- `AutoQuantPdaPrimitiveKind`
- `AutoQuantUnitDirection`
- `AutoQuantPdaUnitScope`
- `AutoQuantPdaUnitBrief`
- `AutoQuantPdaUnitJob`
- `AutoQuantPdaUnitBatchArtifact`

- [ ] **Step 4: Re-run the tests and verify they pass**

Run the same command from Step 2.

Expected: PASS.

### Task 2: Add a new Auto-Quant batch command surface

**Files:**
- Modify: `src/main.rs`
- Modify: `src/application/command_inputs.rs`
- Modify: `src/application/auto_quant/command_entry.rs`
- Test: `src/application/command_inputs.rs`
- Test: `src/application/auto_quant/command_entry.rs`

- [ ] **Step 1: Write failing tests for command input and manifest emission**

Target tests:

```rust
#[test]
fn auto_quant_pda_unit_batch_command_input_carries_explicit_batch_fields() {}

#[test]
fn auto_quant_pda_unit_batch_command_persists_batch_manifest_and_unit_handoffs() {}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test auto_quant_pda_unit_batch -- --nocapture`

Expected: FAIL because the command surface does not exist yet.

- [ ] **Step 3: Implement the CLI command**

Add a new command:
- `auto-quant-pda-unit-batch`

Required inputs:
- `--symbol`
- `--objective`
- `--factors`
- `--combination-size`
- `--directions`
- `--timeframes`
- repeated `--timeframe-data <tf=path>`
- `--max-parallel`
- `--state-dir`

- [ ] **Step 4: Persist one batch artifact plus one unit handoff per unit**

Reuse:
- shared AQ bootstrap/readiness
- existing handoff persistence

But add:
- one isolated unit state dir per unit
- a batch manifest that records all unit jobs and dispatch groups

- [ ] **Step 5: Re-run the tests and verify they pass**

Run: `cargo test auto_quant_pda_unit_batch -- --nocapture`

Expected: PASS.

### Task 3: Attach unit-level natural-language strategy briefs to AQ handoffs

**Files:**
- Modify: `src/application/auto_quant/handoff.rs`
- Modify: `src/application/auto_quant/persistence.rs`
- Test: `src/application/auto_quant/handoff.rs`
- Test: `src/application/auto_quant/persistence.rs`

- [ ] **Step 1: Write failing tests for unit brief embedding**

Target tests:

```rust
#[test]
fn handoff_payload_can_carry_iteration_unit_context() {}

#[test]
fn unit_handoff_prompt_mentions_win_rate_sharpe_return_priority() {}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test handoff_payload_can_carry_iteration_unit_context unit_handoff_prompt_mentions_win_rate_sharpe_return_priority -- --nocapture`

Expected: FAIL because handoff payloads do not yet carry the new unit context.

- [ ] **Step 3: Extend the handoff payload shape minimally**

Add optional unit-context fields so per-unit handoffs can include:
- ordered primitive sequence
- symbol
- timeframe
- direction
- natural-language strategy brief
- evaluation priority

- [ ] **Step 4: Re-run the tests and verify they pass**

Run the same command from Step 2.

Expected: PASS.

### Task 4: Documentation, verification, and commit

**Files:**
- Modify: `docs/bug/0.1.0/2026-04-28-nq-iteration-round-1.md` only if new user-facing bugs are discovered during verification

- [ ] **Step 1: Run the new command in a real NQ single-timeframe batch**

Run:

```bash
./target/debug/ict-engine auto-quant-pda-unit-batch \
  --symbol NQ \
  --objective expansion_manipulation \
  --factors order_block,fair_value_gap,market_structure_shift,cisd \
  --combination-size 1 \
  --directions long,short \
  --timeframes 15m \
  --timeframe-data 15m=/tmp/ict-engine-nq-2023-trimmed-20260428/nq.continuous-15m.2023plus.json \
  --max-parallel 4 \
  --state-dir /tmp/ict-engine-aq-pda-unit-batch-smoke-20260428
```

Expected:
- batch manifest persisted
- one handoff per unit persisted
- dispatch groups emitted
- each unit is isolated to one setup sequence, one symbol, one timeframe, one direction

- [ ] **Step 2: Run full verification**

Run:
- `cargo test --all -- --nocapture`
- `cargo clippy --all-targets --all-features -- -D warnings`

Expected: both commands exit 0.

- [ ] **Step 3: Commit**

```bash
git add docs/plans/2026-04-28-auto-quant-pda-unit-batch-design.md \
        docs/plans/2026-04-28-auto-quant-pda-unit-batch-implementation-plan.md \
        src/application/auto_quant/pda_unit_batch.rs \
        src/application/auto_quant/command_entry.rs \
        src/application/auto_quant/handoff.rs \
        src/application/auto_quant/mod.rs \
        src/application/auto_quant/persistence.rs \
        src/application/command_inputs.rs \
        src/main.rs
git commit -m "feat: add auto-quant pda unit batch dispatch"
```
