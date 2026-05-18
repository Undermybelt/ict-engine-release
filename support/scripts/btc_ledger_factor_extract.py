#!/usr/bin/env python3
import argparse
import csv
import json
import math
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path


def parse_ts(value: str) -> datetime:
    return datetime.fromisoformat(value.replace('Z', '+00:00')).astimezone(timezone.utc)


def safe_float(value):
    if value in (None, '', 'null'):
        return 0.0
    return float(value)


def read_csv_rows(path: Path):
    with path.open('r', encoding='utf-8', newline='') as f:
        return list(csv.DictReader(f))


def top_counts(rows, key, n=10):
    c = Counter(row.get(key, '') or '' for row in rows)
    return [{"key": k, "count": v} for k, v in c.most_common(n)]


def build_trade_summary(rows):
    symbols = Counter()
    sides = Counter()
    ord_types = Counter()
    exec_types = Counter()
    liquidity = Counter()
    realised = defaultdict(float)
    notionals = defaultdict(float)
    hourly = Counter()
    weekday = Counter()
    durations = []
    first_ts = None
    last_ts = None

    for row in rows:
        ts = parse_ts(row['timestamp'])
        first_ts = ts if first_ts is None or ts < first_ts else first_ts
        last_ts = ts if last_ts is None or ts > last_ts else last_ts
        symbol = row.get('symbol', '')
        symbols[symbol] += 1
        sides[row.get('side', '')] += 1
        ord_types[row.get('ordType', '')] += 1
        exec_types[row.get('execType', '')] += 1
        liquidity[row.get('lastLiquidityInd', '')] += 1
        realised[symbol] += safe_float(row.get('realisedPnl'))
        notionals[symbol] += abs(safe_float(row.get('homeNotional')))
        hourly[f'{ts.hour:02d}'] += 1
        weekday[str(ts.weekday())] += 1

    sorted_realised = sorted(realised.items(), key=lambda kv: kv[1], reverse=True)
    sorted_notional = sorted(notionals.items(), key=lambda kv: kv[1], reverse=True)
    return {
        'rows': len(rows),
        'first_timestamp': first_ts.isoformat().replace('+00:00', 'Z') if first_ts else None,
        'last_timestamp': last_ts.isoformat().replace('+00:00', 'Z') if last_ts else None,
        'symbols_top': [{"symbol": k, "count": v} for k, v in symbols.most_common(12)],
        'sides': dict(sides),
        'order_types': dict(ord_types),
        'exec_types': dict(exec_types),
        'liquidity': dict(liquidity),
        'realised_pnl_by_symbol_top': [{"symbol": k, "realisedPnl": v} for k, v in sorted_realised[:12]],
        'home_notional_by_symbol_top': [{"symbol": k, "homeNotionalAbs": v} for k, v in sorted_notional[:12]],
        'hourly_activity_utc': dict(hourly),
        'weekday_activity_utc': dict(weekday),
    }


def build_order_summary(rows):
    statuses = Counter(row.get('ordStatus', '') for row in rows)
    order_types = Counter(row.get('ordType', '') for row in rows)
    tif = Counter(row.get('timeInForce', '') for row in rows)
    symbols = Counter(row.get('symbol', '') for row in rows)
    avg_fill_ratio = 0.0
    if rows:
        ratios = []
        for row in rows:
            qty = safe_float(row.get('orderQty'))
            cum = safe_float(row.get('cumQty'))
            if qty > 0:
                ratios.append(cum / qty)
        avg_fill_ratio = sum(ratios) / len(ratios) if ratios else 0.0
    return {
        'rows': len(rows),
        'statuses': dict(statuses),
        'order_types': dict(order_types),
        'time_in_force': dict(tif),
        'symbols_top': [{"symbol": k, "count": v} for k, v in symbols.most_common(12)],
        'avg_fill_ratio': avg_fill_ratio,
    }


def build_wallet_summary(rows):
    tx_types = Counter(row.get('transactType', '') for row in rows)
    currencies = Counter(row.get('currency', '') for row in rows)
    refs = Counter(row.get('reference', '') for row in rows if 'reference' in row)
    amount_by_type = defaultdict(float)
    fee_by_type = defaultdict(float)
    first_ts = None
    last_ts = None
    for row in rows:
        ts = parse_ts(row['timestamp'])
        first_ts = ts if first_ts is None or ts < first_ts else first_ts
        last_ts = ts if last_ts is None or ts > last_ts else last_ts
        tx_type = row.get('transactType', '')
        amount_by_type[tx_type] += safe_float(row.get('amount'))
        fee_by_type[tx_type] += safe_float(row.get('fee'))
    return {
        'rows': len(rows),
        'first_timestamp': first_ts.isoformat().replace('+00:00', 'Z') if first_ts else None,
        'last_timestamp': last_ts.isoformat().replace('+00:00', 'Z') if last_ts else None,
        'transact_types': dict(tx_types),
        'currencies': dict(currencies),
        'amount_by_transact_type': dict(amount_by_type),
        'fee_by_transact_type': dict(fee_by_type),
    }


