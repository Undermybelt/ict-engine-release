# Factor Iteration v2 — 局部搜索 + Cluster Jump 混合策略

## 背景

上一轮 6-run 线性扫描结论：
- run5 (lookback=10, expansion=1.25) delta=-0.002 最接近 baseline → 局部最优邻域
- 线性扫只覆盖单调方向，搜索面太窄
- `evaluate_expansion_preview=true` 触发额外 bridge/pre_bayes gate 惩罚，干扰纯参数敏感度

## 策略

三阶段执行，全部 background 运行：

### Phase 1: 局部精细搜索（围绕 run5 最优邻域）

以 run5 参数为中心，±小步长网格搜索，`evaluate_expansion_preview=false`。

种子参数中心：
```json
{
  "lookback": 10.0,
  "expansion_threshold": 1.25,
  "sweep_atr_multiplier": 1.05,
  "sweep_weight": 1.3,
  "unconfirmed_sweep_weight": 0.55,
  "opposing_sweep_penalty": 1.2,
  "post_sweep_displacement_weight": 1.25,
  "sweep_recency_bars": 8.0,
  "sweep_return_bars": 7.0
}
```

搜索网格（每个参数 ±2 步）：
- lookback: [8, 9, 10, 11, 12]
- expansion_threshold: [1.15, 1.20, 1.25, 1.30, 1.35]
- sweep_weight: [1.1, 1.2, 1.3, 1.4, 1.5]

其余参数固定在 run5 值。用 Latin Hypercube 采样 ~20 组合（不做全网格 125 组合）。

### Phase 2: Cluster Jump 探索

用引擎内置 4 个 cluster preset 各跑 3 轮 autoresearch：
1. `displacement_fvg_cluster`
2. `mss_bos_cluster`
3. `premium_discount_ote_cluster`
4. `smt_cluster`

每个 cluster 用 `factor-autoresearch --iterations 3 --max-cluster-fail-streak 2`。

### Phase 3: 交叉验证

取 Phase 1 最优 spec + Phase 2 各 cluster 最优 spec，做 5-fold 交叉对比。

---

## 执行命令

### 构建

```bash
cd /Users/thrill3r/projects-ict-engine/ict-engine
cargo build --release
```

### Phase 1 — 局部搜索脚本

```bash
python3 scripts/factor_local_search_v2.py
```

脚本内容（需要先创建）：

