# 2026-05-07 Factor Regime Expansion & Runtime Closure

> 目标：扩展 regime 状态粒度，完成因子→运行时闭环，迭代至满意。

---

## 1. Regime 状态体系重构

### 当前问题
- [x] Rust `Regime` 枚举仅 3 类：Accumulation / ManipulationExpansion / Distribution — ICT 经典三分法
- [x] 外部 HMM 支持 n_states，测了 4 状态，但 family 映射只归为 trend/range/transition 三类
- [x] benchmark 显示 transition 最难识别：family_f1 ~0.4

### 已完成
- [x] 扩展 HMM 至 8 状态，跑通 350k+ bars (NQ 15m)
  - State 2: trend_up_strong (16.9%, sharpe=0.20) ← 最强上涨
  - State 0: trend_down_strong (17.0%, sharpe=-0.17) ← 最强下跌
  - State 3: trend_up_weak (11.9%, sharpe=0.15)
  - State 4: trend_down_weak (15.6%, sharpe=-0.13)
  - State 1: range_quiet (16.3%, sharpe=0.13)
  - State 6: range_volatile (11.6%, sharpe=0.06)
  - State 5: transition (0.9%, sharpe=0.02) ← 最稀有
  - State 7: crash_recovery (10.2%, sharpe=-0.06)
- [x] Rust types.rs 新增 `RegimeV2` 8 状态枚举
- [x] Rust types.rs 新增 `RegimeProbsV2` 8 维概率数组
- [x] FactorContext 新增 `regime_v2` / `regime_probs_v2` 字段
- [x] HMM 脚本支持 `--n-states 8`
- [x] 输出 regime_v2_labels.feather 可供因子运行时加载

### 目标状态体系（8-state）

| State ID | Label | Family | ICT 映射 | 因子策略倾向 |
|----------|-------|--------|----------|-------------|
| 0 | Trend_Down_Strong | trend | Distribution 中后期 | 顺势做空，止损宽 |
| 1 | Range_Quiet | range | Accumulation 压缩期 | 网格/反转，高频小仓 |
| 2 | Trend_Up_Strong | trend | Accumulation 后期 | 顺势做多，止损宽 |
| 3 | Trend_Up_Weak | trend | ManipulationExpansion 初期 | 轻仓试多，止损窄 |
| 4 | Trend_Down_Weak | trend | Distribution 初期 | 轻仓试空，止损窄 |
| 5 | Transition | transition | 状态切换点 | 减仓观望，等方向确认 |
| 6 | Range_Volatile | range | ManipulationExpansion 扩张期 | 突破跟随，宽止损 |
| 7 | Crash_Recovery | transition | 极端事件恢复期 | 反转试仓，极窄止损 |

---

## 2. 因子运行时集成

### FactorContext 扩展
- [x] 新增 `regime_v2: Option<RegimeV2>` 字段
- [x] 新增 `regime_probs_v2: Option<RegimeProbsV2>` 字段
- [x] 保持向后兼容（legacy `regime` 字段保留 deprecated 标记）

### 数据加载
- [x] `load_regime_v2_labels()` 从 JSON 加载（改用 JSON 替代 feather）
- [x] 支持项目数据目录 + `/tmp/hmm_regime_*` fallback
- [x] `parse_regime_v2()` 字符串→枚举映射
- [x] 时间戳 → RegimeV2 HashMap 已完成
- [ ] per-bar regime lookup（当前取 first，需改为按时间戳匹配）

### 下一步
- [ ] 验证 cargo build 是否通过
- [ ] 实现完整 regime loader

---

## 3. 因子迭代闭环

### 当前状态
- 外部 Auto-Quant 已有多个策略 pack
- 但因子迭代未形成完整闭环：regime → factor → backtest → ranking → promotion

### 目标闭环流程

```
Regime Detection (HMM/Classifier)
        ↓
Factor Parameter Selection (regime-aware)
        ↓
Backtest (walk-forward with regime tags)
        ↓
IC/IR/Ranking (per-regime metrics)
        ↓
Promotion Decision (regime-stratified)
        ↓
Execution Tree Integration
```

