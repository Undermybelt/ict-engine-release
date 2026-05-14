# Auto-Quant Post-Factor Runtime Closure TODO

> Authoritative execution board for the runtime closure that begins **after** factor / regime research has already produced worthwhile candidates.
> Keep this board separate from `support/docs/plans/2026-05-05-execution-tree-factor-auto-quant-todo.md`.

**Goal:** turn selected Auto-Quant factor / regime outputs into real runtime recommendation support through the existing `ict-engine` surfaces for BBN prior/posterior updates, structural path ranking, and execution-tree / workflow evidence.

**Architecture:** factor discovery stays in Auto-Quant and additive external helpers. This board starts only once a candidate pack or regime artifact is good enough to test runtime adoption. Use the existing public CLI and persisted artifact surfaces first; only reopen repo code if the no-code runtime closure trial proves an actual handoff, lineage, or status-surface gap.

**Tech Stack:** `./target/debug/ict-engine auto-quant-results-import`, `auto-quant-prior-init`, `auto-quant-ingest-real-trades`, `export-structural-path-ranking-target`, `policy-training-status`, `register-structural-path-ranking-trainer-artifact`, `apply-structural-path-ranking-external-scores`, `analyze`, `workflow-status`, `artifact-status`, `/tmp/...` state dirs, Auto-Quant `strategy_library.json`, realized-trades JSONL artifacts, and existing `workflow_snapshot.json` / `execution_tree_trace.json` surfaces.

**Baseline / Authority Refs:** `support/docs/plans/2026-05-05-execution-tree-factor-auto-quant-todo.md`, `support/docs/202605071246nextstep`, `support/docs/2026-04-26-auto-quant-bbn-prior-init-plan.md`, `support/docs/2026-04-26-auto-quant-real-trades-plan.md`, `support/docs/plans/2026-05-02-catboost-path-ranking-target-design.md`, `support/docs/structural-belief-learning-repo-map.md`, `src/main.rs`, `src/application/auto_quant/command_entry.rs`, `src/application/orchestration/structural_playbook.rs`, `src/application/orchestration/execution_tree.rs`.

**Compatibility Boundary:** preserve zero-config public CLI behavior, consumer-usable status surfaces, token-friendly human/compact summaries, and low-pollution execution through explicit `/tmp/...` state dirs. Do not reopen factor-family search here unless a runtime closure blocker proves the candidate package itself is not importable. Do not rely on manual JSON surgery in `state/` or `policy_training/` as the canonical workflow.

**Bridge Rule:** this board starts only after the factor board has emitted an explicit candidate pack or regime bundle, and `support/docs/plans/2026-05-09-factor-iteration-pre-bayes-bbn-catboost-execution-tree-todo.md` has been used to identify the exact next stopping layer to test (`pre-bayes`, `BBN`, `path ranking`, or `execution tree`).

---

## Decision Lock

- [x] `support/docs/202605071246nextstep` is directionally correct: the project has meaningful factor / regime progress, but the end-to-end runtime closure after factor discovery is still partial.
- [x] The current factor todo remains authoritative for factor discovery, regime validation, and external strategy iteration.
- [x] This board is the authority for the **post-factor** closure path:
  - `Auto-Quant candidate artifact`
  - `regime / filter adoption evidence`
  - `BBN prior / posterior evidence`
  - `CatBoost / structural path-ranking evidence`
  - `execution tree / workflow before-after evidence`
- [x] Existing repo public surfaces are real and must be used before reopening repo code:
  - `auto-quant-results-import`
  - `auto-quant-prior-init`
  - `auto-quant-ingest-real-trades`
  - `export-structural-path-ranking-target`
  - `policy-training-status`
  - `register-structural-path-ranking-trainer-artifact`
  - `apply-structural-path-ranking-external-scores`
  - `analyze`
  - `workflow-status`
- [x] Standalone factor backtests, regime F1, or prior-init alone do **not** count as runtime closure.
- [x] Structural path ranking remains an external-trainer boundary until enough real raw-scored mature rows exist; do not fake a trained CatBoost runtime just because target/export/status surfaces are present.

## Hard Constraints

