# ICT Engine Post-Remediation Small Follow-Up Fix Plan

**Date:** 2026-04-25  
**Repo:** `/Users/thrill3r/projects-ict-engine/ict-engine`  
**Source:** `docs/ict-engine-post-remediation-ten-run-trial-report-2026-04-25.md`

## Goal

Address the small, non-blocking UX friction that remained after the report-driven remediation, while keeping the change surface narrow, reviewable, and free of new runtime or architectural debt.

## Why a Small Follow-Up

The critical issues are already fixed:

- `cargo build` works again
- human-readable `Next:` surfaces are executable again

What remains is not core engine breakage. It is onboarding / expectation-management polish. That means the follow-up should stay deliberately small and avoid reopening stable execution surfaces.

## Scope

Limit follow-up work to one or more of these surfaces only:

1. `--help` wording or defaults presentation for first-run clarity
2. repo docs / README examples that should keep steering users to explicit `/tmp/...` state directories
3. clearer user-facing wording around the demo backtest candle-count boundary

## Out of Scope

- Changing core trading / scoring / research logic
- Reworking Auto-Quant integration architecture
- Large refactors across reporting modules
- Repo-local runtime writes under ignored `state/`
- Any change that makes command defaults less truthful just to hide risk

## Candidate Small Fixes

### Option A: First-Run Help Guidance for `factor-research`

Add a very small help-text or nearby docs clarification that:

- `auto-quant` remains the default backend
- users who want the Rust-only path should pass `--backend native`
- the native demo path is the safest first-run branch

This is the highest-value clarification because it addresses the main remaining “surprising default” seen in the post-remediation ten-run trial.

### Option B: Stronger `/tmp` State Guidance in Docs / Examples

Tighten first-run examples so they consistently show:

- explicit `/tmp/...` `--state-dir`
- no-pollution usage as the default documented trial mode

This does not require lying about the real CLI default (`state`); it just makes the safe path more visible.

### Option C: Demo Backtest Boundary Message or Docs Note

Expose the current known limitation more proactively:

- bundled demo data has 52 candles
- backtest needs at least 71

Prefer docs/help clarification first. Only patch runtime wording if a tiny, precise improvement is clearly justified.

## Preferred Execution Order

1. Clarify `factor-research` first-run guidance
2. Tighten `/tmp` state-dir examples in user-facing docs
3. Only then consider a tiny demo-backtest wording improvement if still needed

## Guardrails

- Do not add new logic bodies to `src/main.rs`
- Prefer docs/help-text changes over behavioral changes
- Keep edits limited to the smallest possible file set
- Preserve truthfulness of actual CLI defaults
- If a wording fix can solve the issue, do not change engine behavior

## Success Criteria

- A first-run user can more easily infer the Rust-only path from help/docs
- Docs continue steering users away from repo-local runtime pollution
- Demo backtest boundary is easier to understand without changing engine semantics
- The diff stays small and easy to review

## Planned Verification

- `cargo build`
- one or two targeted `--help` / docs checks
- no repo-local runtime state writes during verification
