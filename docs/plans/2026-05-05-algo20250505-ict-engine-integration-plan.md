# algo20250505 Integration TODO

> **For Hermes/Codex:** treat this markdown as the execution contract for turning `docs/external/algo20250505.md` into real `ict-engine` implementation slices. Pick exactly one unchecked workstream at a time. Do not blend lanes. Update this same markdown after every landed slice.

**Goal:** convert the external algo/paper shortlist into low-pollution, explicit-artifact integration work that improves `ict-engine` without adding runtime black boxes.

**Architecture:** keep the repo boundary fixed at `offline trainer -> explicit artifact -> runtime read-only consume -> token-friendly status surface`. New optimizers, structure learners, and rule learners stay outside runtime. Runtime only loads validated artifacts, falls back safely when artifacts are absent, and keeps public CLI surfaces generic.

**Tech Stack:** Rust CLI, repo docs under `docs/`, Python research harnesses under `scripts/research/`, persisted artifacts under explicit `--state-dir`, existing `factor-research` / `policy-training-status` / `workflow-status` / structural path ranking surfaces.

---

## Decision Lock

These judgments are already made for this workstream. Do not reopen them unless concrete repo evidence forces a change.

- [x] Prioritize `structural path ranking` explicit rule/tree artifacts before new HMM or BBN work.
- [x] Keep HMM optimization to low-dimensional named numeric parameters only.
- [x] Keep BBN work as constrained offline structure/CPT candidate generation, not online structure learning.
- [x] Preserve the runtime boundary: no population search, annealing schedule, swarm state, or GP program synthesis inside live CLI paths.
- [x] Reject `PSO` as the default path for this repo.
- [x] Reject swarm-zoo methods (`WOA`, `GWO`, `Firefly`, `Bat`, `Cuckoo`, `ABC`) for now.
- [x] Reject unrestricted `GP` / arbitrary program artifacts for now.

## Hard Constraints

- Preserve zero-config default behavior for consumers.
- Keep public CLI wording generic and consumer-usable; do not leak repo-owned research jargon into the default surface.
- Keep `policy-training-status` and `workflow-status` token-friendly.
- Use explicit `/tmp/...` `--state-dir` values for experiments and examples.
- Do not silently reuse maintainer-local state or paths.
- Runtime consumes explicit artifacts only; it must not depend on trainer internals.
- New artifacts must be diffable, versionable, and auditable.
- If the worktree is dirty in unrelated files, isolate your slice and do not revert others' changes.

## Agent Contract

1. Read `docs/external/algo20250505.md` before changing code on this lane.
2. Treat this file as the authoritative TODO for the integration sequence.
3. Pick exactly one unchecked workstream as the active slice.
4. Before editing, identify the owner files and exact verification commands for that slice.
5. Do not add a new public top-level command unless an unchecked item explicitly requires it.
6. Prefer extending existing status/artifact surfaces over creating a second parallel pathway.
7. A slice is not done until code, targeted verification, and this markdown are all updated.
8. After a slice lands, update this same markdown rather than creating a fresh plan doc.

## Current Todo Board

### Done

- [x] Lock the repo-level boundary: `offline trainer -> explicit artifact -> runtime read-only consume`.
- [x] Lock the initial priority order:
  - `Workstream 1` structural path ranking explicit artifact
  - `Workstream 2` HMM numeric outer-loop artifact
  - `Workstream 3` BBN structure/CPT candidate artifact
- [x] Confirm the strongest existing integration anchors:
  - `src/application/entry_models/training_export.rs`
  - `src/application/orchestration/structural_playbook.rs`
  - `src/application/regime/recovery.rs`
  - `src/application/regime/persistence.rs`
  - `src/bbn/trading/topology.rs`
  - `src/bbn/trading/update.rs`
