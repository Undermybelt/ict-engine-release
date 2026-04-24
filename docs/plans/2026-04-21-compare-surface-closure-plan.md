# Compare Surface Closure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the remaining compare-surface gaps by locking final JSON output fields, adding a real human-readable backtest/research compare rendering path, and propagating compare summaries into downstream reflection and agent-facing surfaces.

**Architecture:** Keep compare generation centered in `src/main.rs` for now, but stop hand-building JSON with inline `serde_json::json!` blocks. Extract small render helpers that build the final output payloads for `backtest`, `factor-backtest`, and `factor-research`, so tests can assert exact schema presence and human compare strings without capturing stdout. Once those helpers exist, add a thin human-render layer for the same three commands and thread compare summaries into reflection/agent consumers as summary evidence, not as a new source of truth.

**Tech Stack:** Rust 2021, existing `serde_json`, current `main.rs` command/render flow, `application::reflection`, `application::reporting`, `cargo test`, `cargo fmt`

---

## File Structure

- Modify: `src/main.rs`
- Modify: `src/application/reflection/mod.rs`
- Modify: `src/application/reflection/adapter.rs`
- Modify: `src/application/reporting/analyze_output.rs`
- Modify: `src/application/reporting/compact_report.rs` only if a shared compare summary carrier is needed
- Modify: `docs/plans/2026-04-21-compare-surface-closure-plan.md` only to check boxes if executing directly from this file

Responsibility split:

- `src/main.rs`: final output payload builders, human compare render helpers, command wiring, focused unit tests
- `src/application/reflection/mod.rs`: reflection bundle surface additions for compare summaries
- `src/application/reflection/adapter.rs`: adapter mapping if reflection bundle fields need to cross into final JSON
- `src/application/reporting/analyze_output.rs`: agent/human reporting glue if compare summaries need a stable reporting slot

## Scope Notes

- Current reality: `backtest`, `factor-backtest`, and `factor-research` do **not** accept `OutputFormat::Human` today.
- This plan therefore treats “human surface” as a concrete rendering path to add, not a tiny string tweak.
- Do not broaden into analyze/report redesign. The compare report remains the source artifact; all new human/agent text is derived from it.

### Task 1: Lock Final JSON Compare Fields With Render Helpers

**Files:**
- Modify: `src/main.rs`
- Test: `src/main.rs`

- [x] **Step 1: Write failing schema tests for the three final JSON payloads**

Add three tests near the existing compare tests that assert final payload values, not just helper strings:

```rust
#[test]
fn test_backtest_output_payload_includes_human_compare_summary() {
    let payload = build_backtest_output_payload(
        &sample_backtest_report_for_compare(),
        Some(sample_compare_report("scaled_down")),
        serde_json::json!({"compact": true}),
    );

    assert_eq!(
        payload["human_backtest_compare_summary"],
        serde_json::json!(
            "Backtest compare: duration_sizing_direction=scaled_down | risk=duration_sizing_scale_delta=-0.750 | next=inspect_duration_constraints"
        )
    );
    assert!(payload.get("compact_compare_report").is_some());
    assert!(payload.get("backtest_compare_report").is_some());
}

#[test]
fn test_factor_backtest_output_payload_includes_human_compare_summary() {
    let payload = build_factor_backtest_output_payload(
        &sample_factor_backtest_report_for_compare(),
        Some(sample_compare_report("scaled_down")),
        serde_json::json!({"credibility": true}),
        None,
    );

    assert!(payload.get("human_backtest_compare_summary").is_some());
}

#[test]
fn test_factor_research_output_payload_includes_human_compare_summary() {
    let payload = build_factor_research_output_payload(
        &sample_research_report_for_compare(),
        Some(sample_compare_report("scaled_up")),
        serde_json::json!({"reflection": true}),
        None,
        serde_json::json!({"lifecycle": true}),
    );

    assert_eq!(
        payload["human_research_compare_summary"],
        serde_json::json!(
            "Research compare: duration_sizing_direction=scaled_up | risk=duration_sizing_scale_delta=-0.750 | next=inspect_duration_constraints"
        )
    );
}
```

- [ ] **Step 2: Run the schema tests to verify they fail**

Run: `cargo test output_payload_includes_human_compare_summary -- --nocapture`

Expected: FAIL because `build_backtest_output_payload`, `build_factor_backtest_output_payload`, and `build_factor_research_output_payload` do not exist yet.

- [x] **Step 3: Add minimal payload builders in `src/main.rs`**

Extract the existing inline JSON construction into tiny helpers returning `serde_json::Value`:

```rust
fn build_backtest_output_payload(
    report: &BacktestReport,
    compare: Option<ict_engine::application::backtest::BacktestCompareReport>,
    compact_backtest_report: serde_json::Value,
) -> Value
```

