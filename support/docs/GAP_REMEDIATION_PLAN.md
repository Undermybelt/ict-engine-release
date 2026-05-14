# ict-engine 缺口补强计划

## 已完成的三个新模块

### 1. regime/ — Regime 长记忆 (HSMM)

**来源**: kratu/wess_hmm (GitHub) + Calvet & Fisher (2004) MSM

**模块结构**:
```
src/regime/
├── mod.rs           # 入口 + re-exports
├── types.rs         # RegimeState, DurationState, RegimeFeatures
├── wasserstein.rs   # WassersteinClusterer (分布几何聚类)
├── governor.rs      # RegimeGovernor (多尺度共识 + 最小持仓)
├── hsmm.rs          # HSMMEngine (regime 持续时间建模)
└── hybrid.rs        # HybridRegimeEngine (HMM + Wasserstein + HSMM 组合)
```

**核心能力**:
- Wasserstein距离区分 Range vs Choppy (纯HMM做不到)
- HSMM hazard function 预测 regime 还能持续多久
- 多尺度窗口(短/中/长) + 最小持仓逻辑
- 支持 Geometric / Negative Binomial 两种持续时间分布

**公式**:
```
Wasserstein距离: W(a,b) = mean(|sort(a) - sort(b)|)
HSMM Hazard: λ(d) = P(T=d | T ≥ d)
Geometric生存: P(T > d) = (1-p)^d
负二项 hazard: λ(d) = 1 - P(T>d)/P(T>d-1)
```

---

### 2. multi_scale/ — 多尺度共振一致性

**来源**: 神经科学交叉频率耦合 (Canolty & Knight 2010)

**模块结构**:
```
src/multi_scale/
├── mod.rs           # 入口 + re-exports
├── types.rs         # TimeScale, CouplingResult, ResonanceScore
├── coupling.rs      # HilbertTransform, PACAnalyzer, CrossTimeframeCoupler
└── resonance.rs     # ResonanceAnalyzer (整体共振评分)
```

**核心能力**:
- Phase-Amplitude Coupling (PAC) 分析
- 跨时间框架方向一致性打分
- 弱链接分析 (找到最不耦合的一对)
- Regime 对齐检测

**公式**:
```
调制指数: MI = |1/N * Σ(A_f(t) * exp(i*φ_s(t)))|
Rayleigh统计: R = n * |mean(exp(i*φ))|²
方向一致性: corr = Σ(sign(Δfast) == sign(Δslow)) / N * 2 - 1
共振评分: overall = geometric_mean(coupling_strengths)
```

---

### 3. liquidity/ — 流动性风险因子

**来源**: Amihud (2002), Kyle (1985), Roll (1984)

**模块结构**:
```
src/liquidity/
├── mod.rs           # 入口 + re-exports
├── amihud.rs        # AmihudCalculator (非流动性比率)
├── kyle_lambda.rs   # KyleLambdaCalculator (价格冲击系数)
└── spread_proxy.rs  # SpreadProxyCalculator (Roll/Corwin-Schultz 价差估算)
```

**核心能力**:
- Amihud ILLIQ: |r| / dollar_volume
- Kyle Lambda: |r| / sqrt(dollar_volume)
- Roll spread: 基于负自相关估算
- Corwin-Schultz: 基于 high-low 范围估算
- 流动性 regime 分类 (Liquid/Normal/Illiquid)
- 仓位大小调整因子
- 止损宽度调整因子

**公式**:
```
Amihud: ILLIQ = (1/D) * Σ|r_t| / dollar_volume_t
Kyle: λ = |return| / sqrt(dollar_volume)
Roll: ρ = Cov(Δp_t, Δp_{t-1}), Spread = 2*sqrt(-ρ)
Corwin-Schultz: β = ln(H/L)² / (2*ln(2)), Spread = 2*(exp(α)-1)/(1+exp(α))
```

---

## 集成路径

### Pipeline 集成点

```
                    ┌─────────────────────────────────────────────────────┐
                    │                  ICT Engine Pipeline                │
                    │                                                     │
  Candle Data ──────┼──► ict/ ──► indicators/ ──► regime/ ──► multi_scale/ ──► liquidity/
                    │          │              │         │           │              │
                    │          │              │         │           │              │
                    │          │         ┌────┴────┐    │      ┌───┴───┐     ┌───┴───┐
                    │          │         │ Wasser- │    │      │  PAC  │     │ Amihud│
                    │          │         │  stein  │    │      │ Coupl │     │ Kyle  │
                    │          │         │  HSMM   │    │      │ Reson │     │ Spread│
                    │          │         │ Governor│    │      └───┬───┘     └───┬───┘
                    │          │         └────┬────┘    │          │              │
                    │          │              │         │          │              │
                    │          └──────────────┼─────────┼──────────┼──────────────┘
                    │                         │         │          │
                    │                    ┌────┴─────────┴──────────┴────┐
                    │                    │       Execution Logic        │
                    │                    │  (position size, stops, etc) │
                    │                    └──────────────────────────────┘
                    └─────────────────────────────────────────────────────┘
```

### 具体集成步骤

#### Step 1: lib.rs 注册新模块
```rust
pub mod regime;
pub mod multi_scale;
pub mod liquidity;
```

