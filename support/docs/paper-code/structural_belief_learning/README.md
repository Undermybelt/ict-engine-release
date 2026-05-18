# Structural Belief Learning Paper-Code Index

Date: 2026-04-30
Scope: code-facing index for structural belief learning papers relevant to `ict-engine`

## Purpose

This directory collects code-convertible belief-learning literature for the `node / branch / scenario / path` stack.

It does not claim all papers already have full repo implementations here.
It records:
- which paper supplies which mechanism
- whether code conversion is realistic
- whether external code references exist
- where each paper should land in `ict-engine`

## Best first code candidates

### 1. bayesian_nonparametric_hidden_semi_markov_models
- paper: Johnson, Willsky (2012)
- value: duration-aware latent structural state
- repo target: `structural_prior_state`
- external code status: original paper is method-heavy; common HSMM/HDP-HSMM reference code exists in research ecosystems, but no canonical finance repo identified in this pass
- local status: stub created

### 2. self_calibrating_conformal_prediction
- paper: van der Laan, Alaa (2024)
- value: calibrated path probabilities plus uncertainty bounds
- repo target: `CatBoost path ranking target`
- external code status: paper has modern ML reproducibility expectations; likely open artifacts exist around conformal/Venn-Abers implementations even if not in one canonical repo
- local status: stub created

## Other high-value papers

### online_estimation_of_dynamic_bayesian_network_parameter
- value: discounted online transition update
- target: `BBN node/branch posterior update`
- conversion type: equation-to-repo adaptation, not full standalone package

### likelihood_tempering_in_dynamic_model_averaging
- value: source-reliability weighting via tempered likelihood
- target: `artifact-validation prior source`
- conversion type: lightweight formula integration, not standalone model package

### debiased_off_policy_evaluation_for_recommendation_systems
- value: IPS / DR correction for non-executed paths
- target: `live feedback posterior update`
- conversion type: evaluator utility, not full paper stack

### counterfactual_contextual_bandit_delayed_feedback
- value: delayed reward ranking under partial observation
- target: `CatBoost path ranking target`
- conversion type: training-target and logging design guidance

## Suggested next paper2code order

1. `bayesian_nonparametric_hidden_semi_markov_models`
2. `self_calibrating_conformal_prediction`
3. `debiased_off_policy_evaluation_for_recommendation_systems`
4. `online_estimation_of_dynamic_bayesian_network_parameter`

## Rule of use

For this repo, paper conversion should stay narrow:
- extract only the mechanism needed by `ict-engine`
- do not rebuild the paper's entire experimental stack unless the repo needs it
- prefer `README.md + scoped model/eval skeleton` over bloated reproduction bundles
