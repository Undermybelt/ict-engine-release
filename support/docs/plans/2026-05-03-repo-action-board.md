# Structural Belief Repo Action Board

> Authoritative execution board for the remaining structural-belief / transition / path-ranking closure work. This file intentionally keeps only actionable remaining work and execution rules. Historical reconciliation notes and long landed-item ledgers do not belong here anymore.

**Goal:** close the remaining structural-belief / transition / path-ranking work without reopening already-good-enough profile polish and without breaking zero-config, consumer-usable, token-friendly, low-pollution behavior.

**Source Docs:** `support/docs/plans/20260501repo.md`, `support/docs/plans/2026-04-30-structural-belief-execution-plan.md`

**Do Not Reopen By Default:** provider-profile polish, wording-only surface cleanup, maintainer-local default reuse, or any repo-owned consumer stance that is not required by an unchecked item below.

---

## Hard Constraints

- Preserve the public `node -> branch -> scenario -> path` contract.
- Preserve zero-config default behavior. Any history/profile/runtime artifact reuse must remain explicit opt-in behavior.
- Keep consumer surfaces token-friendly and consumer-usable.
- Do not leak maintainer-local paths, maintainer-local state, or user-specific artifact URIs into default surfaces.
- Keep the repo low-pollution and low-debt. Prefer extraction and deletion of duplicate logic over adding one more wrapper layer.
- Do not spend the current closure slice on surface proliferation when owner closure or verification hardening is still open.

## New Trial Constraints

This board now absorbs one concrete contributor trial result:

- A layered Bayesian network is still a valid direction, but the trial input here is only a source of constraint and anti-pattern evidence, not an already-approved implementation path.
- If a future first-pass layer decomposition is attempted, it should stay simple and inspectable rather than jumping straight to a high-complexity regime pipeline.
- A small-zigzag + tiny-leg pipeline can still be useful, but only as retrospective segmentation and regime summarization.
- Do **not** let zigzag / delayed pivot confirmation become the only live regime judge.
- The runtime path must separate:
  - confirmed retrospective structure
  - current live regime evidence
- If a regime method depends only on confirmed pivots, it is too delayed to be the sole runtime truth.

## What To Keep vs Reject From This Trial

Keep:

- hardcoded first-pass layer probabilities are acceptable if they stay inspectable and replaceable later
- six-period / multi-period evidence comparison is useful
- tiny-leg features are useful for retrospective clustering:
  - leg slope
  - path efficiency
  - leg length
  - leg time
  - max drawdown within leg
- `16 -> 6` clustering/merge can be a research path for retrospective regime discovery

Reject:

- zigzag alone as runtime regime truth
- pivot confirmation alone as current-state evidence
- any pipeline that turns delayed structural confirmation directly into the only regime trigger
- any design that hides layer-by-layer probability contribution from the operator or agent
- directly copying a contributor's trial vocabulary into repo truth before mapping it to existing repo concepts

## Agent Contract

1. Read this file and the two source docs before changing code.
2. Treat this file as the implementation contract for the current slice. Do not renegotiate scope unless blocked by concrete repo evidence.
3. Pick exactly one unchecked item as the active slice. Do not mix multiple workstreams into one vague pass.
4. Before editing, identify the owner files and the exact verification commands for the chosen slice.
5. When touching shared files such as `src/state/types.rs`, `src/application/orchestration/workflow_status.rs`, `src/application/orchestration/structural_playbook.rs`, or `src/main.rs`, reduce ownership instead of adding more inline math or more glue.
6. Do not spend a slice on provider-profile polish, payload cosmetics, or wording cleanup unless an unchecked item explicitly requires it.
7. A slice is not done until code, targeted verification, and this markdown are all updated.
8. After a slice lands, update this same markdown and make a clean commit for that slice. Do not create a new board doc.
9. If the worktree is dirty in unrelated files, isolate your slice and work with the existing state. Do not revert others' changes.
10. If blocked, write a short blocker note into the `Blocked` section with exact file / function / test evidence.

## Agent Quick Start

