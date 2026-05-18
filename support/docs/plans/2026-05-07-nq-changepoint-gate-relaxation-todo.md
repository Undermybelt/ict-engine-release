# NQ Change-Point Matrix And Gate-Relaxation TODO

> Authoritative execution board for the next 2026-05-07 regime-validation slice.  
> This file is the new task contract for turning the current helper state into a measurable NQ change-point matrix plus targeted gate-relaxation report. Update this same markdown in place after each real slice.

**Goal:** produce a full NQ `15m/1h/4h/1d` change-point validation matrix and a targeted gate-relaxation report for the existing four 15m candidates, without adding new strategy families or starting CatBoost policy-surface work early.

**Architecture:** keep `ict-engine` runtime source frozen for this slice. Use additive external helpers, persisted `/tmp/...` artifacts, and existing `ict-engine analyze` surfaces only when the relaxed-candidate import/analyze path is actually reachable. The first closure target is measurement and comparison, not performance promotion.

**Tech Stack:** `support/scripts/auto_quant_external/external_regime_changepoint_labels.py`, `support/scripts/auto_quant_external/entry_drought_diagnostic_v2.py`, `support/scripts/auto_quant_external/regime_factor_benchmark.py`, local NQ feather/candle corpora, `/tmp/ict-engine-changepoint-matrix/`, and optional `./target/debug/ict-engine analyze ...` verification if artifacts can be consumed cleanly.

**Baseline / Authority Refs:** `support/docs/202605071246nextstep`, `support/docs/plans/2026-05-05-execution-tree-factor-auto-quant-todo.md`, `support/scripts/auto_quant_external/external_regime_changepoint_labels.py`, `support/scripts/auto_quant_external/entry_drought_diagnostic_v2.py`, `/tmp/ict-engine-entry-drought-v2.json`.

**Compatibility Boundary:** do not modify existing `ict-engine` runtime code just to keep this slice moving. Do not add new strategy families. Do not optimize standalone Sharpe first. Do not claim regime completeness. Do not treat improved entry density as promotion proof. CatBoost policy-surface work is blocked until the change-point matrix and gate-relaxation report both exist.

---

## Decision Lock

These judgments are accepted for this slice. Do not reopen them unless concrete repo evidence forces a change.

- [x] The evaluation is directionally correct: the project has helper/tooling progress, but not a closed regime-validation loop yet.
- [x] The statement `regime has deployable versions but is not complete` is acceptable as a repo-level assessment, but this slice must not restate it as proof of new closure.
- [x] The statement `BBN, CatBoost policy vote, execution readiness, and portfolio diversity are unchanged` is correct for the current helper-only state.
- [x] The bounded NQ `15m` change-point smoke is only a partial proof-of-path. It is not a full matrix and must not be reported as such.
- [x] The `over_gating_issue` diagnosis from `entry_drought_diagnostic_v2.py` is useful and actionable, but it is still helper-layer evidence. It does not by itself prove the live/runtime drought path is fully resolved.
- [x] `ict-engine analyze` before/after is a secondary target in this slice. If the import/analyze path is unavailable, record a clean blocker and stop; do not fake runtime closure.
- [x] The next slice should stay inside `regime validation + targeted gate relaxation`. It must not jump ahead to CatBoost policy-surface training.

## Hard Constraints

- Preserve repo-code-frozen execution for `ict-engine` runtime source.
- Persist all new change-point artifacts under `/tmp/ict-engine-changepoint-matrix/`.
- Use NQ only in this slice.
- Target the full reachable NQ ladder:
  - `15m`
  - `1h`
  - `4h`
  - `1d`
- Use the widest reachable span, preferring `2018-01-01` to `2025-12-31`.
- Start change-point detection with `window`.
- Add `PELT` / `Binseg` only if runtime is acceptable enough to complete the ladder without stalling the slice.
- If regime-classifier comparison artifacts are unavailable, mark comparison as `blocked`, not `failed`.
- For gate relaxation, mutate only the named gates on the existing named candidates. Do not author new strategy files or new strategy families in this slice.
- A relaxation test may increase density, but no candidate may be promoted on density alone.

