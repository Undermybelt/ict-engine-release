# ICT Engine Docs Map

Purpose: make `support/docs/` navigable, keep current guidance separate from historical
evidence, and stop old trial reports from competing with live execution boards.

This file replaces the old April 25 docs catalog and also consolidates the
useful parts of the deleted April 25 first-run, Auto-Quant, Tomac, and live
factor validation notes.

## Trust Classes

- `CANONICAL`: current source of truth or default starting point.
- `LIVE BOARD`: current executable todo or handoff board. Update in place while
  that lane is active.
- `ACTIVE DESIGN`: useful design input, but code, CLI output, and canonical docs
  outrank it.
- `HISTORICAL`: audit, trial, closeout, or rationale record. Useful for
  archaeology only.
- `LOW TRUST`: speculative, rejected, misleading, prompt-like, or superseded
  material retained only as contrast or warning.

## Folder Policy

- Top-level Markdown files in `support/docs/`: canonical references, stable operator guides, and a small
  number of currently important research briefs.
- `support/docs/plans/`: executable plans, live boards, and handoff todos.
- `support/docs/audits/`: audits, release checks, and verification records.
- `support/docs/experiments/`: experimental probes and research sidecars.
- `support/docs/external/`: external-provider contracts, pattern intake, schemas, and
  third-party integration notes.
- `support/docs/bug/`: bug logs and trial defects.
- `support/docs/todo/`: small backlog notes that are not active execution boards.

Do not add new top-level dated trial reports. Put them under `support/docs/audits/`,
`support/docs/plans/`, or `support/docs/experiments/`, then link them here only if they become
important routing surfaces.

## Read First

Public and contributor-facing:

- `support/docs/first-run.md`: clone-to-useful-output guide.
- `support/docs/smoke-acceptance.md`: execution-level smoke and acceptance flow.
- `support/docs/research-system-map.md`: research subsystem map.
- `support/docs/autoresearch-derived-surfaces-contract.md`: authoritative JSON vs
  derived convenience output boundaries.
- `support/docs/autoresearch-state-transitions.md`: autoresearch write-order and state
  transitions.
- `support/docs/objective-scoring-map.md`: objective scoring semantics and contamination
  warnings.
- `support/docs/environment-variables.md`: env var precedence and meanings.
- `support/docs/state-directory-lifecycle.md`: state directory behavior and
  comparability guidance.

Internal and operator-facing:

- `support/docs/agent-first-runbook.md`: operator entrypoints.
- `support/docs/architecture-boundaries.md`: architecture boundary rules.
- `support/docs/main-rs-guardrails.md`: guardrails for keeping `src/main.rs` thin.
- `support/docs/compact_agent_routing.md`: compact low-token routing.
- `support/docs/release-mirror-runbook.md`: release mirror procedure.
- `support/docs/auto-quant-ictengine-integration-guide.md`: stable Auto-Quant boundary.

Source-referenced docs kept in place for path stability:

- `support/docs/external/kraken-cli-agent-patterns.md`
- `support/docs/2026-04-26-auto-quant-real-trades-plan.md`
- `support/docs/2026-04-26-auto-quant-live-signals-plan.md`
- `support/docs/2026-04-26-auto-quant-bbn-prior-init-plan.md`

## Current Live Boards

These are active execution surfaces. Runtime decisions should resolve through
`ict-engine` code, state artifacts, provider configs, candidate packs, and these
compact current-state docs. The old May 10 A/B logs are not live dependencies.

- `support/docs/plans/2026-05-12-board-a-regime-state-current.md`: compact current
  Board A contract for market-state/regime confidence.
- `support/docs/plans/2026-05-12-board-b-profit-factor-current.md`: compact current
  Board B contract for regime-rooted profitability-factor training.
- `support/docs/plans/2026-05-12-board-ab-cleanup-retention-plan.md`: cleanup,
  retention, and deletion gate for the oversized A/B logs.
- `support/docs/plans/2026-05-09-factor-iteration-pre-bayes-bbn-catboost-execution-tree-todo.md`:
  current Auto-Quant -> filter -> BBN -> CatBoost/path-ranker -> execution-tree
  chain board.