def build_equity_summary(rows):
    first = rows[0] if rows else None
    last = rows[-1] if rows else None
    peak = None
    trough = None
    peak_mult = -1.0
    trough_mult = float('inf')
    for row in rows:
        mult = safe_float(row.get('adjustedWealthMultipleVsBaseline'))
        if mult > peak_mult:
            peak_mult = mult
            peak = row
        if mult < trough_mult:
            trough_mult = mult
            trough = row
    return {
        'rows': len(rows),
        'baseline_timestamp': first.get('baselineTimestamp') if first else None,
        'baseline_balance_xbt': safe_float(first.get('baselineBalanceXBT')) if first else None,
        'latest_adjusted_wealth_xbt': safe_float(last.get('adjustedWealthXBT')) if last else None,
        'latest_adjusted_wealth_multiple': safe_float(last.get('adjustedWealthMultipleVsBaseline')) if last else None,
        'peak_adjusted_wealth_multiple': peak_mult if peak else None,
        'peak_timestamp': peak.get('timestamp') if peak else None,
        'trough_adjusted_wealth_multiple': trough_mult if trough else None,
        'trough_timestamp': trough.get('timestamp') if trough else None,
    }


def derive_factor_hypotheses(trades, orders, wallet, equity):
    top_symbols = [x['symbol'] for x in trades['symbols_top'][:6]]
    trade_liq = trades['liquidity']
    added = trade_liq.get('AddedLiquidity', 0)
    removed = trade_liq.get('RemovedLiquidity', 0)
    total = max(1, added + removed)
    return [
        {
            'name': 'execution_aggression_bias',
            'kind': 'microstructure_style',
            'formula_hint': 'removed_liquidity_share - added_liquidity_share',
            'value_hint': (removed / total) - (added / total),
            'why': '区分主动追价与挂单吸收风格。',
        },
        {
            'name': 'symbol_concentration_entropy',
            'kind': 'regime_focus',
            'formula_hint': 'entropy(symbol fill counts)',
            'value_hint': None,
            'why': '测持仓/交易是否集中于 BTC 主轴或扩散至杂项合约。',
            'top_symbols': top_symbols,
        },
        {
            'name': 'wallet_realized_pnl_pulse',
            'kind': 'equity_feedback',
            'formula_hint': 'daily realised pnl delta from walletHistory',
            'value_hint': wallet['amount_by_transact_type'].get('RealisedPNL', 0.0),
            'why': '可作状态转移/风格切换监督标签。',
        },
        {
            'name': 'fill_completion_pressure',
            'kind': 'order_quality',
            'formula_hint': 'avg cumQty/orderQty by session',
            'value_hint': orders['avg_fill_ratio'],
            'why': '测挂单成交环境与冲击成本。',
        },
        {
            'name': 'equity_drawdown_state',
            'kind': 'regime_state',
            'formula_hint': 'adjusted wealth multiple zscore / drawdown bucket',
            'value_hint': equity['trough_adjusted_wealth_multiple'],
            'why': '可直接当研究标签，不必先还原价格预测。',
        },
    ]


def main():
    parser = argparse.ArgumentParser(description='Extract factor-ready summaries from BTC-Trading-Since-2020 ledger dump')
    parser.add_argument('--dataset-root', required=True)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()

    root = Path(args.dataset_root)
    manifest = json.loads((root / 'manifest.json').read_text(encoding='utf-8'))
    trades = build_trade_summary(read_csv_rows(root / 'api-v1-execution-tradeHistory.csv'))
    orders = build_order_summary(read_csv_rows(root / 'api-v1-order.csv'))
    wallet = build_wallet_summary(read_csv_rows(root / 'api-v1-user-walletHistory.csv'))
    equity = build_equity_summary(read_csv_rows(root / 'derived-equity-curve.csv'))

    payload = {
        'dataset_root': str(root),
        'generated_at': datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z'),
        'manifest_window': manifest.get('dataset_window', {}),
        'trade_summary': trades,
        'order_summary': orders,
        'wallet_summary': wallet,
        'equity_summary': equity,
        'factor_hypotheses': derive_factor_hypotheses(trades, orders, wallet, equity),
    }

    out = Path(args.output)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(payload, indent=2), encoding='utf-8')
    print(str(out))


if __name__ == '__main__':
    main()
