#!/usr/bin/env python3
"""Phase 2: Run 4 cluster autoresearch sessions in parallel.
Each cluster gets its own state_dir to avoid cross-contamination."""

import json
import pathlib
import subprocess
import time
from concurrent.futures import ProcessPoolExecutor, as_completed

REPO = pathlib.Path(__file__).resolve().parents[2]
DEFAULT_DATA_ROOT = REPO.parent.parent / 'Downloads' / 'Tomac' / 'ict-cleaned-mtf'
DATA_BASE = pathlib.Path(
    subprocess.os.environ.get('ICT_ENGINE_DATA_ROOT', DEFAULT_DATA_ROOT)
).expanduser().resolve()
DEFAULT_BIN = REPO / 'target' / 'release' / 'ict-engine'
BIN = pathlib.Path(subprocess.os.environ.get('ICT_ENGINE_BIN', DEFAULT_BIN)).expanduser().resolve()

RUN5_BASE = {
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

CLUSTERS = [
    {
        'name': 'displacement_fvg',
        'state_dir': 'state_cluster_dfvg',
        'base_factor': 'structure_ict',
        'cluster_jump': 'displacement_fvg_cluster',
        'cycle': '0',
        'overrides': {
            'post_sweep_displacement_weight': 1.35,
            'sweep_weight': 1.10,
            'unconfirmed_sweep_weight': 0.45,
            'expansion_threshold': 1.05,
        },
    },
    {
        'name': 'mss_bos',
        'state_dir': 'state_cluster_mss',
        'base_factor': 'structure_ict',
        'cluster_jump': 'mss_bos_cluster',
        'cycle': '1',
        'overrides': {
            'lookback': 10.0,
            'expansion_threshold': 1.18,
            'sweep_return_bars': 5.0,
            'opposing_sweep_penalty': 1.25,
        },
    },
    {
        'name': 'premium_discount_ote',
        'state_dir': 'state_cluster_pdote',
        'base_factor': 'structure_ict',
        'cluster_jump': 'premium_discount_ote_cluster',
        'cycle': '2',
        'overrides': {
            'lookback': 14.0,
            'expansion_threshold': 0.92,
            'sweep_recency_bars': 8.0,
            'sweep_return_bars': 6.0,
        },
    },
    {
        'name': 'smt',
        'state_dir': 'state_cluster_smt',
        'base_factor': 'cross_market_smt',
        'cluster_jump': 'smt_cluster',
        'cycle': '3',
        'overrides': {
            'lookback': 24.0,
            'sweep_atr_multiplier': 0.60,
            'sweep_weight': 0.72,
            'opposing_sweep_penalty': 1.05,
        },
    },
]


def run_cluster(cluster):
    state_dir = str(REPO / cluster['state_dir'])
    pathlib.Path(state_dir).mkdir(parents=True, exist_ok=True)

    params = {**RUN5_BASE, **cluster['overrides']}
    spec = {
        'mutation_id': f'cluster-{cluster["name"]}-001',
        'base_factor': cluster['base_factor'],
        'hypothesis': f'Cluster jump: {cluster["cluster_jump"]} with {cluster["name"]} focus',
        'parameter_overrides': params,
        'direction_hints': {
            'cluster_jump': cluster['cluster_jump'],
            'cluster_jump_cycle': cluster['cycle'],
            'available_clusters': 'displacement_fvg_cluster|mss_bos_cluster|premium_discount_ote_cluster|smt_cluster',
        },
        'step_size_hints': {},
        'enabled_overrides': {},
        'evaluate_expansion_preview': False,
    }

    spec_path = pathlib.Path(state_dir) / 'seed_spec.json'
    spec_path.write_text(json.dumps(spec, indent=2))

    cmd = [
        str(BIN), 'factor-autoresearch', '--symbol', 'NQ',
        '--data', str(DATA_BASE / 'cleaned-15m' / 'nq.continuous-15m.json'),
        '--data-1m', str(DATA_BASE / 'cleaned-1m' / 'nq.continuous-1m.json'),
        '--data-5m', str(DATA_BASE / 'cleaned-5m' / 'nq.continuous-5m.json'),
        '--data-15m', str(DATA_BASE / 'cleaned-15m' / 'nq.continuous-15m.json'),
        '--data-1h', str(DATA_BASE / 'cleaned-1h' / 'nq.continuous-1h.json'),
        '--data-4h', str(DATA_BASE / 'cleaned-4h' / 'nq.continuous-4h.json'),
        '--data-1d', str(DATA_BASE / 'cleaned-1d' / 'nq.continuous-1d.json'),
        '--objective', 'expansion_manipulation',
        '--mutation-spec', str(spec_path),
        '--iterations', '3',
        '--max-cluster-fail-streak', '2',
        '--ensemble',
        '--state-dir', state_dir,
    ]

    t0 = time.time()
    proc = subprocess.run(cmd, cwd=REPO, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    dur = round(time.time() - t0, 2)

    log_path = pathlib.Path(state_dir) / 'autoresearch.log'
    log_path.write_text(proc.stdout)

    return {
        'cluster': cluster['name'],
        'exit_code': proc.returncode,
        'duration_sec': dur,
        'state_dir': state_dir,
    }


if __name__ == '__main__':
    print(f'Running {len(CLUSTERS)} cluster autoresearch sessions in parallel...')
    print(f'Repo: {REPO}')
    print(f'Data root: {DATA_BASE}')
    print(f'Binary: {BIN}')
    results = []
    with ProcessPoolExecutor(max_workers=4) as pool:
        futures = {pool.submit(run_cluster, c): c['name'] for c in CLUSTERS}
        for fut in as_completed(futures):
            name = futures[fut]
            try:
                result = fut.result()
                results.append(result)
                print(f'[{len(results)}/{len(CLUSTERS)}] {name}: exit={result["exit_code"]} dur={result["duration_sec"]}s')
            except Exception as e:
                print(f'{name}: EXCEPTION {e}')

    print('\n=== Phase 2 Complete ===')
    for r in sorted(results, key=lambda x: x['cluster']):
        status = 'OK' if r['exit_code'] == 0 else f'FAIL(exit={r["exit_code"]})'
        print(f'  {r["cluster"]}: {status} ({r["duration_sec"]}s)')
