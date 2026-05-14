#!/usr/bin/env python3
"""
Compare HMM regime labels against classifier labels.

Aggregates HMM 15m labels to daily, then compares family agreement.
"""
import json
import sys
from pathlib import Path
from collections import Counter
import pandas as pd

def main():
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--hmm", default="/tmp/hmm_regime_nq_15m/hmm_regime_4states.json")
    parser.add_argument("--classifier", default="/tmp/classifier_daily_labels.json")
    parser.add_argument("--output", default="/tmp/regime_family_comparison.json")
    args = parser.parse_args()

    # Load HMM
    hmm_data = json.loads(Path(args.hmm).read_text())
    hmm_labels = hmm_data.get("labels", [])
    print(f"HMM: {len(hmm_labels)} bar labels")

    # Load classifier
    clf_data = json.loads(Path(args.classifier).read_text())
    clf_labels = clf_data.get("labels", [])
    print(f"Classifier: {len(clf_labels)} day labels")

    # Aggregate HMM to daily
    hmm_daily = {}
    for bar in hmm_labels:
        ts = bar["ts"]
        # Parse ISO timestamp, extract date
        if "T" in ts:
            date_part = ts.split("T")[0]
        else:
            date_part = ts[:10]
        if date_part not in hmm_daily:
            hmm_daily[date_part] = []
        hmm_daily[date_part].append(bar["family"])

    # Take most common family per day
    hmm_daily_family = {}
    for date, families in hmm_daily.items():
        most_common = Counter(families).most_common(1)[0][0]
        hmm_daily_family[date] = most_common

    print(f"HMM aggregated to {len(hmm_daily_family)} days")

    # Build classifier lookup
    clf_by_date = {}
    for day in clf_labels:
        date = day["date"][:10]
        clf_by_date[date] = day["family"]

    # Compare overlap
    common_dates = set(hmm_daily_family.keys()) & set(clf_by_date.keys())
    print(f"Common dates: {len(common_dates)}")

    if not common_dates:
        print("No overlapping dates for comparison")
        sys.exit(1)

    # Compute agreement
    agreement_count = 0
    disagreement_examples = []
    for date in sorted(common_dates):
        hmm_fam = hmm_daily_family[date]
        clf_fam = clf_by_date[date]
        if hmm_fam == clf_fam:
            agreement_count += 1
        else:
            if len(disagreement_examples) < 10:
                disagreement_examples.append({
                    "date": date,
                    "hmm": hmm_fam,
                    "classifier": clf_fam,
                })

    agreement_rate = agreement_count / len(common_dates)

    # Compute random baseline
    hmm_family_dist = Counter(hmm_daily_family.values())
    clf_family_dist = Counter(clf_by_date.values())
    total_hmm = sum(hmm_family_dist.values())
    total_clf = sum(clf_family_dist.values())

    # Random agreement = sum of (p_hmm_i * p_clf_i) for each family
    random_baseline = 0
    families = set(hmm_family_dist.keys()) | set(clf_family_dist.keys())
    for fam in families:
        p_hmm = hmm_family_dist.get(fam, 0) / total_hmm if total_hmm > 0 else 0
        p_clf = clf_family_dist.get(fam, 0) / total_clf if total_clf > 0 else 0
        random_baseline += p_hmm * p_clf

    # Output
    result = {
        "n_common_days": len(common_dates),
        "hmm_family_distribution": dict(hmm_family_dist),
        "classifier_family_distribution": dict(clf_family_dist),
        "agreement_rate": round(agreement_rate, 4),
        "random_baseline": round(random_baseline, 4),
        "excess_over_random": round(agreement_rate - random_baseline, 4),
        "disagreement_examples": disagreement_examples,
        "success": agreement_rate > random_baseline + 0.1,  # must beat random by 10%
    }

    Path(args.output).write_text(json.dumps(result, indent=2))
    print(f"\n=== Comparison Results ===")
    print(f"Agreement rate: {agreement_rate:.2%}")
    print(f"Random baseline: {random_baseline:.2%}")
    print(f"Excess over random: {agreement_rate - random_baseline:.2%}")
    print(f"SUCCESS: {result['success']}")
    print(f"\nSaved to {args.output}")

    if disagreement_examples:
        print("\nSample disagreements:")
        for ex in disagreement_examples[:5]:
            print(f"  {ex['date']}: HMM={ex['hmm']} vs Classifier={ex['classifier']}")


if __name__ == "__main__":
    main()