```python
#!/usr/bin/env python3
"""Phase 1: Latin Hypercube local search around run5 optimum.
Runs 20 specs in parallel (4 workers), evaluate_expansion_preview=false."""

import json, subprocess, time, pathlib, copy, itertools, random
from concurrent.futures import ProcessPoolExecutor, as_completed

random.seed(2026)

REPO = pathlib.Path('/Users/thrill3r/projects-ict-engine/ict-engine')
STATE = REPO / 'state_local_search_v2'
STATE.mkdir(parents=True, exist_ok=True)
OUTDIR = STATE / 'runs'
OUTDIR.mkdir(parents=True, exist_ok=True)
RESULTS_FILE = STATE / 'results.json'

DATA_BASE = '/Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf'
BIN = str(REPO / 'target' / 'release' / 'ict-engine')

ARGS_COMMON = [
    BIN, 'factor-research', '--symbol', 'NQ',
    '--data', f'{DATA_BASE}/cleaned-15m/nq.continuous-15m.json',
    '--data-1m', f'{DATA_BASE}/cleaned-1m/nq.continuous-1m.json',
    '--data-5m', f'{DATA_BASE}/cleaned-5m/nq.continuous-5m.json',
    '--data-15m', f'{DATA_BASE}/cleaned-15m/nq.continuous-15m.json',
    '--data-1h', f'{DATA_BASE}/cleaned-1h/nq.continuous-1h.json',
    '--data-4h', f'{DATA_BASE}/cleaned-4h/nq.continuous-4h.json',
    '--data-1d', f'{DATA_BASE}/cleaned-1d/nq.continuous-1d.json',
    '--objective', 'expansion_manipulation',
    '--ensemble', '--emit-mutation-evaluation',
    '--state-dir', str(STATE),
]

# Run5 center
CENTER = {
    'lookback': 10.0,
    'expansion_threshold': 1.25,
    'sweep_atr_multiplier': 1.05,
    'sweep_weight': 1.3,
    'unconfirmed_sweep_weight': 0.55,
    'opposing_sweep_penalty': 1.2,
    'post_sweep_displacement_weight': 1.25,
    'sweep_recency_bars': 8.0,
    'sweep_return_bars': 7.0,
}

# 3 key params to grid-search, rest fixed
GRID_AXES = {
    'lookback':              [8.0, 9.0, 10.0, 11.0, 12.0],
    'expansion_threshold':   [1.15, 1.20, 1.25, 1.30, 1.35],
    'sweep_weight':          [1.1, 1.2, 1.3, 1.4, 1.5],
}

# Latin Hypercube: pick 20 from 125 combos
all_combos = list(itertools.product(*GRID_AXES.values()))
random.shuffle(all_combos)
SAMPLES = all_combos[:20]

def make_spec(i, combo):
    spec = {
        'mutation_id': f'local-v2-{i:03d}',
        'base_factor': 'structure_ict',
        'hypothesis': f'Local search around run5 optimum: lookback={combo[0]}, expansion={combo[1]}, sweep_weight={combo[2]}',
        'parameter_overrides': {**CENTER},
        'direction_hints': {},
        'step_size_hints': {},
        'enabled_overrides': {},
        'evaluate_expansion_preview': False,  # 关掉额外 gate
    }
    for j, key in enumerate(GRID_AXES.keys()):
        spec['parameter_overrides'][key] = combo[j]
    return spec

def extract_json(text):
    last_brace = text.rfind('{')
    if last_brace == -1:
        return None
    depth = 0
    for ci, ch in enumerate(text[last_brace:]):
        if ch == '{': depth += 1
        elif ch == '}': depth -= 1
        if depth == 0:
            try:
                return json.loads(text[last_brace:last_brace+ci+1])
            except:
                return None
    return None

def run_one(i, combo):
    spec = make_spec(i, combo)
    spec_path = STATE / f'local-v2-{i:03d}.json'
    spec_path.write_text(json.dumps(spec, indent=2))
    cmd = ARGS_COMMON + ['--mutation-spec', str(spec_path)]
    t0 = time.time()
    proc = subprocess.run(cmd, cwd=REPO, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    dur = round(time.time() - t0, 2)
    (OUTDIR / f'run_{i:03d}.log').write_text(proc.stdout)
    payload = extract_json(proc.stdout)
    row = {
        'i': i, 'params': {k: combo[j] for j, k in enumerate(GRID_AXES.keys())},
        'exit_code': proc.returncode, 'duration_sec': dur,
        'score_delta': None, 'accepted': None, 'reason': None,
        'top_factor': payload.get('top_factor') if payload else None,
    }
    if payload and payload.get('factor_mutation_evaluation'):
        ev = payload['factor_mutation_evaluation']
        row['score_delta'] = ev.get('score_delta')
        row['accepted'] = ev.get('accepted')
        row['reason'] = ev.get('reason')
    return row

if __name__ == '__main__':
    existing = json.loads(RESULTS_FILE.read_text()) if RESULTS_FILE.exists() else []
    done_ids = {r['i'] for r in existing}
    todo = [(i, combo) for i, combo in enumerate(SAMPLES, 1) if i not in done_ids]
    print(f'Done: {len(done_ids)}, remaining: {len(todo)}')
    results = list(existing)
    with ProcessPoolExecutor(max_workers=4) as pool:
        futures = {pool.submit(run_one, i, combo): i for i, combo in todo}
        for fut in as_completed(futures):
            i = futures[fut]
            try:
                row = fut.result()
                results.append(row)
                results_sorted = sorted(results, key=lambda r: r['i'])
                RESULTS_FILE.write_text(json.dumps(results_sorted, indent=2))
                print(f'[{len(results)-len(existing)}/{len(todo)}] run {i:03d}: delta={row["score_delta"]} accepted={row["accepted"]}')
            except Exception as e:
                print(f'run {i:03d}: EXCEPTION {e}')
    results = sorted(results, key=lambda r: r['i'])
    RESULTS_FILE.write_text(json.dumps(results, indent=2))
    deltas = [r['score_delta'] for r in results if r.get('score_delta') is not None]
    if deltas:
        best = max(results, key=lambda r: r.get('score_delta') or -999)
        print(f'\n=== Phase 1 Summary ===')
        print(f'Total: {len(results)}, Best: run {best["i"]:03d} delta={best["score_delta"]:.4f}')
        print(f'Params: {best["params"]}')
        print(f'delta range: [{min(deltas):.4f}, {max(deltas):.4f}], mean={sum(deltas)/len(deltas):.4f}')
```

