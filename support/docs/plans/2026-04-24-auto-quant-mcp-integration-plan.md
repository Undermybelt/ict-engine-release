# Auto-Quant MCP Integration Plan

Date: 2026-04-24

References
- `support/docs/auto-quant-ictengine-integration-guide.md`
- local upstream checkout observed at `/Users/thrill3r/Auto-Quant`

Status
- implementation plan
- prepared after explicit design alignment on:
  - separate upstream repo
  - managed local clone
  - explicit bootstrap/update flow
  - MCP-mediated handoff
  - factor preservation

## Goal

Integrate `Auto-Quant` into `ict-engine` as the managed external engine for:

- factor research
- factor iteration
- historical backtest experimentation

while keeping `ict-engine` as the system of record for:

- orchestration
- state truth
- workflow closure
- keep/discard/adopt/retire decisions
- user/agent prompting

## Non-negotiable constraints

1. `Auto-Quant` remains an independent upstream repo.
2. `ict-engine` must not vendor or silently fork `Auto-Quant`.
3. First use may auto-bootstrap the dependency.
4. Runtime must not silently drift to a new upstream version.
5. Existing `ict-engine` factors must not be deleted or silently overwritten.
6. Public/operator surfaces must not assume a maintainer-local Tomac layout.
7. `ict-engine` owns the final closure decision, not `Auto-Quant`.

## Stage 1 — Strategic design

### Core domain / supporting / generic

| Type | Domain | Why |
|---|---|---|
| Core | `ict-engine` workflow orchestration | It owns the canonical state, artifacts, prompts, and final decisions. |
| Core | factor adoption / keep-discard-retire logic | This is the system truth boundary that must stay inside `ict-engine`. |
| Supporting | `Auto-Quant` execution engine | It generates candidates, runs experiments, and emits research outputs. |
| Supporting | managed dependency lifecycle | Clone / pin / update / rollback is necessary infrastructure for the integration to work across hosts. |
| Generic | git transport / release mirror / local path management | Important, but not domain-defining. |

### Bounded contexts

1. **Dependency Management Context**
   - bootstrap
   - pin
   - update
   - rollback
   - health status

2. **Auto-Quant Execution Context**
   - prepare
   - run
   - resume
   - result export
   - retrospective export

3. **ict-engine Research Orchestration Context**
   - decides when to call Auto-Quant
   - receives candidate package
   - writes canonical artifacts
   - computes next step

4. **Factor Catalog / Retention Context**
   - preserves old factors
   - records lineage and provenance
   - controls explicit retirement only

### Context map

- `ict-engine Research Orchestration` -> `Auto-Quant Execution`
  - relationship: **Customer / Supplier**
  - contract: explicit MCP tool surface

- `Dependency Management` -> `Auto-Quant Execution`
  - relationship: **ACL + Supplier**
  - reason: isolate git/bootstrap/update details from the research workflow

- `Factor Catalog / Retention` <- `ict-engine Research Orchestration`
  - relationship: **same core domain**
  - reason: candidate adoption must remain canonical inside `ict-engine`

### Ubiquitous language

| Term | Meaning |
|---|---|
| managed clone | the local, `ict-engine`-owned checkout of Auto-Quant |
| pinned ref | the exact Auto-Quant commit or tag used by `ict-engine` |
| bootstrap | clone + initial checkout + health verification |
| update check | compare pinned ref with upstream availability |
| handoff package | stable result payload exported from Auto-Quant to `ict-engine` |
| candidate | a proposed evolved strategy/factor produced by Auto-Quant |
| adoption | explicit act of accepting a candidate into `ict-engine` truth |
| retirement | explicit act of deactivating an old factor; never silent |

### Key domain events

- `AutoQuantBootstrapRequired`
- `AutoQuantBootstrapped`
- `AutoQuantUpdateAvailable`
- `AutoQuantUpdateApplied`
- `AutoQuantUpdateRolledBack`
- `AutoQuantRunStarted`
- `AutoQuantRunCompleted`
- `AutoQuantRunFailed`
- `AutoQuantCandidateExported`
- `AutoQuantCandidateAdopted`
- `AutoQuantCandidateDiscarded`
- `FactorRetiredExplicitly`

### Boundary tensions / risks

1. **Version drift**
   - if `ict-engine` reads floating `master`, reproducibility dies

2. **Schema drift**
   - if `ict-engine` reads Auto-Quant's internal files directly, any upstream refactor breaks integration

3. **Factor loss**
   - if adoption is implemented as replacement instead of lineage-preserving add+review

4. **Host-specific assumptions**
   - if the integration assumes local data roots or a pre-existing checkout

5. **Control-plane inversion**
   - if Auto-Quant becomes the decider instead of the execution backend

## Stage 2 — Tactical design

### A. Dependency management context

#### Responsibilities
- track upstream repo URL
- track local clone path
- track pinned ref
- check health
- check for updates
- perform updates
- rollback when update fails

