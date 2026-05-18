# 2026-05-08 Factor Iteration Regime Bundle Handoff Todo

## TaskIntentDraft

- Continue `support/docs/plans/2026-05-08-factor-iteration-filter-belief-catboost-execution-tree-board.md`.
- Keep the public surface zero-config and consumer-usable.
- Make user-specific regime benchmark evidence opt-in and hot-pluggable rather than default-coupled.
- Stay inside additive factor-iteration helper artifacts; do not reopen runtime ingestion.

## BaselineReadSetHint

- `support/docs/plans/2026-05-08-factor-iteration-filter-belief-catboost-execution-tree-board.md`
- `support/docs/factor-artifact-naming-contract.md`
- `config/factor_candidate_harness_presets.json`
- `support/examples/factor_candidate_profiles/thrill3r-nq-auto-quant-v1.json`
- `support/scripts/research/factor_candidate_resolver.py`
- `support/scripts/research/regime_artifact_bundle.py`

## ImpactStatementDraft

- The remaining gap is no longer factor-pack plumbing or regime-bundle wiring.
- The merged baseline line now contains the zero-config registry path, the opt-in personal evidence lane, the regime-only bundle helper, and the recovered Family A breadth packs together.
- If this lane reopens, it should reopen for fresh factor evidence generation or a newly selected family / market / timeframe cell, not for more registry plumbing.

## TodoCheckpointDraft

- Current todo:
  - [x] Audit the board, resolver, preset, profile, and existing regime bundle script.
  - [x] Move concrete regime benchmark paths out of the generic preset surface.
  - [x] Teach resolver/build flow to emit regime artifact bundles from benchmark JSON inputs.
  - [x] Verify generic zero-config vs opt-in profile behavior with real commands.
  - [x] Update the authoritative board with verified regime-lane status.
  - [x] Make corrupted `freqtrade` reusable inputs fail closed instead of crashing the build flow.
  - [x] Continue the Family A breadth lane with one more real explicit candidate pack.
  - [x] Continue the Family A breadth lane with the first 5m timeframe-coverage pack.
  - [x] Continue the Family A breadth lane with the historical 15m and 1d-regime lanes.
  - [x] Continue the Family A breadth lane with the historical 1m lane.
  - [x] Continue the Family A breadth lane with the historical ES lane.
  - [x] Continue the Family A breadth lane with the historical EUR lane.
  - [x] Record the broader-market failure boundaries for YM and XAU.
  - [x] Re-verify the full candidate-registry slice on the newer hot-plug baseline line after integration.
- Active slice:
  - merged integration audit complete; no remaining registry / hot-plug slice is active
- Completed todos:
  - routing complete
  - isolated worktree selected
  - existing branch drift audited before edits
  - generic regime preset de-personalized
  - opt-in profile restored as the only owner of local benchmark JSON paths
  - resolver now emits regime artifact bundles from benchmark JSON inputs
  - board writeback complete
  - zip integrity now gates `artifact_ready` for `freqtrade_backtest_zip`
  - `family_a_fvg_retrace_1h_v1` is now a real profile-backed candidate pack, not a board-only idea
  - `family_a_fvg_retrace_5m_v1` is now a real profile-backed candidate pack, not a board-only idea
  - `strategy_library_json` is now a supported reusable input kind for old explicit evidence
  - `family_a_killzone_breakout_15m_v1` and `family_a_killzone_breakout_1d_regime_v1` are now real profile-backed candidate packs, not board-only notes
  - `family_a_killzone_breakout_1m_v1` is now a real profile-backed candidate pack, not a board-only note
  - `family_a_es_killzone_breakout_1h_v1` is now a real profile-backed candidate pack, not a board-only note
  - `family_a_eur_killzone_breakout_1h_v1` is now a real profile-backed candidate pack, not a board-only note
  - `YM` and `XAU` are explicitly marked as non-promotable family boundaries on this board
  - the full factor-candidate registry slice now replays cleanly on the newer hot-plug baseline worktree
- Next step:
  - if a future slice reopens this lane, begin from new factor evidence generation or a newly chosen family / market / timeframe cell; do not reopen candidate-registry or regime-bundle infrastructure unless a genuinely new artifact kind is introduced

## EvidenceBundleDraft

- `git status` in isolated worktree already showed this line owns:
  - `config/factor_candidate_harness_presets.json`
  - `support/scripts/research/regime_artifact_bundle.py`
  - `support/scripts/research/tests/test_regime_artifact_bundle.py`
- Real local regime benchmark JSONs currently exist under:
  - `/tmp/ict-engine-ibkr-probe/regime_factor_benchmark.*.json`