- If no narrower instruction is given, start with `Workstream 1` and pick the first unchecked item.
- Default first slice: move remaining delayed-reward aggregate / hazard / censoring / competing-risk owner logic out of `src/state/types.rs` into `src/belief_core/source_reliability.rs`.
- If the user explicitly brings new regime-trial input like layered BBN / zigzag / leg clustering / related-stock consistency, run the one-shot checklist below before resuming lower-level owner extraction.
- Do not start in `Workstream 3` unless `Workstream 1`, `Workstream 2`, and `Workstream 4` are no longer the real blocker.
- Before editing, read the exact owner files and function anchors below. Do not guess the entry points.

### One-Shot Checklist For This Trial Input

1. Freeze the role of zigzag.
   - zigzag may cut historical legs and stabilize past pivots
   - zigzag may not be the only live regime input
2. Normalize outside wording into existing repo concepts before changing code or docs.
   - `related-stock relative consistency` should be treated as the repo's existing SMT / correlation-consistency family, not copied as a brand-new concept by default
   - `tiny-leg` / `small zigzag` should be treated as retrospective segmentation candidates, not assumed runtime truth
3. If a layered evidence packet is attempted later, keep the first version small and interpretable rather than assuming the contributor's exact layering is already correct.
4. Make each layer emit an explicit probability or score contribution.
   - first version may be hardcoded
   - contributions must be recorded layer by layer
5. Support multi-period evaluation explicitly.
   - treat six periods as a valid first-pass scaffold
   - do not bury the period dimension in one blended scalar
6. Put tiny-leg clustering on the research side first.
   - tiny zigzag
   - 5 leg factors
   - `16` raw clusters
   - merge to `6` higher-level regimes
   - output is retrospective regime evidence, not sole runtime truth
7. Add a live now-cast regime branch alongside retrospective leg confirmation.
   - runtime belief must have a current-state input not dependent only on delayed pivot confirmation
8. Verify a negative rule:
   - no runtime path may determine regime from zigzag-confirmed pivots alone

### Related Code And Docs For This Trial Input

- `src/analyze_human_output.rs`
  - already exposes a layered human-facing decomposition close to this trial's structure
- `src/domain/regime/hybrid.rs`
  - current hybrid regime packet and PDA-cluster interaction
- `src/pda_sequence/`
  - existing PDA sequence analysis surfaces
- `src/hmm/`
  - current temporal regime inference primitives
- `src/belief_core/`
  - current Bayesian / structural belief core
- `support/docs/regime-aware`
  - existing regime-aware clustering notes
- `support/docs/experiments/oracle-regime-probe.md`
  - research-only guardrails for retrospective label discovery
- `support/docs/plans/nlp-inspired-pda-sequence-clustering-plan.md`
  - DTW / sequence-aware PDA clustering direction
- `support/docs/hybrid-regime-clustering-integration-note.md`
  - current hybrid cluster + HMM integration notes
- `support/docs/regime-aware`
  - existing repo-side GMM / HMM / HSMM clustering notes; use this as the normalization target before importing external wording

### Default First Slice: Open These First

- `src/belief_core/source_reliability.rs`
  - `structural_source_reliability_em_diagnostics`
  - `structural_source_reliability_em_fit_from_state`
  - `refresh_structural_source_reliability_em_state`
  - `structural_delayed_reward_replay_validation`
  - `structural_experience_prior_runtime_metrics`
- `src/state/types.rs`
  - `structural_source_reliability_em_diagnostics`
  - `structural_source_reliability_em_fit_from_state`
  - `refresh_structural_source_reliability_em_state`
  - `rebuild_structural_sequence_priors`
- `src/application/orchestration/structural_playbook.rs`
  - `build_structural_experience_prior_surface_artifact_with_prior_state`
  - call sites of `structural_experience_prior_runtime_metrics(...)`
  - call site of `structural_delayed_reward_replay_validation(...)`

Locate them with:

```bash
rg -n 'structural_source_reliability_em_diagnostics|structural_source_reliability_em_fit_from_state|refresh_structural_source_reliability_em_state|structural_delayed_reward_replay_validation|structural_experience_prior_runtime_metrics|rebuild_structural_sequence_priors' \
  src/belief_core/source_reliability.rs \
  src/state/types.rs \
  src/application/orchestration/structural_playbook.rs
```

Minimum verification for this default first slice:

```bash
cargo check
cargo test source_reliability_em_readiness_requires_multi_source_overlap
cargo test source_reliability_em_fit_learns_lower_reliability_for_conflicting_source
cargo test test_structural_source_reliability_em_holdout_prefers_chronological_split
```

## Already Landed, Do Not Redo

- `belief_core::{structural_state, source_reliability, regime_filter, changepoint_gate, ranking_label, beta_dirichlet_update}` already exist. Do not waste time recreating modules.
- The opt-in structural path-ranker runtime already exists for scored-row reuse, direct weighted-feature models, and declared scoring services.
- `workflow-status` and `policy-training-status` already expose low-token structural validation and ranker-runtime summaries.
- The provider-profile hot-plug lane is good enough for now. Do not reopen it unless fixing a concrete regression.
- Starter source-reliability holdout/replay and delayed-reward replay validation already exist. The remaining work is to strengthen them, not merely restate them.

## Do Now

### Workstream 1: Belief-Core Extraction

**Objective:** remove the remaining structural-learning ownership from oversized mixed-purpose files.

**Done when:** `src/state/types.rs`, `src/application/orchestration/structural_playbook.rs`, `src/application/orchestration/workflow_status.rs`, and `src/main.rs` are consumers or shells rather than dominant owners of structural math.

- [x] Move the remaining delayed-reward aggregate / hazard / censoring / competing-risk owner logic out of `src/state/types.rs` into `src/belief_core/source_reliability.rs`.
- [x] Move the remaining temporal / duration rebuild ownership out of `src/state/types.rs` into `src/belief_core/{regime_filter, changepoint_gate}.rs`.
- [x] Reduce `src/application/orchestration/structural_playbook.rs` to shared-bundle assembly only. Remove remaining duplicated experience-prior and validation math.
- [x] Reduce `src/application/orchestration/workflow_status.rs` to phase selection and rendering only. No structural math ownership should remain there.
- [x] Keep `src/main.rs` on thin dispatch only. Do not add new structural-learning math or new surface-specific structural glue there.

**Latest landed on this lane:**

