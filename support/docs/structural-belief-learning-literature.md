# Structural Belief Learning Literature

Date: 2026-04-30
Status: curated
Scope: principled foundations for `ict-engine` structural belief learning

## Goal

This note curates a short, implementation-oriented paper set for upgrading `ict-engine` from heuristic structural belief updates to more principled prior initialization, posterior updating, source weighting, and path-ranking calibration.

Target questions:
1. how to model structural state
2. how to seed priors from offline evidence
3. how to update posteriors from live path outcomes
4. how to weight heterogeneous evidence sources
5. how to calibrate path ranking under delayed reward and partial compliance

---

## Ranked shortlist

### Top 3 for offline prior seeding
1. Likelihood Tempering in Dynamic Model Averaging
2. Posterior Belief Assessment
3. Bayesian Nonparametric Hidden Semi-Markov Models

### Top 3 for live posterior updating
1. The Beta-Binomial Bayesian Model
2. Online Estimation of Dynamic Bayesian Network Parameter
3. Bayesian Nonparametric Hidden Semi-Markov Models

### Top 3 for path ranking / delayed feedback calibration
1. Debiased Off-Policy Evaluation for Recommendation Systems
2. Counterfactual contextual bandit for recommendation under delayed feedback
3. Self-Calibrating Conformal Prediction

---

## Paper cards

### 1. Bayesian Nonparametric Hidden Semi-Markov Models

1. Citation
- title: Bayesian Nonparametric Hidden Semi-Markov Models
- authors: Matthew J. Johnson, Alan S. Willsky
- venue: arXiv / later journal circulation in Bayesian nonparametric sequence modeling
- year: 2012
- link: https://arxiv.org/abs/1203.1365

2. Why it matters
- strongest state-modeling paper for `node / branch / scenario / path`
- gives explicit duration-aware latent structure, not just instant Markov switching
- directly addresses structural node persistence and branch transitions

3. Exact mechanism to steal
- hidden semi-Markov state with explicit duration:
  - `z_s ~ pi_(z_{s-1})`
  - `d_s ~ Dur(theta_{z_s})`
  - observations within segment share state `z_s`
- replace heuristic node persistence with duration-aware hazard
- use filtered posterior over state and duration as structural prior carrier
- engineerable idea: `node` as latent state, `branch` as transition row, duration prior as regime stickiness prior

4. Suggested repo target
- structural_prior_state

5. Risk / mismatch
- inference in paper is heavier than current repo likely wants online
- may need approximate filtering rather than full Gibbs/MCMC

### 2. Detecting bearish and bullish markets in financial time series using hierarchical hidden Markov models

1. Citation
- title: Detecting bearish and bullish markets in financial time series using hierarchical hidden Markov models
- authors: Lennart Oelschläger, Timo Adam
- venue: Statistical Modelling
- year: 2023
- link: https://arxiv.org/abs/2007.14874 ; DOI: https://doi.org/10.1177/1471082X211034048

2. Why it matters
- gives a finance-native hierarchy across long and short horizon latent states
- useful for separating structural node from path/scenario microstate

3. Exact mechanism to steal
- use HHMM split:
  - upper state = long-horizon structural regime
  - lower state = short-horizon fluctuation or tactical subregime
- steal multi-timescale latent decomposition, not just bull/bear labels
- map:
  - structural node -> upper regime
  - scenario/path -> lower regime conditioned on upper regime
- protects repo from misreading short noise as node transition

4. Suggested repo target
- BBN node/branch posterior update

5. Risk / mismatch
- paper output is still finance-regime labeling, not full playbook execution stack
- reliability weighting must come from elsewhere

### 3. Online Estimation of Dynamic Bayesian Network Parameter

1. Citation
- title: Online Estimation of Dynamic Bayesian Network Parameter
- authors: H.C. Cho, S.M. Fadali
- venue: IJCNN
- year: 2006
- link: DOI https://doi.org/10.1109/IJCNN.2006.247336

2. Why it matters
- one of the cleanest online-update bridges from BBN logic to running evidence streams
- useful for branch posterior updates from rolling observations

3. Exact mechanism to steal
- online recursive sufficient-statistics update with forgetting
- generic pattern:
  - `S_t = lambda * S_(t-1) + s_t`
  - transition counts update from posterior pairwise state probabilities
  - normalize into transition probabilities after each update window
- steal the discounted online transition-count logic for branch posterior maintenance

4. Suggested repo target
- BBN node/branch posterior update

5. Risk / mismatch
- old and generic; not finance-specific
- does not solve heterogeneous source weighting by itself

### 4. Likelihood Tempering in Dynamic Model Averaging

1. Citation
- title: Likelihood Tempering in Dynamic Model Averaging
- authors: Jan Reichl, Kamil Dedecius
- venue: Springer Proceedings in Mathematics & Statistics
- year: 2017
- link: DOI https://doi.org/10.1007/978-3-319-54084-9_7

2. Why it matters
- strongest direct justification for turning source weights into principled likelihood powers
- fits current source ordering in repo

