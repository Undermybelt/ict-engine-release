#!/usr/bin/env python3
import argparse
import json
from datetime import datetime, timezone
from pathlib import Path


def load_json(path: Path):
    return json.loads(path.read_text(encoding='utf-8'))


def utc_now():
    return datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z')


def main():
    ap = argparse.ArgumentParser(description='Translate BTC ledger factor snapshot into ICT-engine objective surfaces')
    ap.add_argument('--input', required=True)
    ap.add_argument('--output', required=True)
    args = ap.parse_args()

    snap = load_json(Path(args.input))
    trade = snap['trade_summary']
    order = snap['order_summary']
    wallet = snap['wallet_summary']
    equity = snap['equity_summary']

    added = trade['liquidity'].get('AddedLiquidity', 0)
    removed = trade['liquidity'].get('RemovedLiquidity', 0)
    total_liq = max(1, added + removed)
    aggression = (removed - added) / total_liq

    filled = order['statuses'].get('Filled', 0)
    canceled = order['statuses'].get('Canceled', 0)
    total_orders = max(1, filled + canceled)
    completion_pressure = filled / total_orders

    latest_mult = equity.get('latest_adjusted_wealth_multiple') or 0.0
    trough_mult = equity.get('trough_adjusted_wealth_multiple') or 0.0
    drawdown_depth = max(0.0, 1.0 - trough_mult)
    compounding_strength = max(0.0, min(1.0, latest_mult / 10.0))

    top_symbols = trade.get('symbols_top', [])
    btc_share = 0.0
    if top_symbols:
        total_top = sum(item['count'] for item in top_symbols)
        btc_count = sum(item['count'] for item in top_symbols if item['symbol'].startswith('XBT'))
        btc_share = btc_count / max(1, total_top)

    wallet_pulse = wallet['amount_by_transact_type'].get('RealisedPNL', 0.0)
    long_horizon_execution_discipline = max(0.0, min(1.0, completion_pressure * 0.55 + (1.0 - abs(aggression)) * 0.20 + btc_share * 0.25))
    ict_style_reaction_after_liquidity = max(0.0, min(1.0, (1.0 - abs(aggression)) * 0.35 + compounding_strength * 0.30 + (1.0 - drawdown_depth) * 0.35))
    evidence_quality = max(0.0, min(1.0, 0.4 * completion_pressure + 0.3 * compounding_strength + 0.3 * btc_share))

    ict_interpretability = {
        'can_explain_with_ict': ict_style_reaction_after_liquidity >= 0.55 and btc_share >= 0.45,
        'verdict': 'ict-compatible execution logic' if ict_style_reaction_after_liquidity >= 0.55 and btc_share >= 0.45 else 'use ledger-native interpretation',
        'reasons': [
            f'aggression_bias={aggression:.4f}',
            f'completion_pressure={completion_pressure:.4f}',
            f'btc_share_top_symbols={btc_share:.4f}',
            f'latest_adjusted_wealth_multiple={latest_mult:.4f}',
            f'trough_adjusted_wealth_multiple={trough_mult:.4f}',
            f'wallet_realised_pnl_total={wallet_pulse:.4f}',
        ],
    }

    surfaces = [
        {
            'factor_name': 'btc_ledger_execution_aggression',
            'research_objective': 'ledger_execution_interpretation',
            'objective_score': f'{abs(aggression):.6f}',
            'ict_mapping': 'liquidity_sweep_follow_through_vs_passive_absorption',
            'ict_interpretation': 'high removed-liquidity share implies chase/continuation behavior; lower absolute aggression implies patient post-sweep execution',
            'ledger_native_interpretation': 'execution style derived from added vs removed liquidity',
        },
        {
            'factor_name': 'btc_ledger_completion_pressure',
            'research_objective': 'ledger_execution_interpretation',
            'objective_score': f'{completion_pressure:.6f}',
            'ict_mapping': 'setup_follow-through_quality',
            'ict_interpretation': 'higher fill completion suggests cleaner participation windows after a move begins',
            'ledger_native_interpretation': 'filled/(filled+canceled) order pressure',
        },
        {
            'factor_name': 'btc_ledger_equity_state',
            'research_objective': 'ledger_execution_interpretation',
            'objective_score': f'{compounding_strength:.6f}',
            'ict_mapping': 'regime_state_compounding_after_manipulation',
            'ict_interpretation': 'persistent wealth expansion can be read as repeated capture of expansion legs after liquidity events',
            'ledger_native_interpretation': 'wealth multiple / drawdown regime',
        },
        {
            'factor_name': 'btc_ledger_ict_interpretability',
            'research_objective': 'ledger_execution_interpretation',
            'objective_score': f'{ict_style_reaction_after_liquidity:.6f}',
            'ict_mapping': 'liquidity_grab_then_expansion_reaction',
            'ict_interpretation': ict_interpretability['verdict'],
            'ledger_native_interpretation': 'blended style score from aggression, concentration, and equity resilience',
        },
        {
            'factor_name': 'btc_ledger_evidence_quality',
            'research_objective': 'ledger_execution_interpretation',
            'objective_score': f'{evidence_quality:.6f}',
            'ict_mapping': 'offline_execution_evidence_quality',
            'ict_interpretation': 'quality gate for whether ledger behavior is stable enough to map into ICT priors',
            'ledger_native_interpretation': 'blend of fill quality, compounding, and BTC concentration',
        },
    ]

    out = {
        'generated_at': utc_now(),
        'input_snapshot': str(Path(args.input)),
        'ict_interpretability': ict_interpretability,
        'objective_surfaces': surfaces,
        'summary': {
            'execution_aggression_bias': aggression,
            'fill_completion_pressure': completion_pressure,
            'btc_top_symbol_share': btc_share,
            'compounding_strength': compounding_strength,
            'drawdown_depth': drawdown_depth,
            'evidence_quality': evidence_quality,
        },
    }

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(out, indent=2), encoding='utf-8')
    print(str(output))


if __name__ == '__main__':
    main()
