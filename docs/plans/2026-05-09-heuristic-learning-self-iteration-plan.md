# Heuristic Learning Self-Iteration Plan for ICT Engine

> For Hermes: Use `subagent-driven-dev` only if implementing this plan. This document is the design and execution contract.

**Goal:** turn `ict-engine` from a one-shot factor/backtest pipeline into a Heuristic System: every factor, regime classifier, BBN evidence edge, CatBoost/path-ranker row, and execution-tree branch must absorb feedback, preserve regressions, and compress failed local patches into reusable structure.

**Architecture:** apply the lesson from "Learning Beyond Gradients": do not expect the model to gain trading skill by weight updates. Give the model an explicit software loop: probes, state detectors, trials, summaries, replays, golden traces, failure tags, mutation specs, verification gates, and retrospectives. The coding agent becomes System 2; `ict-engine` state artifacts become memory; factors/regime/BBN/path-ranker/execution-tree rules become System 1.

**Tech Stack:** Rust `ict-engine`, `src/application/factor_lifecycle/`, `src/application/orchestration/execution_tree.rs`, `src/bbn/evidence.rs`, `src/factors/regime_conditional.rs`, `scripts/research/*.py`, `scripts/auto_quant_external/*.py`, `/tmp/...` state dirs, `docs/factor-catalog.md`, `AGENTS.md`, `factor-research`, `factor-autoresearch`, `pre-bayes-status`, `auto-quant-prior-init`, `export-structural-path-ranking-target`, `policy-training-status`, `workflow-status`, `execution_tree_trace.json`.

---

## Core Insight

The article's useful insight is not "use more heuristics". It is:

```text
feedback / failure / replay
-> agent reads explicit context
-> edits code / config / memory
-> reruns
-> writes trials + summary + regression
-> compresses local patches
-> continues
```

For `ict-engine`, the equivalent loop is:

```text
market data / candidate factor / regime truth / trade outcome / execution trace
-> agent diagnoses weakest layer
-> mutates factor, regime detector, evidence mapping, ranker feature, or tree gate
-> reruns same state-dir pipeline
-> writes candidate pack + attempt row + verdict + replay artifact
-> promotes only if OOS + regime + path-ranking + execution-tree all improve
-> folds lessons into catalog/tests/templates
```

The model cannot "remember what high Sharpe needs" unless the project writes that memory into artifacts the next run can read. Current repo already has most parts: factor lifecycle attempts, candidate packs, regime bundles, BBN structure search, structural feedback, path-ranker trainer, execution-tree traces. Missing piece is a strict HL contract tying them into one self-iteration loop.

---

## Desired Capability

The model should learn, project-locally, to answer and improve these questions:

1. **High Sharpe factors**
   - What payoff shape is being targeted: trend convexity, reversion snapback, carry/VRP, liquidity sweep, session imbalance, cross-market lag, dealer pressure?
   - Is edge from prediction, execution quality, regime selection, or portfolio complementarity?
   - Does the factor survive density, OOS, market/timeframe breadth, and low-correlation contribution?

2. **95% confidence regime factors**
   - Regime factor is a classifier, not an entry system.
   - 95% confidence must mean calibrated posterior / conformal coverage / bootstrap lower bound, not just `confidence=0.95` in one run.
   - Evidence must include long-span labels, independent validation source, transition detection, persistence, flip-rate control, and resonance alignment.

3. **Execution tree and BBN evidence**
   - Nodes should be causal-ish and decision-useful, not metric dumps.
   - Evidence should reduce uncertainty or detect contradiction.
   - Execution-tree branch must explain what to do: fill, wait, block, guardrail; and why the payoff ratio is attractive now.

4. **High win-rate + high R/R opportunities**
   - Do not optimize win-rate alone.
   - Rank setups by expected value, downside truncation, liquidity window, stop distance realism, regime persistence, and path-ranker support.
   - Promote only when the full chain changes practical recommendation, not just backtest metric.

---

## Design Principles Imported from Heuristic Learning

### 1. A factor is not a formula; it is a maintained experiment system

Each promoted factor must own:

- `factor_expression.json`
- `candidate_spec.json`
- `trials.jsonl`
- `summary.csv` or `summary.json`
- `failure_tags`
- `regime_targets`
- `belief_targets`
- `path_ranking_targets`
- `execution_tree_targets`
- OOS replay commands
- golden regression cases

