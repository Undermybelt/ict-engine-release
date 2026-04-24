#!/usr/bin/env python3
import argparse
import json
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path


def utc_now():
    return datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z')


def main():
    ap = argparse.ArgumentParser(description='Summarize ICT-vs-native bucket winners for BTC ledger research')
    ap.add_argument('--research-runs', required=True)
    ap.add_argument('--bucket-objectives', required=True)
    ap.add_argument('--output', required=True)
    args = ap.parse_args()

    runs = json.loads(Path(args.research_runs).read_text(encoding='utf-8'))
    objectives = json.loads(Path(args.bucket_objectives).read_text(encoding='utf-8'))
    objective_map = {item['run_id']: item for item in objectives}

    winner_counts = Counter()
    verdict_counts = Counter()
    mismatch_buckets = []
    largest_gaps = []
    ict_wins = []
    native_wins = []

    for run in runs:
        run_id = run['run_id']
        obj = objective_map.get(run_id, {})
        surfaces = obj.get('objective_surfaces', [])
        if len(surfaces) < 2:
            continue
        surface_scores = {s['factor_name']: float(s['objective_score']) for s in surfaces}
        ict = surface_scores.get('btc_ledger_ict_interpretability', 0.0)
        native = surface_scores.get('btc_ledger_execution_native', 0.0)
        gap = abs(ict - native)
        winner = 'ict' if ict >= native else 'native'
        winner_counts[winner] += 1
        verdict = obj.get('summary', {}).get('interpretation_verdict', 'unknown')
        verdict_counts[verdict] += 1
        item = {
            'run_id': run_id,
            'bucket_start': obj.get('bucket_start'),
            'bucket_end': obj.get('bucket_end'),
            'ict_score': ict,
            'native_score': native,
            'score_gap': gap,
            'winner': winner,
            'best_factor': run.get('best_factor'),
            'aggregate_return': run.get('aggregate_return'),
            'execution_aggression_bias': obj.get('summary', {}).get('execution_aggression_bias'),
            'fill_completion_pressure': obj.get('summary', {}).get('fill_completion_pressure'),
            'top_symbol_share': obj.get('summary', {}).get('top_symbol_share'),
            'interpretation_verdict': verdict,
        }
        largest_gaps.append(item)
        if winner == 'ict':
            ict_wins.append(item)
        else:
            native_wins.append(item)
        if winner == 'native':
            mismatch_buckets.append(item)

    largest_gaps.sort(key=lambda x: x['score_gap'], reverse=True)
    ict_wins.sort(key=lambda x: x['score_gap'], reverse=True)
    native_wins.sort(key=lambda x: x['score_gap'], reverse=True)
    mismatch_buckets.sort(key=lambda x: x['score_gap'], reverse=True)

    out = {
        'generated_at': utc_now(),
        'research_runs_path': args.research_runs,
        'bucket_objectives_path': args.bucket_objectives,
        'totals': {
            'bucket_count': len(largest_gaps),
            'ict_wins': winner_counts.get('ict', 0),
            'native_wins': winner_counts.get('native', 0),
        },
        'verdict_counts': dict(verdict_counts),
        'largest_disagreements': largest_gaps[:12],
        'native_mismatch_buckets': mismatch_buckets[:24],
        'ict_winner_buckets_top': ict_wins[:12],
        'native_winner_buckets_top': native_wins[:12],
        'final_verdict': (
            'ict-dominant with bounded native fallback'
            if winner_counts.get('ict', 0) > winner_counts.get('native', 0)
            else 'native-dominant; ICT is secondary annotation'
        ),
    }

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(out, indent=2), encoding='utf-8')
    print(str(output))


if __name__ == '__main__':
    main()