## Evaluation Check

### Correct

- The current state really is “helper work exists, but validation/reporting loop is not closed.”
- The slice objective should be “turn helper work into measurable regime validation and gate relaxation,” not “continue adding ideas.”
- Blocking CatBoost policy-surface work until the matrix/report are done is the right sequencing.
- Not claiming regime completeness is the right discipline.

### Needs tighter wording

- `All four are over-gating issues` should be read as “all four are over-gating according to the current helper output,” not as “the runtime drought root cause is fully settled.”
- `If possible, run analyze before/after` should remain optional. The slice still passes if it produces the full NQ matrix and targeted gate-relaxation report but documents an honest analyze/import blocker.
- `Compare against the current regime classifier output if available` means reusing or extracting the current classifier outputs cleanly. It does not justify building a new classifier lane inside this slice.

## Current Todo Board

### Done

- [x] `external_regime_changepoint_labels.py` exists, compiles, is unit-tested, and passed a bounded NQ `15m` smoke.
- [x] `entry_drought_diagnostic_v2.py` exists and produced real outputs for:
  - `TomacNQ_RegimeTrendPullbackDense15m`
  - `TomacNQ_RegimeLiquiditySweepReclaim15mWide`
  - `TomacNQ_RegimePersistenceClusterDense15m`
  - `TomacNQ_RegimeVRPCompression15m`
- [x] Current helper output classifies all four current 15m candidates as `over_gating_issue`.
- [x] No execution-tree before/after was run yet.
- [x] BBN, CatBoost policy vote, execution readiness, and portfolio diversity remain unchanged at the current helper-only state.
- [x] The slice boundary is now locked: no new families, no standalone-Sharpe-first drift, no premature CatBoost jump.

### Next

- [x] Built the full NQ change-point matrix under `/tmp/ict-engine-changepoint-matrix/` for:
  - `15m`
  - `1h`
  - `4h`
  - `1d`
- [x] Persisted per-timeframe change-point outputs:
  - breakpoints
  - segment summaries
  - transition proximity score
  - segment duration distribution
  - transition density
  - over-detection / under-detection warnings
- [x] Located the current regime-benchmark summary paths and persisted `/tmp/ict-engine-changepoint-matrix/comparison/comparison_blocked.json` because the available artifacts stop at aggregate `ranked_results` metrics and do not include the bar-level classifier outputs needed to compute:
  - `changepoint_transition_f1`
  - transition precision / recall within tolerance windows
  - flip-rate
  - mean segment bars
  - early / late / noisy / missing classification
- [x] Persisted a clean `comparison_blocked` artifact describing the exact missing comparison surface:
  - `/tmp/ict-engine-changepoint-matrix/comparison/comparison_blocked.json`
- [x] Turned the current gate-drought findings into targeted relaxation tests only for the named gates:
  - `TrendPullbackDense15m`: `reacceleration`, `liquid_window`
  - `LiquiditySweepReclaim15mWide`: `sweep_and_reclaim`, `body_strength_25`
  - `PersistenceClusterDense15m`: `liquid_window`, `persistence`
  - `VRPCompression15m`: `liquid_window`, `bullish_body`, `not_collapsing`
- [x] For every approved relaxation, recorded:
  - baseline entry density
  - relaxed entry density
  - density lift
  - number of zero-entry windows fixed
  - obvious-overtrading assessment
  - disposition:
    - `kept unchanged`
    - `relaxed`
    - `split by regime`
    - `retired`
    - `deferred for more data`
- [x] Audited the `ict-engine analyze` import path only after the matrix and gate-relaxation report artifacts existed; did not fake a before/after run.
- [x] Recorded the exact analyze blocker plus the missing next-step artifact in:
  - `/tmp/ict-engine-changepoint-matrix/analyze/analyze_blocked.json`