### 2. Failures are assets

Every failed candidate must be tagged, not discarded silently.

Minimum failure tags:

```text
under_trades
thin_density
regime_confused
transition_late
high_flip_rate
evidence_contradiction
bbn_no_uncertainty_reduction
path_ranker_no_mature_rows
execution_tree_no_change
high_sharpe_duplicate_source
oos_decay
payoff_tail_risk
session_mismatch
mtf_contradiction
```

### 3. Regression beats memory

If a rule works, encode it as:

- test
- replay state
- golden trace
- candidate-pack invariant
- mutation-template hint
- factor-catalog note

Do not rely on the next model remembering the chat.

### 4. Compression is mandatory

After every 10-20 attempts, run a compression pass:

- merge duplicate factors
- remove dead parameter branches
- lift repeated conditions into shared descriptors
- convert one-off patches into mutation templates
- update `docs/factor-catalog.md`
- update `AGENTS.md` only if traceability changes

Without compression, the project becomes an unmaintainable rule pile.

---

## Target Artifact Layout

Use `/tmp/...` during runs, then commit only summaries/specs/docs/tests worth keeping.

```text
/tmp/ict-hl/<symbol>/<session_id>/
  input/
    data_manifest.json
    provider_budget.json
    regime_truth_sources.json
  candidates/
    <candidate_id>/
      factor_expression.json
      candidate_spec.json
      trials.jsonl
      summary.json
      failure_tags.json
      chain_verdict.json
      replay_commands.md
      golden_trace.json
  regime/
    regime_benchmark.json
    calibration_report.json
    transition_report.json
  bbn/
    evidence_delta.json
    posterior_trace.json
    structure_candidate.json
  path_ranking/
    target.csv
    scores.csv
    trainer_artifact.json
    maturity_report.json
  execution_tree/
    workflow_status.txt
    execution_tree_trace.json
    before_after_diff.json
  retrospective.md
```

Committed docs should live under:

```text
docs/plans/
docs/research/
docs/factor-catalog.md
scripts/research/
tests/
```

---

## Acceptance Gates

### Gate A: High Sharpe factor gate

A trading factor cannot be called promising unless it passes:

```text
trade_count >= 80 on at least one liquid intraday lane
or 30-79 with explicit rewrite plan and no promotion
OOS Sharpe lower confidence bound > 0
max_drawdown and tail loss bounded vs baseline
profit factor improved vs baseline
average R/R improved or stop distance reduced
not a duplicate: pairwise return corr <= 0.65 vs accepted factor in same regime
incremental portfolio Sharpe > standalone replacement baseline
```

Metrics to persist:

```json
{
  "candidate_id": "...",
  "lane": "NQ/15m",
  "trade_count": 0,
  "sharpe_is": 0.0,
  "sharpe_oos": 0.0,
  "sharpe_oos_lcb": 0.0,
  "profit_factor": 0.0,
  "avg_rr": 0.0,
  "max_drawdown": 0.0,
  "tail_loss_p95": 0.0,
  "correlation_to_accepted": 0.0,
  "incremental_portfolio_sharpe": 0.0,
  "verdict": "reject|probe|promote"
}
```

### Gate B: 95% regime confidence gate

A regime factor reaches "95% confidence" only if one of these is true:

1. calibrated probability has empirical coverage >= 95% on validation slices;
2. bootstrap lower bound of macro-F1 / covered precision exceeds configured floor;
3. conformal prediction set covers >= 95% while keeping ambiguity rate acceptable.

Minimum regime metrics:

```text
macro_f1
non_unknown_accuracy
covered_precision
coverage
separation_eta2
transition_f1
flip_rate
mean_segment_bars
calibration_ece
bootstrap_lcb
conformal_coverage
ambiguity_rate
```

Promotion floors:

```text
calibration_ece <= 0.05
conformal_coverage >= 0.95, if conformal mode used
bootstrap_lcb >= 0.70 for macro_f1 or covered_precision, early floor
flip_rate below lane-specific maximum
mean_segment_bars above minimum persistence floor
validated on independent label source, not only MECE self-teacher
```

### Gate C: BBN evidence gate

A BBN node/evidence edge is useful only if it passes at least one:

```text
posterior_entropy_reduction > threshold
contradiction detection improves bad-trade avoidance
calibration improves after CPT update
trade_outcome log-loss improves OOS
execution-tree branch accuracy improves
```

