# Consumer-Agent Boundary Remediation Plan

Date: 2026-04-28
Status: proposed
Scope: repair public/internal boundary without deleting useful internal research tooling

## Goal

Restore the intended architecture:

- consumer intent goes to agent
- agent emits production materials
- CLI orchestrates execution and result collection
- ontology remains internal

## Principles

1. No consumer ontology in public CLI.
2. No consumer language hardcoded into repo semantics.
3. Internal research tooling may stay, but must be fenced.
4. Public contracts must be generic, explicit, and artifact-first.

## Phase 1: Stop the bleed

- [ ] Mark ontology-driven AQ commands as experimental/internal in help text and docs.
- [ ] Add a boundary note to `support/docs/architecture-boundaries.md`:
  - public protocol generic only
  - ontology allowed only in internal tooling
- [ ] Stop presenting `auto-quant-pda-unit-*` as the final architecture in docs.

## Phase 2: Introduce the real public protocol

- [ ] Add generic command surfaces:
  - `agent-material-batch`
  - `agent-material-dispatch`
  - `agent-material-rank`
- [ ] Define one generic material schema:
  - strategy brief
  - execution scope
  - evidence profile
  - provider/runtime requirements
  - evaluation priorities
- [ ] Keep ontology only inside the material payload, if the agent chooses to put it there.

## Phase 3: Demote ontology-specific schemas

- [ ] Convert public fields like:
  - `primitive_sequence`
  - ontology-rich `unit_label`
  - direct setup/factor CLI args
  into internal/experimental fields or generic `strategy_material_ref`.

- [ ] Ensure public ranking/dispatch artifacts talk about:
  - materials
  - jobs
  - results
  - evidence profiles
  not maintainer ontology.

## Phase 4: Preserve internal research value

- [ ] Keep `src/ict/*`, `src/pda_timeline/*`, and related experiment tools available for internal evaluation.
- [ ] Treat them as one possible producer of agent materials, not the public consumer protocol.

## Acceptance criteria

- A consumer can use the project entirely through an agent without ever seeing ontology terms.
- The public CLI can operate on agent-produced materials alone.
- Internal research tooling remains available without defining public usage.
