# ICT Engine Docs Catalog · 2026-04-25

Purpose: give the `docs/` tree an explicit trust map without deleting history.

This catalog keeps three things separate:

- current docs you can trust first
- design docs that are useful but not yet canonical
- historical or low-trust docs that must be preserved for traceability or as negative examples

## How to use this catalog

Read in this order:

1. `README.md`
2. this catalog
3. canonical reference docs
4. active design docs if you are changing architecture or adding new surfaces
5. historical / low-trust docs only when you need archaeology, rationale, or counterexamples

## Trust classes

- `CANONICAL` — current source-of-truth or default starting point
- `ACTIVE DESIGN` — useful current design input, but not runtime truth by itself
- `HISTORICAL` — preserved plans, audits, trials, reviews, and closeout records
- `LOW-TRUST / NEGATIVE EXAMPLE` — retained speculative, superseded, misleading, rejected, or placeholder material that should not be followed directly

## Subtree rules

These subtree rules cover all descendants unless an override is listed.

### `docs/audits/`
- **Class:** `HISTORICAL`
- **Why:** audit chain and machine-generated evidence artifacts
- **Use for:** release archaeology, prior verification context, help audit receipts
- **Do not use as:** default source-of-truth for current behavior unless a canonical doc points back to it

### `docs/plans/`
- **Class:** `HISTORICAL`
- **Why:** implementation-plan archive; useful for design lineage, not default truth
- **Use for:** understanding why a feature or refactor was proposed
- **Do not use as:** proof that a plan landed unchanged

### `docs/experiments/`
- **Default class:** `ACTIVE DESIGN`
- **Overrides:**
  - `docs/experiments/eml-regime-fusion-poc.md` -> `LOW-TRUST / NEGATIVE EXAMPLE`
- **Why:** experimental schema/probe documents are candidate research surfaces, except the explicitly rejected EML PoC

### `docs/external/`
- **Default class:** `HISTORICAL`
- **Overrides:**
  - `docs/external/adapter-contract.md` -> `ACTIVE DESIGN`
  - `docs/external/error-taxonomy.md` -> `ACTIVE DESIGN`
  - `docs/external/external-patterns-synthesis-2026-04-23.md` -> `ACTIVE DESIGN`
  - `docs/external/tool-catalog.schema.json` -> `ACTIVE DESIGN`
- **Why:** intake notes are historical pattern research; the contract/taxonomy/schema/synthesis files are current design inputs

### `docs/paper-code/`
- **Default class:** `HISTORICAL`
- **Overrides:**
  - `docs/paper-code/bayesian-decision-tree/` -> `LOW-TRUST / NEGATIVE EXAMPLE`
  - `docs/paper-code/ising-phase-transition/` -> `LOW-TRUST / NEGATIVE EXAMPLE`
- **Why:** reproduction packs are preserved research support; empty placeholder / duplicate subtrees are retained only for archaeology

### `docs/paper-notes/`
- **Class:** `LOW-TRUST / NEGATIVE EXAMPLE`
- **Why:** currently an empty placeholder, not a trustable documentation surface

## Canonical reference docs

### Public / contributor-facing

- `docs/ict-engine-docs-catalog-2026-04-25.md`
  - Trust map for the whole docs tree.
- `docs/first-run.md`
  - Safe first-run entry surface.
- `docs/research-system-map.md`
  - Command and artifact map for research flows.
- `docs/autoresearch-derived-surfaces-contract.md`
  - Canonical truth boundary between authoritative JSON and derived convenience outputs.
- `docs/autoresearch-state-transitions.md`
  - Canonical write-order and transition semantics.
- `docs/objective-scoring-map.md`
  - Current score semantics and contamination warnings.
- `docs/smoke-acceptance.md`
  - Current smoke / acceptance route.
- `docs/environment-variables.md`
  - Current environment-variable precedence and meanings.
- `docs/state-directory-lifecycle.md`
  - Current state-dir behavior and comparability guidance.

### Internal / operator-facing

- `docs/agent-first-runbook.md`
  - Internal/operator routing and task entrypoints.
- `docs/architecture-boundaries.md`
  - Current architecture boundary rules.
- `docs/main-rs-guardrails.md`
  - Current guardrails for keeping `src/main.rs` thin.
- `docs/compact_agent_routing.md`
  - Canonical compact routing for low-token repo use.
- `docs/release-mirror-runbook.md`
  - Current authoritative release procedure.
- `docs/auto-quant-ictengine-integration-guide.md`
  - Stable operator/agent reference for the Auto-Quant boundary.

## Active design docs

These are useful inputs for current work, but code / CLI / canonical docs still outrank them.

- `docs/auto-quant-integration-plan.md`
  - Calibrated Auto-Quant absorption plan.
- `docs/external-integration-plan.md`
  - Repo-reality-aligned external integration plan.
- `docs/bbn-filter-first-realignment.md`
  - Ordering/design correction for filter -> pre-bayes -> belief routing.
- `docs/hybrid-regime-clustering-integration-note.md`
  - Regime-clustering integration ideas with repo-aligned sequencing.
- `docs/logic-family-layered-cpt-plan.md`
  - Candidate logic-family CPT layering plan.
- `docs/paper-driven-typed-packets-design.md`
  - Typed-packets design direction.
- `docs/typed-packets-paper-upgrade-plan.md`
  - Upgrade plan for the typed-packets line.
- `docs/risk-management.md`
  - Risk-layer design proposal, not current runtime truth.
- `docs/tomac-entry-logic-lexicon.md`
  - Naming/spec input for entry-logic surfaces.

