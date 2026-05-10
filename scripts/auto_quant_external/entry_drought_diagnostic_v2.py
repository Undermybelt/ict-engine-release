#!/usr/bin/env python3
"""
entry_drought_diagnostic_v2.py

Second-pass entry-drought diagnosis with gate ablations, candidate classification,
and added coverage for the IV/HV compression lane.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Callable

import pandas as pd

import entry_drought_diagnostic as v1

NQ_15M_FEATHER = v1.NQ_15M_FEATHER
NQ_1H_FEATHER = v1.NQ_1H_FEATHER
NQ_4H_FEATHER = v1.NQ_4H_FEATHER
QQQ_IV_CSV = Path("/tmp/ict-engine-ibkr-probe/qqq.iv.1d.10y.csv")
QQQ_HV_CSV = Path("/tmp/ict-engine-ibkr-probe/qqq.hv.1d.10y.csv")


def load_close_series(csv_path: Path) -> pd.Series:
    df = pd.read_csv(csv_path)
    df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
    df = df.dropna(subset=["ts", "close"])
    df["date"] = df["ts"].dt.normalize()
    series = df.set_index("date")["close"].astype(float)
    return series[~series.index.duplicated(keep="last")].sort_index()


def pct_rank_252(series: pd.Series) -> pd.Series:
    return series.rolling(252, min_periods=128).rank(pct=True)


def vrp_compression_15m_gates(df: pd.DataFrame) -> dict[str, pd.Series]:
    df = df.copy()
    df["ema21"] = v1.ema(df["close"], 21)
    df["ema89"] = v1.ema(df["close"], 89)
    df["bullish_body"] = df["close"] > df["open"]
    df["hour_utc"] = df.index.hour
    iv_pr = pct_rank_252(load_close_series(QQQ_IV_CSV))
    hv_pr = pct_rank_252(load_close_series(QQQ_HV_CSV))
    normalized_dates = df.index.normalize()
    df["iv_pct_rank_252"] = pd.Series(normalized_dates.map(iv_pr), index=df.index).ffill()
    df["hv_pct_rank_252"] = pd.Series(normalized_dates.map(hv_pr), index=df.index).ffill()
    return {
        "liquid_window": (df["hour_utc"] >= 13) & (df["hour_utc"] <= 21),
        "iv_compressed": df["iv_pct_rank_252"] < 0.30,
        "hv_compressed": df["hv_pct_rank_252"] < 0.30,
        "not_collapsing": df["close"] > df["ema89"],
        "higher_trend_4h": df["ema_fast_4h"] > df["ema_slow_4h"],
        "bullish_body": df["bullish_body"],
    }


def analyze_gate_ablations(gate_df: pd.DataFrame) -> list[dict[str, float | str]]:
    prepared = gate_df.fillna(False).astype(bool)
    baseline_density = float(prepared.all(axis=1).mean())
    split = max(1, len(prepared) // 2)
    rows: list[dict[str, float | str]] = []
    for gate in prepared.columns:
        remainder = prepared.drop(columns=[gate])
        ablated_mask = remainder.all(axis=1) if not remainder.empty else pd.Series(True, index=prepared.index)
        row = {
            "gate": gate,
            "baseline_density": baseline_density,
            "gate_true_rate": float(prepared[gate].mean()),
            "early_true_rate": float(prepared[gate].iloc[:split].mean()),
            "late_true_rate": float(prepared[gate].iloc[split:].mean()),
            "ablated_density": float(ablated_mask.mean()),
        }
        row["density_lift"] = float(row["ablated_density"] - baseline_density)
        row["late_collapse"] = float(row["early_true_rate"] - row["late_true_rate"])
        rows.append(row)
    rows.sort(key=lambda item: (item["density_lift"], item["late_collapse"]), reverse=True)
    return rows


def find_suspect_gates(
    ablations: list[dict[str, float | str]],
    min_density_lift: float = 0.01,
    min_late_collapse: float = 0.15,
) -> list[dict[str, float | str]]:
    suspects: list[dict[str, float | str]] = []
    for item in ablations:
        if float(item["density_lift"]) >= min_density_lift or float(item["late_collapse"]) >= min_late_collapse:
            suspects.append(item)
    return suspects


def classify_density_issue(
    gate_df: pd.DataFrame,
    ablations: list[dict[str, float | str]],
    min_density_lift: float = 0.01,
) -> str:
    prepared = gate_df.fillna(False).astype(bool)
    all_density = float(prepared.all(axis=1).mean())
    if not ablations:
        return "unknown"
    top = ablations[0]
    if float(top["density_lift"]) >= min_density_lift:
        return "over_gating_issue"
    if all_density < 0.002 and all(float(prepared[col].mean()) > 0.70 for col in prepared.columns):
        return "strategy_logic_issue"
    if all_density < 0.002 and any(float(prepared[col].mean()) < 0.20 for col in prepared.columns):
        return "true_regime_scarcity"
    return "mixed_or_data_alignment_issue"


def monthly_gate_table(gate_df: pd.DataFrame) -> pd.DataFrame:
    prepared = gate_df.fillna(False).astype(bool).copy()
    prepared["all_gates"] = prepared.all(axis=1)
    monthly = prepared.resample("MS").mean() * 100.0
    monthly.index = monthly.index.strftime("%Y-%m")
    return monthly.round(1)


def build_base_frame(start: str, end: str) -> pd.DataFrame:
    base = v1.load_feather(NQ_15M_FEATHER)
    h1 = v1.load_feather(NQ_1H_FEATHER)
    h4 = v1.load_feather(NQ_4H_FEATHER)
    df = v1.merge_higher_tf(base, h1, "1h")
    df = v1.merge_higher_tf(df, h4, "4h")
    return df.loc[start:end]


CandidateFn = Callable[[pd.DataFrame], dict[str, pd.Series]]


CANDIDATES: dict[str, CandidateFn] = {
    "TomacNQ_RegimeTrendPullbackDense15m": v1.trend_pullback_dense_gates,
    "TomacNQ_RegimeLiquiditySweepReclaim15mWide": v1.liquidity_sweep_reclaim_15m_wide_gates,
    "TomacNQ_RegimePersistenceClusterDense15m": v1.persistence_cluster_dense_15m_gates,
    "TomacNQ_RegimeVRPCompression15m": vrp_compression_15m_gates,
}


def diagnose_candidate(name: str, base_df: pd.DataFrame) -> dict[str, object]:
    gate_map = CANDIDATES[name](base_df)
    gate_df = pd.DataFrame(gate_map).fillna(False).astype(bool)
    ablations = analyze_gate_ablations(gate_df)
    suspects = find_suspect_gates(ablations)
    return {
        "candidate": name,
        "issue_class": classify_density_issue(gate_df, ablations),
        "baseline_density": float(gate_df.all(axis=1).mean()),
        "monthly_gate_pct": monthly_gate_table(gate_df).to_dict(orient="index"),
        "ablations": ablations,
        "suspect_gates": suspects,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Entry-drought diagnosis with gate ablations")
    parser.add_argument(
        "--candidate",
        action="append",
        choices=sorted(CANDIDATES.keys()),
        help="Candidate(s) to diagnose; defaults to all",
    )
    parser.add_argument("--start", default="2018-01-01")
    parser.add_argument("--end", default="2025-12-31")
    parser.add_argument("--json-out", help="Optional path to write JSON report")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    base_df = build_base_frame(args.start, args.end)
    selected = args.candidate or list(CANDIDATES.keys())
    reports = [diagnose_candidate(name, base_df) for name in selected]
    payload = {
        "data_span": {"start": args.start, "end": args.end},
        "candidates": reports,
    }
    if args.json_out:
        Path(args.json_out).write_text(json.dumps(payload, indent=2) + "\n")
    else:
        for report in reports:
            print("=" * 110)
            print(report["candidate"])
            print("=" * 110)
            print(f"issue_class: {report['issue_class']}")
            print(f"baseline_density: {report['baseline_density']:.6f}")
            print("suspect_gates:")
            for item in report["suspect_gates"]:
                print(
                    f"  - {item['gate']}: density_lift={item['density_lift']:.6f}, "
                    f"late_collapse={item['late_collapse']:.3f}"
                )
            print("monthly_gate_pct:")
            monthly = pd.DataFrame(report["monthly_gate_pct"]).T
            print(monthly.to_string())
            print("ablations:")
            for item in report["ablations"]:
                print(
                    f"  - {item['gate']}: gate_true_rate={item['gate_true_rate']:.3f}, "
                    f"ablated_density={item['ablated_density']:.6f}, "
                    f"density_lift={item['density_lift']:.6f}, "
                    f"late_collapse={item['late_collapse']:.3f}"
                )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
