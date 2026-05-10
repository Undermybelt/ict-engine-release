# Research Bundle Architecture Execution Board

> Authoritative execution board for translating `docs/plans/202605081903searched` into repo-bounded workstreams.
> This file is the execution contract; keep it updated in place.

**Goal:** turn the searched Bundle A/B/C material into architecture-aligned, low-pollution implementation lanes that fit `ict-engine`'s existing public surfaces, artifact boundaries, and split Auto-Quant boards.

**Architecture:** preserve the repo boundary `offline trainer -> explicit artifact -> runtime read-only consume -> token-friendly status surface`. New research systems stay outside the live runtime until they can prove value through explicit artifacts, `/tmp/...` state runs, and existing public CLI surfaces. Do not collapse factor discovery, calibration, provider bootstrap, and runtime closure into one mixed lane.

**Tech Stack:** Rust CLI public surfaces (`provider-status`, `workflow-status`, `factor-research`, `factor-autoresearch-status`, `policy-training-status`, `artifact-status`, `export-structural-path-ranking-target`, `auto-quant-results-import`, `auto-quant-prior-init`, `auto-quant-ingest-real-trades`), repo docs under `docs/`, additive external helpers under `scripts/research/` and `scripts/auto_quant_external/`, explicit `/tmp/...` state dirs, JSON/JSONL/CSV artifact surfaces, optional research-side Parquet companions only when needed.

**Baseline / Authority Refs:** `docs/plans/202605081903searched`, `docs/audits/2026-05-08-paper-search-outline-and-matrix.md`, `docs/structural-belief-learning-repo-map.md`, `docs/plans/2026-05-05-algo20250505-ict-engine-integration-plan.md`, `docs/plans/2026-05-05-execution-tree-factor-auto-quant-todo.md`, `docs/plans/2026-05-07-auto-quant-post-factor-runtime-closure-todo.md`, `docs/plans/2026-05-02-catboost-path-ranking-target-design.md`, `src/application/provider_catalog.rs`, `src/market_catalog/mod.rs`, `src/application/regime/persistence.rs`, `src/application/orchestration/structural_playbook.rs`, `src/application/orchestration/policy_engine.rs`, `src/application/orchestration/workflow_status.rs`.

**Compatibility Boundary:** keep public CLI wording generic, provider-neutral, consumer-usable, and token-friendly. Do not leak `OpenBB`, `OpenFIGI`, `Auctus`, `Aurum`, `OBP`, `VW`, `MAPIE`, `AlphaGen`, or similar upstream tool names into default runtime surfaces. Do not add runtime dependencies on research-only Python libraries just to satisfy a paper-shaped idea. Preserve the split between factor discovery and post-factor runtime closure.

**Verification:** use repo-native command evidence first (`provider-status`, `workflow-status`, `factor-research`, `factor-autoresearch-status`, `policy-training-status`, `artifact-status`) plus targeted unit tests and helper-script compile/smoke checks. Treat fresh `/tmp/...` runs and explicit artifacts as the proof floor.

---

## Decision Lock

- [x] `docs/plans/202605081903searched` is a search-result source file, not an execution contract. This board is the execution contract derived from it.
- [x] The stable repo boundary remains `offline trainer -> explicit artifact -> runtime read-only consume`.
- [x] Bundle A, B, and C must map onto existing repo owners before any new tool or paper is treated as "the architecture."
- [x] `docs/plans/2026-05-05-execution-tree-factor-auto-quant-todo.md` remains authoritative for factor discovery, market/timeframe breadth, trade-density proof, and external strategy iteration.
- [x] `docs/plans/2026-05-07-auto-quant-post-factor-runtime-closure-todo.md` remains authoritative for post-factor import, prior-init, real-trade ingestion, structural path export, ranker registration/application, and execution-tree before/after evidence.
- [x] New research imports should begin as additive helpers and artifact contracts under `scripts/research/` or `scripts/auto_quant_external/` before reopening Rust runtime code.
- [x] JSON / JSONL / CSV remain the default repo contracts. Parquet may exist as a research-side companion, but it is not a required runtime contract.
- [x] Mandatory adoption of heavy external orchestrators (`Dagster`, `OpenLineage`, `MLflow`, `DVC`) is rejected for this lane until the core repo artifact contracts are already proven useful.
- [x] `pyhsmm` is research-only reference material here; it is not a production dependency target.

