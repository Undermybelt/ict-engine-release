# Provider Profile Hot-Plug Plan

Date: 2026-04-29
Status: in_progress
Scope: add an opt-in versioned provider profile JSON and explicit `--profile` selection for `provider-status` / `workflow-status`

## Goal

Ship one versioned provider profile that captures the maintainer's actual data contract without polluting the public default:

- local NQ multi-timeframe historical data
- QQQ paired spot context
- VIX volatility overlay
- QQQ options Greeks / IV / OI
- zero-config live via OpenBB first
- optional operator-managed live runtime reuse via OpenAlice / NoFX
- optional IBKR local bridge reuse

## Boundary Rules

1. Zero-config default remains unchanged when no profile is selected.
2. Personal requirements live only in an explicit profile JSON.
3. Selecting a profile is always opt-in and explicit.
4. Profile loading must be hot-pluggable:
   - bare profile id resolved from repo examples
   - or direct JSON path supplied by the caller
5. `workflow-status` must not silently inherit profile behavior; it only uses the selected profile when the caller passes `--profile`.

## Architecture

- Add a typed provider-profile document loader in the provider catalog layer.
- Store the personal profile as a versioned example JSON under `support/examples/provider_profiles/`.
- Extend `provider-status` with `--profile <id-or-path>`.
- Extend `workflow-status` with `--profile <id-or-path>`.
- When selected:
  - `provider-status` keeps the full global provider truth, plus a profile-specific readiness view
  - `workflow-status` keeps the workflow-specific provider support block, plus the selected profile summary

## File Scope

- Create: `support/examples/provider_profiles/thrill3r-nq-closed-loop-v1.json`
- Modify: `src/application/provider_catalog.rs`
- Modify: `src/application/orchestration/command_entry.rs`
- Modify: `src/application/orchestration/workflow_status.rs`
- Modify: `src/main.rs`
- Modify: `tests/provider_neutral_cli.rs`

## Acceptance

- `provider-status --profile thrill3r-nq-closed-loop-v1 --agent` succeeds and reports profile-specific readiness.
- `workflow-status --profile thrill3r-nq-closed-loop-v1 --agent` exposes the selected profile in `provider_support` without changing default no-profile behavior.
- No profile is auto-loaded from repo state, env vars, or maintainer paths.
- The profile loader works with a repo example id or a caller-supplied JSON path.