#### Suggested `ict-engine` module home
- `src/application/auto_quant/`

#### Suggested submodules
- `config.rs`
- `repo_manager.rs`
- `status.rs`
- `update.rs`
- `health.rs`

#### Canonical records
- `AutoQuantDependencyConfig`
- `AutoQuantDependencyStatus`
- `AutoQuantPinnedVersion`
- `AutoQuantUpdateReport`

#### Suggested persisted artifact
- `state/<symbol or global>/auto_quant_dependency.json`
  - if symbol-scoped is awkward, keep it global under the state root

### B. Auto-Quant execution context

#### Responsibilities
- map `ict-engine` requests to Auto-Quant runs
- surface `show-config`
- surface `prepare`
- run / resume the experiment loop
- export latest results / retrospective

#### Important rule
- do not let `ict-engine` parse arbitrary prose from `program.md` or raw logs as primary truth
- parse only the explicitly exported handoff package

#### Suggested submodules
- `mcp_contract.rs`
- `run_request.rs`
- `run_result.rs`
- `retrospective.rs`

### C. Research orchestration context

#### Responsibilities
- decide when `factor-research` / `factor-autoresearch` should delegate to Auto-Quant
- preserve old `ict-engine` factors
- ingest the handoff package
- emit canonical artifacts and workflow next-step surfaces

#### Suggested rule
- do not delete the current native path immediately
- first add a backend selection layer:
  - native
  - auto-quant
- only after stable adoption should Auto-Quant become the default execution backend for the targeted workflows

#### Suggested command/backend model
- `factor-research --backend native|auto-quant`
- `factor-autoresearch --backend native|auto-quant`

### D. Factor retention context

#### Responsibilities
- preserve all old factors
- track parent/provenance/version/status on candidates
- require explicit retirement

#### Mandatory lineage fields
- `parent`
- `provenance`
- `version`
- `status`
- `source_backend`
- `source_run_id`

## MCP contract

### Tool surface

- `auto_quant.status`
- `auto_quant.show_config`
- `auto_quant.ensure_bootstrap`
- `auto_quant.check_update`
- `auto_quant.update`
- `auto_quant.start_run`
- `auto_quant.resume_run`
- `auto_quant.latest_results`
- `auto_quant.retrospective`
- `auto_quant.export_candidate`
- `auto_quant.export_summary_for_ict_engine`

### Handoff package minimum schema

- `auto_quant_version`
- `pinned_ref`
- `run_id`
- `strategy_candidates`
- `aggregate_metrics`
- `per_asset_metrics`
- `suspicious_flags`
- `recommended_disposition`
- `artifact_paths`
- `notes`

### ACL rule

Raw Auto-Quant files are external payloads.
They must be translated into `ict-engine` domain objects before reaching:

- workflow logic
- artifact ledgers
- factor adoption logic
- next-step routing

## User / agent prompting

### Startup/operator surfaces must expose

- whether Auto-Quant is bootstrapped
- current pinned ref
- whether an update is available
- whether the local checkout is healthy
- whether the cleaned data root is ready

### Agent-facing prompts should say

- Auto-Quant is managed and versioned
- use status/config before mutation or update
- update is explicit, not automatic
- old factors are retained by default
- adoption and retirement are review operations

## Cross-host rules

### Must work on a fresh host

- no hardcoded local clone path
- no hardcoded data root
- no assumption that Auto-Quant is already installed
- no silent fallback to maintainer-local paths

### Required behavior

- missing dependency -> bootstrap
- unhealthy dependency -> explicit error
- missing data root -> explicit prompt / status surface
- available update -> explicit prompt, not silent switch

## Suggested implementation order

1. **Dependency lifecycle first**
   - managed clone
   - pinned ref
   - status
   - update / rollback

2. **MCP adapter second**
   - stable tool calls
   - handoff package export

3. **Backend selection third**
   - `factor-research` and `factor-autoresearch` gain backend selection

4. **Candidate ingestion fourth**
   - lineage-preserving candidate import
   - no factor loss

5. **Prompt and workflow closure fifth**
   - startup notices
   - agent next-step hints
   - explicit update prompts

## Verification checklist

### Dependency lifecycle
- bootstrap works on a clean host
- update check detects a newer upstream ref
- update can switch to a new ref
- failed update rolls back

### Research orchestration
- old factors remain present after candidate import
- candidate import adds lineage metadata
- adoption/discard decisions are explicit

### Prompting
- startup surface shows bootstrap/update status
- agent surface includes next-step recommendation for update/bootstrap

### Cross-host
- integration does not require maintainer-local paths
- missing cleaned-data root is explicit and recoverable

## Immediate follow-up

After this plan is accepted, the next concrete artifact should be:

- `support/docs/plans/2026-04-24-auto-quant-mcp-implementation-plan.md`

That follow-up should break the work into executable tasks with exact file paths and verification commands.
