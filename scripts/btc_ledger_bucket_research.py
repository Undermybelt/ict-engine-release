#!/usr/bin/env python3
import argparse
import csv
import json
from collections import Counter, defaultdict
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from pathlib import Path
from statistics import mean

PROMPT_VERSION = 'btc-ledger-bucket-research-v1'
FACTOR_VERSION = 'btc-ledger-derived-v1'


def parse_ts(value: str) -> datetime:
    return datetime.fromisoformat(value.replace('Z', '+00:00')).astimezone(timezone.utc)


def safe_float(value):
    if value in (None, '', 'null'):
        return 0.0
    return float(value)


def compute_hash(parts):
    import hashlib
    payload = '||'.join(str(p) for p in parts)
    return hashlib.sha256(payload.encode('utf-8')).hexdigest()


def load_csv(path: Path):
    with path.open('r', encoding='utf-8', newline='') as f:
        return list(csv.DictReader(f))


@dataclass
class Bucket:
    start: datetime
    end: datetime
    trades: list
    orders: list
    wallet: list
    equity: list


def bucket_rows(rows, start_key='timestamp', days=30):
    parsed = []
    for row in rows:
        ts = parse_ts(row[start_key])
        parsed.append((ts, row))
    parsed.sort(key=lambda x: x[0])
    if not parsed:
        return []
    buckets = []
    cursor = parsed[0][0]
    end_limit = parsed[-1][0]
    idx = 0
    while cursor <= end_limit:
        bucket_end = cursor + timedelta(days=days)
        chunk = []
        while idx < len(parsed) and parsed[idx][0] < bucket_end:
            chunk.append(parsed[idx][1])
            idx += 1
        buckets.append((cursor, bucket_end, chunk))
        cursor = bucket_end
    return buckets


def rows_for_range(rows, start, end, key='timestamp'):
    return [row for row in rows if start <= parse_ts(row[key]) < end]


def summarise_bucket(trades, orders, wallet, equity):
    liq = Counter(row.get('lastLiquidityInd', '') for row in trades)
    added = liq.get('AddedLiquidity', 0)
    removed = liq.get('RemovedLiquidity', 0)
    total_liq = max(1, added + removed)
    aggression = (removed - added) / total_liq

    statuses = Counter(row.get('ordStatus', '') for row in orders)
    filled = statuses.get('Filled', 0)
    canceled = statuses.get('Canceled', 0)
    completion = filled / max(1, filled + canceled)

    symbols = Counter(row.get('symbol', '') for row in trades)
    top_symbol_share = 0.0
    if symbols:
        _, top_count = symbols.most_common(1)[0]
        top_symbol_share = top_count / max(1, sum(symbols.values()))

    realised_wallet = sum(safe_float(row.get('amount')) for row in wallet if row.get('transactType') == 'RealisedPNL')
    trade_realised = sum(safe_float(row.get('realisedPnl')) for row in trades)

    start_mult = safe_float(equity[0].get('adjustedWealthMultipleVsBaseline')) if equity else 0.0
    end_mult = safe_float(equity[-1].get('adjustedWealthMultipleVsBaseline')) if equity else 0.0
    trough_mult = min((safe_float(row.get('adjustedWealthMultipleVsBaseline')) for row in equity), default=0.0)
    peak_mult = max((safe_float(row.get('adjustedWealthMultipleVsBaseline')) for row in equity), default=0.0)
    wealth_delta = end_mult - start_mult

    ict_score = max(0.0, min(1.0, completion * 0.40 + (1.0 - abs(aggression)) * 0.25 + min(1.0, end_mult / 5.0) * 0.20 + top_symbol_share * 0.15))
    evidence_quality = max(0.0, min(1.0, completion * 0.35 + top_symbol_share * 0.25 + min(1.0, peak_mult / 5.0) * 0.20 + (1.0 - max(0.0, 1.0 - trough_mult)) * 0.20))

    if ict_score >= 0.60:
        verdict = 'ict-compatible execution logic'
    else:
        verdict = 'ledger-native interpretation preferred'

    return {
        'trade_count': len(trades),
        'order_count': len(orders),
        'wallet_event_count': len(wallet),
        'equity_points': len(equity),
        'execution_aggression_bias': aggression,
        'fill_completion_pressure': completion,
        'top_symbol_share': top_symbol_share,
        'wallet_realized_pnl_pulse': realised_wallet,
        'trade_realised_pnl_sum': trade_realised,
        'wealth_multiple_start': start_mult,
        'wealth_multiple_end': end_mult,
        'wealth_multiple_delta': wealth_delta,
        'wealth_multiple_peak': peak_mult,
        'wealth_multiple_trough': trough_mult,
        'ict_interpretability_score': ict_score,
        'evidence_quality_score': evidence_quality,
        'interpretation_verdict': verdict,
        'top_symbols': [{"symbol": s, "count": c} for s, c in symbols.most_common(5)],
        'liquidity': dict(liq),
        'order_statuses': dict(statuses),
    }