3. Exact mechanism to steal
- tempered dynamic update:
  - `p_t(m) propto p_(t-1)(m)^alpha * L_t(m)^tau`
- `alpha` = forgetting / persistence of previous belief
- `tau` = source reliability / evidence intensity
- map current source weighting to `tau_source`
- map coverage, break penalty, mutation acceptance, execution readiness into source-specific tempering strength

4. Suggested repo target
- artifact-validation prior source

5. Risk / mismatch
- model-averaging context, not path-level belief tree by itself
- still requires design choice for pseudo-likelihood construction

### 5. Posterior Belief Assessment: Extracting Meaningful Subjective Judgements from Bayesian Analyses with Complex Statistical Models

1. Citation
- title: Posterior Belief Assessment: Extracting Meaningful Subjective Judgements from Bayesian Analyses with Complex Statistical Models
- authors: Daniel Williamson, Michael Goldstein
- venue: Bayesian Analysis
- year: 2015
- link: https://arxiv.org/abs/1512.00969 ; DOI: https://doi.org/10.1214/15-BA966SI

2. Why it matters
- directly relevant when offline seed does not come from one clean likelihood model
- useful for combining multiple imperfect modeling views into one posterior-facing belief surface

3. Exact mechanism to steal
- run multiple alternative Bayesian analyses under different judgement sets
- combine posterior summaries via posterior belief assessment rather than forcing one canonical prior/likelihood too early
- engineerable translation:
  - maintain per-source posterior summaries
  - combine via shrinkage/covariance-aware aggregation instead of hard overwrite
- best use: offline prior synthesis panel before pushing into canonical prior state

4. Suggested repo target
- structural_prior_state

5. Risk / mismatch
- mathematically rich, implementation heavier than simple weighted pseudo-counts
- better for offline synthesis than live tick-by-tick updating

### 6. The Beta-Binomial Bayesian Model

1. Citation
- title: The Beta-Binomial Bayesian Model
- authors: Alicia A. Johnson, Miles Q. Ott, Mine Dogucu
- venue: Bayes Rules! chapter
- year: 2022
- link: DOI https://doi.org/10.1201/9780429288340-3

2. Why it matters
- simplest clean posterior update mechanism for path success/failure under sparse data
- most directly implementable for live path lineage updates

3. Exact mechanism to steal
- prior and posterior:
  - `p_k ~ Beta(alpha_k, beta_k)`
  - posterior after outcomes: `Beta(alpha_k + s, beta_k + f)`
- key transfer is fractional pseudo-counts:
  - `alpha += w_source * q_quality * c_compliance * reward_credit`
  - `beta  += w_source * q_quality * c_compliance * loss_credit`
- delayed resolution can accumulate pending evidence then settle later
- posterior mean or lower credible bound can drive ranking support metrics

4. Suggested repo target
- live feedback posterior update

5. Risk / mismatch
- binary success framing may be too simple for rich path outcome taxonomy
- multi-class scenario outcomes would need Dirichlet extension

### 7. Conformal Predictive Systems Under Covariate Shift

1. Citation
- title: Conformal Predictive Systems Under Covariate Shift
- authors: Jef Jonkers, Glenn Van Wallendael, Luc Duchateau, Sofie Van Hoecke
- venue: COPA / arXiv
- year: 2024
- link: https://arxiv.org/abs/2404.15018

2. Why it matters
- cleanest calibration-under-drift paper in the set
- useful for maintaining ranking reliability as market context changes

3. Exact mechanism to steal
- weighted conformal predictive systems using density-ratio weights
- key form:
  - `w_i(x_test) propto p_test(x_i) / p_train(x_i)`
- transfer idea:
  - maintain regime-conditional calibration buffers
  - reweight calibration examples toward current structural node context
- keeps path probability outputs better calibrated under covariate shift

4. Suggested repo target
- CatBoost path ranking target

5. Risk / mismatch
- requires decent density-ratio estimation
- calibration only; not a full delayed-reward learning mechanism

### 8. Self-Calibrating Conformal Prediction

1. Citation
- title: Self-Calibrating Conformal Prediction
- authors: Lars van der Laan, Ahmed M. Alaa
- venue: NeurIPS
- year: 2024
- link: https://arxiv.org/abs/2402.07307

2. Why it matters
- strongest immediate calibration layer for ranker outputs
- gives probability calibration plus valid uncertainty wrappers

3. Exact mechanism to steal
- first calibrate scores into probabilities using Venn-Abers style calibration
- then wrap with conformal prediction to obtain uncertainty-aware outputs
- repo transfer:
  - CatBoost raw score -> calibrated path probability
  - decision gate can consume lower confidence bound rather than raw score
- useful when positives are sparse and execution decisions need reliability thresholds

4. Suggested repo target
- CatBoost path ranking target

5. Risk / mismatch
- handles reliability of outputs, not policy-selection bias from non-executed paths
- requires a decent held-out / rolling calibration scheme

