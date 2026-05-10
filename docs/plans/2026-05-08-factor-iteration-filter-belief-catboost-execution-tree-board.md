# Factor Iteration Filter-Belief-CatBoost-Execution-Tree Board

> Authoritative execution board for factor iteration only.  
> This file is intentionally limited to the factor iteration path that must close the loop through pre-bayes filters, belief / BBN evidence, CatBoost-compatible path ranking, and execution-tree consumption.  
> Do not mix provider bootstrap, generic runtime closure, or unrelated repo UX work into this board.

**Goal:** produce factor candidate packs that can be iterated externally, evaluated across markets/timeframes, mapped explicitly into `pre_bayes -> belief / BBN -> structural path ranking -> execution tree`, and handed off only after they satisfy breadth, density, resonance, and structural-feedback requirements.

**Architecture:** keep `ict-engine` runtime generic and read-only. Factor iteration happens through external factor/strategy code and additive helper artifacts. Every promoted candidate must first exist as an explicit white-box factor pack, then prove which middle-layer surfaces it feeds, then prove breadth / density / resonance, and only then become eligible for the post-factor runtime-closure board.

**Tech Stack:** `scripts/research/factor_candidate_pack.py`, `scripts/research/path_rule_trainer.py`, `scripts/auto_quant_external/*`, `./target/debug/ict-engine factor-research --backend auto-quant`, `factor-autoresearch-status`, `workflow-status --phase structural-playbook`, `workflow-status --phase structural-recommended-path-bundle`, `policy-training-status`, explicit `/tmp/...` state dirs, JSON artifact packs.

**Hard Constraints**

- Only factor iteration content belongs here.
- No provider bootstrap, no generic workflow UX, no first-run remediation, no unrelated runtime refactors.
- Keep the runtime boundary explicit: `offline factor iteration -> explicit artifact -> runtime read-only consume`.
- Do not promote a factor family from one market or one thin trade cell alone.
- Do not treat regime F1, path-ranking readiness, or execution-tree movement as interchangeable; each layer needs its own evidence.
- Keep all promotion evidence explicit, reviewable, and `/tmp/...` isolated.
- Use `docs/factor-artifact-naming-contract.md` as the canonical interpretation layer for:
  - `board_record`
  - `reusable_input`
  - `candidate_pack`
  - `temp_state_dir`
  so board evidence is never conflated with current reusable input again.

**Layer Contract**

Every candidate pack must declare all five of these:

1. `pre_bayes_targets`
2. `belief_targets`
3. `path_ranking_targets`
4. `execution_tree_targets`
5. `structural_feedback_required`

If any of the above is missing, the candidate is incomplete and cannot close the loop.

**Required Candidate Pack Artifacts**

- `factor_expression.json`
- `factor_eval_grid_summary.json`
- `transfer_score.json`

`factor_expression.json` must carry:

- `expression_text`
- `operator_set`
- `complexity`
- `target_market_hypothesis`
- `base_timeframe`
- `context_timeframes`
- `regime_role`
- `filter_belief_execution_mapping`

`filter_belief_execution_mapping` must carry:

- `pre_bayes_targets`
- `belief_targets`
- `path_ranking_targets`
- `execution_tree_targets`
- `structural_feedback_required`

**Promotion Rule**

A candidate may only move to the post-factor runtime-closure board after all of the following are true:

- `factor_eval_grid_summary.trade_density_summary.aggregate_label` is not `invalid` or `anecdotal`
- `transfer_score.status` is not `single_market_only` unless the lane is explicitly regime-only and still inside exploration
- resonance context is explicit for the base timeframe
- middle-layer mapping is explicit
- the lane states whether structural feedback lineage is required before honest runtime validation

---

## Current Todo Board

### Done

- [x] Locked the factor iteration board to factor content only.
- [x] Locked the loop boundary to `pre_bayes -> belief / BBN -> path ranking -> execution tree`.
- [x] Locked the candidate pack minimum artifact family:
  - `factor_expression.json`
  - `factor_eval_grid_summary.json`
  - `transfer_score.json`
- [x] Extended `scripts/research/factor_candidate_pack.py` so `factor_expression.json` now carries `filter_belief_execution_mapping`.
- [x] Added regression coverage in `scripts/research/tests/test_factor_candidate_pack.py` for:
  - populated middle-layer mapping
  - empty/default middle-layer mapping
