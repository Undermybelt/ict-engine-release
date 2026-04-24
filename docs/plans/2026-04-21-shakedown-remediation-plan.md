# Shakedown Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the highest-priority issues from `docs/audit-2026-04-21-full-codebase-shakedown.md` so contributor onboarding, human CLI usage, and agent consumption all behave consistently and predictably.

**Architecture:** Keep the current product logic and decision flow intact. Limit this plan to surface-level fixes: output rendering consistency, empty-state contract cleanup, command/help symmetry, and documentation/test alignment. Prefer small helper extraction over broad refactors, and only touch runtime behavior where the current surface is clearly inconsistent or misleading.

**Tech Stack:** Rust 2021, clap 4, serde_json, existing `main.rs` CLI dispatch, `application::reporting`, `application::orchestration`, repo docs under `docs/`, `cargo test`, `cargo fmt`

---

## File Structure

- Modify: `src/main.rs`
- Modify: `src/application/reporting/analyze_output.rs`
- Modify: `src/application/reporting/agent_report.rs` only if a shared next-step formatter is needed
- Modify: `src/application/orchestration/workflow_status.rs`
- Modify: `src/application/release_closure/mod.rs`
- Modify: `README.md`
- Modify: `docs/audit-2026-04-21-cross-surface-review.md`
- Create: `tests/output_format_conflicts.rs` if `main.rs` tests become too crowded
- Create: `tests/empty_state_contracts.rs` if command-contract tests become too crowded

Responsibility split:

- `src/main.rs`: command conflict enforcement, CLI alias/help symmetry, command output wiring
- `src/application/reporting/*`: human vs agent next-step rendering consistency
- `src/application/orchestration/workflow_status.rs`: status/empty-state shaping and humanized guidance
- `src/application/release_closure/mod.rs`: verdict behavior when evidence is missing
- `README.md` and audit docs: public contract and contributor expectation updates

## Scope Notes

- Do not change market logic, ranking logic, regime logic, or learning logic.
- Do not rewrite core data models beyond adding surface metadata or empty-state shaping.
- Do not broaden into `main.rs` extraction; document that separately if needed.
- Backward compatibility on persisted state should be preserved.

---

### Task 1: Enforce Output-Format Conflict Rules Consistently

**Files:**
- Modify: `src/main.rs`
- Test: `src/main.rs` or `tests/output_format_conflicts.rs`

- [ ] **Step 1: Write failing regression tests for conflicting analyze output flags**

Add tests that prove `analyze` rejects mixed format inputs:

```rust
#[test]
fn test_analyze_rejects_human_and_explicit_json_mix() {
    let error = resolve_output_format("json", false, false, true).unwrap_err();
    assert!(error
        .to_string()
        .contains("do not combine --output-format with --compact/--agent/--human"));
}

#[test]
fn test_cli_analyze_rejects_json_alias_mix_contract() {
    let cli = Cli::try_parse_from([
        "ict-engine",
        "analyze",
        "--symbol",
        "DEMO",
        "--demo",
        "--human",
        "--output-format",
        "json",
    ]);
    assert!(cli.is_ok());
}
```

- [ ] **Step 2: Run the focused tests to confirm current mismatch still exists**

Run: `cargo test output_format -- --nocapture`

Expected: at least one analyze-path regression is missing or not protected at the dispatch level.

- [ ] **Step 3: Route `analyze` through the same conflict guard used elsewhere**

Keep `resolve_output_format(...)` as the single authority and ensure the analyze command path never bypasses it.

- [ ] **Step 4: Add a help-text contract note for JSON default vs alias flags**

Update analyze/workflow-status/factor-research/factor-backtest help strings so the operator understands:
- `json` is the default
- `--compact`, `--agent`, `--human` are sugar over `--output-format`

- [ ] **Step 5: Re-run the focused tests**

Run: `cargo test output_format -- --nocapture`

Expected: PASS.

---

### Task 2: Humanize Human Surfaces

