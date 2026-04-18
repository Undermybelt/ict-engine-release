# Evidence Quality / Bridge Gap Optimization Experiment

目标：不再继续低价值的 `structure_ict` 局部参数细扫，转而针对真正的瓶颈：
- `evidence_quality_score`
- `pre_bayes_gate_status`
- `pre_bayes_bridge_probability_gap`
做定向实验。

## 已知事实

1. `structure_ict` 参数面在 `expansion_manipulation` 下已接近局部最优：
- baseline defaults (`lookback=20`, `expansion_threshold=1.5`) 最优
- 25 个 isolated runs 无一超过 baseline

2. `evaluate_expansion_preview=true` 激活后，score 从 0.3678 → 0.6908 (+88%)
- expansion metrics 现在有效
- 但 baseline 仍最优

3. 当前真正瓶颈：
- `objective_market_shrink_weight = 0.55`（硬压分）
- `objective_market_credibility_score ≈ 0.524`
- `pre_bayes_bridge_gap_score ≈ 0.029`
- `pre_bayes_gate_status = pass_neutralized`（不是 pass_hard）

## 核心策略

不再主攻 structure_ict 参数，而是直接攻击：

### A. Evidence Quality Score
公式：
```text
evidence_quality_score =
  0.55
  + support_gap.min(0.5) * 0.50
  - uncertainty * 0.35
  + 0.15 if !directional_conflict
  - directional_conflict_penalty (0.20)
  - mixed_alignment_penalty (0.10)
  - multi_timeframe_direction_conflict_penalty (0.18)
  - multi_timeframe_alignment_penalty (0.10)
  + multi_timeframe_alignment_bonus (0.05)
  - multi_timeframe_entry_penalty (0.08)
  - hostile_liquidity_penalty (0.10)
  + favorable_liquidity_bonus (0.05)
```

当前 `hard_pass_quality_threshold = 0.75`，要从 0.524 提到 0.75+ 才能拿到 `pass_hard`。

### B. Bridge Gap
当前 `pre_bayes_bridge_probability_gap ≈ 0.007`，几乎没有信号分离。
桥差分是 long/short signal probability gap，需要：
- 更强的方向性分离
- 更小的 uncertainty

## 实验设计：三条平行路径

### Path 1: Policy Tuning (最可行，低风险)

不改 factor 本体，先调 PreBayes policy 环境变量，看看分数是否显著提高。

要测试的 env vars：
- `ICT_ENGINE_PREBAYES_HARD_PASS_QUALITY_THRESHOLD` (0.75 → 0.70 / 0.65 / 0.60)
- `ICT_ENGINE_PREBAYES_DIRECTIONAL_CONFLICT_PENALTY` (0.20 → 0.15 / 0.10)
- `ICT_ENGINE_PREBAYES_MIXED_ALIGNMENT_PENALTY` (0.10 → 0.05)
- `ICT_ENGINE_PREBAYES_MTF_DIRECTION_CONFLICT_PENALTY` (0.18 → 0.12 / 0.08)
- `ICT_ENGINE_PREBAYES_MTF_ALIGNMENT_PENALTY` (0.10 → 0.05)
- `ICT_ENGINE_PREBAYES_MTF_ALIGNMENT_BONUS` (0.05 → 0.08 / 0.10)
- `ICT_ENGINE_PREBAYES_HOSTILE_LIQUIDITY_PENALTY` (0.10 → 0.05)

目的：
- 看 `gate_status` 能否从 `pass_neutralized` → `pass_hard`
- 看 `objective_market_shrink_weight` 是否从 0.55 抬高
- 看 `composite_score` 是否 > 0.6908

### Path 2: StructureIct Bridge-Sensitive Param Sweep

固定 `evaluate_expansion_preview=true`，只扫最可能影响 bridge_gap 的参数：
- `sweep_weight`
- `unconfirmed_sweep_weight`
- `opposing_sweep_penalty`
- `post_sweep_displacement_weight`
- `sweep_atr_multiplier`

重点不是 composite，而是：
- `pre_bayes_bridge_probability_gap`
- `pre_bayes_gate_status`
- `objective_market_shrink_weight`

### Path 3: Cross-Market SMT Bridge Experiment

用 `cross_market_smt` 作为主因子，探索是否能直接改善 bridge gap：
- `lookback` 轴：[10, 20, 30, 40]
- `evaluate_expansion_preview=true`

目的：
- 看 `cross_market_smt` 是否更容易提高 `support_gap`
- 看它是否能给 `evidence_quality_score` 更高的基础

---

## 执行顺序

1. Path 1（policy tuning）
2. 若 policy tuning 可提升 score ≥ +0.03，则优先走 policy 方向
3. 否则跑 Path 2 和 Path 3 比较：
   - 谁更能抬高 bridge_gap / gate_status / shrink_weight
4. 最后做综合推荐

---

## 需要的脚本

### scripts/pre_bayes_policy_tuning.py
- 每个 run 独立 state_dir
- 用 baseline defaults + `evaluate_expansion_preview=true`
- 设置 env var 组合
- 记录：
  - score_after
  - gate_status
  - bridge_gap
  - objective_market_shrink_weight
  - selected_win_probability

### scripts/bridge_gap_param_sweep.py
- 扫 5 个 bridge-sensitive params
- 每个 run isolated
- 输出 bridge_gap 排名表

### scripts/cross_market_smt_bridge_experiment.py
- base_factor=`cross_market_smt`
- 扫 lookback
- 比较 bridge_gap / gate / score

### scripts/evidence_bridge_summary.py
- 汇总三条路径的最佳结果
- 输出最优方向

---

## 成功标准

任一方向满足以下之一即算成功：
1. `score_after > 0.72`
2. `gate_status = pass_hard`
3. `objective_market_shrink_weight > 0.70`
4. `pre_bayes_bridge_probability_gap > 0.05`

---

## 背景命令模板

建议用 `/background` 跑：
- `python3 scripts/pre_bayes_policy_tuning.py`
- `python3 scripts/bridge_gap_param_sweep.py`
- `python3 scripts/cross_market_smt_bridge_experiment.py`
- `python3 scripts/evidence_bridge_summary.py`