- [x] Appended the real results back into this same markdown in the required final-report shape.

### Not Yet

- [ ] CatBoost policy-surface training
- [ ] New strategy families
- [ ] Cross-market expansion beyond NQ
- [ ] Regime-completeness claims
- [ ] Promotion of any strategy based only on density lift

## Artifact Contract

The slice should aim to create these artifact groups under `/tmp/ict-engine-changepoint-matrix/`:

- `15m/`
  - `changepoints.json`
  - `segment_summary.json`
  - `transition_metrics.json`
  - `warning_summary.json`
- `1h/`
  - same shape
- `4h/`
  - same shape
- `1d/`
  - same shape
- `comparison/`
  - `regime_classifier_transition_comparison.json`
  - or `comparison_blocked.json`
- `gate_relaxation/`
  - one report per candidate
  - one merged summary report

If the exact file names need to differ for implementation reasons, keep the directory intent but document the final actual names in the result writeback.

## Ordered Execution Checklist

1. Verify the reachable NQ data paths for `15m/1h/4h/1d` and lock the actual span used per timeframe.
2. Run `window`-only change-point detection across the full NQ ladder and persist artifacts per timeframe.
3. Compute duration / density / warning summaries for each timeframe.
4. Measure runtime. Only if acceptable, add `PELT` and/or `Binseg`; otherwise record `window-only` as the bounded matrix for this slice.
5. Find and reuse current regime-classifier outputs if they already exist.
6. Compute transition comparison metrics; if unavailable, write a blocked artifact instead of stalling.
7. Turn `entry_drought_diagnostic_v2.py` into targeted gate-relaxation measurements for only the approved gates.
8. Classify each candidate as:
   - unchanged
   - relaxed
   - split by regime
   - retired
   - deferred
9. Attempt analyze/import only if there is a clean path from relaxed candidates into the existing runtime/analyze surfaces.
10. Write the final result in the exact required report shape and update this file in place.

## Success Standard

This slice is successful only if both of these are true:

- a full NQ `15m/1h/4h/1d` change-point matrix exists, even if some algorithms are intentionally bounded to `window` because of runtime budget;
- a targeted gate-relaxation report exists for the four current 15m candidates.

This slice is still considered successful if:

- regime-classifier comparison is blocked, but the block is made explicit with a persisted artifact;
- analyze/import is blocked, but the block is made explicit with a persisted artifact.

This slice is not successful if:

- it drifts into new strategy families;
- it starts CatBoost work early;
- it reports density improvements without the targeted-gate disposition;
- it calls change-point validation complete without the full NQ ladder metrics.

## Expected Final Report Shape

When the slice is actually executed, the final result should be written in this shape:

```md
## 2026-05-07 Next Slice Result: change-point matrix + targeted gate relaxation

### what was run
### exact commands
### data span
### market/timeframe coverage
### change-point outputs
### transition validation metrics
### entry-drought gate-relaxation results
### execution-tree before/after
### blockers
### whether the result changes
- regime confidence:
- BBN evidence quality:
- CatBoost policy vote:
- execution readiness:
- portfolio diversity:

### conclusion
```

## 2026-05-07 Next Slice Result: change-point matrix + targeted gate relaxation

### what was run

* Generated a full NQ `15m/1h/4h/1d` window-only change-point matrix under `/tmp/ict-engine-changepoint-matrix/`.
* Persisted per-timeframe:
  * `changepoints.json`
  * `segment_summary.json`
  * `transition_metrics.json`
  * `warning_summary.json`
* Generated targeted gate-relaxation reports for:
  * `TomacNQ_RegimeTrendPullbackDense15m`
  * `TomacNQ_RegimeLiquiditySweepReclaim15mWide`
  * `TomacNQ_RegimePersistenceClusterDense15m`
  * `TomacNQ_RegimeVRPCompression15m`
