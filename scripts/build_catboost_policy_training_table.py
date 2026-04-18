#!/usr/bin/env python3
import csv
import re
from functools import lru_cache
from pathlib import Path
import pandas as pd

BASE = Path('/Users/thrill3r/Downloads/Tomac')
OUT = Path('/Users/thrill3r/projects-ict-engine/ict-engine/state/policy_training/tomac_policy_training_v3.csv')

FIELDNAMES = [
    'symbol',
    'timestamp',
    'source_file',
    'strategy_code',
    'strategy_family',
    'logic_node_name',
    'direction_label',
    'result_label',
    'net_pnl',
    'entry',
    'exit',
    'pnl_positive',
    'qualified_label',
    'recommended_command_label',
    'action_label',
    'factor_alignment',
    'factor_uncertainty',
    'gating_status',
    'selected_entry_quality',
    'recommended_command',
    'selected_direction',
    'evidence_quality_score',
    'risk_reward',
    'kelly_fraction',
    'feature_schema_version',
    'label_source',
]

SKIP_NAMES = {
    'full_backtest_summary.csv',
    '90wr1.5rrr_final_summary.csv',
    '90wr1.5rrr_all_contracts_summary.csv',
}

SYMBOL_HINTS = ['EUR', 'YM', 'XAU', 'NQ', 'ES']
PY_FILES = sorted(BASE.glob('*.py'))
SYMBOL_PATH_PATTERNS = [
    (re.compile(r'\bes future\b', re.IGNORECASE), 'ES'),
    (re.compile(r'\bnq future\b', re.IGNORECASE), 'NQ'),
    (re.compile(r'\bym future\b', re.IGNORECASE), 'YM'),
    (re.compile(r'\beur future\b', re.IGNORECASE), 'EUR'),
    (re.compile(r'\bxau future\b', re.IGNORECASE), 'XAU'),
]


def infer_symbol(name: str, py_path: Path | None = None) -> str:
    upper = name.upper()
    for hint in SYMBOL_HINTS:
        if hint in upper:
            return hint
    if py_path:
        py_upper = py_path.name.upper()
        for hint in SYMBOL_HINTS:
            if hint in py_upper:
                return hint
    return 'UNKNOWN'


def normalize_direction(value):
    if pd.isna(value):
        return 'Unknown'
    text = str(value).strip()
    lowered = text.lower()
    if lowered in {'1', 'long', 'buy', 'bull', 'ict_bullish_sfp'}:
        return 'Long'
    if lowered in {'-1', 'short', 'sell', 'bear', 'ict_bearish_sfp'}:
        return 'Short'
    if lowered in {'tp', 'sl', 'eod', 'moc', '', 'nan'}:
        return 'Unknown'
    return text.capitalize() if text else 'Unknown'


def clean_scalar(value):
    if isinstance(value, pd.Series):
        value = value.iloc[0]
    text = str(value)
    if 'dtype:' in text and '\n' in text:
        text = text.split('\n')[0]
        if text.startswith('exit'):
            parts = text.split()
            if len(parts) >= 2:
                return parts[-1]
    return value


def normalize_result(value):
    text = str(value).strip()
    upper = text.upper()
    if upper.startswith('TP'):
        return 'TP'
    if upper.startswith('BE'):
        return 'BE'
    if upper.startswith('SL'):
        return 'SL'
    if upper.startswith('EOD'):
        return 'EOD'
    return text


def infer_symbol_from_text(text: str) -> str | None:
    for pattern, symbol in SYMBOL_PATH_PATTERNS:
        if pattern.search(text):
            return symbol
    return None





def infer_direction_from_reason(value):
    if pd.isna(value):
        return None
    text = str(value).strip().lower()
    if not text:
        return None
    if any(token in text for token in ['bearish', 'short']):
        return 'Short'
    if any(token in text for token in ['bullish', 'long']):
        return 'Long'
    return None


