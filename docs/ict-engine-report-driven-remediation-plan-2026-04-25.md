# ICT Engine Report-Driven Remediation Plan

**Date:** 2026-04-25  
**Repo:** `/Users/thrill3r/projects-ict-engine/ict-engine`  
**Operator:** Cascade  
**Input:** `docs/ict-engine-ten-run-user-trial-report-2026-04-25.md`

## Goal

Apply the report in priority order while preserving two constraints:

- no pollution
- no new structural debt

## Scope

### In scope

1. Restore fresh `cargo build`.
2. If the build fix lands cleanly, repair the human `Next:` surface so it is executable rather than placeholder-only.
3. Verify using focused commands and `/tmp` runtime state only.

### Out of scope

- Factor tuning.
- Auto-Quant product redesign.
- Repo-local runtime writes under ignored `state/`.
- Large refactors unrelated to the current break/fix surface.
- “Temporary” hacks that knowingly deepen output/path inconsistency.

## Guardrails

- Prefer the smallest library-level fix over CLI-surface duplication.
- Do not add new logic bodies to `src/main.rs` unless unavoidable entrypoint glue is required.
- If path redaction and executable next-step behavior conflict, resolve it by making the behavior explicit and testable rather than by adding more ad hoc placeholder rules.
- Verification must use `/tmp/ict-engine-report-remediation-20260425` or tighter throwaway paths.

## Planned Steps

1. Reproduce and localize the current `cargo build` failure.
2. Fix the missing/incorrect analyze output reporting surface references.
3. Re-run `cargo build` and targeted command help/smoke checks.
4. Inspect `Next:` generation for human output surfaces.
5. Implement the smallest coherent fix so human `Next:` commands are executable without regressing path hygiene.
6. Re-run focused smoke commands with `/tmp` state and document outcomes.

## Success Criteria

- `cargo build` succeeds on the current source tree.
- `analyze --human`, `factor-research --human`, and `workflow-status --human` do not emit misleading placeholder-only next-step commands.
- No repo-local runtime state is created by verification.
- The change surface stays tight and reviewable.
