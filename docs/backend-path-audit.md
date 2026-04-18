# Backend Path Audit

Purpose: document release-blocking hard-coded local paths still present in archived or auxiliary Python backends.

## Summary

Current public wrappers are release-safer than the archived backends they call.
The wrappers now default to help-only mode and expose non-executing backend summaries.
However, many archived or auxiliary scripts still embed machine-local absolute paths such as:
- `/Users/thrill3r/projects-ict-engine/ict-engine`
- `/Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf`
- repo-local `state/*` outputs anchored from hard-coded absolute repo roots

## Highest-impact archived backends behind public wrappers

### `scripts/archive/factor_local_search_v2d.py`
- hard-coded repo root
- hard-coded cleaned data root
- hard-coded release binary path via absolute repo root

### `scripts/archive/factor_cluster_jump_v2.py`
- hard-coded repo root
- hard-coded cleaned data root
- hard-coded release binary path via absolute repo root

### `scripts/archive/pre_bayes_policy_tuning.py`
- hard-coded repo root
- hard-coded cleaned data root
- hard-coded release binary path via absolute repo root

## Other affected scripts

Examples observed during scan:
- `scripts/archive/factor_expansion_preview_v2.py`
- `scripts/archive/physics_feature_ablation_v1.py`
- `scripts/archive/cross_market_smt_focus.py`
- `scripts/archive/factor_local_search_v2b.py`
- `scripts/build_catboost_policy_training_table.py`
- `scripts/build_catboost_policy_training_table_v4.py`
- `scripts/build_repo_bbn_cpt_init.py`
- `scripts/build_repo_bbn_cpt_init_smoothed.py`
- `scripts/build_tomac_cpt_seed_tables.py`
- `scripts/merge_tomac_entry_logic_and_build_bbn_v1.py`
- `scripts/merge_tomac_entry_logic_and_build_bbn_v2.py`

## Release implication

Current release posture should be stated honestly:
- wrapper UX: preview-release ready
- archived backend portability: not yet portable across machines without path cleanup

## Recommended next cleanup order

1. Public-wrapper backends first:
   - `factor_local_search_v2d.py`
   - `factor_cluster_jump_v2.py`
   - `pre_bayes_policy_tuning.py`
2. Shared helper extraction for:
   - repo root discovery
   - cleaned data root selection
   - binary resolution
3. Then secondary archived scripts and training-table builders.

## Preferred fix pattern

Replace hard-coded absolute paths with one of:
- `pathlib.Path(__file__).resolve().parents[...]` for repo discovery
- CLI args such as `--data-root`, `--state-dir`, `--bin`
- environment variables with clear defaults

Avoid claiming full cross-machine support until this audit list is closed.
