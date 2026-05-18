#!/usr/bin/env python3
import csv
from pathlib import Path
from collections import Counter

from path_defaults import resolve_policy_training_dir

BASE = resolve_policy_training_dir(__file__)
LOGIC_MAP = BASE / 'tomac_entry_logic_map.csv'
V4 = BASE / 'tomac_policy_training_v4.csv'
V4_TRAINABLE = BASE / 'tomac_policy_training_v4_trainable.csv'
V4_ENRICHED = BASE / 'tomac_policy_training_v4_logic_enriched.csv'
V4_TRAINABLE_ENRICHED = BASE / 'tomac_policy_training_v4_trainable_logic_enriched.csv'
BBN_V1 = BASE / 'tomac_bbn_evidence_v1.csv'
BBN_DOC = BASE / 'tomac_bbn_evidence_v1_fields.txt'
BBN_REPORT = BASE / 'tomac_bbn_evidence_v1_report.txt'

LOGIC_FIELDS = [
    'logic_family',
    'entry_logic_id',
    'entry_logic_name',
    'entry_logic_long_label',
    'entry_logic_short_label',
    'logic_node_seed',
    'primary_functions',
    'logic_signature',
    'bbn_hint_node',
    'execution_tree_hint',
]


def load_logic_map():
    mapping = {}
    with LOGIC_MAP.open(newline='') as f:
        r = csv.DictReader(f)
        for row in r:
            mapping[row['source_file']] = row
    return mapping