- [x] Confirm that `factor_tucker_core` already has math + persistence, so this TODO should not start by reopening Tucker plumbing.
- [x] Execute `Workstream 1 / Slice 1`: formalize and land the explicit path-ranker trainer artifact contract on top of the existing structural-path-ranking export/register/runtime surface.
  - landed contract fields on `structural_path_ranking_trainer_artifact.json`:
    - `model_family`
    - `selected_features`
    - `trained_rows`
    - `history_rows`
    - `validation_metrics`
    - `calibration_metrics`
    - `rule_list` or `tree_json`
  - added explicit family support:
    - `corels`
    - `gosdt`
    - `ga_mask_tree`
  - kept the existing trainer-manifest path as the versioned/export truth instead of creating a second manifest type
  - made runtime consume explicit rule/tree artifacts through the existing opt-in structural path-ranker runtime lane
  - updated `policy-training-status` to surface `trainer_artifact_status` so the status surface can distinguish:
    - `missing`
    - `present_validation_insufficient`
    - `runtime_eligible`
  - preserved zero-config default behavior because runtime still stays opt-in and disabled unless the artifact is explicitly registered and enabled
  - verification:
    - `cargo check`
    - `cargo test --lib policy_training_status_lists_registered_providers -- --nocapture`
    - `cargo test --lib structural_path_ranking_target_training_status_reads_summary -- --nocapture`
    - `cargo test --lib structural_path_ranking_target_training_status_reports_calibration_quality -- --nocapture`
    - `cargo test --lib structural_path_ranking_target_training_status_reports_production_validation_ready -- --nocapture`
    - `cargo test --bin ict-engine render_policy_training_status_low_token_emits_three_summary_lines -- --nocapture`
    - `cargo test --lib register_structural_path_ranking_trainer_artifact_requires_rule_or_tree_for_explicit_family -- --nocapture`
    - `cargo test --lib agent_workflow_status_can_consume_registered_explicit_rule_artifact -- --nocapture`
    - `cargo test --lib register_structural_path_ranking_trainer_artifact_writes_ready_artifact -- --nocapture`
    - `cargo test --lib runtime_status_reports_registered_direct_model_when_available -- --nocapture`
    - `cargo test --lib runtime_status_reports_registered_service_when_available -- --nocapture`
- [x] Execute `Workstream 1 / Slice 2`: add a minimal external rule/tree trainer harness under `scripts/research/` and prove artifact registration works end-to-end on `/tmp/...`.
  - landed `scripts/research/path_rule_trainer.py`
    - stdlib-only
    - consumes exported `structural_path_ranking_target.jsonl` and optional history rows
    - emits one explicit `corels` / `gosdt` / `ga_mask_tree` compatible artifact
    - falls back to current candidate proxy labels when mature labels are not yet available, and records that fallback in artifact notes
  - proved the opt-in end-to-end loop on `/tmp/ict-engine-path-ranker` without adding a new public top-level command:
    - native `factor-research` to materialize state
    - `export-structural-path-ranking-target`
    - `python3 scripts/research/path_rule_trainer.py`
    - `register-structural-path-ranking-trainer-artifact --model-family corels`
    - `enable-structural-path-ranking-runtime`
    - `policy-training-status --human`
  - verification:
    - `python3 -m py_compile scripts/research/path_rule_trainer.py`
    - `cargo test --lib register_structural_path_ranking_trainer_artifact_prefers_history_counts -- --nocapture`
    - `cargo test --lib clear_structural_path_ranking_trainer_artifact_removes_registered_artifact -- --nocapture`
    - `cargo test --lib enable_and_disable_structural_path_ranking_runtime_updates_status_surface -- --nocapture`
    - `cargo test --lib export_structural_path_ranking_target_from_state_dir_uses_persisted_snapshot_and_learning_state -- --nocapture`
    - real `/tmp` closure:
      - `./target/debug/ict-engine factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-path-ranker --backend native --human`
      - `./target/debug/ict-engine export-structural-path-ranking-target --symbol DEMO --state-dir /tmp/ict-engine-path-ranker`
      - `python3 scripts/research/path_rule_trainer.py --target-jsonl /tmp/ict-engine-path-ranker/DEMO/policy_training/structural_path_ranking_target.jsonl --history-jsonl /tmp/ict-engine-path-ranker/DEMO/policy_training/structural_path_ranking_target_history.jsonl --model-family corels --out /tmp/ict-engine-path-ranker/path-ranker-artifact.json`
      - `./target/debug/ict-engine register-structural-path-ranking-trainer-artifact --symbol DEMO --state-dir /tmp/ict-engine-path-ranker --artifact-uri /tmp/ict-engine-path-ranker/path-ranker-artifact.json --model-family corels`
      - `./target/debug/ict-engine enable-structural-path-ranking-runtime --symbol DEMO --state-dir /tmp/ict-engine-path-ranker`
      - `./target/debug/ict-engine policy-training-status --symbol DEMO --state-dir /tmp/ict-engine-path-ranker --human`
      - `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-path-ranker --agent`
