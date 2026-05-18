# CatBoost Path Ranking Target Implementation Plan

> **For Hermes:** Use subagent-driven-dev skill to implement this plan task-by-task.

**Goal:** Add a P6 target surface for path ranking that keeps CatBoost inside the declared structural candidate set and makes delayed rewards, propensity, and calibration fields explicit.

**Architecture:** The structural layer remains the authority for `node / branch / scenario / path` candidates. The CatBoost layer may score and calibrate those existing candidates, but it must not create hidden nodes, hidden branches, or extra paths. The first code slice should add a serializable target artifact and compact workflow surface with optional calibration fields; model training remains downstream until the logged target rows are stable.

**Tech Stack:** Rust, serde, existing `src/application/orchestration/structural_playbook.rs` artifacts, `workflow-status` JSON surfaces, cargo unit tests.

---

## Current Status

Implemented:
- serializable target artifact and rows
- `structural-path-ranking-target` workflow surface
- candidate-set scoped CSV / JSONL / summary export under `policy_training/`
- explicit CLI export path for structural path-ranking target rows, so users can refresh the export on demand from persisted state instead of relying on update-side effects
- summary-level trainer manifest for external ranker services; Rust core should not require CatBoost
- `policy-training-status` readiness reporting for export and calibration state
- `policy-training-status` trainer manifest readiness/counts for the external ranker handoff contract
- optional `policy_training/structural_path_ranking_trainer_artifact.json` status boundary for trained external ranker artifacts; the status surface reports compact readiness/count fields and URI presence without dumping the URI
- explicit opt-in CLI registration path for external trainer artifacts, so users can wire in a personal artifact URI without hand-authoring the JSON boundary file
- explicit CLI clearing path for external trainer artifacts, so users can stop using a personal artifact without manual JSON edits
- explicit CLI score-apply path for external ranker outputs, so users can merge a personal scored CSV/JSONL file back into the latest export plus the accumulated history dataset without manual JSONL edits
- empirical Beta-smoothed calibration from raw-scored mature observations
- compact calibration-quality evaluation fields in `policy-training-status`
- compact raw-scored mature-row sufficiency / shortfall fields in `policy-training-status`, separated from propensity-weighted production-validation readiness
- compact `policy-training-status` summary lines that carry the path-ranking shortfalls instead of hiding them only in nested fields
- cumulative upserted history export in `policy_training/structural_path_ranking_target_history.jsonl`, so production-validation counts can accumulate across exports instead of resetting to the latest candidate set snapshot
- cumulative history CSV export in `policy_training/structural_path_ranking_target_history.csv`, so external trainers can consume the accumulated dataset without translating JSONL first
- external artifact registration now defaults trained/calibration row counts from accumulated history when it exists, instead of only the latest candidate snapshot
- `policy-training-status` history-side score/calibration/propensity/training-weight counters now derive from the accumulated dataset when it exists, so readiness counts stay aligned with the same history-backed basis
- externally applied raw scores now persist through later target re-exports when the candidate-set/path key still matches, instead of being dropped on the next export pass
- `policy-training-status` now distinguishes “pending update templates exist but no structural feedback has been applied yet”, “feedback rows exist but none carry structural refs”, and cases where there is simply no update/feedback history at all

Still not implemented:
- production calibration validation after enough exported raw-scored rows exist
- model training or propensity-aware evaluation loop; a real external trainer artifact/service is still user-supplied and opt-in

## Contract

The target surface is candidate-set scoped. Every row must be traceable to an existing structural path candidate and must carry the same `candidate_set_id` used by recommended path bundles and feedback templates.

Required row fields:

- `raw_path_score`: optional model/raw ranker score. Do not alias the existing structural composite score to this field unless a ranker actually emitted it.
- `calibrated_path_prob`: optional calibrated acceptance probability derived from a calibration layer.
- `path_prob_lower_bound`: optional conformal or other lower-bound probability used by execution gates.
- `pending_reward_state`: explicit delayed-feedback state.
- `calibrated_label`: optional mature reward label; absent for censored/unobserved rows.
- `propensity_estimate`: optional observation/execution propensity estimate for off-policy evaluation.
- `ips_weight` / `training_weight`: optional clipped inverse-propensity and mature-row sample weights for downstream rankers.
- `regime_calibration_bucket`: compact bucket key for calibration, initially `symbol:active_regime` or `symbol:unknown`.

Supporting fields should keep the row auditable without expanding public ontology:

- `candidate_set_id`
- `candidate_set_size`
- `rank`
- `path_id`
- `scenario_id`
- `path_label`
- `direction`
- `behavior_policy_probability`
- `execution_propensity`
- `experience_prior`
- `current_posterior`
- `structural_baseline_score`

Delayed-feedback states:

- `unobserved`: no feedback row exists for this path.
- `pending`: latest feedback exists but outcome is `pending`, `delayed`, `unresolved`, or `awaiting`.
- `matured_success`: latest followed feedback resolved as success.
- `matured_failure`: latest followed feedback resolved as loss, abandoned, or breakeven failure semantics.
- `matured_invalidated`: latest followed feedback resolved as invalidated.
- `not_followed`: latest feedback indicates the candidate was not followed.

Propensity rule for the first code slice:

```text
propensity_estimate = behavior_policy_probability * execution_propensity
```

Use the existing smoothed execution propensity from structural prior/path history. If execution propensity is unavailable, leave `propensity_estimate` absent instead of defaulting to a personal or environment-specific assumption.

Calibration rule for the first code slice:

```text
raw_path_score -> calibrated_path_prob -> path_prob_lower_bound
```

Leave all three fields absent when no ranker/calibrator emitted them. The structural baseline score remains separately visible as `structural_baseline_score`.

## Task 1: Add Target Artifact Structs

**Objective:** Add backward-compatible serde structs for the P6 target surface.

**Files:**
- Modify: `src/application/orchestration/structural_playbook.rs`

**Step 1: Write failing tests**

Add tests near existing structural playbook artifact tests:

```rust
#[test]
fn structural_path_ranking_target_rows_keep_ranker_fields_optional() {
    let row = StructuralPathRankingTargetRow::default();
    assert!(row.raw_path_score.is_none());
    assert!(row.calibrated_path_prob.is_none());
    assert!(row.path_prob_lower_bound.is_none());
    assert!(row.propensity_estimate.is_none());
}
```

**Step 2: Run test to verify failure**

Run:

```bash
cargo test --lib structural_path_ranking_target_rows_keep_ranker_fields_optional
```

Expected: FAIL because `StructuralPathRankingTargetRow` does not exist.

**Step 3: Implement structs**