## Hard Constraints

- Preserve zero-config default behavior for consumers.
- Keep public status surfaces generic; no repo-owned ontology or vendor-specific wording should leak into the default CLI.
- Use explicit `/tmp/...` state dirs for all new verification runs.
- Prefer external helper scripts over runtime code changes when the gap is still "prove the artifact shape" rather than "runtime lacks a reader."
- Do not create a third overlapping factor/runtime board; update the existing authoritative boards when a slice belongs there.
- Do not claim a literature import is needed if the repo already has the same capability in narrower but usable form.
- If a paper suggests a black-box online learner in the runtime path, stop and remap it to an offline artifact or reject it.

## Repo Truth Snapshot

### Bundle A already has partial repo anchors

- `src/application/provider_catalog.rs` already owns provider readiness, install prompts, and `provider-status --agent/--compact`.
- `src/market_catalog/mod.rs` and `src/application/data_sources/live_defaults.rs` already own repo-side market-key/default inference.
- `src/application/orchestration/workflow_status.rs` already routes users toward provider/workflow next steps without requiring them to parse a raw provider table.
- Missing piece: a generic dataset resolver / symbol-resolution artifact that removes manual historical-path selection without forcing repo runtime to hard-bind a vendor stack.

### Bundle B already has stronger repo coverage than the searched doc implies

- `src/application/regime/persistence.rs` already supports `hmm_numeric_trainer_artifact.json` and runtime fallback loading.
- `docs/structural-belief-learning-repo-map.md` confirms delayed reward, IPS, SNIPS, DR, hazard, duration, censoring, and competing-risk surfaces are already partially landed.
- `src/application/orchestration/structural_playbook.rs` and `src/application/orchestration/workflow_status.rs` already export and summarize structural path-ranking rows, propensity, mature-row status, and external-score application boundaries.
- `scripts/research/hmm_numeric_trainer.py`, `scripts/auto_quant_external/external_regime_hmm_viterbi.py`, and `scripts/auto_quant_external/external_regime_changepoint_labels.py` already provide external HMM / changepoint hooks.
- Missing piece: explicit reviewable report artifacts that package these signals into canonical OPE / delayed-truth outputs instead of leaving them scattered across status surfaces and ad hoc helper outputs.

### Bundle C already has real anchors and a strict split

- `docs/plans/2026-05-05-execution-tree-factor-auto-quant-todo.md` already defines the breadth / trade-density / resonance / provider-budget rules for factor iteration.
- `src/application/orchestration/policy_engine.rs` already exposes a CatBoost-compatible artifact reader boundary rather than hard-binding CatBoost into runtime.
- `docs/plans/2026-05-02-catboost-path-ranking-target-design.md` and `docs/structural-belief-learning-repo-map.md` already define the path-ranking target/export/status contract.
- `scripts/auto_quant_external/pandas_path_ranker_trainer.py` and `scripts/auto_quant_external/path_ranker_integration.py` already prove external ranker hot-plug and repo-native artifact registration paths.
- Missing piece: a white-box factor artifact contract that is consistent with the existing factor board and cleanly hands candidate packs to the runtime-closure board without creating another parallel factor loop.

## Bundle-To-Repo Map

| Bundle / spike | Best repo landing zone | Current truth | Main gap to close |
| --- | --- | --- | --- |
| Bundle A / Spike 1 zero-config bootstrap | provider catalog + market catalog + external dataset resolver helper | provider/workflow surfaces and market defaults already exist | no canonical dataset resolver artifact or deterministic symbol-resolution artifact |
| Bundle B / Spike 2 belief/path calibration | structural feedback + path-ranking target export + `policy-training-status` | IPS/SNIPS/DR, propensity, and calibration diagnostics already exist | no compact `policy_correction_report.json` / `ope_ci.json` review artifact |
| Bundle B / Spike 3 delayed reward + regime truth | regime persistence + structural prior surfaces + external HMM/changepoint helpers | HMM numeric artifact, hazard/duration diagnostics, and helper scripts already exist | no canonical delayed-truth export/report contract for survival/changepoint review |
| Bundle C / Spike 4 white-box factor iteration | factor board first, runtime-closure board second | factor breadth rules and external ranker boundary already exist | no explicit white-box factor artifact contract tied to the current factor board |