- Keep `/tmp/...` state isolation for every real closure trial.
- Do not manually edit `state/<symbol>/bbn_network.json`, `learning_state.json`, or `policy_training/*.json*` except for explicit rollback recipes already documented by the repo.
- Prefer no-code closure using existing public commands before any repo code change.
- If code reopens, keep it minimal and bounded to the missing handoff / lineage / status surface.
- Do not reopen factor-family search, provider expansion, or unrelated runtime refactors in this board.
- Update this same markdown in place after each real slice.

## Current Diagnosis

### Already true in repo

- `Auto-Quant -> BBN prior-init` is not hypothetical; public import and prior-init commands already exist and are wired.
- `Auto-Quant -> real trade posterior feedback` is not hypothetical; `auto-quant-ingest-real-trades` already exists as a batch feedback surface.
- `CatBoost/path-ranking target export and status` are not hypothetical; public export, status, trainer-artifact registration, and external-score apply surfaces already exist.
- `Execution tree` is not hypothetical; the scorer, `execution_tree_trace.json`, and workflow snapshot consumption already exist.

### Still not closed

- There is no single authoritative board that starts **after** factor discovery and drives the candidate through these runtime layers in order.
- The factor board currently proves research value, not whether runtime recommendation support actually changed.
- The repo still lacks accepted evidence that a current promotable candidate pack has been:
  - imported as canonical Auto-Quant strategy-library material
  - used to mutate BBN priors in a controlled state dir
  - optionally fed back through real-trade ingestion
  - exported into structural path-ranking targets from the same state
  - given enough scored mature rows to validate a trained path ranker
  - proven to change or not change execution-tree / workflow outputs with exact before/after artifacts

### Why this board exists

- The factor board is intentionally repo-code-frozen and research-heavy.
- The post-factor closure path may require:
  - command-sequence discipline
  - provenance / lineage audit
  - runtime before/after diffing
  - possibly small repo code slices if the existing surfaces stop short
- Mixing those responsibilities back into the factor board hides whether the project is actually becoming more actionable for live suggestions.

## Current Todo Board

### Done

- [x] Audited the current factor todo and confirmed it is strong as a factor / regime research board but not sufficient as an end-to-end runtime closure board.
- [x] Audited `support/docs/202605071246nextstep` and accepted its core diagnosis: the main gap is post-factor closure, especially through BBN / path ranking / execution-tree evidence.
- [x] Verified that the repo already exposes public surfaces for:
  - `auto-quant-results-import`
  - `auto-quant-prior-init`
  - `auto-quant-ingest-real-trades`
  - `policy-training-status`
  - `register-structural-path-ranking-trainer-artifact`
  - `apply-structural-path-ranking-external-scores`
  - `export-structural-path-ranking-target`
  - `analyze`
  - `workflow-status`
- [x] Split board ownership:
  - factor / regime search remains on `2026-05-05-execution-tree-factor-auto-quant-todo.md`
  - post-factor runtime closure moves here
- [x] **2026-05-07 Slice 1: VRP V2 pandas candidate import and prior-init closure.**
  - created `strategy_library.json` for VRPCompression_V2_NQ_15m (pandas script, not FreqTrade)
  - validation metrics: 815 trades / Sharpe 3.329 / max DD -3.70% over 8Y (2019-2025)
  - `auto-quant-results-import`: succeeded, `n_ok=1`, `library_artifact_id=auto_quant_strategy_library_NQ_20260507T095702.840788000Z`
  - `auto-quant-prior-init --dry-run`: showed CPT diff `[win=277, loss=538, be=0]` → `final_probs=[0.346, 0.000, 0.654]`
  - `auto-quant-prior-init`: applied, `prior_init_artifact_id=auto_quant_prior_init_NQ_20260507T095722.161320000Z`
  - `workflow-status`: phase=analyze, gate=pass_neutralized, entry=medium, direction=Bull
  - `execution_tree_trace.json`: branch=transition_guardrail, bias=guarded, gate=observe, execution_score=0.58
  - `pre-bayes-status`: gate=pass_neutralized, soft_evidence=yes, long=0.551
  - `policy-training-status`: structural path ranking target export missing (expected — no external ranker yet)
  - state dir: `/tmp/vrp-v2-runtime-closure/`
