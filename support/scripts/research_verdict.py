#!/usr/bin/env python3
"""Research verdict synthesizer.

Reads one or more state/result dirs and emits a compact verdict surface.
"""

import argparse
import json
import pathlib
from collections import Counter, defaultdict


def load_json(path):
    try:
        return json.loads(path.read_text())
    except Exception:
        return None


def collect_attempts(root: pathlib.Path):
    attempts = []
    for p in root.rglob('factor_autoresearch_attempts.json'):
        data = load_json(p)
        if isinstance(data, list):
            attempts.extend(data)
    return attempts


def collect_finals(root: pathlib.Path):
    finals = []
    for p in root.rglob('factor_autoresearch_final.json'):
        data = load_json(p)
        if isinstance(data, dict):
            finals.append((p, data))
    return finals


def collect_result_tables(root: pathlib.Path):
    tables = []
    for p in root.rglob('results.json'):
        data = load_json(p)
        if isinstance(data, list):
            tables.append((p, data))
    return tables


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('paths', nargs='+', help='state dirs or result dirs')
    args = ap.parse_args()

    roots = [pathlib.Path(p).expanduser().resolve() for p in args.paths]
    attempts = []
    finals = []
    tables = []
    for root in roots:
        attempts.extend(collect_attempts(root))
        finals.extend(collect_finals(root))
        tables.extend(collect_result_tables(root))

    best_attempt = None
    best_delta = float('-inf')
    decision_counts = Counter()
    failure_counts = Counter()
    clusters = Counter()
    bad_regions = []

    for a in attempts:
        dec = a.get('decision', {})
        ev = a.get('evaluation', {})
        decision_counts.update([dec.get('status', 'unknown')])
        failure_counts.update(ev.get('failure_tags', []))
        cluster = a.get('candidate_mutation_spec', {}).get('direction_hints', {}).get('cluster_jump', 'none')
        clusters.update([cluster])
        delta = dec.get('score_delta')
        if isinstance(delta, (int, float)) and delta > best_delta:
            best_delta = delta
            best_attempt = a
        if dec.get('status') == 'discard' and ev.get('failure_tags'):
            bad_regions.append({
                'cluster': cluster,
                'base_factor': a.get('candidate_mutation_spec', {}).get('base_factor'),
                'failure_tags': ev.get('failure_tags', []),
            })

    contaminated = False
    for path, table in tables:
        deltas = [r.get('score_delta') for r in table if isinstance(r.get('score_delta'), (int, float))]
        if len(deltas) >= 4:
            if all(x <= y for x, y in zip(deltas, deltas[1:])) or all(x >= y for x, y in zip(deltas, deltas[1:])):
                contaminated = True

    bottleneck = 'unknown'
    next_experiment = 'inspect factor-pipeline-debug'
    stop_or_continue = 'continue'
    if best_attempt:
        tags = best_attempt.get('evaluation', {}).get('failure_tags', [])
        if 'bridge_gap_too_small' in tags:
            bottleneck = 'bridge_gap'
            next_experiment = 'run evidence_quality_breakdown and bridge-gap focused experiment'
        elif 'pre_bayes_gate_regressed' in tags:
            bottleneck = 'pre_bayes_gate'
            next_experiment = 'run evidence_quality_breakdown'
        elif 'best_factor_composite_regressed' in tags:
            bottleneck = 'composite_score'
            next_experiment = 'stop local parameter sweep and pivot to structural change'
        else:
            bottleneck = 'evidence_quality_or_structure'
            next_experiment = 'inspect evidence-quality-breakdown'
        if best_delta <= 0:
            stop_or_continue = 'stop_as_local_optimum' if decision_counts.get('keep', 0) == 0 else 'pivot'

    best_known_baseline = None
    if finals:
        p, data = finals[-1]
        best_known_baseline = {
            'path': str(p),
            'session_id': data.get('session', {}).get('session_id'),
            'base_factor': data.get('session', {}).get('base_factor'),
            'baseline_mutation_id': data.get('session', {}).get('baseline_mutation_id'),
            'baseline_score': data.get('session', {}).get('baseline_score'),
        }

    out = {
        'best_known_baseline': best_known_baseline,
        'proven_bad_regions': bad_regions[:12],
        'current_bottleneck': bottleneck,
        'recommended_next_experiment': next_experiment,
        'stop_or_continue': stop_or_continue,
        'comparison_contaminated': contaminated,
        'decision_counts': dict(decision_counts),
        'top_failure_tags': failure_counts.most_common(8),
        'cluster_counts': dict(clusters),
        'best_attempt': best_attempt,
    }
    print(json.dumps(out, indent=2))


if __name__ == '__main__':
    main()