Add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralPathRankingTargetArtifact {
    pub protocol_version: String,
    pub symbol: String,
    pub candidate_set_id: String,
    pub candidate_set_size: usize,
    pub generated_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<StructuralPathRankingTargetRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralPathRankingTargetRow {
    pub rank: usize,
    pub candidate_set_id: String,
    pub candidate_set_size: usize,
    pub path_id: String,
    pub scenario_id: String,
    pub path_label: String,
    pub direction: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_path_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibrated_path_prob: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_prob_lower_bound: Option<f64>,
    pub pending_reward_state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub propensity_estimate: Option<f64>,
    pub regime_calibration_bucket: String,
    pub behavior_policy_probability: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_propensity: Option<f64>,
    pub experience_prior: f64,
    pub current_posterior: f64,
    pub structural_baseline_score: f64,
}
```

**Step 4: Run test to verify pass**

Run:

```bash
cargo test --lib structural_path_ranking_target_rows_keep_ranker_fields_optional
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src/application/orchestration/structural_playbook.rs
git commit -m "feat: add structural path ranking target artifact"
```

## Task 2: Carry Execution Propensity Onto Path Candidates

**Objective:** Make path-level execution propensity available before target rows are built.

**Files:**
- Modify: `src/application/orchestration/structural_playbook.rs`

**Step 1: Write failing test**

Add a test proving a path with prior/path history exposes `execution_propensity`.

**Step 2: Implement field**

Add to `StructuralPathArtifact`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub execution_propensity: Option<f64>,
```

Populate it from `prior_stats` using the existing structural prior execution-propensity helper. If prior stats are absent, leave it absent.

**Step 3: Preserve public compactness**

Do not add user-specific identifiers, account data, provider defaults, or environment-derived assumptions.

**Step 4: Verify**

Run:

```bash
cargo test --lib structural_path
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src/application/orchestration/structural_playbook.rs
git commit -m "feat: surface structural path execution propensity"
```

## Task 3: Build Target Rows From Declared Candidates Only

**Objective:** Add builder functions that reuse the existing structural candidate set and never synthesize hidden candidates.

**Files:**
- Modify: `src/application/orchestration/structural_playbook.rs`

**Step 1: Write failing test**

Test that the target artifact has the same `candidate_set_id`, candidate count, and path ids as `build_structural_top_path_candidates_artifact_with_prior_state`.

**Step 2: Implement builder**

Add:

```rust
pub fn build_structural_path_ranking_target_artifact_with_prior_state(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
) -> StructuralPathRankingTargetArtifact
```

Implementation rules:

- call `structural_ranked_paths_with_prior_state(...)`
- truncate to the same candidate-set boundary used by top-path candidates unless a future caller explicitly passes a wider declared set
- compute `candidate_set_id` with `structural_candidate_set_id`
- compute `behavior_policy_probability` with `structural_candidate_policy_probability`
- set `raw_path_score` from `path.catboost_score`
- leave calibration fields absent until a calibrator exists
- set `structural_baseline_score` from `path.composite_preference_score`

**Step 3: Verify**

Run:

```bash
cargo test --lib structural_path_ranking_target
```

Expected: PASS.

**Step 4: Commit**

```bash
git add src/application/orchestration/structural_playbook.rs
git commit -m "feat: build structural path ranking target rows"
```

## Task 4: Add Pending Reward State Helper

**Objective:** Separate pending/censored rewards from realized feedback.

**Files:**
- Modify: `src/application/orchestration/structural_playbook.rs`

**Step 1: Write failing tests**

Add tests for:

- no feedback -> `unobserved`
- `pending` / `delayed` / `unresolved` / `awaiting` -> `pending`
- followed `win` -> `matured_success`
- followed `loss` / `abandoned` / breakeven failure semantics -> `matured_failure`
- followed `invalidated` -> `matured_invalidated`
- not-followed row -> `not_followed`

**Step 2: Implement helper**

Add a private helper that scans latest structural feedback for the same `path_id` and returns the state string.

**Step 3: Verify**

Run:

```bash
cargo test --lib structural_path_ranking_target_pending_reward_state
```

Expected: PASS.

**Step 4: Commit**

```bash
git add src/application/orchestration/structural_playbook.rs
git commit -m "feat: classify delayed path ranking rewards"
```

## Task 5: Add Propensity Estimate Helper

**Objective:** Add propensity-aware target rows without treating unobserved candidates as negatives.

**Files:**
- Modify: `src/application/orchestration/structural_playbook.rs`

**Step 1: Write failing test**

Test:

```rust
assert_eq!(
    structural_path_ranking_propensity_estimate(Some(0.6), 0.25),
    Some(0.15)
);
assert_eq!(structural_path_ranking_propensity_estimate(None, 0.25), None);
```

**Step 2: Implement helper**

Clamp both inputs to `[0, 1]` and return `None` when execution propensity is absent.

**Step 3: Verify**

Run:

```bash
cargo test --lib structural_path_ranking_propensity
```

Expected: PASS.

**Step 4: Commit**

```bash
git add src/application/orchestration/structural_playbook.rs
git commit -m "feat: add path ranking propensity estimates"
```

## Task 6: Surface Target Rows In Workflow Status

**Objective:** Expose a token-friendly JSON surface for agents and keep human output compact.

**Files:**
- Modify: `src/application/orchestration/workflow_status.rs`
- Modify: `src/application/orchestration/structural_playbook.rs` if exports are needed

**Step 1: Write failing test**

Add a workflow-status test proving:

- `path_ranking_target.candidate_set_id` equals the recommended path bundle candidate set
- rows contain `pending_reward_state`
- absent calibration fields serialize as absent or null according to existing workflow JSON style

**Step 2: Implement JSON surface**

Add a compact object under structural workflow output:

```json
{
  "candidate_set_id": "...",
  "candidate_set_size": 3,
  "rows": [
    {
      "rank": 1,
      "path_id": "...",
      "pending_reward_state": "unobserved",
      "propensity_estimate": 0.15,
      "regime_calibration_bucket": "NQ:trend"
    }
  ]
}
```

Do not duplicate long trigger/confirmation/stop text inside this target surface.

**Step 3: Verify**

Run:

```bash
cargo test --lib workflow_status_structural_path_ranking_target
```

Expected: PASS.

**Step 4: Commit**

```bash
git add src/application/orchestration/workflow_status.rs src/application/orchestration/structural_playbook.rs
git commit -m "feat: surface structural path ranking targets"
```

## Task 7: Update Docs And Guardrails

**Objective:** Mark P6 target surface implementation without overstating trained CatBoost runtime support.

**Files:**
- Modify: `support/docs/plans/2026-04-30-structural-belief-execution-plan.md`
- Modify: `support/docs/structural-belief-learning-repo-map.md`

**Step 1: Update status**

Mark only the target-surface and explicit-field items done. Keep CatBoost model training, fitted calibration maps, and production execution gating as not done until real implementation exists.

**Step 2: Verify docs**

Run:

```bash
git diff --check
```

Expected: PASS.

**Step 3: Commit**

```bash
git add support/docs/plans/2026-04-30-structural-belief-execution-plan.md support/docs/structural-belief-learning-repo-map.md
git commit -m "docs: update catboost path target status"
```

## Full Verification

Run after all tasks:

```bash
cargo test --lib structural_path_ranking_target
cargo test --lib workflow_status_structural_path_ranking_target
cargo test --lib structural_path
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
git diff --check
git status --short
```

Expected:

- all tests pass
- no clippy warnings
- no whitespace errors
- clean worktree after commits

## Non-Goals

- no trained CatBoost runtime
- no external provider or account defaults
- no new required CLI flags
- no personal/user-specific data loading
- no hidden candidate expansion outside the structural candidate set
- no claiming probability calibration is live until a real calibrator writes `calibrated_path_prob` and `path_prob_lower_bound`