- [x] Execute `Workstream 2 / Slice 1`: define the HMM numeric trainer artifact schema and integrate artifact loading/fallback into the existing MECE recovery / regime persistence lane.
  - landed `hmm_numeric_trainer_artifact.json` schema in `src/application/regime/persistence.rs`
  - required fields now include:
    - `parameter_vector`
    - `parameter_names`
    - `bounds`
    - `objective_breakdown`
    - `seed`
    - `split_id`
    - `best_iteration`
    - `source_data_hash`
    - `state_count`
  - current supported numeric knobs for runtime consume:
    - `transition_smoothing`
    - `emission_std_floor`
    - `posterior_temperature` accepted in schema and ignored safely by current HMM runtime until a later runtime consumer needs it
  - the canonical HMM loader now lives in `src/application/regime/persistence.rs`:
    - if a valid HMM numeric artifact exists, runtime uses it
    - if the artifact is absent or invalid, runtime falls back safely to existing `hmm_params.json`
    - if neither exists, runtime falls back to default `init_hmm_params(...)`
  - `main.rs` now delegates HMM artifact loading to the regime persistence lane instead of owning the fallback logic itself
  - verification:
    - `cargo check`
    - `cargo test --lib compute_rollout_segments_splits_and_measures -- --nocapture`
    - `cargo test --lib promotes_at_or_above_threshold -- --nocapture`
    - `cargo test --lib blocks_below_threshold -- --nocapture`
    - `cargo test --lib combined_gate_blocks_on_any_subgate_failure -- --nocapture`
    - `cargo test --lib load_or_init_hmm_params_with_numeric_artifact_prefers_artifact_when_valid -- --nocapture`
    - `cargo test --lib load_or_init_hmm_params_with_numeric_artifact_falls_back_to_saved_state_when_artifact_invalid -- --nocapture`
- [x] Execute `Workstream 2 / Slice 2`: add the first bounded HMM numeric outer-loop trainer harness.
  - landed `scripts/research/hmm_numeric_trainer.py`
    - stdlib-only
    - bounded candidate grid
    - emits:
      - `hmm_numeric_trainer_artifact.json`
      - `candidate_history.jsonl`
      - `replay_summary.json`
    - writes only into caller-supplied `/tmp/...` state dirs and output dirs
    - evaluates candidates through existing `ict-engine analyze` and `mece_recovery_artifact.json`
  - trainer objective is explicitly composed from repo surfaces:
    - `accuracy`
    - `macro_f1`
    - execution-validity coverage derived from `execution_validity_summary`
    - rollout segment quality derived from persisted `segments`
  - real `/tmp` proof completed at:
    - state root: `/tmp/ict-engine-hmm-trainer`
    - artifact dir: `/tmp/ict-engine-hmm-trainer-out`
  - verification:
    - `python3 -m py_compile scripts/research/hmm_numeric_trainer.py`
    - `python3 scripts/research/hmm_numeric_trainer.py --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-hmm-trainer --out /tmp/ict-engine-hmm-trainer-out/hmm_numeric_trainer_artifact.json --bin ./target/debug/ict-engine`
    - output artifacts observed:
      - `/tmp/ict-engine-hmm-trainer-out/hmm_numeric_trainer_artifact.json`
      - `/tmp/ict-engine-hmm-trainer-out/candidate_history.jsonl`
      - `/tmp/ict-engine-hmm-trainer-out/replay_summary.json`