```rust
fn build_factor_backtest_output_payload(
    report: &ict_engine::factor_lab::FactorBacktestReport,
    compare: Option<ict_engine::application::backtest::BacktestCompareReport>,
    credibility_summary: serde_json::Value,
    ensemble_surface: Option<serde_json::Value>,
) -> Value
```

```rust
fn build_factor_research_output_payload(
    report: &ict_engine::factor_lab::ResearchReport,
    compare: Option<ict_engine::application::backtest::BacktestCompareReport>,
    reflection_bundle: serde_json::Value,
    ensemble_surface: Option<serde_json::Value>,
    factor_lifecycle: serde_json::Value,
) -> Value
```

Inside each helper, derive these fields from `compare.as_ref()` only once:

```rust
let compact_compare_report = build_compact_compare_report(compare.as_ref());
let human_backtest_compare_summary = human_backtest_compare_summary(compare.as_ref());
let human_research_compare_summary = human_research_compare_summary(compare.as_ref());
```

Use those helpers from the command bodies instead of inline `serde_json::json!`.

- [x] **Step 4: Run the schema tests to verify they pass**

Run: `cargo test output_payload_includes_human_compare_summary -- --nocapture`

Expected: PASS for all three payload tests, with direct assertions on `human_*_compare_summary` and `compact_compare_report`.

- [x] **Step 5: Run the existing compare regression to prove no behavior drift**

Run: `cargo test test_run_factor_research_builds_compare_report_from_persisted_runs -- --nocapture`

Expected: PASS with the same persisted-runs compare behavior as before.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs docs/plans/2026-04-21-compare-surface-closure-plan.md
git commit -m "test: lock compare fields in final output payloads"
```

### Task 2: Add a Real Human Render Path for Backtest and Research Compare

**Files:**
- Modify: `src/main.rs`
- Test: `src/main.rs`

- [x] **Step 1: Write failing tests for human render output**

Add focused tests that assert a stable multi-line render, not just a one-line helper prefix:

```rust
#[test]
fn test_render_backtest_human_output_includes_compare_block() {
    let rendered = render_backtest_human_output(
        &sample_backtest_report_for_compare(),
        Some(&sample_compare_report("scaled_down")),
    );

    assert!(rendered.contains("Backtest ran with"));
    assert!(rendered.contains("Backtest compare:"));
    assert!(rendered.contains("risk=duration_sizing_scale_delta=-0.750"));
}

#[test]
fn test_render_research_human_output_includes_compare_block() {
    let rendered = render_factor_research_human_output(
        &sample_research_report_for_compare(),
        Some(&sample_compare_report("scaled_up")),
    );

    assert!(rendered.contains("Research compare:"));
    assert!(rendered.contains("next=inspect_duration_constraints"));
}
```

- [ ] **Step 2: Run the human render tests to verify they fail**

Run: `cargo test render_.*human_output.*compare_block -- --nocapture`

Expected: FAIL because the render helpers do not exist.

- [x] **Step 3: Implement minimal human render helpers**

Add dedicated renderers instead of retrofitting `OutputFormat` immediately:

```rust
fn render_backtest_human_output(
    report: &BacktestReport,
    compare: Option<&ict_engine::application::backtest::BacktestCompareReport>,
) -> String
```

```rust
fn render_factor_backtest_human_output(
    report: &ict_engine::factor_lab::FactorBacktestReport,
    compare: Option<&ict_engine::application::backtest::BacktestCompareReport>,
) -> String
```

```rust
fn render_factor_research_human_output(
    report: &ict_engine::factor_lab::ResearchReport,
    compare: Option<&ict_engine::application::backtest::BacktestCompareReport>,
) -> String
```

Minimal shape:

```rust
let mut lines = vec![base_summary_line.to_string()];
if let Some(compare_summary) = human_backtest_compare_summary(compare) {
    lines.push(compare_summary);
}
lines.join("\n")
```

Keep JSON output unchanged. Add a follow-up note in code comments that CLI flag exposure can come later once the render path is proven.

- [x] **Step 4: Thread the renderers into a callable surface**

Add one internal branch per command:

```rust
let human_output = render_backtest_human_output(&report, backtest_compare_report.as_ref());
```

Do not change clap args yet. Store the rendered string in the payload under a new explicit field:

```rust
"human_output": human_output,
```

This closes the human surface without broadening CLI parsing in the same patch.

- [x] **Step 5: Run the new tests to verify they pass**

Run: `cargo test human_output_includes_compare_block -- --nocapture`

Expected: PASS for backtest and research render tests, and the final JSON payload now contains `human_output`.

- [x] **Step 6: Run a smoke test for both payload and renderer**

Run: `cargo test human_compare_summary -- --nocapture`

Expected: PASS for the original helper tests plus the new render tests.

- [ ] **Step 7: Commit**

```bash
git add src/main.rs docs/plans/2026-04-21-compare-surface-closure-plan.md
git commit -m "feat: add human compare render path for backtest and research outputs"
```

### Task 3: Propagate Compare Summaries Into Reflection and Agent Consumers

**Files:**
- Modify: `src/application/reflection/mod.rs`
- Modify: `src/application/reflection/adapter.rs`
- Modify: `src/application/reporting/analyze_output.rs`
- Modify: `src/main.rs`
- Test: `src/main.rs`

- [x] **Step 1: Write failing tests for downstream compare consumption**

Add assertions that reflection or agent-facing structures receive compare text:

```rust
#[test]
fn test_research_reflection_bundle_includes_compare_summary() {
    let mut report = sample_research_report_for_compare();
    let bundle = build_research_reflection_bundle_with_compare(
        "NQ",
        &report,
        Some("Research compare: duration_sizing_direction=scaled_up | risk=duration_sizing_scale_delta=-0.750 | next=inspect_duration_constraints"),
    );

    assert!(bundle.evidence.iter().any(|line| line.contains("Research compare:")));
}

