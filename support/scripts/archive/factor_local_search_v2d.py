#!/usr/bin/env python3
"""Phase 1.7: Extend grid beyond v2c boundary.
lookback [15, 16, 18, 20] × expansion [1.40, 1.45, 1.50, 1.60] = 16 runs.
Each run isolated state_dir."""

import itertools
import json
import pathlib
import subprocess
import time
from concurrent.futures import ProcessPoolExecutor, as_completed
from path_defaults import resolve_binary_path, resolve_cleaned_data_root, resolve_repo_root

REPO = resolve_repo_root(__file__)
PHASE_DIR = REPO / 'state_isolated_v2d'
PHASE_DIR.mkdir(parents=True, exist_ok=True)
RESULTS_FILE = PHASE_DIR / 'results.json'
DATA_BASE = resolve_cleaned_data_root(__file__)
BIN = resolve_binary_path(__file__)

FIXED = {
    'sweep_atr_multiplier': 1.05,
    'sweep_weight': 1.3,
    'unconfirmed_sweep_weight': 0.55,
    'opposing_sweep_penalty': 1.2,
    'post_sweep_displacement_weight': 1.25,
    'sweep_recency_bars': 8.0,
    'sweep_return_bars': 7.0,
}

GRID = {
    'lookback':            [15.0, 16.0, 18.0, 20.0],
    'expansion_threshold': [1.40, 1.45, 1.50, 1.60],
}

specs = []
for i, combo in enumerate(itertools.product(*GRID.values()), 1):
    overrides = {**FIXED}
    desc_parts = []
    for j, key in enumerate(GRID.keys()):
        overrides[key] = combo[j]
        desc_parts.append(f'{key}={combo[j]}')
    specs.append((i, overrides, ', '.join(desc_parts)))

TOTAL = len(specs)
print(f'Total specs: {TOTAL} (each with isolated state_dir)')
print(f'Repo: {REPO}')
print(f'Data root: {DATA_BASE}')
print(f'Binary: {BIN}')


def extract_json(text):
    first_brace = text.find('{')
    if first_brace == -1:
        return None
    depth = 0
    for ci, ch in enumerate(text[first_brace:]):
        if ch == '{':
            depth += 1
        elif ch == '}':
            depth -= 1
        if depth == 0:
            try:
                return json.loads(text[first_brace:first_brace + ci + 1])
            except Exception:
                return None
    return None


def run_one(args):
    i, overrides, desc = args
    run_state = PHASE_DIR / f'state_run_{i:03d}'
    run_state.mkdir(parents=True, exist_ok=True)

    spec = {
        'mutation_id': f'isolated-v2d-{i:03d}',
        'base_factor': 'structure_ict',
        'hypothesis': f'Extended grid search: {desc}',
        'parameter_overrides': overrides,
        'direction_hints': {},
        'step_size_hints': {},
        'enabled_overrides': {},
        'evaluate_expansion_preview': False,
    }
    spec_path = run_state / 'spec.json'
    spec_path.write_text(json.dumps(spec, indent=2))

    cmd = [
        str(BIN), 'factor-research', '--symbol', 'NQ',
        '--data', str(DATA_BASE / 'cleaned-15m' / 'nq.continuous-15m.json'),
        '--data-1m', str(DATA_BASE / 'cleaned-1m' / 'nq.continuous-1m.json'),
        '--data-5m', str(DATA_BASE / 'cleaned-5m' / 'nq.continuous-5m.json'),
        '--data-15m', str(DATA_BASE / 'cleaned-15m' / 'nq.continuous-15m.json'),
        '--data-1h', str(DATA_BASE / 'cleaned-1h' / 'nq.continuous-1h.json'),
        '--data-4h', str(DATA_BASE / 'cleaned-4h' / 'nq.continuous-4h.json'),
        '--data-1d', str(DATA_BASE / 'cleaned-1d' / 'nq.continuous-1d.json'),
        '--objective', 'expansion_manipulation',
        '--ensemble', '--emit-mutation-evaluation',
        '--state-dir', str(run_state),
        '--mutation-spec', str(spec_path),
    ]

    t0 = time.time()
    proc = subprocess.run(cmd, cwd=REPO, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    dur = round(time.time() - t0, 2)
    (run_state / 'output.log').write_text(proc.stdout)

    payload = extract_json(proc.stdout)
    row = {
        'i': i, 'desc': desc, 'overrides': overrides,
        'exit_code': proc.returncode, 'duration_sec': dur,
        'score_delta': None, 'accepted': None, 'reason': None,
        'composite_before': None, 'composite_after': None,
        'top_factor': payload.get('top_factor') if payload else None,
    }
    if payload and payload.get('factor_mutation_evaluation'):
        ev = payload['factor_mutation_evaluation']
        row['score_delta'] = ev.get('score_delta')
        row['accepted'] = ev.get('accepted')
        row['reason'] = ev.get('reason')
        if ev.get('metrics_before'):
            row['composite_before'] = ev['metrics_before'].get('best_factor_composite_score')
        if ev.get('metrics_after'):
            row['composite_after'] = ev['metrics_after'].get('best_factor_composite_score')
    return row


if __name__ == '__main__':
    existing = json.loads(RESULTS_FILE.read_text()) if RESULTS_FILE.exists() else []
    done_ids = {r['i'] for r in existing}
    todo = [(i, ov, desc) for i, ov, desc in specs if i not in done_ids]
    print(f'Done: {len(done_ids)}, remaining: {len(todo)}')

    results = list(existing)
    with ProcessPoolExecutor(max_workers=4) as pool:
        futures = {pool.submit(run_one, (i, ov, desc)): i for i, ov, desc in todo}
        for fut in as_completed(futures):
            i = futures[fut]
            try:
                row = fut.result()
                results.append(row)
                results_sorted = sorted(results, key=lambda r: r['i'])
                RESULTS_FILE.write_text(json.dumps(results_sorted, indent=2))
                d = row['score_delta']
                d_str = f'{d:+.4f}' if d is not None else 'N/A'
                c = row.get('composite_after')
                c_str = f'{c:.4f}' if c is not None else '?'
                print(f'[{len(results) - len(existing)}/{len(todo)}] run {i:03d}: delta={d_str} composite={c_str} | {row["desc"]}')
            except Exception as e:
                print(f'run {i:03d}: EXCEPTION {e}')

    results = sorted(results, key=lambda r: r['i'])
    RESULTS_FILE.write_text(json.dumps(results, indent=2))

    print(f'\n{"=" * 80}')
    print('Combined results: v2c (9 runs) + v2d (16 runs)')
    print(f'{"=" * 80}')

    all_results = list(results)
    v2c_file = REPO / 'state_isolated_v2c' / 'results.json'
    if v2c_file.exists():
        v2c = json.loads(v2c_file.read_text())
        for r in v2c:
            r['source'] = 'v2c'
        all_results = v2c + [dict(**r, source='v2d') for r in results]

    print(f'\n{"src":>4} {"run":>4} | {"lookback":>8} | {"expansion":>10} | {"composite":>10} | {"delta":>10}')
    print('-' * 70)
    for r in sorted(all_results, key=lambda x: x.get('composite_after') or 0, reverse=True):
        ov = r['overrides']
        c = r.get('composite_after')
        d = r.get('score_delta')
        c_str = f'{c:.4f}' if c is not None else 'N/A'
        d_str = f'{d:+.4f}' if d is not None else 'N/A'
        src = r.get('source', '?')
        print(f'{src:>4} {r["i"]:4d} | {ov["lookback"]:>8} | {ov["expansion_threshold"]:>10} | {c_str:>10} | {d_str:>10}')