- `support/docs/plans/2026-05-05-execution-tree-factor-auto-quant-todo.md`: broad
  factor-family and execution-tree factor-supply board.
- `support/docs/plans/2026-05-07-auto-quant-post-factor-runtime-closure-todo.md`:
  post-factor runtime closure board.

## Active Design Inputs

- `support/docs/auto-quant-integration-plan.md`: calibrated Auto-Quant absorption plan.
- `support/docs/external-integration-plan.md`: external integration plan aligned to repo
  reality.
- `support/docs/bbn-filter-first-realignment.md`: filter -> pre-Bayes -> belief routing
  correction.
- `support/docs/hybrid-regime-clustering-integration-note.md`: regime clustering
  sequencing ideas.
- `support/docs/logic-family-layered-cpt-plan.md`: logic-family CPT layering candidate.
- `support/docs/paper-driven-typed-packets-design.md`: typed-packets design direction.
- `support/docs/typed-packets-paper-upgrade-plan.md`: typed-packets upgrade plan.
- `support/docs/risk-management.md`: risk-layer proposal, not runtime truth.
- `support/docs/tomac-entry-logic-lexicon.md`: entry-logic naming/spec input.
- `support/docs/market-regime-profitable-strategy-research-2026-05-10.md`: current
  market-regime/profitable-strategy research prompt and chain context.

## Retained Low-Trust Examples

These files are not default guidance:

- `support/docs/GAP_REMEDIATION_PLAN.md`: legacy remediation narrative; verify against
  code before trusting claims.
- `support/docs/bbn_upgrade_plan.md`: upgrade prompt, not implemented truth.
- `support/docs/oracle-labelling`: speculative research proposal draft.
- `support/docs/pda_type`: rich heuristic draft, not canonical PDA truth.
- `support/docs/regime-aware`: prompt-like success-pattern note.
- `support/docs/experiments/eml-regime-fusion-poc.md`: rejected experiment verdict.

## Consolidated April 25 Archive

The following sections preserve the useful conclusions from the removed
top-level April 25 reports and plans.

### First-Run And CLI UX Chain

Initial first-run evidence showed the Rust-only demo path could work, but the
default first-run story was too easy to derail:

- `analyze --demo --human` was the best first screen: short, readable, and
  actionable.
- `factor-research` defaulted to `auto-quant`, which exposed external dependency
  setup too early for users only trying the core CLI.
- `factor-research --backend native --human` originally returned an oversized
  JSON-like summary and showed UTF-8 path/prompt issues.
- Demo backtest failure on the bundled 52-candle sample was a valid product
  boundary, not a runtime regression.
- `workflow-status --human` was useful but needed compact next-step output.

The later ten-run and post-fix trials confirmed the repair direction:

- Fresh build needed to pass before any user-facing path could be trusted.
- Human `Next:` lines needed to stay copyable and preserve `--backend native`
  for Rust-only loops.
- Explicit `/tmp/...` state dirs should be shown in onboarding examples to avoid
  repo-local runtime pollution.
- After remediation, `factor-research --human` and `workflow-status --human`
  emitted concise output and direct commands.
- `factor-pipeline-debug` remained valuable but intentionally verbose, so it
  should stay diagnostic rather than first-screen onboarding.

### Auto-Quant Integration Chain

The April 25 Auto-Quant experiments established the correct control-plane
boundary:

- `ict-engine auto-quant-status`, `auto-quant-bootstrap`,
  `factor-research --backend auto-quant`, and
  `factor-autoresearch --backend auto-quant` could drive Auto-Quant to
  `ready_for_external_run`.
- The low-pollution parallel pattern was one shared managed Auto-Quant checkout
  plus isolated `--state-dir` lanes.
- `prepare.py` could advance readiness from data missing to data ready.
- The first true blocker was not dependency management or data preparation; it
  was missing strategy files under `user_data/strategies/`.
- Once seed strategies were actually generated and backtested, the loop produced
  usable candidates. The strongest archived seed from that run was
  `BTCLeaderBreakX` with Sharpe `1.0716` and positive return across five pairs.
- The useful seed set from that archive was `BTCLeaderBreakX`,
  `MTFTrendStack` baseline, and `VolBBSqueeze`.