BBN nodes should stay few and typed:

```text
market_regime
liquidity_context
factor_alignment
factor_uncertainty
multi_timeframe_resonance
crowding_pressure
dealer_pressure
session_quality
entry_quality
trade_outcome
```

Avoid node explosion. New node requires:

- observable data source
- discretization rule
- missing-data behavior
- CPT update path
- OOS value proof

### Gate D: CatBoost/path-ranker gate

A path-ranker is not mature just because `trainer_artifact=ready`.

Promotion floors:

```text
mature_rows >= 30 minimum
raw_scored_mature >= 30
calibration fitted or explicitly not required for current mode
registered model artifact used by runtime
execution_tree_trace includes ranker contribution
workflow recommendation changes for at least one replayable setup
```

### Gate E: Execution-tree gate

Execution-tree improvement requires before/after trace proof:

```text
branch changed for the right reason
execution_score changed in expected direction
gate_status reflects execution risk
split_reason_lineage includes factor/regime/BBN/ranker source
recommended_next_command or human next-action text improves
```

---

## Implementation Plan

### Task 1: Add Heuristic System run contract doc

**Objective:** make future agents run factor iteration as HL, not as isolated backtest search.

**Files:**
- Create: `docs/research/heuristic-learning-runtime-contract.md`
- Modify: `docs/factor-catalog.md`

**Steps:**
1. Write contract with the loop:
   `probe -> candidate -> run -> record -> diagnose -> mutate -> replay -> regress -> compress`.
2. Add factor-level artifact checklist.
3. Add failure tag taxonomy.
4. Add promotion gates A-E from this plan.
5. Add a link from `docs/factor-catalog.md`.

**Verify:**
```bash
grep -n "Heuristic System" docs/research/heuristic-learning-runtime-contract.md
grep -n "heuristic-learning-runtime-contract" docs/factor-catalog.md
```

### Task 2: Add candidate attempt schema

**Objective:** every run emits enough structured evidence for the next model to learn from.

**Files:**
- Create: `scripts/research/heuristic_attempt_schema.py`
- Test: `scripts/research/tests/test_heuristic_attempt_schema.py`

**Schema fields:**
```json
{
  "schema_version": "ict-hl-attempt/v1",
  "candidate_id": "string",
  "symbol": "string",
  "timeframe": "string",
  "family": "A|B|C|D|E|F|G|H",
  "hypothesis": "string",
  "target_layer": "factor|regime|bbn|path_ranking|execution_tree",
  "metrics": {},
  "failure_tags": [],
  "promotion_gate": "reject|probe|promote",
  "replay_commands": [],
  "artifacts": {},
  "next_mutation_hint": {}
}
```

**Verify:**
```bash
python3 -m pytest scripts/research/tests/test_heuristic_attempt_schema.py -q
```

### Task 3: Add chain verdict builder

**Objective:** produce one explicit verdict per candidate: where it stopped and why.

**Files:**
- Create: `scripts/research/heuristic_chain_verdict.py`
- Test: `scripts/research/tests/test_heuristic_chain_verdict.py`

**Inputs:**
- factor summary
- pre-bayes status text/json
- BBN posterior trace
- path-ranking maturity report
- execution-tree trace

**Output:**
```json
{
  "candidate_id": "...",
  "stopped_at": "factor|pre_bayes|bbn|path_ranking|execution_tree|closed_loop_changed",
  "blocking_metric": "mature_rows=0",
  "failure_tags": ["path_ranker_no_mature_rows"],
  "next_action": "collect_structural_feedback_rows",
  "evidence_paths": []
}
```

**Verify:**
```bash
python3 -m pytest scripts/research/tests/test_heuristic_chain_verdict.py -q
```

### Task 4: Add regime confidence calibration report

**Objective:** make "95% confidence" mean calibrated evidence, not raw confidence rhetoric.

**Files:**
- Create: `scripts/research/regime_confidence_report.py`
- Test: `scripts/research/tests/test_regime_confidence_report.py`

**Metrics:**
- ECE
- bootstrap lower bound
- conformal coverage
- ambiguity rate
- transition F1
- flip rate
- mean segment bars

**Verify:**
```bash
python3 -m pytest scripts/research/tests/test_regime_confidence_report.py -q
```

### Task 5: Add factor payoff-shape report

**Objective:** teach the loop what high Sharpe needs by decomposing return source and failure mode.

