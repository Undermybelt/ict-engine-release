# External Patterns Synthesis — 2026-04-23

Goal: consolidate the recent external pattern intake docs into one execution-oriented decision surface for `ict-engine`.

Reviewed intake docs:

- `docs/external/autoresearch-pattern-intake-2026-04-23.md`
- `docs/external/nautilus-trader-pattern-intake-2026-04-23.md`
- `docs/external/freqtrade-pattern-intake-2026-04-23.md`
- `docs/external/tradecat-pattern-intake-2026-04-23.md`

This document is not another intake note.
It is the repo-level answer to:

- which patterns should be adopted now
- which patterns should be adapted later
- which patterns should be explicitly rejected
- where each adopted pattern belongs in the current `ict-engine` docs

## Executive summary

Across all four external repos, the strongest common signal is not “add more features.”

It is:

1. make the preferred truth-reading surface explicit
2. keep canonical truth separate from convenience surfaces
3. write experiment and state transitions as ordered operator rules
4. document mode/state isolation as a correctness boundary
5. explain metrics and warnings in practical operator language

The strongest anti-signal is also consistent:

- do not expand `ict-engine` into a live-trading platform by imitation
- do not let git history, TSV files, or Markdown recaps become canonical truth
- do not solve clarity problems by adding service or config sprawl

## Decision matrix

| Pattern | Source(s) | Decision | Why |
|---|---|---|---|
| one preferred truth-reading surface | TradeCat, autoresearch | **adopt now** | already fits `factor-autoresearch-status` + canonical JSON model |
| canonical truth vs derived convenience split | autoresearch, TradeCat | **adopt now** | directly aligned with `attempts/sessions/live/final` vs `status/tsv/retrospective` |
| one-attempt/one-row human ledger | autoresearch | **adopt now** | already matches `experiments.tsv` direction |
| concise retrospective/operator recap | autoresearch | **adopt now** | fits new retrospective surface |
| state isolation as correctness rule | Freqtrade | **adopt now** | directly relevant to isolated vs shared state dirs |
| explicit config/override precedence docs | Freqtrade | **adopt now** | needed around env/default/flag overlaps |
| event-style transition writing | NautilusTrader | **adopt now** | strongest missing documentation pattern in current repo |
| adapter/support-tier governance | NautilusTrader | **adapt later** | useful if integration count grows, but not urgent for current core |
| smoke-check style truth-surface verification | TradeCat | **adapt later** | valuable, but secondary to writing the contracts first |
| fixed wall-clock research budget | autoresearch | **adapt later** | concept transfers, literal mechanism does not |
| risk/expectancy worked-example docs | Freqtrade | **adapt later** | useful for scoring/gating docs after current truth docs settle |
| research/live parity as product promise | NautilusTrader | **reject** | wrong scope for current repo |
| git-reset/commit-hash experiment truth | autoresearch | **reject** | conflicts with canonical JSON state model |
| TSV-first or Markdown-first persistence | autoresearch, TradeCat | **reject** | derived surfaces must stay derived |
| live bot shell / Telegram-first control model | Freqtrade, TradeCat | **reject** | wrong default repo identity |
| microservice sprawl as default solution | TradeCat | **reject** | would add complexity without solving current core issues |

## Patterns to adopt now

### 1. Preferred truth-reading surface

Adopted rule:

- for autoresearch session truth, start with `factor-autoresearch-status`
- if exact authority is needed, fall back to canonical JSON artifacts
- derived TSV/Markdown surfaces are never the first authority

Current best landing spots:

- `docs/autoresearch-derived-surfaces-contract.md`
- `docs/research-system-map.md`
- `README.md`

Why now:

- the repo already has the surfaces
- ambiguity now is mostly documentation ambiguity, not missing implementation

### 2. Canonical vs derived split

Adopted rule:

- canonical JSON artifacts define truth
- status/TSV/retrospective summarize or reformat that truth
- disagreement resolves in favor of canonical JSON

Current best landing spots:

- `docs/autoresearch-derived-surfaces-contract.md`
- `docs/research-system-map.md`

Why now:

- this boundary is already active in current work
- leaving it implicit invites drift immediately

### 3. One-attempt/one-row ledger discipline

Adopted rule:

- each autoresearch attempt should compress to one operator-facing ledger row
- status labels stay mechanically simple
- short free-text notes are allowed, but not as canonical truth

Current best landing spots:

- `docs/autoresearch-derived-surfaces-contract.md`
- future `experiments.tsv` operator docs