* Audited reuse of existing regime-benchmark outputs and wrote:
  * `/tmp/ict-engine-changepoint-matrix/comparison/existing_output_inventory.json`
  * `/tmp/ict-engine-changepoint-matrix/comparison/comparison_blocked.json`
* Audited runtime before/after reachability and wrote:
  * `/tmp/ict-engine-changepoint-matrix/analyze/analyze_blocked.json`

### exact commands

* Change-point matrix:
  * `uv run --with pandas --with numpy --with pyarrow --with ruptures python3 - <<'PY'`
  * inline script imported `external_regime_changepoint_labels.py`, ran `window` across `NQ_USD-15m/1h/4h/1d.feather`, and wrote `/tmp/ict-engine-changepoint-matrix/{15m,1h,4h,1d}/*`
  * `PY`
* Gate relaxation:
  * `uv run --with pandas --with numpy --with pyarrow python3 - <<'PY'`
  * inline script imported `entry_drought_diagnostic_v2.py`, rebuilt the `2018-01-01` to `2025-12-31` NQ base frame, ran approved single-gate ablation upper bounds only, and wrote `/tmp/ict-engine-changepoint-matrix/gate_relaxation/*`
  * `PY`
* Analyze-path audit:
  * `/Users/thrill3r/projects-ict-engine/ict-engine/target/debug/ict-engine analyze --help`

### data span

* Change-point matrix actual spans:
  * `15m`: `2011-01-02T23:00:00Z` to `2025-12-31T21:45:00Z`
  * `1h`: `2011-01-02T23:00:00Z` to `2025-12-31T21:00:00Z`
  * `4h`: `2011-01-02T20:00:00Z` to `2025-12-31T20:00:00Z`
  * `1d`: `2011-01-02T00:00:00Z` to `2025-12-31T00:00:00Z`
* Gate-relaxation span:
  * `2018-01-01` to `2025-12-31`
  * based on `NQ` `15m` with `1h` and `4h` merged context, plus daily `QQQ` IV/HV where required

### market/timeframe coverage

* Market:
  * `NQ` only, by slice lock
* Change-point matrix timeframes:
  * `15m`
  * `1h`
  * `4h`
  * `1d`
* Gate-relaxation coverage:
  * current four `15m` candidates only

### change-point outputs

* Matrix root:
  * `/tmp/ict-engine-changepoint-matrix/matrix_run_summary.json`
* Per-timeframe counts:
  * `15m`: `351288` bars, `12` clustered breakpoints, `13` segments, `4.665s`
  * `1h`: `89250` bars, `12` clustered breakpoints, `13` segments, `0.845s`
  * `4h`: `23879` bars, `12` clustered breakpoints, `13` segments, `0.226s`
  * `1d`: `4651` bars, `12` clustered breakpoints, `13` segments, `0.064s`
* Bounded algorithm choice:
  * this slice closed with `window_only`
  * all four ladders hit `breakpoint_cap_hit`, so `PELT` / `Binseg` should be deferred until after this measurement/report closure rather than reopened mid-slice

### transition validation metrics

* This slice produced helper-level change-point transition metrics, not full classifier-vs-matrix comparison metrics.
* Transition density from the new matrix:
  * `15m`: `0.000376`
  * `1h`: `0.001479`
  * `4h`: `0.005528`
  * `1d`: `0.028381`
* Segment duration medians:
  * `15m`: `10135` bars
  * `1h`: `5315` bars
  * `4h`: `795` bars
  * `1d`: `205` bars
* Warning summary:
  * all four timeframes hit `breakpoint_cap_hit`
  * `1h` and `1d` also hit `over_detection_risk` because at least one segment is shorter than the configured window width
  * all four timeframes remained `sparse_transition_density`

### entry-drought gate-relaxation results

* Merged report:
  * `/tmp/ict-engine-changepoint-matrix/gate_relaxation/merged_summary.json`