**Files:**
- Modify: `src/main.rs`
- Modify: `src/application/reporting/analyze_output.rs`
- Modify: `src/application/reporting/agent_report.rs` only if shared formatting is cleaner

- [ ] **Step 1: Write failing tests proving human output must not leak `ask-user:` wire strings**

Add tests like:

```rust
#[test]
fn test_analyze_human_output_does_not_show_raw_ask_user_wire_protocol() {
    let rendered = /* build or render human analyze output with ask-user next step */;
    assert!(!rendered.contains("ask-user:"));
    assert!(rendered.contains("Ask the user"));
}

#[test]
fn test_factor_backtest_human_output_is_multiline_not_raw_json_dump() {
    let rendered = render_factor_backtest_human_output(/* sample report */, None);
    assert!(rendered.contains("Factor backtest"));
    assert!(!rendered.contains("\"factor_results\""));
}
```

- [ ] **Step 2: Run the focused tests to confirm they fail**

Run: `cargo test human_output -- --nocapture`

Expected: FAIL due to current wire-string leak and serializer-dump behavior.

- [ ] **Step 3: Introduce one human-facing next-step formatter**

Requirements:
- raw wire strings remain unchanged in JSON/agent surfaces
- human surfaces convert `ask-user:` protocol into natural language
- ordinary `ict-engine ...` commands remain displayable as commands

- [ ] **Step 4: Replace `factor-backtest --human` serializer dump with a real renderer**

Human renderer should include:
- best factor
- aggregate return
- trade count
- credibility summary or top warnings
- next step

- [ ] **Step 5: Verify analyze/factor-backtest human surfaces**

Run:
- `cargo test human_output -- --nocapture`
- `cargo run -- analyze --symbol DEMO --demo --human`
- `cargo run -- factor-backtest --symbol DEMO --data examples/demo/demo-15m.json --human`

Expected:
- no raw `ask-user:` in human output
- factor-backtest human mode is concise and multiline

---

### Task 3: Fix Empty-State Contracts For Agent Consumers

**Files:**
- Modify: `src/application/orchestration/workflow_status.rs`
- Modify: `src/application/release_closure/mod.rs`
- Test: `src/main.rs` or `tests/empty_state_contracts.rs`

- [ ] **Step 1: Write failing tests for empty `factor-autoresearch-status`**

Add assertions that empty state should not emit placeholder epochs/timestamps:

```rust
#[test]
fn test_factor_autoresearch_status_empty_state_returns_explicit_no_state_contract() {
    let value = factor_autoresearch_status_value_for_empty_state("DEMO", "state");
    assert_eq!(value["status"], "no_autoresearch_state");
    assert!(value["live_snapshot"].is_null());
}
```

- [ ] **Step 2: Write failing tests for zero-research-run verdict**

```rust
#[test]
fn test_research_verdict_requires_bootstrap_when_no_research_runs_exist() {
    let verdict = /* build verdict on empty state */;
    assert_eq!(verdict["stop_or_continue"], "bootstrap_required");
}
```

- [ ] **Step 3: Implement explicit empty-state shaping**

For `factor-autoresearch-status`:
- `status: "no_autoresearch_state"`
- `live_snapshot: null`
- `recommended_next_step` points to the bootstrap command

For `research-verdict`:
- no research runs means bootstrap, not continue

- [ ] **Step 4: Preserve backward compatibility on non-empty states**

Do not change fields for real populated states unless needed for consistency.

- [ ] **Step 5: Re-run the empty-state tests and real commands**

Run:
- `cargo test empty_state_contracts -- --nocapture`
- `cargo run -- factor-autoresearch-status --symbol DEMO --state-dir state --latest-only`
- `cargo run -- research-verdict --symbol DEMO --state-dir state`

Expected:
- empty surfaces are explicit, not placeholder-heavy

---

### Task 4: Clarify Workflow and Artifact Status Semantics

**Files:**
- Modify: `src/main.rs`
- Modify: `README.md`
- Test: `src/main.rs`

- [ ] **Step 1: Write a regression test for `artifact-status --latest-only` semantics**

