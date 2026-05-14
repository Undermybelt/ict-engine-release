# Self-Calibrating Conformal Prediction

Paper: https://arxiv.org/abs/2402.07307
Title: "Self-Calibrating Conformal Prediction"
Authors: Lars van der Laan, Ahmed M. Alaa (2024)

## Core Contribution

Combine score calibration with conformal prediction so predictions are both probability-calibrated and uncertainty-aware.

For `ict-engine`, this is the best calibration-layer candidate for path ranking outputs.

## Why this matters here

Current target shape is:
- BBN / structural layer narrows candidate paths
- CatBoost ranks candidate paths
- execution gate should not trust raw score blindly

This paper supports a cleaner output surface:
- raw path score
- calibrated path probability
- lower confidence bound for execution gating

## Exact mechanism to steal

Sequence:
1. calibrate model outputs into probabilities via Venn-Abers style calibration
2. construct conformal uncertainty object on top of calibrated outputs

Transfer to repo:
- `raw_path_score -> calibrated_path_prob`
- `calibrated_path_prob -> path_prob_lower_bound`
- gate execution on lower bound, not just point estimate

## Suggested repo target
- `CatBoost path ranking target`

## Code conversion scope

Minimal local implementation should include:
- probability calibration wrapper
- rolling calibration buffer
- lower-bound extraction utility for execution gating
- note on regime-conditional calibration extension

## External code / references

No canonical GitHub repo verified in this pass.
Likely practical references will come from:
- Venn-Abers implementations
- conformal prediction libraries
- small adapter around CatBoost scores

Search terms to resume with:
- `self calibrating conformal prediction github`
- `venn abers calibration github`
- `conformal prediction calibration python github`

## Implementation notes for ict-engine

Do not overbuild.
First version can be:
1. export held-out path scores
2. fit calibration map
3. attach calibrated probability to path ranking artifact
4. add lower-bound threshold as secondary execution gate