### 9. Counterfactual contextual bandit for recommendation under delayed feedback

1. Citation
- title: Counterfactual contextual bandit for recommendation under delayed feedback
- authors: Ruichu Cai, Ruming Lu, Wei Chen, Zhifeng Hao
- venue: Neural Computing and Applications
- year: 2024
- link: DOI https://doi.org/10.1007/s00521-024-09800-0

2. Why it matters
- one of the best delayed-feedback analogues for candidate path ranking
- maps well to recommended path now, realized outcome later

3. Exact mechanism to steal
- contextual bandit with delayed feedback and counterfactual correction
- usable template:
  - keep pending rewards separate from observed rewards
  - estimate reward under counterfactual exposure model when reward is delayed or unobserved
- doubly robust style correction can be adapted if repo logs path-selection propensities

4. Suggested repo target
- CatBoost path ranking target

5. Risk / mismatch
- recommender framing, not trading-specific
- usefulness depends on how much path exposure / propensity logging the repo can support

### 10. Debiased Off-Policy Evaluation for Recommendation Systems

1. Citation
- title: Debiased Off-Policy Evaluation for Recommendation Systems
- authors: Yusuke Narita, Shota Yasui, Kohei Yata
- venue: RecSys
- year: 2021
- link: DOI https://doi.org/10.1145/3460231.3474231

2. Why it matters
- strongest paper for correcting selection bias when recommendations are not always followed
- highly relevant to path-ranking evaluation and live feedback attribution

3. Exact mechanism to steal
- inverse propensity scoring and doubly robust correction:
  - `V_IPS = (1/n) sum [pi_e(a|x) / pi_b(a|x)] * r`
  - `V_DR = (1/n) sum q(x, pi_e) + [pi_e(a|x) / pi_b(a|x)] * (r - q(x,a))`
- direct transfer:
  - do not treat non-executed paths as plain negatives
  - estimate executed-policy bias before updating ranker or validating policy changes

4. Suggested repo target
- live feedback posterior update

5. Risk / mismatch
- needs policy logging or good propensity approximation
- focuses on evaluation/debiasing, not latent state modeling

### 11. Off-Policy Evaluation for Human Feedback

1. Citation
- title: Off-Policy Evaluation for Human Feedback
- authors: Min Chi, Juncheng Dong, Ge Gao, Qitong Gao, Miroslav Pajic
- venue: NeurIPS
- year: 2023
- link: DOI https://doi.org/10.52202/075280-0398

2. Why it matters
- best fit for repo requirement that live user feedback and realized outcomes belong in one belief chain
- extends OPE logic into feedback-bearing environments

3. Exact mechanism to steal
- learn/evaluate target policy from human feedback logs with policy mismatch correction
- repo transfer:
  - treat user compliance / rejection / qualitative feedback as structured reward side-channel
  - fold that into path outcome scoring through reward models plus off-policy correction
- best when reward is not pure PnL but composite execution outcome

4. Suggested repo target
- live feedback posterior update

5. Risk / mismatch
- human-feedback framing needs translation into trading outcome taxonomy
- less useful if live feedback remains thin and unstructured

### 12. Optimal execution in high-frequency trading with Bayesian learning

1. Citation
- title: Optimal execution in high-frequency trading with Bayesian learning
- authors: Bian Du, Hongliang Zhu, Jingdong Zhao
- venue: Physica A
- year: 2016
- link: DOI https://doi.org/10.1016/j.physa.2016.06.021

2. Why it matters
- one of the few direct bridges between Bayesian learning and execution policy adaptation in markets
- keeps belief update coupled to action, not isolated in analysis layer

3. Exact mechanism to steal
- sequentially update latent market belief, then adapt execution policy accordingly
- direct transfer:
  - trigger / confirmation observations update posterior state
  - posterior state influences executable path preference or aggression
- this is useful for keeping structural belief connected to executable path choice

4. Suggested repo target
- BBN node/branch posterior update

5. Risk / mismatch
- more optimal-control flavored than repo may need
- likely requires discretization and simplification for current architecture

---

## Practical takeaways for `ict-engine`

### Structural-state kernel
- prefer HSMM/HHMM style latent state hierarchy
- use explicit node duration and multi-timescale decomposition

### Offline prior seeding
- convert source weighting into likelihood tempering / pseudo-likelihood power
- maintain source-specific posterior panels before canonical aggregation

### Live posterior updating
- use Beta/Dirichlet pseudo-count surfaces with fractional weights
- update branch transition counts with discounted sufficient statistics

### Ranker calibration
- separate three concerns:
  1. debias evaluation under partial execution
  2. model delayed feedback
  3. calibrate output probability under drift

### Suggested minimum viable theory stack
1. HSMM for `structural_prior_state`
2. tempered pseudo-likelihood for offline source weighting
3. Beta-Binomial fractional pseudo-counts for live path outcomes
4. DR / IPS correction for non-executed recommendations
5. Venn-Abers + conformal for path ranking reliability
