# tradecat pattern intake — 2026-04-23

Source reviewed:
- local repo: `/Users/thrill3r/tradecat`
- files read:
  - `AGENTS.md`
  - `README.md`
  - `README_EN.md`
  - `scripts/smoke_query_service.sh`

## Verdict

Worth absorbing as service-boundary, single-read-outlet, and operator-runbook patterns.

Not worth absorbing as a direct architectural template.

`ict-engine` should learn from TradeCat's:

- explicit layered service responsibilities
- “consumption must not read DB directly” rule
- single source of truth for runtime config
- smoke-checkable query/read surface

But it should reject:

- service sprawl as a default goal
- turning every research concern into a microservice
- importing Telegram/public-service product packaging into the core engine identity

## What the repo does well

### 1. It states the read boundary mechanically

The strongest pattern in TradeCat is this rule:

- consumption does not read the database directly
- Query Service is the unique read outlet

That is a genuinely useful pattern for `ict-engine`, even though `ict-engine` is not a service mesh.

The transferable idea is:

- define one preferred truth-reading surface
- make it explicit which downstream consumers are forbidden from bypassing it
- document that bypassing the read surface is a correctness violation, not just a style violation

`ict-engine` already has an analogous candidate:

- `factor-autoresearch-status` as the preferred rollup
- canonical JSON as the underlying source of truth
- derived surfaces as convenience-only readers

TradeCat is a good reminder that this relationship should be written as a hard rule.

### 2. It turns architecture into an operator map

TradeCat's README does a good job showing:

- ingestion
- compute
- query
- consumption

as a mechanical system, not just a directory tree.

That is useful for `ict-engine`.

The repo already has many surfaces:

- analyze
- factor-research
- factor-autoresearch
- workflow-status
- artifact ledger
- state dirs

but fewer diagrams or service-style maps explaining how information flows from one layer into another.

The best transferable lesson is not “add more services.”
It is “draw the truth path and the read path separately.”

### 3. It names configuration truth clearly

TradeCat is explicit that runtime reads one config file as the single source of truth.

This is valuable documentation discipline.

For `ict-engine`, the exact config model is smaller, but the pattern transfers:

- say which file/env/flag is authoritative
- say which knobs are examples vs runtime truth
- say what must be updated together when ports/paths/defaults change

This is especially relevant around:

- `ICT_ENGINE_STATE_DIR`
- explicit `--state-dir`
- docs that mention example state paths

### 4. It ships a smoke check for the read outlet

`scripts/smoke_query_service.sh` is a valuable pattern because it tests:

- base URL
- auth mode
- auth-required behavior
- capabilities endpoint

without dumping secrets.

`ict-engine` can benefit from this style of operator documentation even without a networked query service.

The closest translation would be smoke-style docs/scripts for:

- current workflow truth surface
- current autoresearch truth surface
- current artifact ledger truth surface

The important idea is:

- verify the preferred read outlet directly
- verify auth/guard semantics where relevant
- make the success/failure contract easy for an operator to understand

### 5. It writes “must not do” rules in the architecture docs

TradeCat does not just describe layers.
It also states forbidden crossings.

That is exactly the kind of rigor `ict-engine` benefits from in docs.

Examples that parallel well:

- do not treat derived TSV/Markdown as canonical state
- do not use shared state for fair comparison experiments
- do not infer completed status without the right canonical artifact combination

TradeCat's writing style reinforces that negative rules are part of the architecture, not a footnote.

## What must not be copied into ict-engine

### 1. Do not turn the repo into a service garden

TradeCat is service-oriented because that fits its public-service/product shape.

`ict-engine` should not interpret this as a reason to split every concern into a service.

The right transfer is:

- boundary writing
- read-outlet discipline
- runbook quality

not:

- more daemons
- more ports
- more internal HTTP by default

### 2. Do not replace local typed state with remote-query indirection

TradeCat's Query Service exists because multiple consumption services need a unified read outlet.

`ict-engine` already has local canonical state files and CLI surfaces.

So the lesson is not “add an HTTP query layer.”
The lesson is “be explicit about the preferred read contract.”

In `ict-engine`, that preferred contract can remain:

- canonical JSON files
- typed rollup CLI surfaces

without adding service indirection.

### 3. Do not import public bot/application surface as core scope

TradeCat's Telegram/public dashboard/service packaging is product-specific.

`ict-engine` should not absorb:

- public-facing bot service sprawl
- many always-on consumption daemons
- API-service-first product identity

unless the repo scope changes explicitly.

## Best-fit migration targets inside ict-engine

### 1. Truth-surface docs

Best fit:

- `docs/autoresearch-derived-surfaces-contract.md`
- `docs/research-system-map.md`
- future workflow-truth docs

Pattern to absorb:

- preferred read outlet
- canonical backing truth
- forbidden bypasses
- exact read order

### 2. Architecture map docs

Best fit:

- `docs/architecture-boundaries.md`
- future system or flow diagrams

Pattern to absorb:

- ingestion/compute/reporting/state distinctions
- clear data-flow arrows
- negative boundary rules

### 3. Smoke/runbook docs

Best fit:

- `docs/first-run.md`
- `docs/smoke-acceptance.md`
- future operator checklists

Pattern to absorb:

- fast read-surface verification
- auth/guard semantics where applicable
- secret-safe diagnostics

## Concrete recommendations for ict-engine docs

### Recommendation 1

Keep reinforcing that `factor-autoresearch-status` is the preferred read outlet for session truth, while canonical JSON remains the underlying authority.

TradeCat is a strong example of why the preferred read outlet should be documented as a rule.

### Recommendation 2

Write at least one future flow-style doc or diagram for:

- command execution
- canonical artifact writes
- derived surface refresh
- operator read path

This is the `ict-engine` analogue of TradeCat's ingestion -> compute -> query -> consumption map.

### Recommendation 3

Where `ict-engine` already has a “must not” rule, keep writing it as an architecture rule instead of a soft suggestion.

Examples:

- no shared state for fair comparison
- no derived surface as source of truth
- no completed inference without the right canonical closure

## Net effect on ict-engine

The right absorption is:

- **yes** to single preferred read-outlet documentation
- **yes** to stronger negative boundary rules
- **yes** to flow-map style architecture docs
- **yes** to smoke-check style operator runbooks
- **no** to microservice sprawl as a default answer
- **no** to replacing local typed truth with unnecessary HTTP indirection

## One-sentence takeaway

TradeCat is useful to `ict-engine` because it shows how much clarity you gain when the repo says, in plain language, which surface is the only approved read outlet, which layers may not bypass it, and how an operator should verify that truth path end to end.
