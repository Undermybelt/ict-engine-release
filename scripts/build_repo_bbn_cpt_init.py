#!/usr/bin/env python3
import csv
import json
from pathlib import Path
from collections import Counter, defaultdict

from path_defaults import resolve_policy_training_dir

BASE = resolve_policy_training_dir(__file__)
SRC = BASE / 'tomac_bbn_evidence_v2.csv'
OUT_JSON = BASE / 'repo_bbn_trading_cpt_init.json'
OUT_MD = BASE / 'repo_bbn_trading_cpt_init_summary.txt'

MARKET_STATES = ['bull', 'bear', 'range']
LIQUIDITY_STATES = ['favorable', 'neutral', 'hostile']
ENTRY_STATES = ['high', 'medium', 'low']
OUTCOME_STATES = ['win', 'breakeven', 'loss']

rows=[]
with SRC.open(newline='') as f:
    r=csv.DictReader(f)
    rows=list(r)

def norm(counter, states):
    total=sum(counter.values())
    if total == 0:
        return [round(1.0/len(states), 6) for _ in states]
    vals=[counter.get(s, 0)/total for s in states]
    rounded=[round(v, 6) for v in vals]
    drift=round(1.0-sum(rounded), 6)
    rounded[-1]=round(rounded[-1]+drift, 6)
    return rounded

market=Counter(r['filtered_market_regime_label'] for r in rows)
liquidity=Counter(r['filtered_liquidity_context_label'] for r in rows)
entry=defaultdict(Counter)
outcome=defaultdict(Counter)
for r in rows:
    entry[(r['filtered_market_regime_label'], r['filtered_liquidity_context_label'])][r['entry_quality']]+=1
    outcome[r['entry_quality']][r['trade_outcome']]+=1

payload={
    'schema_version': 'tomac-cpt-init-v1',
    'source_csv': str(SRC),
    'nodes': {
        'market_regime': {
            'states': MARKET_STATES,
            'prior': norm(market, MARKET_STATES),
            'cpt_entries': [[[], norm(market, MARKET_STATES)]],
        },
        'liquidity_context': {
            'states': LIQUIDITY_STATES,
            'prior': norm(liquidity, LIQUIDITY_STATES),
            'cpt_entries': [[[], norm(liquidity, LIQUIDITY_STATES)]],
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
        payload['nodes']['entry_quality']['cpt_entries'].append([[i,j], norm(entry[(m,l)], ENTRY_STATES)])
for i,e in enumerate(ENTRY_STATES):
    payload['nodes']['trade_outcome']['cpt_entries'].append([[i], norm(outcome[e], OUTCOME_STATES)])

OUT_JSON.write_text(json.dumps(payload, indent=2))

lines=[]
lines.append('repo_bbn_trading_cpt_init summary')
lines.append('')
lines.append(f'source={SRC}')
lines.append(f'output={OUT_JSON}')
lines.append('')
lines.append('market_regime prior:')
for s,p in zip(MARKET_STATES, payload['nodes']['market_regime']['prior']):
    lines.append(f'- {s}: {p}')
lines.append('liquidity_context prior:')
for s,p in zip(LIQUIDITY_STATES, payload['nodes']['liquidity_context']['prior']):
    lines.append(f'- {s}: {p}')
lines.append('entry_quality | market_regime, liquidity_context')
for cfg,dist in payload['nodes']['entry_quality']['cpt_entries']:
    mr=MARKET_STATES[cfg[0]]; lc=LIQUIDITY_STATES[cfg[1]]
    lines.append(f'- ({mr}, {lc}) -> high={dist[0]} medium={dist[1]} low={dist[2]}')
lines.append('trade_outcome | entry_quality')
for cfg,dist in payload['nodes']['trade_outcome']['cpt_entries']:
    eq=ENTRY_STATES[cfg[0]]
    lines.append(f'- ({eq}) -> win={dist[0]} breakeven={dist[1]} loss={dist[2]}')
OUT_MD.write_text('\n'.join(lines))
print(OUT_JSON)
print(OUT_MD)
