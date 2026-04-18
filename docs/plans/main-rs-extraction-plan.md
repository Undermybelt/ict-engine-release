# main.rs extraction plan

Status: release-adjacent structural debt, non-blocking.

Goals
- shrink `src/main.rs`
- move stable command surfaces into focused modules
- preserve CLI behavior and release output compatibility
- isolate release-closure surfaces so future productization does not deepen the monolith

Stage 1
- extract analyze output helpers
- target: `emit_analyze_output`, `emit_analyze_live_output`, human/compact/agent render assembly
- home: `src/application/reporting/` or `src/application/cli_output/`
- include shared agent next-step parsing helpers so analyze/workflow surfaces do not duplicate protocol parsing

Stage 2
- extract workflow-status output helpers
- target: compact/agent/human workflow views, redaction-print boundary
- home: `src/application/orchestration/`
- include structured `next_step` generation and command-humanization helpers

Stage 3
- extract release-closure commands
- target:
  - `research_verdict_command`
  - `evidence_quality_breakdown_command`
  - contamination / experiment-integrity heuristics
- home: `src/application/release_closure/`
- rationale:
  - these surfaces are product-facing and likely to grow with local/cluster ingestion
  - they should not accumulate inside `main.rs`

Stage 4
- extract analyze/backtest/update command input parsing helpers
- move command-specific resolution logic out of main match arms
- keep clap enum in `main.rs` until later stage

Stage 5
- extract test helpers from `src/main.rs`
- move reusable fixtures like `sample_candles` and CLI fixture builders into dedicated test modules
- add focused module tests for release-closure surfaces rather than only monolith tests

Guardrails
- one surface at a time
- preserve serialized field names unless release note says otherwise
- after each extraction run `cargo check` and targeted tests first, then broader suite
- avoid mixing feature work with structural moves
- when moving release-closure logic, keep a stable JSON contract for:
  - `research-verdict`
  - `evidence-quality-breakdown`
  - `workflow-status --agent.next_step`

Post-release optimization targets
- deepen `research-verdict` ingestion for local-search / cluster result dirs without coupling it to ad hoc filenames
- centralize contamination heuristics into one reusable evaluator
- persist richer raw evidence-quality intermediates so breakdown output can stop inferring some terms from policy labels

Release note
- do not block current release on this plan alone
- if a post-release cleanup branch is opened, start with Stage 1 and Stage 3
  - Stage 1 reduces output-surface drift
  - Stage 3 prevents release-closure logic from becoming the next monolith hotspot
