# Agent First Runbook

Audience: internal/operator agents. For release onboarding, read `support/docs/first-run.md` first.

## One-line identity

`ict-engine` is a Rust CLI research OS for ICT-style market analysis, probabilistic trade reasoning, factor research, backtesting, and agent-readable workflow state.

## What this project is for

This repo is not just a market analyzer and not just a backtester.
It is a layered research system that lets an agent or human:
- inspect market structure
- compute probabilistic trade evidence
- research factors
- run mutation / autoresearch loops
- track workflow truth across runs
- decide whether to continue, pivot, or stop an experiment

## What an agent can do for the user here

1. Market-analysis assistant
- Run `analyze` / `factor-pipeline-debug`
- Translate outputs into human-readable trade reasoning
- Explain regime, gate, bridge, and trade-plan implications

2. Factor-research assistant
- Run `factor-research`
- Propose mutation specs
- Run isolated searches or autoresearch loops
- Identify local optima vs. real improvements

3. Research-integrity guard
- Detect contaminated comparisons
- Distinguish isolated vs shared-state experiments
- Check final summary artifacts before calling a run completed
- Prevent bad inputs like CSV/history JSON being used as mutation specs

4. Bottleneck debugger
- Diagnose evidence-quality, gate, bridge, and shrink issues
- Determine whether the blocker is factor quality or scoring logic

5. Workflow orchestrator
- Choose between isolated runs, autoresearch, background execution, and stop/pivot decisions

## The 6 most common user intents

### 1. "现在市场怎么看？"
Use:
```bash
cargo run -- analyze --symbol <SYM> --data-root <clean-root> --state-dir <state-dir>
```
If the question is specifically about why a factor/gate behaved a certain way, use:
```bash
cargo run -- factor-pipeline-debug --symbol <SYM> --data <cleaned-15m.json> --factor structure_ict --objective expansion_manipulation
```

### 2. "优化因子 / 跑实验"
If comparing parameter sets fairly:
- use isolated state dirs
- prefer `factor-research`
- or dedicated local-search scripts

If continuing a keep/discard loop:
```bash
cargo run -- factor-autoresearch --symbol <SYM> --data <cleaned-15m.json> --mutation-spec <spec.json> --iterations N --state-dir <dir>
```
Resume:
```bash
cargo run -- factor-autoresearch --symbol <SYM> --data <cleaned-15m.json> --resume-latest --iterations N --state-dir <dir>
```

### 3. "为什么 gate 不过？"
Use:
```bash
cargo run -- factor-pipeline-debug --symbol <SYM> --data <cleaned-15m.json> --factor <factor> --objective expansion_manipulation
```
Inspect:
- `evidence_quality_score`
- `gating_status`
- `bridge_gap`
- `recommended_actions`
- `frame_physics_trace`

### 4. "看看上轮结果 / 现在状态"
Use:
```bash
cargo run -- factor-autoresearch-status --symbol <SYM> --state-dir <dir> --latest-only
```
Trust this before chat memory.
Check:
- `effective_status`
- `interrupted`
- `final_summary_exists`
- `best_attempt`
- `cluster_scoreboard`

### 5. "后台跑，跑完回我"
Use Hermes slash command routing first:
- `/background <self-contained prompt>`
Do not only start a raw background shell process if the user clearly wants autonomous continuation and a delivered summary.

### 6. "能不能交易 / 该不该进场？"
Use:
- `analyze` for human-facing synthesis
- `factor-pipeline-debug` for precise gate/bridge truth
Do not answer from top-level factor score alone.

## First command to try by task type

| User need | First command |
|---|---|
| Human market read | `analyze` |
| Gate / bridge diagnosis | `factor-pipeline-debug` |
| Single factor mutation eval | `factor-research --emit-mutation-evaluation` |
| Keep/discard iterative loop | `factor-autoresearch` |
| Read autoresearch truth | `factor-autoresearch-status` |
| Compare parameter grids | isolated `factor-research` or local-search scripts |

## Core command surfaces

### `factor-research`
Single run research. Best for:
- isolated comparisons
- one-off mutation spec checks
- local search primitives