- [x] Execute `Workstream 3 / Slice 1`: add a BBN structure-learning export contract and candidate artifact schema.
  - landed export contract:
    - `bbn_structure_learning_rows.jsonl`
    - row fields:
      - `market_regime`
      - `liquidity_context`
      - `factor_alignment`
      - `factor_uncertainty`
      - `multi_timeframe_resonance`
      - `entry_quality`
      - `trade_outcome`
  - landed candidate artifact schema:
    - `bbn_structure_candidate_artifact.json`
    - required fields:
      - `required_edges_satisfied`
      - `forbidden_edges_violated`
      - `max_parent_count`
      - `score_name`
      - `score_value`
      - `structure_edges`
      - `cpt_overrides`
      - `source_dataset_hash`
  - validation helper now enforces:
    - no unknown node vocabulary
    - no self edges
    - no parent count above `max_parent_count`
  - runtime boundary is still fixed-DAG consumer only; the new candidate artifact is review/import input, not permission for online search
  - verification:
    - `cargo check`
    - `cargo test --lib family_overlay_changes_trade_outcome_distribution -- --nocapture`
    - `cargo test --lib test_trade_evidence_with_timed_pda_summary_overrides_entry_quality -- --nocapture`
    - `cargo test --lib test_infer_trade_outcome_with_entry_quality_bias -- --nocapture`
    - `cargo test --lib validate_bbn_structure_candidate_artifact_rejects_unknown_nodes -- --nocapture`
    - `cargo test --lib validate_bbn_structure_candidate_artifact_rejects_parent_overflow -- --nocapture`
- [x] Execute `Workstream 3 / Slice 2`: add constrained BBN candidate import/review wiring without turning runtime into an online learner.
  - landed `scripts/research/bbn_structure_search.py`
    - constrained backend contract:
      - `pgmpy_hc`
      - `pgmpy_ges`
      - `bnlearn_hc`
      - `bnlearn_tabu`
      - `gobnilp_oracle`
    - when those optional research backends are unavailable locally, the script records `heuristic_fallback` in the candidate `score_name` instead of pretending runtime can learn online
  - landed review/import helpers in `src/bbn/trading/persistence.rs`:
    - `load_bbn_structure_candidate_artifact(...)`
    - `review_bbn_structure_candidate_artifact(...)`
  - updated `docs/repo-bbn-cpt-loader-notes.md` so reviewed candidates map back into the fixed-DAG loader path rather than a new runtime learner
  - verification:
    - `python3 -m py_compile scripts/research/bbn_structure_search.py`
    - sample `/tmp` candidate generation:
      - `python3 scripts/research/bbn_structure_search.py --rows-jsonl /tmp/ict-engine-bbn-rows.jsonl --out /tmp/ict-engine-bbn-candidate.json --max-parent-count 3 --backend pgmpy_hc`
    - output artifact observed:
      - `/tmp/ict-engine-bbn-candidate.json`

- [x] Decide whether `posterior_temperature` and any gate thresholds should remain schema-only for now or gain a first runtime consumer in the next HMM slice.
  - `posterior_temperature` stays schema-only for now.
  - rationale:
    - the current runtime HMM consumer only owns `transition_smoothing` and `emission_std_floor`
    - forcing a `posterior_temperature` runtime consumer now would widen the runtime contract beyond the bounded slice
    - the loader accepts the field and ignores it safely until a later explicit runtime consumer is approved

- [x] Audit whether `Workstream 1` still needs one small follow-up for explicit artifact calibration/mature-row data quality or whether the remaining gaps are intentionally deferred to better labels/history.
  - result: not a blocker for this algo lane closure
  - reason:
    - `policy-training-status` now explicitly reports `trainer_artifact_status=present_validation_insufficient`
    - fresh `/tmp` first-pass loops can still export, register, enable, and consume explicit artifacts without pretending validation is green
    - richer matured labels/history are a legitimate later-quality lane, not a schema/runtime-boundary blocker

### Next

- [x] No open implementation slice remains in this board.

### Not Yet

- [x] Final whole-plan completion audit across Workstreams 1-3, including whether the remaining `posterior_temperature` / mature-row follow-up is a blocker or an explicitly deferred lane.