def make_ranking(summary, factor_name, composite_score, weaknesses, prompt):
    return {
        'factor_name': factor_name,
        'regime': 'btc_ledger_bucket',
        'ic': 0.0,
        'ir': 0.0,
        'backtest_return': summary['wealth_multiple_delta'],
        'sharpe': 0.0,
        'stability': summary['evidence_quality_score'],
        'win_rate': max(0.0, min(1.0, summary['fill_completion_pressure'])),
        'profit_factor': max(0.0, 1.0 + summary['wealth_multiple_delta']),
        'trade_count': summary['trade_count'],
        'conformal_coverage_1sigma': 0.0,
        'conformal_miscoverage_1sigma': 0.0,
        'mean_prediction_interval_half_width': 0.0,
        'worst_window_miscoverage': 0.0,
        'regime_break_penalty': 0.0,
        'weight': composite_score,
        'regime_scores': {'btc_ledger_bucket': composite_score},
        'composite_score': composite_score,
        'score_breakdown': {
            'fill_completion_pressure': summary['fill_completion_pressure'],
            'ict_interpretability_score': summary['ict_interpretability_score'],
            'evidence_quality_score': summary['evidence_quality_score'],
            'top_symbol_share': summary['top_symbol_share'],
        },
        'grade': 'A' if composite_score >= 0.75 else 'B' if composite_score >= 0.60 else 'C',
        'iteration_action': 'keep' if composite_score >= 0.75 else 'observe' if composite_score >= 0.55 else 'replace',
        'replacement_candidate': composite_score < 0.55,
        'weaknesses': weaknesses,
        'agent_prompt': prompt,
    }


def make_family_decisions(rankings):
    avg = mean(r['composite_score'] for r in rankings) if rankings else 0.0
    return [{
        'family': 'btc_ledger',
        'factor_count': len(rankings),
        'avg_score': avg,
        'actions': sorted(set(r['iteration_action'] for r in rankings)),
        'replacement_candidates': [r['factor_name'] for r in rankings if r['replacement_candidate']],
    }]


