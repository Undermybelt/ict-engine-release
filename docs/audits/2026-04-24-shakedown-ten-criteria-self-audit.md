# ict-engine Ten-Criteria Self-Audit · 2026-04-24

## Scope

This audit grades the current `green-baseline` tree against ten explicit project criteria. The intent is to lock a fact-based baseline that future work can be measured against.

- branch: `green-baseline`
- HEAD at audit time: `6b34fc6 Tighten workflow status and update templates`
- CI gates all green: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- Smoke baseline: `/tmp/ict-engine-smoke-2026-04-24/final-summary.md` from the same date

## The ten criteria

1. Lightweight
2. Agent-friendly
3. Token-friendly
4. Guidance-friendly (including guiding an agent to produce human-friendly output)
5. Has a data source
6. Has a historical-data backtest path
7. Has factor creation / backtest / iteration / evolution / promotion-to-live capability
8. Has live-decision timeliness
9. Has prior validation and post-hoc reflection
10. Eventually lands into painless BBN iteration

## Summary table

| # | Criterion | Verdict | Core evidence |
| --- | --- | --- | --- |
| 1 | Lightweight | Met | single Rust binary, cold-start 0.03-0.2 s, `--state-dir` is the only required side-effect, CI only runs fmt / clippy / test |
| 2 | Agent-friendly | Met | `--agent` and `--compact` output modes, `workflow-status` emits `next_step` / `next_command`, `docs/agent-first-runbook.md` defines agent intent routing |
| 3 | Token-friendly | Met | compact / agent / human outputs are small and stable; `analyze --output-format json` now trims growing ledger arrays by default and preserves opt-in full inline mode via `--inline-ledger` |
| 4 | Guidance-friendly | Met | `README.md` + `docs/first-run.md` + `docs/agent-first-runbook.md`; `src/application/reporting/human_report.rs::humanize_next_step_line` prevents raw `ask-user:` syntax from leaking into human output |
| 5 | Data source | Met | `analyze-live` over openbb / openalice / nofx backends; `clean-futures` for TOMAC CSV ingest; `examples/demo/demo-15m.json` bundled demo; `src/application/data_sources/` submodule |
| 6 | Historical backtest path | Met | `backtest`, `factor-backtest` (walk-forward + learning updates), `futures-sop`, `expansion-sop` |
| 7 | Factor creation / backtest / iteration / evolution / promotion | Met | `factor-research` -> `factor-mutation-status` -> `factor-autoresearch` -> `factor-autoresearch-status` -> `research-verdict` -> `update` |
| 8 | Live-decision timeliness | Met | `src/application/decision_freshness.rs`, `data_sources/source_freshness.rs`, `data_sources/source_health.rs`, freshness/staleness handling in `analyze_output.rs`, 10-minute stale threshold on autoresearch live snapshots |
| 9 | Prior validation and post-hoc reflection | Met | `src/application/reflection/`: `prior_artifact.rs`, `postmortem_artifact.rs`, `attribution.rs`, `research_adapter.rs`; `factor_autoresearch_retrospective.md` derived surface |
| 10 | Painless BBN iteration | Met | `src/bbn/` covers dag / evidence / inference / nodes / schema; `src/bbn/engine/`, `bbn/learning/cpt_updater.rs`, `bbn/trading/cpt_init.rs`, `bbn/trading/update.rs`, `bbn/trading/family_overlay.rs`; `update --symbol X --outcome <label>` is a single-command update; `factor-backtest` now emits `suggested_update_command` and the update recommendation surface exposes a ready=false command template when the realized outcome is still missing |

Final count: **10 met, 0 partial.**

## Evidence details

### 1. Lightweight

- 48 MB debug binary (`target/debug/ict-engine`); release build is smaller.
- Phase B smoke showed 0.03-0.04 s steady-state invocation latency for empty-state queries.
- The only required runtime side effect is the caller-provided `--state-dir`.
- CI is just three gates: `fmt --check`, `clippy --all-targets -- -D warnings`, `test`.

