# Regime Classifier R2-R4 Handoff TODO

> Live board for high-confidence regime classifier sidecar chain: R2 ontology -> R3 features -> R4 unsupervised discovery.

**Goal:** make regime labels, feature inputs, and unsupervised discovery artifacts machine-readable before R5 expert training / R6 conformal calibration / BBN + execution-tree integration.

**Scope:** sidecar support/scripts/tests/docs only. Ignore unrelated worktree drift.

---

## Routing / Process

- Primary route used: `test-driven-dev` for R2/R3/R4 implementation.
- Handoff route used: `plans`.
- Project router: `/Users/thrill3r/.hermes/routing/project-router.md` missing; no override.
- Repo entry map read: `AGENTS.md`.
- Domain skill read: `ict-engine-runtime`.
- TDD rule followed: tests first, RED verified, minimal GREEN, full research test sweep.

## Current Worktree Note

Do not touch unrelated dirty files unless explicitly asked:

- `src/auto_quant_command.rs`
- `src/validate_market_state_command.rs`

Current sidecar files from this chain:

- `support/scripts/research/regime_ontology_manifest.py`
- `support/scripts/research/regime_feature_builder.py`
- `support/scripts/research/regime_discovery_cluster.py`
- `support/scripts/research/regime_discovery_hmm.py`
- `support/scripts/research/tests/test_regime_ontology_manifest.py`
- `support/scripts/research/tests/test_regime_feature_builder.py`
- `support/scripts/research/tests/test_regime_discovery.py`
- `support/docs/plans/2026-05-09-regime-classifier-research-and-99-confidence-todo.md`
- `support/docs/plans/2026-05-09-regime-classifier-r2-r4-handoff-todo.md`

---

## Slice R2: Regime Ontology Manifest

### Done

- [x] Wrote failing tests first: `support/scripts/research/tests/test_regime_ontology_manifest.py`
- [x] RED verified: `ModuleNotFoundError: No module named 'regime_ontology_manifest'`
- [x] Implemented: `support/scripts/research/regime_ontology_manifest.py`
- [x] Emits `regime_ontology_manifest.json`
- [x] Emits `regime_expert_bank_manifest.jsonl`
- [x] Emits 53 experts:
  - 5 primary
  - 16 secondary
  - 24 dimension
  - 8 transition
- [x] Covers current Rust/runtime labels:
  - `TrendExpansion`
  - `RangeConsolidation`
  - `ExtremeStress`
  - `ReversalBrewing`
  - `Unknown`
- [x] Marks `Unknown` / `Neutral` style classes as abstain/fallback.
- [x] Each expert carries confidence contract fields:
  - `target_coverage`
  - `abstain_policy`
  - `min_support`
  - `positive_definition`
  - `negative_definition`
  - `required_features`
  - `allowed_data_sources`
  - `promotion_gates`

### Verification

- [x] Target tests:
  - `python3 -m unittest support/scripts/research/tests/test_regime_ontology_manifest.py -v` -> 4 OK
- [x] Full research tests after R2:
  - `python3 -m unittest discover -s support/scripts/research/tests -p 'test_*.py'` -> 57 OK

### CLI Floor

```bash
python3 support/scripts/research/regime_ontology_manifest.py \
  --output-json /tmp/ict-regime/regime_ontology_manifest.json \
  --output-jsonl /tmp/ict-regime/regime_expert_bank_manifest.jsonl
```

---

## Slice R3: Regime Feature Builder

### Done

- [x] Wrote failing tests first: `support/scripts/research/tests/test_regime_feature_builder.py`
- [x] RED verified: `ModuleNotFoundError: No module named 'regime_feature_builder'`
- [x] Implemented: `support/scripts/research/regime_feature_builder.py`
- [x] Reads OHLCV CSV.
- [x] Reads OHLCV JSONL.
- [x] Optional join by `timestamp`:
  - `--auxiliary-evidence`
  - `--mtf-pda-events`
- [x] Outputs feature table CSV.
- [x] Outputs feature quality report JSON.
- [x] Zero-config OHLCV-only path works.
- [x] Missing optional inputs do not fail; report marks them missing.
- [x] User VRP/NQ auxiliary fields pass through:
  - `qqq_hv_level`
  - `nq_vs_200d_pct`
  - `vix3m_level`
  - `qqq_hv_pct_rank_252`
  - `vvix_over_vix`