- [x] Extended `scripts/research/factor_candidate_pack.py` so the helper can now build a candidate pack directly from reusable `Freqtrade` backtest zip evidence instead of requiring a hand-authored `strategy_library.json` first.
- [x] Added regression coverage in `scripts/research/tests/test_factor_candidate_pack.py` for:
  - manifest extraction from a `Freqtrade` backtest zip
  - `--freqtrade-backtest-zip` CLI input
  - cross-market evidence merged through candidate-spec JSON
- [x] Added a generic zero-config candidate-spec registry at `config/factor_candidate_harness_presets.json`.
- [x] Added an opt-in personal evidence profile lane at `examples/factor_candidate_profiles/thrill3r-nq-auto-quant-v1.json`.
- [x] Added `scripts/research/factor_candidate_resolver.py` plus `scripts/research/tests/test_factor_candidate_resolver.py` so the board now has one additive helper that:
  - resolves generic candidate presets without personal path assumptions
  - optionally reuses local Auto-Quant backtest evidence when a profile is explicitly selected
  - emits repo-reviewable `candidate_registry.json`, `candidate_spec_index.json`, and `candidate_pack_index.json`
- [x] Added a canonical naming / evidence-interpretation layer at `docs/factor-artifact-naming-contract.md`.
- [x] Extended resolver output so every registry entry now distinguishes:
  - `evidence_status`
  - `artifact_kind`
  - `board_evidence_status`
  - `pack_build_reason`
  instead of overloading `state` / `profile` / historical board references.
- [x] Extended resolver output with machine-readable cross-layer lookup fields:
  - `board_ref`
  - `reusable_input_refs`
  - `summary.naming_contract_version`
  so a later agent can answer “is this only board evidence, or can I rebuild it right now?” without re-deriving that distinction from prose.
- [x] Built explicit candidate packs from real local evidence under `/tmp/ict-engine-factor-candidate-registry-20260508` for:
  - `family_f_vrp_compression_15m_v1`
  - `family_f_trend_pullback_dense_15m_v1`
  - `family_d_liquidity_sweep_reclaim_15m_wide_v1`
- [x] Extended the registry coverage beyond the first three execution candidates:
  - built one real Family A baseline pack from fresh local evidence:
    - `family_a_killzone_breakout_1h_v1`
  - built one real Family A displacement pack from fresh local evidence:
    - `family_a_killzone_displacement_pending_v1`
  - recorded one remaining explicit skipped-but-active lane with a machine-readable reason:
    - `regime_primary_gate_pending_v1` -> `regime_only_waiting_for_classifier`
- [x] Upgraded the Family A baseline pack from `single_market_only` to a real cross-market candidate using fresh local `Freqtrade` evidence on:
  - `NQ/USD` 8Y baseline
  - `SPY/USD` 1h local feather window
  - `IWM/USD` 1h local feather window
  - `GLD/USD` 1h local feather window
- [x] Converted the Family A displacement lane from board-only history into a real local candidate pack using:
  - repo strategy source
  - local Auto-Quant runtime strategy copy
  - fresh `NQ/USD` 8Y backtest zip
- [x] Upgraded the Family A displacement pack to real `IBKR`-backed cross-market evidence on:
  - `NQ/USD` 8Y baseline
  - `SPY/USD` 1h local feather window
  - `IWM/USD` 1h local feather window
  - `GLD/USD` 1h local feather window
- [x] Verified the Family A displacement pack is now a real `cross_market_candidate` with mixed-but-positive off-NQ breadth:
  - `aggregate_trade_count=147`
  - `aggregate_label=preferred_density`
  - `transfer_score.status=cross_market_candidate`
  - `covered_markets=NQ/USD,SPY/USD,IWM/USD,GLD/USD`
  - `SPY/USD`: `sharpe=0.23`, `profit_factor=1.34`, `trade_count=22`
  - `IWM/USD`: `sharpe=0.17`, `profit_factor=1.18`, `trade_count=27`
  - `GLD/USD`: `sharpe=0.61`, `profit_factor=2.15`, `trade_count=24`
  - but `NQ/USD` 8Y aggregate remains weak (`sharpe=-0.01`, `profit_factor=0.97`, `max_drawdown_pct=17.81`)
  - therefore the lane is no longer blocked by missing artifact plumbing or by missing cross-market breadth; it is now blocked by mixed regime stability / market-quality asymmetry
