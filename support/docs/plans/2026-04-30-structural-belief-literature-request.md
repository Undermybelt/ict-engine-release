# Structural Belief Literature Request

Date: 2026-04-30
Status: requested
Scope: external literature request for the next stage of ict-engine structural belief learning

## Current learning stage

The repo is no longer at "display-only structural output".

Current state:

1. Structural consumer protocol exists
- node
- branch
- scenario
- path
- trigger / confirmation / stop / invalidation

2. Live feedback loop exists
- user-followed path outcomes can persist
- structural refs map back to `node_id / branch_id / scenario_id / path_id`

3. Structural prior state exists
- `learning_state.structural_prior_state`
- live feedback updates it
- analyze / research / backtest / mutation / artifact validation can seed it

4. Source weighting exists
- live feedback > artifact validation > backtest > research > mutation > analyze

5. Offline seed calibration exists
- uses aggregate return, execution readiness, score deltas, mutation acceptance, and some break / coverage metrics

## What is still missing

The repo is not yet at the target system:

1. Stable canonical market-structure anchor across downstream phases
- analyze may identify a market structure regime
- research/backtest can still collapse into workflow-phase labels like `research_iteration` / `backtest:actionable`
- we need a cleaner theory for persistent node identity

2. Formal belief update math
- current source weighting and support calibration are heuristic
- we need principled pseudo-count / posterior update rules

3. Better offline-to-online fusion
- current offline signals and live outcomes enter the same prior chain
- but not yet through a rigorously justified weighting framework

4. Path-ranking training target
- CatBoost should eventually rank candidate paths inside structural constraints
- we need literature on how to define and calibrate that target under sparse, delayed outcomes

## What we need literature for

We need papers that help answer:

1. How should structural state be modeled?
- hierarchical node -> branch -> scenario -> path
- dynamic latent-state or regime graph
- sparse observations, noisy labels, path-dependent outcomes

2. How should priors be initialized from offline evidence?
- backtests
- research scorecards
- mutation evaluation
- artifact validation summaries

3. How should posteriors be updated from live outcomes?
- user follows / does not follow
- win / loss / breakeven / invalidated / abandoned
- delayed reward and sparse path counts

4. How should different evidence sources be weighted?
- direct realized outcome
- offline backtest
- research metrics
- mutation acceptance / rejection
- consumed-artifact validation

5. How should path rankers be calibrated under structural constraints?
- CatBoost or similar ranker operates only on declared candidate paths
- not freeform market prediction

## Highest-priority paper buckets

### A. Dynamic Bayesian / hierarchical belief updates

Need:
- dynamic Bayesian networks
- hierarchical Bayesian models
- hidden semi-Markov / switching-state models
- regime-transition posterior updates

Why:
- likely best foundation for `node / branch / scenario / path` belief propagation

### B. Reliability-weighted pseudo-count updating

Need:
- Beta-Binomial / Dirichlet-multinomial style updates
- source-reliability weighting
- posterior tempering
- Bayesian model averaging with heterogeneous evidence quality

Why:
- directly relevant to offline seed + live feedback fusion

### C. Online probability calibration under drift

Need:
- online calibration
- conformal or post-hoc calibration under distribution shift
- concept drift and probability reliability maintenance

Why:
- structural priors and path rankers will drift as market regimes change

### D. Sequential decision / ranking under delayed outcomes

Need:
- contextual bandits or ranking under delayed reward
- off-policy evaluation where actions are recommended but not always executed
- ranking calibration under sparse positives

Why:
- user may or may not follow recommended paths
- need proper treatment of partial compliance

### E. Market regime / structure modeling for execution systems

Need:
- papers that explicitly separate regime node, transition branch, execution policy
- not just raw return prediction

Why:
- closer to desired system shape than generic forecasting papers

## Lower-priority buckets

- graph neural nets for market state graphs
- Bayesian reinforcement learning
- meta-learning for few-shot path adaptation

Useful, but not first-pass must-have.

## What to exclude

Please deprioritize papers that are mostly:

- pure next-return regression
- black-box direction prediction without explicit state structure
- LLM-for-trading marketing papers
- agentic finance demos with no update math
- option pricing papers unrelated to belief updating

## Deliverable format

Please return at most 12 papers.

For each paper, include:

1. Citation
- title
- authors
- venue
- year
- arXiv / SSRN / DOI link

2. Why it matters to ict-engine
- which missing problem it addresses

3. What exact mechanism to steal
- formula
- update rule
- weighting scheme
- state-transition idea
- calibration method

4. Suggested repo target
- `structural_prior_state`
- BBN node/branch posterior update
- CatBoost path ranking target
- artifact-validation prior source
- live feedback posterior update

5. Risk / mismatch
- what part does not fit our architecture

## Search query hints

Suggested query themes:

- "dynamic bayesian network regime switching online update"
- "beta binomial reliability weighted pseudo counts"
- "hierarchical bayesian sequential decision delayed reward"
- "online probability calibration concept drift finance"
- "market regime transition bayesian execution policy"
- "off policy evaluation delayed feedback ranking"
- "bayesian evidence weighting heterogeneous data sources"
- "hidden semi markov financial regime transition posterior"

## Preferred outcome

Best-case result is not "many papers".

Best-case result is:

- 3-5 papers that directly justify offline prior seeding
- 2-3 papers that justify live posterior updating with sparse path feedback
- 2-3 papers that justify path ranking and calibration under non-executed recommendations

That is enough to move the repo from heuristic calibration to a more principled belief-learning architecture.