The test should encode whichever behavior is intended:
- single latest overall, or
- latest-per-relevant-kind

```rust
#[test]
fn test_artifact_status_latest_only_contract_is_explicit() {
    // assert on count/shape semantics, not just presence
}
```

- [ ] **Step 2: Decide and codify the contract**

Preferred option:
- keep current behavior
- document it as “latest relevant entries” or “latest per kind”

Alternative:
- rename behavior and add a new explicit flag

- [ ] **Step 3: Align README command descriptions with actual runtime behavior**

Document:
- `workflow-status --json` is not currently an alias
- `artifact-status --latest-only` semantics
- demo support limitations for `backtest`

- [ ] **Step 4: Verify help and docs alignment**

Run:
- `cargo run -- workflow-status --help`
- `cargo run -- artifact-status --help`

Expected:
- docs and help no longer leave ambiguous semantics

---

### Task 5: Reconcile Audit and Contributor Expectations

**Files:**
- Modify: `docs/audit-2026-04-21-cross-surface-review.md`
- Modify: `README.md`
- Optionally create: `docs/CONTRIBUTOR_STATUS.md`

- [ ] **Step 1: Amend the earlier audit with current repo facts**

Add a short top note:
- `cargo test` now passes
- local CI file now exists
- `duration_sizing_scale` regression is fixed locally

- [ ] **Step 2: Document the real contributor baseline**

README or contributor status note should state:
- required: `cargo check`, `cargo test`
- current clippy status: advisory / noisy, not clean

- [ ] **Step 3: Capture current clippy mismatch in docs**

Do not claim clippy is clean unless it is actually clean.

- [ ] **Step 4: Verify documentation references**

Run: `rg -n "clippy|CI|duration_sizing_scale|cargo test" README.md docs/`

Expected:
- no stale contradictory statements remain in public-facing docs

---

### Task 6: Final Verification Sweep

**Files:**
- Modify only if fixes above require follow-up

- [ ] **Step 1: Run targeted regressions**

Run:
- `cargo test output_format -- --nocapture`
- `cargo test human_output -- --nocapture`
- `cargo test empty_state_contracts -- --nocapture`
- `cargo test workflow_status -- --nocapture`

- [ ] **Step 2: Run representative command scenarios**

Run:
- `cargo run -- analyze --symbol DEMO --demo --human`
- `cargo run -- analyze --symbol DEMO --demo --agent`
- `cargo run -- workflow-status --symbol DEMO --state-dir state --human`
- `cargo run -- workflow-status --symbol DEMO --state-dir state --agent`
- `cargo run -- factor-backtest --symbol DEMO --data examples/demo/demo-15m.json --human`
- `cargo run -- factor-autoresearch-status --symbol DEMO --state-dir state --latest-only`
- `cargo run -- research-verdict --symbol DEMO --state-dir state`

- [ ] **Step 3: Run repo baseline**

Run:
- `cargo check --all-targets`
- `cargo test`

- [ ] **Step 4: Record any intentional residual risks**

If any item remains unfixed, add a short residual-risk note to the shakedown audit or a follow-up plan doc.

---

## Priority Summary

### Phase 1

- Task 1: output-format conflict enforcement
- Task 2: human surface cleanup
- Task 3: empty-state contract cleanup

### Phase 2

- Task 4: workflow/artifact semantic clarification
- Task 5: audit and contributor expectation reconciliation

### Phase 3

- Task 6: final verification sweep

## Completion Criteria

- human surfaces do not leak raw `ask-user:` protocol strings
- `analyze` rejects mixed format directives consistently
- empty autoresearch/verdict states are explicit and agent-safe
- help/docs reflect actual runtime behavior
- repo verification remains green on `cargo check --all-targets` and `cargo test`

## Residual risks (updated post-follow-up, 2026-04-22)

- `cargo clippy --all-targets --no-deps` still emits repo-wide warnings across both lib and test targets. The touched `workflow_status` / `release_closure` paths were cleaned up, but clippy is still advisory overall and not a merge gate yet.
