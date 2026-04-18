#!/usr/bin/env python3
"""Phase 3: Collect best specs from Phase 1 (local search) and Phase 2 (cluster jumps).
Print comparison table and recommend next action."""

import json, pathlib

REPO = pathlib.Path('/Users/thrill3r/projects-ict-engine/ict-engine')


def load_best_from_local_search():
    results_file = REPO / 'state_local_search_v2' / 'results.json'
    if not results_file.exists():
        return None
    results = json.loads(results_file.read_text())
    valid = [r for r in results if r.get('score_delta') is not None]
    if not valid:
        return None
    best = max(valid, key=lambda r: r['score_delta'])
    return {
        'source': 'local_search',
        'run': best['i'],
        'delta': best['score_delta'],
        'params': best.get('params'),
        'accepted': best.get('accepted'),
    }


def load_best_from_cluster(cluster_name, state_dir_name):
    attempts_file = REPO / state_dir_name / 'NQ' / 'factor_autoresearch_attempts.json'
    if not attempts_file.exists():
        return None
    attempts = json.loads(attempts_file.read_text())
    if not attempts:
        return None
    best = max(attempts, key=lambda a: a['decision'].get('candidate_score', 0))
    return {
        'source': cluster_name,
        'delta': best['evaluation'].get('score_delta', 0),
        'score': best['decision'].get('candidate_score', 0),
        'promoted': best['decision'].get('promoted_to_baseline', False),
        'attempt_id': best['attempt_id'],
    }


if __name__ == '__main__':
    candidates = []

    local = load_best_from_local_search()
    if local:
        candidates.append(local)
        print(f"Local search best: run {local['run']:03d}, delta={local['delta']:.4f}, "
              f"params={local['params']}, accepted={local['accepted']}")
    else:
        print("Local search: no results")

    for cluster, state_dir in [
        ('displacement_fvg', 'state_cluster_dfvg'),
        ('mss_bos', 'state_cluster_mss'),
        ('premium_discount_ote', 'state_cluster_pdote'),
        ('smt', 'state_cluster_smt'),
    ]:
        result = load_best_from_cluster(cluster, state_dir)
        if result:
            candidates.append(result)
            print(f"{cluster}: delta={result['delta']:.4f}, score={result['score']:.4f}, "
                  f"promoted={result['promoted']}")
        else:
            print(f"{cluster}: no results")

    if candidates:
        best = max(candidates, key=lambda c: c.get('delta', -999))
        print(f"\n{'=' * 50}")
        print(f"WINNER: {best['source']} with delta={best.get('delta', 'N/A')}")
        print(f"{'=' * 50}")

        # Recommendation
        if best.get('delta', -999) > 0:
            print("\n→ Positive delta found! Promote this spec to baseline.")
        elif best.get('delta', -999) > -0.005:
            print("\n→ Near-neutral delta. Consider expanding search radius or trying different objective.")
        else:
            print("\n→ All deltas negative. Current baseline may already be near global optimum for this objective.")
            print("  Consider: different objective, different data window, or structural model changes.")
    else:
        print("\nNo results found. Check logs in state_local_search_v2/ and state_cluster_*/.")