Operational lesson: an Auto-Quant handoff must fail closed if no strategy files
exist, then require the agent to create two or three seed strategies and run the
first backtest before claiming completion.

### Tomac Material Intake

The Tomac validation notes showed that external material intake worked within
the intended boundary:

- Real Tomac directories could be discovered read-only.
- Python and CSV summaries could enter handoff payloads, notes, and agent
  prompts.
- The flow stayed explicit opt-in, read-only, non-executing, and non-hard
  dependency.
- Ranking by largest `trade_rows` was not good enough for seed selection.

Recommended retained rule: prefer seed-guidance quality over raw row count. Give
extra weight to readable strategy names, paired `.py`/`.csv` evidence, useful
columns such as `Score`, adequate but not maximal trade counts, and conservative
family suffix normalization such as `_es`, `_nq`, `_ym`, `_eur`, `_xau`, `_pro`,
`_final`, `_v2`, and `_v3`.

### Live Factor Evidence Chain

The live-factor validation report established a precise boundary:

- Verified chain: live data -> `analyze-live` -> `FactorEngine` -> pre-Bayes
  filter -> existing BBN node evidence -> execution tree artifact -> ensemble
  vote artifact.
- Factor diagnostics are projected into existing BBN evidence and policy feature
  vectors.
- This is not the same as creating new BBN nodes from factors.
- The CatBoost-compatible policy surface was file-backed/sample-compatible, not
  a proven trained CatBoost runtime.
- The next real closure step was replacing sample policy JSON with trained model
  artifacts while preserving the same `PolicyFeatureVector` boundary.

## Removed Into This Archive

These redundant top-level documents were consolidated here and deleted. The
names below are retained as historical source names, not live paths:

- `ict-engine-docs-catalog-2026-04-25.md`
- `ict-engine-docs-classification-plan-2026-04-25.md`
- `2026-04-25-auto-quant-parallel-try-plan.md`
- `2026-04-25-auto-quant-parallel-try-report.md`
- `2026-04-25-auto-quant-seeded-strategy-backtest-report.md`
- `2026-04-25-auto-quant-tomac-live-handoff-validation-plan.md`
- `2026-04-25-auto-quant-tomac-live-handoff-validation-report.md`
- `2026-04-25-auto-quant-tomac-material-ingestion-plan.md`
- `2026-04-25-auto-quant-tomac-material-ranking-fix-plan.md`
- `2026-04-25-auto-quant-tomac-seed-evidence-plan.md`
- `2026-04-25-live-factor-evidence-validation-plan.md`
- `2026-04-25-live-factor-evidence-validation-report.md`
- `ict-engine-first-run-fix-plan-2026-04-25.md`
- `ict-engine-first-run-trial-plan-2026-04-25.md`
- `ict-engine-first-run-trial-report-2026-04-25.md`
- `ict-engine-post-remediation-small-followup-fix-plan-2026-04-25.md`
- `ict-engine-post-remediation-ten-run-trial-plan-2026-04-25.md`
- `ict-engine-post-remediation-ten-run-trial-report-2026-04-25.md`
- `ict-engine-postfix-trial-log-2026-04-25.md`
- `ict-engine-report-driven-remediation-plan-2026-04-25.md`
- `ict-engine-smoke-bug-hunt-plan-2026-04-25.md`
- `ict-engine-ten-run-user-trial-plan-2026-04-25.md`
- `ict-engine-ten-run-user-trial-report-2026-04-25.md`

## Moved For Folder Hygiene

These useful historical audits were moved out of the top-level docs namespace:

- `audit-2026-04-21-cross-surface-review.md` ->
  `support/docs/audits/2026-04-21-cross-surface-review.md`
- `audit-2026-04-21-full-codebase-shakedown.md` ->
  `support/docs/audits/2026-04-21-full-codebase-shakedown.md`

## Cleanup Rules

- Keep canonical docs and live boards in place.
- Delete or consolidate old top-level trial reports once their conclusions are
  summarized here.
- Do not move source-referenced docs unless all code comments, README links, and
  docs links are updated in the same change.
- Do not treat `support/docs/plans/` as proof that work landed; plans are evidence of
  intent unless a current board or verification artifact says otherwise.
- When a board is active, update that same board instead of creating a sibling
  progress report.
