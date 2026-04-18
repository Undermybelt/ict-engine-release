#!/usr/bin/env python3
"""Cross-market SMT focused experiment.

Rationale:
- structure_ict parameter sweeps are exhausted; baseline defaults are best.
- policy tuning configs tied.
- cross_market_smt was the only route that produced keep in autoresearch.

Goal:
- Explore cross_market_smt lookback / paired-data sensitivity in isolated state dirs.
- Measure score_after, score_delta, gate, bridge_gap, and top_factor_names.
"""

import json
import pathlib
import subprocess
import time
from concurrent.futures import ProcessPoolExecutor, as_completed

REPO = pathlib.Path('/Users/thrill3r/projects-ict-engine/ict-engine')
PHASE_DIR = REPO / 'state_cross_market_smt_focus'
PHASE_DIR.mkdir(parents=True, exist_ok=True)
RESULTS_FILE = PHASE_DIR / 'results.json'

DATA_BASE = '/Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf'
BIN = str(REPO / 'target' / 'release' / 'ict-engine')

DATA = f'{DATA_BASE}/cleaned-15m/nq.continuous-15m.json'
PAIRED_CANDIDATES = {
    'none': None,
    'es_15m': f'{DATA_BASE}/cleaned-15m/es.continuous-15m.json',
    'ym_15m': f'{DATA_BASE}/cleaned-15m/ym.continuous-15m.json',
}

LOOKBACKS = [8.0, 12.0, 16.0, 20.0, 24.0, 30.0, 40.0, 60.0]

specs = []
i = 1
for paired_name, paired_path in PAIRED_CANDIDATES.items():
    for lookback in LOOKBACKS:
        specs.append((i, paired_name, paired_path, lookback))
        i += 1

print(f'Total specs: {len(specs)} = {len(PAIRED_CANDIDATES)} paired modes × {len(LOOKBACKS)} lookbacks')


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
    i, paired_name, paired_path, lookback = args
    run_state = PHASE_DIR / f'state_run_{i:03d}_{paired_name}_lb{int(lookback)}'
    run_state.mkdir(parents=True, exist_ok=True)

    spec = {
        'mutation_id': f'smt-focus-{i:03d}',
        'base_factor': 'cross_market_smt',
        'hypothesis': f'Cross-market SMT focus: paired={paired_name}, lookback={lookback}',
        'parameter_overrides': {
            'lookback': lookback,
        },
        'direction_hints': {
            'cluster_jump': 'smt_cluster',
            'paired_market': paired_name,
        },
        'step_size_hints': {},
        'enabled_overrides': {},
        'evaluate_expansion_preview': True,
    }
    spec_path = run_state / 'spec.json'
    spec_path.write_text(json.dumps(spec, indent=2))

    cmd = [
        BIN, 'factor-research', '--symbol', 'NQ',
        '--data', DATA,
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
    if paired_path:
        cmd += ['--paired-data', paired_path]

    t0 = time.time()
    proc = subprocess.run(cmd, cwd=REPO, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    dur = round(time.time() - t0, 2)
    (run_state / 'output.log').write_text(proc.stdout)

    payload = extract_json(proc.stdout)
    row = {
        'i': i,
        'paired': paired_name,
        'lookback': lookback,
        'paired_path': paired_path,
        'exit_code': proc.returncode,
        'duration_sec': dur,
        'score_delta': None,
        'accepted': None,
        'score_before': None,
        'score_after': None,
        'reason': None,
        'top_factor': payload.get('top_factor') if payload else None,
        'top_factor_names': None,
        'gate_before': None,
        'gate_after': None,
        'bridge_before': None,
        'bridge_after': None,
        'win_probability_after': None,
        'composite_after': None,
    }

    if payload and payload.get('factor_mutation_evaluation'):
        ev = payload['factor_mutation_evaluation']
        row['score_delta'] = ev.get('score_delta')
        row['accepted'] = ev.get('accepted')
        row['score_before'] = ev.get('score_before')
        row['score_after'] = ev.get('score_after')
        row['reason'] = ev.get('reason')
        before = ev.get('metrics_before', {}) or {}
        after = ev.get('metrics_after', {}) or {}
        row['top_factor_names'] = after.get('top_factor_names')
        row['gate_before'] = before.get('pre_bayes_gate_status')
        row['gate_after'] = after.get('pre_bayes_gate_status')
        row['bridge_before'] = before.get('pre_bayes_bridge_probability_gap')
        row['bridge_after'] = after.get('pre_bayes_bridge_probability_gap')
        row['win_probability_after'] = after.get('expansion_selected_win_probability')
        row['composite_after'] = after.get('best_factor_composite_score')
    return row


if __name__ == '__main__':
    existing = json.loads(RESULTS_FILE.read_text()) if RESULTS_FILE.exists() else []
    done_ids = {r['i'] for r in existing}
    todo = [spec for spec in specs if spec[0] not in done_ids]
    print(f'Done: {len(done_ids)}, remaining: {len(todo)}')

    results = list(existing)
    with ProcessPoolExecutor(max_workers=4) as pool:
        futures = {pool.submit(run_one, spec): spec[0] for spec in todo}
        for fut in as_completed(futures):
            i = futures[fut]
            try:
                row = fut.result()
                results.append(row)
                results_sorted = sorted(results, key=lambda r: r['i'])
                RESULTS_FILE.write_text(json.dumps(results_sorted, indent=2))
                d = row['score_delta']
                d_s = f'{d:+.4f}' if d is not None else 'N/A'
                s = row['score_after']
                s_s = f'{s:.4f}' if s is not None else 'N/A'
                bridge = row['bridge_after']
                b_s = f'{bridge:.4f}' if bridge is not None else 'null'
                print(f'[{len(results)-len(existing)}/{len(todo)}] run {i:03d}: paired={row["paired"]} lb={row["lookback"]} score={s_s} delta={d_s} gate={row["gate_after"]} bridge={b_s} accepted={row["accepted"]}')
            except Exception as e:
                print(f'run {i:03d}: EXCEPTION {e}')

    results = sorted(results, key=lambda r: r['i'])
    RESULTS_FILE.write_text(json.dumps(results, indent=2))

    print(f'\n{"=" * 100}')
    print('Cross-Market SMT Focus Results')
    print(f'{"=" * 100}')
    print(f'{"run":>4} | {"paired":>8} | {"lb":>5} | {"score":>7} | {"delta":>8} | {"gate":>15} | {"bridge":>7} | {"accepted":>8}')
    print('-' * 100)
    for r in sorted(results, key=lambda x: x.get('score_after') or 0, reverse=True):
        s = r.get('score_after')
        d = r.get('score_delta')
        b = r.get('bridge_after')
        s_s = f'{s:.4f}' if s is not None else 'N/A'
        d_s = f'{d:+.4f}' if d is not None else 'N/A'
        b_s = f'{b:.4f}' if b is not None else 'null'
        print(f'{r["i"]:4d} | {r["paired"]:>8} | {r["lookback"]:>5.0f} | {s_s:>7} | {d_s:>8} | {str(r.get("gate_after")):>15} | {b_s:>7} | {str(r.get("accepted")):>8}')

    valid = [r for r in results if r.get('score_after') is not None]
    if valid:
        best = max(valid, key=lambda r: r['score_after'])
        print(f'\nBEST: run {best["i"]:03d} paired={best["paired"]} lookback={best["lookback"]} score={best["score_after"]:.4f} delta={best["score_delta"]:+.4f}')
        print(f'  top_factor_names={best.get("top_factor_names")}')
        print(f'  gate_after={best.get("gate_after")} bridge_after={best.get("bridge_after")}')
