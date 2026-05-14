#!/usr/bin/env python3
"""Phase 2: Unlock expansion_preview metrics.
Run baseline defaults + top param combos with evaluate_expansion_preview=true.
Each run isolated state_dir."""

import json, subprocess, time, pathlib, itertools
from concurrent.futures import ProcessPoolExecutor, as_completed
from path_defaults import resolve_binary_path, resolve_cleaned_data_root, resolve_repo_root

REPO = resolve_repo_root(__file__)
PHASE_DIR = REPO / 'state_expansion_preview'
PHASE_DIR.mkdir(parents=True, exist_ok=True)
RESULTS_FILE = PHASE_DIR / 'results.json'

DATA_BASE = str(resolve_cleaned_data_root(__file__))
BIN = str(resolve_binary_path(__file__))

# Default structure_ict params (from factor_definition.rs)
DEFAULTS = {
    'lookback': 20.0,
    'expansion_threshold': 1.5,
    'sweep_atr_multiplier': 0.45,
    'sweep_return_bars': 6.0,
    'sweep_recency_bars': 4.0,
    'sweep_weight': 0.18,
    'unconfirmed_sweep_weight': 0.04,
    'opposing_sweep_penalty': 0.10,
    'post_sweep_displacement_weight': 0.12,
}

# Previous "best" from v2c/v2d (highest composite in isolated runs)
BEST_V2D = {
    'lookback': 20.0,
    'expansion_threshold': 1.5,
    'sweep_atr_multiplier': 1.05,
    'sweep_weight': 1.3,
    'unconfirmed_sweep_weight': 0.55,
    'opposing_sweep_penalty': 1.2,
    'post_sweep_displacement_weight': 1.25,
    'sweep_recency_bars': 8.0,
    'sweep_return_bars': 7.0,
}

# Specs: baseline defaults, best v2d, and a grid of expansion-sensitive params
specs = [
    (1, {**DEFAULTS}, 'baseline defaults (expansion_preview=true)'),
    (2, {**BEST_V2D}, 'best v2d params (expansion_preview=true)'),
    # Vary lookback with defaults (expansion_preview=true)
    (3, {**DEFAULTS, 'lookback': 15.0}, 'defaults + lookback=15'),
    (4, {**DEFAULTS, 'lookback': 25.0}, 'defaults + lookback=25'),
    (5, {**DEFAULTS, 'lookback': 30.0}, 'defaults + lookback=30'),
    # Vary expansion_threshold with defaults
    (6, {**DEFAULTS, 'expansion_threshold': 1.3}, 'defaults + expansion=1.3'),
    (7, {**DEFAULTS, 'expansion_threshold': 1.7}, 'defaults + expansion=1.7'),
    (8, {**DEFAULTS, 'expansion_threshold': 2.0}, 'defaults + expansion=2.0'),
    # Combined: best lookback + expansion combos
    (9, {**DEFAULTS, 'lookback': 25.0, 'expansion_threshold': 1.3}, 'lb=25 + exp=1.3'),
    (10, {**DEFAULTS, 'lookback': 25.0, 'expansion_threshold': 1.7}, 'lb=25 + exp=1.7'),
    (11, {**DEFAULTS, 'lookback': 30.0, 'expansion_threshold': 1.5}, 'lb=30 + exp=1.5'),
    (12, {**DEFAULTS, 'lookback': 15.0, 'expansion_threshold': 1.3}, 'lb=15 + exp=1.3'),
]