## Workstream 1: Structural Path Ranking Explicit Artifact

**Objective:** make the structural path ranking lane consume a compact, explicit, externally trained rule-list or sparse-tree artifact rather than only generic external scores.

**Why this workstream is first**

- The repo already has the strongest export/register/runtime closure here.
- Consumer benefit is immediate because `policy-training-status` and `workflow-status` already summarize runtime readiness.
- This lane best matches the repo preference for explicit, small, inspectable artifacts.

**Done when**

- an explicit trainer artifact format exists for rule-list / sparse-tree rankers
- `register_structural_path_ranking_trainer_artifact` accepts and validates that format
- runtime can enable the artifact without adding a new public workflow concept
- `policy-training-status` clearly reports runtime readiness and validation sufficiency

**Read first**

- `docs/external/algo20250505.md`
- `src/policy_training_command.rs`
- `src/application/entry_models/training_export.rs`
- `src/application/orchestration/structural_playbook.rs`
- `src/application/orchestration/workflow_status.rs`
- `docs/plans/2026-05-03-repo-action-board.md`

**Owner files**

- `src/policy_training_command.rs`
- `src/application/entry_models/training_export.rs`
- `src/application/orchestration/structural_playbook.rs`
- `src/application/orchestration/workflow_status.rs`
- `scripts/research/path_rule_trainer.py`
- `docs/plans/2026-05-05-algo20250505-ict-engine-integration-plan.md`

### Slice 1: Trainer Artifact Contract

- [x] Define one explicit trainer artifact schema for path ranking:
  - required top-level fields:
    - `model_family`
    - `selected_features`
    - `trained_rows`
    - `history_rows`
    - `validation_metrics`
    - `calibration_metrics`
    - either `rule_list` or `tree_json`
  - supported `model_family` values:
    - `corels`
    - `gosdt`
    - `ga_mask_tree`
- [x] Add versioning and readiness checks to the existing trainer-manifest path rather than introducing a parallel manifest type.
- [x] Keep the runtime artifact small and diff-friendly. Do not store trainer-internal search state.
- [x] Update the status surface so `policy-training-status` can distinguish:
  - artifact missing
  - artifact present but validation-insufficient
  - artifact present and runtime-eligible

**Minimum verification**

```bash
cargo check
cargo test --lib policy_training_status_lists_registered_providers
cargo test --lib structural_path_ranking_target_training_status_reads_summary
cargo test --lib structural_path_ranking_target_training_status_reports_calibration_quality
cargo test --lib structural_path_ranking_target_training_status_reports_production_validation_ready
cargo test --bin ict-engine render_policy_training_status_low_token_emits_three_summary_lines
```

### Slice 2: External Trainer Harness And End-to-End Registration

- [x] Add `scripts/research/path_rule_trainer.py`.
- [x] Make the script consume:
  - `structural_path_ranking_target.csv`
  - `structural_path_ranking_target.jsonl`
  - optional history rows
- [x] Make the script emit one explicit artifact file compatible with Slice 1.
- [x] Add a docs/example snippet showing:
  - export target rows
  - run trainer on `/tmp/...`
  - register artifact
  - enable runtime
  - inspect `policy-training-status`
- [x] Prove the workflow works without adding a new top-level command.

**Suggested command sequence**

```bash
cargo run -- export-structural-path-ranking-target --symbol DEMO --state-dir /tmp/ict-engine-path-ranker
python3 scripts/research/path_rule_trainer.py \
  --target-jsonl /tmp/ict-engine-path-ranker/DEMO/structural_path_ranking_target.jsonl \
  --out /tmp/ict-engine-path-ranker/path-ranker-artifact.json
cargo run -- register-structural-path-ranking-trainer-artifact \
  --symbol DEMO \
  --state-dir /tmp/ict-engine-path-ranker \
  --artifact-uri /tmp/ict-engine-path-ranker/path-ranker-artifact.json \
  --model-family corels
cargo run -- enable-structural-path-ranking-runtime \
  --symbol DEMO \
  --state-dir /tmp/ict-engine-path-ranker
cargo run -- policy-training-status --symbol DEMO --state-dir /tmp/ict-engine-path-ranker --human
```

