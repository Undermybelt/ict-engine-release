# Auto-Quant × ict-engine Integration Guide

Date: 2026-04-24

Status
- guidance doc for the Auto-Quant integration path
- intended as a stable operator / agent reference
- not an implementation plan

## Goal

Integrate the independently versioned `Auto-Quant` repo into `ict-engine` as the external engine that handles factor research / factor iteration / historical backtesting, while keeping `ict-engine` as the control plane that:

- starts the workflow
- locks and updates the dependency
- receives candidate results
- keeps old factors
- decides what gets adopted, kept, discarded, or retired
- closes the loop back into `next_step` / agent prompts / human surfaces

## Core boundary

### Auto-Quant owns

- strategy rewrite
- strategy evolution
- historical backtest execution
- experiment history
- retrospective generation
- candidate suggestion for factor iteration

### ict-engine owns

- orchestration
- dependency bootstrap/update
- version pinning
- user/agent prompting
- canonical decision making
- artifact ingestion
- keep/discard/adopt/retire
- final workflow closure

## Non-goals

- Do not vendor Auto-Quant directly into `ict-engine`
- Do not silently auto-upgrade Auto-Quant at runtime
- Do not replace `ict-engine`'s control plane with Auto-Quant
- Do not delete or silently overwrite old factors
- Do not assume a maintainer-local Tomac layout exists on every host

## Recommended architecture

### 1. Independent upstream repo

Keep `Auto-Quant` as a separate repo with its own tags / commits / releases.

### 2. Managed local clone

`ict-engine` should maintain a managed local checkout of Auto-Quant:

- initialized on first use
- pinned to a specific commit or tag
- updated only through an explicit update flow
- rollbackable to the previous known-good ref

### 3. MCP adapter layer

Expose Auto-Quant through a stable tool contract instead of raw file-path coupling.

The adapter should present:

- `status`
- `show_config`
- `ensure_bootstrap`
- `check_update`
- `update`
- `start_run`
- `resume_run`
- `latest_results`
- `retrospective`
- `export_candidate`
- `export_summary_for_ict_engine`

### 4. ict-engine closure layer

`ict-engine` should consume the Auto-Quant outputs and convert them into its own canonical artifacts, prompts, and next-step logic.

## Versioning and update policy

### Locking

Keep a lock record for Auto-Quant containing:

- repo URL
- local path
- pinned ref
- adapter version
- last sync time
- health status

### First use

If the dependency is missing, `ict-engine` may bootstrap it automatically.

### Normal use

Use the pinned ref by default.

### Update

Only update when explicitly requested.

If update fails, rollback to the last known-good ref.

### User visibility

Users and agents must be able to see:

- current pinned version
- whether upstream has a newer version
- whether bootstrap is required
- whether the current checkout is healthy

## Required data-readiness rule

Public wrappers and Auto-Quant integrations must not assume the maintainer's local Tomac cleaned-data layout exists.

Instead:

- expose config first
- require explicit data readiness before execution
- refuse to run if cleaned data is not present
- ask for `--data-root` / explicit override on unfamiliar machines

## Closed-loop handoff

Auto-Quant output should be handed back to `ict-engine` in a stable package.

Minimum handoff payload:

- `auto_quant_version`
- `pinned_ref`
- `run_id`
- `strategy_candidates`
- `aggregate_metrics`
- `per_asset_metrics`
- `suspicious_flags`
- `recommended_disposition`
- `artifact_paths`

`ict-engine` then converts that payload into:

- canonical artifacts
- `recommended_next_command`
- `next_step`
- agent prompts
- human-readable summary
- adoption / discard / retire decisions

## Factor retention rule

Old factors must never be lost.

Rules:

- keep all historical factors
- Auto-Quant may add candidates, not silently replace truth
- every evolved factor must carry:
  - `parent`
  - `provenance`
  - `version`
  - `status`
- retirement must be explicit

## Suggested workflow

1. `ict-engine` checks Auto-Quant status
2. if missing, bootstrap Auto-Quant
3. if update is available, inform user/agent
4. run factor research / iteration through Auto-Quant
5. export candidate summary back to `ict-engine`
6. `ict-engine` decides keep / discard / adopt / retire
7. if needed, trigger another Auto-Quant run

## Agent / terminal guidance

The operator-facing prompt should say:

- Auto-Quant is external, versioned, and managed
- never assume local data layout
- use the status/config command first
- update only through the explicit update flow
- keep old factors unless a review explicitly retires them

## Release / cross-host rule

This integration must work on another user's machine:

- first run can bootstrap automatically
- path discovery must be explicit and inspectable
- no local-only defaults that depend on the maintainer's home directory
- no silent version drift

## Immediate follow-up documents

- `docs/release-mirror-runbook.md`
- `docs/release-notes-draft.md`
- `docs/first-run.md`
- `docs/2026-04-24-open-source-shakedown-handoff.md`

## Summary

Auto-Quant should become `ict-engine`'s managed external factor-research engine:

- independent upstream version
- explicit bootstrap/update
- stable MCP contract
- no silent data/layout assumptions
- no factor loss
- canonical closure stays in `ict-engine`