## Resource Disposition

This section exists so the searched document's named resources are all normalized into repo terms instead of left as a flat recommendation list.

### Adopt now as external-helper or artifact references

| Resource family | Repo landing | Disposition |
| --- | --- | --- |
| OpenBB / OpenFIGI / pytickersymbols | Bundle A dataset resolver helper | adopt only as external lookup/reference inputs behind a generic symbol-resolution artifact |
| Open Bandit Pipeline / Vowpal Wabbit / Doubly Robust | Bundle B policy-correction report | adopt as offline OPE helper implementations if needed, not as runtime libraries |
| MAPIE / TorchCP / conformal risk control | Bundle B path confidence artifacts | adopt only through explicit confidence/lower-bound report artifacts |
| lifelines / scikit-survival / ruptures / hmmlearn | Bundle B delayed-truth and regime helper lane | adopt as offline report/trainer helpers feeding explicit artifacts |
| pgmpy / pyAgrum / DoWhy | existing BBN external artifact lane | adopt only as offline BBN evidence / structure / refutation helpers that emit explicit candidate artifacts |
| AlphaGen / PySR | Bundle C white-box factor artifact lane | adopt only as candidate-expression generators; never direct runtime factor engines |
| Alphalens / vectorbt | factor board breadth / trade-density / transfer evaluation | adopt as offline evaluation helpers referenced by factor artifacts |
| CatBoost / LightGBM LambdaRank | existing path-ranker external model boundary | adopt only as external ranker artifact producers feeding repo-native status/runtime contracts |

### Read later, but do not make part of the immediate execution lane

| Resource family | Why defer |
| --- | --- |
| Auctus / Aurum | useful conceptual references for dataset discovery and relationship graphs, but current repo gap is simpler: deterministic dataset resolver artifacts first |
| Dagster / OpenLineage / MLflow / DVC | useful lineage/orchestration references, but current repo already has artifact-ledger and explicit state-dir discipline; mandatory infra adoption would overreach |
| Qlib | useful benchmark harness reference, but not needed before the factor artifact contract is explicit |
| AlphaAgent / Alpha-GPT / AlphaBench | useful research reading for proposal generation and benchmarking, but not suitable as the immediate runtime or factor-loop architecture |
| Cross-Market Alpha paper | good validation rubric for transfer checks; use as evaluation guidance, not as infrastructure |

### Reject from the runtime hot path

| Resource family | Reason |
| --- | --- |
| vendor-specific provider concepts in public CLI | violates generic consumer-facing surface boundary |
| black-box online policy optimization loops | violates explicit-artifact, runtime-read-only boundary |
| unmanaged pyhsmm-style production dependency | stale and unnecessary for current repo artifact boundary |

## Current Todo Board

### Done

- [x] Read and normalize `docs/plans/202605081903searched` into repo-bounded concerns instead of treating its upstream tools as direct architecture.
- [x] Verify the current repo already has live anchors for:
  - `provider-status`
  - `workflow-status`
  - `policy-training-status`
  - structural path-ranking target export / registration / score apply
  - HMM numeric artifact loading
  - external HMM / changepoint / CatBoost helper scripts
- [x] Lock the board split:
  - Bundle C factor discovery stays on `2026-05-05-execution-tree-factor-auto-quant-todo.md`
  - Bundle C runtime adoption stays on `2026-05-07-auto-quant-post-factor-runtime-closure-todo.md`
- [x] Reject direct runtime adoption of searched upstream systems where the repo already has a narrower explicit-artifact equivalent.
- [x] Workstream 1 / Slice 1 completed as an external-only helper:
  - Added `scripts/research/market_data_resolver.py`.
  - Added `scripts/research/tests/test_market_data_resolver.py`.
  - The helper emits `symbol_resolution.json`, `data_catalog.json`, and `normalized_dataset_summary.json` from existing market presets plus optional provider-profile opt-in lanes.
  - Verified generic zero-config behavior and personal opt-in reuse behavior without reopening runtime code.
