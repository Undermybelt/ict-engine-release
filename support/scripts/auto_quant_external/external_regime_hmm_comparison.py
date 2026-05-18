#!/usr/bin/env python3
"""
Compare HMM regime labels against existing regime classifier.

Per TODO Workstream 1:
- Baseline: random family match rate
- Compare: current regime classifier (base+pda + ExtraTrees) vs HMM family labels
- Promote only if agreement > baseline
"""
from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Dict, List, Optional, Tuple

import numpy as np
import pandas as pd


FAMILY_MAP = {
    "trend_up": "trend",
    "trend_down": "trend",
    "trend_volatile": "trend",
    "range_volatile": "range",
    "range_quiet": "range",
    "range_consolidation": "range",
    "transition": "transition",
    "crash": "transition",
    "recovery": "transition",
    "unknown": "unknown",
}


def load_hmm_output(path: Path) -> Dict:
    """Load HMM output JSON."""
    with open(path) as f:
        return json.load(f)


def load_regime_labels(path: Path) -> pd.DataFrame:
    """Load regime labels from feather/CSV."""
    if path.suffix == ".feather":
        df = pd.read_feather(path)
    else:
        df = pd.read_csv(path)
    return df


def compute_baseline_agreement(
    family_distribution: Dict[str, float], n_samples: int = 1000
) -> float:
    """Compute baseline family match rate (random)."""
    # Random assignment weighted by distribution
    families = list(family_distribution.keys())
    probs = np.array(list(family_distribution.values()))
    probs = probs / probs.sum()
    
    random_matches = []
    for _ in range(n_samples):
        true_family = np.random.choice(families, p=probs)
        pred_family = np.random.choice(families, p=probs)
        match = FAMILY_MAP.get(true_family, "unknown") == FAMILY_MAP.get(pred_family, "unknown")
        random_matches.append(match)
    
    return np.mean(random_matches)


def compute_hmm_classifier_agreement(
    hmm_output: Dict, regime_df: pd.DataFrame, date_col: str = "date"
) -> Tuple[float, Dict]:
    """Compute agreement between HMM and existing classifier."""
    # Extract HMM family distribution
    hmm_family_dist = {}
    for state_id, state_data in hmm_output["state_interpretation"].items():
        family = state_data["family_label"]
        pct = state_data["pct"] / 100
        hmm_family_dist[family] = hmm_family_dist.get(family, 0) + pct
    
    # If no overlapping dates, use distribution-based comparison
    # For now, compare family distributions
    hmm_families = list(hmm_family_dist.keys())
    hmm_weights = np.array(list(hmm_family_dist.values()))
    
    # Compare with regime_df family distribution if available
    if "regime_family" in regime_df.columns:
        clf_family_dist = regime_df["regime_family"].value_counts(normalize=True).to_dict()
        
        # Compute overlap
        all_families = set(hmm_families) | set(clf_family_dist.keys())
        hmm_vec = np.array([hmm_family_dist.get(f, 0) for f in all_families])
        clf_vec = np.array([clf_family_dist.get(f, 0) for f in all_families])
        
        # Normalize
        hmm_vec = hmm_vec / (hmm_vec.sum() + 1e-9)
        clf_vec = clf_vec / (clf_vec.sum() + 1e-9)
        
        # Cosine similarity
        cosine_sim = np.dot(hmm_vec, clf_vec) / (np.linalg.norm(hmm_vec) * np.linalg.norm(clf_vec) + 1e-9)
        
        return cosine_sim, {
            "hmm_distribution": hmm_family_dist,
            "classifier_distribution": clf_family_dist,
            "cosine_similarity": cosine_sim,
        }
    
    return 0.0, {"note": "no regime_family column in regime_df"}


def compute_family_switch_agreement(
    hmm_output: Dict, regime_df: Optional[pd.DataFrame]
) -> Dict:
    """Compare family switch timing between HMM and classifier."""
    hmm_flip_rate = hmm_output["transition_metrics"]["flip_rate"]
    
    if regime_df is not None and "regime_family" in regime_df.columns:
        clf_families = regime_df["regime_family"].tolist()
        clf_switches = sum(1 for i in range(len(clf_families) - 1) if clf_families[i] != clf_families[i + 1])
        clf_flip_rate = clf_switches / (len(clf_families) - 1) if len(clf_families) > 1 else 0
        
        return {
            "hmm_flip_rate": hmm_flip_rate,
            "clf_flip_rate": clf_flip_rate,
            "flip_rate_diff": abs(hmm_flip_rate - clf_flip_rate),
        }
    
    return {"hmm_flip_rate": hmm_flip_rate, "clf_flip_rate": None}


def main():
    parser = argparse.ArgumentParser(description="Compare HMM vs regime classifier")
    parser.add_argument("--hmm-output", required=True, help="Path to HMM output JSON")
    parser.add_argument("--regime-labels", default=None, help="Path to regime labels feather/CSV")
    parser.add_argument("--output-dir", default="/tmp/hmm_comparison")
    args = parser.parse_args()
    
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # Load HMM output
    hmm_output = load_hmm_output(Path(args.hmm_output))
    print(f"Loaded HMM output: {args.hmm_output}")
    
    # Extract HMM family distribution
    hmm_family_dist = {}
    for state_id, state_data in hmm_output["state_interpretation"].items():
        family = FAMILY_MAP.get(state_data["family_label"], "unknown")
        pct = state_data["pct"] / 100
        hmm_family_dist[family] = hmm_family_dist.get(family, 0) + pct
    
    print(f"HMM family distribution: {hmm_family_dist}")
    
    # Compute baseline
    baseline = compute_baseline_agreement(hmm_family_dist)
    print(f"Baseline random agreement: {baseline:.4f}")
    
    # Load regime labels if provided
    regime_df = None
    if args.regime_labels:
        regime_df = load_regime_labels(Path(args.regime_labels))
        print(f"Loaded regime labels: {len(regime_df)} rows")
    
    # Compute agreement
    if regime_df is not None:
        agreement, details = compute_hmm_classifier_agreement(hmm_output, regime_df)
        print(f"HMM-Classifier agreement (cosine): {agreement:.4f}")
    else:
        agreement = 0.0
        details = {"note": "no regime labels provided"}
        print("No regime labels provided, skipping agreement computation")
    
    # Compute flip rate comparison
    flip_comparison = compute_family_switch_agreement(hmm_output, regime_df)
    print(f"Flip rate comparison: {flip_comparison}")
    
    # Output
    output = {
        "hmm_output_path": str(args.hmm_output),
        "hmm_family_distribution": hmm_family_dist,
        "baseline_random_agreement": float(baseline),
        "hmm_classifier_agreement": float(agreement),
        "agreement_details": details,
        "flip_rate_comparison": flip_comparison,
        "promote": bool(agreement > baseline),
    }
    
    output_path = output_dir / "hmm_comparison.json"
    with open(output_path, "w") as f:
        json.dump(output, f, indent=2)
    print(f"\nSaved to {output_path}")
    
    # Verdict
    if output["promote"]:
        print("\n[PROMOTE] HMM agreement > baseline")
    else:
        print("\n[DO NOT PROMOTE] HMM agreement <= baseline")
    
    return output


if __name__ == "__main__":
    main()