### Phase 2 — Cluster Jump Autoresearch

每个 cluster 独立 state_dir，避免互相污染：

```bash
# displacement_fvg_cluster
cat > /tmp/seed_displacement_fvg.json << 'EOF'
{
  "mutation_id": "cluster-displacement-fvg-001",
  "base_factor": "structure_ict",
  "hypothesis": "Cluster jump: displacement_fvg with post_sweep_displacement focus",
  "parameter_overrides": {
    "post_sweep_displacement_weight": 1.35,
    "sweep_weight": 1.10,
    "unconfirmed_sweep_weight": 0.45,
    "expansion_threshold": 1.05,
    "lookback": 10.0,
    "sweep_recency_bars": 8.0,
    "sweep_return_bars": 7.0
  },
  "direction_hints": {
    "cluster_jump": "displacement_fvg_cluster",
    "cluster_jump_cycle": "0",
    "available_clusters": "displacement_fvg_cluster|mss_bos_cluster|premium_discount_ote_cluster|smt_cluster"
  },
  "step_size_hints": {},
  "enabled_overrides": {},
  "evaluate_expansion_preview": false
}
EOF

./target/release/ict-engine factor-autoresearch \
  --symbol NQ \
  --data /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-15m/nq.continuous-15m.json \
  --data-1m /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-1m/nq.continuous-1m.json \
  --data-5m /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-5m/nq.continuous-5m.json \
  --data-15m /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-15m/nq.continuous-15m.json \
  --data-1h /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-1h/nq.continuous-1h.json \
  --data-4h /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-4h/nq.continuous-4h.json \
  --data-1d /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-1d/nq.continuous-1d.json \
  --objective expansion_manipulation \
  --mutation-spec /tmp/seed_displacement_fvg.json \
  --iterations 3 --max-cluster-fail-streak 2 \
  --ensemble \
  --state-dir state_cluster_dfvg \
  2>&1 | tee state_cluster_dfvg/autoresearch.log &

# mss_bos_cluster
cat > /tmp/seed_mss_bos.json << 'EOF'
{
  "mutation_id": "cluster-mss-bos-001",
  "base_factor": "structure_ict",
  "hypothesis": "Cluster jump: mss_bos with lookback/expansion focus",
  "parameter_overrides": {
    "lookback": 10.0,
    "expansion_threshold": 1.18,
    "sweep_return_bars": 5.0,
    "opposing_sweep_penalty": 1.25,
    "sweep_weight": 1.3,
    "unconfirmed_sweep_weight": 0.55,
    "sweep_recency_bars": 8.0
  },
  "direction_hints": {
    "cluster_jump": "mss_bos_cluster",
    "cluster_jump_cycle": "1",
    "available_clusters": "displacement_fvg_cluster|mss_bos_cluster|premium_discount_ote_cluster|smt_cluster"
  },
  "step_size_hints": {},
  "enabled_overrides": {},
  "evaluate_expansion_preview": false
}
EOF

./target/release/ict-engine factor-autoresearch \
  --symbol NQ \
  --data /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-15m/nq.continuous-15m.json \
  --data-1m /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-1m/nq.continuous-1m.json \
  --data-5m /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-5m/nq.continuous-5m.json \
  --data-15m /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-15m/nq.continuous-15m.json \
  --data-1h /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-1h/nq.continuous-1h.json \
  --data-4h /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-4h/nq.continuous-4h.json \
  --data-1d /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-1d/nq.continuous-1d.json \
  --objective expansion_manipulation \
  --mutation-spec /tmp/seed_mss_bos.json \
  --iterations 3 --max-cluster-fail-streak 2 \
  --ensemble \
  --state-dir state_cluster_mss \
  2>&1 | tee state_cluster_mss/autoresearch.log &

# premium_discount_ote_cluster
cat > /tmp/seed_pd_ote.json << 'EOF'
{
  "mutation_id": "cluster-pd-ote-001",
  "base_factor": "structure_ict",
  "hypothesis": "Cluster jump: premium_discount_ote with recency/return bars focus",
  "parameter_overrides": {
    "lookback": 14.0,
    "expansion_threshold": 0.92,
    "sweep_recency_bars": 8.0,
    "sweep_return_bars": 6.0,
    "sweep_weight": 1.3,
    "unconfirmed_sweep_weight": 0.55,
    "opposing_sweep_penalty": 1.2
  },
  "direction_hints": {
    "cluster_jump": "premium_discount_ote_cluster",
    "cluster_jump_cycle": "2",
    "available_clusters": "displacement_fvg_cluster|mss_bos_cluster|premium_discount_ote_cluster|smt_cluster"
  },
  "step_size_hints": {},
  "enabled_overrides": {},
  "evaluate_expansion_preview": false
}
EOF

./target/release/ict-engine factor-autoresearch \
  --symbol NQ \
  --data /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-15m/nq.continuous-15m.json \
  --data-1m /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-1m/nq.continuous-1m.json \
  --data-5m /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-5m/nq.continuous-5m.json \
  --data-15m /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-15m/nq.continuous-15m.json \
  --data-1h /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-1h/nq.continuous-1h.json \
  --data-4h /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-4h/nq.continuous-4h.json \
  --data-1d /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-1d/nq.continuous-1d.json \
  --objective expansion_manipulation \
  --mutation-spec /tmp/seed_pd_ote.json \
  --iterations 3 --max-cluster-fail-streak 2 \
  --ensemble \
  --state-dir state_cluster_pdote \
  2>&1 | tee state_cluster_pdote/autoresearch.log &

# smt_cluster
cat > /tmp/seed_smt.json << 'EOF'
{
  "mutation_id": "cluster-smt-001",
  "base_factor": "cross_market_smt",
  "hypothesis": "Cluster jump: smt with cross-market divergence focus",
  "parameter_overrides": {
    "lookback": 24.0,
    "sweep_atr_multiplier": 0.60,
    "sweep_weight": 0.72,
    "opposing_sweep_penalty": 1.05,
    "expansion_threshold": 1.25,
    "unconfirmed_sweep_weight": 0.55,
    "sweep_recency_bars": 8.0,
    "sweep_return_bars": 7.0
  },
  "direction_hints": {
    "cluster_jump": "smt_cluster",
    "cluster_jump_cycle": "3",
    "available_clusters": "displacement_fvg_cluster|mss_bos_cluster|premium_discount_ote_cluster|smt_cluster"
  },
  "step_size_hints": {},
  "enabled_overrides": {},
  "evaluate_expansion_preview": false
}
EOF

./target/release/ict-engine factor-autoresearch \
  --symbol NQ \
  --data /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-15m/nq.continuous-15m.json \
  --data-1m /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-1m/nq.continuous-1m.json \
  --data-5m /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-5m/nq.continuous-5m.json \
  --data-15m /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-15m/nq.continuous-15m.json \
  --data-1h /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-1h/nq.continuous-1h.json \
  --data-4h /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-4h/nq.continuous-4h.json \
  --data-1d /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-1d/nq.continuous-1d.json \
  --objective expansion_manipulation \
  --mutation-spec /tmp/seed_smt.json \
  --iterations 3 --max-cluster-fail-streak 2 \
  --ensemble \
  --state-dir state_cluster_smt \
  2>&1 | tee state_cluster_smt/autoresearch.log &

# 等待全部完成
wait
echo "=== All cluster autoresearch done ==="
```