- [x] Verified the Family A baseline remains explicit candidate material rather than a winner-by-default:
  - `transfer_score.status=cross_market_candidate`
  - but market quality is mixed:
    - `NQ/USD`: `sharpe=-0.01`, `profit_factor=0.98`
    - `SPY/USD`: `sharpe=-0.17`, `profit_factor=0.85`
    - `IWM/USD`: `sharpe=0.14`, `profit_factor=1.09`
    - `GLD/USD`: `sharpe=0.44`, `profit_factor=1.42`
  - so the Family A baseline is now breadth-explicit, yet still not strong enough to skip deeper Family A breadth work
- [x] Verified those three real candidate packs all now carry:
  - explicit five-layer mapping
  - explicit resonance context
  - `structural_feedback_required=true`
  - `transfer_score.status=cross_market_candidate`
  - `trade_density_summary.aggregate_label=preferred_density`
- [x] Verified the fresh Family A baseline pack truthfully fails the current promotion gate:
  - `aggregate_trade_count=157` so density is no longer the blocker
  - cross-market breadth is now explicit instead of missing
  - but aggregate quality remains weak (`sharpe=-0.01`, `profit_factor=0.98`, `max_drawdown_pct=16.98`)
  - therefore it stays explicit candidate material, not a promotable pack
- [x] Converted the regime-only placeholder into a real hot-pluggable artifact lane without polluting the generic surface:
  - generic `config/factor_candidate_harness_presets.json` now keeps `regime_primary_gate_pending_v1` zero-config by default
  - the lane declares `reusable_input_kind=regime_benchmark_json` rather than embedding personal `/tmp/...` paths
  - `examples/factor_candidate_profiles/thrill3r-nq-auto-quant-v1.json` now opt-in injects the concrete local benchmark JSON bundle
  - `scripts/research/factor_candidate_resolver.py` now builds either:
    - execution candidate packs from `freqtrade_backtest_zip`
    - or regime artifact bundles from `regime_benchmark_jsons`
  - `scripts/research/regime_artifact_bundle.py` now serves as the additive classifier / transition / resonance artifact emitter
  - verified generic mode stays clean:
    - `python3 scripts/research/factor_candidate_resolver.py --repo-root . --output-dir /tmp/ict-engine-factor-candidate-registry-generic-20260508`
    - result: `selection_mode=generic_zero_config`, `buildable_count=0`, `built_pack_count=0`
    - `regime_primary_gate_pending_v1` stays `artifact_kind=regime_benchmark_json`, `artifact_ready=false`, `pack_build_reason=opt_in_regime_benchmark_profile_required`
  - verified opt-in profile mode builds the real regime bundle:
    - `python3 scripts/research/factor_candidate_resolver.py --repo-root . --profile thrill3r_nq_auto_quant_v1 --build-packs --output-dir /tmp/ict-engine-factor-candidate-registry-profile-20260508`
    - result: `selection_mode=profile_opt_in`, `buildable_count=6`, `built_pack_count=6`
    - `packs/regime_primary_gate_pending_v1/` now contains:
      - `regime_classifier_summary.json`
      - `transition_summary.json`
      - `resonance_summary.json`
      - `cross_market_summary.json`
    - current profile-backed regime summary:
      - `covered_markets=NQ,SPY,QQQ,GLD`
    - `best_market=GLD`
    - `best_eval_macro_f1=0.478629`
    - `average_eval_macro_f1=0.448097`
    - `best_transition_f1=0.074074`
- [x] Tightened reusable-input validation so hot-pluggable evidence is consumer-safe rather than path-only:
  - `scripts/research/factor_candidate_resolver.py` now validates `freqtrade_backtest_zip` readability with zip integrity checks before marking a lane `artifact_ready=true`
  - a broken zip now surfaces `pack_build_reason=invalid_artifact:...` and is skipped instead of crashing the whole `--build-packs` run
  - regression coverage added in `scripts/research/tests/test_factor_candidate_resolver.py` for:
    - invalid zip stays unbuildable in registry mode
    - invalid zip is skipped cleanly in pack-build mode