**Minimum verification**

```bash
cargo check
cargo test --lib register_structural_path_ranking_trainer_artifact_writes_ready_artifact
cargo test --lib register_structural_path_ranking_trainer_artifact_prefers_history_counts
cargo test --lib clear_structural_path_ranking_trainer_artifact_removes_registered_artifact
cargo test --lib enable_and_disable_structural_path_ranking_runtime_updates_status_surface
cargo test --lib export_structural_path_ranking_target_from_state_dir_uses_persisted_snapshot_and_learning_state
```

## Workstream 2: HMM Numeric Outer-Loop Artifact

**Objective:** make HMM/gate numeric tuning a repeatable offline artifact flow instead of hand-tuned constants.

**Done when**

- one HMM numeric trainer artifact schema exists
- the artifact records bounds, objective breakdown, seed, split, and best parameter vector
- existing regime persistence can load the artifact without requiring trainer internals
- absence of the artifact falls back safely to current defaults

**Read first**

- `docs/external/algo20250505.md`
- `src/application/regime/recovery.rs`
- `src/application/regime/persistence.rs`
- `src/domain/regime/mece_artifact.rs`
- `src/hmm/baum_welch.rs`
- `src/hmm/viterbi.rs`
- `src/factor_research_command.rs`
- `src/main.rs`

**Owner files**

- `src/application/regime/recovery.rs`
- `src/application/regime/persistence.rs`
- `src/domain/regime/mece_artifact.rs`
- `src/factor_research_command.rs`
- `src/main.rs`
- `scripts/research/hmm_numeric_trainer.py`
- `docs/plans/2026-05-05-algo20250505-ict-engine-integration-plan.md`

### Slice 1: Artifact Schema And Fallback Wiring

- [x] Define `hmm_numeric_trainer_artifact.json`.
- [x] Required fields:
  - `parameter_vector`
  - `parameter_names`
  - `bounds`
  - `objective_breakdown`
  - `seed`
  - `split_id`
  - `best_iteration`
  - `source_data_hash`
  - `state_count`
- [x] Restrict the first supported parameter set to low-dimensional numeric knobs only:
  - `transition_smoothing`
  - `emission_std_floor`
  - `posterior_temperature`
  - selected gate thresholds if they are already explicit numeric inputs
- [x] Keep state-count selection outside this trainer for the first pass.
- [x] Add loader/fallback behavior in the regime persistence lane. Runtime must not fail when the artifact is absent.

**Minimum verification**

```bash
cargo check
cargo test --lib classify_mece_recovery_combined_gate
cargo test --lib compute_rollout_segments_splits_and_measures
cargo test --lib promotes_at_or_above_threshold
cargo test --lib blocks_below_threshold
```

### Slice 2: Offline Trainer Harness

- [x] Add `scripts/research/hmm_numeric_trainer.py`.
- [x] The trainer must:
  - run only on explicit paths and explicit `/tmp/...` state dirs
  - log candidate history separately from the best artifact
  - never write repo-default `state/`
- [x] The trainer objective must be composed from existing repo surfaces:
  - `accuracy`
  - `macro_f1`
  - `execution_validity_histogram`
  - rollout segment quality
- [x] Persist:
  - `hmm_numeric_trainer_artifact.json`
  - `candidate_history.jsonl`
  - `replay_summary.json`

**Suggested command sequence**

```bash
python3 scripts/research/hmm_numeric_trainer.py \
  --symbol DEMO \
  --data examples/demo/demo-15m.json \
  --state-dir /tmp/ict-engine-hmm-trainer \
  --out /tmp/ict-engine-hmm-trainer/hmm_numeric_trainer_artifact.json
```

## Workstream 3: BBN Structure / CPT Candidate Artifact

**Objective:** create a constrained offline structure-learning lane that generates reviewable DAG/CPT candidates without moving BBN structure learning into runtime.

**Done when**

- one export contract exists for BBN structure-learning rows
- one candidate artifact schema exists for structure + CPT proposals
- the candidate can be reviewed/imported against the existing topology/CPT files
- runtime remains a fixed DAG consumer

