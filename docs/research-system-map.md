# Research System Map

## Goal

本文件是 ict-engine 当前研究系统的总览图。目标是让人或 agent 不必翻遍命令、状态文件、脚本，直接知道：
- 哪些命令负责什么
- 哪些状态文件是 source of truth
- 哪些脚本适合哪类实验
- 何时用 isolated state，何时允许 batch
- 跑完后应看哪个产物判断真相

## Core command surfaces

### 1. `factor-research`
用途：
- 单次因子研究
- 产出因子排名 / reflection / optional mutation evaluation

常用参数：
- `--symbol`
- `--data`
- `--data-1m/5m/15m/1h/4h/1d`
- `--objective`
- `--mutation-spec`
- `--emit-mutation-evaluation`
- `--state-dir`

适合：
- 单次 spec 验证
- isolated run
- local search / cluster search 的底层执行器

### 2. `factor-autoresearch`
用途：
- 多轮 keep/discard 迭代
- 自动写 attempts / sessions / live snapshot / final summary

常用参数：
- `--symbol`
- `--data`
- `--data-1m/5m/15m/1h/4h/1d`
- `--objective`
- `--mutation-spec`
- `--resume-latest`
- `--iterations`
- `--max-cluster-fail-streak`
- `--state-dir`

适合：
- 簇跳转实验
- 续跑 session
- 跑完后直接看 session 级结果

### 3. `factor-autoresearch-status`
用途：
- 读 autoresearch 会话与 attempts
- 输出 session / attempts / cluster_scoreboard / best_attempt / interrupted 状态

常用参数：
- `--symbol`
- `--state-dir`
- `--latest-only`
- `--session-id`
- `--limit`

适合：
- 跑完后汇总真相
- 判断 completed / interrupted
- 看 cluster 层面的 keep/discard 与 best attempt

### 4. `factor-pipeline-debug`
用途：
- 看最新 sample 的全链路：
  - factor signal
  - diagnostics
  - pre-bayes
  - bridge
  - verdict
  - physics features

适合：
- 查 evidence/gate/bridge 瓶颈
- 看 `frame_physics_trace`
- 验证新 feature 是否真接到 debug 面

## State files (source of truth)

按 symbol 存在：`<state_dir>/<SYMBOL>/`

### Autoresearch truth
- `factor_autoresearch_attempts.json`
  - 每轮 attempt 记录
  - 包含 candidate spec / evaluation / decision / branch_summary
- `factor_autoresearch_sessions.json`
  - session 汇总
- `factor_autoresearch_live.json`
  - 运行中 snapshot
- `factor_autoresearch_final.json`
  - 正常完成时的 final summary artifact

### Factor mutation truth
- `factor_mutation_runs.json`
  - 单次 mutation evaluation 历史
  - 注意：这是 history，不是单个 mutation spec 输入

### Other important state
- `research_runs.json`
- `backtest_runs.json`
- `learning_state.json`
- `workflow_snapshot.json`
- `artifact_ledger.json`

## Correct result-reading order

### For single isolated run
1. `factor_mutation_runs.json`
2. stdout JSON
3. `factor-pipeline-debug` if needed

### For autoresearch session
1. `factor-autoresearch-status --latest-only`
2. `factor_autoresearch_final.json`
3. `factor_autoresearch_attempts.json`
4. `factor-pipeline-debug` on best/current attempt if needed

## Experiment modes

### A. Isolated state (preferred for comparison)
定义：
- 每个 run 独立 `state_dir`
- 不共享 learning state / mutation history / baseline drift

适合：
- 参数网格搜索
- objective 对比
- 真正的 apples-to-apples 对比

必须用 isolated 的场景：
- local search
- objective scoring对比
- feature ablation
- paired-data quality comparison

### B. Shared state (only for iterative session)
定义：
- 多轮 run 共用一个 `state_dir`
- baseline 会逐轮漂移

适合：
- `factor-autoresearch`
- 有明确 keep/discard / resume 语义的迭代实验

风险：
- 不能把不同轮结果当成独立样本比较
- 易出现 baseline contamination / cumulative uplift 假象

## Integrity rules

1. `--mutation-spec` 只能喂单个 spec JSON
   - 禁止喂：
     - `.csv`
     - history array json
     - attempt/run artifact json

2. 比较型实验默认 isolated state
3. `completed` 判定必须看：
   - `factor_autoresearch_live.json`
   - `factor_autoresearch_final.json`
   - session/attempts 闭合
4. 若 final artifact 不存在，不可把 session 当作可靠 completed

## Script families

### Public wrappers
- `scripts/search_local.py`
- `scripts/search_cluster.py`
- `scripts/evaluate_bottleneck.py`
- `scripts/research_verdict.py`
- `scripts/evidence_quality_breakdown.py`

用途：
- `search_local.py`: isolated param search
- `search_cluster.py`: cluster jump exploration
- `evaluate_bottleneck.py`: evidence/gate/shrink/bridge bottleneck experiments
- `research_verdict.py`: summarize existing state/result dirs into one verdict surface
- `evidence_quality_breakdown.py`: decompose evidence-quality score inputs transparently

默认规则：
- wrapper 默认只显示 help，不执行长跑
- `--run` 才执行 archived backend
- `--target` 看 backend 路径
- `--backend-help` 看非执行摘要

### Archived backends
主要 archived backends 现位于：
- `scripts/archive/factor_local_search_v2d.py`
- `scripts/archive/factor_cluster_jump_v2.py`
- `scripts/archive/pre_bayes_policy_tuning.py`
- 以及其余历史实验脚本

这些 backend 仍有研究属性，且部分仍含本机路径假设；对外优先经 public wrappers 进入。

## Recommended workflows

### Workflow 1: parameter comparison
- use isolated state
- run `python3 scripts/search_local.py --run`
- read `results.json`
- optionally summarize with `python3 scripts/research_verdict.py <state-or-result-dir>`
- never trust cumulative autoresearch deltas for comparison

### Workflow 2: keep/discard iteration
- use `factor-autoresearch`
- use one shared `state_dir`
- read `factor-autoresearch-status`
- use `--resume-latest` to continue

### Workflow 3: bottleneck diagnosis
- run `factor-pipeline-debug`
- inspect:
  - `gating_status`
  - `bridge_gap`
  - `evidence_quality_score`
  - `paired_market_quality_report`
  - `frame_physics_trace`
  - `recommended_actions`
- if still unclear, run:
  - `python3 scripts/evaluate_bottleneck.py --run`
  - `python3 scripts/evidence_quality_breakdown.py ...`

## Current known truths

1. `structure_ict` narrow param sweeps are near local optimum under current objective
2. enabling `evaluate_expansion_preview=true` materially changes scoring surface
3. `cross_market_smt` can produce keep in autoresearch, but direct paired-data parameter sweep was mostly flat
4. `frame_physics_trace` now exposes Pythagorean + OU features on debug surface
5. main bottlenecks remain around:
   - `evidence_quality_score`
   - `pre_bayes_gate_status`
   - `objective_market_shrink_weight`
   - `bridge_gap`

## If you only do one thing

After any non-trivial run, execute:

```bash
cargo run -- factor-autoresearch-status --symbol <SYM> --state-dir <DIR> --latest-only
```

and trust that output before chat summaries.
