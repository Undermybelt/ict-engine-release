# NQ Iteration Blocker Remediation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the first-round `NQ` factor-iteration blockers without polluting the managed Auto-Quant checkout or adding new repo-owned market assumptions.

**Architecture:** Keep `ict-engine` as the control plane. Fix the native `factor-autoresearch` first-run/runtime visibility path so `structure_ict` can iterate immediately, then repair the Auto-Quant readiness/handoff surface so it points at the additive `NQ/USD` execution toolkit that already lives in this repo instead of the crypto-only default path.

**Tech Stack:** Rust CLI, existing Rust state persistence, additive Python helper scripts under `support/scripts/auto_quant_external/`

---

### Task 1: Lock native first-run autoresearch behaviour

**Files:**
- Modify: `src/application/factor_lifecycle/command_entry.rs`
- Modify: `src/state/types.rs`
- Test: `src/application/factor_lifecycle/command_entry.rs`

- [ ] **Step 1: Keep failing/targeted tests in place for first-run seed bootstrapping and heartbeat**

Use these tests as the contract:

```rust
#[test]
fn default_first_run_autoresearch_seed_spec_uses_structure_ict_for_expansion_objective() {}

#[test]
fn heartbeat_updates_live_snapshot_stage_and_timestamp() {}

#[test]
fn factor_autoresearch_command_bootstraps_seed_spec_when_missing() {}
```

- [ ] **Step 2: Verify those tests still pass after the final code shape**

Run: `cargo test factor_autoresearch_command_bootstraps_seed_spec_when_missing heartbeat_updates_live_snapshot_stage_and_timestamp default_first_run_autoresearch_seed_spec_uses_structure_ict_for_expansion_objective -- --nocapture`

Expected: all listed tests pass.

### Task 2: Correct Auto-Quant readiness and handoff commands

**Files:**
- Modify: `src/application/auto_quant/readiness.rs`
- Modify: `src/application/auto_quant/handoff.rs`
- Modify: `src/application/auto_quant/mod.rs`
- Test: `src/application/auto_quant/mod.rs`
- Test: `src/application/auto_quant/handoff.rs`

- [ ] **Step 1: Add/adjust tests for the new command contract**

Target behaviours:

```rust
#[test]
fn readiness_reports_data_missing_with_talib_prepare_command() {}

#[test]
fn readiness_reports_run_ready_with_talib_run_command() {}

#[test]
fn handoff_suggested_commands_use_talib_and_actionable_runtime_paths() {}
```

- [ ] **Step 2: Implement the minimal command-surface fix**

Required outcomes:
- `recommended_next_command` uses `uv run --with ta-lib ...`
- handoff `suggested_commands` stop pointing at the broken `uv run prepare.py` / `uv run run.py` defaults
- notes and next-step text stay provider-neutral at the repo level

- [ ] **Step 3: Re-run targeted Auto-Quant tests**

Run: `cargo test auto_quant -- --nocapture`

Expected: readiness and handoff tests pass.

### Task 3: Update bug ledger and prove the repaired runtime path

**Files:**
- Modify: `support/docs/bug/0.1.0/2026-04-28-nq-iteration-round-1.md`

- [ ] **Step 1: Append remediation notes instead of deleting original findings**

Document:
- which findings are fixed in this change
- which ones remain open by design
- what command now proves the fixed path

- [ ] **Step 2: Run a real `NQ` runtime smoke check**

Run:

```bash
cargo run --quiet -- factor-autoresearch \
  --symbol NQ \
  --data /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-15m/nq.continuous-15m.json \
  --state-dir /tmp/ict-engine-nq-native-smoke-20260428 \
  --backend native \
  --iterations 1
```

And separately:

```bash
./target/debug/ict-engine factor-autoresearch-status \
  --symbol NQ \
  --state-dir /tmp/ict-engine-nq-native-smoke-20260428 \
  --latest-only
```

Expected: first run does not require `--mutation-spec`, live status shows an advancing `updated_at`, and `current_stage` is no longer stuck at startup.

### Task 4: Final verification and commit

**Files:**
- No code changes expected

- [ ] **Step 1: Run full verification**

Run:
- `cargo test --all -- --nocapture`
- `cargo clippy --all-targets --all-features -- -D warnings`

Expected: both commands exit 0.

- [ ] **Step 2: Commit only after fresh verification**

```bash
git add support/docs/bug/0.1.0/2026-04-28-nq-iteration-round-1.md \
        support/docs/plans/2026-04-28-nq-iteration-blocker-remediation-plan.md \
        src/application/auto_quant/handoff.rs \
        src/application/auto_quant/mod.rs \
        src/application/auto_quant/readiness.rs \
        src/application/factor_lifecycle/command_entry.rs \
        src/state/types.rs
git commit -m "fix: unblock first-round nq factor iteration"
```