#[test]
fn test_agent_compare_surface_uses_compact_summary_when_present() {
    let value = build_factor_research_output_payload(
        &sample_research_report_for_compare(),
        Some(sample_compare_report("scaled_up")),
        serde_json::json!({"reflection": true}),
        None,
        serde_json::json!({"lifecycle": true}),
    );

    assert!(value["reflection_bundle"]
        .to_string()
        .contains("Research compare:"));
}
```

- [ ] **Step 2: Run the downstream tests to verify they fail**

Run: `cargo test compare_summary -- --nocapture`

Expected: FAIL because reflection and agent payloads do not yet ingest compare summaries.

- [x] **Step 3: Add a single compare summary field to the reflection bundle path**

Prefer one field over many:

```rust
pub compare_summary: Option<String>,
```

If the reflection type already has a free-form evidence vector, append instead of adding a second redundant vector:

```rust
if let Some(compare_summary) = compare_summary {
    bundle.evidence.push(compare_summary.to_string());
}
```

Use the research-specific summary for research flows and backtest-specific summary for backtest flows.

- [x] **Step 4: Surface compare summary through the final payload**

When building `reflection_bundle` in `factor_research_command`, pass the summary in explicitly:

```rust
let compare_summary = human_research_compare_summary(research_compare_report.as_ref());
let reflection_bundle =
    build_research_reflection_bundle_with_compare(symbol, &report, compare_summary.as_deref());
```

If `adapter.rs` or `analyze_output.rs` already defines a stable place for “supporting evidence”, map the same string there rather than inventing a parallel field.

- [x] **Step 5: Run the downstream tests to verify they pass**

Run: `cargo test compare_summary -- --nocapture`

Expected: PASS with compare text visible in the reflection/agent-facing path.

- [x] **Step 6: Run the narrow regression set**

Run: `cargo test test_run_factor_research_builds_compare_report_from_persisted_runs -- --nocapture`

Run: `cargo test test_human_backtest_compare_summary_labels_backtest_surface -- --nocapture`

Run: `cargo test test_human_research_compare_summary_labels_research_surface -- --nocapture`

Expected: PASS for all three, proving persisted compare generation still works and the summary labels remain stable.

- [ ] **Step 7: Commit**

```bash
git add src/main.rs src/application/reflection/mod.rs src/application/reflection/adapter.rs src/application/reporting/analyze_output.rs docs/plans/2026-04-21-compare-surface-closure-plan.md
git commit -m "feat: propagate compare summaries into reflection and agent surfaces"
```

## Verification Checklist

- [x] Final JSON payload tests assert `human_backtest_compare_summary` for `backtest`
- [x] Final JSON payload tests assert `human_backtest_compare_summary` for `factor-backtest`
- [x] Final JSON payload tests assert `human_research_compare_summary` for `factor-research`
- [x] Human render helpers produce stable multi-line text with compare lines
- [x] Reflection or agent-facing consumer receives a derived compare summary
- [x] Persisted-runs compare regression still passes after render/payload extraction
- [x] `cargo fmt --all` runs cleanly

## Self-Review

- Spec coverage: the three known gaps are covered by Task 1 (schema lock), Task 2 (real human render path), and Task 3 (downstream reflection/agent consumption).
- Placeholder scan: no `TODO`/`TBD` placeholders remain; each task has concrete files, commands, and expected failures/passes.
- Type consistency: the plan uses the same existing compare artifact type everywhere, `ict_engine::application::backtest::BacktestCompareReport`, and keeps the human surface derived from that single source.