- Focused verification now completed:
  - `python3 -m unittest support.scripts.research.tests.test_regime_artifact_bundle support.scripts.research.tests.test_factor_candidate_resolver`
  - generic resolver run:
    - `python3 support/scripts/research/factor_candidate_resolver.py --repo-root . --output-dir /tmp/ict-engine-factor-candidate-registry-generic-20260508`
    - result: `selection_mode=generic_zero_config`, `buildable_count=0`, `built_pack_count=0`
  - opt-in resolver run:
    - `python3 support/scripts/research/factor_candidate_resolver.py --repo-root . --profile thrill3r_nq_auto_quant_v1 --build-packs --output-dir /tmp/ict-engine-factor-candidate-registry-profile-20260508`
    - result: `selection_mode=profile_opt_in`, `buildable_count=6`, `built_pack_count=6`
  - generated regime bundle snapshot:
    - `covered_markets=NQ,SPY,QQQ,GLD`
    - `best_market=GLD`
    - `best_eval_macro_f1=0.478629`
    - `average_eval_macro_f1=0.448097`
    - `best_transition_f1=0.074074`
  - new negative-path verification:
    - `python3 -m unittest support.scripts.research.tests.test_factor_candidate_resolver.FactorCandidateResolverTests.test_build_candidate_registry_marks_invalid_freqtrade_zip_unbuildable`
    - `python3 -m unittest support.scripts.research.tests.test_factor_candidate_resolver.FactorCandidateResolverTests.test_build_candidate_packs_skips_invalid_freqtrade_zip`
    - both now pass
  - real local proof point:
    - `/Users/thrill3r/Auto-Quant/user_data/backtest_results/backtest-result-2026-05-08_23-11-33.zip` fails `unzip -t`
    - this file would previously be misclassified as buildable; now it would be surfaced as `invalid_artifact:...`
  - Family A breadth inventory check:
    - scanned local `Auto-Quant/user_data/backtest_results/*.zip` for:
      - `TomacNQKillzoneBreakout5m`
      - `TomacNQKillzoneBreakout15m`
      - `TomacNQ_KillzoneBreakout1dRegime`
      - `TomacNQ_RegimeFVGRetrace`
      - `TomacNQ_RegimeFVGRetrace5m`
    - current result: no valid reusable backtest zips found for those variants
    - implication: the next Family A breadth slice is evidence-generation first, not registry-only bookkeeping
  - New Family A breadth evidence generated this turn:
    - `TomacNQ_RegimeFVGRetrace` base `NQ/USD` 8Y run:
      - command: `uv run --with ta-lib python .../run_tomac_one.py TomacNQ_RegimeFVGRetrace 1h /tmp/ict-engine-family-a-fvg-retrace-nq-export.json NQ/USD 20180101-20251231`
      - result: `12` trades, `sharpe=0.015`, `profit_factor=1.92`, `total_profit_pct=0.57`
      - reusable zip: `backtest-result-2026-05-08_23-46-20.zip`
    - `TomacNQ_RegimeFVGRetrace` cross-market 1Y run:
      - command: `uv run --with ta-lib python .../run_tomac_one.py TomacNQ_RegimeFVGRetrace 1h /tmp/ict-engine-family-a-fvg-retrace-cross-export.json SPY/USD,IWM/USD,GLD/USD 20250507-20251231`
      - result: `23` trades aggregate, `sharpe=0.075`, `profit_factor=1.10`
      - per-market:
        - `SPY/USD`: `11` trades, `sharpe=0.377`, `profit_factor=2.68`
        - `IWM/USD`: `2` trades, anecdotal
        - `GLD/USD`: `10` trades, `sharpe=-0.195`, `profit_factor=0.58`
      - reusable zip: `backtest-result-2026-05-08_23-47-45.zip`
    - registry/profile were updated and resolver now emits `family_a_fvg_retrace_1h_v1`
    - `TomacNQ_RegimeFVGRetrace5m` base `NQ/USD` 8Y run:
      - command: `uv run --with ta-lib python .../run_tomac_one.py TomacNQ_RegimeFVGRetrace5m 5m /tmp/ict-engine-family-a-fvg-retrace-5m-nq-export.json NQ/USD 20180101-20251231`
      - result: `82` trades, `aggregate_label=preferred_density`, `sharpe=-0.0199`, `profit_factor=0.8399`, `total_profit_pct=-0.47`
      - reusable zip: `backtest-result-2026-05-08_23-55-11.zip`
    - registry/profile were updated and resolver now emits `family_a_fvg_retrace_5m_v1`
  - Historical Family A strategy-library artifacts recovered this turn:
    - `strategy_library_json` reusable-input support added to resolver
    - `TomacNQKillzoneBreakout15m`
      - source artifact: `/tmp/ict-engine-family-a-nq-15m-profile/.deps/auto-quant/strategy_library_15m.json`
      - result: `22` trades, `aggregate_label=probe_only`, `sharpe=0.0746`, `profit_factor=1.1272`
      - registry/profile now emit `family_a_killzone_breakout_15m_v1`
    - `TomacNQ_KillzoneBreakout1dRegime`
      - source artifact: `/tmp/ict-engine-family-a-profile-1dregime-check/.deps/auto-quant/strategy_library_round3.json`
      - result: `2` trades, `aggregate_label=anecdotal`, `sharpe=0.4468`, `total_profit_pct=2.26`
      - registry/profile now emit `family_a_killzone_breakout_1d_regime_v1`
    - `TomacNQKillzoneBreakout1m`
      - source artifact: `/tmp/ict-engine-family-a-nq-1m-profile/.deps/auto-quant/strategy_library_1m.json`
      - result: `56` trades, `aggregate_label=thin`, `sharpe=-0.3518`, `profit_factor=0.6742`, `total_profit_pct=-8.2`
      - registry/profile now emit `family_a_killzone_breakout_1m_v1`
    - `TomacKillzoneBreakout` on `ES`
      - source artifact: `/tmp/ict-engine-family-a-es-profile/ES/auto_quant_strategy_library.json`
      - result: `40` trades, `aggregate_label=thin`, `sharpe=0.2889`, `profit_factor=2.1103`, `total_profit_pct=16.98`
      - registry/profile now emit `family_a_es_killzone_breakout_1h_v1`
    - `TomacKillzoneBreakout` on `EUR`
      - source artifact: `/tmp/ict-engine-family-a-eur-profile/EUR/auto_quant_strategy_library.json`
      - result: `6` trades, `aggregate_label=anecdotal`, `sharpe=-0.0459`, `profit_factor=0.6891`, `total_profit_pct=-0.37`
      - registry/profile now emit `family_a_eur_killzone_breakout_1h_v1`
  - Merged-line completion audit on the newer hot-plug baseline worktree:
    - worktree:
      - `/Users/thrill3r/.config/aegis/worktrees/ict-engine/feature-factor-iteration-pack-20260509`
    - verification:
      - `python3 -m unittest support.scripts.research.tests.test_factor_candidate_pack support.scripts.research.tests.test_factor_candidate_resolver support.scripts.research.tests.test_regime_artifact_bundle`
      - result: `15` tests, `OK`
    - generic merged resolver run:
      - `python3 support/scripts/research/factor_candidate_resolver.py --repo-root . --output-dir /tmp/ict-engine-factor-candidate-registry-generic-20260509-merged2`
      - result: `selection_mode=generic_zero_config`, `candidate_count=13`, `buildable_count=0`, `built_pack_count=0`
    - opt-in merged resolver run:
      - `python3 support/scripts/research/factor_candidate_resolver.py --repo-root . --profile thrill3r_nq_auto_quant_v1 --build-packs --output-dir /private/tmp/ict-engine-factor-candidate-registry-profile-20260509-merged`
      - result: `selection_mode=profile_opt_in`, `candidate_count=13`, `buildable_count=13`, `built_pack_count=13`
    - merged candidate-pack audit:
      - all `12` execution entries emit:
        - `factor_expression.json`
        - `factor_eval_grid_summary.json`
        - `transfer_score.json`
      - every emitted execution pack carries the required mapping keys:
        - `pre_bayes_targets`
        - `belief_targets`
        - `path_ranking_targets`
        - `execution_tree_targets`
        - `structural_feedback_required`
      - `regime_primary_gate_pending_v1` emits the full regime-only bundle:
        - `regime_classifier_summary.json`
        - `transition_summary.json`
        - `resonance_summary.json`
        - `cross_market_summary.json`