## Historical record docs

These are valuable context, but they are not the first documents to trust for current behavior.

- `docs/2026-04-24-open-source-shakedown-handoff.md`
  - Contributor/agent handoff snapshot.
- `docs/anti-drift-skill-research-notes.md`
  - Hermes/anti-drift research notes preserved for context.
- `docs/audit-2026-04-21-cross-surface-review.md`
  - Broad cross-surface audit snapshot.
- `docs/audit-2026-04-21-full-codebase-shakedown.md`
  - Whole-repo shakedown audit snapshot.
- `docs/backend-path-audit.md`
  - Historical backend portability audit; explicitly no longer current state.
- `docs/btc-ledger-bucket-research.md`
  - Bucketed ledger research record.
- `docs/btc-ledger-bucket-verdict.md`
  - Ledger bucket verdict record.
- `docs/btc-ledger-factor-intake.md`
  - Ledger intake artifact record.
- `docs/btc-ledger-ict-translation.md`
  - Ledger-to-ICT translation record.
- `docs/change-surface.md`
  - Task-scoped change envelope from a prior assessment.
- `docs/drift-ledger.md`
  - Prior drift assessment log.
- `docs/execution-first-4-sprint-next-steps.md`
  - Roadmap snapshot / next-step derivative plan.
- `docs/execution-first-4-sprint-plan.md`
  - Roadmap snapshot.
- `docs/execution-paper-notes-and-plan-update.md`
  - Research-note-driven roadmap update.
- `docs/hermes_compact_timed_pda_recovery.md`
  - Recovery snapshot for a prior timed-PDA repair effort.
- `docs/ict-engine-docs-classification-plan-2026-04-25.md`
  - Planning artifact for this catalog.
- `docs/ict-engine-first-run-fix-plan-2026-04-25.md`
  - Small fix plan artifact.
- `docs/ict-engine-first-run-trial-plan-2026-04-25.md`
  - Trial plan artifact.
- `docs/ict-engine-first-run-trial-report-2026-04-25.md`
  - Trial report artifact.
- `docs/ict-engine-post-remediation-small-followup-fix-plan-2026-04-25.md`
  - Follow-up fix plan artifact.
- `docs/ict-engine-post-remediation-ten-run-trial-plan-2026-04-25.md`
  - Ten-run trial plan artifact.
- `docs/ict-engine-post-remediation-ten-run-trial-report-2026-04-25.md`
  - Ten-run trial report artifact.
- `docs/ict-engine-postfix-trial-log-2026-04-25.md`
  - Postfix trial log artifact.
- `docs/ict-engine-report-driven-remediation-plan-2026-04-25.md`
  - Report-driven remediation plan artifact.
- `docs/ict-engine-smoke-bug-hunt-plan-2026-04-25.md`
  - Smoke bug-hunt plan artifact.
- `docs/ict-engine-ten-run-user-trial-plan-2026-04-25.md`
  - User-trial plan artifact.
- `docs/ict-engine-ten-run-user-trial-report-2026-04-25.md`
  - User-trial report artifact.
- `docs/ict-factor-mutation-optimization-plan.md`
  - Experiment-specific optimization plan.
- `docs/main-rs-extraction-closeout-2026-04-23.md`
  - Historical closeout with its own stale/superseded warning.
- `docs/project_optimization_review.md`
  - Historical project review and recommendations.
- `docs/release-notes-draft.md`
  - Draft release notes, not authoritative runtime truth.
- `docs/repo-bbn-cpt-loader-notes.md`
  - Historical loader note / blocker snapshot.
- `docs/research-gaps-and-paper-synthesis.md`
  - Historical synthesis / gap review.
- `docs/the_well-physics-sim-ict-insights.md`
  - Cross-domain inspiration notes.

## Low-trust / negative-example docs

These files are preserved on purpose, but should not be followed directly.

- `docs/GAP_REMEDIATION_PLAN.md`
  - Legacy remediation narrative; do not trust completion claims without code verification.
- `docs/bbn_upgrade_plan.md`
  - Explicit upgrade prompt, not an implemented or repo-grounded plan.
- `docs/oracle-labelling`
  - Speculative research proposal draft; not production truth.
- `docs/pda_type`
  - Rich heuristic draft for execution-layer ideation, not canonical PDA truth.
- `docs/regime-aware`
  - Prompt-like success-pattern note, not repo source-of-truth.
- `docs/experiments/eml-regime-fusion-poc.md`
  - Explicitly rejected experiment verdict; keep as a negative example.

## Quick routing summary

If you only need the shortest correct route:

- **First run / safe usage:** `docs/first-run.md`
- **Research system map:** `docs/research-system-map.md`
- **State and derived-surface truth:** `docs/autoresearch-derived-surfaces-contract.md`, `docs/autoresearch-state-transitions.md`, `docs/state-directory-lifecycle.md`
- **Scoring semantics:** `docs/objective-scoring-map.md`
- **Architecture guardrails:** `docs/architecture-boundaries.md`, `docs/main-rs-guardrails.md`
- **Release flow:** `docs/release-mirror-runbook.md`
- **Need archaeology / prior rationale:** `docs/audits/`, `docs/plans/`, and the dated 2026-04-25 trial/remediation docs
- **Need counterexamples of what not to trust directly:** `docs/GAP_REMEDIATION_PLAN.md`, `docs/bbn_upgrade_plan.md`, `docs/oracle-labelling`, `docs/pda_type`, `docs/regime-aware`, `docs/experiments/eml-regime-fusion-poc.md`
