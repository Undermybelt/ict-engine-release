# kraken-cli agent pattern review

Verdict
- Worth absorbing as contract patterns.
- Not worth absorbing as trading-execution surface.

What is worth learning
- JSON-first stdout/stderr/exit-code contract
- stable machine-routable error categories
- machine-readable tool catalog
- dangerous command metadata
- paper/sim-first workflow packaging
- skill/recipe structure for reusable agent flows

What must be excluded from ict-engine
- live order execution
- account auth flows as first-class repo concern
- withdrawal / transfer / staking operations
- paper-to-live promotion flows
- any design that turns ict-engine into a broker shell

Best-fit migration targets in ict-engine
- `docs/external/*` for adapter contract and catalog shape
- `src/adapters/*` for read-only market-data adapter surface
- `src/types.rs` / adapter contract types for stable error taxonomy
- `src/application/*` for workflow packaging and error routing
- `src/state/*` for adapter provenance and failure audit

Most useful insight
The key value is not exchange breadth. The key value is that the CLI exposes a machine-readable, safety-aware, JSON-first contract that an agent can call without scraping prose. That pattern transfers well to `ict-engine` as a market-data/source adapter layer.

Red flags
- built-in MCP around real-account commands
- explicit live and funding surfaces
- skills that normalize paper-to-live promotion
- dangerous commands as first-class workflows

Recommended ict-engine usage model
- treat kraken-like designs as data-source contract references only
- keep all adapters read-only by default
- keep replay/snapshot/sim paths first-class
- require explicit separate design approval for any future privileged integration