### 实施路径

- [ ] **Step 3.1**：扩展 `BacktestMetrics` 结构
  - 添加 `per_regime_sharpe: HashMap<RegimeV2, f64>`
  - 添加 `per_regime_trade_count: HashMap<RegimeV2, usize>`
  - 添加 `regime_transition_pnl: Vec<(RegimeV2, RegimeV2, f64)>`

- [ ] **Step 3.2**：更新 IC 计算
  - 修改 `src/factors/ic_calculator.rs`
  - 支持 `fn compute_ic_by_regime(trades: &[FactorTrade], regime: RegimeV2) -> f64`
  - 输出 per-regime IC 分布

- [ ] **Step 3.3**：创建因子排名器
  - 路径：`src/factor_lab/ranker.rs`
  - 功能：按 regime 分组排名因子
  - 排名维度：IC, IR, Sharpe, Trade Density
  - 输出：`FactorRanking { regime: RegimeV2, factors: Vec<RankedFactor> }`

- [ ] **Step 3.4**：实现 promotion 逻辑
  - 在 `src/factor_lab/promotion.rs` 添加 regime-stratified 检验
  - 因子需在 ≥2 个 regime 下 IC > 0.02 且 Trade ≥ 30 方可 promotion
  - 避免单一 regime 过拟合

- [ ] **Step 3.5**：集成到 execution tree
  - 在 `src/belief_core/` 添加 regime-aware 分支选择
  - 根据当前 regime 选择最优因子组合
  - 支持 `--regime-aware` CLI flag

---

## 4. 验证与验收

### 数据准备

- [x] 使用 `/Users/thrill3r/Auto-Quant/user_data/data/NQ_USD-15m.feather`
- [x] 运行 8-state HMM 生成 regime labels → `/tmp/hmm_regime_nq_15m_v8/`
- [ ] 对比 4-state vs 8-state 的 transition detection

### 回测验证

- [ ] 比较 regime-aware vs baseline：
  - Sharpe ratio 改善 ≥ 10%
  - Max drawdown 改善 ≥ 5%
  - Trade count 稳定或增加

### 代码验收

- [ ] `cargo test --all` 通过
- [ ] `cargo clippy -- -D warnings` 无新增 warning
- [ ] `cargo fmt --check` 通过
- [ ] 新增 doc comments 覆盖公共 API

---

## 5. Commit 策略

每个 Step 完成后立即 commit：

- [ ] `feat(regime): expand Regime enum to 8 states (v2)`
- [ ] `feat(factor): add regime_v2 to FactorContext`
- [ ] `feat(hmm): support 8-state HMM in external script`
- [ ] `feat(data): add regime loader for factor runtime`
- [ ] `feat(factor): implement regime-aware parameter switching`
- [ ] `feat(backtest): add per-regime metrics`
- [ ] `feat(ranking): add regime-stratified factor ranker`
- [ ] `feat(execution): integrate regime-aware factor selection`

---

## 6. 风险与缓解

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| 8-state HMM 过拟合 | 中 | 高 | 严格 OOS 验证，要求 train/test regime 分布一致 |
| Regime 切换延迟 | 高 | 中 | 使用 forward-looking 惩罚，模拟真实延迟 |
| 因子参数切换频繁 | 低 | 中 | 添加 regime persistence 惩罚，鼓励稳定持仓 |
| 数据对齐失败 | 中 | 高 | 时间戳精确匹配，缺失时使用 nearest-before |

---

## 7. 当前状态

### Completed This Session
1. [x] 扩展 `src/types.rs` 添加 `RegimeV2` 8 状态枚举
2. [x] 运行 8-state HMM 获取新 regime labels
3. [x] 更新 `FactorContext` 添加 regime_v2 字段
4. [x] 添加 regime loader 框架代码
5. [x] HMM 脚本升级支持 8 状态

### Next Action
1. 验证 cargo build 通过
2. 完善 regime loader 的 feather 解析
3. 实现 per-bar regime lookup

### Blockers
- cargo check 较慢（大型项目），需等待编译结果