- [x] MTF fields pass through:
  - `mtf_alignment`
  - `pda_event_count`

### Current Feature Columns

- `return_1`
- `candle_range`
- `body_pct`
- `upper_wick_pct`
- `lower_wick_pct`
- `atr_3`
- `atr_percentile`
- `volume_percentile`
- `directional_efficiency_3`
- `slope_3`
- `range_position`
- `rsi_3`
- `realized_vol_3`
- `mtf_alignment`
- `pda_event_count`
- VRP/NQ pass-through fields when present

### Verification

- [x] Target tests:
  - `python3 -m unittest support/scripts/research/tests/test_regime_feature_builder.py -v` -> 4 OK
- [x] Full research tests after R3:
  - `python3 -m unittest discover -s support/scripts/research/tests -p 'test_*.py'` -> 61 OK

### CLI Floor

```bash
python3 support/scripts/research/regime_feature_builder.py \
  --ohlcv /tmp/ict-regime/ohlcv.csv \
  --auxiliary-evidence /tmp/ict-regime/auxiliary.jsonl \
  --mtf-pda-events /tmp/ict-regime/mtf_pda_events.jsonl \
  --output-features /tmp/ict-regime/regime_features.csv \
  --output-report /tmp/ict-regime/feature_quality_report.json
```

---

## Slice R4: Unsupervised Regime Discovery

### Done

- [x] Wrote failing tests first: `support/scripts/research/tests/test_regime_discovery.py`
- [x] RED verified: `ModuleNotFoundError: No module named 'regime_discovery_cluster'`
- [x] Implemented: `support/scripts/research/regime_discovery_cluster.py`
- [x] Implemented: `support/scripts/research/regime_discovery_hmm.py`
- [x] Cluster sidecar evaluates `k=3..12`.
- [x] HMM sidecar evaluates `k=3..12`.
- [x] Stores metrics:
  - `silhouette`
  - `bic`
  - `aic`
  - `transition_persistence`
- [x] Maps discovered state profiles to candidate ICT labels:
  - `primary::TrendExpansion`
  - `primary::RangeConsolidation`
  - `primary::ExtremeStress`
  - `primary::ReversalBrewing`
  - `primary::Unknown`
- [x] Ontology is read-only; discovery does not overwrite fixed ontology.
- [x] Uses deterministic pure-Python fallback; no sklearn/hmmlearn hard dependency.

### Verification

- [x] Target tests:
  - `python3 -m unittest support/scripts/research/tests/test_regime_discovery.py -v` -> 3 OK
- [x] Full research tests after R4:
  - `python3 -m unittest discover -s support/scripts/research/tests -p 'test_*.py'` -> 64 OK

### CLI Floor

```bash
python3 support/scripts/research/regime_discovery_cluster.py \
  --features /tmp/ict-regime/regime_features.csv \
  --ontology /tmp/ict-regime/regime_ontology_manifest.json \
  --output-json /tmp/ict-regime/cluster_regime_discovery_report.json

python3 support/scripts/research/regime_discovery_hmm.py \
  --features /tmp/ict-regime/regime_features.csv \
  --ontology /tmp/ict-regime/regime_ontology_manifest.json \
  --output-json /tmp/ict-regime/hmm_regime_discovery_report.json
```

---

## Immediate Next Slice: R5 One-vs-Rest Expert Training

### Objective

Train or score one binary expert per ontology label using R3 features and optional R4 discovered-state hints, with offline fallback and precision-first thresholds.

### Create

- [ ] `support/scripts/research/regime_expert_trainer.py`
- [ ] `support/scripts/research/tests/test_regime_expert_trainer.py`

### Inputs

- [ ] `regime_ontology_manifest.json`
- [ ] `regime_features.csv` or JSONL
- [ ] Optional `cluster_regime_discovery_report.json`
- [ ] Optional `hmm_regime_discovery_report.json`
- [ ] Optional labeled rows with `truth` / `label` / `primary_label`

### Outputs

- [ ] `regime_expert_scores.jsonl`
- [ ] `regime_expert_training_report.json`
- [ ] Optional `regime_expert_artifacts/`

### Acceptance

- [ ] Trains or scores one binary expert per ontology label.
- [ ] Supports pure-Python threshold fallback when sklearn unavailable.
- [ ] Supports per-label threshold search for precision-first mode.
- [ ] Reports per-label:
  - precision
  - recall
  - F1
  - Brier proxy
  - ECE proxy
  - support
  - threshold
