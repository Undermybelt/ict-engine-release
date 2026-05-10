# Full Docs Plan/Reality Audit · 2026-04-24

Purpose
- audit the full `docs/` tree for features or constraints that docs imply should exist
- verify whether current code/CLI still matches those claims
- identify stale or misleading docs that could make a contributor think functionality is present or absent when the repo says otherwise

Scope
- audited `109` files under `docs/`
- used full-text scan plus targeted spot review of current entry surfaces and recent state-transition/release docs
- did not treat clearly historical plans/audits as bugs if they already carry an explicit stale/historical status note

Method
1. full-tree grep for time-sensitive doc claims:
   - `main.rs` hotspot / “substantially closed”
   - oversized-history / release transport blocker
   - wrapper path assumptions / machine-local path claims
   - `research-verdict` / contamination / `evidence-quality-breakdown` capability claims
2. targeted review of:
   - `README.md`
   - `docs/first-run.md`
   - `docs/release-mirror-runbook.md`
   - `docs/audits/release-signoff.md`
   - `docs/plans/2026-04-24-post-main-debt-inventory.md`
   - `docs/research-system-map.md`
   - `docs/2026-04-24-open-source-shakedown-handoff.md`
3. current-repo truth compared against:
   - current wrapper behavior (`--show-config`, data-readiness gate)
   - current release routing (source repo pushable again, mirror `v0.1.0` exists)
   - current runtime-hotspot status (`main.rs` extraction line landed)
   - current release-closure surface depth

## Findings

### Active findings

These are docs that still act like current operator docs, but contain at least one stale or incomplete instruction.

#### 1. `docs/release-notes-draft.md` still contains stale wrapper/run guidance

Severity
- medium

Why it matters
- this file is still written like a current release-facing note, not an archival note
- it still contains examples and caveats that no longer match the wrapper behavior we now enforce

Current mismatches
- it still lists `python3 scripts/search_local.py --run` as the “new habit” instead of the guarded flow
- it still says “some experiments still assume local cleaned-data layouts and repo-root-relative state conventions”
  - current wrapper contract is stricter than that:
    - wrappers expose `--show-config`
    - wrappers refuse `--run` if `cleaned_data_ready=false`
    - explicit `--data-root` is the documented path

Evidence
- `docs/release-notes-draft.md:67`
- `docs/release-notes-draft.md:79`

Recommended action
- update examples to the guarded flow:
  - `--show-config`
  - then `--run --data-root <ict-cleaned-mtf>`
- reword caveat to “data must be explicitly ready” rather than “the script may assume local layout”

#### 2. `docs/research-system-map.md` still contains one stale direct-run workflow

Severity
- medium

Why it matters
- this doc reads like a current system/operator map
- one workflow line still suggests direct `--run` on a public wrapper without the explicit config/readiness step

Current mismatch
- Workflow 1 was updated to the guarded flow
- but the later bottleneck workflow still includes:
  - `python3 scripts/evaluate_bottleneck.py --run`
  - without the new `--show-config` / `--data-root` gate

Evidence
- `docs/research-system-map.md:230`

Recommended action
- align all wrapper workflows in this file to the same explicit pattern

### Archival findings

These are stale statements, but they are acceptable as historical context because they are already clearly marked as such or sit inside historical plans/audits.

#### 3. `docs/main-rs-extraction-closeout-2026-04-23.md` still contains the old “substantially closed” body text

Severity
- low

Why it is acceptable
- the file already has a front-loaded status note declaring the body stale and historical

Evidence
- `docs/main-rs-extraction-closeout-2026-04-23.md:10`
- `docs/main-rs-extraction-closeout-2026-04-23.md:58`

Action
- no urgent change required

#### 4. `docs/plans/2026-04-24-one-shot-structural-debt-closure-plan.md` still describes the old pre-landing debt picture

Severity
- low

Why it is acceptable
- the file already marks itself as largely executed historical context

Evidence
- `docs/plans/2026-04-24-one-shot-structural-debt-closure-plan.md:12`

Action
- no urgent change required

#### 5. Older external or audit notes still mention source-repo oversized-history sensitivity

Severity
- low

Why it is acceptable
- these are historical intake/review notes, not current release instructions

Evidence
- `docs/external/autoresearch-pattern-intake-2026-04-23.md:116`

Action
- no urgent change required

## Non-findings

These were checked and do not currently look like missing-feature regressions.

- `README.md`
  - current wrapper contract is documented correctly
- `docs/first-run.md`
  - current wrapper contract is documented correctly
- `docs/release-mirror-runbook.md`
  - now reflects that source history blocker is cleared, while mirror release remains preferred
- `docs/audits/release-signoff.md`
  - now reflects the post-history-cleanup state
- `docs/plans/2026-04-24-post-main-debt-inventory.md`
  - current post-`main.rs` debt picture is aligned with repo reality
- `docs/2026-04-24-open-source-shakedown-handoff.md`
  - current wrapper guardrail invariant is explicitly documented

## Bottom line

The repo does **not** currently look like it lost major previously-documented functionality due to rollback.

What the audit found instead:
- most stale docs are now historical docs that already carry explicit stale/historical framing
- the remaining real doc debt is concentrated in a small number of active operator docs
- the two active docs still worth fixing are:
  - `docs/release-notes-draft.md`
  - `docs/research-system-map.md`

## Recommended next step

Make one small doc-only follow-up that:
- updates `docs/release-notes-draft.md` to the guarded wrapper flow
- updates the remaining wrapper examples in `docs/research-system-map.md`

Tree status at audit time
- clean
