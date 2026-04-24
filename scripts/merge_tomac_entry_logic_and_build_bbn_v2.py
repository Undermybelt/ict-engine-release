#!/usr/bin/env python3
import csv
import json
from pathlib import Path
from collections import Counter

from path_defaults import resolve_policy_training_dir

BASE = resolve_policy_training_dir(__file__)
LOGIC_MAP = BASE / 'tomac_entry_logic_map.csv'
V4_TRAINABLE = BASE / 'tomac_policy_training_v4_trainable.csv'
V4_TRAINABLE_ENRICHED = BASE / 'tomac_policy_training_v4_trainable_logic_enriched_v2.csv'
BBN_V2 = BASE / 'tomac_bbn_evidence_v2.csv'
BBN_DOC = BASE / 'tomac_bbn_evidence_v2_fields.txt'
BBN_REPORT = BASE / 'tomac_bbn_evidence_v2_report.txt'
PRE_BAYES_JSONL = BASE / 'tomac_pre_bayes_packets_v1.jsonl'
BELIEF_JSONL = BASE / 'tomac_belief_packets_v1.jsonl'

LOGIC_FIELDS = [
    'logic_family','entry_logic_id','entry_logic_name','entry_logic_long_label','entry_logic_short_label',
    'logic_node_seed','primary_functions','logic_signature','bbn_hint_node','execution_tree_hint'
]


def load_logic_map():
    mapping = {}
    with LOGIC_MAP.open(newline='') as f:
        r = csv.DictReader(f)
        for row in r:
            mapping[row['source_file']] = row
    return mapping


