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

# Run5 center — best from previous sweep (delta=-0.002)
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

# 3 key params to grid-search, rest fixed at run5 values
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
        'evaluate_expansion_preview': False,
    }
    for j, key in enumerate(GRID_AXES.keys()):
        spec['parameter_overrides'][key] = combo[j]
    return spec


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
    i, combo = args
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
        'i': i,
        'params': {k: combo[j] for j, k in enumerate(GRID_AXES.keys())},
        'exit_code': proc.returncode,
        'duration_sec': dur,
        'score_delta': None,
        'accepted': None,
        'reason': None,
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
        futures = {pool.submit(run_one, (i, combo)): i for i, combo in todo}
        for fut in as_completed(futures):
            i = futures[fut]
            try:
                row = fut.result()
                results.append(row)
                results_sorted = sorted(results, key=lambda r: r['i'])
                RESULTS_FILE.write_text(json.dumps(results_sorted, indent=2))
                print(f'[{len(results) - len(existing)}/{len(todo)}] run {i:03d}: '
                      f'delta={row["score_delta"]} accepted={row["accepted"]}')
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
        print(f'delta range: [{min(deltas):.4f}, {max(deltas):.4f}], mean={sum(deltas) / len(deltas):.4f}')