- `7fadd58` moved source-reliability EM diagnostics / fit / persisted-refresh ownership into `src/belief_core/source_reliability.rs`, with `src/state/types.rs` reduced to thin public wrappers for that lane.
- delayed-reward aggregate / hazard / censoring / competing-risk formula ownership now also routes through `src/belief_core/source_reliability.rs`, with `src/state/types.rs` reduced further toward refresh-call consumers instead of formula owners.
- delayed-reward feedback aggregation now also routes through `src/belief_core/source_reliability.rs`, with `src/state/types.rs` reduced to thin callers for stats/source-summary updates; verified with `cargo check`, `cargo test source_reliability_em_readiness_requires_multi_source_overlap`, `cargo test --lib source_reliability_em_fit_learns_lower_reliability_for_conflicting_source`, `cargo test --lib test_structural_source_reliability_em_holdout_prefers_chronological_split`, and `cargo test --lib test_structural_feedback_records_snips_and_dr_policy_priors`.
- temporal / duration rebuild ownership now routes through `src/belief_core/changepoint_gate.rs` and `src/belief_core/regime_filter.rs`, with `src/state/types.rs::rebuild_structural_sequence_priors(...)` reduced to a thin shell plus EM refresh; verified with `cargo check`, `cargo test --lib test_structural_node_duration_priors_discount_older_streaks`, and `cargo test --lib test_structural_prior_seed_rebuilds_branch_transition_priors`.
- experience-prior helper math now routes through `src/belief_core/source_reliability.rs`, with `src/application/orchestration/structural_playbook.rs` reduced to entry assembly plus path-specific replay-data selection; verified with `cargo check`, `cargo test --lib source_reliability_em_readiness_requires_multi_source_overlap`, `cargo test --lib source_reliability_em_fit_learns_lower_reliability_for_conflicting_source`, `cargo test --lib experience_prior_surface_path_includes_delayed_reward_replay_validation`, and `cargo test --lib panel_derived_prior_uses_persisted_source_reliability_em_summary`.
- `src/application/orchestration/workflow_status.rs` is now structural phase selection plus human/agent rendering only: structural validation, temporal, experience-prior, path-ranking, and recommended-bundle data all come from delegated builders/surfaces; verified with `cargo check`, `cargo test --lib agent_and_human_workflow_status_views_expose_experience_prior_surface`, `cargo test --lib agent_and_human_workflow_status_views_expose_structural_temporal_summary`, and `cargo test --lib workflow_status_phase_structural_recommended_path_bundle_is_token_friendly`.
- `src/main.rs` no longer owns duration sizing / transition guardrail helpers or duplicated duration-surface parsing: those now route through `src/application/belief/structural_temporal_adjustment.rs` and `src/application/backtest/command_entry.rs`; verified with `cargo check`, `cargo test --bin ict-engine test_apply_regime_execution_guardrail_blocks_on_high_transition_hazard`, `cargo test --bin ict-engine test_apply_duration_sizing_adjustment_zeroes_size_for_tight_duration`, `cargo test --bin ict-engine test_duration_sizing_scale_is_market_family_aware`, and `cargo test --bin ict-engine test_build_duration_surface_from_artifacts_uses_snapshot_and_scale_summary`. Remaining `main.rs` ownership is the workflow snapshot / canonical ensemble overlay cluster, so this final Workstream 1 item stays open.
- workflow snapshot overlay, blocking-truth, diff, and disagreement helpers now route through `src/workflow_snapshot_runtime.rs`, with `src/main.rs` reduced further toward phase snapshot DTO constructors plus `gate_aware_recommended_next_command(...)`; verified with `cargo check`, `cargo test --bin ict-engine test_workflow_snapshot_contains_actionable_and_promotable_artifacts`, `cargo test --bin ict-engine test_workflow_snapshot_overlays_analyze_ensemble_vote_with_canonical_structural_posterior`, and `cargo test --bin ict-engine test_workflow_snapshot_detects_analyze_update_disagreement`.
- phase snapshot DTO constructors and gate-aware next-command selection now also route through `src/workflow_snapshot_runtime.rs`, leaving `src/main.rs` on command orchestration plus non-workflow runtime concerns rather than structural workflow ownership; verified with `cargo check`, `cargo test --bin ict-engine test_workflow_phase_snapshot_from_backtest_run_surfaces_objective_market_shrink`, `cargo test --bin ict-engine test_workflow_phase_snapshot_from_research_run_surfaces_canonical_structural_regime`, `cargo test --bin ict-engine test_workflow_phase_snapshot_from_update_run_prefers_consumed_canonical_structural_regime_posterior`, and `cargo test --bin ict-engine test_workflow_snapshot_contains_actionable_and_promotable_artifacts`.
- maintained node / branch transition refresh is now owned by `src/belief_core/regime_filter.rs`: `src/state/types.rs` only delegates to `rebuild_transition_posteriors_from_events(...)`, `src/application/belief/structural_temporal_adjustment.rs` only re-exports or consumes transition helpers, and maintained refresh stays in `rebuild_transition_posteriors_from_events(...)`, `refresh_node_transition_posteriors(...)`, and `refresh_branch_transition_posteriors(...)`; verified with `cargo test --lib test_structural_prior_seed_rebuilds_branch_transition_priors` and `cargo test --lib test_structural_transition_priors_discount_older_transitions`.

**Read first when working this lane:**

- `src/state/types.rs`: `rebuild_structural_sequence_priors(...)`
- `src/belief_core/source_reliability.rs`: EM / delayed-reward owner functions listed in `Agent Quick Start`
- `src/application/orchestration/structural_playbook.rs`: experience-prior surface assembly
- `src/application/orchestration/workflow_status.rs`: validation-summary and ranker-summary rendering only if a surface contract is touched

### Workstream 2: Transition And Changepoint Closure

**Objective:** replace heuristic transition and break blending with one maintained core.

**Done when:** transition refresh and break maintenance have one clear owner each, and snapshot-time reweighting is only a consumer of maintained state.

- [x] Make `src/belief_core/regime_filter.rs` the single owner of maintained node / branch transition refresh.
- [x] Make `src/belief_core/changepoint_gate.rs` the single owner of break / sequence-break maintenance.
- [x] Remove fixed weighted blends from consumer layers. Snapshot-time reweighting must consume maintained filter state instead of acting as the main engine.
- [x] Reduce `src/application/belief/structural_temporal_adjustment.rs` to compatibility-only usage of the maintained core.

Current evidence on this lane:

- transition refresh ownership is now concentrated in `src/belief_core/regime_filter.rs`; `src/state/types.rs` only delegates to `rebuild_transition_posteriors_from_events(...)`, and `src/application/belief/structural_temporal_adjustment.rs` only re-exports or consumes transition helpers. Verified with `cargo test --lib test_structural_prior_seed_rebuilds_branch_transition_priors` and `cargo test --lib test_structural_transition_priors_discount_older_transitions`.
- break and sequence-break maintenance is now concentrated in `src/belief_core/changepoint_gate.rs`; `src/state/types.rs` only delegates to `rebuild_node_duration_priors_from_events(...)`, while BOCPD break math and duration/sequence-break maintenance live in `structural_bocpd_break_probability(...)`, `structural_node_bocpd_sequence_break_probability(...)`, and `rebuild_discounted_node_duration_priors(...)`. Verified with `cargo test --lib test_structural_node_duration_priors_discount_older_streaks`, `cargo test --lib bocpd_break_probability_rises_with_surprise_and_negative_outcomes`, and `cargo test --lib sequence_break_probability_increases_with_sequence_change`.
- snapshot-time consumer reweighting now consumes maintained temporal state or leaves the base probability unchanged; it no longer derives blend weight from raw `duration_prior` / `transition_prior` fallbacks. Verified with `cargo check`, `cargo test --lib transition_adjusted_branch_posteriors_respects_transition_outcome_support`, `cargo test --lib transition_adjusted_branch_posteriors_prefers_persisted_temporal_state_over_transition_prior`, and `cargo test --lib blend_node_posterior_prefers_persisted_temporal_state_over_duration_prior`.
- `src/application/belief/structural_temporal_adjustment.rs` is now a compatibility surface over maintained transition helpers only; duration sizing and transition guardrail execution policies moved to `src/application/belief/execution_temporal_controls.rs`. Verified with `cargo check`, `cargo test --bin ict-engine test_apply_regime_execution_guardrail_blocks_on_high_transition_hazard`, and `cargo test --bin ict-engine test_apply_duration_sizing_adjustment_zeroes_size_for_tight_duration`.

**Read first when working this lane:**

- `src/state/types.rs`: `rebuild_structural_sequence_priors(...)`
- `src/belief_core/regime_filter.rs`
  - `refresh_node_transition_posteriors`
  - branch-temporal posterior refresh / recursive transition helpers
- `src/belief_core/changepoint_gate.rs`
  - `structural_bocpd_break_probability`
  - `structural_node_bocpd_sequence_break_probability`
  - duration-prior changepoint update block
- `src/application/belief/structural_temporal_adjustment.rs`: compatibility-shell call sites and tests

Locate them with:

```bash
rg -n 'rebuild_structural_sequence_priors|refresh_node_transition_posteriors|branch_temporal_posteriors|bocpd|sequence_break|break_probability' \
  src/state/types.rs \
  src/belief_core/regime_filter.rs \
  src/belief_core/changepoint_gate.rs \
  src/application/belief/structural_temporal_adjustment.rs
```

### Workstream 4: Validation Hardening

**Objective:** move from compact diagnostics to stronger evidence-bearing validation.

**Done when:** validation surfaces say not only `ready / not ready`, but also why, how much evidence exists, and what split or panel produced the status.

- [x] Strengthen source-reliability validation beyond compact holdout / replay summaries into larger-panel or more explicit out-of-sample evaluation.
- [x] Keep validation summaries low-token, but always expose panel size, coverage, split boundary, and failure reason.
- [x] Strengthen delayed-reward validation beyond current horizon-only diagnostics. If full event-time competing-risk is not landed in the slice, leave a clearer intermediate owner and explicit remaining gap here.
- [x] Keep the target-policy upgrade path explicit. Do not silently entrench the current `symbol:regime:direction` bucket-posterior model as the final state.
- [x] Add an explicit validation rule that retrospective zigzag / tiny-leg / cluster outputs are not sufficient by themselves for live regime truth.

Current evidence on this lane:

