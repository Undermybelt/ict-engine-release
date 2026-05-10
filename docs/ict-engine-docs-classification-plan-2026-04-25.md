# ICT Engine Docs Classification Plan

**Date:** 2026-04-25  
**Repo:** `/Users/thrill3r/projects-ict-engine/ict-engine`  
**Operator:** Cascade

## Goal

Organize the repository documentation into clear classes without deleting historical material, so contributors can distinguish:

- current canonical reference docs
- active engineering plans / design notes
- historical audits, trials, and remediation records
- retained anti-pattern / superseded / low-trust materials kept as negative examples

## Why This Work Is Needed

The `docs/` tree has accumulated multiple generations of:

- public reference docs
- engineering design notes
- audit artifacts
- trial plans and reports
- prompt-like research drafts

Some of these are useful as current guidance, while others are only useful as historical context or as examples of what not to treat as source-of-truth. Today that distinction is visible only by filename intuition.

## Scope

### In scope

1. Inventory the current `docs/` tree, including nested directories.
2. Define a small, explicit classification system.
3. Create a stable docs index / catalog inside the repo.
4. Mark outdated, incorrect, draft-like, or superseded materials as retained historical or anti-pattern references rather than deleting them.
5. Keep the change surface limited to documentation organization only.

### Out of scope

- Rewriting every legacy document into a new canonical standard.
- Deleting historical artifacts.
- Large-scale filename churn or risky path moves unless clearly justified.
- Changing code or runtime behavior.

## Classification Rules

Each document should be assigned one primary class:

1. **Canonical reference**
   - Current source of truth for contributors or operators.
   - Safe to link from `README.md` or use as default guidance.

2. **Active design / plan**
   - Forward-looking design, integration, or implementation planning.
   - Useful, but not source-of-truth for runtime behavior unless promoted.

3. **Historical record**
   - Trial plans, trial reports, audits, remediation chains, closeout notes.
   - Kept for traceability and context.

4. **Retained anti-pattern / low-trust draft**
   - Draft prompts, speculative proposals, outdated or misleading surfaces, or materials that should not be followed directly.
   - Retained deliberately as contrast, archaeology, or negative examples.

## Planned Output

1. A docs catalog file that groups the existing docs by class.
2. Short notes for ambiguous files explaining why they are canonical, historical, or anti-pattern.
3. A small README update if needed so readers can find the catalog quickly.

## Guardrails

- Prefer indexing and explicit status labels over mass file moves.
- Preserve historical traceability.
- If a file is wrong or obsolete, mark it as retained anti-pattern instead of silently deleting it.
- Do not claim a document is canonical unless the repo already treats it that way or the content clearly defines stable truth boundaries.

## Success Criteria

- A contributor can quickly answer: “Which docs should I trust first?”
- Historical trial / audit materials remain available.
- Wrong or obsolete docs are still preserved, but clearly marked as non-canonical.
- The resulting organization is reviewable and low-risk.
