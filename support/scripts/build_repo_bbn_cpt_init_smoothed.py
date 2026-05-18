#!/usr/bin/env python3
import csv
import json
from pathlib import Path
from collections import Counter, defaultdict

from path_defaults import resolve_policy_training_dir

BASE = resolve_policy_training_dir(__file__)
SRC = BASE / 'tomac_bbn_evidence_v2.csv'
OUT_JSON = BASE / 'repo_bbn_trading_cpt_init_smoothed.json'
OUT_SUMMARY = BASE / 'repo_bbn_trading_cpt_init_smoothed_summary.txt'
LAPLACE = 1.0

MARKET_STATES = ['bull', 'bear', 'range']
LIQUIDITY_STATES = ['favorable', 'neutral', 'hostile']
ENTRY_STATES = ['high', 'medium', 'low']
OUTCOME_STATES = ['win', 'breakeven', 'loss']

rows=[]
with SRC.open(newline='') as f:
    r=csv.DictReader(f)
    rows=list(r)

def norm(counter, states, alpha=0.0):
    total=sum(counter.get(s, 0) + alpha for s in states)
    vals=[(counter.get(s, 0)+alpha)/total for s in states]
    rounded=[round(v, 6) for v in vals]
    drift=round(1.0-sum(rounded), 6)
    rounded[-1]=round(rounded[-1]+drift, 6)
    return rounded

market=Counter(r['filtered_market_regime_label'] for r in rows)
liquidity=Counter(r['filtered_liquidity_context_label'] for r in rows)
entry=defaultdict(Counter)
outcome=defaultdict(Counter)
by_logic=Counter()
by_family=Counter()
for r in rows:
    entry[(r['filtered_market_regime_label'], r['filtered_liquidity_context_label'])][r['entry_quality']]+=1
    outcome[r['entry_quality']][r['trade_outcome']]+=1
    by_logic[(r['entry_logic_id'], r['trade_outcome'])]+=1
    by_family[(r['logic_family'], r['trade_outcome'])]+=1

payload={
    'schema_version': 'tomac-cpt-init-v2-smoothed',
    'source_csv': str(SRC),
    'laplace_alpha': LAPLACE,
    'nodes': {
        'market_regime': {
            'states': MARKET_STATES,
            'prior': norm(market, MARKET_STATES, LAPLACE),
            'cpt_entries': [[[], norm(market, MARKET_STATES, LAPLACE)]],
        },
        'liquidity_context': {
            'states': LIQUIDITY_STATES,
            'prior': norm(liquidity, LIQUIDITY_STATES, LAPLACE),
            'cpt_entries': [[[], norm(liquidity, LIQUIDITY_STATES, LAPLACE)]],
        },
        'entry_quality': {
            'states': ENTRY_STATES,
            'parents': ['market_regime', 'liquidity_context'],
            'cpt_entries': [],
        },
        'trade_outcome': {
            'states': OUTCOME_STATES,
            'parents': ['entry_quality'],
            'cpt_entries': [],
        },
    }
}
for i,m in enumerate(MARKET_STATES):
    for j,l in enumerate(LIQUIDITY_STATES):
        payload['nodes']['entry_quality']['cpt_entries'].append([[i,j], norm(entry[(m,l)], ENTRY_STATES, LAPLACE)])
for i,e in enumerate(ENTRY_STATES):
    payload['nodes']['trade_outcome']['cpt_entries'].append([[i], norm(outcome[e], OUTCOME_STATES, LAPLACE)])

OUT_JSON.write_text(json.dumps(payload, indent=2))

lines=[]
lines.append('repo_bbn_trading_cpt_init_smoothed summary')
lines.append(f'laplace_alpha={LAPLACE}')
lines.append('')
lines.append('trade_outcome | entry_quality')
for cfg,dist in payload['nodes']['trade_outcome']['cpt_entries']:
    eq=ENTRY_STATES[cfg[0]]
    lines.append(f'- ({eq}) -> win={dist[0]} breakeven={dist[1]} loss={dist[2]}')
lines.append('')
lines.append('top logic outcome counts')
logic_keys=sorted(set(k for k,_ in by_logic.keys()))
for logic in logic_keys[:20]:
    c=Counter()
    for (lg,to),count in by_logic.items():
        if lg==logic:
            c[to]=count
    lines.append(f'- {logic}: {dict(c)}')
lines.append('')
lines.append('top family outcome counts')
family_keys=sorted(set(k for k,_ in by_family.keys()))
for fam in family_keys[:20]:
    c=Counter()
    for (fg,to),count in by_family.items():
        if fg==fam:
            c[to]=count
    lines.append(f'- {fam}: {dict(c)}')
OUT_SUMMARY.write_text('\n'.join(lines))
print(OUT_JSON)
print(OUT_SUMMARY)
