# Tomac Entry Logic Lexicon

目的：先定义每一单背后之入场逻辑，再为 BBN 节点命名与执行树分叉铺垫。

## 原则
1. 节点先以“入场逻辑”命名，不先以结果命名。
2. 长短方向可共属一逻辑族，必要时再拆 long/short 子节点。
3. BBN 节点名应用抽象稳定语义；执行树节点可贴近策略分支。
4. `source_file -> entry_logic_id` 必须先固定，再谈监督学习或 belief update。

## 推荐命名层级
- logic_family: 大类
- entry_logic_id: 稳定逻辑 ID
- logic_node_seed: 给 BBN / execution tree 的命名种子
- long/short_label: 方向子面

格式建议：
- BBN 观测/证据节点：`entry.<family>.<logic>`
- 执行树分支：`<logic>_branch`

## 已识别逻辑族

### 1. divergence_sweep
- 代表文件：`70wr2.49rrr5.84pf62.93e.py`
- 核心：流动性扫损 + MACD 背离 + RSI 过滤
- long: `long_divergence_ssl_sweep`
- short: `short_divergence_bsl_sweep`
- BBN seed: `entry.divergence.liquidity_sweep`

### 2. ict_sfp
- 代表文件：`77wr6.99pf2rrr.py`
- 核心：ICT Bullish/Bearish SFP typed setups
- long: `ict_bullish_sfp`
- short: `ict_bearish_sfp`
- BBN seed: `entry.sfp.ict`

### 3. contextual_turtle_soup
- 代表文件：`85wr12.9pf2.21rrr.py`
- 核心：sweep high/low 后，再看三种上下文确认：
  - `SFP_Trend_Continuation`
  - `SFP_Mom_Reversal`
  - `SFP_Vol_Expansion`
- 这是最像 BBN 上游证据分叉的逻辑族。
- BBN 可拆成：
  - `sweep_side`
  - `continuation_context`
  - `momentum_reversal_context`
  - `volatility_expansion_context`
- seed: `entry.sfp.contextual_turtle_soup`

### 4. ema_volume_sweep
- 代表文件：`90wr1.5rrr_fast.py`, `90wr1.5rrr_v2.py`
- 核心：EMA 趋势偏置 + 成交量过滤 + PDH/PDL 或分形扫损 + WPR 双极值
- seed: `entry.ema_volume.sweep`

### 5. strict_wpr_rsi_sweep
- 代表文件：`90wr1.5rrr_final.py`
- 核心：WPR + RSI 极值 + 流动性扫损 + volume + low-vol + EMA 趋势
- seed: `entry.strict.wpr_rsi_sweep`

### 6. confluence_reversal
- 代表文件：`90wr1.5rrr_v3.py`
- 核心：多因子共振反转，后接保本与追踪止损
- seed: `entry.confluence.reversal`

### 7. multi_confirm_sweep_reversal
- 代表文件：`90wr1.5rrr_strategy.py`
- 核心：PDL/SSL 或 PDH/BSL sweep + WPR extreme + hour open + volume + ADX
- seed: `entry.multi_confirm.sweep_reversal`

### 8. ict_ote_liquidity_sweep
- 代表文件：`ict_90wr_1.5rrr_strategy.py`
- 核心：liquidity sweep + OTE zone + near FVG/OB + bias
- 最适合挂到 `liquidity_context` 与 `entry_quality` 之间
- seed: `entry.ict.ote_liquidity_sweep`

### 9. ict_zone_reversal
- 代表文件：`final_ict_perfect.py`
- 核心：sweep + extreme WPR + reclaim + FVG/OB zone + volume
- seed: `entry.perfect.ict_zone_reversal`

### 10. ict_sniper_sweep
- 代表文件：`92w0.5rrrr5.54pf.py`
- 核心：PDH/PDL + pool sweep + silver bullet or RSI + candle confirm
- seed: `entry.sniper.ict_sweep`

### 11. purified_wpr_sweep
- 代表文件：`98wr0.8rrr41.07pf.py`
- 核心：purified data + WPR sweep family
- seed: `entry.purified.wpr_sweep`

### 12. no_be / optimal_be / balanced / pro / validation variants
- 这些更像同一 sweep-reversal 家族的参数化变体。
- 可暂归为二层：
  - family: `parameterized_liquidity_reversal`
  - variant node: `no_be`, `optimal_be`, `balanced`, `pro`, `validation`

## 现成映射文件
见：
- `state/policy_training/tomac_entry_logic_map.csv`

该表已含：
- `source_file`
- `entry_logic_id`
- `logic_family`
- `logic_node_seed`
- `source_py`
- `primary_functions`
- `logic_signature`
- `bbn_hint_node`
- `execution_tree_hint`

## 下一步建议
1. 把 `entry_logic_id` 合入 BBN evidence builder。
2. 先将 `entry_logic_id` 视作 observed/proxy node。
3. 再从 `logic_family` 派生执行树第一层分叉。
4. 对 `85wr` 这类 context-rich 逻辑，拆细子节点，不要只挂一层粗标签。
