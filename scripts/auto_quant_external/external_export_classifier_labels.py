#!/usr/bin/env python3
"""
Export daily regime labels from the existing classifier for comparison.

Captures the `regime_table` created by regime_attribution.py and
saves as JSON for comparison with HMM labels.
"""
import json
import sys
from pathlib import Path

import pandas as pd
import numpy as np

NQ_DAILY_FEATHER = Path("/Users/thrill3r/Auto-Quant/user_data/data/NQ_USD-1d.feather")
VIX_CSV = Path("/tmp/ict-engine-ibkr-probe/vix.1d.10y.csv")


def load_daily_regime_table() -> pd.DataFrame:
    """Replicate classifier from regime_attribution.py."""
    nq = pd.read_feather(NQ_DAILY_FEATHER)
    nq["date"] = pd.to_datetime(nq["date"], unit="ms", utc=True).dt.normalize()
    nq = nq.set_index("date").sort_index()

    vix_df = pd.read_csv(VIX_CSV)
    vix_df["ts"] = pd.to_datetime(vix_df["ts"], utc=True, errors="coerce")
    vix_df = vix_df.dropna(subset=["ts", "close"])
    vix_df["date"] = vix_df["ts"].dt.normalize()
    vix = vix_df.set_index("date")["close"].astype(float)
    vix = vix[~vix.index.duplicated(keep="last")].sort_index()

    df = pd.DataFrame(index=nq.index)
    df["nq_close"] = nq["close"]
    df["nq_sma200"] = nq["close"].rolling(200).mean()
    df["nq_above_sma200"] = (df["nq_close"] > df["nq_sma200"]).astype(int)
    df["nq_sma200_slope"] = df["nq_sma200"].pct_change(20)
    df["nq_60d_high"] = nq["close"].rolling(60).max()
    df["nq_drawdown_60d"] = df["nq_close"] / df["nq_60d_high"] - 1.0
    df["vix"] = vix.reindex(df.index).ffill()

    def classify(row: pd.Series) -> str:
        if not (np.isfinite(row["nq_above_sma200"]) and np.isfinite(row["vix"])):
            return "unknown"
        if row["nq_above_sma200"] == 1 and row["nq_sma200_slope"] > 0.005 and row["vix"] < 20:
            return "TrendingCalm"
        if row["nq_above_sma200"] == 1 and row["vix"] >= 20:
            return "TrendingNervous"
        if row["nq_drawdown_60d"] < -0.07 and row["vix"] >= 20:
            return "BearishStress"
        if abs(row["nq_close"] / row["nq_sma200"] - 1.0) < 0.05:
            return "ChopRange"
        if row["nq_above_sma200"] == 0 and row["nq_sma200_slope"] < 0:
            return "BearishStress"
        return "Other"

    df["regime"] = df.apply(classify, axis=1)
    return df


def family_from_regime(regime: str) -> str:
    """Map detailed regime to family for comparison with HMM."""
    mapping = {
        "TrendingCalm": "trend_up",
        "TrendingNervous": "trend_up",  # still trending, just elevated vol
        "BearishStress": "trend_down",
        "ChopRange": "range",
        "Other": "unknown",
        "unknown": "unknown",
    }
    return mapping.get(regime, "unknown")


def main():
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", default="/tmp/classifier_daily_labels.json")
    args = parser.parse_args()

    print("Loading daily regime table...")
    regime_table = load_daily_regime_table()

    # Build per-day labels
    labels = []
    for date, row in regime_table.iterrows():
        labels.append({
            "date": date.isoformat(),
            "regime": row["regime"],
            "family": family_from_regime(row["regime"]),
        })

    output = {
        "source": "classifier_sma200_vix",
        "n_days": len(labels),
        "date_range": {
            "start": labels[0]["date"],
            "end": labels[-1]["date"],
        },
        "regime_distribution": regime_table["regime"].value_counts().to_dict(),
        "family_distribution": {},
        "labels": labels,
    }

    # Compute family distribution
    for label in labels:
        fam = label["family"]
        output["family_distribution"][fam] = output["family_distribution"].get(fam, 0) + 1

    Path(args.output).write_text(json.dumps(output, indent=2))
    print(f"Saved {len(labels)} daily labels to {args.output}")
    print(f"Regime distribution: {output['regime_distribution']}")
    print(f"Family distribution: {output['family_distribution']}")


if __name__ == "__main__":
    main()