### 2. Agent-friendly

- `analyze --agent`, `workflow-status --agent`, etc. all emit JSON with explicit `next_step` and `next_command` routing fields.
- `--compact` is a token-efficient agent mode that still carries the routing fields.
- `docs/agent-first-runbook.md` defines six common agent intents and maps them onto CLI routes.
- `src/agent/prompts.rs` carries machine-targeted prompt surfaces.

### 3. Token-friendly

Original issue observed in `/tmp/ict-engine-smoke-2026-04-24/final-summary.md` Phase A:

- `analyze --demo --human`: 22 lines, stable after first warm-up run
- `analyze --demo --agent`: 28 lines, constant
- `analyze --demo --compact`: 24 lines, constant
- `analyze --demo --output-format json`: **3892 -> 6438 lines across 10 repeated invocations in the same state dir**, ~144 line growth per invocation after bootstrap

That was not a crash and `rc` stayed 0 across all 10 invocations. It did however mean that:

- long-running agents that repeatedly call `analyze` on the same state dir would eat their own context
- human users would see a monotonically growing JSON report
- the JSON surface was the one that did not respect a token budget

Status now:

- `analyze --output-format json` trims the growing `workflow_snapshot.actionable_artifacts` and `workflow_snapshot.artifact_lineage_summaries` arrays to a fixed tail by default
- sibling `*_inline_meta` objects expose `total_count`, `omitted_count`, and `pointer_command`
- `--inline-ledger` restores the legacy full-inline behavior when needed

### 4. Guidance-friendly

- `README.md`, `docs/first-run.md`, `docs/agent-first-runbook.md`, `docs/smoke-acceptance.md`, `docs/release-mirror-runbook.md` form a complete onboarding chain for contributors, agents, operators, and release owners.
- `src/application/reporting/human_report.rs::humanize_next_step_line` converts `ask-user:` protocol syntax to natural-language prompts for human output.
- `src/application/reporting/analyze_output.rs` asserts that human output does not contain `ask-user:` tokens, preventing the machine protocol from leaking.
- Phase A showed `--human` is byte-stable after warm-up (2 distinct hashes across 10 runs, run 01 vs runs 02-10).

### 5. Data source

- `analyze-live --futures-backend <openbb|openalice|nofx> --aux-backend <...>`: live futures + auxiliary spot/options evidence over three switchable backends.
- `clean-futures --root <TOMAC csv root> --output-dir <cleaned json dir> --interval 15m --multi-timeframe`: bootstraps cleaned-candle bundles from local TOMAC-style futures CSVs.
- `src/application/data_sources/`: `clean_futures.rs`, `live_defaults.rs`, `sop_reports.rs` (36 KB of SOP helpers), `source_freshness.rs`, `source_health.rs`, `source_snapshot.rs`.
- `examples/demo/demo-15m.json` is a bundled demo candle bundle so every smoke run works without external data.

### 6. Historical backtest path

- `backtest` for full historical replay.
- `factor-backtest --data <cleaned-candles.json>` for factor-level walk-forward with learning updates.
- `futures-sop` and `expansion-sop` wrap the clean -> research -> summarize -> rank pipeline as one command.

### 7. Factor lifecycle: create / backtest / iterate / evolve / promote

CLI-level chain, all already present:

- `factor-research`: factor research sandbox
- `factor-mutation-status`: mutation history + clustered failure tags
- `factor-autoresearch`: checkpointed keep/discard autoresearch loop
- `factor-autoresearch-status`: read-only view over sessions / attempts / cluster scoreboard
- `research-verdict`: single compact verdict for research closure
- `update`: promote realized outcome into BBN

Derived-surface support (added in the previous commit `88e388c`):

- `experiments.tsv` under `<state_dir>/<symbol>/`
- `factor_autoresearch_retrospective.md` under the same directory

### 8. Live-decision timeliness