TOTAL = len(specs)
print(f'Total specs: {TOTAL} (all with expansion_preview=true, isolated state_dir)')


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
    run_state = PHASE_DIR / f'state_run_{i:03d}'
    run_state.mkdir(parents=True, exist_ok=True)

    spec = {
        'mutation_id': f'expansion-preview-{i:03d}',
        'base_factor': 'structure_ict',
        'hypothesis': f'Expansion preview enabled: {desc}',
        'parameter_overrides': overrides,
        'direction_hints': {},
        'step_size_hints': {},
        'enabled_overrides': {},
        'evaluate_expansion_preview': True,  # THE KEY CHANGE
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
        'score_before': None, 'score_after': None,
        'composite_before': None, 'composite_after': None,
        'expansion_ba_before': None, 'expansion_ba_after': None,
        'expansion_da_before': None, 'expansion_da_after': None,
        'expansion_wp_before': None, 'expansion_wp_after': None,
        'bridge_gap_before': None, 'bridge_gap_after': None,
    }
    if payload and payload.get('factor_mutation_evaluation'):
        ev = payload['factor_mutation_evaluation']
        row['score_delta'] = ev.get('score_delta')
        row['accepted'] = ev.get('accepted')
        row['reason'] = ev.get('reason')
        row['score_before'] = ev.get('score_before')
        row['score_after'] = ev.get('score_after')
        mb = ev.get('metrics_before', {})
        ma = ev.get('metrics_after', {})
        row['composite_before'] = mb.get('best_factor_composite_score')
        row['composite_after'] = ma.get('best_factor_composite_score')
        row['expansion_ba_before'] = mb.get('expansion_balanced_accuracy')
        row['expansion_ba_after'] = ma.get('expansion_balanced_accuracy')
        row['expansion_da_before'] = mb.get('expansion_directional_accuracy')
        row['expansion_da_after'] = ma.get('expansion_directional_accuracy')
        row['expansion_wp_before'] = mb.get('expansion_selected_win_probability')
        row['expansion_wp_after'] = ma.get('expansion_selected_win_probability')
        row['bridge_gap_before'] = mb.get('pre_bayes_bridge_probability_gap')
        row['bridge_gap_after'] = ma.get('pre_bayes_bridge_probability_gap')
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
                sa = row.get('score_after')
                sa_str = f'{sa:.4f}' if sa is not None else '?'
                ba = row.get('expansion_ba_after')
                ba_str = f'{ba:.3f}' if ba is not None else 'null'
                print(f'[{len(results) - len(existing)}/{len(todo)}] run {i:03d}: score_after={sa_str} delta={d_str} exp_ba={ba_str} | {row["desc"]}')
            except Exception as e:
                print(f'run {i:03d}: EXCEPTION {e}')

    results = sorted(results, key=lambda r: r['i'])
    RESULTS_FILE.write_text(json.dumps(results, indent=2))

    print(f'\n{"=" * 90}')
    print(f'Phase 2: Expansion Preview Results')
    print(f'{"=" * 90}')
    print(f'\n{"run":>4} | {"score_after":>11} | {"delta":>8} | {"composite":>10} | {"exp_ba":>7} | {"exp_da":>7} | {"exp_wp":>7} | {"bridge":>7} | desc')
    print('-' * 100)
    for r in sorted(results, key=lambda x: x.get('score_after') or 0, reverse=True):
        sa = r.get('score_after')
        d = r.get('score_delta')
        c = r.get('composite_after')
        ba = r.get('expansion_ba_after')
        da = r.get('expansion_da_after')
        wp = r.get('expansion_wp_after')
        bg = r.get('bridge_gap_after')
        sa_s = f'{sa:.4f}' if sa is not None else 'N/A'
        d_s = f'{d:+.4f}' if d is not None else 'N/A'
        c_s = f'{c:.4f}' if c is not None else 'N/A'
        ba_s = f'{ba:.3f}' if ba is not None else 'null'
        da_s = f'{da:.3f}' if da is not None else 'null'
        wp_s = f'{wp:.3f}' if wp is not None else 'null'
        bg_s = f'{bg:.3f}' if bg is not None else 'null'
        print(f'{r["i"]:4d} | {sa_s:>11} | {d_s:>8} | {c_s:>10} | {ba_s:>7} | {da_s:>7} | {wp_s:>7} | {bg_s:>7} | {r["desc"]}')

    # Compare with previous best (no expansion_preview)
    print(f'\nPrevious best (no expansion_preview): score ≈ 0.3678, composite=0.4806')
    if results:
        best = max(results, key=lambda r: r.get('score_after') or 0)
        print(f'New best (with expansion_preview): score={best.get("score_after"):.4f}, composite={best.get("composite_after"):.4f}')
        print(f'  expansion_ba={best.get("expansion_ba_after")}, expansion_da={best.get("expansion_da_after")}')
        print(f'  expansion_wp={best.get("expansion_wp_after")}, bridge_gap={best.get("bridge_gap_after")}')
        old_score = 0.3678
        new_score = best.get('score_after', 0)
        print(f'  Score improvement: {old_score:.4f} → {new_score:.4f} ({new_score - old_score:+.4f})')