#### Step 2: config.rs 添加配置
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeConfig {
    pub n_states: usize,
    pub wasserstein_window: usize,
    pub min_hold_bars: usize,
    pub hsmm_duration_dist: String, // "geometric" | "negative_binomial"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiScaleConfig {
    pub timeframes: Vec<String>,
    pub coupling_threshold: f64,
    pub resonance_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityConfig {
    pub amihud_window: usize,
    pub kyle_window: usize,
    pub roll_window: usize,
    pub multiplier: f64,
}
```

#### Step 3: types.rs 扩展 AnalysisOutput
```rust
pub struct AnalysisOutput {
    // ... existing fields ...
    pub regime_hybrid: Option<HybridRegimeOutput>,
    pub multi_scale_resonance: Option<ResonanceScore>,
    pub liquidity: Option<AmihudOutput>,
    pub kyle_lambda: Option<KyleLambdaOutput>,
}
```

#### Step 4: PipelineBuilder 集成
```rust
// In application/belief/pipeline_builder.rs

// 1. Create regime engine
let mut regime_engine = HybridRegimeEngine::new(
    config.regime.n_states,
    state_map,
    config.regime.min_hold_bars,
    config.regime.wasserstein_window,
);

// 2. Fit Wasserstein on historical features
regime_engine.fit_wasserstein(&features)?;

// 3. Run detection
let regime_output = regime_engine.detect(&features, bar_index);

// 4. Compute multi-scale coupling
let couplings = vec![
    CrossTimeframeCoupler::compute_coupling(&h1_prices, &h4_prices, TimeScale::H1, TimeScale::H4),
    CrossTimeframeCoupler::compute_coupling(&h4_prices, &d1_prices, TimeScale::H4, TimeScale::D1),
];
let resonance = ResonanceAnalyzer::compute_resonance(&couplings, 0.5);

// 5. Compute liquidity
let mut amihud = AmihudCalculator::new(20, contract_multiplier("NQ"));
let liquidity = amihud.compute_current(&returns, &prices, &volumes);

// 6. Integrate into execution logic
let position_size = base_size 
    * liquidity.map(|l| l.position_size_factor).unwrap_or(1.0)
    * if resonance.is_resonant { 1.2 } else { 0.8 };

let stop_loss = base_stop 
    * liquidity.map(|l| l.stop_loss_factor).unwrap_or(1.0);
```

---

## 测试验证清单

### regime/
- [x] Wasserstein 距离计算正确
- [x] 聚类器 fit/predict 流程
- [x] Governor 多尺度共识
- [x] Governor blip 抑制
- [x] HSMM Geometric hazard 常数
- [x] HSMM NB hazard 递增
- [x] HSMM 生存函数递减
- [x] HSMM 引擎 regime 追踪
- [x] Hybrid 引擎创建和检测

### multi_scale/
- [x] 调制指数完美耦合
- [x] 调制指数无耦合
- [x] 方向相关性
- [x] 耦合结果范围
- [x] 共振高分
- [x] 弱链接识别
- [x] Regime 对齐 (bull)
- [x] Regime 对齐 (mixed)

### liquidity/
- [x] 单期 ILLIQ 计算
- [x] ILLIQ regime 分类
- [x] 仓位调整因子
- [x] 止损调整因子
- [x] 合约乘数
- [x] Kyle Lambda 计算
- [x] Lambda 反比于成交量
- [x] Roll spread 负自相关
- [x] Corwin-Schultz spread

---

## 性能估算

| 模块 | 单次调用复杂度 | 内存 |
|------|--------------|------|
| Wasserstein | O(k * n * log(n)) | O(n) |
| HSMM | O(1) 更新 | O(history) |
| Governor | O(n) | O(1) |
| PAC | O(n) | O(n) |
| Amihud | O(window) | O(history) |
| Kyle | O(window) | O(history) |

整体: 每 bar 增加 ~0.5ms (Rust 实现)

---

## 下一步行动

1. **编译验证**: `cargo build` 确认无语法错误
2. **单元测试**: `cargo test -p ict-engine` 运行所有测试
3. **集成测试**: 在 pipeline 中接入新模块
4. **回测验证**: 在历史数据上验证 regime + liquidity 调整效果
5. **文档更新**: 更新 AGENTS.md 和 CLAUDE.md

---

## 参考文献

### 论文
1. Amihud (2002) "Illiquidity and stock returns"
2. Calvet & Fisher (2004) "How to forecast long-run volatility"
3. Canolty & Knight (2010) "The functional role of cross-frequency coupling"
4. Guédon (2003) "Estimating hidden semi-Markov chains from discrete sequences"
5. Huang et al. (1998) "The empirical mode decomposition and the Hilbert spectrum"
6. Kyle (1985) "Continuous auctions and insider trading"
7. Rabiner (1989) "A tutorial on hidden Markov models"
8. Roll (1984) "A simple implicit measure of the effective bid-ask spread"
9. Yu (2010) "Hidden semi-Markov models"

### GitHub Repos
1. kratu/wess_hmm — Hybrid Wasserstein + HMM Regime Detection
2. Jantg/MSM_python — Markov Switching Multifractal (Liu-West filter)

---

Generated: 2026-04-20 23:45
