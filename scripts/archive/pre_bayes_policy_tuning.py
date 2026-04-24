#!/usr/bin/env python3
"""Path 1: PreBayes policy tuning.
Each run uses baseline structure_ict defaults with expansion_preview=true,
modifying only environment variables that control evidence_quality_score."""

import itertools
import json
import os
import pathlib
import subprocess
import time
from concurrent.futures import ProcessPoolExecutor, as_completed
from path_defaults import resolve_binary_path, resolve_cleaned_data_root, resolve_repo_root

REPO = resolve_repo_root(__file__)
PHASE_DIR = REPO / 'state_policy_tuning'
PHASE_DIR.mkdir(parents=True, exist_ok=True)
RESULTS_FILE = PHASE_DIR / 'results.json'
DATA_BASE = resolve_cleaned_data_root(__file__)
BIN = resolve_binary_path(__file__)

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

CONFIGS = [
    {'name': 'baseline', 'env': {}},
    {'name': 'hardpass_070', 'env': {'ICT_ENGINE_PREBAYES_HARD_PASS_QUALITY_THRESHOLD': '0.70'}},
    {'name': 'hardpass_065', 'env': {'ICT_ENGINE_PREBAYES_HARD_PASS_QUALITY_THRESHOLD': '0.65'}},
    {'name': 'hardpass_060', 'env': {'ICT_ENGINE_PREBAYES_HARD_PASS_QUALITY_THRESHOLD': '0.60'}},
    {'name': 'conflict_soft', 'env': {
        'ICT_ENGINE_PREBAYES_DIRECTIONAL_CONFLICT_PENALTY': '0.10',
        'ICT_ENGINE_PREBAYES_MIXED_ALIGNMENT_PENALTY': '0.05',
    }},
    {'name': 'mtf_soft', 'env': {
        'ICT_ENGINE_PREBAYES_MTF_DIRECTION_CONFLICT_PENALTY': '0.08',
        'ICT_ENGINE_PREBAYES_MTF_ALIGNMENT_PENALTY': '0.05',
        'ICT_ENGINE_PREBAYES_MTF_ALIGNMENT_BONUS': '0.10',
    }},
    {'name': 'liquidity_soft', 'env': {
        'ICT_ENGINE_PREBAYES_HOSTILE_LIQUIDITY_PENALTY': '0.05',
    }},
    {'name': 'combined_070', 'env': {
        'ICT_ENGINE_PREBAYES_HARD_PASS_QUALITY_THRESHOLD': '0.70',
        'ICT_ENGINE_PREBAYES_DIRECTIONAL_CONFLICT_PENALTY': '0.10',
        'ICT_ENGINE_PREBAYES_MIXED_ALIGNMENT_PENALTY': '0.05',
        'ICT_ENGINE_PREBAYES_MTF_DIRECTION_CONFLICT_PENALTY': '0.08',
        'ICT_ENGINE_PREBAYES_MTF_ALIGNMENT_PENALTY': '0.05',
        'ICT_ENGINE_PREBAYES_MTF_ALIGNMENT_BONUS': '0.10',
        'ICT_ENGINE_PREBAYES_HOSTILE_LIQUIDITY_PENALTY': '0.05',
    }},
    {'name': 'combined_065', 'env': {
        'ICT_ENGINE_PREBAYES_HARD_PASS_QUALITY_THRESHOLD': '0.65',
        'ICT_ENGINE_PREBAYES_DIRECTIONAL_CONFLICT_PENALTY': '0.10',
        'ICT_ENGINE_PREBAYES_MIXED_ALIGNMENT_PENALTY': '0.05',
        'ICT_ENGINE_PREBAYES_MTF_DIRECTION_CONFLICT_PENALTY': '0.08',
        'ICT_ENGINE_PREBAYES_MTF_ALIGNMENT_PENALTY': '0.05',
        'ICT_ENGINE_PREBAYES_MTF_ALIGNMENT_BONUS': '0.10',
        'ICT_ENGINE_PREBAYES_HOSTILE_LIQUIDITY_PENALTY': '0.05',
    }},
    {'name': 'combined_060', 'env': {
        'ICT_ENGINE_PREBAYES_HARD_PASS_QUALITY_THRESHOLD': '0.60',
        'ICT_ENGINE_PREBAYES_DIRECTIONAL_CONFLICT_PENALTY': '0.10',
        'ICT_ENGINE_PREBAYES_MIXED_ALIGNMENT_PENALTY': '0.05',
        'ICT_ENGINE_PREBAYES_MTF_DIRECTION_CONFLICT_PENALTY': '0.08',
        'ICT_ENGINE_PREBAYES_MTF_ALIGNMENT_PENALTY': '0.05',
        'ICT_ENGINE_PREBAYES_MTF_ALIGNMENT_BONUS': '0.10',
        'ICT_ENGINE_PREBAYES_HOSTILE_LIQUIDITY_PENALTY': '0.05',
    }},
]


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
    i, cfg = args
    run_state = PHASE_DIR / f'state_run_{i:03d}_{cfg["name"]}'
    run_state.mkdir(parents=True, exist_ok=True)

    spec = {
        'mutation_id': f'policy-tune-{i:03d}',
        'base_factor': 'structure_ict',
        'hypothesis': f'Policy tuning: {cfg["name"]}',
        'parameter_overrides': DEFAULTS,
        'direction_hints': {},
        'step_size_hints': {},
        'enabled_overrides': {},
        'evaluate_expansion_preview': True,
    }
    spec_path = run_state / 'spec.json'
    spec_path.write_text(json.dumps(spec, indent=2))

    env = os.environ.copy()
    env.update(cfg['env'])

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
    proc = subprocess.run(cmd, cwd=REPO, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    dur = round(time.time() - t0, 2)
    (run_state / 'output.log').write_text(proc.stdout)

    payload = extract_json(proc.stdout)
    row = {
        'i': i, 'config': cfg['name'], 'env': cfg['env'],
        'exit_code': proc.returncode, 'duration_sec': dur,
        'score_delta': None, 'accepted': None,
        'score_before': None, 'score_after': None,
        'gate_before': None, 'gate_after': None,
        'bridge_gap_before': None, 'bridge_gap_after': None,
    }
    if payload and payload.get('factor_mutation_evaluation'):
        ev = payload['factor_mutation_evaluation']
        row['score_delta'] = ev.get('score_delta')
        row['accepted'] = ev.get('accepted')
        row['score_before'] = ev.get('score_before')
        row['score_after'] = ev.get('score_after')
        mb = ev.get('metrics_before', {})
        ma = ev.get('metrics_after', {})
        row['gate_before'] = mb.get('pre_bayes_gate_status')
        row['gate_after'] = ma.get('pre_bayes_gate_status')
        row['bridge_gap_before'] = mb.get('pre_bayes_bridge_probability_gap')
        row['bridge_gap_after'] = ma.get('pre_bayes_bridge_probability_gap')
    return row


if __name__ == '__main__':
    existing = json.loads(RESULTS_FILE.read_text()) if RESULTS_FILE.exists() else []
    done_ids = {r['i'] for r in existing}
    todo = [(i, cfg) for i, cfg in enumerate(CONFIGS, 1) if i not in done_ids]
    print(f'Total configs: {len(CONFIGS)}, done: {len(done_ids)}, remaining: {len(todo)}')
    print(f'Repo: {REPO}')
    print(f'Data root: {DATA_BASE}')
    print(f'Binary: {BIN}')

    results = list(existing)
    with ProcessPoolExecutor(max_workers=4) as pool:
        futures = {pool.submit(run_one, (i, cfg)): i for i, cfg in todo}
        for fut in as_completed(futures):
            i = futures[fut]
            try:
                row = fut.result()
                results.append(row)
                results_sorted = sorted(results, key=lambda r: r['i'])
                RESULTS_FILE.write_text(json.dumps(results_sorted, indent=2))
                d = row['score_delta']
                d_str = f'{d:+.4f}' if d is not None else 'N/A'
                score_after = row['score_after']
                score_after_str = f'{score_after:.4f}' if score_after is not None else 'N/A'
                print(f'[{len(results) - len(existing)}/{len(todo)}] {row["config"]}: '
                      f'score_after={score_after_str} delta={d_str} gate={row["gate_after"]} bridge={row["bridge_gap_after"]}')
            except Exception as e:
                print(f'config {i}: EXCEPTION {e}')

    results = sorted(results, key=lambda r: r['score_after'] or 0, reverse=True)
    RESULTS_FILE.write_text(json.dumps(results, indent=2))

    print(f'\n{"=" * 80}')
    print('Policy Tuning Results')
    print(f'{"=" * 80}')
    print(f'\n{"config":<15} {"score":>8} {"delta":>8} {"gate":>15} {"bridge":>8}')
    print('-' * 60)
    for r in results:
        d = r.get('score_delta')
        d_s = f'{d:+.4f}' if d is not None else 'N/A'
        s_s = f'{r["score_after"]:.4f}' if r.get('score_after') is not None else 'N/A'
        bg = r.get('bridge_gap_after')
        bg_s = f'{bg:.4f}' if bg is not None else 'null'
        print(f'{r["config"]:<15} {s_s:>8} {d_s:>8} {str(r.get("gate_after"))[:15]:>15} {bg_s:>8}')