def infer_direction_from_row(row, path: Path):
    explicit = normalize_direction(clean_scalar(row.get('direction', 'Unknown')))
    if explicit != 'Unknown':
        return explicit

    reason_based = infer_direction_from_reason(row.get('reason', ''))
    if reason_based:
        return reason_based

    source = path.name.lower()
    if source in {'no_be_results.csv', 'optimal_be_1.0_results.csv', '90wr1.5rrr_es_results.csv', '90wr1.5rrr_nq_results.csv', 'ict_final_1year.csv', '98wr0.8rrr41.07pf.csv'}:
        return 'Long'
    if source == '85wr12.9pf2.21rrr.csv':
        text = str(row.get('reason', '')).strip().lower()
        if 'trend_continuation' in text:
            return 'Long'
        if 'mom_reversal' in text:
            return 'Short'
    return 'Unknown'

def normalize_columns(df):
    df = df.copy()
    mapping = {}
    for col in df.columns:
        c = str(col).strip().lower()
        if c in {'time', 'entry time', 'entry_time'}:
            mapping[col] = 'timestamp'
        elif c in {'exit time', 'exit_time'}:
            mapping[col] = 'exit_time'
        elif c in {'net pnl', 'pnl'}:
            mapping[col] = 'net_pnl'
        elif c in {'result', 'res'}:
            mapping[col] = 'result'
        elif c == 'reason':
            mapping[col] = 'reason'
        elif c in {'direction', 'type', 'dir'}:
            mapping[col] = 'direction'
        elif c == 'entry':
            mapping[col] = 'entry'
        elif c in {'exit', 'tp1'}:
            mapping[col] = 'exit'
        elif c == 'score':
            mapping[col] = 'score'
    return df.rename(columns=mapping)


def classify(result, pnl, direction):
    result = normalize_result(result).upper()
    direction = normalize_direction(direction)
    pnl = float(pnl) if pd.notna(pnl) else 0.0

    pnl_positive = 1 if pnl > 0 else 0
    qualified = 'qualified' if result in {'TP', 'BE'} else 'disqualified'

    if result == 'TP' and pnl > 0:
        recommended = 'update'
    elif result == 'BE':
        recommended = 'factor-backtest'
    elif result in {'SL', 'EOD'}:
        recommended = 'factor-research'
    else:
        recommended = 'observe'

    if direction == 'Long' and qualified == 'qualified':
        action = 'Bull'
    elif direction == 'Short' and qualified == 'qualified':
        action = 'Bear'
    else:
        action = 'Observe'

    return pnl_positive, qualified, recommended, action


def extract_logic_node_name(py_path: Path):
    if not py_path.exists():
        return 'logic_node_unavailable', 'family_unavailable'
    text = py_path.read_text(errors='ignore')
    patterns = [
        r'def\s+(check_[A-Za-z0-9_]+)',
        r'def\s+(run_[A-Za-z0-9_]+)',
        r'def\s+(calc_[A-Za-z0-9_]+)',
        r'class\s+([A-Za-z0-9_]+Engine)',
    ]
    names = []
    for pattern in patterns:
        names.extend(re.findall(pattern, text))
    family = 'ict_logic' if 'ict' in py_path.name.lower() or 'ict' in text.lower() else 'generic_logic'
    if names:
        return '|'.join(names[:6]), family
    return py_path.stem, family


@lru_cache(maxsize=None)
def inspect_py_metadata(py_path_str: str):
    py_path = Path(py_path_str)
    if not py_path.exists():
        return {
            'symbol': None,
            'logic_node_name': 'logic_node_unavailable',
            'strategy_family': 'family_unavailable',
        }
    text = py_path.read_text(errors='ignore')
    logic_node_name, strategy_family = extract_logic_node_name(py_path)
    symbol = infer_symbol_from_text(text) or infer_symbol(py_path.name)
    return {
        'symbol': symbol,
        'logic_node_name': logic_node_name,
        'strategy_family': strategy_family,
    }


def find_matching_py(csv_path: Path):
    stem = csv_path.stem.lower()
    candidates = []
    for py in PY_FILES:
        py_stem = py.stem.lower()
        overlap = len(set(re.split(r'[_\-. ]+', stem)) & set(re.split(r'[_\-. ]+', py_stem)))
        if overlap > 0:
            candidates.append((overlap, py))
    if not candidates:
        return None
    candidates.sort(key=lambda x: (-x[0], len(x[1].name)))
    return candidates[0][1]


