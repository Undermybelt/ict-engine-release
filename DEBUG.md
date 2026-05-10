# 2026-05-08 Whole Repo Audit Debug Trail

- Reproduction baseline:
  - `cargo clippy --all-targets -- -D warnings`
- Current evidence:
  - Clippy fails on release-quality issues across provider catalog, market_state modules, auto_quant workspace profile helpers, orchestration helpers, reporting payload builders, and belief_core helper signatures.
  - This is the first blocking audit layer before broader CLI smoke and user-surface verification.
- Immediate hypothesis:
  - Some findings are straightforward hygiene regressions (`unused import`, `redundant field names`, `manual_strip`, `iter_cloned_collect`).
  - Some findings are structural pressure signals (`too_many_arguments`, dead code / unused helper surfaces) and need selective refactor rather than blind `allow`.
- Next actions:
  - Fix deterministic low-risk clippy violations first.
  - Re-run clippy.
  - Then run focused CLI smoke and record any remaining functional gaps.

## 2026-05-08 Round 2

- Reproduction:
  - Re-ran `cargo clippy --all-targets -- -D warnings` after first low-risk cleanup pass.
- Current reduced failure set:
  - `market_state/config.rs`: `available_profiles`, `UserWeightsTemplate`, `template`, `apply_to` are public but not exported/used outside local tests.
  - `market_state/liquidity.rs`: `detect_session` is public but not exported/used.
  - structural/reporting helpers still fail `clippy::too_many_arguments`:
    - `build_structural_path_plan_artifact_with_runtime_context_and_prior_state`
    - `emit_workflow_status_output`
    - `build_factor_backtest_output_payload`
    - `weighted_seed_beta_update`
    - `structural_path_ranking_target_export_summary`
    - `build_structural_temporal_summary_artifact`
    - `apply_structural_delayed_reward_feedback`
- Root-cause ranking:
  1. `market_state` dead-code is a boundary/export gap: public hot-pluggable helpers exist but the module only re-exports a subset.
  2. Remaining clippy failures are signature-design debt, not random bugs; they need input/context structs at the boundary instead of suppression.
- Next actions:
  - Export/legitimize the intended `market_state` hot-plug helpers.
  - Introduce small input structs for the public/near-public `too_many_arguments` functions, starting with workflow/reporting surfaces.

## 2026-05-08 Round 3

- Progress since Round 2:
  - Cleared deterministic clippy hygiene items across provider catalog, market_state behavior/liquidity/structure, auto_quant workspace profile, reporting helper plumbing, and assorted tests.
  - Re-exported `market_state` hot-plug helpers so the public API matches intended usage instead of tripping dead-code.
  - Refactored these wide signatures to input/context structs:
    - `build_factor_backtest_output_payload`
    - `emit_workflow_status_output`
    - `weighted_seed_beta_update`
    - `build_structural_temporal_summary_artifact`
    - `structural_path_ranking_target_export_summary`
    - `update_structural_policy_correction_stats`
    - `EnhancedAggregator::aggregate`
    - `MarketStateClassifier::aggregate_regimes`
- Current state:
  - A single whole-repo `cargo clippy --all-targets -- -D warnings` process is still running; do not start parallel clippy jobs.
  - Next action depends on its final remaining error set.

## 2026-05-08 Round 4

- Final whole-repo gate:
  - `cargo clippy --all-targets -- -D warnings`
  - result: pass
- Additional real-surface fixes landed after structural cleanup:
  - `workflow-status --human` no longer shows the NQ-specific personal provider lane for unrelated symbols like `NEWSYM`.
  - `factor-autoresearch-status` empty-state now returns an `ask-user` contract with explicit `--data` follow-up instead of an incomplete command.
- New repo-local artifact created:
  - `docs/audits/2026-05-08-whole-repo-limitations-and-keywords.md`