- [x] Workstream 2 / Slice 1 completed as an external-only report helper:
  - Added `scripts/research/policy_truth_reports.py`.
  - Added `scripts/research/tests/test_policy_truth_reports.py`.
  - The helper emits `policy_correction_report.json`, `ope_ci.json`, `path_confidence_bounds.json`, `duration_posterior.json`, and `hazard_summary.json` from existing repo JSON surfaces.
  - Verified the report family can be built from current `policy-training-status` and `workflow-status` structural phases without adding a new runtime trainer.
- [x] Workstream 3 / Slice 1 completed as an external-only factor-candidate helper:
  - Added `scripts/research/factor_candidate_pack.py`.
  - Added `scripts/research/tests/test_factor_candidate_pack.py`.
  - The helper emits `factor_expression.json`, `factor_eval_grid_summary.json`, and `transfer_score.json` from existing Auto-Quant manifest evidence plus an optional candidate-spec JSON.
  - Verified the candidate pack can be built from current strategy-library evidence without reopening `ict-engine` runtime code.
- [x] Workstream 1 / Slice 2 completed as a minimal runtime readback:
  - `workflow-status --agent` now surfaces `dataset_resolution_line` from existing selected-profile state or from the generic zero-config lane.
  - The richer dataset contract stays on the agent/JSON surface; the human surface remains shorter and token-friendly.
  - No runtime dependency on `scripts/research/market_data_resolver.py` was introduced.
- [x] Workstream 2 / Slice 2 decision locked:
  - no new runtime trainer or extra CLI surface is needed for this lane right now
  - the minimal runtime readback set is:
    - `policy-training-status`
    - `workflow-status --phase structural-validation-summary`
    - `workflow-status --phase structural-temporal-summary`
    - `workflow-status --phase structural-recommended-path-bundle`
  - any later reopening should be a compact reader/summarizer on these owners, not a parallel calibration surface

### Next

- [x] No additional active slice remains on this board. If Bundle B later needs runtime reopening, keep it to a compact reader/summarizer on the existing owners instead of adding a new training surface.

### Not Yet

- [ ] Mandatory runtime integration of `OpenBB`, `OpenFIGI`, `Auctus`, `Aurum`, `OBP`, `VW`, `MAPIE`, `AlphaGen`, `PySR`, or `Qlib`
- [ ] Dagster / OpenLineage / MLflow / DVC as required repo infrastructure
- [ ] Turning research-side Parquet into a required runtime dependency
- [ ] Merging the factor board and the runtime-closure board into one mega-plan
- [ ] Direct runtime LLM factor generation

## Workstream 1: Bundle A Dataset Resolver / Bootstrap

**Objective:** remove manual historical-file-path friction by introducing a generic dataset-resolver artifact lane without polluting public CLI wording or hard-binding runtime to a vendor stack.

**Why this workstream exists**

- `docs/plans/202605081903searched` correctly identifies manual dataset selection as a real usability gap.
- The repo already has provider and market catalog surfaces, so the next step is not "replace them with OpenBB"; it is "build an artifact that uses them and can later inform them."

**Primary owner files**

- `src/application/provider_catalog.rs`
- `src/application/orchestration/workflow_status.rs`
- `src/market_catalog/mod.rs`
- `src/application/data_sources/live_defaults.rs`
- future additive helper under `scripts/research/` such as `market_data_resolver.py`

**Execution contract**

1. Start external-only.
2. Build a deterministic artifact set:
   - `data_catalog.json`
   - `symbol_resolution.json`
   - optional `normalized_dataset.parquet`
   - required repo-native companion `normalized_dataset_summary.json`
3. Use current repo concepts first:
   - market key
   - provider capability
   - alias / symbol normalization
   - explicit date range / bar count / timeframe coverage
4. Keep public CLI wording generic:
   - "dataset available"
   - "resolution ready"
   - "provider candidates"
   - not `OpenBB`, `FIGI`, or vendor names unless the user explicitly drills into details
5. Only consider runtime code changes after the helper produces stable artifacts that a consumer can actually reuse.

**Exit gate**

- A fresh `/tmp/...` run can generate deterministic symbol-resolution and dataset-catalog artifacts from existing repo/provider inputs.
- `workflow-status` and `provider-status` remain generic and token-friendly.
- No runtime dependency on a research Python library is introduced just to prove the artifact shape.

**Current implementation**

