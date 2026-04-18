#!/usr/bin/env python3
"""Phase 1.5: Fine-grained search around run013 optimum (lookback=12, expansion=1.30, sweep_weight=1.3).
Expands lookback upward to 13+, tightens expansion grid, and adds secondary param perturbation."""

import json, subprocess, time, pathlib, itertools, random
from concurrent.futures import ProcessPoolExecutor, as_completed

random.seed(2027)

REPO = pathlib.Path('/Users/thrill3r/projects-ict-engine/ict-engine')
STATE = REPO / 'state_local_search_v2b'
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

# Run013 center (best from Phase 1, delta=+0.0025)
CENTER = {
    'lookback': 12.0,
    'expansion_threshold': 1.30,
    'sweep_atr_multiplier': 1.05,
    'sweep_weight': 1.3,
    'unconfirmed_sweep_weight': 0.55,
    'opposing_sweep_penalty': 1.2,
    'post_sweep_displacement_weight': 1.25,
    'sweep_recency_bars': 8.0,
    'sweep_return_bars': 7.0,
}

# Primary axes: finer grid around run013
GRID_PRIMARY = {
    'lookback':            [11.0, 11.5, 12.0, 12.5, 13.0, 14.0],
    'expansion_threshold': [1.22, 1.25, 1.28, 1.30, 1.33, 1.36],
}

# Secondary axes: perturb one at a time from center
SECONDARY_PERTURBATIONS = [
    {'opposing_sweep_penalty': 1.10},
    {'opposing_sweep_penalty': 1.30},
    {'post_sweep_displacement_weight': 1.15},
    {'post_sweep_displacement_weight': 1.35},
    {'unconfirmed_sweep_weight': 0.45},
    {'unconfirmed_sweep_weight': 0.65},
    {'sweep_recency_bars': 6.0},
    {'sweep_recency_bars': 10.0},
]

# Build sample list:
# 1) Latin Hypercube from primary grid: 18 of 36 combos
all_primary = list(itertools.product(*GRID_PRIMARY.values()))
random.shuffle(all_primary)
primary_samples = all_primary[:18]

# 2) Secondary perturbations at run013 center: 8 runs
specs = []
for i, combo in enumerate(primary_samples, 1):
    overrides = {**CENTER}
    for j, key in enumerate(GRID_PRIMARY.keys()):
        overrides[key] = combo[j]
    specs.append((i, overrides, f'lookback={combo[0]}, expansion={combo[1]}'))

for k, perturb in enumerate(SECONDARY_PERTURBATIONS):
    i = len(primary_samples) + k + 1
    overrides = {**CENTER, **perturb}
    desc = ', '.join(f'{p}={v}' for p, v in perturb.items())
    specs.append((i, overrides, f'center + {desc}'))

TOTAL = len(specs)
print(f'Total specs: {TOTAL} ({len(primary_samples)} primary + {len(SECONDARY_PERTURBATIONS)} secondary)')


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
    spec = {
        'mutation_id': f'local-v2b-{i:03d}',
        'base_factor': 'structure_ict',
        'hypothesis': f'Fine search around run013: {desc}',
        'parameter_overrides': overrides,
        'direction_hints': {},
        'step_size_hints': {},
        'enabled_overrides': {},
        'evaluate_expansion_preview': False,
    }
    spec_path = STATE / f'local-v2b-{i:03d}.json'
    spec_path.write_text(json.dumps(spec, indent=2))
    cmd = ARGS_COMMON + ['--mutation-spec', str(spec_path)]
    t0 = time.time()
    proc = subprocess.run(cmd, cwd=REPO, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    dur = round(time.time() - t0, 2)
    (OUTDIR / f'run_{i:03d}.log').write_text(proc.stdout)
    payload = extract_json(proc.stdout)
    row = {
        'i': i, 'desc': desc, 'overrides': overrides,
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
                marker = ' <<<' if d is not None and d > 0.0025 else ''
                print(f'[{len(results) - len(existing)}/{len(todo)}] run {i:03d}: delta={d_str} accepted={row["accepted"]} | {row["desc"]}{marker}')
            except Exception as e:
                print(f'run {i:03d}: EXCEPTION {e}')

    results = sorted(results, key=lambda r: r['i'])
    RESULTS_FILE.write_text(json.dumps(results, indent=2))
    deltas = [r['score_delta'] for r in results if r.get('score_delta') is not None]
    if deltas:
        best = max(results, key=lambda r: r.get('score_delta') or -999)
        print(f'\n{"=" * 60}')
        print(f'Phase 1.5 Summary')
        print(f'{"=" * 60}')
        print(f'Total: {len(results)}, Positive: {sum(1 for d in deltas if d > 0)}')
        print(f'Best: run {best["i"]:03d} delta={best["score_delta"]:+.4f}')
        print(f'  {best["desc"]}')
        print(f'  overrides: {json.dumps(best["overrides"], indent=2)}')
        print(f'delta range: [{min(deltas):.4f}, {max(deltas):.4f}], mean={sum(deltas) / len(deltas):.4f}')
        # Top 5
        top5 = sorted(results, key=lambda r: r.get('score_delta') or -999, reverse=True)[:5]
        print(f'\nTop 5:')
        for r in top5:
            print(f'  run {r["i"]:03d}: delta={r["score_delta"]:+.4f} | {r["desc"]}')
