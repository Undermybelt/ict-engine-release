#!/usr/bin/env python3
import csv
from pathlib import Path
from collections import Counter, defaultdict

SRC = Path('/Users/thrill3r/projects-ict-engine/ict-engine/state/policy_training/tomac_bbn_evidence_v2.csv')
OUT_DIR = Path('/Users/thrill3r/projects-ict-engine/ict-engine/state/policy_training/cpt_seed_tables')
OUT_DIR.mkdir(parents=True, exist_ok=True)

rows=[]
with SRC.open(newline='') as f:
    r=csv.DictReader(f)
    rows=list(r)

# P(entry_quality | market_regime, liquidity_context)
counts_eq=defaultdict(Counter)
for row in rows:
    key=(row['filtered_market_regime_label'], row['filtered_liquidity_context_label'])
    counts_eq[key][row['entry_quality']]+=1
with (OUT_DIR/'entry_quality_given_regime_liquidity.csv').open('w', newline='') as f:
    w=csv.writer(f)
    w.writerow(['market_regime','liquidity_context','entry_quality','count','prob'])
    for key,c in sorted(counts_eq.items()):
        total=sum(c.values())
        for state,count in sorted(c.items()):
            w.writerow([key[0],key[1],state,count,f'{count/total:.6f}'])

# P(trade_outcome | entry_quality)
counts_to=defaultdict(Counter)
for row in rows:
    counts_to[row['entry_quality']][row['trade_outcome']]+=1
with (OUT_DIR/'trade_outcome_given_entry_quality.csv').open('w', newline='') as f:
    w=csv.writer(f)
    w.writerow(['entry_quality','trade_outcome','count','prob'])
    for key,c in sorted(counts_to.items()):
        total=sum(c.values())
        for state,count in sorted(c.items()):
            w.writerow([key,state,count,f'{count/total:.6f}'])

# P(gating_status | factor_uncertainty, resonance)
counts_gate=defaultdict(Counter)
for row in rows:
    key=(row['filtered_factor_uncertainty'], row['filtered_multi_timeframe_resonance_label'])
    counts_gate[key][row['gating_status']]+=1
with (OUT_DIR/'gating_status_given_uncertainty_resonance.csv').open('w', newline='') as f:
    w=csv.writer(f)
    w.writerow(['factor_uncertainty','multi_timeframe_resonance','gating_status','count','prob'])
    for key,c in sorted(counts_gate.items()):
        total=sum(c.values())
        for state,count in sorted(c.items()):
            w.writerow([key[0],key[1],state,count,f'{count/total:.6f}'])

# P(liquidity_context | entry_logic_id)
counts_liq=defaultdict(Counter)
for row in rows:
    counts_liq[row['entry_logic_id']][row['filtered_liquidity_context_label']]+=1
with (OUT_DIR/'liquidity_context_given_entry_logic.csv').open('w', newline='') as f:
    w=csv.writer(f)
    w.writerow(['entry_logic_id','liquidity_context','count','prob'])
    for key,c in sorted(counts_liq.items()):
        total=sum(c.values())
        for state,count in sorted(c.items()):
            w.writerow([key,state,count,f'{count/total:.6f}'])

print(OUT_DIR)