- `scripts/research/market_data_resolver.py` now materializes the Bundle A artifact family directly from:
  - `config/market_data_harness_presets.json`
  - `config/market_relationships.json`
  - optional `examples/provider_profiles/*.json`
- The helper keeps the default lane generic and zero-config:
  - market selector or alias in
  - provider-neutral `symbol_resolution.json` / `data_catalog.json` / `normalized_dataset_summary.json` out
- The helper also supports a hot-pluggable personal lane:
  - `--profile thrill3r_nq_closed_loop_v1` reuses the existing personal provider-profile contract
  - no default CLI/runtime surface changes are required for consumers who do not opt into the personal lane
- `workflow-status` now provides one generic runtime readback for the lane:
  - profile-aware agent output exposes `dataset_resolution_line`
  - profile-less output keeps the zero-config default plus optional personal-lane discovery
  - human output stays shorter and continues to surface the optional personal lane instead of a longer artifact contract dump

**Verification floor**

- `cargo run -- provider-status --compact`
- `cargo run -- provider-status --agent`
- `cargo run -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-bundle-a --human`
- `cargo test provider_catalog --lib -- --nocapture`
- helper-script compile/smoke check once the external helper exists
- `cargo test --lib agent_and_human_workflow_status_views_expose_dataset_resolution_line -- --nocapture`
- `cargo test --lib generic_workflow_status_views_expose_zero_config_dataset_resolution_line -- --nocapture`

**Latest evidence**

- `python3 -m unittest scripts.research.tests.test_market_data_resolver`
- `python3 -m py_compile scripts/research/market_data_resolver.py scripts/research/tests/test_market_data_resolver.py`
- `python3 scripts/research/market_data_resolver.py --repo-root . --market NQ --profile thrill3r_nq_closed_loop_v1 --output-dir /tmp/ict-engine-bundle-a --timeframe 15m --timeframe 1h --bar-count 1200`
- `./target/debug/ict-engine provider-status --compact`
- `./target/debug/ict-engine provider-status --agent`
- `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-bundle-a-workflow --human`
- `cargo test --lib agent_and_human_workflow_status_views_expose_dataset_resolution_line -- --nocapture`
- `cargo test --lib generic_workflow_status_views_expose_zero_config_dataset_resolution_line -- --nocapture`
- `cargo test --lib human_workflow_status_view_exposes_dataset_resolution_line -- --nocapture`
- `./target/debug/ict-engine workflow-status --symbol NQ --state-dir /tmp/ict-engine-bundle-a-workflow --profile thrill3r-nq-closed-loop-v1 --output-format agent`

## Workstream 2: Bundle B Calibration / Delayed-Truth Reports

**Objective:** package existing path-calibration, off-policy, and delayed-reward signals into explicit review artifacts before introducing any new model family into runtime.

**Why this workstream exists**

- The searched doc is directionally right about OPE, delayed reward, changepoints, and HMM regime truth.
- The repo already stores much of this signal; the missing piece is a canonical export/report contract that makes the evidence reviewable and reusable.

**Primary owner files**

- `src/application/orchestration/structural_playbook.rs`
- `src/application/orchestration/workflow_status.rs`
- `src/application/entry_models/training_export.rs`
- `src/application/regime/persistence.rs`
- `scripts/research/hmm_numeric_trainer.py`
- `scripts/auto_quant_external/external_regime_hmm_viterbi.py`
- `scripts/auto_quant_external/external_regime_changepoint_labels.py`
- future additive helpers under `scripts/research/`

**Execution contract**

1. Do not re-implement IPS / SNIPS / DR / hazard / censoring math inside new runtime branches until the report contract proves what is already present.
2. First additive report targets:
   - `policy_correction_report.json`
   - `ope_ci.json`
   - `path_confidence_bounds.json`
   - `duration_posterior.json`
   - `hazard_summary.json`
   - optional research-side `regime_posterior.parquet` only when dense analysis really needs it
3. Build these reports from existing repo truth:
   - structural feedback history
   - path-ranking target rows/history
   - propensity and calibration fields
   - delayed-reward counters and hazard summaries
   - HMM numeric artifacts and external changepoint labels
4. Treat external libraries such as `OBP`, `VW`, `lifelines`, `scikit-survival`, and `ruptures` as helper implementations, not runtime architecture.
5. If a runtime readback is later justified, make it a compact status or artifact reader, not a new trainer embedded into the CLI hot path.

