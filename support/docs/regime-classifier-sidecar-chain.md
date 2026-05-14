# Regime Classifier Sidecar Chain

Zero-config, opt-in sidecar chain for high-confidence ICT regime classification.

## What it does

Runs R2-R10 into an explicit output directory:

- R2 ontology manifest
- R3 feature builder
- R5 expert trainer / scorer
- R6 conformal calibration
- R7 distributional agreement
- R8 transition governor
- R9 high-confidence decision
- R10 consumer bundle

Main runtime is unchanged. Consumers opt in by running this sidecar.

## One-command usage

```bash
python3 support/scripts/research/regime_sidecar_pipeline.py \
  --ohlcv /path/to/ohlcv.csv \
  --output-dir /tmp/ict-regime \
  --label-prefix primary::Trend
```

Optional user-specific evidence:

```bash
python3 support/scripts/research/regime_sidecar_pipeline.py \
  --ohlcv /path/to/ohlcv.csv \
  --auxiliary-evidence /path/to/aux.csv \
  --truth /path/to/truth.jsonl \
  --output-dir /tmp/ict-regime \
  --label-prefix primary::Trend
```

Aux fields passed through when present:

- `qqq_hv_level`
- `nq_vs_200d_pct`
- `vix3m_level`
- `qqq_hv_pct_rank_252`
- `vvix_over_vix`

## Input contract

Required:

- `--ohlcv`: CSV or JSONL with `timestamp,open,high,low,close,volume`.

Optional:

- `--output-dir`: default `/tmp/ict-regime-sidecar`.
- `--label-prefix`: default `primary::Trend`.
- `--auxiliary-evidence`: timestamp-joined CSV/JSONL.
- `--truth`: JSONL labels keyed by `timestamp`.

If `--ohlcv` is missing, the command exits `2`, prints the input contract, and does not create repo-root state.

## Consumer outputs

Primary artifact:

- `/tmp/ict-regime/regime_consumer_bundle.json`

Key fields:

- `latest_decision`
- `consumer_hints.execution_tree_hint`
- `consumer_hints.bbn_evidence_hint`
- `consumer_hints.path_ranker_context`
- `consumer_hints.user_vrp_nq_context`
- `missing_artifacts`

Clean narrowed-scope smoke should produce:

- `decision_state=single_label_99`
- `trade_usable=true`
- `final_label=primary::TrendExpansion`
- `execution_tree_hint=accept_regime`

Broad/noisy scopes may produce `label_set`, `transitional`, or `unknown_abstain`. That is safe evidence, not command failure.