- `src/application/orchestration/workflow_status.rs` now surfaces low-token validation reasons alongside counts and split boundaries: `holdout_reason`, `replay_reason`, delayed-reward `status_reason`, `holdout_split_strategy`, `replay_split_strategy`, coverage fields, training/evaluation counts, and replay train/eval boundary text in the human line. Verified with `cargo check`, `cargo test --lib workflow_status_phase_structural_validation_summarizes_holdout_and_replay`, and `cargo test --lib agent_and_human_workflow_status_views_expose_experience_prior_surface`.
- source-reliability validation now also surfaces calibration/out-of-sample fields directly: `calibration_status`, `calibration_observation_count`, `calibration_source_count`, `calibration_brier_score`, and `calibration_log_loss`, so the surface is not limited to terse holdout/replay status flags.
- target-policy validation still names the current model as `symbol:regime:direction_bucket_posterior` while keeping `upgrade_path=learned_contextual_model_not_yet_landed` explicit, so the current bucket-posterior lane is not silently treated as final.
- validation surfaces now carry an explicit `live_regime_truth_rule` enforcing that retrospective zigzag, tiny-leg, or cluster outputs are not sufficient by themselves for live regime truth.
- delayed-reward validation now carries explicit intermediate-owner metadata: `validation_owner=horizon_replay_validation` and `remaining_gap=full_event_time_competing_risk_validation_not_yet_landed`, so the current horizon-based lane is clearly marked as an intermediate stage rather than the final event-time validation design.

### Workstream 5: Trial-Driven Layered Regime Intake

**Objective:** convert the contributor trial into repo-compatible constraints and a bounded research/implementation intake path, not a copy-through implementation.

**Done when:** the repo has a clear dual-track design where retrospective tiny-leg regime evidence and live current-state evidence both exist, and any layered BBN attempt is mapped onto existing repo concepts before promotion.

- [x] Normalize contributor wording into existing repo concepts before implementation.
  - `related-stock relative consistency` -> existing SMT / correlation-consistency lane
  - `tiny-leg zigzag regime` -> retrospective segmentation / clustering lane
- [x] Keep any first-pass layered evidence attempt simple and inspectable; do not assume the contributor's exact layering is already correct.
- [x] Model six-period evidence as an explicit dimension rather than collapsing it immediately.
- [x] Add a research-only tiny-leg regime discovery lane:
  - small zigzag
  - 5 leg factors
  - `16` raw clusters
  - merge to `6` higher-level regimes
- [x] Keep that tiny-leg regime discovery lane out of sole runtime truth until a live now-cast branch exists.
- [x] Add or refine a live current-state regime branch that is not delayed purely by pivot confirmation.
- [x] Expose layer-by-layer probability contribution to human and agent surfaces instead of only a final blended scalar.

Current evidence on this lane:

- `support/docs/hybrid-regime-clustering-integration-note.md` now normalizes trial wording into repo concepts: `related-stock relative consistency -> SMT / correlation-consistency` and `tiny-leg zigzag regime -> retrospective segmentation / clustering`.
- The same note now fixes the first-pass intake contract as small and inspectable, and it explicitly names six-period evaluation as a first-class dimension rather than a hidden blended scalar.
- `support/docs/experiments/oracle-regime-probe.md` now records the research-only guardrail for tiny-leg / zigzag intake and keeps retrospective outputs separate from live regime truth.
- `src/factor_lab/oracle_probe.rs` now provides a research-only tiny-leg/oracle probe scaffold with explicit `small_zigzag`, 5 leg features, `16 -> 6` cluster settings, six evaluation periods, and a `RequiresLiveNowcastBranch` promotion boundary. Verified with `cargo check`, `cargo test --lib tiny_leg_probe_defaults_stay_research_only_and_six_period`, and `cargo test --lib oracle_probe_report_carries_live_truth_rule_and_layer_contributions`.
- `src/application/orchestration/workflow_status.rs` now names the current live branch explicitly as `temporal_hmm_pre_bayes_nowcast` and marks `pivot_confirmation_dependency=not_required`, so the repo’s live regime path is no longer implied only by surrounding docs or validation rules. Verified with `cargo test --lib workflow_status_phase_structural_validation_summarizes_holdout_and_replay` and `cargo test --lib agent_and_human_workflow_status_views_expose_experience_prior_surface`.
- `src/factor_lab/oracle_probe.rs` now also exposes layer contributions through explicit human and agent surfaces (`render_oracle_regime_probe_human_lines(...)` and `build_oracle_regime_probe_agent_surface(...)`) instead of leaving them as an opaque final scalar only. Verified with `cargo test --lib oracle_probe_surfaces_render_layer_contributions_for_humans_and_agents`.

