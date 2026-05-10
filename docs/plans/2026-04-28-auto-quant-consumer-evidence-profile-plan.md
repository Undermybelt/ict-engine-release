# Auto-Quant Consumer Evidence Profile Plan

Date: 2026-04-28
Status: implementation-active
Scope: extend `auto-quant-pda-unit-batch` so consumer agents can declare required evidence explicitly

Boundary note:
- The evidence-profile concept is still valid.
- The final consumer-facing carrier for it should be `agent-material-*`, not the ontology-driven PDA unit surface.

## Goal

Make `auto-quant-pda-unit-batch` consumer-ready by letting the caller attach an explicit evidence profile per unit batch, instead of assuming that repo PDA primitives are the full strategy truth.

## Contract

The new evidence profile must support:

- required evidence surfaces
- required indicator names
- freeform user/agent notes
- provider guidance derived from the declared surfaces

Examples of surfaces:

- `indicators`
- `volatility`
- `greeks`
- `open_interest`
- `implied_volatility`
- `options_chain`
- `cross_market`
- `session_context`

## Required behavior

1. The CLI accepts explicit evidence requirements.
2. The batch manifest persists them once at batch level.
3. Each unit brief includes them in plain language.
4. Each AQ handoff includes them as structured unit context.
5. Agent prompts explicitly say when a provider/runtime must expose Greeks/OI/IV rather than fabricating them locally.

## Non-goals

- no local re-synthesis of missing Greeks/OI/IV from a weak local model
- no hardcoded repo claim that consumers must use the maintainer's preferred evidence stack
- no provider-specific lock-in beyond guidance derived from requested evidence surfaces