**Exit gate**

- A user can inspect one explicit report file per concern instead of reconstructing the picture from nested status surfaces.
- Runtime still consumes only explicit artifacts and compact summaries.
- No claim of "better calibration" is accepted without a fresh `/tmp/...` artifact and command evidence.

**Current implementation**

- `scripts/research/policy_truth_reports.py` now packages the existing repo surfaces into reviewable artifacts:
  - `policy-training-status --output-format json`
  - `workflow-status --phase structural-validation-summary --output-format json`
  - `workflow-status --phase structural-temporal-summary --output-format json`
  - `workflow-status --phase structural-recommended-path-bundle --output-format json`
- The helper emits:
  - `policy_correction_report.json`
  - `ope_ci.json`
  - `path_confidence_bounds.json`
  - `duration_posterior.json`
  - `hazard_summary.json`
- This keeps the current runtime boundary intact:
  - existing Rust surfaces stay authoritative
  - the new review artifacts are external packaging, not a second trainer or a duplicate runtime status path
- Minimal runtime readback is now explicitly locked to current owners:
  - `policy-training-status` remains the top-level compact readiness/validation summary
  - `workflow-status` structural phases remain the canonical low-token readback for:
    - validation / OPE summary
    - temporal / hazard summary
    - recommended-path / confidence context
  - do not add a second calibration CLI surface unless these owners prove insufficient on a real downstream consumer slice

**Current implementation**

- `scripts/research/factor_candidate_pack.py` now packages Auto-Quant manifest evidence into the white-box candidate pack:
  - `factor_expression.json`
  - `factor_eval_grid_summary.json`
  - `transfer_score.json`
- The helper accepts an optional candidate-spec JSON for explicit expression / resonance metadata, but it can fall back to the manifest itself when no separate spec exists.
- The pack is intentionally review-first:
  - breadth matrix
  - trade-density label
  - resonance stack
  - cross-market transfer score
  - all remain explicit JSON artifacts rather than hidden runtime state

**Verification floor**

- `cargo test --lib export_structural_path_ranking_target_from_state_dir -- --nocapture`
- `cargo test --lib structural_path_ranking_target_training_status -- --nocapture`
- `cargo test --lib load_or_init_hmm_params_with_numeric_artifact_prefers_artifact_when_valid -- --nocapture`
- `python3 -m py_compile scripts/research/hmm_numeric_trainer.py`
- `python3 -m py_compile scripts/auto_quant_external/external_regime_hmm_viterbi.py scripts/auto_quant_external/external_regime_changepoint_labels.py`
- `./target/debug/ict-engine policy-training-status --symbol DEMO --state-dir /tmp/ict-engine-bundle-b --human`

**Latest evidence**

- `python3 -m unittest scripts.research.tests.test_policy_truth_reports`
- `python3 -m py_compile scripts/research/policy_truth_reports.py scripts/research/tests/test_policy_truth_reports.py`
- `./target/debug/ict-engine policy-training-status --symbol DEMO --state-dir /tmp/ict-engine-bundle-b/state --output-format json > /tmp/ict-engine-bundle-b/input/policy_status.json`
- `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-bundle-b/state --phase structural-validation-summary --output-format json > /tmp/ict-engine-bundle-b/input/validation_summary.json`
- `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-bundle-b/state --phase structural-temporal-summary --output-format json > /tmp/ict-engine-bundle-b/input/temporal_summary.json`
- `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-bundle-b/state --phase structural-recommended-path-bundle --output-format json > /tmp/ict-engine-bundle-b/input/recommended_path.json`
- `python3 scripts/research/policy_truth_reports.py --symbol DEMO --policy-status-json /tmp/ict-engine-bundle-b/input/policy_status.json --validation-summary-json /tmp/ict-engine-bundle-b/input/validation_summary.json --temporal-summary-json /tmp/ict-engine-bundle-b/input/temporal_summary.json --recommended-path-json /tmp/ict-engine-bundle-b/input/recommended_path.json --output-dir /tmp/ict-engine-bundle-b/reports`
- `python3 -m unittest scripts.research.tests.test_factor_candidate_pack`
- `python3 -m py_compile scripts/research/factor_candidate_pack.py scripts/research/tests/test_factor_candidate_pack.py`
- `python3 scripts/research/factor_candidate_pack.py --manifest-json /tmp/ict-engine-bundle-c/strategy_library.json --strategy-name TrendPullbackDense15m --candidate-spec-json /tmp/ict-engine-bundle-c/candidate_spec.json --autoresearch-status-json /tmp/ict-engine-bundle-c/autoresearch_status.json --output-dir /tmp/ict-engine-bundle-c/out`