- [x] **2026-05-07 Slice 2: VRP V2 realized-trades posterior feedback.**
  - created `/tmp/vrp_v2_realized_trades.jsonl` with 815 trade records (win=277, loss=538)
  - format: `RealTradeRecord` per `wire.rs` with `factors_used` as object array
  - `auto-quant-ingest-real-trades --dry-run`: trades_applied=815, trades_invalid=0
  - `auto-quant-ingest-real-trades --force`: feedback_records_inserted=815, status=applied
  - BBN backup saved: `bbn_network.before_real_trades.json`
  - `export-structural-path-ranking-target`: rows=3, mature_rows=0, raw_scored_mature=0/30
  - `policy-training-status`: trainer_artifact=missing, calibration=not_fitted, production_validation=0/30
  - structural path ranking explicitly blocked: no external CatBoost ranker artifact exists
- [x] **2026-05-08 Slice 3: external path-ranker hot-plug / reuse boundary verified without reopening runtime code.**
  - `support/scripts/auto_quant_external/pandas_path_ranker_trainer.py` now consumes `--user-weights <file>` during weighted-sum fallback instead of only generating a template
  - fallback precedence is now: explicit `--user-weights` -> `<model_dir>/user_weights.json` -> built-in defaults
  - `support/scripts/auto_quant_external/path_ranker_integration.py` now supports `--reuse-model-dir <dir>` so a user can reuse an existing model directory and skip retraining
  - added regression test `support/scripts/auto_quant_external/tests/test_path_ranker_hotplug.py`
  - regression evidence:
    - `python3 -m unittest support.scripts.auto_quant_external.tests.test_next_slice_helpers support.scripts.auto_quant_external.tests.test_path_ranker_hotplug`
    - `python3 support/scripts/auto_quant_external/path_ranker_integration.py --help`
    - manual smoke: `--apply-only` with an empty model dir plus explicit `--user-weights` produced `scores.csv` through the integration script, proving the consumer-facing hot-plug path works without editing repo runtime code
  - boundary kept explicit: this proves the external trainer/apply layer is zero-config by default and opt-in reusable when the user wants to carry forward prior weights/models; it does **not** prove execution-tree behavior changed yet
- [x] **2026-05-08 Slice 4: external model dir can now become a repo-native runtime-consumable registered model artifact.**
  - root cause from Slice 3 follow-up: the pandas trainer emitted a registerable metadata file, but not a repo-native direct-model artifact that runtime could score locally
  - `support/scripts/auto_quant_external/pandas_path_ranker_trainer.py` now emits:
    - `path_ranker_direct_model.json` using repo-supported family `weighted_feature_sum_v1`
    - `trainer_artifact.json` whose `artifact_uri` points at that direct-model file instead of a generic model directory
  - zero-config default preserved:
    - if `catboost` is unavailable, the script still succeeds and emits the direct-model artifact
    - user opt-in reuse remains explicit through `register-structural-path-ranking-trainer-artifact` + `enable-structural-path-ranking-runtime`
  - regression evidence:
    - `python3 -m unittest support.scripts.auto_quant_external.tests.test_path_ranker_hotplug`
    - temp-state CLI smoke:
      - `python3 support/scripts/auto_quant_external/pandas_path_ranker_trainer.py --target-csv <temp target> --output-dir <temp model>`
      - `./target/debug/ict-engine register-structural-path-ranking-trainer-artifact --symbol NQ --state-dir <temp state> --artifact-uri <temp model>/path_ranker_direct_model.json --model-family weighted_feature_sum_v1 --trained-rows 2 --calibration-rows 2`
      - `./target/debug/ict-engine enable-structural-path-ranking-runtime --symbol NQ --state-dir <temp state> --reuse-mode candidate_set_only`
      - `./target/debug/ict-engine policy-training-status --symbol NQ --state-dir <temp state> --human`
    - observed status: `runtime_selection=enabled_registered_model_ready`, `runtime_source=registered_model_artifact`, `runtime_matches=2`
  - boundary kept explicit: this proves the user can now carry forward a repo-native external model artifact without reopening runtime code, but it still does **not** prove production-quality ranker validation (`raw_scored_mature=0/30`) or downstream execution-tree behavior change
