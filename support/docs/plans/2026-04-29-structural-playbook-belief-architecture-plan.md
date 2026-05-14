# Structural Playbook + Belief Architecture Plan

Date: 2026-04-29
Status: proposed
Scope: evolve ict-engine from direction-first scoring into node/branch/scenario/path orchestration with BBN priors/posteriors and CatBoost path ranking

## Goal

Make the system answer the real consumer question:

1. What structural node is the market in now?
2. From this node, what future branches are plausible?
3. For each branch, what playbook / scenario applies?
4. For each scenario, what executable paths exist?
5. For each path, what are the trigger, stop, invalidation, and confirmation rules?
6. Which path is preferred right now, and why?

The target is not a one-shot long/short classifier. The target is a
structured decision stack where:

- `BBN` owns node/branch priors and posteriors
- `CatBoost` ranks candidate paths inside the allowed structural space
- live user feedback and factor-iteration outcomes both flow back into the same
  belief/prior enrichment loop

## Boundary Decision

### 1. BBN responsibility

`BBN` is the belief engine, not a display-only explanation layer.

It owns:

- current structural node belief
- branch transition priors/posteriors
- regime-conditioned path plausibility
- uncertainty / disagreement / invalidation pressure
- feedback-based posterior updates
- prior-init from offline research / strategy validation

### 2. CatBoost responsibility

`CatBoost` is a conditional path ranker, not a freeform market oracle.

It must not invent:

- new structural nodes
- hidden branches outside the node graph
- playbooks not declared upstream
- stops / triggers / invalidation rules

It may rank:

- candidate path quality
- path execution feasibility
- trigger cleanliness
- expected holding/decay behavior
- branch-specific risk/reward

### 3. Public consumer protocol

The consumer-facing protocol must stay generic and explicit:

- node
- branch
- scenario
- path
- trigger
- stop
- invalidation
- confirmation
- belief
- score
- feedback

No maintainer ontology is required from the consumer.

## Target Artifact Stack

### A. Structural node artifact

Represents the current best-known structural state.

Required fields:

- `node_id`
- `node_family`
- `node_label`
- `market_context`
- `timeframe_scope`
- `supporting_evidence`
- `invalidating_evidence`
- `belief_prior`
- `belief_posterior`
- `posterior_confidence`
- `origin_artifacts`

Examples of node families:

- `accumulation`
- `sweep_reversal`
- `continuation_retest`
- `imbalance_resolution`
- `distribution_transition`

### B. Branch set artifact

Enumerates plausible next transitions from the current node.

Required fields:

- `from_node_id`
- `branches[]`

Each branch needs:

- `branch_id`
- `target_node_id`
- `branch_label`
- `prior_probability`
- `posterior_probability`
- `activation_conditions`
- `failure_conditions`
- `supporting_evidence`

### C. Scenario playbook artifact

Maps each branch to one or more executable market narratives.

Required fields:

- `scenario_id`
- `branch_id`
- `scenario_label`
- `narrative`
- `required_confirmations`
- `hard_invalidations`
- `timing_constraints`
- `path_ids[]`

### D. Path plan artifact

This is the consumer/action surface.

Required fields per path:

- `path_id`
- `scenario_id`
- `path_label`
- `direction`
- `entry_style`
- `trigger_conditions`
- `confirmation_conditions`
- `stop_definition`
- `target_definition`
- `invalidation_conditions`
- `expected_failure_mode`
- `max_time_in_trade`
- `path_prior`
- `path_posterior`
- `catboost_score`
- `bbn_support_score`
- `composite_preference_score`

## Learning Loop

### Channel 1: Offline prior enrichment

Source:

- factor iteration
- Auto-Quant strategy validation
- future scenario/path backtest libraries

Effect:

- enrich node priors
- enrich branch transition priors
- enrich path baseline win/loss likelihoods

This is prior init / pseudo-count style updating.

### Channel 2: Live posterior update

Source:

- real user consultations
- agent recommendation output
- user-reported execution path
- realized win/loss / scratch / invalidation

Effect:

- update node/branch/path posteriors
- calibrate disagreement penalties
- strengthen or weaken specific path definitions

This must land in the same canonical feedback chain already used for BBN
feedback, not a parallel scoring sidecar.

### Required feedback payload

Each live consultation outcome must map back to:

- selected `node_id`
- selected `branch_id`
- selected `scenario_id`
- selected `path_id`
- recommendation timestamp
- whether the user followed the path
- realized outcome
- realized pnl / categorical result
- stop hit / target hit / invalidated / abandoned

Without this mapping, the system can only learn “direction right/wrong”, which
is weaker than the desired path-level learning.

## Personal Data Contract As Optional Input

The public default must remain zero-config and provider-neutral.

Your personal data contract is allowed only as an opt-in, hot-pluggable input:

- Tomac cleaned multi-timeframe NQ history
- QQQ spot context
- VIX overlay
- QQQ options Greeks / IV / OI
- OpenBB zero-config first
- optional OpenAlice / NoFX reuse
- optional IBKR reuse
- optional Kraken authenticated track

This personal profile must remain:

- versioned
- opt-in
- replaceable
- never auto-loaded

It may influence:

- which branches/scenarios/paths are available
- which evidence surfaces are considered complete
- which provider/runtime prompts are shown

It must not influence:

- the public default CLI semantics
- consumer-required ontology
- hidden fallback behavior

## Current Repo Pieces That Already Fit

- `BBN` prior/posterior update path
- feedback persistence and merge
- provider-profile opt-in loading
- `agent-material-*` generic direction
- policy engine / CatBoost-compatible artifact infrastructure
- artifact ledger / lineage / workflow-status surfaces

## Missing Pieces

1. explicit node artifact schema
2. explicit branch-set artifact schema
3. explicit scenario playbook schema
4. explicit path-plan schema
5. feedback payload that references `node_id/branch_id/scenario_id/path_id`
6. CatBoost path-ranking target contract
7. BBN node/branch/path mapping layer

## Recommended Phase Order

### Phase 1: Structural protocol

- add typed `node/branch/scenario/path` artifact schemas
- keep them generic and consumer-safe
- no CatBoost retraining yet

### Phase 2: BBN mapping

- map current pre-bayes / belief / execution outputs into node + branch priors
- persist explicit node/branch artifacts

### Phase 3: Path planning

- produce scenario and path artifacts from current structural state
- attach trigger/stop/invalidation/confirmation fields

### Phase 4: CatBoost path ranking

- redefine policy-engine features around candidate paths
- score paths, not freeform actions

### Phase 5: Feedback loop completion

- persist live consultation recommendation references
- map user-reported outcomes back to path ids
- update BBN posterior history and path calibration

## Acceptance Criteria

- the system can emit a current structural node
- the system can emit explicit future branches from that node
- each branch has one or more scenarios
- each scenario has one or more executable paths
- every path has trigger / stop / invalidation / confirmation
- `BBN` owns priors/posteriors for node/branch/path state
- `CatBoost` ranks paths inside that declared space
- real user feedback and factor iteration both enrich the same canonical belief chain
