# Ambiguity Audit — RAmmStein (2602.19419v2)

## Specified items
- OU SDE: dS = θ(μ-S)dt + σdW [SPECIFIED] §III-E, Eq.10
- State vector components (8-dim) [SPECIFIED] §IV-B
- DDQN architecture (2-layer, 128 hidden) [SPECIFIED] §VI-A
- Learning rate 1e-4, γ=0.99, batch=64 [SPECIFIED] §VI-A
- Replay buffer 100k [SPECIFIED] §VI-A
- Target network update every 1000 steps [SPECIFIED] §VI-A
- Reward = fee_accrued - rebalance_cost [SPECIFIED] §IV-C
- ε-greedy: 1.0→0.01 over 50k steps [SPECIFIED] §VI-A

## Partially specified
- OU estimation window [PARTIALLY_SPECIFIED] "rolling window" but size unspecified — §III-E
- Action space: {wait=0, rebalance=1} [SPECIFIED] §IV-C
- Price range width λ [PARTIALLY_SPECIFIED] "user-defined" — §IV-B

## Unspecified
- Exact activation function in DDQN hidden layers [UNSPECIFIED]
- Weight initialization scheme [UNSPECIFIED]
- State normalization method beyond clipping [UNSPECIFIED]
- How θ is truncated to [0,1] (exact formula) [UNSPECIFIED]
- Training episode length [UNSPECIFIED]
- How OU params are re-estimated during training [UNSPECIFIED]