- `src/application/decision_freshness.rs`: pipeline-level freshness model.
- `src/application/data_sources/source_freshness.rs`, `source_health.rs`, `source_snapshot.rs`: data-source-level freshness and health.
- `src/application/reporting/analyze_output.rs`: 13 match sites handling freshness / staleness in rendering.
- `factor-autoresearch-status` treats a live snapshot as `interrupted` if it has claimed `running` for more than 10 minutes without a final-summary write.

### 9. Prior validation and post-hoc reflection

- `src/application/reflection/prior_artifact.rs`: prior snapshots.
- `src/application/reflection/postmortem_artifact.rs`: post-hoc reflection artifacts.
- `src/application/reflection/attribution.rs`: attribution surfaces.
- `src/application/reflection/research_adapter.rs`, `adapter.rs`: adapters into the reflection pipeline.
- `factor_autoresearch_retrospective.md` is the newly-landed derived surface for a human- and agent-readable retrospective.

### 10. Painless BBN iteration

Model and update machinery:

- `src/bbn/`: `dag.rs`, `evidence.rs`, `inference.rs`, `node.rs`, `ict_node_schema.rs`, plus the `engine/`, `learning/`, `trading/`, `model/`, `temporal/`, `adapters/` subtrees.
- `src/bbn/trading/update.rs` is where realized trade outcomes land.
- `src/bbn/learning/cpt_updater.rs` handles CPT updates.
- `src/bbn/trading/cpt_init.rs` and `src/bbn/trading/family_overlay.rs` manage CPT initialization and family overlays.

Agent / human surfaces into BBN:

- `update --symbol <sym> --outcome <label>`: single-command update (optional `--pnl`, `--regime`, `--direction`, `--feedback-file`).
- `pre-bayes-status`, `pre-bayes-diff`: inspect the Pre-Bayes layer state and per-step diffs.
- `evidence-quality-breakdown`: composition of the latest Pre-Bayes evidence quality score.
- `artifact-lineage`, `artifact-status`, `artifact-diff`: artifact-level lineage, status, and diff.

The remaining polish point here is no longer command discoverability. `factor-backtest` now emits `suggested_update_command`, and the update recommendation contract surfaces a template command even when `ready=false` because the realized outcome is still missing.

## Closed Follow-Ups

The three follow-ups called out in the original audit have now landed:

1. `factor-backtest -> update` now emits `suggested_update_command`.
2. `analyze --output-format json` now respects a token budget by default, with `--inline-ledger` as the explicit opt-out.
3. `workflow-status --stable` now strips volatile timestamp-like fields so agents can cache or diff stable output.

## Recommended follow-ups

Ordered by impact-to-effort:

1. Keep reducing `src/main.rs` by continuing the extraction plan in `docs/plans/main-rs-extraction-plan.md`.
2. Deepen `research-verdict` only if the project truly needs a fuller experiment analytics engine.
3. Persist richer raw evidence-quality intermediates so `evidence-quality-breakdown` can stop inferring some terms from persisted policy/filter state.

Items 1 and 2 are explicitly scheduled as the two follow-up commits after this audit lands.

## How this audit was produced

- CLI enumeration: `./target/debug/ict-engine --help` and per-subcommand `--help`.
- Static source probes over `src/bbn/`, `src/application/reflection/`, `src/application/data_sources/`, `src/application/decision_freshness.rs`, `src/application/reporting/`.
- Doc cross-check against `README.md`, `docs/agent-first-runbook.md`, `docs/first-run.md`, `docs/smoke-acceptance.md`, `docs/release-mirror-runbook.md`, `docs/autoresearch-derived-surfaces-contract.md`, `docs/autoresearch-state-transitions.md`.
- Runtime evidence from `/tmp/ict-engine-smoke-2026-04-24/final-summary.md`: 100 CLI runs + 10 `cargo test` runs, all `rc=0`, 617/617 tests green across all 10 runs.
