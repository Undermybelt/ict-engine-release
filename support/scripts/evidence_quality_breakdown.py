#!/usr/bin/env python3
"""Evidence-quality breakdown helper.

Given explicit components, emits a breakdown and gaps to thresholds.
This mirrors the current config.rs logic in a transparent surface.
"""

import argparse
import json


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--support-gap', type=float, required=True)
    ap.add_argument('--uncertainty', type=float, required=True)
    ap.add_argument('--directional-conflict', action='store_true')
    ap.add_argument('--mixed-alignment', action='store_true')
    ap.add_argument('--mtf-direction-conflict', action='store_true')
    ap.add_argument('--mtf-alignment-weak', action='store_true')
    ap.add_argument('--mtf-entry-weak', action='store_true')
    ap.add_argument('--mtf-alignment-good', action='store_true')
    ap.add_argument('--hostile-liquidity', action='store_true')
    ap.add_argument('--favorable-liquidity', action='store_true')
    ap.add_argument('--hard-pass-threshold', type=float, default=0.75)
    ap.add_argument('--neutralized-threshold', type=float, default=0.40)
    ap.add_argument('--directional-conflict-penalty', type=float, default=0.20)
    ap.add_argument('--mixed-alignment-penalty', type=float, default=0.10)
    ap.add_argument('--mtf-direction-conflict-penalty', type=float, default=0.18)
    ap.add_argument('--mtf-alignment-penalty', type=float, default=0.10)
    ap.add_argument('--mtf-entry-penalty', type=float, default=0.08)
    ap.add_argument('--mtf-alignment-bonus', type=float, default=0.05)
    ap.add_argument('--hostile-liquidity-penalty', type=float, default=0.10)
    ap.add_argument('--favorable-liquidity-bonus', type=float, default=0.05)
    args = ap.parse_args()

    base_score = 0.55
    support_gap_contribution = min(args.support_gap, 0.5) * 0.50
    uncertainty_penalty = args.uncertainty * 0.35
    directional_term = 0.15 if not args.directional_conflict else -args.directional_conflict_penalty
    mixed_penalty = -args.mixed_alignment_penalty if args.mixed_alignment else 0.0
    mtf_direction_penalty = -args.mtf_direction_conflict_penalty if args.mtf_direction_conflict else 0.0
    mtf_alignment_penalty = -args.mtf_alignment_penalty if args.mtf_alignment_weak else 0.0
    mtf_alignment_bonus = args.mtf_alignment_bonus if args.mtf_alignment_good else 0.0
    mtf_entry_penalty = -args.mtf_entry_penalty if args.mtf_entry_weak else 0.0
    liquidity_term = 0.0
    if args.hostile_liquidity:
        liquidity_term -= args.hostile_liquidity_penalty
    elif args.favorable_liquidity:
        liquidity_term += args.favorable_liquidity_bonus

    raw = (
        base_score
        + support_gap_contribution
        - uncertainty_penalty
        + directional_term
        + mixed_penalty
        + mtf_direction_penalty
        + mtf_alignment_penalty
        + mtf_alignment_bonus
        + mtf_entry_penalty
        + liquidity_term
    )
    final_score = max(0.0, min(1.0, raw))
    if final_score >= args.hard_pass_threshold:
        gate = 'pass_hard'
    elif final_score >= args.neutralized_threshold:
        gate = 'pass_neutralized'
    else:
        gate = 'observe_only'

    out = {
        'base_score': base_score,
        'support_gap_contribution': support_gap_contribution,
        'uncertainty_penalty': uncertainty_penalty,
        'directional_term': directional_term,
        'mixed_alignment_penalty': mixed_penalty,
        'mtf_direction_conflict_penalty': mtf_direction_penalty,
        'mtf_alignment_penalty': mtf_alignment_penalty,
        'mtf_alignment_bonus': mtf_alignment_bonus,
        'mtf_entry_penalty': mtf_entry_penalty,
        'liquidity_term': liquidity_term,
        'raw_score': raw,
        'final_evidence_quality_score': final_score,
        'hard_pass_gap': args.hard_pass_threshold - final_score,
        'neutralized_gap': args.neutralized_threshold - final_score,
        'gate_status': gate,
    }
    print(json.dumps(out, indent=2))


if __name__ == '__main__':
    main()
