# Handoff TODO — Factor Path Optimization & Hot-Plug Implementation

> Living document. Update after every concrete step. No stale entries.
> Created: 2026-05-09

## Problem Diagnosis

GPT agents entering this repo consistently report "no usable factors" because:
1. No `AGENTS.md` existed — no entry map for agents
2. Factor families A-H documented only in a 436KB TODO doc — unscannable
3. Families E/F/H have zero code presence — grep returns nothing
4. Factor code split across `factor_lab/` and `factors/` — no index

## Completed Steps

- [x] Created `AGENTS.md` — agent entry map with full factor traceability table
- [x] Created `support/docs/factor-catalog.md` — single-page factor family → code → status index
- [x] Created this handoff document
- [x] Phase 1-5: factor hotplug, auto-quant isolation, config, verification, doc sync
- [x] Phase 6: sweep remaining FactorRegistry::default() call sites with hotplug (sop_reports.rs)
- [x] Phase 6: consolidate 23 experiment state_* dirs into state_experiments/ — repo root clean
- [x] Phase 6: .gitignore updated — state_experiments/ excluded

## Active TODO

### Phase 1: Hot-Plug FactorCategory Enum Extension

- [x] Add `FactorCategory` variants: `CrowdingHerding`, `SpectralRhythm`, `SessionLiquidity`
- [x] Add `FactorDefinition` constructors for each new variant
- [x] Register new variants in `FactorRegistry::default()`
- [x] Add `FactorRole` mappings in `allowed_roles()` for new categories
- [x] Wire evaluate dispatch for new categories
- [x] Add compute stubs for E (crowding), F (spectral), H (session)
- [x] Add mutation_parameter_group, mutation_direction_hint, mutation_step_size_hint for E/F/H
- [x] Verify `cargo check` passes with new enums and compute paths

### Phase 1b: Auto-Quant Output Path Isolation

- [x] Added `resolve_auto_quant_output_dir()` — routes auto-quant to `<state_dir>/auto-quant/` subdirectory
- [x] Added `ICT_ENGINE_AUTO_QUANT_OUTPUT_DIR` env var for user override
- [x] Updated all auto-quant shell functions to use `aq_state_dir()` resolver
- [x] Auto-quant artifacts now land in `state/auto-quant/<SYMBOL>/` instead of `state/<SYMBOL>/`
- [x] Repo root stays clean; zero pollution

### Phase 2: Minimal Compute Stubs for E/F/H

- [x] Family E: `evaluate_crowding` — volume-participation ratio + same-side pressure proxy
- [x] Family F: `evaluate_spectral` — spectral entropy + dominant cycle energy from returns
- [x] Family H: `evaluate_session` — kill-zone alignment + session participation quality
- [x] Wire stubs into `FactorEngine` evaluation dispatch

### Phase 3: Hot-Plug Configuration

- [x] Add `config/factor_hotplug.yaml` — user can enable/disable families per symbol
- [x] FactorHotplugConfig Rust module: load, parse, apply to FactorRegistry
- [x] FactorEngine reads hotplug config at init; skips disabled families
- [x] Zero-config default: all families enabled; user can opt out via YAML or env var
- [x] ICT_ENGINE_FACTOR_HOTPLUG_CONFIG env var for custom config path override
- [x] serde_yaml dependency added for YAML parsing
- [x] 3 unit tests passing (default all-enabled, custom disable, apply-to-registry)

### Phase 4: Verification

- [x] `cargo check --all-targets` green
- [x] `cargo clippy --all-targets -- -D warnings` green
- [x] `cargo test` green (888 tests)
- [x] `ict-engine analyze --demo --human` still works with new families
- [x] Hot-plug config works: disabling E/F/H via YAML reduces quality score as expected

### Phase 5: Commit & Doc Sync

- [x] Commit all changes (3 commits: f36e468, e31fa98, 30b843f)
- [x] Update `support/docs/factor-catalog.md` status columns
- [x] Update `AGENTS.md` traceability table
- [x] Update this handoff TODO

## Architecture Constraints

- Zero-config: `ict-engine analyze --demo --human` must work with zero setup
- Consumer-usable: CLI surfaces unchanged; new factors flow through existing engine
- Token-friendly: `--human` compact output; `--compact` machine output
- No pollution: state dirs remain `/tmp/...` by default
- No debt: new FactorCategory variants must have compute stubs, not just enum shells
- Hot-pluggable: user can disable any family via optional config; engine gracefully skips

## Design Notes

### Family E: Crowding / Herding

Compute proxies from available data (no external data required):
- `participation_concentration`: volume spike ratio vs rolling median
- `same_side_pressure`: directional volume imbalance
- `crowding_relief`: post-sweep volume decay rate

### Family F: Spectral Rhythm / Chaos

Reuse existing spectral infrastructure from execution tree:
- `spectral_entropy`: already computed as execution-tree input
- `dominant_cycle_energy`: already computed as execution-tree input
- `rhythm_stability`: cycle-phase alignment variance

### Family H: Session / Liquidity Window

Based on timestamp + volume pattern:
- `session_participation_quality`: volume vs session-average profile
- `kill_zone_alignment`: time-of-day proximity to known kill zones
- `session_transition_risk`: near session boundary detection

## Blockers

None currently.

## Changelog

- 2026-05-09: Created. Phase 1 in progress.
- 2026-05-09: Phase 1/1b/2 completed. Commit f36e468.
- 2026-05-09: Phase 3 completed. FactorHotplugConfig + YAML hot-plug + serde_yaml.