- [x] Continued the Family A breadth lane with one more real explicit candidate pack:
  - added `family_a_fvg_retrace_1h_v1` to the generic registry as a zero-config Family A structural-retrace lane
  - injected the real local reusable evidence only through the opt-in profile:
    - base `NQ/USD` 8Y zip: `backtest-result-2026-05-08_23-46-20.zip`
    - cross-market `SPY/USD,IWM/USD,GLD/USD` 1Y zip: `backtest-result-2026-05-08_23-47-45.zip`
  - verified `python3 scripts/research/factor_candidate_resolver.py --repo-root . --profile thrill3r_nq_auto_quant_v1 --build-packs --output-dir /tmp/ict-engine-factor-candidate-registry-profile-20260508-v3`
    now builds `candidate_count=7`, `buildable_count=7`, `built_pack_count=7`
  - verified `packs/family_a_fvg_retrace_1h_v1/` carries the required five-layer mapping and explicit breadth stats
  - current evidence is honest but not promotable:
    - aggregate `NQ/USD` base window: `trade_count=12`, `aggregate_label=probe_only`, `sharpe=0.015`, `profit_factor=1.92`
    - transfer layer: `status=cross_market_candidate`, `covered_markets=NQ/USD,SPY/USD,IWM/USD,GLD/USD`, `overall_transfer_score=0.365891`
    - cross-market cells are mixed:
      - `SPY/USD`: `11` trades, `sharpe=0.377`, `profit_factor=2.68`
      - `IWM/USD`: `2` trades, `anecdotal`
    - `GLD/USD`: `10` trades, `sharpe=-0.195`, `profit_factor=0.58`
    - therefore this lane is now breadth-explicit candidate material, but still blocked by thin density and mixed market quality
- [x] Continued the Family A breadth lane with the first explicit 5m timeframe-coverage pack:
  - added `family_a_fvg_retrace_5m_v1` to the generic registry as the 5m resonance-heavy Family A retrace lane
  - injected the real local reusable evidence through the opt-in profile:
    - base `NQ/USD` 8Y 5m zip: `backtest-result-2026-05-08_23-55-11.zip`
  - verified `python3 scripts/research/factor_candidate_resolver.py --repo-root . --profile thrill3r_nq_auto_quant_v1 --build-packs --output-dir /private/tmp/ict-engine-factor-candidate-registry-profile-20260508-v5`
    now builds `candidate_count=8`, `buildable_count=8`, `built_pack_count=8`
  - verified `packs/family_a_fvg_retrace_5m_v1/` carries the required five-layer mapping and explicit timeframe context:
    - `base_timeframe=5m`
    - `context_timeframes=15m,1h,4h`
  - current evidence is explicit and useful, but still a rejection candidate rather than a winner:
    - aggregate `NQ/USD` base window: `trade_count=82`, `aggregate_label=preferred_density`
    - quality remains negative: `sharpe=-0.0199`, `profit_factor=0.8399`, `total_profit_pct=-0.47`, `max_drawdown_pct=1.716`
    - transfer layer remains `single_market_only`
    - therefore the 5m lane successfully closes the timeframe-coverage evidence gap, but it does not close the Family A quality gate
- [x] Extended the hot-pluggable reusable-input layer beyond `freqtrade_backtest_zip` so old explicit Family A evidence can still be consumed without rerunning the whole historical workspace:
  - `scripts/research/factor_candidate_resolver.py` now accepts `strategy_library_json` as another additive reusable input kind
  - this keeps the generic surface zero-config while letting an opt-in profile point at old explicit strategy-library artifacts when those are the only durable evidence left
  - regression coverage added in `scripts/research/tests/test_factor_candidate_resolver.py` for:
    - registry mode recognizes `strategy_library_json` as `artifact_kind=strategy_library_json`
    - pack-build mode converts it into the normal `factor_expression.json` / `factor_eval_grid_summary.json` / `transfer_score.json` outputs
- [x] Continued the Family A breadth lane with the historical 15m strategy-library lane:
  - added `family_a_killzone_breakout_15m_v1` to the generic registry
  - injected the reusable evidence only through the opt-in profile:
    - `strategy_library_json=/tmp/ict-engine-family-a-nq-15m-profile/.deps/auto-quant/strategy_library_15m.json`
  - verified the current profile build now reaches `candidate_count=10`, `buildable_count=10`, `built_pack_count=10`
  - current evidence is explicit but still weak:
    - `trade_count=22`, `aggregate_label=probe_only`
    - `sharpe=0.0746`, `profit_factor=1.1272`, `total_profit_pct=1.18`
    - `transfer_status=single_market_only`
  - therefore the 15m lane is now explicit coverage evidence, not a promotable Family A replacement
