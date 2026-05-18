# Provider Support Workflow Integration Plan

Date: 2026-04-29
Status: in_progress
Scope: feed `provider-status --agent` into `workflow-status` agent/human surfaces without polluting the public default contract

## Goal

Make provider gaps actionable from the existing workflow surfaces:

- `workflow-status --agent` should carry a low-token provider support block when the next step depends on missing providers
- `workflow-status --human` / `--phase human-next` should emit direct install/config guidance instead of only telling the operator to run another command

## Constraints

1. Zero-config default: the command must still work on a clean machine with no extra profile file.
2. Consumer-usable: agent payloads must be structured and directly reusable.
3. Token-friendly: workflow output should include only the relevant provider subset, not the full global catalog dump.
4. No pollution: maintainer-specific provider preferences must not become the public runtime default.
5. Hot-plug ready: any future personal/provider profile layer must plug into one shared builder instead of branching `workflow_status.rs`.

## Architecture

- Keep `provider-status` as the full global catalog truth.
- Add a shared workflow-facing provider-support builder that:
  - starts from the existing `provider-status --agent` surface
  - filters to the providers relevant to the current workflow command/block reason
  - returns a small structured support view with pending providers and actionable prompts
- Wire that support view into:
  - `workflow-status --agent`
  - `workflow-status --human`
  - `workflow-status --phase human-next`

## File Scope

- Modify: `src/application/provider_catalog.rs`
- Modify: `src/application/orchestration/workflow_status.rs`
- Modify: `tests/provider_neutral_cli.rs`

## Acceptance

- `provider-status --agent` remains the canonical full table.
- `workflow-status --agent` exposes provider guidance only when it is relevant.
- `workflow-status --human` stays short, but includes install/config hints when provider readiness is the current blocker.
- The integration path is shared and typed, so an optional future personal profile can be added without reworking the workflow surfaces.
