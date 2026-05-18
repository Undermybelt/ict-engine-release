#!/usr/bin/env python3
"""Minimal ablation for new physics features.

Compares four conditions on NQ:
1. baseline
2. + distance only
3. + OU only
4. + both

For now this is a reporting/diagnostics ablation, not a deep model retrain.
We reuse factor-pipeline-debug / factor-research surfaces and compare:
- evidence_quality_score
- pre_bayes_gate_status
- bridge_gap
- score_after

Implementation note:
Because the new physics features are currently consumed through the debug/human surface,
this script first measures their values, then simulates feature-intake conditions via env toggles
that will be used by future gate/scoring wiring.
"""

import json
import pathlib
import subprocess
import time
from concurrent.futures import ProcessPoolExecutor, as_completed
from path_defaults import resolve_binary_path, resolve_cleaned_data_root, resolve_repo_root

REPO = resolve_repo_root(__file__)
STATE = REPO / 'state_feature_ablation_v1'
STATE.mkdir(parents=True, exist_ok=True)
RESULTS_FILE = STATE / 'results.json'

DATA_BASE = str(resolve_cleaned_data_root(__file__))
BIN = str(resolve_binary_path(__file__))

RUNS = [
    {
        'id': 'baseline',
        'env': {},
        'label': 'baseline',
    },
    {
        'id': 'distance_only',
        'env': {
            'ICT_ENGINE_FEATURE_DISTANCE_ONLY': '1',
        },
        'label': 'distance_only',
    },
    {
        'id': 'ou_only',
        'env': {
            'ICT_ENGINE_FEATURE_OU_ONLY': '1',
        },
        'label': 'ou_only',
    },
    {
        'id': 'both',
        'env': {
            'ICT_ENGINE_FEATURE_DISTANCE_ONLY': '1',
            'ICT_ENGINE_FEATURE_OU_ONLY': '1',
        },
        'label': 'both',
    },
]


def extract_json(text: str):
    start = text.find('{')
    if start == -1:
        return None
    depth = 0
    for i, ch in enumerate(text[start:]):
        if ch == '{':
            depth += 1
        elif ch == '}':
            depth -= 1
        if depth == 0:
            try:
                return json.loads(text[start:start + i + 1])
            except Exception:
                return None
    return None


def run_one(item):
    run_id = item['id']
    env = dict(item['env'])
    work_state = STATE / run_id
    work_state.mkdir(parents=True, exist_ok=True)

    cmd = [
        BIN,
        'factor-pipeline-debug',
        '--symbol', 'NQ',
        '--data', f'{DATA_BASE}/cleaned-15m/nq.continuous-15m.json',
        '--factor', 'structure_ict',
        '--objective', 'expansion_manipulation',
        '--data-1m', f'{DATA_BASE}/cleaned-1m/nq.continuous-1m.json',
        '--data-5m', f'{DATA_BASE}/cleaned-5m/nq.continuous-5m.json',
        '--data-15m', f'{DATA_BASE}/cleaned-15m/nq.continuous-15m.json',
        '--data-1h', f'{DATA_BASE}/cleaned-1h/nq.continuous-1h.json',
        '--data-4h', f'{DATA_BASE}/cleaned-4h/nq.continuous-4h.json',
        '--data-1d', f'{DATA_BASE}/cleaned-1d/nq.continuous-1d.json',
    ]

    full_env = dict(**env)
    full_env.update({
        'PATH': str(pathlib.Path('/usr/bin').parent) + ':' + str(pathlib.Path('/opt/homebrew/bin')) + ':' + str(pathlib.Path('/usr/local/bin')) + ':' + str(pathlib.Path('/bin')) + ':' + str(pathlib.Path('/usr/sbin')) + ':' + str(pathlib.Path('/sbin')),
    })

    t0 = time.time()
    proc = subprocess.run(
        cmd,
        cwd=REPO,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        env={**full_env, **dict(pathlib=os.environ.get('PATH', ''))} if False else {**subprocess.os.environ, **env},
    )
    dur = round(time.time() - t0, 2)
    (work_state / 'output.log').write_text(proc.stdout)

    payload = extract_json(proc.stdout)
    row = {
        'id': run_id,
        'label': item['label'],
        'exit_code': proc.returncode,
        'duration_sec': dur,
        'evidence_quality_score': None,
        'pre_bayes_gate_status': None,
        'bridge_gap': None,
        'frame_physics_trace': None,
        'recommended_actions': None,
        'score_after': None,
    }
    if payload:
        row['evidence_quality_score'] = payload.get('evidence_quality_score')
        row['pre_bayes_gate_status'] = payload.get('gating_status')
        row['bridge_gap'] = payload.get('bridge_gap')
        row['frame_physics_trace'] = payload.get('frame_physics_trace')
        row['recommended_actions'] = payload.get('recommended_actions')
    return row


if __name__ == '__main__':
    results = []
    with ProcessPoolExecutor(max_workers=4) as pool:
        futures = {pool.submit(run_one, item): item['id'] for item in RUNS}
        for fut in as_completed(futures):
            row = fut.result()
            results.append(row)
            RESULTS_FILE.write_text(json.dumps(sorted(results, key=lambda r: r['id']), indent=2))
            print(f"{row['id']}: gate={row['pre_bayes_gate_status']} evidence={row['evidence_quality_score']} bridge={row['bridge_gap']}")

    results = sorted(results, key=lambda r: r['id'])
    RESULTS_FILE.write_text(json.dumps(results, indent=2))

    print('\n=== Ablation Summary ===')
    for row in results:
        print(json.dumps({
            'id': row['id'],
            'evidence_quality_score': row['evidence_quality_score'],
            'pre_bayes_gate_status': row['pre_bayes_gate_status'],
            'bridge_gap': row['bridge_gap'],
            'frame_physics_trace': row['frame_physics_trace'],
        }, indent=2))
