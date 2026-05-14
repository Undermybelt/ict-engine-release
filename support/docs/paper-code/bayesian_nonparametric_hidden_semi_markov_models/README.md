# Bayesian Nonparametric Hidden Semi-Markov Models

Paper: https://arxiv.org/abs/1203.1365
Title: "Bayesian Nonparametric Hidden Semi-Markov Models"
Authors: Matthew J. Johnson, Alan S. Willsky (2012)

## Core Contribution

Explicit-duration hidden semi-Markov modeling for sequential latent states.

For `ict-engine`, the transferable core is not the full Bayesian nonparametric machinery.
It is this:
- structural states should carry duration priors
- transitions should be estimated separately from dwell-time persistence
- node persistence should not be approximated as purely geometric switching

## Why this matters here

`ict-engine` already has structural node / branch / scenario / path surfaces.
This paper gives the strongest theoretical backing for:
- `node` as latent state
- `branch` as transition row
- duration-aware structural persistence in `structural_prior_state`

## Exact mechanism to steal

Latent segment structure:
- `z_s ~ pi_(z_{s-1})`
- `d_s ~ Dur(theta_{z_s})`
- observation sequence inside segment `s` is emitted under state `z_s`

Transfer to repo:
- `node_prior_mass` <- posterior over `z_s`
- `branch_transition_prior` <- transition matrix `pi`
- `node_duration_prior` <- duration family parameters `theta`

## Suggested repo target
- `structural_prior_state`

## Code conversion scope

Minimal local implementation should include:
- `src/model.py` or Rust-adjacent pseudocode note for explicit-duration filtering
- `configs/base.yaml` for duration prior family and persistence penalty controls
- `REPRODUCTION_NOTES.md` listing which parts are exact vs approximated

## External code / references

No canonical GitHub repo verified in this pass.
If converting later, prefer a narrow local implementation over importing an unvetted research stack.
Search terms to resume with:
- `hdp hsmm github johnson willsksy`
- `pyhsmm explicit duration hsmm`

## Implementation notes for ict-engine

Start narrow:
1. add duration-aware node persistence state
2. add discounted branch transition counts
3. keep scenario/path logic downstream; do not force full nonparametric hierarchy on day one