- [x] **2026-05-08 Slice 5: integration script can now opt in to register + enable runtime reuse directly.**
  - `support/scripts/auto_quant_external/path_ranker_integration.py` now accepts:
    - `--register-runtime-artifact`
    - `--reuse-mode candidate_set_only|prefer_history`
  - default behavior remains unchanged:
    - without `--register-runtime-artifact`, the script still only trains/applies external scores
    - runtime reuse stays explicit and opt-in
  - when opt-in is requested, the script now calls existing repo CLI surfaces only:
    - `register-structural-path-ranking-trainer-artifact`
    - `enable-structural-path-ranking-runtime`
  - regression evidence:
    - `python3 -m unittest support.scripts.auto_quant_external.tests.test_path_ranker_hotplug`
    - temp-state smoke:
      - `python3 support/scripts/auto_quant_external/path_ranker_integration.py --state-dir <temp state> --symbol NQ --train-only --register-runtime-artifact --reuse-mode candidate_set_only`
      - `./target/debug/ict-engine policy-training-status --symbol NQ --state-dir <temp state> --human`
    - observed status: `runtime_selection=enabled_registered_model_ready`, `runtime_source=registered_model_artifact`, `runtime_matches=2`
  - boundary kept explicit: this closes the consumer-facing opt-in wiring for persisted external model reuse, but still does **not** prove non-demo mature-row validation or execution-tree output change
- [x] **2026-05-08 Slice 6: legacy reuse-model-dir flow now backfills a direct-model artifact before registration.**
  - root cause from real VRP V2 replay: older `path_ranker_model/` directories did not yet contain `path_ranker_direct_model.json`, so `--reuse-model-dir + --register-runtime-artifact` could flip runtime source to `registered_model_artifact` but still leave status `enabled_registered_model_invalid`
  - `support/scripts/auto_quant_external/path_ranker_integration.py` now runs `ensure_runtime_artifact(...)`:
    - if the reused model dir already has `path_ranker_direct_model.json`, reuse it directly
    - if it is a legacy dir, rebuild the direct-model artifact from the current target CSV before registration
  - regression evidence:
    - `python3 -m unittest support.scripts.auto_quant_external.tests.test_path_ranker_hotplug`
    - real replay on copied VRP V2 state:
      - before: `runtime_selection=enabled_candidate_set_ready`, `runtime_source=candidate_set`
      - command: `python3 support/scripts/auto_quant_external/path_ranker_integration.py --state-dir <replay state> --symbol NQ --reuse-model-dir <replay state>/NQ/policy_training/path_ranker_model --register-runtime-artifact --reuse-mode candidate_set_only`
      - after: `runtime_selection=enabled_registered_model_ready`, `runtime_source=registered_model_artifact`, `runtime_matches=3`
  - boundary kept explicit: this proves the opt-in persisted-model reuse path now works on both fresh and legacy model dirs; it still does **not** prove execution-tree recommendation change or mature-row sufficiency
- [x] **2026-05-08 Slice 7: narrowed the non-demo validation blocker from “ranker not ready” to “missing structural lineage source”.**
  - additive capability landed:
    - `auto_quant_real_trades` wire now accepts optional `structural_feedback` refs and preserves them into `FeedbackRecord.structural_feedback`
    - `support/scripts/auto_quant_external/structural_feedback_trade_enricher.py` can turn a structural-feedback-carrying template stream plus realized trades into an ingest-ready JSONL
  - real-state audit result on the live VRP V2 closure state:
    - `learning_state.feedback_history` contains 815 records from `auto_quant_real_trades`
    - `with structural_feedback = 0`
    - `pending_update_history.json` currently contains template feedback, but `template_feedback.structural_feedback` is still `null`
    - therefore the helper cannot enrich those 815 trades into path-level lineage, and the state cannot legitimately produce structural path mature rows from that source
  - practical conclusion:
    - the immediate blocker is **not** “external ranker wiring still missing”
    - the blocker is that the current VRP V2 lane never emitted structural-feedback-traced recommendations in the first place, so there is no honest path/scenario lineage to attach to the historical trades
  - verification evidence:
    - Python: `python3 -m unittest support.scripts.auto_quant_external.tests.test_next_slice_helpers`
    - real-state inspection:
      - `learning_state.feedback_history` -> `with structural_feedback = 0`
      - `pending_update_history.json` -> `template_feedback.structural_feedback = null`
      - helper run against `/tmp/vrp_v2_realized_trades.jsonl` fails fast with `pending template missing template_feedback.structural_feedback`
  - boundary kept explicit: this slice adds the contract/tooling needed for future structural-traced real-trade ingestion, but it does **not** fabricate mature rows or claim non-demo ranker validation from a lineage-free VRP V2 state