### Phase 3 — 汇总对比

```bash
python3 scripts/factor_iteration_v2_summary.py
```

脚本内容（需要先创建）：

```python
#!/usr/bin/env python3
"""Phase 3: Collect best specs from Phase 1 (local search) and Phase 2 (cluster jumps).
Print comparison table and recommend next action."""

import json, pathlib

REPO = pathlib.Path('/Users/thrill3r/projects-ict-engine/ict-engine')

def load_best_from_local_search():
    results_file = REPO / 'state_local_search_v2' / 'results.json'
    if not results_file.exists():
        return None
    results = json.loads(results_file.read_text())
    valid = [r for r in results if r.get('score_delta') is not None]
    if not valid:
        return None
    best = max(valid, key=lambda r: r['score_delta'])
    return {'source': 'local_search', 'run': best['i'], 'delta': best['score_delta'],
            'params': best.get('params'), 'accepted': best.get('accepted')}

def load_best_from_cluster(cluster_name, state_dir_name):
    summary_file = REPO / state_dir_name / 'NQ' / 'factor_autoresearch_sessions.json'
    attempts_file = REPO / state_dir_name / 'NQ' / 'factor_autoresearch_attempts.json'
    if not attempts_file.exists():
        return None
    attempts = json.loads(attempts_file.read_text())
    if not attempts:
        return None
    best = max(attempts, key=lambda a: a['decision'].get('candidate_score', 0))
    return {'source': cluster_name, 'delta': best['evaluation'].get('score_delta', 0),
            'score': best['decision'].get('candidate_score', 0),
            'promoted': best['decision'].get('promoted_to_baseline', False),
            'attempt_id': best['attempt_id']}

if __name__ == '__main__':
    candidates = []
    local = load_best_from_local_search()
    if local:
        candidates.append(local)
        print(f"Local search best: run {local['run']:03d}, delta={local['delta']:.4f}, params={local['params']}")

    for cluster, state_dir in [
        ('displacement_fvg', 'state_cluster_dfvg'),
        ('mss_bos', 'state_cluster_mss'),
        ('premium_discount_ote', 'state_cluster_pdote'),
        ('smt', 'state_cluster_smt'),
    ]:
        result = load_best_from_cluster(cluster, state_dir)
        if result:
            candidates.append(result)
            print(f"{cluster}: delta={result['delta']:.4f}, score={result['score']:.4f}, promoted={result['promoted']}")
        else:
            print(f"{cluster}: no results")

    if candidates:
        best = max(candidates, key=lambda c: c.get('delta', -999))
        print(f"\n=== WINNER: {best['source']} with delta={best.get('delta', 'N/A')} ===")
    else:
        print("\nNo results found. Check logs.")
```