**Files:**
- Create: `scripts/research/factor_payoff_shape_report.py`
- Test: `scripts/research/tests/test_factor_payoff_shape_report.py`

**Report fields:**
```text
return_source
payoff_shape
trade_density
R/R distribution
tail risk
session concentration
regime concentration
correlation_to_existing
incremental_portfolio_sharpe
```

**Verify:**
```bash
python3 -m pytest scripts/research/tests/test_factor_payoff_shape_report.py -q
```

### Task 6: Extend BBN structure candidate review

**Objective:** prevent evidence-node sprawl and require uncertainty-reduction proof.

**Files:**
- Modify: `scripts/research/bbn_structure_search.py`
- Test: `scripts/research/tests/test_bbn_structure_search.py`

**Add checks:**
- max parent count
- allowed vocabulary
- posterior entropy delta
- OOS log-loss delta if labels exist
- missing-data behavior field

**Verify:**
```bash
python3 -m pytest scripts/research/tests/test_bbn_structure_search.py -q
```

### Task 7: Add execution-tree replay pack

**Objective:** preserve old execution-tree behavior while allowing better decisions.

**Files:**
- Create: `scripts/research/execution_tree_replay_pack.py`
- Test: `scripts/research/tests/test_execution_tree_replay_pack.py`

**Pack contains:**
- before trace
- after trace
- decisive input deltas
- branch/gate/bias diff
- expected recommendation text
- regression fixtures

**Verify:**
```bash
python3 -m pytest scripts/research/tests/test_execution_tree_replay_pack.py -q
```

### Task 8: Add compression retrospective command

**Objective:** keep the Heuristic System from turning into rule sludge.

**Files:**
- Create: `scripts/research/heuristic_retrospective.py`
- Test: `scripts/research/tests/test_heuristic_retrospective.py`

**Compression output:**
```text
top repeated failure tags
dead branches
duplicate hypotheses
families needing rewrite
rules to promote into mutation templates
tests/golden traces to add
catalog updates required
```

**Verify:**
```bash
python3 -m pytest scripts/research/tests/test_heuristic_retrospective.py -q
```

### Task 9: Wire docs to agent entry points

**Objective:** make future models discover the HL loop before doing random factor search.

**Files:**
- Modify: `AGENTS.md`
- Modify: `docs/factor-catalog.md`
- Modify: `docs/plans/2026-05-05-execution-tree-factor-auto-quant-todo.md`

**Rules to add:**
- read HL contract before factor iteration
- emit candidate attempt schema for every candidate
- run chain verdict before promotion
- run compression retrospective every 10-20 attempts

**Verify:**
```bash
grep -n "Heuristic Learning" AGENTS.md docs/factor-catalog.md docs/plans/2026-05-05-execution-tree-factor-auto-quant-todo.md
```

### Task 10: Run one no-code smoke loop

**Objective:** prove the contract can run without touching Rust runtime.

**Commands:**
```bash
cargo build
./target/debug/ict-engine analyze --demo --human --state-dir /tmp/ict-hl-smoke
./target/debug/ict-engine workflow-status --symbol NQ --state-dir /tmp/ict-hl-smoke --human
python3 scripts/research/heuristic_chain_verdict.py \
  --candidate-id demo-smoke \
  --state-dir /tmp/ict-hl-smoke \
  --output /tmp/ict-hl-smoke/NQ/candidates/demo-smoke/chain_verdict.json
python3 scripts/research/heuristic_retrospective.py \
  --state-dir /tmp/ict-hl-smoke \
  --output /tmp/ict-hl-smoke/retrospective.md
```

**Expected:**
- no repo pollution
- verdict artifact exists
- retrospective artifact exists
- workflow trace remains readable

---

## How This Changes Factor Search

Old behavior:

```text
try factor -> see Sharpe -> mutate params -> repeat
```

New behavior:

```text
choose target layer -> define hypothesis -> run factor -> classify failure layer
-> if factor weak: rewrite payoff source
-> if regime weak: improve classifier/calibration
-> if BBN weak: add/modify evidence only with entropy/log-loss proof
-> if path-ranker weak: collect mature structural rows
-> if execution-tree weak: add replay/golden trace and adjust branch evidence
-> compress lessons into templates/catalog/tests
```

This is how the model gains project-local "intuition": intuition is stored as structured failure history and reusable mutation surfaces.

---

## Practical Heuristics for Higher Sharpe