def main():
    ap = argparse.ArgumentParser(description='Bucket BTC ledger into ICT-engine-like research runs')
    ap.add_argument('--dataset-root', required=True)
    ap.add_argument('--state-dir', required=True)
    ap.add_argument('--symbol', default='BTC_LEDGER')
    ap.add_argument('--bucket-days', type=int, default=30)
    args = ap.parse_args()

    root = Path(args.dataset_root)
    state_symbol_dir = Path(args.state_dir) / args.symbol
    state_symbol_dir.mkdir(parents=True, exist_ok=True)

    trades_all = load_csv(root / 'api-v1-execution-tradeHistory.csv')
    orders_all = load_csv(root / 'api-v1-order.csv')
    wallet_all = load_csv(root / 'api-v1-user-walletHistory.csv')
    equity_all = load_csv(root / 'derived-equity-curve.csv')

    trade_buckets = bucket_rows(trades_all, 'timestamp', args.bucket_days)
    research_runs = []
    objective_runs = []

    previous_rankings = []
    previous_family = []
    previous_runs_light = []

    for idx, (start, end, trades) in enumerate(trade_buckets, start=1):
        if not trades:
            continue
        orders = rows_for_range(orders_all, start, end)
        wallet = rows_for_range(wallet_all, start, end)
        equity = rows_for_range(equity_all, start, end)
        summary = summarise_bucket(trades, orders, wallet, equity)

        weaknesses = []
        if summary['fill_completion_pressure'] < 0.55:
            weaknesses.append('completion_pressure_weak')
        if abs(summary['execution_aggression_bias']) > 0.25:
            weaknesses.append('execution_aggression_extreme')
        if summary['ict_interpretability_score'] < 0.60:
            weaknesses.append('ict_mapping_weak')
        if summary['wealth_multiple_delta'] < 0.0:
            weaknesses.append('bucket_wealth_contracted')

        ict_prompt = f"Bucket {idx}: test whether fills after likely liquidity events resolve into expansion rather than noise. completion={summary['fill_completion_pressure']:.3f} aggression={summary['execution_aggression_bias']:.3f} wealth_delta={summary['wealth_multiple_delta']:.3f}."
        native_prompt = f"Bucket {idx}: model execution style natively from liquidity mix, fill quality, and wealth state. top_symbol_share={summary['top_symbol_share']:.3f} evidence={summary['evidence_quality_score']:.3f}."

        ict_rank = make_ranking(summary, 'btc_ledger_ict_interpretability', summary['ict_interpretability_score'], weaknesses[:], ict_prompt)
        native_rank = make_ranking(summary, 'btc_ledger_execution_native', summary['evidence_quality_score'], weaknesses[:], native_prompt)
        rankings = sorted([ict_rank, native_rank], key=lambda r: r['composite_score'], reverse=True)
        families = make_family_decisions(rankings)

        run_timestamp = end.isoformat().replace('+00:00', 'Z')
        run_id = f'btc-ledger-bucket:{args.symbol}:{idx:04d}'
        data_fp = compute_hash([
            args.symbol,
            idx,
            start.isoformat(),
            end.isoformat(),
            len(trades),
            len(orders),
            len(wallet),
            len(equity),
            summary['wealth_multiple_end'],
        ])
        provenance = {
            'prompt_version': PROMPT_VERSION,
            'factor_version': FACTOR_VERSION,
            'config_hash': compute_hash(['btc-ledger-bucket', args.bucket_days, args.symbol]),
            'data_fingerprint': data_fp,
        }

        factor_score_deltas = []
        prev_map = {r['factor_name']: r for r in previous_rankings}
        for rank in rankings:
            prev = prev_map.get(rank['factor_name'])
            factor_score_deltas.append({
                'factor_name': rank['factor_name'],
                'previous_score': prev['composite_score'] if prev else None,
                'new_score': rank['composite_score'],
                'score_delta': rank['composite_score'] - (prev['composite_score'] if prev else 0.0),
                'previous_weight': prev['weight'] if prev else None,
                'new_weight': rank['weight'],
                'weight_delta': rank['weight'] - (prev['weight'] if prev else 0.0),
                'previous_action': prev['iteration_action'] if prev else None,
                'new_action': rank['iteration_action'],
            })

        family_diffs = []
        prev_family_map = {f['family']: f for f in previous_family}
        for fam in families:
            prev = prev_family_map.get(fam['family'])
            family_diffs.append({
                'family': fam['family'],
                'previous_avg_score': prev['avg_score'] if prev else None,
                'new_avg_score': fam['avg_score'],
                'avg_score_delta': fam['avg_score'] - (prev['avg_score'] if prev else 0.0),
                'previous_replacement_count': len(prev['replacement_candidates']) if prev else 0,
                'new_replacement_count': len(fam['replacement_candidates']),
            })

        promotion = {
            'approved': rankings[0]['composite_score'] >= 0.70,
            'status': 'approved' if rankings[0]['composite_score'] >= 0.70 else 'observe_only',
            'reason': summary['interpretation_verdict'],
            'target_factors': [rankings[0]['factor_name']],
            'target_families': ['btc_ledger'],
        }
        rollback = {
            'should_rollback': rankings[0]['composite_score'] < 0.45,
            'scope': 'btc_ledger_family' if rankings[0]['composite_score'] < 0.45 else 'none',
            'reason': 'bucket quality degraded' if rankings[0]['composite_score'] < 0.45 else 'stable',
            'target_factors': [r['factor_name'] for r in rankings if r['replacement_candidate']],
            'target_families': ['btc_ledger'] if rankings[0]['composite_score'] < 0.45 else [],
        }
        decision_hist = {
            'total_runs': len(previous_runs_light) + 1,
            'promotion_approved_runs': sum(1 for p, _ in previous_runs_light if p['approved']) + (1 if promotion['approved'] else 0),
            'rollback_recommended_runs': sum(1 for _, r in previous_runs_light if r['should_rollback']) + (1 if rollback['should_rollback'] else 0),
            'latest_promotion_status': promotion['status'],
            'latest_rollback_scope': rollback['scope'],
        }

        family_history = [{
            'family': 'btc_ledger',
            'window_size': len(previous_runs_light) + 1,
            'recent_run_ids': [f'btc-ledger-bucket:{args.symbol}:{i:04d}' for i in range(max(1, idx - 4), idx + 1)],
            'recent_timestamps': [run_timestamp],
            'recent_avg_scores': [families[0]['avg_score']],
            'recent_replacement_counts': [len(families[0]['replacement_candidates'])],
            'score_trend': 'stable',
            'replacement_trend': 'stable',
        }]

        bucket_payload = {
            'run_id': run_id,
            'timestamp': run_timestamp,
            'bucket_start': start.isoformat().replace('+00:00', 'Z'),
            'bucket_end': end.isoformat().replace('+00:00', 'Z'),
            'summary': summary,
            'rankings': rankings,
            'factor_family_decisions': families,
            'factor_family_diffs': family_diffs,
            'factor_score_deltas': factor_score_deltas,
            'promotion_decision': promotion,
            'rollback_recommendation': rollback,
            'decision_history_summary': decision_hist,
            'objective_surfaces': [
                {
                    'factor_name': ict_rank['factor_name'],
                    'research_objective': 'ledger_execution_interpretation',
                    'objective_score': f"{ict_rank['composite_score']:.6f}",
                    'ict_mapping': 'liquidity_grab_then_expansion_reaction',
                    'bucket_range': f"{start.date()}->{(end - timedelta(seconds=1)).date()}",
                    'interpretation': summary['interpretation_verdict'],
                },
                {
                    'factor_name': native_rank['factor_name'],
                    'research_objective': 'ledger_execution_interpretation',
                    'objective_score': f"{native_rank['composite_score']:.6f}",
                    'ict_mapping': 'ledger_native_execution_style',
                    'bucket_range': f"{start.date()}->{(end - timedelta(seconds=1)).date()}",
                    'interpretation': 'ledger-native backup surface',
                },
            ],
        }

        bucket_path = state_symbol_dir / f'ledger_bucket_{idx:04d}.json'
        bucket_path.write_text(json.dumps(bucket_payload, indent=2), encoding='utf-8')

        research_runs.append({
            'run_id': run_id,
            'timestamp': run_timestamp,
            'symbol': args.symbol,
            'research_objective': 'ledger_execution_interpretation',
            'provenance': provenance,
            'decision_thresholds': {'source': 'btc_ledger_bucket_defaults'},
            'dataset_comparability': {
                'comparable': bool(previous_rankings),
                'previous_run_id': research_runs[-1]['run_id'] if research_runs else None,
                'reason': 'same_bucket_protocol' if research_runs else 'first_bucket',
                'comparison_class': 'btc_ledger_bucket',
                'same_data': False,
                'same_config': True,
                'same_prompt_version': True,
                'same_factor_version': True,
            },
            'promotion_decision': promotion,
            'rollback_recommendation': rollback,
            'family_history_window': len(previous_runs_light) + 1,
            'data_path': str(bucket_path),
            'paired_data_path': None,
            'candles': len(trades),
            'paired_candles': None,
            'config_name': f'btc_ledger_bucket_{args.bucket_days}d',
            'source_command': 'btc-ledger-bucket-research',
            'factor_count': len(rankings),
            'best_factor': rankings[0]['factor_name'],
            'aggregate_return': summary['wealth_multiple_delta'],
            'feedback_records_generated': 0,
            'feedback_records_applied': 0,
            'factor_score_deltas': factor_score_deltas,
            'factor_family_decisions': families,
            'factor_family_outcomes': [],
            'factor_family_diffs': family_diffs,
            'factor_family_history': family_history,
            'decision_history_summary': decision_hist,
            'workflow_state': {'phase': 'offline_btc_ledger_bucket'},
            'agent_action_plan': {'items': []},
            'recommended_commands': {'commands': []},
            'recommended_next_command': 'compare bucketed ICT vs ledger-native stability',
            'agent_context_bundle': {'notes': [summary['interpretation_verdict']]},
            'agent_context_bundle_minimal': {'notes': [summary['interpretation_verdict']]},
            'feedback_history_summary': {'generated': 0, 'applied': 0},
            'artifact_action_summary': [],
            'artifact_decision_summary': {'status': 'n/a'},
            'artifact_decision_section': {'headline': 'n/a'},
            'agent_prompts': {'workflow': 'btc_ledger_bucket_research', 'prompts': []},
            'prompt_workflow': 'btc_ledger_bucket_research',
            'factor_mutation_evaluation': None,
            'multi_timeframe_summary': [
                f"bucket_start={start.isoformat().replace('+00:00', 'Z')}",
                f"bucket_end={end.isoformat().replace('+00:00', 'Z')}",
                f"ict_interpretability_score={summary['ict_interpretability_score']:.6f}",
                f"evidence_quality_score={summary['evidence_quality_score']:.6f}",
            ],
        })

        objective_runs.append({
            'run_id': run_id,
            'bucket_start': start.isoformat().replace('+00:00', 'Z'),
            'bucket_end': end.isoformat().replace('+00:00', 'Z'),
            'summary': summary,
            'objective_surfaces': bucket_payload['objective_surfaces'],
        })

        previous_rankings = rankings
        previous_family = families
        previous_runs_light.append((promotion, rollback))

    (state_symbol_dir / 'research_runs.json').write_text(json.dumps(research_runs, indent=2), encoding='utf-8')
    (state_symbol_dir / 'ledger_bucket_objectives.json').write_text(json.dumps(objective_runs, indent=2), encoding='utf-8')
    print(str(state_symbol_dir / 'research_runs.json'))


if __name__ == '__main__':
    main()