**Read first when working this lane:**

- `src/belief_core/source_reliability.rs`: holdout / replay / delayed-reward replay owner functions and tests
- `src/application/orchestration/workflow_status.rs`
  - `build_structural_validation_summary_value`
  - `build_path_ranker_summary_value`
- `src/application/orchestration/structural_playbook.rs`: experience-prior artifact assembly

Locate them with:

```bash
rg -n 'build_structural_validation_summary_value|build_path_ranker_summary_value|structural_validation_summary|path_ranker_summary|structural_delayed_reward_replay_validation|structural_source_reliability_em_diagnostics' \
  src/application/orchestration/workflow_status.rs \
  src/application/orchestration/structural_playbook.rs \
  src/belief_core/source_reliability.rs
```

Minimum verification for this lane:

```bash
cargo check
cargo test delayed_reward_replay_validation_scores_future_resolution_horizons
cargo test source_reliability_em_readiness_requires_multi_source_overlap
cargo test test_structural_source_reliability_em_holdout_prefers_chronological_split
```

## Later Only If Still Needed

### Workstream 3: Broader Runtime Closure

**Objective:** close remaining downstream runtime-consumer gaps only after Workstream 1, Workstream 2, and Workstream 4 stop being the main blocker.

- [x] Decide the remaining downstream consumer closure boundary for structural path-ranking runtime beyond the current direct-model and scoring-service contract.
- [x] Make artifact-backed behavior versus sample / placeholder behavior explicit at every remaining downstream consumer boundary.
- [x] Do not expand this lane before Workstream 1, Workstream 2, and Workstream 4 unless a concrete failing consumer proves it is the current blocker.

Current evidence on this lane:

- the current downstream runtime closure boundary is explicit: `registered_artifact`, direct-model, and scoring-service consumers are the supported lanes; no broader runtime expansion is required to keep zero-config defaults intact right now.
- runtime-versus-placeholder behavior is explicitly surfaced through `runtime_source`, match counts, `baseline_only` / registered-artifact status, and warning lists on the structural path-ranking target and recommended-path bundle surfaces.
- this lane was intentionally left closed until Workstream 1, Workstream 2, and Workstream 4 landed; no separate failing consumer proved it had to preempt those lanes.

### Deeper Model Work

- [x] Defer a deeper learned or contextual target-policy probability model until runtime evidence justifies replacing the current bucket posterior.
- [x] Defer a fuller elapsed-time competing-risk delayed-reward model until validation proves the compact horizon replay model is insufficient.

Current evidence on this lane:

- target-policy surfaces still advertise `upgrade_path=learned_contextual_model_not_yet_landed`, while current runtime/validation evidence does not yet show enough mature production rows to justify replacing the bucket-posterior baseline.
- delayed-reward validation now names `remaining_gap=full_event_time_competing_risk_validation_not_yet_landed`, but it does not yet prove that the compact horizon replay model is insufficient for the current closure slice.

## Execution Order

1. Finish Workstream 1 owner extraction.
2. Land Workstream 2 on top of that extracted core.
3. Deepen Workstream 4 validation.
4. Only then reopen broader Workstream 3 runtime closure if it still blocks real usage.
5. After those land, run fake-real operator / contributor shakedown and fix the resulting bugs before cleanup-only work.

## Verification Gate For Every Slice

- Required: `cargo check`
- Required: targeted tests for the touched owner files or surfaces
- Required: if a CLI or status contract changes, verify the real surface, not just a helper
- Required: update this same markdown with the landed slice and the remaining gap
- Not sufficient: static inspection only
- Not sufficient: formatting-only diffs
- Not sufficient: green helper tests if the real CLI or surface contract is still unverified

## Blocked

- Existing red baseline in `src/state/types.rs`: `cargo test --lib test_structural_prior_seed_rebuilds_node_duration_priors` still fails on this branch and on parent commit `35cac8a` at `trend.bocpd_recursive_run_length_mode_probability > 0.5`; this slice did not introduce that failure, but the lane is not fully green until that expectation or the underlying changepoint math is reconciled.
