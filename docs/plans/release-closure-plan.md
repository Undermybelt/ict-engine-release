# Release Closure Plan

Goal: productize the current research system so users can trust conclusions, inspect bottlenecks, run the right scripts, and avoid paired-data traps.

Priority order:
1. `research-verdict`
2. `evidence-quality-breakdown`
3. script family convergence + archive legacy scripts
4. paired-data admission gate / quality report
5. XGBoost skipped by user decision

## 1. research-verdict

Purpose:
- turn scattered experimental truth into one machine-readable verdict surface

Inputs:
- autoresearch status
- local search results
- cluster results
- final artifacts
- contamination signals
- best attempt

Output fields:
- `best_known_baseline`
- `proven_bad_regions`
- `current_bottleneck`
- `recommended_next_experiment`
- `stop_or_continue`
- `comparison_contaminated`

Implementation sketch:
- add CLI subcommand in `src/main.rs`
- read JSON result/state files from one or more dirs
- synthesize short verdict JSON only

## 2. evidence-quality-breakdown

Purpose:
- expose the exact composition of `evidence_quality_score`

Output fields:
- `base_score`
- `support_gap_contribution`
- `uncertainty_penalty`
- `directional_conflict_penalty`
- `mixed_alignment_penalty`
- `mtf_direction_conflict_penalty`
- `mtf_alignment_penalty`
- `mtf_alignment_bonus`
- `mtf_entry_penalty`
- `liquidity_penalty_or_bonus`
- `final_evidence_quality_score`
- `hard_pass_gap`
- `neutralized_gap`

Implementation sketch:
- add pure helper in `src/config.rs` or adjacent module
- thread into CLI command surface via `factor-pipeline-debug` helper or dedicated subcommand

## 3. Script family convergence

Target public script families:
- `scripts/search_local.py`
- `scripts/search_cluster.py`
- `scripts/evaluate_bottleneck.py`

Archive old scripts to:
- `scripts/archive/`

Mapping:
- `factor_local_search_v2*.py` -> `search_local.py`
- `factor_cluster_jump_v2.py`, `cross_market_smt_focus.py` -> `search_cluster.py`
- `factor_expansion_preview*.py`, `pre_bayes_policy_tuning.py`, `physics_feature_ablation_v1.py` -> `evaluate_bottleneck.py`
- summary scripts can become helper modes or archive

## 4. paired-data admission gate / quality report

Purpose:
- stop invalid SMT / paired runs before scoring

Output fields:
- `paired_market_quality`
- `aligned_length`
- `primary_length`
- `paired_length`
- `overlap_ratio`
- `safe_lookback`
- `status = valid | invalid_due_to_pair_quality | valid_but_flat`
- `reason`

Implementation sketch:
- add helper near cross-market SMT factor evaluation path
- return structured quality info alongside signal generation
- surface in debug/report output

## Verification

After implementation:
- `cargo fmt`
- `cargo check`
- targeted tests for new commands/helpers
- smoke run for:
  - `research-verdict`
  - `evidence-quality-breakdown`
  - paired-data quality path

## Constraints
- Keep changes mechanical and observable
- Prefer short JSON outputs over prose blobs
- Archive rather than delete old scripts where possible