- [x] Continued the Family A breadth lane with the historical 1d-regime strategy-library lane:
  - added `family_a_killzone_breakout_1d_regime_v1` to the generic registry
  - injected the reusable evidence only through the opt-in profile:
    - `strategy_library_json=/tmp/ict-engine-family-a-profile-1dregime-check/.deps/auto-quant/strategy_library_round3.json`
  - current evidence is explicit but too thin to count:
    - `trade_count=2`, `aggregate_label=anecdotal`
    - `sharpe=0.4468`, `total_profit_pct=2.26`
    - `transfer_status=single_market_only`
  - therefore the 1d-regime lane is now recorded as a real explicit failure-to-scale rather than an unmaterialized idea
- [x] Continued the Family A breadth lane with the historical 1m strategy-library lane:
  - added `family_a_killzone_breakout_1m_v1` to the generic registry
  - injected the reusable evidence only through the opt-in profile:
    - `strategy_library_json=/tmp/ict-engine-family-a-nq-1m-profile/.deps/auto-quant/strategy_library_1m.json`
  - verified the current profile build now reaches `candidate_count=11`, `buildable_count=11`, `built_pack_count=11`
  - current evidence closes another timeframe-coverage gap but remains a rejection candidate:
    - `trade_count=56`, `aggregate_label=thin`
    - `sharpe=-0.3518`, `profit_factor=0.6742`, `total_profit_pct=-8.2`, `max_drawdown_pct=-12.4045`
    - `transfer_status=single_market_only`
  - therefore the 1m lane is now explicit negative evidence, not a viable Family A promotion path
- [x] Continued the Family A breadth lane with the historical ES strategy-library lane:
  - added `family_a_es_killzone_breakout_1h_v1` to the generic registry
  - injected the reusable evidence only through the opt-in profile:
    - `strategy_library_json=/tmp/ict-engine-family-a-es-profile/ES/auto_quant_strategy_library.json`
  - verified the current profile build now reaches `candidate_count=12`, `buildable_count=12`, `built_pack_count=12`
  - current evidence is explicit positive coverage, but still not a cross-market winner:
    - `trade_count=40`, `aggregate_label=thin`
    - `sharpe=0.2889`, `profit_factor=2.1103`, `total_profit_pct=16.98`
    - `transfer_status=single_market_only`
  - therefore the ES lane is now explicit market-coverage evidence for Family A, not a generalization proof
- [x] Continued the Family A breadth lane with the historical EUR strategy-library lane:
  - added `family_a_eur_killzone_breakout_1h_v1` to the generic registry
  - injected the reusable evidence only through the opt-in profile:
    - `strategy_library_json=/tmp/ict-engine-family-a-eur-profile/EUR/auto_quant_strategy_library.json`
  - verified the current profile build now reaches `candidate_count=13`, `buildable_count=13`, `built_pack_count=13`
  - current evidence is explicit but weak:
    - `trade_count=6`, `aggregate_label=anecdotal`
    - `sharpe=-0.0459`, `profit_factor=0.6891`, `total_profit_pct=-0.37`
    - `transfer_status=single_market_only`
  - therefore the EUR lane is now explicit negative market-coverage evidence, not a Family A quality proof point
- [x] Closed the remaining historically accessible but non-promotable broader market probes for this board:
  - `YM`
    - current profile materializes, but all structure candidates remain absent or zero-trade
    - current `auto_quant_strategy_library.json` has no positive Family A lane to recover
    - therefore `YM` remains an explicit failure/coverage boundary, not a recoverable candidate on this board
  - `XAU`
    - current profile materializes, but all structure candidates remain zero-trade
    - therefore `XAU` remains an explicit failure/coverage boundary, not a recoverable candidate on this board

### Next

- [ ] For each active factor family, write or refresh one candidate-spec JSON that explicitly fills:
  - `pre_bayes_targets`
  - `belief_targets`
  - `path_ranking_targets`
  - `execution_tree_targets`
  - `structural_feedback_required`
- [ ] For each active factor family, rebuild its candidate pack through `scripts/research/factor_candidate_pack.py`.
- [ ] Reject any candidate whose pack still lacks middle-layer mapping even if backtest metrics look good.
- [ ] For regime-only candidates:
  - require classifier / transition / resonance evidence first
  - do not force execution trade density as the primary gate
- [ ] For execution candidates:
  - require trade density, resonance, and cross-market evidence
  - require the candidate pack to state which execution-tree blockers it intends to move
- [ ] For any candidate expected to influence runtime recommendation support later:
  - set `structural_feedback_required=true`
  - explicitly note that non-demo runtime validation cannot be honest until structural lineage exists in the downstream real-trade source
