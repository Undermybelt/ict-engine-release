# One-Shot Structural Debt Closure Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the remaining high-value structural debt in `ict-engine` with one focused cleanup pass, centered on the still-heavy execution chains in `src/main.rs` and stale debt narrative in docs.

**Architecture:** Keep behavior and serialized contracts stable while moving long execution chains out of `src/main.rs` into already-established homes under `src/application/backtest/` and bin-side shell modules. Treat this as a refactor-and-verification pass, not feature work: no output contract drift, no heuristic rewrites, no business-logic changes unless needed to preserve extracted ownership boundaries.

**Tech Stack:** Rust, Cargo, existing `src/application/*` module layout, bin-side shell modules in `src/*.rs`, markdown docs under `docs/`.

---

## Scope Summary

This plan replaces the now-stale claim in `docs/main-rs-extraction-closeout-2026-04-23.md` that the extraction debt is "substantially closed."

Current repo facts on `2026-04-24`:

- `src/main.rs` is still `16405` lines.
- The largest remaining execution hotspots are:
  - `update_command` at `src/main.rs:4187-4777` (`591` lines)
  - `run_factor_research` at `src/main.rs:4804-5476` (`673` lines)
  - `run_factor_backtest` at `src/main.rs:5904-6376` (`473` lines)
  - `finalize_backtest_report` at `src/main.rs:8239-9012` (`774` lines)
- `analyze`, `analyze-live`, multiple command wrappers, output helpers, ensemble persistence helpers, and analyze persistence helpers have already been moved out successfully.

## Debt Inventory

### A. One-shot structural debt to clear in this branch

These are the debts this plan is meant to close:

1. `src/main.rs` still owns four long execution chains that are no longer just "entrypoint glue."
2. `update_command` mixes:
   - pending artifact ingestion
   - BBN update application
   - artifact preview/consumption logic
   - agent prompt assembly
   - workflow/recommendation/context assembly
   - output emission handoff
3. `run_factor_research` mixes:
   - mutation spec loading/application
   - research runtime execution
   - mutation evaluation
   - artifact/prompt/workflow post-processing
   - persistence and calibration writes
4. `run_factor_backtest` mixes:
   - data loading and MTF resolution
   - research-derived runtime report assembly
   - learning/BBN updates
   - artifact/workflow/posterior post-processing
   - persistence and calibration writes
5. `finalize_backtest_report` still centralizes too much probabilistic backtest post-processing despite a large `src/application/backtest/` surface already existing.
6. The debt narrative in docs is internally inconsistent:
   - `docs/main-rs-extraction-closeout-2026-04-23.md` says the extraction line is substantially closed
   - active code reality says the dominant structural hotspots are still concentrated in `src/main.rs`

### B. Real debt, but not part of this one-shot cleanup

These should be tracked honestly, but not mixed into this refactor pass:

1. Archived backend portability debt documented in `docs/backend-path-audit.md`
2. Release transport/history debt from oversized tracked historical state files
3. Compact-but-conservative analytics depth debt in:
   - `research-verdict`
   - contamination heuristics
   - `evidence-quality-breakdown`

These are not `src/main.rs` extraction tasks and should not block this pass.

## End-State Targets

By the end of this plan:

- `src/main.rs` should be reduced to CLI parsing, dispatch, thin adapters, and a smaller set of low-level shared helpers.
- No remaining command/runtime function in `src/main.rs` should exceed roughly `250-300` lines.
- The four hotspots above should move to focused modules with ownership aligned to current repo structure.
- Current output contracts for:
  - `analyze`
  - `analyze-live`
  - `factor-research`
  - `factor-backtest`
  - `backtest`
  - `update`
  - `workflow-status`
  must remain unchanged unless a matching release-note/doc update is included.
- Docs must stop claiming the debt is already closed.

## File Ownership Map

### Existing modules to extend

- Modify: `src/update_output.rs`
  - Keep output-only and update-adjacent helper logic
- Modify: `src/application/backtest/mod.rs`
  - Re-export any newly extracted runtime/finalization modules
- Modify: `src/application/backtest/finalize_*.rs`
  - Reuse existing finalize surface instead of rebuilding post-processing in `main.rs`
- Modify: `src/application/factor_lifecycle/mod.rs`
  - Re-export research/mutation runtime helpers if they land there
- Modify: `src/main.rs`
  - Remove moved runtime code
- Modify: `docs/main-rs-extraction-closeout-2026-04-23.md`
  - Mark as superseded by current plan

### New modules expected from this plan

