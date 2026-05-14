# Multi-source intake notes — 2026-04-14

Sources absorbed:
- iVolatility article: SPY IV Study
- MIT diffusion lecture notes PDF
- coherence-lattice-alpha
- GenericAgent
- FinRobot

## Decisions

### iVolatility SPY IV Study
Disposition: learn only.
Why: useful as volatility-regime reference, but web article alone is not a durable integration surface. Keep as idea input for future volatility/regime heuristics, not direct code source.

### MIT diffusion lecture notes
Disposition: learn only.
Why: useful conceptual reservoir for generative/time-series ideas, but too broad for direct ict-engine insertion without a defined modeling objective.

### coherence-lattice-alpha
Disposition: reject for direct ict-engine integration; only minor conceptual inspiration.
Why: claims are highly speculative and physics-specific. Do not route its alpha-constant derivations into trading logic. The only transferable idea is dynamic coupling / coherence-vs-structure tension, but even that should be treated as metaphor, not imported machinery.

### GenericAgent
Disposition: absorb patterns.
Valuable patterns:
- minimal atomic toolset
- layered memory model
- execution-path crystallization into reusable SOP/skill
- machine-readable tool schema
For ict-engine, the best transfer is workflow/productization discipline, not OS/browser control.

### FinRobot
Disposition: learn + selective absorb.
Valuable patterns:
- financial workflow decomposition into data -> analysis -> report
- separation of agents, data sources, and functional modules
- quantitative/report surfaces kept explicit
Do not adopt its full multi-agent LLM stack or API-key-heavy runtime as-is.

## Net effect on ict-engine

Useful cross-source principles:
1. Keep workflow surfaces explicit and typed.
2. Keep minimal primitive tools / modules, grow higher-level SOPs on top.
3. Separate data ingestion, quantitative analysis, and report generation more mechanically.
4. Treat volatility regime research as a first-class conditional surface.
5. Avoid speculative grand unified theories in trading logic.