**Read first**

- `docs/external/algo20250505.md`
- `src/bbn/trading/topology.rs`
- `src/bbn/trading/update.rs`
- `src/bbn/trading/persistence.rs`
- `docs/repo-bbn-cpt-loader-notes.md`

**Owner files**

- `src/bbn/trading/topology.rs`
- `src/bbn/trading/update.rs`
- `src/bbn/trading/persistence.rs`
- `scripts/research/bbn_structure_search.py`
- `docs/repo-bbn-cpt-loader-notes.md`
- `docs/plans/2026-05-05-algo20250505-ict-engine-integration-plan.md`

### Slice 1: Export And Candidate Schema

- [x] Define one structure-learning export contract with these columns:
  - `market_regime`
  - `liquidity_context`
  - `factor_alignment`
  - `factor_uncertainty`
  - `multi_timeframe_resonance`
  - `entry_quality`
  - `trade_outcome`
- [x] Define one candidate artifact with:
  - `required_edges_satisfied`
  - `forbidden_edges_violated`
  - `max_parent_count`
  - `score_name`
  - `score_value`
  - `structure_edges`
  - `cpt_overrides`
  - `source_dataset_hash`
- [x] Lock the first-pass constraints:
  - no new public node vocabulary
  - no runtime online edge search
  - no unrestricted parent growth

### Slice 2: Candidate Review / Import Path

- [x] Add `scripts/research/bbn_structure_search.py` for external candidate generation.
- [x] Keep the first search stack constrained to:
  - `pgmpy` HillClimb / GES with expert knowledge
  - `bnlearn` HC / Tabu with whitelist / blacklist / `maxp`
  - optional reduced-node `GOBNILP` oracle
- [x] Map reviewed candidates back into:
  - `src/bbn/trading/topology.rs`
  - `src/bbn/trading/persistence.rs`
  - `docs/repo-bbn-cpt-loader-notes.md`

**Minimum verification**

```bash
cargo check
cargo test --lib family_overlay_changes_trade_outcome_distribution
cargo test --lib test_trade_evidence_with_timed_pda_summary_overrides_entry_quality
cargo test --lib test_infer_trade_outcome_with_entry_quality_bias
```

## Ordered Execution Checklist

1. Start with `Workstream 1 / Slice 1`.
2. Do not touch HMM or BBN files while `Workstream 1 / Slice 1` is active.
3. After `Workstream 1 / Slice 1` lands, update this markdown:
   - mark the slice `[x]`
   - record the commit
   - record the exact verification commands that passed
4. Only then move to `Workstream 1 / Slice 2`.
5. Only after `Workstream 1` has a validated end-to-end artifact loop should `Workstream 2` open.
6. Keep `Workstream 3` last unless the user explicitly reprioritizes it.

## Blocked

- None yet.

If blocked later, record:

- exact file / function / command
- what assumption failed
- whether the blocker is schema, data, or runtime-boundary related

## Do Not Do

- [x] Do not add runtime `CMA-ES`, `DE`, `PSO`, `SA`, `ACO`, or GP loops.
- [x] Do not add a second public ranking framework parallel to structural path ranking.
- [x] Do not expose optimizer jargon in default consumer surfaces.
- [x] Do not let BBN candidate generation redefine repo vocabulary without review.
- [x] Do not turn `factor_tucker_core` back into the first active slice for this TODO.

## Minimum Repo Hygiene Commands

Run these before and after any active slice:

```bash
git status --short
cargo check
```

For experiment/demo commands, prefer explicit `/tmp/...` state dirs:

```bash
mkdir -p /tmp/ict-engine-algo20250505
```

## Success Standard

- [x] `Workstream 1` produces one explicit rule/tree artifact that can be exported, registered, enabled, and summarized through existing status surfaces.
- [x] `Workstream 2` produces one explicit HMM numeric artifact that can be loaded with safe fallback.
- [x] `Workstream 3` produces one explicit BBN structure/CPT candidate artifact without moving learning into runtime.
- [x] No slice breaks zero-config consumer behavior.
- [x] No slice introduces repo-default pollution or maintainer-local path leakage.
