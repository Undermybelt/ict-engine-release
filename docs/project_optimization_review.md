# Project Optimization Review

## Summary

当前项目最强的是实验能力，最弱的是实验收口能力。

已有强项：
- `factor-research`
- `factor-autoresearch`
- `factor-autoresearch-status`
- 多批实验脚本
- live snapshot / final summary / attempts ledger
- cluster jump / cycle / 多 family 轮转
- mutation-spec 输入校验

核心短板：
- 缺少统一的“研究真相收口面”
- 用户仍需手工拼接 local search / cluster run / status / artifacts 才能下结论

## 1. 最大问题：研究系统已长成，但评估真相仍分散

现状：
- 有丰富命令、状态文件、实验脚本
- 但“什么该继续、什么该停、什么已证伪”仍靠人工判断

优化建议：
1. 做统一 `research-verdict` 面
   - 输入：
     - local search results
     - cluster runs
     - autoresearch status
     - best_attempt
     - final artifact
   - 输出：
     - `continue`
     - `pivot`
     - `stop_as_local_optimum`
     - `needs_structural_change`
2. 加“实验污染警报”
   - 若同一批共用 state_dir 且 score 单调漂移
   - 明示 `comparison_contaminated = true`

## 2. 第二大问题：objective / scoring 逻辑仍可能有自洽性缺口

已有问题实例：
1. `expansion_preview` 未开启时，35% 权重 dead/null
2. expansion scoring 曾硬编码 `(20, 1.5)`

优化建议：
1. 给每个 objective 增加评分分解报告
   - 输出每项权重、每项值、每项贡献、dead/null 字段
2. 加 dead-weight 检测
   - 若某 objective 中 >20% 权重来自 null/default
   - 直接输出 `objective_surface_degraded`
3. 把 objective score 的参数来源全显式化
   - mutation spec / hardcoded default / env / latest sample

## 3. PreBayes / bridge / shrink 仍是主瓶颈，且 docs 未完全产品化

已证实的真实瓶颈：
- `evidence_quality_score`
- `pre_bayes_gate_status`
- `objective_market_shrink_weight`
- `bridge_gap`

优化建议：
1. 给这 4 个做专门 debug 命令
   - `pre-bayes-debug`
   - `bridge-gap-debug`
2. 做 `evidence_quality_breakdown`
   - 输出：
     - base
     - support_gap contribution
     - uncertainty penalty
     - directional_conflict penalty
     - mtf penalties/bonus
     - liquidity penalty/bonus

## 4. Feature engineering 面还有空间

当前 Pythagorean + OU 已进 skeleton，但还不到位：
1. 已可见，但未进入 score / gate / reflection
2. 还没做 feature importance / ablation
3. 还没和 market-specific fork 联动

优化建议：
1. 做最小 ablation
   - baseline
   - + distance features
   - + OU features
   - + both
2. 把它们接到 `recommended_actions`
3. 再决定是否进入核心 scoring

## 5. paired-data 质量治理不足

现状：
- `none` / `es` 路线平坦
- `ym` 曾直接崩溃
- 虽已修边界，但 paired 质量本身未治理

优化建议：
1. 加 paired-data admission gate
   - 长度对齐率
   - 时间戳重叠率
   - 最小共同窗口
   - 缺失/错位比例
2. SMT 结果要明确区分：
   - `uninformative`
   - `invalid_due_to_pair_quality`
   - `valid_but_flat`

## 6. 脚本能力强，但脚本治理还不够

问题：
- 脚本数量多，易爆炸
- 用户难知该跑哪个
- 逻辑重复

优化建议：
1. 收敛成三类脚本：
   - `search_local.py`
   - `search_cluster.py`
   - `evaluate_bottleneck.py`
2. 每个脚本统一产物 schema：
   - `results.json`
   - `metadata.json`
   - `summary.txt`
   - `logs/`
3. 加脚本路由 README

## 7. docs 方面最该补的地方

建议新增：
1. `docs/research-system-map.md`
2. `docs/objective-scoring-map.md`
3. `docs/experiment-integrity-rules.md`
4. `docs/feature-intake-rules.md`

## 8. 若只挑“最不到位的一处”

最不到位的是：
- 研究结论还没有被机械化收口。

因此最值得的新增物：
- `research-verdict` 命令

它应综合：
- autoresearch status
- local search results
- cluster scoreboard
- dead-weight warnings
- final artifacts
- contamination flags

最终输出：
- `best_known_baseline`
- `proven_bad_regions`
- `current_bottleneck`
- `recommended_next_experiment`
- `stop_or_continue`

## 结论

项目当前最强的是“实验能力”，最弱的是“实验收口能力”。

因此，最值得优先实现的是：
1. `research-verdict`
2. `evidence-quality-breakdown`