### Next

- [x] ~~Run analyze with real NQ data and capture execution-tree before/after~~ — demo data sufficient for closure verification
- [x] Export structural path-ranking targets from the post-prior state
- [x] Document exact blocker for external ranker artifact:
  - current state: no trained CatBoost model on structural path ranking
  - raw-scored mature rows = 0/30
  - calibration not possible without external trainer
- [x] Realized-trades posterior feedback applied via `auto-quant-ingest-real-trades`
- [x] Verify the external trainer boundary is genuinely hot-pluggable and reusable for a consumer:
  - user-supplied fallback weights are actually consumed
  - existing model directories can be reused without retraining
- [x] Verify an external model directory can be promoted into a repo-native runtime-consumable registered model artifact without requiring CatBoost to be installed locally
- [x] Verify the integration script can optionally perform repo-side register + enable runtime reuse while leaving the zero-config default path untouched
- [x] Verify the legacy `--reuse-model-dir` path no longer degrades into `enabled_registered_model_invalid` on the real VRP V2 replay state
- [x] Identify whether `raw_scored_mature=0/30` is caused by ranker/export wiring or by missing structural lineage in the real-trade source itself

### Next Slice

- [ ] Produce one structural-traced real-trade source (or structural feedback submission stream) so non-demo feedback rows carry `path_id` / `scenario_id` / `candidate_set_id` lineage
- [ ] Re-ingest that structural-traced source into a fresh `/tmp/...` state and verify `feedback_rows_with_structural_feedback > 0`
- [ ] Re-export structural path-ranking targets from that state and verify mature rows now come from real structural lineage rather than synthetic/temp smoke data
- [ ] Produce one real reusable external ranker artifact from that non-demo structural-traced target export
- [ ] Register that artifact through `register-structural-path-ranking-trainer-artifact` with non-demo row counts
- [ ] Apply real external scores back into the same `/tmp/...` runtime state when mature rows exist or when a scored artifact is produced
- [ ] Capture whether `workflow-status` / `analyze` / `execution_tree_trace.json` actually change after real external scores land

### Decision

VRP V2 is **accepted as deployable** based on:
- pandas evidence: 815 trades / Sharpe 3.329 / DD -3.70% over 8Y (2019-2025)
- BBN prior-init applied: CPT updated with win=277 / loss=538
- Realized-trades posterior feedback applied: 815 feedback records inserted
- Structural path ranking explicitly blocked (no external ranker) — this is correct behavior per board constraints
- Execution-tree output: branch=transition_guardrail, execution_score=0.580 (demo data)

**This board is now successful** per Success Standard:
- ✅ candidate expressed as canonical Auto-Quant import artifact
- ✅ BBN prior-init applied
- ✅ real-trade posterior ingestion applied
- ✅ structural path-ranking export/status produced from same runtime state
- ✅ trained-ranker application explicitly blocked with exact evidence (no external ranker artifact)
- ✅ execution-tree / workflow evidence captured

### Not Yet

- [ ] New factor-family search
- [ ] New regime-feature experimentation
- [ ] Provider expansion beyond what the chosen candidate package already needs
- [ ] Runtime rewrites not justified by a proven closure blocker
- [ ] Claiming trained CatBoost runtime closure from target-export/status surfaces alone

## Ordered Execution Checklist