- [ ] Emits score rows with:
  - `timestamp`
  - `label_id`
  - `score`
  - `threshold`
  - `decision`
  - `abstain_reason`
- [ ] Uses purged split / embargo-compatible interface, even if first implementation uses deterministic fallback.
- [ ] Does not mutate ontology.

### First RED Tests

- [ ] Missing `regime_expert_trainer` import fails.
- [ ] Trainer loads R2 ontology and produces one expert summary per label.
- [ ] Trainer reads R3 features and emits `regime_expert_scores.jsonl`.
- [ ] Precision-first thresholding raises threshold for ambiguous labels.
- [ ] Unknown/abstain labels never become trade-usable positive decisions.
- [ ] CLI writes report and scores.

### CLI Target

```bash
python3 support/scripts/research/regime_expert_trainer.py \
  --ontology /tmp/ict-regime/regime_ontology_manifest.json \
  --features /tmp/ict-regime/regime_features.csv \
  --cluster-report /tmp/ict-regime/cluster_regime_discovery_report.json \
  --hmm-report /tmp/ict-regime/hmm_regime_discovery_report.json \
  --output-scores /tmp/ict-regime/regime_expert_scores.jsonl \
  --output-report /tmp/ict-regime/regime_expert_training_report.json
```

---

## Later Slices

### R6: Conformal Calibration Layer

- [ ] Create `support/scripts/research/regime_conformal_calibration_report.py`.
- [ ] Add class-conditional conformal coverage.
- [ ] Support `0.95` and `0.99` target coverage.
- [ ] Emit singleton rate and set size.
- [ ] Only emit `confidence_95` / `confidence_99` when coverage gates pass.

### R7: Distributional Agreement Layer

- [ ] Create `support/scripts/research/regime_distributional_agreement_report.py`.
- [ ] Compare candidate regime states to archetype distributions.
- [ ] Use scipy Wasserstein if present.
- [ ] Fallback to quantile / energy-distance proxy.

### R8: Drift / Transition Governor

- [ ] Extend or reuse `transition_evidence_aggregator.py`.
- [ ] Consume R6/R7 reports.
- [ ] Emit transition guardrail reasons.

### R9: High-Confidence Regime Decision

- [ ] Create `support/scripts/research/regime_high_confidence_decision.py`.
- [ ] Combine expert scores, conformal report, distributional agreement, transition governor, BBN evidence value.
- [ ] Emit `regime_high_confidence_decision.json`.
- [ ] States:
  - `single_label_95`
  - `single_label_99`
  - `label_set`
  - `transitional`
  - `unknown_abstain`

### R10: BBN / Execution-Tree Integration Plan

- [ ] Map high-confidence regime states into BBN evidence.
- [ ] Add execution tree trace field: `regime_confidence_gate`.
- [ ] Export path-ranker features:
  - `regime_high_confidence_label`
  - `conformal_set_size`
  - `transition_hazard`
  - `distributional_agreement`
- [ ] `analyze --human` explains accepted/rejected regime evidence.

---

## Commit Plan

Commit only this sidecar chain; do not include unrelated Rust changes.

### Suggested Add

```bash
git add \
  support/docs/plans/2026-05-09-regime-classifier-research-and-99-confidence-todo.md \
  support/docs/plans/2026-05-09-regime-classifier-r2-r4-handoff-todo.md \
  support/scripts/research/regime_ontology_manifest.py \
  support/scripts/research/regime_feature_builder.py \
  support/scripts/research/regime_discovery_cluster.py \
  support/scripts/research/regime_discovery_hmm.py \
  support/scripts/research/tests/test_regime_ontology_manifest.py \
  support/scripts/research/tests/test_regime_feature_builder.py \
  support/scripts/research/tests/test_regime_discovery.py
```

### Suggested Commit

```bash
git commit -m "feat: add regime classifier sidecar discovery chain"
```

## Final Verification Before Commit

- [x] Latest full sidecar test sweep:
  - `python3 -m unittest discover -s support/scripts/research/tests -p 'test_*.py'` -> 68 OK
- [x] Re-run full sidecar test sweep immediately before commit.
- [x] Confirm `git diff --cached --name-only` excludes unrelated Rust files before committing.
