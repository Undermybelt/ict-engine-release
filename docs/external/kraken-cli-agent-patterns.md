# Kraken CLI agent patterns for ict-engine

Source: `krakenfx/kraken-cli` README, AGENTS/CLAUDE integration docs, and skills catalog.

Absorbed patterns:

1. JSON-first subprocess contract
- Always invoke data/tool commands in machine mode.
- stdout carries structured payload only.
- stderr is diagnostics only.
- non-zero exit means failure and should route through stable error categories.

2. Stable error taxonomy
- Route on a compact category code, not message text.
- Minimum useful buckets for ict-engine external adapters:
  - `api`
  - `auth`
  - `network`
  - `rate_limit`
  - `validation`
  - `config`
  - `io`
  - `parse`

3. Safe paper-first execution surface
- All strategy or order flows should have a paper-safe mirror before live.
- For ict-engine this implies simulated execution surfaces should stay first-class, not ad hoc.

4. Goal-oriented skills over raw commands
- Package workflows as compact reusable skills/recipes, not only primitive commands.
- Good fit for ict-engine report generation, factor-debug loops, market brief flows, and regime-specific playbooks.

5. Tool catalog as machine contract
- Keep command metadata machine-readable: args, auth, safety, output type.
- This is a strong reference pattern if ict-engine exposes more external tool surfaces.

Non-adopted parts:
- Do not adopt any live-trading behavior into ict-engine by default.
- Do not wire exchange execution without explicit user scope and separate safety gates.

Suggested ict-engine applications:
- external data adapters should return stable JSON envelopes
- add compact error category routing for adapter failures
- keep paper/backtest/sim path symmetrical with any future live path
- package common workflows as local skills/prompts rather than only CLI flags