---

## 监控

```bash
# 查看 Phase 1 进度
tail -f state_local_search_v2/results.json | python3 -c "import sys,json; [print(f'runs: {len(json.loads(line))}') for line in sys.stdin if line.strip()]"

# 查看各 cluster 进度
for d in state_cluster_dfvg state_cluster_mss state_cluster_pdote state_cluster_smt; do
  echo "=== $d ===";
  cat $d/NQ/factor_autoresearch_live_snapshot.json 2>/dev/null | python3 -m json.tool 2>/dev/null || echo "not started";
done

# 查看 autoresearch session 状态
for d in state_cluster_dfvg state_cluster_mss state_cluster_pdote state_cluster_smt; do
  echo "=== $d ===";
  ./target/release/ict-engine factor-autoresearch-status --symbol NQ --state-dir $d --latest-only 2>/dev/null || echo "no data";
done
```

## 关键设计决策

1. `evaluate_expansion_preview=false` — 全部关掉，做纯参数敏感度分析
2. 每个 cluster 独立 state_dir — 避免 mutation_runs / learning_state 互相污染
3. Phase 1 用 Latin Hypercube 而非全网格 — 20 runs 覆盖 125 组合空间的代表性子集
4. Cluster seed specs 混合了 run5 最优参数 + 引擎内置 cluster preset 参数 — 不从零开始
5. `--iterations 3 --max-cluster-fail-streak 2` — 每个 cluster 最多 3 轮，连续失败 2 次自动跳下一个 cycle