## DriftCheckDraft

- Scope:
  - still inside factor-iteration helper artifacts only
- Compatibility:
  - public default surface stays generic
  - personal data remains opt-in
- Retirement:
  - no new runtime fallback or compatibility alias introduced
- Decision:
  - pause-for-user
  - current registry / hot-plug slice is complete; reopen only for new factor evidence generation or a newly chosen family cell

## ResumeStateHint

- Re-read this handoff todo plus the board before further edits.
- Re-run resolver in both modes:
  - generic zero-config
  - `--profile thrill3r_nq_auto_quant_v1 --build-packs`
- If the generic path still exposes concrete `/tmp` benchmark JSONs, the slice regressed.
- If a future profile selects a broken backtest zip, the lane must stay
  `artifact_ready=false`; do not "fix" this by catching the crash later in pack
  construction.
- If starting the next slice, begin from new factor evidence generation rather than reopening the regime lane unless new shared benchmark artifacts appear.
- `family_a_fvg_retrace_1h_v1` already exists now; do not regenerate the same pack unless the underlying reusable zips are replaced.
- `family_a_fvg_retrace_5m_v1` already exists now; do not regenerate the same pack unless the underlying reusable zip is replaced.
- `family_a_killzone_breakout_15m_v1` and `family_a_killzone_breakout_1d_regime_v1` now exist; prefer fresh cross-market evidence or another missing variant instead of re-ingesting the same historical manifests again.
- `family_a_killzone_breakout_1m_v1` now exists; prefer another missing Family A variant over re-ingesting the same historical manifest again.
- `family_a_es_killzone_breakout_1h_v1` now exists; prefer another missing Family A variant over re-ingesting the same historical manifest again.
- `family_a_eur_killzone_breakout_1h_v1` now exists; prefer another missing Family A variant over re-ingesting the same historical manifest again.
- `YM` and `XAU` are now explicit dead-ends on this board; do not spend more Family A breadth budget there unless new reusable evidence appears outside the current profile material.