def choose_symbol(csv_path: Path, py_path: Path | None):
    by_name = infer_symbol(csv_path.name, py_path)
    if by_name != 'UNKNOWN':
        return by_name
    if py_path:
        meta = inspect_py_metadata(str(py_path))
        if meta['symbol']:
            return meta['symbol']
    return 'UNKNOWN'


def strategy_code_for(csv_path: Path, py_path: Path | None):
    if py_path:
        return py_path.stem
    return csv_path.stem


def iter_rows(path: Path):
    if path.name in SKIP_NAMES:
        return
    df = pd.read_csv(path)
    df = normalize_columns(df)
    if 'net_pnl' not in df.columns or 'result' not in df.columns:
        return

    py_path = find_matching_py(path)
    symbol = choose_symbol(path, py_path)
    if py_path:
        meta = inspect_py_metadata(str(py_path))
        logic_node_name = meta['logic_node_name']
        strategy_family = meta['strategy_family']
    else:
        logic_node_name, strategy_family = ('logic_node_unavailable', 'family_unavailable')
    strategy_code = strategy_code_for(path, py_path)

    direction_series = df['direction'] if 'direction' in df.columns else pd.Series(['Unknown'] * len(df), index=df.index)
    timestamp_series = df['timestamp'] if 'timestamp' in df.columns else pd.Series(['timestamp_unavailable'] * len(df), index=df.index)
    entry_series = df['entry'] if 'entry' in df.columns else pd.Series([''] * len(df), index=df.index)
    exit_series = df['exit'] if 'exit' in df.columns else pd.Series([''] * len(df), index=df.index)

    for idx, row in df.iterrows():
        raw_direction = direction_series.iloc[idx]
        direction_value = infer_direction_from_row(row, path)
        result_value = normalize_result(clean_scalar(row.get('result', '')))
        entry_value = clean_scalar(entry_series.iloc[idx])
        exit_value = clean_scalar(exit_series.iloc[idx])
        pnl_positive, qualified, recommended, action = classify(
            result_value,
            row.get('net_pnl', 0.0),
            direction_value,
        )
        yield {
            'symbol': symbol,
            'timestamp': str(clean_scalar(timestamp_series.iloc[idx])),
            'source_file': path.name,
            'strategy_code': strategy_code,
            'strategy_family': strategy_family,
            'logic_node_name': logic_node_name,
            'direction_label': direction_value,
            'result_label': result_value,
            'net_pnl': row.get('net_pnl', ''),
            'entry': entry_value,
            'exit': exit_value,
            'pnl_positive': pnl_positive,
            'qualified_label': qualified,
            'recommended_command_label': recommended,
            'action_label': action,
            'factor_alignment': 'alignment_unavailable_from_tomac',
            'factor_uncertainty': 'uncertainty_unavailable_from_tomac',
            'gating_status': 'gate_unavailable_from_tomac',
            'selected_entry_quality': 'entry_quality_unavailable_from_tomac',
            'recommended_command': recommended,
            'selected_direction': action,
            'evidence_quality_score': row.get('score', ''),
            'risk_reward': '',
            'kelly_fraction': '',
            'feature_schema_version': 'policy_features_v1_tomac_bootstrap',
            'label_source': 'tomac_backtest_result_plus_strategy_logic_name',
        }


def main():
    OUT.parent.mkdir(parents=True, exist_ok=True)
    files = sorted(BASE.glob('*.csv'))
    total = 0
    with OUT.open('w', newline='') as f_out:
        writer = csv.DictWriter(f_out, fieldnames=FIELDNAMES)
        writer.writeheader()
        for path in files:
            for row in iter_rows(path) or []:
                writer.writerow(row)
                total += 1
    print(f'wrote={OUT}')
    print(f'rows={total}')
    print(f'files={len(files)}')


if __name__ == '__main__':
    main()
