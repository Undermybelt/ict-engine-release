# Backend Path Audit

Purpose: document release-blocking hard-coded local paths still present in archived or auxiliary Python backends.

Status note (2026-04-24)

The highest-impact script path hard-coding called out in this audit has now been removed from the active `support/scripts/` tree.
Shared path discovery now lives in `support/scripts/path_defaults.py`, and the previously named archived backends no longer embed machine-local `/Users/...` repo/data/bin paths.

This file should now be read as historical audit context plus a follow-up sweep list for any future script additions, not as the current state of the named backends below.

## Summary

Current public wrappers are release-safer than the archived backends they call.
The wrappers now default to help-only mode and expose non-executing backend summaries.
Historically, many archived or auxiliary scripts embedded machine-local absolute paths such as:
- `/Users/thrill3r/projects-ict-engine/ict-engine`
- `/Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf`
- repo-local `state/*` outputs anchored from hard-coded absolute repo roots

## Highest-impact archived backends behind public wrappers

### `support/scripts/archive/factor_local_search_v2d.py`
- hard-coded repo root
- hard-coded cleaned data root
- hard-coded release binary path via absolute repo root

### `support/scripts/archive/factor_cluster_jump_v2.py`
- hard-coded repo root
- hard-coded cleaned data root
- hard-coded release binary path via absolute repo root

### `support/scripts/archive/pre_bayes_policy_tuning.py`
- hard-coded repo root
- hard-coded cleaned data root
- hard-coded release binary path via absolute repo root

## Other affected scripts

Examples observed during scan:
- `support/scripts/archive/factor_expansion_preview_v2.py`
- `support/scripts/archive/physics_feature_ablation_v1.py`
- `support/scripts/archive/cross_market_smt_focus.py`
- `support/scripts/archive/factor_local_search_v2b.py`
- `support/scripts/build_catboost_policy_training_table.py`
- `support/scripts/build_catboost_policy_training_table_v4.py`
- `support/scripts/build_repo_bbn_cpt_init.py`
- `support/scripts/build_repo_bbn_cpt_init_smoothed.py`
- `support/scripts/build_tomac_cpt_seed_tables.py`
- `support/scripts/merge_tomac_entry_logic_and_build_bbn_v1.py`
- `support/scripts/merge_tomac_entry_logic_and_build_bbn_v2.py`

## Release implication

Current release posture should be stated honestly:
- wrapper UX: preview-release ready
- archived backend path discovery for the named backends above: fixed
- public script family remains experiment-grade rather than a stable packaged interface

## Recommended next cleanup order

1. Keep new scripts on the shared helper path:
   - repo root discovery
   - cleaned data root selection
   - binary resolution
2. Sweep any future archived additions for new machine-local literals before release.
3. If public wrappers gain richer argparse surfaces, route those through the same helper rather than duplicating path logic.

## Preferred fix pattern

Replace hard-coded absolute paths with one of:
- `pathlib.Path(__file__).resolve().parents[...]` for repo discovery
- CLI args such as `--data-root`, `--state-dir`, `--bin`
- environment variables with clear defaults

Avoid claiming full cross-machine support until this audit list is closed.