## Workstream 3: Bundle C White-Box Factor Artifact Contract

**Objective:** define the missing white-box factor artifact contract that lets external factor generators feed the existing factor board and then hand off worthwhile candidates to the runtime-closure board.

**Why this workstream exists**

- The searched doc correctly pushes toward interpretable factor generation, breadth, and transfer checks.
- The repo already has the factor board, path-ranking target contract, and runtime-closure board; what is missing is the artifact contract that joins those pieces cleanly.

**Primary owner files**

- `docs/plans/2026-05-05-execution-tree-factor-auto-quant-todo.md`
- `docs/plans/2026-05-07-auto-quant-post-factor-runtime-closure-todo.md`
- `scripts/auto_quant_external/pandas_path_ranker_trainer.py`
- `scripts/auto_quant_external/path_ranker_integration.py`
- `scripts/research/path_rule_trainer.py`
- future additive white-box helper under `scripts/auto_quant_external/` or `scripts/research/`

**Execution contract**

1. Keep factor discovery on the existing factor board.
2. Add a generic artifact contract for white-box candidates:
   - `factor_expression.json`
   - optional `factor_eval_grid.parquet`
   - required `factor_eval_grid_summary.json`
   - `transfer_score.json`
3. Required artifact fields should support the current factor board, not invent a new loop:
   - expression or rule text
   - operator set / complexity
   - target market / timeframe hypothesis
   - breadth matrix
   - trade-density summary
   - resonance summary
   - transfer / cross-market result
   - whether the candidate is regime-only, execution-only, or mixed
4. Once a candidate pack becomes worthwhile, do not continue work here. Move to `2026-05-07-auto-quant-post-factor-runtime-closure-todo.md`.
5. Path-ranking direct-model artifacts, external score application, and execution-tree before/after evidence remain runtime-closure work, not factor-board work.

**Exit gate**

- The factor board can reference one explicit factor artifact family instead of a loose pile of experiment outputs.
- Runtime closure is triggered only by candidate packs that already passed breadth / trade-density / resonance rules.
- No runtime code is reopened just to keep factor generation moving.

**Verification floor**

- `cargo run -- factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-bundle-c --backend auto-quant --human`
- `cargo run -- factor-autoresearch-status --symbol DEMO --state-dir /tmp/ict-engine-bundle-c --latest-only`
- `python3 -m unittest scripts.auto_quant_external.tests.test_path_ranker_hotplug`
- `./target/debug/ict-engine export-structural-path-ranking-target --symbol DEMO --state-dir /tmp/ict-engine-bundle-c`

## Ordered Execution Checklist

1. Read `docs/plans/202605081903searched` only as source material, not as the implementation contract.
2. Decide which bundle is actually blocking repo progress:
   - Bundle A for user bootstrap and dataset friction
   - Bundle B for calibration / truth quality
   - Bundle C for factor throughput and candidate packaging
3. Confirm the slice does not already belong to an existing authoritative board.
4. Start with an additive helper or artifact export if the current gap is still "prove the shape."
5. Use `/tmp/...` state dirs and real command evidence before proposing runtime code changes.
6. If the slice proves value and only then reveals a missing runtime read surface, reopen the smallest owner file that can read or summarize the artifact.
7. Update this board plus whichever authoritative downstream board the slice actually touched.

## Success Standard

This board is successful only if all of the following are true:

- The searched Bundle A/B/C material is fully mapped to concrete repo owners and existing boards.
- Every recommended next slice is expressed as an explicit artifact-first lane, not a vague literature direction.
- The factor-discovery board and runtime-closure board remain separate.
- Out-of-scope heavyweight systems and black-box runtime proposals are explicitly rejected or deferred.
- A future implementer can pick one workstream from this file and know:
  - which files own it
  - which existing surfaces it must respect
  - which commands prove it
  - which board it must update when done