Why now:

- this is the cleanest part of the `autoresearch` transfer

### 4. State isolation as a correctness rule

Adopted rule:

- isolated state for fair comparison
- shared state only for intentional cumulative loops
- reusing a state dir changes the meaning of later results

Current best landing spots:

- `docs/state-directory-lifecycle.md`
- `docs/research-system-map.md`
- `docs/first-run.md`

Why now:

- current repo already depends on this distinction heavily
- the repo has enough state dirs that ambiguity is costly

### 5. Event-style transition writing

Adopted rule:

- document important state transitions as ordered steps
- say what updates first
- say which later surfaces are derived from that update

Current best landing spots:

- future doc: `docs/autoresearch-state-transitions.md`
- `docs/research-system-map.md`
- future workflow/lineage docs

Why now:

- this is the biggest documentation gap revealed by the external reviews

## Patterns to adapt later

### 1. Smoke-style truth verification

Keep as a near-future candidate, not immediate priority.

Good eventual shapes:

- a small smoke checklist for autoresearch truth
- a small smoke checklist for workflow-status truth
- a small smoke checklist for artifact ledger truth

This should come after the written contracts are stable.

### 2. Config precedence matrix

The principle should be adopted now, but a full matrix can wait until the repo has a slightly larger explicit config surface.

Good eventual topics:

- `--state-dir` vs `ICT_ENGINE_STATE_DIR`
- explicit output mode vs shorthand flags
- user dataset selection vs previously recorded paths

### 3. Risk/expectancy explanation style

This should be introduced selectively where metrics are already stable.

Good targets:

- `docs/objective-scoring-map.md`
- future gating-quality docs

But avoid adding pseudo-precision to unstable metrics.

### 4. Adapter/support-tier governance

Useful if `ict-engine` grows more external data/source integrations.

Not urgent for the current repo core, but worth keeping as a documented future pattern.

## Patterns to reject explicitly

### 1. Git as experiment truth

Rejected because:

- repo-local state artifacts already exist
- git history should not define keep/discard truth
- the source repo already has artifact-history pressure

### 2. Derived file as authoritative state

Rejected because:

- TSV and Markdown are lossy and convenience-oriented
- they are easy to read but wrong as the primary truth surface

### 3. Live-trading scope creep

Rejected because:

- current repo strength is research/evidence/orchestration
- live execution semantics would radically widen the problem

### 4. Service-first expansion

Rejected because:

- current clarity problems are mostly truth-surface and contract problems
- services would add deployment and boundary burden before solving core ambiguity

## Recommended document changes next

These are documentation-only next steps, ordered by leverage.

### P0

Create:

- `docs/autoresearch-state-transitions.md`

Purpose:

- describe, step by step, what happens during an autoresearch iteration
- describe which files update on attempt append
- describe what changes on session completion
- describe where derived surfaces are refreshed from

Why:

- this is the highest-value pattern imported from NautilusTrader + TradeCat + autoresearch combined

### P0

Update:

- `docs/state-directory-lifecycle.md`

Add:

- a short “isolated vs shared state” decision table
- examples of bad mixed-state comparisons
- explicit note that reused state changes experiment semantics

Why:

- strongest immediate transfer from Freqtrade

### P1

Update:

- `docs/first-run.md`
- `README.md`

Add:

- a compact precedence note for `--state-dir` and `ICT_ENGINE_STATE_DIR`
- a compact note that derived surfaces never outrank canonical JSON

### P1

Update:

- `docs/objective-scoring-map.md`

Add:

- a worked-example style explanation for one or two key metrics
- a misuse warning section

Why:

- strongest transferable part of Freqtrade's risk/expectancy writing style

## Adopt / adapt / reject checklist

### Adopt now

- preferred truth-reading surface
- canonical vs derived split
- one-attempt/one-row operator ledger
- concise retrospective/operator recap
- state isolation as a correctness rule
- event-style transition docs

### Adapt later

- smoke-style read-surface verification
- fuller config precedence matrix
- risk/expectancy worked-example docs
- adapter/support-tier governance

### Reject

- git-driven experiment truth
- TSV-first or Markdown-first persistence
- live-trading scope creep
- service sprawl as the default answer

## One-sentence synthesis

The combined external signal is that `ict-engine` should become more explicit, not more complex: clearer truth outlets, clearer state contracts, clearer experiment semantics, and stronger operator-facing documentation around what counts as real evidence versus convenient recap.
