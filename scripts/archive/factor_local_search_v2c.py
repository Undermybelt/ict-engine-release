#!/usr/bin/env python3
"""Phase 1.6: Clean isolated comparison — each run gets its own state_dir.
Grid: lookback [13, 14, 15] × expansion [1.33, 1.36, 1.40] = 9 runs.
No baseline contamination."""

import json, subprocess, time, pathlib, itertools
from concurrent.futures import ProcessPoolExecutor, as_completed

REPO = pathlib.Path('/Users/thrill3r/projects-ict-engine/ict-engine')
PHASE_DIR = REPO / 'state_isolated_v2c'
PHASE_DIR.mkdir(parents=True, exist_ok=True)
RESULTS_FILE = PHASE_DIR / 'results.json'

DATA_BASE = '/Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf'
BIN = str(REPO / 'target' / 'release' / 'ict-engine')

# Fixed secondary params at run013 center (confirmed near-optimal)
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
    'lookback':            [13.0, 14.0, 15.0],
    'expansion_threshold': [1.33, 1.36, 1.40],
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


def extract_json(text):
    first_brace = text.find('{')
    if first_brace == -1:
        return None
    depth = 0
    for ci, ch in enumerate(text[first_brace:]):
        if ch == '{': depth += 1
        elif ch == '}': depth -= 1
        if depth == 0:
            try:
                return json.loads(text[first_brace:first_brace + ci + 1])
            except Exception:
                return None
    return None


def run_one(args):
    i, overrides, desc = args
    # Each run gets its own state_dir — no baseline contamination
    run_state = PHASE_DIR / f'state_run_{i:03d}'
    run_state.mkdir(parents=True, exist_ok=True)

    spec = {
        'mutation_id': f'isolated-v2c-{i:03d}',
        'base_factor': 'structure_ict',
        'hypothesis': f'Isolated clean comparison: {desc}',
        'parameter_overrides': overrides,
        'direction_hints': {},
        'step_size_hints': {},
        'enabled_overrides': {},
        'evaluate_expansion_preview': False,
    }
    spec_path = run_state / 'spec.json'
    spec_path.write_text(json.dumps(spec, indent=2))

    cmd = [
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
                c_after = row.get('composite_after')
                c_str = f'composite={c_after:.4f}' if c_after is not None else ''
                print(f'[{len(results) - len(existing)}/{len(todo)}] run {i:03d}: delta={d_str} {c_str} | {row["desc"]}')
            except Exception as e:
                print(f'run {i:03d}: EXCEPTION {e}')

    results = sorted(results, key=lambda r: r['i'])
    RESULTS_FILE.write_text(json.dumps(results, indent=2))

    print(f'\n{"=" * 70}')
    print(f'Phase 1.6 — Isolated Clean Comparison')
    print(f'{"=" * 70}')
    print(f'\n{"run":>4} | {"lookback":>8} | {"expansion":>10} | {"delta":>10} | {"composite":>10} | accepted')
    print('-' * 70)
    for r in sorted(results, key=lambda x: x.get('score_delta') or -999, reverse=True):
        ov = r['overrides']
        d = r.get('score_delta')
        d_str = f'{d:+.4f}' if d is not None else 'N/A'
        c = r.get('composite_after')
        c_str = f'{c:.4f}' if c is not None else 'N/A'
        print(f'{r["i"]:4d} | {ov["lookback"]:>8} | {ov["expansion_threshold"]:>10} | {d_str:>10} | {c_str:>10} | {r.get("accepted", "?")}')

    deltas = [r['score_delta'] for r in results if r.get('score_delta') is not None]
    composites = [r['composite_after'] for r in results if r.get('composite_after') is not None]
    if deltas:
        print(f'\ndelta range: [{min(deltas):+.4f}, {max(deltas):+.4f}]')
    if composites:
        print(f'composite range: [{min(composites):.4f}, {max(composites):.4f}]')
        best = max(results, key=lambda r: r.get('composite_after') or -999)
        print(f'\nBest by composite: run {best["i"]:03d} composite={best["composite_after"]:.4f} | {best["desc"]}')
    best_d = max(results, key=lambda r: r.get('score_delta') or -999)
    print(f'Best by delta:     run {best_d["i"]:03d} delta={best_d["score_delta"]:+.4f} | {best_d["desc"]}')
