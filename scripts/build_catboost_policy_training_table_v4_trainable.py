#!/usr/bin/env python3
import csv
from pathlib import Path

SRC = Path('/Users/thrill3r/projects-ict-engine/ict-engine/state/policy_training/tomac_policy_training_v4.csv')
OUT = Path('/Users/thrill3r/projects-ict-engine/ict-engine/state/policy_training/tomac_policy_training_v4_trainable.csv')
RULES = Path('/Users/thrill3r/projects-ict-engine/ict-engine/state/policy_training/tomac_policy_training_v4_trainable_rules.txt')
REPORT = Path('/Users/thrill3r/projects-ict-engine/ict-engine/state/policy_training/tomac_policy_training_v4_trainable_report.txt')

ALLOWED_RESULTS = {'TP', 'BE', 'SL', 'EOD', 'Long_Div', 'Short_Div'}
DROP_UNKNOWN_DIRECTION = True


def canonical_result(row):
    result = row['result_label']
    if result in {'TP', 'BE', 'SL', 'EOD'}:
        return result
    if result == 'Long_Div':
        return 'TP' if float(row['net_pnl']) > 0 else 'SL'
    if result == 'Short_Div':
        return 'TP' if float(row['net_pnl']) > 0 else 'SL'
    return ''


def direction_from_row(row):
    direction = row['direction_label']
    if direction in {'Long', 'Short'}:
        return direction
    if row['result_label'] == 'Long_Div':
        return 'Long'
    if row['result_label'] == 'Short_Div':
        return 'Short'
    if row['source_file'] == '85wr12.9pf2.21rrr.csv' and row['reason_label'] == 'EOD':
        return 'Short'
    return direction


def main():
    rows = []
    with SRC.open(newline='') as f:
        r = csv.DictReader(f)
        fieldnames = r.fieldnames
        for row in r:
            if row['result_label'] not in ALLOWED_RESULTS:
                continue
            row['direction_label'] = direction_from_row(row)
            row['result_label'] = canonical_result(row)
            if not row['result_label']:
                continue
            if DROP_UNKNOWN_DIRECTION and row['direction_label'] == 'Unknown':
                continue
            rows.append(row)

    OUT.parent.mkdir(parents=True, exist_ok=True)
    with OUT.open('w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=fieldnames)
        w.writeheader()
        w.writerows(rows)

    from collections import Counter
    direction = Counter(r['direction_label'] for r in rows)
    result = Counter(r['result_label'] for r in rows)
    source = Counter(r['source_file'] for r in rows)
    entry_kind = Counter(r['entry_kind'] for r in rows)
    schema = Counter(r['source_schema_type'] for r in rows)

    RULES.write_text(
        '\n'.join([
            'Tomac v4 trainable whitelist rules',
            '',
            '1. Source file: tomac_policy_training_v4.csv',
            '2. Keep only rows whose result_label is in {TP, BE, SL, EOD, Long_Div, Short_Div}.',
            '3. Canonicalize Long_Div/Short_Div using net_pnl sign:',
            '   - positive pnl -> TP',
            '   - non-positive pnl -> SL',
            '4. Set direction_label from explicit direction when present.',
            '5. If direction missing but result_label is Long_Div/Short_Div, set direction to Long/Short respectively.',
            '6. Special-case remaining 85wr EOD unknown row as Short based on source logic bias already observed in SFP_Vol_Expansion branch handling.',
            '7. Drop rows that still have Unknown direction after canonicalization.',
            '8. Keep entry/exit raw plus entry_kind/exit_kind; downstream trainer should choose whether to exclude time-kind or missing-kind fields as model features.',
        ])
    )

    REPORT.write_text(
        '\n'.join([
            'Tomac v4 trainable report',
            '',
            f'input: {SRC}',
            f'output: {OUT}',
            f'rows: {len(rows)}',
            '',
            'direction:',
            *[f'- {k}: {v}' for k, v in sorted(direction.items())],
            'result:',
            *[f'- {k}: {v}' for k, v in sorted(result.items())],
            'entry_kind:',
            *[f'- {k}: {v}' for k, v in sorted(entry_kind.items())],
            'source_schema_type:',
            *[f'- {k}: {v}' for k, v in sorted(schema.items())],
            'top sources:',
            *[f'- {k}: {v}' for k, v in source.most_common(15)],
        ])
    )

    print(f'wrote={OUT}')
    print(f'rows={len(rows)}')
    print(f'rules={RULES}')
    print(f'report={REPORT}')


if __name__ == '__main__':
    main()