* `TomacNQ_RegimeTrendPullbackDense15m`
  * baseline density: `0.238631`
  * `reacceleration -> 0.450209` (`+0.211578`), `high_overtrading_risk`, candidate disposition `split by regime`
  * `liquid_window -> 0.375921` (`+0.137290`), `moderate_overtrading_risk`
* `TomacNQ_RegimeLiquiditySweepReclaim15mWide`
  * baseline density: `0.010310`
  * `sweep_and_reclaim -> 0.162758` (`+0.152448`), `moderate_overtrading_risk`
  * `body_strength_25 -> 0.031689` (`+0.021380`), `no_obvious_overtrading`, candidate disposition `deferred for more data`
* `TomacNQ_RegimePersistenceClusterDense15m`
  * baseline density: `0.319375`
  * `liquid_window -> 0.495850` (`+0.176476`), `high_overtrading_risk`, candidate disposition `split by regime`
  * `persistence -> 0.401718` (`+0.082344`), `moderate_overtrading_risk`
* `TomacNQ_RegimeVRPCompression15m`
  * baseline density: `0.029796`
  * `liquid_window -> 0.079999` (`+0.050203`)
  * `bullish_body -> 0.050926` (`+0.021130`)
  * `not_collapsing -> 0.043280` (`+0.013484`)
  * all three approved relaxations stayed `no_obvious_overtrading`; candidate disposition `relaxed`

### execution-tree before/after

* No runtime before/after was executed.
* The import-path audit happened only after the matrix and gate-relaxation artifacts existed.
* Result:
  * `ict-engine analyze` is reachable as a binary surface
  * this slice did not produce a runtime-consumable relaxed-candidate artifact that `analyze` can score before/after

### blockers

* Comparison blocker:
  * `/tmp/ict-engine-changepoint-matrix/comparison/comparison_blocked.json`
  * existing regime-benchmark JSON files expose aggregate `ranked_results` only, not the bar-level classifier outputs needed for early/late/noisy/missing transition auditing against the new matrix
* Analyze/import blocker:
  * `/tmp/ict-engine-changepoint-matrix/analyze/analyze_blocked.json`
  * helper-layer gate-relaxation JSON is not a direct `ict-engine analyze` import surface, so forcing a before/after run here would be fake closure

### whether the result changes

* regime confidence:
  * slightly up on measurement discipline only
  * not up on closure; the new matrix says the current `window` segmentation is still sparse and cap-limited
* BBN evidence quality:
  * unchanged
* CatBoost policy vote:
  * unchanged
* execution readiness:
  * unchanged at runtime level
  * improved at helper level because approved gate relaxations now have explicit density/risk bounds instead of only `over_gating_issue` narratives
* portfolio diversity:
  * unchanged

### conclusion

* This slice cleared the two required closure gates:
  * a full NQ `15m/1h/4h/1d` change-point matrix now exists
  * a targeted gate-relaxation report now exists for the four current `15m` candidates
* It did not clear runtime comparison closure:
  * classifier comparison is explicitly blocked by missing bar-level current-classifier outputs
  * `ict-engine analyze` before/after is explicitly blocked by the missing runtime-consumable relaxed-candidate artifact
* The next correct move is narrower than “more ideas”:
  * export bar-level classifier outputs for the chosen current classifier across `15m/1h/4h/1d`
  * materialize at least one relaxed candidate into an analyze-consumable path
  * then re-run the comparison/before-after slice without reopening runtime code

## Conclusion

This evaluation is good enough to adopt, but only after tightening the wording around two boundaries:

- helper-level `over_gating` evidence is actionable, not final runtime closure;
- bounded change-point smoke is proof-of-path, not ladder completion.

So the right next move is not “继续写评价” and not “直接开 CatBoost”, but a narrower board:

- finish the NQ change-point matrix;
- finish the targeted gate-relaxation report;
- then decide whether the next slice should continue regime/gate closure or finally move to CatBoost policy surface.