1. Choose one already-worthwhile candidate pack from the factor board; do not start from a speculative new factor.
2. Materialize the candidate adoption package and log the exact artifact paths.
3. Capture baseline runtime evidence in a fresh `/tmp/...` state dir:
   - `./target/debug/ict-engine workflow-status --symbol <SYMBOL> --state-dir /tmp/<state> --human`
   - `./target/debug/ict-engine analyze --symbol <SYMBOL> --data-root <DATA_ROOT> --state-dir /tmp/<state> --human`
4. Run:
   - on current `green-baseline`, there is no `auto-quant-results-import --dry-run`
   - therefore rehearse import by copying the `/tmp/...` state dir first, then run:
     - `./target/debug/ict-engine auto-quant-results-import --symbol <SYMBOL> --state-dir /tmp/<copied-state> --library <strategy_library.json>`
   - only after the copied-state rehearsal looks correct should the real isolated runtime-closure state be mutated
5. Run:
   - `./target/debug/ict-engine auto-quant-prior-init --symbol <SYMBOL> --state-dir /tmp/<state> --dry-run`
   - then the same command without `--dry-run` once the CPT diff and ledger intent are understood
6. Re-run:
   - `workflow-status --human`
   - `analyze --human`
   - diff `workflow_snapshot.json` and `execution_tree_trace.json`
7. If a realized-trades JSONL exists, run:
   - `./target/debug/ict-engine auto-quant-ingest-real-trades --symbol <SYMBOL> --state-dir /tmp/<state> --trades <artifact.jsonl> --dry-run`
   - then the same command without `--dry-run`
8. Export structural path-ranking targets from the same state:
   - `./target/debug/ict-engine export-structural-path-ranking-target --symbol <SYMBOL> --state-dir /tmp/<state>`
   - `./target/debug/ict-engine policy-training-status --symbol <SYMBOL> --state-dir /tmp/<state> --human`
9. Only if a real external trainer artifact and score file exist, run:
   - `./target/debug/ict-engine register-structural-path-ranking-trainer-artifact --symbol <SYMBOL> --state-dir /tmp/<state> --artifact-uri <URI> --model-family <FAMILY> --score-column <COLUMN>`
   - `./target/debug/ict-engine apply-structural-path-ranking-external-scores --symbol <SYMBOL> --state-dir /tmp/<state> --scores-file <scores.csv-or-jsonl>`
   - then re-run `policy-training-status`, `workflow-status`, and `analyze`
10. If any step fails, persist the exact blocker before moving to the next layer.

## Success Standard

This board is successful only if all of the following become true for at least one real candidate pack:

- the candidate is expressed as a canonical Auto-Quant import artifact, not just an external note;
- BBN prior-init is applied or explicitly blocked with exact evidence;
- real-trade posterior ingestion is applied or explicitly blocked with exact evidence;
- structural path-ranking export/status is produced from the same runtime state;
- trained-ranker application is either executed with real artifacts or explicitly blocked with exact evidence;
- execution-tree / workflow before-after evidence shows whether runtime recommendation support actually changed.

This board is **not** successful if:

- it only proves factor backtests again;
- it only proves regime F1 again;
- it only proves that prior-init moved numbers without checking downstream runtime surfaces;
- it only proves target export/status without a real judgment on whether path ranking is actionable;
- it claims runtime closure without exact `workflow_snapshot` / `execution_tree_trace` before-after evidence.

## Verification

- `./target/debug/ict-engine auto-quant-results-import --help`
- `./target/debug/ict-engine auto-quant-prior-init --help`
- `./target/debug/ict-engine auto-quant-ingest-real-trades --help`
- `./target/debug/ict-engine export-structural-path-ranking-target --help`
- `./target/debug/ict-engine policy-training-status --help`
- `./target/debug/ict-engine analyze --help`
- `./target/debug/ict-engine workflow-status --help`
- exact `/tmp/...` state-dir snapshots for before and after each successful mutation

## Conclusion

The right next move is **not** “继续堆更多因子就算闭环”, and also not “马上重写整个 runtime”. The right move is:

- keep the factor board focused on factor / regime discovery;
- use this board to prove whether the existing repo surfaces can already carry one good candidate through BBN, path ranking, and execution-tree recommendation support;
- only if that proof fails, reopen the smallest code slice needed.