### `factor-autoresearch`
Multi-iteration keep/discard loop. Best for:
- resumable iterative search
- cluster jump exploration
- session-level experimentation

### `factor-autoresearch-status`
Truth reader for autoresearch sessions. Best for:
- checking completed vs interrupted
- reading best attempt
- seeing cluster scoreboard / fail streaks

### `factor-pipeline-debug`
Best command for explaining *why* a factor/gate behaved the way it did.

## Source-of-truth state files

Under `<state_dir>/<SYMBOL>/`:

### Autoresearch
- `factor_autoresearch_attempts.json`
- `factor_autoresearch_sessions.json`
- `factor_autoresearch_live.json`
- `factor_autoresearch_final.json`

### Mutation / research
- `factor_mutation_runs.json`
- `research_runs.json`
- `backtest_runs.json`
- `learning_state.json`
- `workflow_snapshot.json`
- `artifact_ledger.json`

## Correct result-reading order

### For a single isolated run
1. stdout JSON
2. `factor_mutation_runs.json`
3. `factor-pipeline-debug` if explanation is needed

### For autoresearch
1. `factor-autoresearch-status --latest-only`
2. `factor_autoresearch_final.json`
3. `factor_autoresearch_attempts.json`
4. `factor-pipeline-debug` on the best/current attempt if needed

## Experimental integrity rules

### Isolated vs shared state

#### Isolated state (default for fair comparison)
Use isolated state dirs when:
- comparing parameter sets
- comparing objectives
- doing feature ablation
- comparing paired-data quality

#### Shared state (only for iterative loops)
Use shared state only when:
- running `factor-autoresearch`
- intentionally allowing baseline promotion and cumulative keep/discard logic

### Do not confuse these two
A shared-state batch is not an independent comparison experiment.

## Hard rules / common mistakes

1. Do not pass raw CSV to `--data` for factor research.
   - Use cleaned JSON surfaces.

2. Do not pass `factor_mutation_runs.json` to `--mutation-spec`.
   - That is run history, not a single spec.

3. Do not trust a run as completed unless status closes.
   - Look for:
     - `factor_autoresearch_live.json`
     - `factor_autoresearch_final.json`
     - `effective_status`

4. Do not compare grid search results produced from one shared state dir.
   - That causes baseline contamination / cumulative drift.

5. Do not keep sweeping `structure_ict` parameters once isolated comparisons show baseline defaults remain best.

6. Do not treat PDA / ICT labels as direct Bayesian evidence without checking the actual evidence path.

## Current known truths (as of this repo state)

1. `structure_ict` narrow parameter sweeps are largely exhausted under current objective.
   - baseline defaults remain the strongest confirmed isolated parameter set.

2. `evaluate_expansion_preview=true` materially improves the scoring surface.
   - turning it off hides too much of the objective.

3. `cross_market_smt` can produce keep inside autoresearch,
   but direct paired-data parameter sweep was mostly flat,
   and YM paired-data previously exposed a boundary bug (now fixed).

4. Pythagorean + OU features are now exposed on the debug surface.
   - they are visible in `frame_physics_trace`
   - they also generate human/debug interpretation in `recommended_actions`
   - they are not yet part of core scoring/gate logic

5. The main bottlenecks remain:
- `evidence_quality_score`
- `pre_bayes_gate_status`
- `objective_market_shrink_weight`
- `bridge_gap`

## What to do when results are still unclear

If a run is hard to interpret:
1. run `factor-pipeline-debug`
2. inspect score decomposition docs
3. inspect autoresearch status
4. if still ambiguous, build/consult:
   - `evidence-quality-breakdown`
   - `research-verdict`

## Recommended reading order for a new agent

1. `README.md`
2. `support/docs/first-run.md`
3. `support/docs/research-system-map.md`
4. `support/docs/objective-scoring-map.md`
5. this file
6. then run the command matching the user intent

## If you only remember one thing

Before giving a conclusion, run:
```bash
cargo run -- factor-autoresearch-status --symbol <SYM> --state-dir <dir> --latest-only
```

and ground your answer in that output, not chat memory.