- Create: `src/update_command.rs`
  - Own the `update_command` runtime path, parallel to `src/analyze_command.rs`
- Create: `src/application/backtest/research_runtime.rs`
  - Own `run_factor_research` runtime and mutation-oriented post-processing
- Create: `src/application/backtest/factor_backtest_runtime.rs`
  - Own `run_factor_backtest`
- Create: `src/application/backtest/probabilistic_runtime.rs`
  - Own `run_probabilistic_backtest` and/or the remaining backtest runtime shell around finalization
- Create if needed: `src/application/backtest/finalize_runtime.rs`
  - Only if `finalize_backtest_report` cannot be cleanly absorbed by existing `finalize_*` modules

## Task 1: Correct the Debt Narrative

**Files:**
- Create: `docs/plans/2026-04-24-one-shot-structural-debt-closure-plan.md`
- Modify: `docs/main-rs-extraction-closeout-2026-04-23.md`
- Modify: `docs/plans/main-rs-extraction-plan.md`

- [ ] **Step 1: Add the current debt inventory and one-shot closure plan**

Write the new plan doc with:
- exact remaining hotspots
- exact line spans
- in-scope vs out-of-scope debt split
- concrete extraction order
- required verification commands

- [ ] **Step 2: Mark the old closeout doc as superseded**

Add a short status note at the top of `docs/main-rs-extraction-closeout-2026-04-23.md` pointing to this plan and stating that the prior "substantially closed" conclusion is stale as of `2026-04-24`.

- [ ] **Step 3: Update the old extraction plan header**

Add a note to `docs/plans/main-rs-extraction-plan.md` that the plan is now partially superseded by this newer one-shot closure plan and should no longer be treated as current state reporting.

- [ ] **Step 4: Commit**

```bash
git add docs/plans/2026-04-24-one-shot-structural-debt-closure-plan.md \
        docs/main-rs-extraction-closeout-2026-04-23.md \
        docs/plans/main-rs-extraction-plan.md
git commit -m "docs: inventory remaining structural debt"
```

## Task 2: Move `update_command` Out of `main.rs`

**Files:**
- Create: `src/update_command.rs`
- Modify: `src/update_output.rs`
- Modify: `src/main.rs`
- Test: existing `src/main.rs` update-related tests

- [ ] **Step 1: Move the command runtime into `src/update_command.rs`**

Target ownership:
- feedback/artifact ingestion
- BBN update application
- artifact preview and consumption handling
- recommendation/context assembly
- final handoff to `emit_update_output`

`main.rs` should keep only:
- clap argument parsing
- conversion into `UpdateCommandInput`
- direct call into `update_command(...)`

- [ ] **Step 2: Leave output-only helpers in `src/update_output.rs`**

Do not re-mix output rendering with update runtime logic. If new shared helpers are discovered, place them next to the owning runtime or output surface instead of keeping them in `main.rs`.

- [ ] **Step 3: Run targeted verification**

Run:

```bash
cargo fmt --manifest-path Cargo.toml
cargo check --manifest-path Cargo.toml
cargo test --manifest-path Cargo.toml test_recommended_next_command_meta_classifies_ict_engine_command
```

- [ ] **Step 4: Commit**

```bash
git add src/update_command.rs src/update_output.rs src/main.rs
git commit -m "Move update command into bin module"
```

## Task 3: Move `run_factor_research` Into `src/application/backtest/`

**Files:**
- Create: `src/application/backtest/research_runtime.rs`
- Modify: `src/application/backtest/mod.rs`
- Modify: `src/main.rs`
- Test: existing factor research runtime tests

- [ ] **Step 1: Move runtime execution, not just wrappers**

`research_runtime.rs` should own:
- data loading
- mutation spec application
- objective handling
- mutation metric/evaluation path
- workflow/artifact/prompt post-processing
- persistence and calibration writes

- [ ] **Step 2: Keep command-entry parsing where it already lives**

`src/application/backtest/command_entry.rs` already owns the command wrapper surface. Reuse that split instead of inventing another wrapper layer.

- [ ] **Step 3: Re-export from `src/application/backtest/mod.rs`**

Expose a focused runtime entry such as:
- `run_factor_research`
- or `run_factor_research_runtime`

- [ ] **Step 4: Run targeted verification**

Run:

```bash
cargo fmt --manifest-path Cargo.toml
cargo check --manifest-path Cargo.toml
cargo test --manifest-path Cargo.toml test_factor_research_output_payload_includes_human_compare_summary
```

- [ ] **Step 5: Commit**

