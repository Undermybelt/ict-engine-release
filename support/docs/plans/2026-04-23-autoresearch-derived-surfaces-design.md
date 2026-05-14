# Autoresearch Derived Surfaces Design

> Goal: add Auto-Quant-inspired human-readable derived surfaces on top of existing `factor-autoresearch` truth without changing source-of-truth semantics.

## Why this design exists

Current repo already has the hard part:
- `factor-autoresearch` loop
- session / attempt / live / final JSON persistence
- `factor-autoresearch-status` aggregation logic

What is still missing is the human-readable layer:
- a ledger you can `tail`
- a retrospective you can read without opening raw JSON
- a warning-only integrity layer for suspicious uplifts

This design intentionally treats those as **derived surfaces**, not a second execution system.

## Scope

### In scope

1. `experiments.tsv`
2. `factor_autoresearch_retrospective.md`
3. warning-only suspicious uplift detection
4. extraction of current autoresearch status aggregation out of `main.rs`

### Out of scope

- git / worktree orchestration
- auto rollback
- commit-hash-first experiment management
- replacing `factor_autoresearch_attempts.json`
- introducing a second optimizer or second mutation engine

## Existing anchors

### Command and persistence truth

- `src/main.rs`
  - `factor_autoresearch_command(...)`
  - `factor_autoresearch_status_command(...)`
- `src/state/persistence.rs`
  - `append_factor_autoresearch_attempt(...)`
  - `load_factor_autoresearch_attempts(...)`
  - `load_factor_autoresearch_sessions(...)`
  - `save_factor_autoresearch_live_snapshot(...)`
  - `save_factor_autoresearch_final_summary(...)`
- `src/state/types.rs`
  - `FactorAutoresearchAttempt`
  - `FactorAutoresearchSession`
  - `FactorAutoresearchLiveSnapshot`
  - `FactorAutoresearchSummary`
- `src/application/release_closure/mod.rs`
  - already contains contamination / monotonicity heuristics that can inform warning design
- `src/application/factor_lifecycle/lifecycle_surface.rs`
  - shows existing pattern for mutation-derived lifecycle surfaces outside `main.rs`
- `src/application/reflection/research_adapter.rs`
  - shows existing pattern for building derived reflection artifacts from stable report objects

### Newly verified Auto-Quant upstream facts

From the cloned `~/Auto-Quant` repo:

#### v0.1.0 artifacts (already documented)

- `README.md` — positions `results.tsv` as the primary interpretable output
- `program.md` — defines `results.tsv` schema, `note` field as load-bearing, Goodhart watch section
- `.gitignore` — `results.tsv` untracked, survives `git reset --hard`
- `analysis.ipynb` — TSV as downstream analysis surface
- `versions/0.1.0/retrospective.md` — research arc, aha moments, behavior observations, limitations

#### v0.2.0 new findings (pulled 2026-04-23)

From `~/Auto-Quant` pull — v0.2.0 introduced multi-strategy architecture and produced substantial new evidence:

- **`versions/0.2.0/retrospective.md`**
  - 81 rounds, 209 events, 5 paradigms tested, 3 with positive edge
  - Clean-edge Sharpe 0.67 vs v0.1.0's true-edge 0.19 (3.5× improvement)
  - Zero Goodhart signatures in 81 rounds (vs v0.1.0's 3 exploits)
  - Seven distinct behavioral observations with evidence
  - Cross-paradigm findings that single-strategy runs cannot produce

- **`versions/0.2.0/meta.yaml`**
  - Structured `aha_moments` with round, type, commit, summary
  - `comparative_findings` — cross-paradigm knowledge with evidence commits
  - `behavioral_observations` — zero forks, zero Goodhart, three kill types
  - `killed_strategies` with `kill_reason` taxonomy

- **Three kill types discovered** (directly relevant to ict-engine's decision taxonomy):
  1. **Opportunity-cost kill** — positive edge but thin sample, slot reallocated
  2. **Paradigm-swap-with-revert** — kill working strategy to test radical alternative, revert on failure, extract "existing design validated"
  3. **One-round failure** — new paradigm fails immediately, kill without multi-round tuning

- **Cross-paradigm comparison as gaming detection** (relevant to warning design):
  - If only one strategy's Sharpe jumps while others stay flat → likely real discovery
  - If ALL strategies jump on same commit → suspect shared cause or oracle hole
  - This is the multi-strategy analogue of ict-engine's cluster comparison

- **Agent self-correction patterns** (relevant to retrospective design):
  - Plateau retraction: agent declared "MR capped at 0.29", later broke to 0.45 and retracted
  - Null-result replication: agent tested ADX on MACD *knowing* it failed on TrendEMA, confirmed generalization
  - Epistemic humility: revise earlier assertions when new evidence appears

- **Fork operation never used** in 81 rounds:
  - Cap+kill design was expressive enough
  - Fork only rational when agent wants hedge (preserve known-good while testing risky variant)
  - Suggests ict-engine's `fork` event type may be rarely used in practice

- **`program.md` v0.2.0 Goodhart watch specifics**:
  - `exit_profit_only=True` → 100% win rate by never realizing losses (regime-dependent)
  - Tight `minimal_roi` clipping → tiny uniform returns → low stddev → huge Sharpe (profit goes DOWN as Sharpe goes UP)
  - Multi-strategy comparison makes gaming visible: "if ALL three strategies' Sharpe jumped on the same commit — you probably modified something shared, or the oracle itself has a hole"

## Core design decisions

### 1. JSON truth remains canonical

The following remain the only source-of-truth artifacts:
- `factor_autoresearch_attempts.json`
- `factor_autoresearch_sessions.json`
- `factor_autoresearch_live.json`
- `factor_autoresearch_final.json`

`experiments.tsv` and `factor_autoresearch_retrospective.md` are regenerated from those files.

### 2. Derived artifacts are rewritten, not incrementally appended

Do **not** append to `experiments.tsv` directly as the primary write path.

Reason:
- JSON attempt append already exists and is canonical
- if JSON write succeeds but TSV append fails, drift appears immediately
- rewriting the full TSV from attempts history is mechanically safer and still cheap at expected session sizes

This is a deliberate divergence from Auto-Quant's append-only `results.tsv` flow.
Auto-Quant needs that journal to survive `git reset --hard` because git history is part of its runtime.
`ict-engine` already persists canonical JSON state outside git-reset semantics, so full regeneration is safer here.

### 3. Suspicious uplift is warning-only in v1

Warnings should:
- never change keep / discard
- never modify resume semantics
- be traceable to specific attempt ids and metrics

This is also a deliberate divergence from Auto-Quant v0.1.0 behavior, where the agent could retroactively discard prior keeps after recognizing gaming.
For `ict-engine`, v1 should first surface the signal clearly before introducing any retrospective policy that mutates canonical decisions.

### 4. First extract status logic, then build ledger / retrospective on top

Today the most reusable aggregation logic is trapped inside `factor_autoresearch_status_command(...)` in `main.rs`.
That should be extracted before adding more surfaces.

### 5. Absorb the event-log idea, not the exact event vocabulary

Auto-Quant's TSV schema is:
- `commit`
- `event`
- `strategy_name`
- `sharpe`
- `max_dd`
- `note`

That exact schema should **not** be copied into `ict-engine` because the underlying loop is different:
- Auto-Quant mutates a small active set of named strategies inside git
- `ict-engine` mutates one candidate spec at a time against a session baseline
- `ict-engine` already has structured decision / evaluation objects that are richer than a 6-column free-text ledger

What we should absorb is:
- one derived row per meaningful research event
- explicit human-readable reasoning column(s)
- a ledger shape that is easy to `tail`, diff, and meta-analyze

### 6. Retrospective should preserve research behavior, not just metrics

The cloned `versions/0.1.0/retrospective.md` makes clear that the real value of retrospective output is not only:
- best result
- counts
- warnings

It also captures:
- phase shifts in the search
- what the agent learned
- which discoveries were legitimate vs gaming
- what was never explored
- limitations of the oracle itself

`ict-engine` should preserve that spirit, but derive it from structured state instead of from narrative memory alone.

## What to absorb vs what to reject

### Absorb

- human-readable journal as a first-class artifact surface
- load-bearing reasoning text in the derived ledger
- retrospective as a research-behavior document, not a vanity metrics page
- Goodhart-watch style warning heuristics
- downstream meta-analysis surfaces that read the ledger rather than re-running experiments
- cross-cluster comparative findings (v0.2.0's key innovation — knowledge only producible via multi-cluster comparison)
- null-result replication tracking (deliberate re-tests of known failures to confirm generalization)
- self-correction / epistemic humility events (plateau retractions, assertion reversals)
- kill-reason taxonomy (opportunity-cost / paradigm-swap-with-revert / one-round-failure)

### Reject or adapt

- direct use of git commit hash as primary experiment identity
- `create | evolve | stable | fork | kill` as the event taxonomy
- "one row per active strategy per round" semantics
- retroactively mutating canonical keep/discard in v1
- dependence on free-text-only notes as the sole source of retrospective truth
- premature expansion into dedicated `--retrospective` / `--output-format tsv` style flags before the derived surfaces themselves are stable
- fork as a frequently-used operation (v0.2.0 used it zero times in 81 rounds — cap+kill was expressive enough)

## Target v1 deliverables

### A. Status surface extraction

Create a pure builder module that turns raw state into a typed autoresearch status surface.

Recommended new file:
- `src/application/factor_lifecycle/autoresearch_surface.rs`

Recommended exports:
- `build_factor_autoresearch_status_surface(...)`
- `build_factor_autoresearch_experiment_rows(...)`
- `render_factor_autoresearch_experiments_tsv(...)`
- `build_factor_autoresearch_warning_surface(...)`
- `build_factor_autoresearch_retrospective(...)`
- `render_factor_autoresearch_retrospective_markdown(...)`

Recommended update:
- `src/application/factor_lifecycle/mod.rs`

## Proposed derived types

These are **application-layer** types, not source-of-truth state types.

### `FactorAutoresearchStatusSurface`

Purpose:
- replace the ad-hoc JSON assembly currently embedded in `factor_autoresearch_status_command(...)`

Suggested fields:
- `symbol: String`
- `state_dir: String`
- `session_filter: Option<String>`
- `effective_status: String`
- `interrupted: bool`
- `final_summary_exists: bool`
- `live_snapshot: Option<FactorAutoresearchLiveSnapshot>`
- `sessions: Vec<FactorAutoresearchSession>`
- `attempts: Vec<FactorAutoresearchAttempt>`
- `decision_counts: BTreeMap<String, usize>`
- `failure_tag_counts: BTreeMap<String, usize>`
- `cluster_scoreboard: Vec<FactorAutoresearchClusterScorecardEntry>`
- `cluster_fail_streaks: BTreeMap<String, usize>`
- `best_attempt: Option<FactorAutoresearchAttempt>`

### `FactorAutoresearchClusterScorecardEntry`

Suggested fields:
- `cluster: String`
- `attempts: usize`
- `avg_score_delta: f64`
- `best_score_delta: f64`

### `FactorAutoresearchExperimentRow`

Purpose:
- one row in `experiments.tsv`

This is the `ict-engine` analogue of Auto-Quant's `results.tsv`, but mapped to mutation attempts rather than strategy lifecycle events.

Suggested fields:
- `timestamp: String`
- `session_id: String`
- `attempt_id: String`
- `attempt_status: String`
- `base_factor: String`
- `mutation_id: String`
- `decision_status: String`
- `score_before: f64`
- `score_after: f64`
- `score_delta: f64`
- `aggregate_return_after: f64`
- `aggregate_return_before: Option<f64>`
- `top_factor: String`
- `failure_reason: String`
- `recommended_directions: String`
- `hypothesis: String`
- `failure_tags: String`
- `branch_summary: String`
- `note: String`

### `FactorAutoresearchDerivedWarning`

Purpose:
- reusable warning object for status / retrospective / future UI

Suggested fields:
- `attempt_id: String`
- `session_id: String`
- `code: String`
- `severity: String`
- `summary: String`
- `evidence: Vec<String>`

### `FactorAutoresearchRetrospective`

Purpose:
- typed intermediate before Markdown rendering
- directly informed by Auto-Quant v0.2.0's retrospective structure

Suggested fields:
- `symbol: String`
- `session_id: Option<String>`
- `effective_status: String`
- `interrupted: bool`
- `attempts_total: usize`
- `kept_attempts: usize`
- `discarded_attempts: usize`
- `best_attempt_id: Option<String>`
- `best_score_delta: Option<f64>`
- `score_trajectory: Vec<(String, f64)>`
- `top_failure_tags: Vec<(String, usize)>`
- `cluster_scoreboard: Vec<FactorAutoresearchClusterScorecardEntry>`
- `derived_warnings: Vec<FactorAutoresearchDerivedWarning>`
- `decision_counts: BTreeMap<String, usize>` — accepted/rejected/failed distribution
- `cluster_comparative_findings: Vec<FactorAutoresearchClusterComparison>` — cross-cluster knowledge (v0.2.0's key innovation)
- `aha_attempts: Vec<FactorAutoresearchRetrospectiveHighlight>`
- `most_valuable_attempt: Option<FactorAutoresearchRetrospectiveHighlight>`
- `null_result_replications: Vec<FactorAutoresearchNullResult>` — when agent deliberately re-tests known-failure to confirm generalization
- `self_corrections: Vec<FactorAutoresearchSelfCorrection>` — plateau retractions, earlier-assertion reversals
- `research_arc: Vec<FactorAutoresearchRetrospectivePhase>`
- `behavior_observations: Vec<String>`
- `unexplored_directions: Vec<String>`
- `oracle_limitations: Vec<String>`
- `evaluation_summary: Vec<String>`
- `open_questions: Vec<String>`
- `recommended_next_focus: Vec<String>`

### `FactorAutoresearchClusterComparison` (NEW — from v0.2.0)

Purpose:
- capture cross-cluster knowledge that single-cluster runs cannot produce
- direct analogue of Auto-Quant's `comparative_findings` in `meta.yaml`

Suggested fields:
- `title: String` — e.g. "Volume expansion filter generalizes across all clusters"
- `evidence_attempt_ids: Vec<String>` — attempts that support this finding
- `detail: String` — why this is cross-cluster, not cluster-specific
- `confidence: String` — "replicated", "single-observation", "hypothesis"

### `FactorAutoresearchNullResult` (NEW — from v0.2.0)

Purpose:
- capture deliberate null-result replications
- v0.2.0 example: agent tested ADX on MACD *knowing* it failed on TrendEMA, confirmed generalization

Suggested fields:
- `attempt_id: String`
- `original_failure_attempt_id: String` — the earlier failure being re-tested
- `hypothesis: String` — what the agent expected
- `outcome: String` — confirmed-generalization / contradicted / inconclusive
- `note: String` — agent's reasoning

### `FactorAutoresearchSelfCorrection` (NEW — from v0.2.0)

Purpose:
- capture epistemic humility events
- v0.2.0 example: "MR appears capped at 0.29" → later broke to 0.45, agent retracted

Suggested fields:
- `original_attempt_id: String` — where the original claim was made
- `correcting_attempt_id: String` — where the claim was retracted
- `original_claim: String` — e.g. "MR capped at ~0.29 sharpe"
- `correction: String` — e.g. "MR NOT actually capped — earlier rounds couldn't find the right param"
- `evidence: String` — what changed (e.g. "BB period 20→15, Sharpe 0.29→0.45")

### `FactorAutoresearchRetrospectivePhase`

Purpose:
- summarize one stage in the session's research arc

Suggested fields:
- `label: String`
- `attempt_range: String`
- `character: String`
- `score_trajectory: String`

### `FactorAutoresearchRetrospectiveHighlight`

Purpose:
- pin one attempt worth surfacing in the Markdown narrative

Suggested fields:
- `attempt_id: String`
- `title: String`
- `why_it_matters: String`
- `evidence: Vec<String>`

## State and filename additions

Add constants to `src/state/types.rs`:
- `FACTOR_AUTORESEARCH_EXPERIMENTS_FILE: &str = "experiments.tsv";`
- `FACTOR_AUTORESEARCH_RETROSPECTIVE_FILE: &str = "factor_autoresearch_retrospective.md";`

Do **not** add `ExperimentRow` or `Retrospective` structs to `src/state/types.rs`.
Those are derived surfaces and should live in application-layer modules.

## Persistence helper additions

Add text-artifact helpers to `src/state/persistence.rs`:
- `save_text_state(...)`
- optional: `load_text_state(...)`

Recommended shape:
- mirror `save_state(...)`
- reuse symbol-scoped directory layout
- create parent directory if missing
- write plain text exactly once per sync

This keeps text artifact writing out of `main.rs` and avoids ad-hoc `std::fs::write` calls scattered through command handlers.

## Detailed design

### A. Extract status builder out of `main.rs`

#### New pure builder

Move the aggregation logic currently in `factor_autoresearch_status_command(...)` into:
- `build_factor_autoresearch_status_surface(symbol, state_dir, session_id, latest_only, limit)`

Responsibilities:
- load sessions / attempts / live snapshot
- compute `effective_status`
- compute `interrupted`
- compute `decision_counts`
- compute `failure_tag_counts`
- compute `cluster_scoreboard`
- compute `cluster_fail_streaks`
- select `best_attempt`

`factor_autoresearch_status_command(...)` should become a thin wrapper:
1. call builder
2. serialize to JSON
3. print

#### Why this is first

Ledger, retrospective, and warnings all need the same normalized aggregation layer.
Without extraction, every new surface would either:
- duplicate `main.rs` logic
- or keep reaching back into untyped JSON assembly

### B. `experiments.tsv`

#### Row mapping rules

Each attempt maps to exactly one row.

This keeps the ledger aligned with `ict-engine`'s real execution grain.
Unlike Auto-Quant, there is no concept of several concurrently active strategy files each needing a row for the same round.

Field mapping:
- `timestamp` <- `attempt.timestamp`
- `session_id` <- `attempt.session_id`
- `attempt_id` <- `attempt.attempt_id`
- `attempt_status` <- session-relative state such as `accepted_candidate` / `rejected_candidate`, or simpler v1 alias of `decision.status`
- `base_factor` <- `attempt.base_factor`
- `mutation_id` <- `attempt.candidate_mutation_spec.mutation_id`
- `decision_status` <- `attempt.decision.status`
- `score_before` <- `attempt.decision.baseline_score_before`
- `score_after` <- `attempt.decision.candidate_score`
- `score_delta` <- `attempt.decision.score_delta`
- `aggregate_return_before` <- `attempt.evaluation.metrics_before.map(|m| m.aggregate_return)`
- `aggregate_return_after` <- `attempt.evaluation.metrics_after.aggregate_return`
- `top_factor` <- first item of `metrics_after.top_factor_names`, fallback `unknown`
- `failure_reason` <- `attempt.evaluation.reason`
- `recommended_directions` <- `attempt.evaluation.recommended_mutation_directions.join("|")`
- `hypothesis` <- `attempt.candidate_mutation_spec.hypothesis`
- `failure_tags` <- `attempt.evaluation.failure_tags.join("|")`
- `branch_summary` <- sanitized `attempt.branch_summary.join(" | ")`
- `note` <- derived human-readable synthesis from `hypothesis + reason + branch_summary + recommended_directions`

`note` matters because Auto-Quant's upstream design proved that later retrospective quality depends heavily on having concise, load-bearing textual rationale at the ledger row level.

#### Render rules

- fixed header row
- UTF-8 text
- tab-separated
- replace embedded tabs and newlines in textual fields with spaces
- preserve row order as chronological attempt order within the selected session/history
- make `note` short but information-dense; do not dump full JSON blobs into one cell

#### Git semantics

Unlike Auto-Quant's `results.tsv`, `ict-engine` should not rely on git reset behavior for the ledger's survival.

However, we should still preserve the upstream's operational intent:
- TSV must be human-readable and cheap to inspect during iterative work
- TSV should be safe to keep outside canonical JSON workflows
- if later we decide to gitignore `experiments.tsv`, that should be treated as an operational choice, not as a source-of-truth decision

#### Sync strategy

Add a pure sync helper:
- `sync_factor_autoresearch_experiments_tsv(state_dir, symbol) -> Result<()>`

Algorithm:
1. load attempts via canonical JSON helper
2. build row structs
3. render TSV string
4. write to `experiments.tsv` via `save_text_state(...)`

#### Trigger points

Call sync after:
- `append_factor_autoresearch_attempt(...)`
- optionally after session completion as a final consistency pass

Do **not** call it before canonical attempt JSON is written.

### C. `factor_autoresearch_retrospective.md`

#### Recommended implementation path

Build retrospective from the extracted typed status surface.

Reason:
- it needs the same `effective_status` / `interrupted` logic as the status command
- it should reuse cluster and decision aggregation rather than recomputing them differently

#### Markdown sections

Recommended sections:
- `# Session Summary`
- `## Status`
- `## Headline`
- `## Research Arc`
- `## Decision Mix`
- `## Score Trajectory`
- `## Best Attempt`
- `## Aha Attempts`
- `## Most Valuable Attempt`
- `## Failure Tag Concentration`
- `## Cluster Scoreboard`
- `## Cross-Cluster Findings` — NEW: knowledge that only multi-cluster comparison can produce (v0.2.0's key innovation)
- `## Null-Result Replications` — NEW: deliberate re-tests of known failures to confirm generalization
- `## Self-Corrections` — NEW: plateau retractions, earlier-assertion reversals (epistemic humility events)
- `## Behavior Observations`
- `## What Was Not Explored`
- `## Oracle Limitations`
- `## Evaluation`
- `## Open Questions`
- `## Derived Warnings`
- `## Suggested Next Focus`

These sections are directly informed by the structure of Auto-Quant's v0.1.0 and v0.2.0 retrospectives, but should be populated from structured `ict-engine` state rather than handwritten prose.

#### Suggested content rules

- every major conclusion should cite counts, attempt ids, or score deltas
- no narrative claims that cannot be tied to raw state
- if `effective_status == interrupted`, say so explicitly
- if there are no attempts, emit a short empty-state markdown instead of failing
- keep the writing crisp and audit-friendly; this is a research postmortem, not marketing copy
- distinguish clearly between:
  - observed result
  - suspected mechanism
  - open hypothesis

#### Write trigger

Recommended v1 trigger:
- write retrospective on successful session completion
- allow future explicit refresh surface for stale / interrupted sessions

Why not auto-write after every attempt in v1:
- interrupted status is time-sensitive and best determined at read time
- writing on every attempt creates a stale “completed-looking” narrative risk

#### Future-compatible extension

If interrupted-session retrospectives become important, add a dedicated refresh path later:
- either a thin Rust subcommand
- or a small wrapper script once the pure builder exists

### D. warning-only suspicious uplift layer

#### Important design choice

Warnings are computed from truth and exposed as derived output.
They are **not** part of v1 canonical attempt persistence.

This avoids two problems:
- stale warnings when heuristics evolve
- unnecessary schema churn in source-of-truth JSON

#### Proposed heuristics for v1

1. `score_delta_jump`
- trigger when current `score_delta` is materially above recent session baseline
- compare against recent rolling median / max of prior attempts
- require a minimum absolute delta floor to avoid noise
- **v0.2.0 analogue**: if only one cluster's score jumps while others stay flat → likely real discovery. If ALL clusters jump on same mutation → suspect shared cause.

2. `return_mismatch`
- trigger when `score_after` improved but `metrics_before.aggregate_return` to `metrics_after.aggregate_return` did not meaningfully improve
- only evaluate when `metrics_before` exists
- this is the direct `ict-engine` analogue of Auto-Quant's "Sharpe up, profit down" Goodhart watch
- **v0.2.0 specifics**: `exit_profit_only=True` → 100% win rate but profit flat; tight `minimal_roi` → tiny uniform returns → low stddev → huge Sharpe while profit goes DOWN

3. `near_zero_drawdown_like_pattern`
- if an available downstream metric or branch summary implies unusually collapsed downside while score jumps materially, emit a warning
- if the necessary downside metric is not stably present in `FactorMutationMetricSet`, defer this heuristic instead of inventing a fake proxy
- **v0.2.0 analogue**: DD collapsing toward zero is a gaming signal (v0.1.0 observed this with `stoploss = -0.99` + `exit_profit_only`)

4. `keep_with_failure_tags`
- trigger when a kept attempt still carries strong failure tags
- initial implementation can be simple: any non-empty `failure_tags` on a kept attempt
- later refine with allowlist / denylist

5. `cluster_overconcentration`
- trigger when one `cluster_jump` bucket dominates the attempt population past a minimum sample size
- intended to catch template overfitting or narrow search collapse
- **v0.2.0 analogue**: single-strategy anchoring (v0.1.0's 99 rounds of RSI mean-rev). Multi-strategy architecture was designed specifically to prevent this.

6. `monotonic_shared_state_signal`
- reuse or adapt contamination logic already present in `build_research_verdict_report(...)`
- if deltas become suspiciously monotonic across many attempts, flag possible comparison contamination

7. `shared_surface_jump`
- if multiple nearby attempts across the same session show sudden broad uplift with similar signatures, flag potential shared-state or common-surface change
- this is the `ict-engine` analogue of Auto-Quant's upstream warning: "if all strategies jump on the same commit, suspect shared cause rather than local discovery"
- **v0.2.0 confirmed**: "if ALL three strategies' Sharpe jumped on the same commit — you probably modified something shared, or the oracle itself has a hole"

8. `plateau_retraction` (NEW — from v0.2.0 observation)
- trigger when agent declares a ceiling/floor and later breaks it
- detect via `note` field containing "capped at", "appears limited", "ceiling" followed by later attempt exceeding the claimed limit
- purpose: surface epistemic humility events for retrospective analysis, not as warnings per se
- **v0.2.0 example**: round 21 "MR appears capped at ~0.29 sharpe" → round 31 broke to 0.45, agent retracted

#### Warning computation entry point

Recommended pure helper:
- `build_factor_autoresearch_warning_surface(surface: &FactorAutoresearchStatusSurface) -> Vec<FactorAutoresearchDerivedWarning>`

#### Initial exposure surfaces

Phase 3 should expose warnings in:
- retrospective markdown
- `factor-autoresearch-status` JSON output as additive fields

Phase 3 should **not**:
- block session completion
- alter keep / discard
- auto-write rollback hints into canonical truth

## Proposed sequencing

### Phase A: status extraction

Files:
- `src/application/factor_lifecycle/mod.rs`
- new `src/application/factor_lifecycle/autoresearch_surface.rs`
- `src/main.rs`

Outcome:
- `factor_autoresearch_status_command(...)` becomes thin
- one typed status builder exists

### Phase B: ledger sync

Files:
- `src/state/types.rs`
- `src/state/persistence.rs`
- `src/application/factor_lifecycle/autoresearch_surface.rs`
- `src/main.rs`

Outcome:
- `experiments.tsv` rewrites from JSON truth after each attempt append

### Phase C: retrospective generation

Files:
- `src/state/types.rs`
- `src/state/persistence.rs`
- `src/application/factor_lifecycle/autoresearch_surface.rs`
- `src/main.rs`

Outcome:
- completed sessions emit `factor_autoresearch_retrospective.md`
- markdown is fully derived from the typed status surface
- markdown includes research arc / highlights / limitations / open questions, not just summary stats

### Phase D: warning-only integrity layer

Files:
- `src/application/factor_lifecycle/autoresearch_surface.rs`
- `src/main.rs`

Outcome:
- `factor-autoresearch-status` gets additive warning fields
- retrospective includes warning section

## Planned file touches

### Must touch

- `support/docs/plans/2026-04-23-autoresearch-derived-surfaces-design.md`
- `src/application/factor_lifecycle/mod.rs`
- new `src/application/factor_lifecycle/autoresearch_surface.rs`
- `src/state/types.rs`
- `src/state/persistence.rs`
- `src/main.rs`

### Likely touch

- `support/docs/auto-quant-integration-plan.md`
- `support/docs/external-integration-plan.md`

### Optional later touch

- `src/application/release_closure/mod.rs`
  - only if contamination heuristics are explicitly shared instead of duplicated

## Verification plan

### Unit tests

1. status extraction parity
- extracted builder matches current status semantics
- completed / running / interrupted states all covered

2. TSV rendering
- row count equals attempt count
- field mapping is correct
- tabs / newlines are sanitized

3. retrospective rendering
- completed session output
- empty session output
- interrupted session output

4. warning heuristics
- one fixture per warning code
- no warning when thresholds are not met

### Smoke checks

- `cargo fmt`
- `cargo check`
- targeted tests for new builder / renderer helpers
- smoke run of `cargo run -- factor-autoresearch-status --help`
- one short autoresearch run to confirm:
  - attempts JSON updates
  - `experiments.tsv` regenerates
  - completed session writes retrospective

## Non-goals and guardrails

- do not rename existing autoresearch JSON files
- do not force TSV consumers into the main execution path
- do not treat `experiments.tsv` as resumable state
- do not persist warning heuristics as authoritative truth in v1
- do not leave aggregation logic duplicated in both `main.rs` and a new module
- do not copy Auto-Quant's git-driven runtime assumptions into `ict-engine` where JSON state already solves persistence
- do not claim a retrospective conclusion unless it can be derived from attempts, sessions, metrics, or explicit warnings

## Recommended first implementation cut

The safest first cut is:
1. extract status builder from `main.rs`
2. add text persistence helper
3. add `experiments.tsv` full-regeneration sync
4. only after that add retrospective and warnings

This order keeps the first shipped change mechanical, observable, and easy to regression-test.