The loop should prefer factors with one clear return source:

1. **Trend convexity**
   - wins are larger than losses
   - accepts lower win-rate
   - needs persistence + MTF alignment + low crowding

2. **Mean reversion snapback**
   - high win-rate possible
   - needs overstretch + liquidity reclaim + exhaustion
   - fails in trend expansion; regime gate critical

3. **Volatility risk premium / carry**
   - high hit-rate but tail-risk heavy
   - needs VVIX/VIX, IV-HV, VIX term structure, stress filter
   - BBN contradiction handling critical

4. **Liquidity/session edge**
   - depends on killzone, sweep, reclaim, volume participation
   - execution tree must own stop realism and slippage risk

5. **Cross-market lag/confirmation**
   - useful for evidence quality and BBN
   - less likely to be standalone trade engine

High Sharpe usually comes from selecting the correct payoff source for the current regime, not from making one universal signal stronger.

---

## Practical Heuristics for 95% Regime Confidence

A regime factor should be built from these angles:

1. **Persistence:** mean segment length, transition hazard, flip-rate suppression.
2. **Separation:** factor distributions differ by regime; use eta2 / mutual information.
3. **Transition timing:** detects regime change within an allowed window.
4. **Cross-source agreement:** MECE label, HMM/Viterbi, outcome-defined regimes, changepoint labels.
5. **Resonance:** lower timeframe state agrees with higher timeframe context unless reversal hypothesis says otherwise.
6. **Calibration:** predicted confidence matches empirical correctness.
7. **Coverage:** high precision at 95% confidence is useless if it covers only rare anecdotes; report both.

95% should be a calibrated operating mode:

```text
if confidence >= 0.95 and conformal_set_size == 1 and calibration_ece <= 0.05:
    allow regime-specific execution factors
else:
    keep regime as context / evidence only
```

---

## Practical Heuristics for BBN Nodes and Evidence

Keep BBN small. Nodes should answer decision questions:

```text
market_regime: what environment are we in?
liquidity_context: can this setup execute cleanly?
factor_alignment: do independent signals agree?
factor_uncertainty: how fragile is the signal?
multi_timeframe_resonance: is timeframe stack supportive?
crowding_pressure: is the trade overcrowded?
dealer_pressure: is options positioning supportive or hostile?
session_quality: is this the right liquidity window?
entry_quality: should this setup be considered?
trade_outcome: what happened?
```

Evidence rule:

```text
new evidence must either increase posterior confidence when right,
or increase contradiction/uncertainty when wrong.
```

If evidence only adds noise, it should stay out of BBN and maybe remain as raw feature for CatBoost.

---

## Practical Heuristics for Execution Tree

Execution tree should not be a score blender. It should be an action policy:

```text
fill_viable: act now; edge + execution + regime agree
wait_for_reversion: signal interesting but price path not ready
block_crowded: prediction may be right but execution risk is bad
transition_guardrail: regime change risk; reduce size or wait
```

For higher win-rate and better R/R, execution tree needs:

- stop distance realism
- session liquidity quality
- slippage/crowding block
- expected path asymmetry
- MTF contradiction handling
- BBN uncertainty visible in split reason
- path-ranker contribution visible in trace

---

## First Milestone

**Milestone:** one candidate can be replayed end-to-end as HL.

Done when:

```text
candidate attempt schema exists
chain verdict exists
regime confidence report exists
path-ranking maturity report exists
execution-tree replay pack exists
retrospective exists
all artifacts are under /tmp state dir
one docs link lets future agents find the loop
```

This is enough to let future models self-iterate without chat memory.

---

## Risk Controls

- Do not modify runtime Rust until a public artifact proves the missing surface.
- Do not promote from IS-only Sharpe.
- Do not accept 95% confidence without calibration/coverage proof.
- Do not add BBN nodes without uncertainty/log-loss value.
- Do not treat `trainer_artifact=ready` as mature CatBoost closure.
- Do not let Auto-Quant output pollute repo root.
- Do not overwrite unrelated dirty work; current repo has existing modified files.

---

## Final Answer

Yes: the article gives a directly applicable pattern. The model gains this ability not by "understanding trading" in one prompt, but by turning `ict-engine` into a maintained Heuristic System. The project already has the raw organs. This plan adds the missing nervous system: structured attempts, failure memory, calibrated regime confidence, evidence gates, replay packs, and compression retrospectives.
