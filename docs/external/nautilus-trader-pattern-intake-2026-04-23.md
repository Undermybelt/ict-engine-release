# nautilus-trader pattern intake — 2026-04-23

Source reviewed:
- local clone: `/Users/thrill3r/nautilus_trader`
- files read:
  - `README.md`
  - `ADAPTERS.md`
  - `docs/concepts/events.md`
  - `docs/getting_started/backtest_low_level.py`

## Verdict

Worth absorbing as engine-boundary, adapter-governance, and execution-event semantics patterns.

Not worth absorbing as a direct product target.

`ict-engine` should learn from NautilusTrader's:

- explicit engine decomposition
- research/simulation/live boundary writing
- adapter tier governance
- event and position lifecycle semantics

But it should reject:

- becoming a live multi-venue trading runtime
- adopting a full adapter ecosystem as a core roadmap obligation
- importing portfolio/order-management complexity before the current research surfaces stabilize

## What the repo does well

### 1. It names the system boundary clearly

NautilusTrader is unusually explicit about what it is:

- a Rust-native trading engine
- event-driven
- spanning research, deterministic simulation, and live execution
- with adapters as first-class integration boundaries

That clarity matters.

`ict-engine` is not trying to be the same product, but it benefits from the same style of writing:

- what belongs in the core engine
- what belongs in adapters
- what belongs in reporting/application layers
- what is intentionally out of scope

This is already partially visible in `docs/architecture-boundaries.md`; NautilusTrader is a strong external example of doing this consistently.

### 2. It decomposes runtime responsibilities into named engines

The most useful conceptual transfer is the way NautilusTrader separates concerns:

- data engine
- execution engine
- risk engine
- portfolio/account tracking
- adapters as translation boundaries

`ict-engine` should not clone this architecture wholesale, but the decomposition principle is valuable.

For `ict-engine`, the closest analogue is:

- data intake / cleaned dataset boundary
- factor research / scoring boundary
- execution-quality / gate boundary
- state/artifact boundary
- reporting / workflow boundary

The lesson is not “add more engines.”
The lesson is “name the runtime responsibility and keep the semantics local.”

### 3. It documents event semantics instead of burying them in code

The `events.md` material is particularly valuable.

It explains, step by step:

- what event arrived
- which engine processed it
- which cache/state objects changed
- which downstream events were emitted

This is a very strong pattern for `ict-engine`.

Right now `ict-engine` has many artifacts and derived surfaces, but fewer written step-by-step state-transition contracts.

This suggests a good documentation direction:

- when a research run finishes, which files update
- when an autoresearch attempt is kept or discarded, which canonical and derived artifacts change
- when a workflow surface says `completed` or `interrupted`, which state combination caused that classification

NautilusTrader shows that this kind of lifecycle writing pays off.

### 4. It treats adapters as governed product surfaces

`ADAPTERS.md` is useful not because `ict-engine` needs dozens of integrations, but because it defines:

- official vs community vs external
- adoption path
- support expectations
- demotion/maintenance logic

This is directly relevant if `ict-engine` ever grows external data-source or execution-quality adapters.

Even before code changes, the documentation pattern is good:

- write who owns an integration
- write what support level it has
- write what must be true before it becomes “core”

That avoids silent sprawl.

### 5. It makes repeated-run ergonomics explicit

The backtest docs clearly say:

- configure the engine
- add venue
- add data
- add strategy
- add execution algorithm
- run
- generate reports
- reset

This is strong operator-facing writing.

For `ict-engine`, the equivalent opportunity is not to mimic `BacktestEngine`, but to make the repeated experiment loop similarly mechanical:

- choose isolated or shared state
- add data inputs
- run research or autoresearch
- read status
- inspect canonical truth
- inspect derived recap
- reset or fork state intentionally

## What must not be copied into ict-engine

### 1. Do not let “research-to-live parity” expand the repo scope prematurely

NautilusTrader's core promise is research/simulation/live parity.

That is central to its product identity.

For `ict-engine`, copying that promise too early would be a mistake.

Why:

- the repo is currently strongest as a research/evidence engine
- live-ordering semantics are far more demanding than current scope
- adapter, execution, account, and risk surfaces would explode complexity fast

So the transferable pattern is boundary discipline, not the end-state product promise.

### 2. Do not import full OMS/portfolio complexity by imitation

NautilusTrader has rich position and order lifecycle semantics because it is a real execution engine.

`ict-engine` should only absorb what improves its current truth surfaces:

- typed artifacts
- explicit state transitions
- clear event causality

It should not prematurely add:

- full order state machines
- account update subsystems
- broker-grade routing semantics

unless the product scope explicitly changes.

### 3. Do not turn adapter breadth into a vanity metric

NautilusTrader has a broad adapter matrix because adapter breadth is part of its value proposition.

For `ict-engine`, many-source breadth is only useful if it improves:

- research evidence quality
- replay quality
- paired-market integrity
- deterministic analysis

So the right lesson is:

- govern adapters tightly
- prefer quality and boundary clarity over count

not “list more exchanges.”

## Best-fit migration targets inside ict-engine

### 1. Artifact and workflow transition docs

Best fit:

- `docs/research-system-map.md`
- `docs/autoresearch-derived-surfaces-contract.md`
- future workflow-state or artifact-lifecycle docs

Pattern to absorb:

- write state transitions as ordered steps
- say which artifact updates first
- say which downstream surface is derived
- say what event/state combination justifies a status label

### 2. Boundary governance docs

Best fit:

- `docs/architecture-boundaries.md`
- `docs/external/*`

Pattern to absorb:

- what belongs in core
- what belongs in adapter/integration space
- what support level each integration pattern has
- when an external idea is “learn only” vs “core-worthy”

### 3. Repeated-run operator docs

Best fit:

- `docs/first-run.md`
- `docs/state-directory-lifecycle.md`
- future backtest/research operator runbooks

Pattern to absorb:

- run sequence
- post-run read sequence
- reset/reuse semantics
- repeated-run guardrails

## Concrete recommendations for ict-engine docs

### Recommendation 1

Write more event-style transition docs for current truth surfaces.

Example topics:

- how `factor-autoresearch` updates attempts, sessions, live snapshot, final summary, TSV, and retrospective
- how `workflow-status` decides blocked vs actionable vs completed
- how artifact lineage should be read after a run

This is the strongest direct documentation lesson from NautilusTrader.

### Recommendation 2

If `ict-engine` adds more integrations, document them with explicit support tiers.

Suggested minimal categories:

- core-supported
- experimental/local
- external reference only

That keeps repo promises honest.

### Recommendation 3

Keep emphasizing deterministic research semantics over live-trading ambition.

The good transfer from NautilusTrader is:

- deterministic processing mindset
- explicit lifecycle semantics
- typed boundaries

Not:

- full live brokerage ambition

## Net effect on ict-engine

The right absorption is:

- **yes** to stronger engine and artifact boundary writing
- **yes** to state-transition documentation
- **yes** to adapter/support-tier governance
- **yes** to more explicit repeated-run semantics
- **no** to premature live-trading scope expansion
- **no** to importing OMS/portfolio complexity just because another engine has it

## One-sentence takeaway

NautilusTrader is useful to `ict-engine` less as a feature template and more as a documentation standard for how to name runtime boundaries, explain event causality, and keep integration sprawl under governance.
