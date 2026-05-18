# 95 Confidence Asset Recovery

Updated: `2026-05-13 11:27:04 +0800`

Purpose: recovered index of high-confidence assets that were buried in the May
10 append-only logs and experiment roots. This file is a pointer and operating
rule; the structured ledger is the CSV below.

Structured ledger:

```text
support/docs/experiments/actionable-regime-confidence/recovered_95_confidence_assets.csv
```

## Recovery Result

The old logs did contain real 95+ confidence assets. They were not the same as
the current Board B profit-factor candidate packs. Most recovered assets are
Board A regime/source-confidence gates or direct-event manipulation overlays.

Verified `2026-05-13 11:27 +0800`: the recovered assets are no longer only a
Markdown pointer. They are available through the native runtime entrypoint:

```bash
cargo run --quiet -- regime-confidence-assets --symbol REGIME_CONFIDENCE_ASSETS --state-dir /tmp/ict-engine-regime-confidence-assets --output-format human
cargo run --quiet -- policy-training-status --symbol REGIME_CONFIDENCE_ASSETS --state-dir /tmp/ict-engine-regime-confidence-assets --output-format human
cargo run --quiet -- artifact-status --symbol REGIME_CONFIDENCE_ASSETS --state-dir /tmp/ict-engine-regime-confidence-assets --latest-only
cargo test regime_confidence_asset -- --nocapture
```

Current verified readback:

- `regime-confidence-assets`: `asset_count=18`, `board_a_gate=11`,
  `direct_event=2`, `diagnostic=4`, `contrast_evidence=10`.
- Persisted artifact:
  `/tmp/ict-engine-regime-confidence-assets/REGIME_CONFIDENCE_ASSETS/regime_confidence_asset_inventory.json`.
- `policy-training-status`: `Regime confidence assets: inventory=ready
  count=18 board_a_gate=11 direct_event=2 diagnostic=4 contrast_evidence=10
  promotion_allowed=false runtime_selection=disabled`.
- `artifact-status --latest-only`: one
  `regime_confidence_asset_inventory` ledger row with `status=ready_preserved`,
  `path_exists=true`, `promote_candidate=false`, `actionable=false`.
- Tests: `cargo test regime_confidence_asset -- --nocapture` passed 3 native
  tests.

Boundary: this is a real repo/runtime entrance and training-readiness readback,
not trade promotion. `workflow-status` still reports `no_workflow_state` for
this symbol, so the remaining gap is live workflow surfacing/admission through
provider/AQ -> filter -> belief -> path tree -> execution review, not asset
recovery.

Recovered first-pass inventory:

- 5 baseline accepted MainRegimeV2 assets:
  - `Bull`: `bull_sourcebacked_drawdown_volatility_v1`
  - `Bear`: `bear_sourcebacked_drawdown_return_ratio_v1`
  - `Sideways`: `sideways_sourcebacked_abs_return_range_v1`
  - `Crisis`: `crisis_range_ratio_intraday_v1`
  - `Manipulation`: `manipulation_telegram_direct_event_v1`
- 2 same-source weekly/monthly timeframe gates:
  - `same_source_timeframe_1mo_sideways_v1`
  - `weekly_bull_source_consensus_v1`
  - `weekly_sideways_source_consensus_v1`
- 4 scoped stock-market-regime parent-root gates:
  - `stock_market_regime_bull_v1`
  - `stock_market_regime_bear_v1`
  - `stock_market_regime_sideways_v1`
  - `stock_market_regime_crisis_v1`
- 4 HGB numeric diagnostic confidence assets:
  - `hgb_numeric_bear_confidence_v1`
  - `hgb_numeric_bull_confidence_v1`
  - `hgb_numeric_crisis_confidence_v1`
  - `hgb_numeric_sideways_confidence_v1`
- 2 support/overlay assets:
  - `crisis_crash_crosswalk_v1`
  - `bsc_meme_wash_maker_direct_slice_v1`

## Operating Rule

Do not discard these assets because they are not in
`support/examples/factor_candidate_packs/curated-auto-quant-v1/`.

Do not misclassify them as trade-usable profit factors either.

Use `usable_as` from the CSV:

- `board_a_regime_gate`: candidate for a Board A regime-gate artifact surface.
- `direct_event_overlay`: usable as suppression, abstain, cooldown, or direct
  event overlay evidence until full manipulation coverage is proven.
- `diagnostic_after_source_control_unlock`: high-value diagnostic confidence
  assets that need source/control unlock before canonical merge.
- `board_a_regime_gate_support`: supporting provenance for an already accepted
  parent root, not a new root by itself.

## Evidence Boundary

These assets are recovered, preserved, and visible through
`regime-confidence-assets`, the artifact ledger, and `policy-training-status`.
They are intentionally not visible through `factor-candidate-packs` because that
command is Board B profit-factor candidate infrastructure.

The correct next implementation is a Board A regime-confidence asset entrypoint
or adapter, not stuffing these into the Board B profit-factor pack loop.

Minimum next artifact contract:

```text
recovered_95_confidence_assets.csv
-> Board A regime-gate inventory
-> policy/training readback for regime confidence
-> provider/source-control parity check
-> downstream admission only after Board A gates pass
```

## Do Not Delete

Until the Board A inventory/admission adapter exists, the following are
retention roots:

- `support/docs/experiments/actionable-regime-confidence/runs/20260511T035045-codex-kaggle-bull-coverage-buffer-gate`
- `support/docs/experiments/actionable-regime-confidence/runs/20260511T041923-codex-yahoo-sourcebacked-parent-root-gate`
- `support/docs/experiments/actionable-regime-confidence/runs/20260510T235220-codex-broader-root-v2-probe`
- `support/docs/experiments/actionable-regime-confidence/runs/20260511T045102-codex-mehrnoom-telegram-direct-manipulation-gate`
- `support/docs/experiments/actionable-regime-confidence/runs/20260512T051844-codex-source-label-hgb-numeric-threshold-screen-v1`
- `support/docs/experiments/actionable-regime-confidence/runs/20260512T053852-codex-hgb-per-regime-field-materialization-v1`

Any cleanup slice must treat the CSV ledger plus these roots as protected
evidence until a native Board A entrypoint supersedes them.
