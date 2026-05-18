# Logic-family layered CPT plan

现状：
- 主网络已能加载全局 Tomac CPT init。
- 现有证据面仍不直接携带 `entry_logic_id` 到 Rust inference surface。
- 因此“运行时按 logic_family 切子 CPT”尚无自然入口，不宜硬塞进 `build_trading_network()` 的全局默认路径。

## 已完成的前置条件
- 逻辑映射表：`state/policy_training/tomac_entry_logic_map.csv`
- BBN evidence v2：`state/policy_training/tomac_bbn_evidence_v2.csv`
- logic/family outcome 统计：`repo_bbn_trading_cpt_init_smoothed_summary.txt`

## 当前最稳分层方案
先分两层：
1. 全局默认 CPT
   - 由 `repo_bbn_trading_cpt_init_smoothed.json` 提供
2. 家族特化 CPT（离线产物）
   - 先生成每个 `logic_family` 的 CPT seed
   - 供未来在 evidence 携带 `entry_logic_id` / `logic_family` 时选择性覆写

## 运行时选择点建议
最佳挂点不在 topology builder，而在：
- `trade_evidence_from_pre_bayes_filter()` 之前，或
- higher-level orchestration/policy engine 中

原因：
- `build_trading_network()` 仅建全局结构；此时没有 trade-specific `logic_family`
- `logic_family` 是逐笔/逐策略实例级信息，应在 evidence 已知后再决定是否切换子 CPT

## 推荐未来接口
- `build_trading_network()` -> global smoothed CPT
- `build_trading_network_for_logic_family(family: &str)` -> global + family overlay
- 或 `apply_trading_cpt_family_overlay(network, family)`

## 下一步真正要做的事
1. 在 repo 某个可持续面引入 `entry_logic_id` / `logic_family`
2. 生成 family overlay JSON
3. 用 overlay 覆盖 `entry_quality` / `trade_outcome` 的 CPT 行
4. 为 bull/bear/family 组合做 regression tests