```bash
git add src/application/backtest/research_runtime.rs \
        src/application/backtest/mod.rs \
        src/main.rs
git commit -m "Extract factor research runtime"
```

## Task 4: Move `run_factor_backtest` Into `src/application/backtest/`

**Files:**
- Create: `src/application/backtest/factor_backtest_runtime.rs`
- Modify: `src/application/backtest/mod.rs`
- Modify: `src/main.rs`
- Test: existing factor backtest tests

- [ ] **Step 1: Move the runtime body**

`factor_backtest_runtime.rs` should own:
- candle loading
- MTF summary/signal assembly
- research-derived report construction
- feedback enrichment
- BBN/learning-state update path
- report persistence/calibration writes

- [ ] **Step 2: Keep `main.rs` as thin dispatch**

After extraction, `main.rs` should not know the detailed sequencing of score deltas, artifact trends, agent prompts, or backtest persistence.

- [ ] **Step 3: Run targeted verification**

Run:

```bash
cargo fmt --manifest-path Cargo.toml
cargo check --manifest-path Cargo.toml
cargo test --manifest-path Cargo.toml test_render_backtest_human_output_includes_compare_block
```

- [ ] **Step 4: Commit**

```bash
git add src/application/backtest/factor_backtest_runtime.rs \
        src/application/backtest/mod.rs \
        src/main.rs
git commit -m "Extract factor backtest runtime"
```

## Task 5: Collapse Probabilistic Backtest Finalization Into Existing Backtest Modules

**Files:**
- Create: `src/application/backtest/probabilistic_runtime.rs`
- Modify: `src/application/backtest/finalize_*.rs`
- Modify: `src/application/backtest/mod.rs`
- Modify: `src/main.rs`
- Test: existing probabilistic/backtest finalize tests

- [ ] **Step 1: Move `run_probabilistic_backtest` runtime ownership**

Place runtime sequencing in `probabilistic_runtime.rs`.

- [ ] **Step 2: Shrink `finalize_backtest_report` aggressively**

Absorb logic into current finalize modules wherever possible:
- `finalize_context.rs`
- `finalize_decisions.rs`
- `finalize_enrichment.rs`
- `finalize_surfaces.rs`
- `finalized_run.rs`

Only create a new `finalize_runtime.rs` if the existing finalize modules cannot own the remaining logic without becoming awkward.

- [ ] **Step 3: Run targeted verification**

Run:

```bash
cargo fmt --manifest-path Cargo.toml
cargo check --manifest-path Cargo.toml
cargo test --manifest-path Cargo.toml test_backtest_command_reads_compare_surface
```

- [ ] **Step 4: Commit**

```bash
git add src/application/backtest/probabilistic_runtime.rs \
        src/application/backtest/*.rs \
        src/main.rs
git commit -m "Extract probabilistic backtest runtime"
```

## Task 6: Final Verification and Closeout

**Files:**
- Modify: `docs/main-rs-extraction-closeout-2026-04-23.md`
- Optionally create: `docs/main-rs-extraction-closeout-2026-04-24.md`

- [ ] **Step 1: Re-count `src/main.rs`**

Run:

```bash
wc -l src/main.rs
```

Record the new line count in the closeout note.

- [ ] **Step 2: Run full verification**

Run:

```bash
cargo fmt --check --manifest-path Cargo.toml
cargo check --all-targets --manifest-path Cargo.toml
cargo test --manifest-path Cargo.toml
cargo clippy --all-targets --manifest-path Cargo.toml -- -D warnings
```

- [ ] **Step 3: Confirm debt closure criteria**

Confirm:
- `main.rs` no longer contains the four long runtime hotspots above
- the old stale closeout claim is corrected
- tree is clean

- [ ] **Step 4: Commit closeout**

```bash
git add docs/main-rs-extraction-closeout-2026-04-23.md docs/plans/2026-04-24-one-shot-structural-debt-closure-plan.md src
git commit -m "Close remaining structural debt hotspots"
```

## Verification Gates

These gates apply after every extraction task:

1. `cargo fmt --manifest-path Cargo.toml`
2. `cargo check --manifest-path Cargo.toml`
3. At least one targeted test for the moved surface
4. `git status --short --branch` must be clean before starting the next task

## Abort Conditions

Stop and split the plan if any extraction:

- causes a dependency explosion similar to the reverted `run_factor_research` attempt
- requires business-logic rewrites instead of ownership moves
- changes serialized output or next-command contracts without explicit docs and tests

If that happens, downgrade from "one-shot closure" to "two-pass closure" and record the break point in this file rather than improvising in chat.