- [ ] Hand off to `docs/plans/2026-05-07-auto-quant-post-factor-runtime-closure-todo.md` only after a candidate pack is explicit enough to answer all of the following from artifacts, not chat:
  - what this candidate expects the `pre-bayes / filter gate` to do
  - what BBN evidence/prior it expects to strengthen or weaken
  - what structural path-ranking surface it expects to move
  - what execution-tree blocker / branch / gate it is trying to affect
- [ ] Use `docs/plans/2026-05-09-factor-iteration-pre-bayes-bbn-catboost-execution-tree-todo.md` as the sequencing bridge once a candidate leaves pure factor iteration:
  - factor board owns candidate generation and pack truth
  - bridge board owns chain-level diagnosis and stopping-layer labeling
  - post-factor board owns runtime mutation and before/after evidence
- [x] Re-verified the merged baseline line after integrating the factor-candidate registry slice on top of the newer hot-plug baseline:
  - `python3 -m unittest scripts.research.tests.test_factor_candidate_pack scripts.research.tests.test_factor_candidate_resolver scripts.research.tests.test_regime_artifact_bundle`
  - result: `15` tests, `OK`
  - generic merged audit:
    - `python3 scripts/research/factor_candidate_resolver.py --repo-root . --output-dir /tmp/ict-engine-factor-candidate-registry-generic-20260509-merged2`
    - result: `selection_mode=generic_zero_config`, `candidate_count=13`, `buildable_count=0`, `built_pack_count=0`
  - opt-in merged audit:
    - `python3 scripts/research/factor_candidate_resolver.py --repo-root . --profile thrill3r_nq_auto_quant_v1 --build-packs --output-dir /private/tmp/ict-engine-factor-candidate-registry-profile-20260509-merged`
    - result: `selection_mode=profile_opt_in`, `candidate_count=13`, `buildable_count=13`, `built_pack_count=13`
- [x] Verified every currently active registry entry on the merged line emits an explicit reviewable artifact family:
  - `12` execution entries emit:
    - `factor_expression.json`
    - `factor_eval_grid_summary.json`
    - `transfer_score.json`
  - every execution pack still carries the required five-layer mapping:
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
- [x] No additional candidate-registry or hot-pluggable artifact-plumbing slice remains active on this board.
- [x] If this board is reopened later, start from new factor evidence generation or a newly nominated family lane rather than reworking the registry / bundle infrastructure again.

### Not Yet

- [ ] Move any factor candidate to the post-factor runtime-closure board before its candidate pack exists
- [ ] Treat single-market thin proof as family closure
- [ ] Add new runtime-only factor ingestion code just to compensate for a missing external candidate pack
- [ ] Fold provider bootstrap or generic UX requirements into this board

---

## Ordered Execution Checklist

1. Choose one factor family from the active Auto-Quant lane.
2. State its intended role:
   - `regime_only`
   - `execution_only`
   - `mixed`
3. Write or update a candidate-spec JSON that includes explicit `filter_belief_execution_mapping`.
4. Build the candidate pack with:
  - `python3 scripts/research/factor_candidate_pack.py --manifest-json <strategy_library.json> --strategy-name <strategy> --candidate-spec-json <candidate_spec.json> --autoresearch-status-json <autoresearch_status.json> --output-dir /tmp/<candidate-pack>`
  - or `python3 scripts/research/factor_candidate_pack.py --freqtrade-backtest-zip <backtest.zip> --strategy-name <strategy> --candidate-spec-json <candidate_spec.json> --output-dir /tmp/<candidate-pack>`
  - or `python3 scripts/research/factor_candidate_resolver.py --repo-root . [--profile <opt-in-profile>] --build-packs --output-dir /tmp/<candidate-pack-root>`
5. Inspect:
  - `factor_expression.json`
  - `factor_eval_grid_summary.json`
  - `transfer_score.json`
6. Reject immediately if:
   - mapping is incomplete
   - density is anecdotal/invalid
   - resonance is missing for the claimed base timeframe
7. Only after a candidate pack is explicit and reviewable may the lane continue toward runtime closure evidence.

---

## Success Standard

This board is successful only if all of the following are true:

- Every active factor family has an explicit candidate pack.
- Every candidate pack states exactly how it feeds:
  - pre-bayes filter
  - belief / BBN evidence
  - path ranking
  - execution tree
- Every candidate pack states whether structural feedback lineage is required for honest downstream validation.
- Promotion out of factor iteration is blocked by explicit artifact gates, not by chat-only judgment.
- Any downstream closure attempt can start from the emitted pack alone, without needing to reconstruct intent from older board prose.
