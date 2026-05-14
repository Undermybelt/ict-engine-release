# Oracle regime probe

Status: experimental research design only

Purpose
- Evaluate whether manually/rule-labelled regimes can help discover informative factor subsets.
- Use oracle labels only as a research probe inside `factor-research`.
- Produce comparison artifacts, not production truth.

Non-goals
- Not a live regime detector.
- Not a replacement for canonical timed-PDA / pre-bayes / BBN / ensemble paths.
- Not a justification to promote hand labels directly into execution logic.
- Not a promise of "perfect recovery".

Why this exists
A top-down regime workflow can be useful in research:
1. define a small MECE-like label set for historical segments
2. search factor subsets against those labels
3. measure HMM/HSMM recovery quality
4. keep only the report/artifacts if the discovery is useful

This is acceptable as offline discovery.
It is not acceptable as production truth without separate validation.

Repo-compatible framing
- Put behind a research-only mode, e.g. `factor-research --regime-probe oracle`
- Outputs should land as versioned experiment artifacts
- Results should be compared against current canonical belief / timed-PDA surfaces, not replace them

Recommended artifacts
- `oracle_regime_map.json`
- `oracle_regime_probe_report.json`
- `oracle_regime_confusion_matrix.json`
- `oracle_feature_saliency.json`
- markdown summary under `docs/experiments/`

Suggested pipeline
1. Oracle labeler
   - input: candles + existing derived features
   - output: discrete regime labels for research only
   - rule source must be explicit and versioned

2. Factor subset search
   - try curated subsets first
   - then wrapper/greedy search if needed
   - measure:
     - Viterbi path agreement
     - posterior match score
     - confusion matrix diagonal
     - stability across folds/time windows

3. Validation gate
   - reject if recovery only works on the calibration slice
   - reject if labels are obviously tautological with the oracle rules
   - reject if recovered regimes do not improve downstream policy discrimination

4. Reporting
   - list which features mattered
   - list which oracle classes were recoverable vs not
   - show drift/failure cases
   - compare against current production regime surfaces

Guardrails
- Oracle labels are for factor discovery, not production supervision truth.
- Do not wire oracle labels directly into `EntryExecution`, `reflection_bundle`, or voting decisions as truth.
- Any production promotion requires separate out-of-sample evidence.
- If the oracle label set bakes in the same exact rules as the tested features, call out leakage explicitly.
- Prefer "label recovery score" over "perfect classification" language.

Recommended naming
Use:
- `oracle_regime_probe`
- `label_recovery_score`
- `feature_saliency_report`
Avoid:
- `perfect_recovery`
- `production_lock_in`
- `zero_cost_integration`

Minimal architecture sketch
- `src/factor_lab/oracle_probe.rs`
  - `oracle_regime_labels(...)`
  - `search_feature_subsets_for_label_recovery(...)`
  - `score_label_recovery(...)`
- `src/factor_lab/research.rs`
  - optional research-only branch for oracle probe mode
- `docs/experiments/`
  - run summaries and conclusions

Suggested success criteria
A probe is useful if it shows all of:
- robust recovery on holdout slices
- non-trivial factor saliency concentration
- no obvious label leakage
- downstream alignment with existing policy surfaces

Suggested failure criteria
Reject or archive if any of:
- recovery collapses on time-split holdout
- only works when oracle rules and feature rules are nearly identical
- produces no better downstream separation than current canonical surfaces
- pushes the team toward replacing timed-PDA / pre-bayes truth with hand labels

How to discuss results
Prefer:
- "oracle probe suggests feature family X helps separate label Y"
- "recovery quality is moderate/high on holdout"
- "candidate research surface only"
Avoid:
- "HMM perfectly found the true regimes"
- "safe to use in production now"

Practical next step
If implemented, keep the first milestone tiny:
- one small oracle label set
- one fixed dataset slice
- one holdout split
- one artifact report
- no production wiring

Trial intake extension
- Normalize `related-stock relative consistency` to the repo's SMT / correlation-consistency lane.
- Normalize `tiny-leg zigzag regime` to a retrospective segmentation / clustering lane.
- If a tiny-leg probe is added, keep it research-only:
  - small zigzag
  - 5 leg factors
  - optional `16` raw clusters merged to `6`
- Do not treat retrospective tiny-leg or zigzag outputs as live regime truth.
- Any later live promotion requires a separate now-cast branch that is not delayed purely by pivot confirmation.
