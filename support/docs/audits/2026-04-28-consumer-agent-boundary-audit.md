# Consumer-Agent Boundary Audit

Date: 2026-04-28
Status: active
Scope: public CLI, internal research tooling, Auto-Quant orchestration, artifact contracts

## Decision

The project must separate into two explicit strata:

1. **Public consumer-agent protocol**
2. **Internal research tooling**

The current repo violates this boundary in several places.

## Required boundary model

### 1. Public consumer-agent protocol

This is the only layer consumers should reach, always through an agent.

Allowed concepts:

- `strategy material package`
- `evidence requirements`
- `provider/runtime requirements`
- `data bindings`
- `evaluation priorities`
- `dispatch`
- `ingest`
- `rank`
- `results`

Forbidden concepts:

- `MSS`
- `FVG`
- `CISD`
- `OTE`
- `Unicorn`
- `Silver Bullet`
- `Judas Swing`
- any other ontology term from the maintainer's research vocabulary

Rule:

- consumer intent must be transformed into agent-produced materials
- repo code must not require consumers to speak the repo's ontology

### 2. Internal research tooling

This layer may keep:

- ICT/PDA ontology
- canonical setup matchers
- primitive factor experiments
- PB12/control-matrix experiments
- domain-specific diagnostics

But it must be explicitly fenced as:

- internal
- experimental
- maintainer/research-oriented

Rule:

- internal research tools may inform agent-produced materials
- they must not be the public protocol itself

## Current boundary violations

### A. Public CLI exposes ontology directly

Current commands:

- `auto-quant-pda-unit-batch`
- `auto-quant-pda-unit-dispatch`

Current public arguments expose ontology directly:

- `--factors order_block,fair_value_gap,...`
- `--combination-size`
- unit labels like `NQ:1h:long:order_block`
- artifact fields like `primitive_sequence`

Why this is wrong:

- these commands require the caller or the calling agent to use the repo's ontology explicitly
- that makes the CLI consumer-facing surface depend on maintainer semantics
- this is the opposite of a generic orchestration protocol

Severity: high

### B. Public orchestration artifacts carry ontology as first-class truth

Current artifact/JSON fields:

- `primitive_sequence`
- `iteration_unit`
- ontology-specific unit labels

Why this is wrong:

- generic orchestration artifacts should carry `strategy spec` / `materials`, not internal taxonomy
- ontology should remain inside agent-produced materials or internal experiment records

Severity: high

### C. CLI currently generates strategy code from ontology

Current dispatch behavior:

- the CLI synthesizes strategy files directly from primitive names
- the repo owns templates for `order_block`, `fvg`, `mss`, etc.

Why this is wrong:

- strategy-generation semantics belong to agent output materials, not to the public CLI contract
- the CLI should execute agent-produced materials, not become the canonical author of trading logic

Severity: high

### D. Repo ontology is not explicitly fenced as internal

Current ontology-rich code lives in:

- `src/ict/*`
- `src/pda_timeline/*`
- `src/factor_lab/factor_definition.rs`
- `auto-quant-promote-canonical-setup`

Why this is only partially wrong:

- keeping internal ontology for research is acceptable
- the problem is that it currently bleeds into the public command surface

Severity: medium

### E. Provider-neutral progress exists but generic strategy protocol is missing

Good existing direction:

- `market-data-harness` request contract is explicit/provider-neutral
- `consumer_evidence_profile` is generic

Missing:

- equivalent generic contract for strategy materials

Severity: medium

## Boundary truths that are already correct

These parts are directionally correct and should be preserved:

- explicit provider/runtime configuration
- `state_dir` / isolated artifact storage
- external Auto-Quant as executor, not control plane
- versioned artifact ledger
- provenance/comparability/state snapshots
- evidence requirements as an explicit contract

## Immediate corrections required

### 1. Freeze ontology-driven public commands as experimental

Short-term rule:

- `auto-quant-pda-unit-batch`
- `auto-quant-pda-unit-dispatch`

must be treated as internal experiment tooling, not final public architecture.

They may stay temporarily, but only if clearly marked:

- internal
- experimental
- subject to replacement

### 2. Introduce generic public names

Replace ontology-first public naming with:

- `agent-material-batch`
- `agent-material-dispatch`
- `agent-material-rank`

These should accept:

- strategy material package path
- evidence profile
- provider/runtime requirements
- evaluation priorities

### 3. Move ontology from protocol into materials

Ontology belongs in:

- agent-generated `.json`
- agent-generated `.md`
- agent-generated `.py`
- isolated `state_dir`

not in:

- public command semantics
- public artifact schema names
- required CLI arguments

### 4. Stop making the CLI the author of trading logic

The CLI may:

- validate
- stage
- dispatch
- ingest
- rank

The CLI should not be the default author of:

- entry-model logic
- primitive-to-strategy translation
- consumer-specific semantics

## Recommended final architecture

### Public path

- consumer -> agent
- agent -> strategy material package
- CLI -> orchestration
- AQ -> execution
- CLI -> result collection
- agent -> user-facing summary

### Internal path

- ontology experiments
- canonical setup matching
- primitive/unit research
- PB12/control-matrix discovery

These may continue, but they should no longer define the public protocol.

## Bottom line

The repo does **not** need to erase ontology everywhere.

It **does** need to stop exporting ontology as the public way consumers use the project.

The main mistake is not "ICT code exists".

The main mistake is:

- ontology-rich research tools became public CLI protocol

That is the boundary we need to repair next.
