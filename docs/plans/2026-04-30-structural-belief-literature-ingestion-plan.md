# Structural Belief Literature Ingestion Plan

Date: 2026-04-30
Status: in_progress
Scope: organize structural belief learning literature into repo docs and paper-code surfaces

## Goal

Turn the external structural belief literature shortlist into versioned repo artifacts under `docs/` so the belief-learning upgrade has local citations, mechanism notes, repo mapping, and code-entry candidates.

## Deliverables

1. `docs/structural-belief-learning-literature.md`
- canonical shortlist of up to 12 papers
- per-paper mechanism notes
- repo target mapping
- mismatch notes
- ranked reading lists by use-case

2. `docs/structural-belief-learning-repo-map.md`
- map each paper to repo surfaces:
  - `structural_prior_state`
  - `BBN node/branch posterior update`
  - `live feedback posterior update`
  - `artifact-validation prior source`
  - `CatBoost path ranking target`
- convert paper insights into implementation hooks

3. `docs/paper-code/structural_belief_learning/README.md`
- paper-code index for this literature bucket
- list candidate code repos / reference implementations
- identify which papers are documentation-only vs code-convertible

4. `docs/paper-code/<slug>/README.md` for 1-2 strongest code candidates
- paper summary
- exact mechanism to implement
- code/repo references
- scoped implementation notes

## Selection rule

Prioritize papers with one or more of:
- explicit update equations
- pseudo-count / conjugate update mechanism
- delayed-feedback correction math
- online calibration under drift
- state-transition hierarchy directly reusable for `node / branch / scenario / path`

Deprioritize papers that are only conceptual background.

## Execution order

1. Write canonical literature synthesis doc.
2. Write repo mapping doc.
3. Build `docs/paper-code/structural_belief_learning/` index.
4. Create 2 paper-code stubs for the best implementation candidates.
5. Verify all paths exist and read cleanly.

## Candidate paper-code first picks

1. `bayesian_nonparametric_hidden_semi_markov_models`
- strongest structural-state modeling candidate
- maps to regime node duration and branch transition surfaces

2. `self_calibrating_conformal_prediction`
- strongest ranking calibration candidate
- maps to CatBoost path ranking output calibration

## Notes

- This task is docs-first. No source code changes required.
- `paper2code` can be invoked later for full code conversion if needed.
- Existing `docs/paper-code/*` layout suggests README-first per paper; follow that pattern.