def enrich_file(src: Path, dst: Path, mapping):
    with src.open(newline='') as f:
        r = csv.DictReader(f)
        fieldnames = r.fieldnames + [c for c in LOGIC_FIELDS if c not in r.fieldnames]
        rows = []
        for row in r:
            m = mapping.get(row['source_file'], {})
            for c in LOGIC_FIELDS:
                row[c] = m.get(c, '')
            rows.append(row)
    with dst.open('w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=fieldnames)
        w.writeheader()
        w.writerows(rows)
    return rows, fieldnames


def liquidity_context_from_row(row):
    reason = row.get('reason_label', '')
    logic_family = row.get('logic_family', '')
    source_schema = row.get('source_schema_type', '')
    if 'OTE' in reason or 'FVG' in reason or 'OB' in reason:
        return 'favorable'
    if 'Sweep' in reason or 'SFP' in reason or 'sweep' in logic_family or 'sfp' in logic_family:
        return 'favorable'
    if source_schema in {'result_only', 'timestamp_result_only'}:
        return 'neutral'
    return 'neutral'


def market_regime_from_row(row):
    logic_family = row.get('logic_family', '')
    direction = row.get('direction_label', '')
    if any(x in logic_family for x in ['divergence', 'reversal', 'sweep']):
        return 'range'
    if direction == 'Long':
        return 'bull'
    if direction == 'Short':
        return 'bear'
    return 'range'


def filter_stage_from_row(row):
    schema = row.get('source_schema_type', '')
    if schema == 'price_pair_with_direction':
        return 'price_explicit'
    if schema in {'time_pair_reason', 'time_pair_type_result'}:
        return 'time_explicit'
    if schema in {'result_only', 'timestamp_result_only'}:
        return 'proxy_only'
    return 'raw_proxy'


def filter_confidence_from_row(row):
    schema = row.get('source_schema_type', '')
    reason = row.get('reason_label', '')
    logic = row.get('entry_logic_id', '')
    if schema == 'price_pair_with_direction':
        return 'high'
    if schema in {'time_pair_reason', 'time_pair_type_result'} and reason:
        return 'high'
    if schema == 'timestamp_result_only' and logic:
        return 'medium'
    if schema == 'result_only' and logic:
        return 'medium'
    return 'low'


def filter_quality_bucket(row):
    conf = filter_confidence_from_row(row)
    entry_kind = row.get('entry_kind', '')
    if conf == 'high' and entry_kind == 'price':
        return 'high'
    if conf == 'high':
        return 'medium'
    if conf == 'medium':
        return 'medium'
    return 'low'


def entry_quality_from_row(row):
    result = row.get('result_label', '')
    conf = filter_confidence_from_row(row)
    if result == 'TP' and conf == 'high':
        return 'high'
    if result in {'TP', 'BE'}:
        return 'medium'
    return 'low'


def trade_outcome_from_row(row):
    result = row.get('result_label', '')
    if result == 'TP':
        return 'win'
    if result == 'BE':
        return 'breakeven'
    return 'loss'


def build_bbn(rows):
    fieldnames = [
        'symbol',
        'timestamp',
        'source_file',
        'strategy_code',
        'logic_family',
        'entry_logic_id',
        'logic_node_seed',
        'direction_label',
        'reason_label',
        'source_schema_type',
        'entry_kind',
        'exit_kind',
        'market_regime',
        'liquidity_context',
        'entry_quality',
        'trade_outcome',
        'filter_stage',
        'filter_confidence',
        'filter_quality_bucket',
        'bbn_hint_node',
        'execution_tree_hint',
        'net_pnl',
        'result_label',
    ]
    out_rows = []
    for row in rows:
        out_rows.append({
            'symbol': row['symbol'],
            'timestamp': row['timestamp'],
            'source_file': row['source_file'],
            'strategy_code': row['strategy_code'],
            'logic_family': row.get('logic_family', ''),
            'entry_logic_id': row.get('entry_logic_id', ''),
            'logic_node_seed': row.get('logic_node_seed', ''),
            'direction_label': row['direction_label'],
            'reason_label': row.get('reason_label', ''),
            'source_schema_type': row.get('source_schema_type', ''),
            'entry_kind': row.get('entry_kind', ''),
            'exit_kind': row.get('exit_kind', ''),
            'market_regime': market_regime_from_row(row),
            'liquidity_context': liquidity_context_from_row(row),
            'entry_quality': entry_quality_from_row(row),
            'trade_outcome': trade_outcome_from_row(row),
            'filter_stage': filter_stage_from_row(row),
            'filter_confidence': filter_confidence_from_row(row),
            'filter_quality_bucket': filter_quality_bucket(row),
            'bbn_hint_node': row.get('bbn_hint_node', ''),
            'execution_tree_hint': row.get('execution_tree_hint', ''),
            'net_pnl': row['net_pnl'],
            'result_label': row['result_label'],
        })
    with BBN_V1.open('w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=fieldnames)
        w.writeheader()
        w.writerows(out_rows)
    return out_rows


def write_docs(rows):
    BBN_DOC.write_text('\n'.join([
        'tomac_bbn_evidence_v1 fields',
        '',
        'market_regime: proxy node -> bull | bear | range',
        'liquidity_context: proxy node -> favorable | neutral | hostile (current build emits favorable/neutral)',
        'entry_quality: proxy node -> high | medium | low',
        'trade_outcome: canonical node -> win | breakeven | loss',
        'filter_stage: price_explicit | time_explicit | proxy_only | raw_proxy',
        'filter_confidence: high | medium | low',
        'filter_quality_bucket: high | medium | low',
        'entry_logic_id: stable logic label extracted from py strategy logic',
        'logic_node_seed: naming seed for BBN/execution tree nodes',
        'bbn_hint_node: nearest existing BBN surface this logic should hang under',
    ]))

    direction = Counter(r['direction_label'] for r in rows)
    regime = Counter(r['market_regime'] for r in rows)
    liq = Counter(r['liquidity_context'] for r in rows)
    eq = Counter(r['entry_quality'] for r in rows)
    out = Counter(r['trade_outcome'] for r in rows)
    conf = Counter(r['filter_confidence'] for r in rows)
    stage = Counter(r['filter_stage'] for r in rows)
    logic = Counter(r['entry_logic_id'] for r in rows)
    BBN_REPORT.write_text('\n'.join([
        'tomac_bbn_evidence_v1 report',
        '',
        f'rows: {len(rows)}',
        'direction:', *[f'- {k}: {v}' for k, v in sorted(direction.items())],
        'market_regime:', *[f'- {k}: {v}' for k, v in sorted(regime.items())],
        'liquidity_context:', *[f'- {k}: {v}' for k, v in sorted(liq.items())],
        'entry_quality:', *[f'- {k}: {v}' for k, v in sorted(eq.items())],
        'trade_outcome:', *[f'- {k}: {v}' for k, v in sorted(out.items())],
        'filter_confidence:', *[f'- {k}: {v}' for k, v in sorted(conf.items())],
        'filter_stage:', *[f'- {k}: {v}' for k, v in sorted(stage.items())],
        'top entry_logic_id:', *[f'- {k}: {v}' for k, v in logic.most_common(15)],
    ]))


def main():
    mapping = load_logic_map()
    enrich_file(V4, V4_ENRICHED, mapping)
    train_rows, _ = enrich_file(V4_TRAINABLE, V4_TRAINABLE_ENRICHED, mapping)
    bbn_rows = build_bbn(train_rows)
    write_docs(bbn_rows)
    print(f'wrote={V4_ENRICHED}')
    print(f'wrote={V4_TRAINABLE_ENRICHED}')
    print(f'wrote={BBN_V1}')
    print(f'doc={BBN_DOC}')
    print(f'report={BBN_REPORT}')
    print(f'rows={len(bbn_rows)}')

if __name__ == '__main__':
    main()