def enrich_rows():
    mapping = load_logic_map()
    with V4_TRAINABLE.open(newline='') as f:
        r = csv.DictReader(f)
        fieldnames = r.fieldnames + [c for c in LOGIC_FIELDS if c not in r.fieldnames]
        rows = []
        for row in r:
            m = mapping.get(row['source_file'], {})
            for c in LOGIC_FIELDS:
                row[c] = m.get(c, '')
            rows.append(row)
    with V4_TRAINABLE_ENRICHED.open('w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=fieldnames)
        w.writeheader()
        w.writerows(rows)
    return rows


def filtered_market_regime_label(row):
    logic_family = row.get('logic_family', '')
    direction = row.get('direction_label', '')
    if any(x in logic_family for x in ['divergence', 'reversal', 'sweep', 'sfp']):
        return 'range'
    if direction == 'Long':
        return 'bull'
    if direction == 'Short':
        return 'bear'
    return 'range'


def filtered_liquidity_context_label(row):
    reason = row.get('reason_label', '')
    logic_family = row.get('logic_family', '')
    stage = filter_stage(row)
    if any(x in reason for x in ['Sweep', 'SFP']) or any(x in logic_family for x in ['sweep', 'sfp', 'ote', 'ict_zone']):
        return 'favorable'
    if stage == 'raw_proxy':
        return 'hostile'
    if stage in {'result_only', 'timestamp_result_only', 'proxy_only'} or row.get('source_schema_type') in {'result_only', 'timestamp_result_only'}:
        return 'neutral'
    return 'neutral'


def filtered_factor_alignment(row):
    if row.get('direction_label') == 'Long':
        return 'bullish'
    if row.get('direction_label') == 'Short':
        return 'bearish'
    return 'mixed'


def filtered_factor_uncertainty(row):
    stage = filter_stage(row)
    if stage == 'price_explicit':
        return 'low'
    if stage in {'time_explicit', 'proxy_only'}:
        return 'medium'
    return 'high'


def filtered_multi_timeframe_resonance_label(row):
    stage = filter_stage(row)
    if stage == 'price_explicit':
        return 'aligned'
    if stage in {'time_explicit', 'proxy_only'}:
        return 'mixed'
    return 'dislocated'


def filter_stage(row):
    schema = row.get('source_schema_type', '')
    if schema == 'price_pair_with_direction':
        return 'price_explicit'
    if schema in {'time_pair_reason', 'time_pair_type_result'}:
        return 'time_explicit'
    if schema in {'result_only', 'timestamp_result_only'}:
        return 'proxy_only'
    return 'raw_proxy'


def evidence_quality_score(row):
    stage = filter_stage(row)
    result = row.get('result_label', '')
    score = 0.55
    if stage == 'price_explicit':
        score += 0.25
    elif stage == 'time_explicit':
        score += 0.10
    elif stage == 'proxy_only':
        score -= 0.05
    else:
        score -= 0.15
    if result == 'TP':
        score += 0.10
    elif result == 'BE':
        score += 0.02
    elif result in {'SL', 'EOD'}:
        score -= 0.05
    return f"{max(0.0, min(1.0, score)):.3f}"


def gating_status(row):
    score = float(evidence_quality_score(row))
    if score >= 0.80:
        return 'pass_hard'
    if score >= 0.60:
        return 'pass_neutralized'
    return 'observe_only'


def trade_outcome(row):
    result = row.get('result_label', '')
    if result == 'TP':
        return 'win'
    if result == 'BE':
        return 'breakeven'
    return 'loss'


def entry_quality(row):
    score = float(evidence_quality_score(row))
    if score >= 0.80 and row.get('result_label') == 'TP':
        return 'high'
    if score >= 0.60:
        return 'medium'
    return 'low'


def conflict_flags(row):
    flags = []
    if filtered_multi_timeframe_resonance_label(row) == 'dislocated':
        flags.append('multi_timeframe_direction_conflict')
    if filtered_factor_uncertainty(row) == 'high':
        flags.append('high_factor_uncertainty')
    if filtered_liquidity_context_label(row) == 'neutral' and filter_stage(row) == 'raw_proxy':
        flags.append('low_directional_separation')
    return '|'.join(flags)


def rationale(row):
    parts = [
        f"entry_logic_id={row.get('entry_logic_id','')}",
        f"logic_family={row.get('logic_family','')}",
        f"stage={filter_stage(row)}",
        f"result={row.get('result_label','')}",
    ]
    if row.get('reason_label'):
        parts.append(f"reason={row['reason_label']}")
    return ' ; '.join(parts)


def build_rows(rows):
    out_fields = [
        'symbol','timestamp','source_file','strategy_code','entry_logic_id','logic_family','logic_node_seed',
        'direction_label','reason_label','source_schema_type','entry_kind','exit_kind',
        'filtered_market_regime_label','filtered_liquidity_context_label','filtered_factor_alignment',
        'filtered_factor_uncertainty','filtered_multi_timeframe_resonance_label','evidence_quality_score',
        'gating_status','conflict_flags','rationale','entry_quality','trade_outcome','bbn_hint_node','execution_tree_hint',
        'result_label','net_pnl'
    ]
    out_rows = []
    for row in rows:
        out_rows.append({
            'symbol': row['symbol'],
            'timestamp': row['timestamp'],
            'source_file': row['source_file'],
            'strategy_code': row['strategy_code'],
            'entry_logic_id': row.get('entry_logic_id',''),
            'logic_family': row.get('logic_family',''),
            'logic_node_seed': row.get('logic_node_seed',''),
            'direction_label': row['direction_label'],
            'reason_label': row.get('reason_label',''),
            'source_schema_type': row.get('source_schema_type',''),
            'entry_kind': row.get('entry_kind',''),
            'exit_kind': row.get('exit_kind',''),
            'filtered_market_regime_label': filtered_market_regime_label(row),
            'filtered_liquidity_context_label': filtered_liquidity_context_label(row),
            'filtered_factor_alignment': filtered_factor_alignment(row),
            'filtered_factor_uncertainty': filtered_factor_uncertainty(row),
            'filtered_multi_timeframe_resonance_label': filtered_multi_timeframe_resonance_label(row),
            'evidence_quality_score': evidence_quality_score(row),
            'gating_status': gating_status(row),
            'conflict_flags': conflict_flags(row),
            'rationale': rationale(row),
            'entry_quality': entry_quality(row),
            'trade_outcome': trade_outcome(row),
            'bbn_hint_node': row.get('bbn_hint_node',''),
            'execution_tree_hint': row.get('execution_tree_hint',''),
            'result_label': row.get('result_label',''),
            'net_pnl': row.get('net_pnl',''),
        })
    with BBN_V2.open('w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=out_fields)
        w.writeheader()
        w.writerows(out_rows)
    return out_rows




def write_packet_exports(rows):
    with PRE_BAYES_JSONL.open('w') as f_pre, BELIEF_JSONL.open('w') as f_belief:
        for row in rows:
            evidence_assignments = {
                'market_regime': row['filtered_market_regime_label'],
                'liquidity_context': row['filtered_liquidity_context_label'],
                'factor_alignment': row['filtered_factor_alignment'],
                'factor_uncertainty': row['filtered_factor_uncertainty'],
                'multi_timeframe_resonance': row['filtered_multi_timeframe_resonance_label'],
            }
            conflict_flags = row['conflict_flags'].split('|') if row['conflict_flags'] else []
            rationale_list = [part.strip() for part in row['rationale'].split(';')] if row['rationale'] else []
            pre = {
                'filter': {
                    'raw_market_regime_label': row['filtered_market_regime_label'],
                    'raw_liquidity_context_label': row['filtered_liquidity_context_label'],
                    'raw_factor_alignment': row['filtered_factor_alignment'],
                    'raw_factor_uncertainty': row['filtered_factor_uncertainty'],
                    'raw_multi_timeframe_direction_bias': row['direction_label'].lower(),
                    'filtered_market_regime_label': row['filtered_market_regime_label'],
                    'filtered_liquidity_context_label': row['filtered_liquidity_context_label'],
                    'filtered_factor_alignment': row['filtered_factor_alignment'],
                    'filtered_factor_uncertainty': row['filtered_factor_uncertainty'],
                    'filtered_multi_timeframe_direction_bias': row['direction_label'].lower(),
                    'raw_multi_timeframe_resonance_label': row['filtered_multi_timeframe_resonance_label'],
                    'filtered_multi_timeframe_resonance_label': row['filtered_multi_timeframe_resonance_label'],
                    'raw_multi_timeframe_alignment_score': None,
                    'filtered_multi_timeframe_alignment_score': None,
                    'raw_multi_timeframe_entry_alignment_score': None,
                    'filtered_multi_timeframe_entry_alignment_score': None,
                    'evidence_quality_score': float(row['evidence_quality_score']),
                    'gating_status': row['gating_status'],
                    'conflict_flags': conflict_flags,
                    'rationale': rationale_list,
                    'evidence_assignments': evidence_assignments,
                    'uses_soft_evidence': row['gating_status'] != 'pass_hard',
                    'soft_market_regime_distribution': {},
                    'soft_liquidity_context_distribution': {},
                    'active_pda_count': 0,
                    'inversed_pda_count': 0,
                    'stale_pda_count': 0,
                    'nearest_active_pda': None,
                    'nearest_inversed_pda': None,
                },
                'evidence_assignments': evidence_assignments,
                'timed_pda_summary': {
                    'bias': None,
                    'dealing_range': None,
                    'session': None,
                    'active_pda_count': 0,
                    'inversed_pda_count': 0,
                    'stale_pda_count': 0,
                    'nearest_active_pda': None,
                    'nearest_inversed_pda': None,
                    'notes': [],
                },
                'raw_market_regime_trace': {'label': row['filtered_market_regime_label'], 'derivation': 'tomac_bbn_v2_proxy', 'evidence': rationale_list},
                'raw_liquidity_context_trace': {'label': row['filtered_liquidity_context_label'], 'derivation': 'tomac_bbn_v2_proxy', 'evidence': rationale_list},
                'raw_multi_timeframe_resonance_trace': {'label': row['filtered_multi_timeframe_resonance_label'], 'derivation': 'tomac_bbn_v2_proxy', 'evidence': rationale_list},
            }
            belief = {
                'symbol': row['symbol'],
                'market': row['symbol'],
                'timestamp': row['timestamp'],
                'regime_features': {
                    'market_regime_label': row['filtered_market_regime_label'],
                    'volatility_regime_label': row['filtered_factor_uncertainty'],
                    'liquidity_regime_label': row['filtered_liquidity_context_label'],
                    'stress_score': 1.0 - float(row['evidence_quality_score']),
                    'transition_score': 1.0 if row['filtered_multi_timeframe_resonance_label']=='dislocated' else (0.5 if row['filtered_multi_timeframe_resonance_label']=='mixed' else 0.0),
                    'evidence': conflict_flags,
                },
                'market_evidence': [f"entry_logic_id={row['entry_logic_id']}", f"logic_family={row['logic_family']}"],
                'factor_evidence': rationale_list,
                'timed_pda_summary': {'active_pda_count': '0', 'inversed_pda_count': '0', 'stale_pda_count': '0'},
                'multi_timeframe_evidence': {
                    'raw_direction_bias': row['direction_label'].lower(),
                    'filtered_direction_bias': row['direction_label'].lower(),
                    'raw_resonance_label': row['filtered_multi_timeframe_resonance_label'],
                    'filtered_resonance_label': row['filtered_multi_timeframe_resonance_label'],
                },
                'evidence_assignments': evidence_assignments,
            }
            f_pre.write(json.dumps(pre) + '\n')
            f_belief.write(json.dumps(belief) + '\n')

def write_docs(rows):
    BBN_DOC.write_text('\n'.join([
        'tomac_bbn_evidence_v2 fields',
        '',
        'This version aligns names to legacy_pre_bayes style surfaces.',
        'filtered_market_regime_label: bull | bear | range',
        'filtered_liquidity_context_label: favorable | neutral | hostile (current build emits favorable/neutral)',
        'filtered_factor_alignment: bullish | bearish | mixed',
        'filtered_factor_uncertainty: low | medium | high',
        'filtered_multi_timeframe_resonance_label: aligned | mixed | dislocated',
        'evidence_quality_score: 0..1 proxy score',
        'gating_status: pass_hard | pass_neutralized | observe_only',
        'conflict_flags: pipe-delimited proxy conflict flags',
        'rationale: compact provenance line for each row',
        'entry_logic_id / logic_family / logic_node_seed remain the naming spine for BBN and execution tree nodes',
    ]))
    c = lambda key: Counter(r[key] for r in rows)
    report = []
    report.append('tomac_bbn_evidence_v2 report')
    report.append('')
    report.append(f'rows: {len(rows)}')
    for key in [
        'filtered_market_regime_label','filtered_liquidity_context_label','filtered_factor_alignment',
        'filtered_factor_uncertainty','filtered_multi_timeframe_resonance_label','gating_status','entry_quality','trade_outcome'
    ]:
        report.append(f'{key}:')
        for k,v in sorted(c(key).items()):
            report.append(f'- {k}: {v}')
    report.append('top entry_logic_id:')
    for k,v in Counter(r['entry_logic_id'] for r in rows).most_common(15):
        report.append(f'- {k}: {v}')
    BBN_REPORT.write_text('\n'.join(report))


def main():
    rows = enrich_rows()
    out_rows = build_rows(rows)
    write_docs(out_rows)
    write_packet_exports(out_rows)
    print(f'wrote={V4_TRAINABLE_ENRICHED}')
    print(f'wrote={BBN_V2}')
    print(f'doc={BBN_DOC}')
    print(f'report={BBN_REPORT}')
    print(f'pre_bayes_jsonl={PRE_BAYES_JSONL}')
    print(f'belief_jsonl={BELIEF_JSONL}')
    print(f'rows={len(out_rows)}')

if __name__ == '__main__':
    main()
