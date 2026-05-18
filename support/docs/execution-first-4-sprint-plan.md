# ict-engine：Execution-First 4 Sprint 开发计划

目标
- 把 `ict-engine` 从“预测/分析 CLI”推进为真正的 `stateful research OS`。
- 核心护城河改为 `Execution > Prediction`。
- 保持现有闭环：`分析 -> 研究 -> 回测 -> 回灌 -> artifact ledger -> reflection_bundle -> hard gate`。
- 所有新增能力都必须进入可审计状态面，而非停留在 prompt 或 chat 解释层。

前置判断
- 从 `BTC-Trading-Since-2020` 的分桶结果看，账本原生 execution logic 在多数 bucket 中强于强行 ICT 解释。
- 因此，本计划不把 execution 逻辑硬编码成 ICT 形而上学；而是把 execution 抽象成 first-class factor family，再允许 ICT / OU / Ising / geometry 去解释它。
- 原则：Execution 先成公共决策面，再谈具体解释学。
- 根据 Execution 论文读后更新：本计划不再采用“Prediction System + Execution Features”思路，而改为 `Execution Operating Surface + Prediction as Input`。
- 因而所有 Sprint 的优先级排序统一改为：`execution feasibility -> execution attribution -> prediction value -> regime explanation`。

免费论文 PDF 链接
- Execution 论文
  - https://download.ssrn.com/2026/4/3/6191618.pdf?response-content-disposition=inline&X-Amz-Security-Token=IQoJb3JpZ2luX2VjECQaCXVzLWVhc3QtMSJHMEUCIH3rBgpbYpZcsQ%2F27oFrmdGgMZYhmGTzS8J7VflsWjtvAiEAp13NlaDaVizk4pJB2KalPkym17zaA%2FM25sjlJoAyPsgqxQUI7P%2F%2F%2F%2F%2F%2F%2F%2F%2F%2FARAEGgwzMDg0NzUzMDEyNTciDNBX1WYwtNPewAJ6ayqZBezTJ3DNcUljzYulQFDxotirUdYMCeg85zWI5PFr7U5UKyKiY8bWeXyRv2Fqv%2FncbOMFg8h8ViwnFWMBlllK2MjB0hH2DAV7KH62ecLRvAP8o7jRjCT5kVn%2Bgxvb4NGKDdJrZi%2BZfN%2FTXZ2kx5NMenW2AnQpKVnzI2kRACii7QDJ7afctaSyvCpEFuGc5FmpZUSOcUUDF745QTtHyZetXa11XsyjS%2BaCIoB5XOaBZPQ7cp%2Bq5v%2FdFBAtc4l5gFIDpMydEoyn0eZ8WFQq7T2GH6q9AaTFwdnWhQfr%2Bf9XK9U9xiDPMDOmXMrrlHJzkRo%2Fu7KjH6UJuTmyPDlSYruopG2UtdwuiOPogw6UA%2BVAXup3je1%2FjlklRjIt2Mb5BbAprExqa7WcXouCEJ%2BgDj1Tv4PCVKoxB08Ot2xZnexBlouvGrHS355OPWyjVpPhxbezUzzci1Vstv1NdiyscERIXBwrO1DjqmQCkKDfMziiVxFazsHXtglkBT7ihjxpvMIxXKBQNOhumSht61crP0A5byXKkOie37GOag8LAWtkTQTxs2Hc4gqfCBGxG7hfLOiyPEaS53NCLDmEdVT4m3ysVZUYPaz0K6EWOOkSMXZLGqKNfU8Ul5BNEmVUhMjWFY%2FY8vgYxoehxcyrVeqBa8Y3tNWTlOhfwlbuHvPOBB8f0u5ySfJOfBDca88kZmAe2sZSOF4icA7l4DuPoHALi0Sy1bZBJk14vJuxQpu9kUkGyFr69UQZ8E2qPI558O36WUmeJdl8osAiERibtrD3h1u9b4h2HVsSSTHjemHotIh2Z9%2BMiR%2BTG8yrl%2B1yyg%2FYNbaVM92gwS0i98k%2BDy4mzQOJtN8m2OdLb2nvDz79IC5vsjibYzaN72DtKetbMIXQjc8GOrEBrYOkDEUeaL8pKRVjg0QrGRtBa0hjmwPs1hNHELHX3E5rVOEgAQOBJYxm2WR1ib332WIAOeNPp7U2zD%2FSrJs41blbueRjwFtC7IgB2YZxABbAQ5kJOkz7%2F0LnBjApne0%2BMAtpbW%2Bdcz8r%2FQq%2F5xL5plMzXZ304FwPV%2F9r57tsBt323rzuxMmOcBVQr8QMVQl9vXm%2FqbVrziy4186rz4dM2pkrBxBXt7gxj1yEqZuyKeSG&X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Date=20260418T113650Z&X-Amz-SignedHeaders=host&X-Amz-Expires=300&X-Amz-Credential=ASIAUPUUPRWESCLANVXW%2F20260418%2Fus-east-1%2Fs3%2Faws4_request&X-Amz-Signature=4f50fbd315f4b999a05c9df8fa4b3c3c2233053c66628c1bb42f58718fff438f&abstractId=6191618
- HMM ensemble voting
  - https://www.aimspress.com/data/article/preview/pdf/69045d2fba35de34708adb5d.pdf
- Ising 相变金融模型
  - https://arxiv.org/pdf/2504.19050v1
- Bayesian Decision Tree
  - https://arxiv.org/pdf/2309.15312

---

## Sprint 1（2-3 周）：Execution-First 基础强化

### 具体怎么做

#### 1. 新增 execution 领域模块
建议新增：
- `src/domain/execution/mod.rs`
- `src/domain/execution/types.rs`
- `src/domain/execution/ou.rs`
- `src/domain/execution/score.rs`
- `src/application/execution/mod.rs`
- `src/application/execution/artifact.rs`
- `src/application/execution/reflection.rs`

目标：把 execution 从散落在 trade plan / factor score / update 输出中的隐含变量，升级为显式 domain object。

#### 2. 扩展现有特征面
建议新增到现有 `ICTFeatures` / `FrameFeatures` / `EntryExecution` 一类结构：
```rust
pub struct ExecutionFeatures {
    pub execution_score: f64,
    pub aggression_bias: f64,
    pub completion_pressure: f64,
    pub liquidity_absorption_bias: f64,
    pub overextension_distance: Option<f64>,
    pub reversion_speed: Option<f64>,
    pub evidence_quality: f64,
}

pub struct EntryExecution {
    pub regime_gate_passed: bool,
    pub execution_features: ExecutionFeatures,
    pub execution_note: String,
}
```

建议接入点：
- `src/config.rs` 的 frame feature build surface
- `src/application/belief/pipeline_types.rs`
- `src/planner/trade_plan.rs`
- `src/application/reporting/analyze_output.rs`

#### 3. 在 voting layer 后新增 ExecutionArtifact
新增状态产物：
- `state/<SYMBOL>/execution_artifact.json`
- `artifact_ledger.json` 记录 kind=`execution_artifact`

建议类型：
```rust
pub struct ExecutionArtifact {
    pub artifact_id: String,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub execution_score: f64,
    pub prediction_score: f64,
    pub execution_edge_share: f64,
    pub prediction_edge_share: f64,
    pub overextension_distance: Option<f64>,
    pub reversion_speed: Option<f64>,
    pub hard_gate_status: String,
    pub provenance: RunProvenance,
}
```

集成点：
- `CatBoost voting` 之后
- `reflection_bundle` 生成之前
- `workflow_snapshot` 写盘之前

#### 4. 注入 OU 过程做 execution 弹性度量
建议：
- 在 `src/domain/execution/ou.rs` 中实现局部 OU 参数估计
- 用 rolling price deviation / realized spread / momentum decay 拟合：
```rust
pub struct OuExecutionMetrics {
    pub mean_level: f64,
    pub theta: f64,
    pub sigma: f64,
    pub overextension_distance: f64,
    pub reversion_speed: f64,
}

pub fn estimate_ou_execution_metrics(
    prices: &[f64],
    timestamps: &[DateTime<Utc>],
) -> Option<OuExecutionMetrics>;
```

集成方式：
- 不取代 HMM
- 只作为 execution tree / execution artifact 的额外特征
- hard gate 可设：`overextension_distance` 过大而 `reversion_speed` 过弱时，降级 execution readiness

#### 5. reflection_bundle 明确拆 execution vs prediction
建议在 `reflection_bundle` / `human report` 强制输出：
- `execution_edge_share`
- `prediction_edge_share`
- `execution_readiness`
- `why_execution_dominates`
- `why_prediction_is_demoted`

示意：
```json
{
  "execution_edge_share": 0.73,
  "prediction_edge_share": 0.27,
  "execution_summary": "entry quality relies more on fillability, mean reversion elasticity, and completion pressure than directional forecast confidence"
}
```

### 能得到什么明确优势
- comparability：execution quality 首次成为跨 run 可比较字段，不再只比较方向概率。
- provenance：每次 analyze/research/backtest 都可回答“这次 edge 到底来自 execution 还是 prediction”。
- reflection_bundle 可审计性：从叙述性解释变成结构化 execution attribution。
- agent safety：human-next triage 不再被高方向概率误导；若 execution weak，hard gate 在 Sprint 1 即直接阻断，而不是延后到产品化阶段。
- contributor workflow：任何人改特征、模型、gate，都必须面对 execution artifact 的回归测试。
- 量化目标：
  - 让 `reflection_bundle` 100% 输出 execution/prediction edge split
  - 让 `artifact_ledger` 新增 execution artifacts 后可回放比较
  - 让 `workflow-status --agent` 能直接给出 execution-first next step
  - 让 analyze/research 的默认结论先回答“能否做”，再回答“方向如何”

---

## Sprint 2（2-3 周）：物理学模型注入

### 具体怎么做

#### 1. Pythagorean 几何距离进入 PDArray / line-extension surface
新增模块：
- `src/math/geometry.rs`
- `src/ict/pythagorean_extension.rs`

建议函数：
```rust
pub struct PythagoreanExtensionMetrics {
    pub trendline_distance: f64,
    pub orthogonal_extension: f64,
    pub normalized_overstretch: f64,
}

pub fn measure_pythagorean_extension(
    anchor_a: (f64, f64),
    anchor_b: (f64, f64),
    current: (f64, f64),
) -> PythagoreanExtensionMetrics;
```

集成点：
- `PDArrayType::detect`
- `FrameFeatures`
- `factor_lab/factor_definition.rs` 中新增 execution-sensitive factor inputs

#### 2. Ising mean-field 层进入 herd / phase transition 检测
新增模块：
- `src/domain/regime/ising.rs`
- `src/application/belief/ising_overlay.rs`

建议抽象：
```rust
pub struct IsingState {
    pub magnetization: f64,
    pub coupling_strength: f64,
    pub phase_transition_risk: f64,
    pub herding_bias: f64,
}

pub fn estimate_ising_state(
    aligned_signals: &[f64],
    participation_weights: &[f64],
) -> Option<IsingState>;
```

接入方式：
- 不直接替代 BBN posterior
- 在 HMM posterior 之后追加 overlay
- 用于 execution tree 的 branch feature：若 phase transition risk 高，则更偏“执行安全/吸收能力”而非方向置信

#### 3. 在 HMM posterior 后增加 OU filter
新增：
- `src/application/belief/ou_overlay.rs`

注意：此层第一优先不是解释 regime，而是解释 execution feasibility；只有在 execution surface 已接入后，才允许它提升到 regime explanation。

建议接口：
```rust
pub struct ExecutionPhysicsOverlay {
    pub ou: Option<OuExecutionMetrics>,
    pub ising: Option<IsingState>,
    pub pythagorean: Option<PythagoreanExtensionMetrics>,
}

pub fn build_execution_physics_overlay(
    candles: &[Candle],
    frame_features: &FrameFeatures,
) -> ExecutionPhysicsOverlay;
```

集成点：
- `pipeline_builder.rs`
- `policy_engine.rs`
- `trace_builder.rs`

#### 4. 在 factor research 中记录 physics feature lineage
扩展：
- `objective_surfaces`
- `artifact ledger`
- `reflection_bundle`

新增字段：
- `ou_overextension_distance`
- `ou_reversion_speed`
- `ising_phase_transition_risk`
- `pythagorean_overstretch`

### 能得到什么明确优势
- regime 检测不再只有统计概率，而有物理解释面：弹性、相变、过伸。
- factor ranking 可区分“方向对但执行差”与“方向一般但 execution 可做”。
- forward-test 稳定性会比纯分类信号更强，因为 execution physics 比方向信号更平滑、更可迁移。
- hard gate 更可信：不是黑箱 veto，而是“过伸 + herd risk + mean reversion elasticity”三重证据。
- artifact ledger 提升：能审计每个 regime / factor 决策到底是被哪种 physics overlay 推动。
- contributor workflow：未来任何人新增 physics 特征，必须落到 objective surface 与 lineage，而不是孤立研究脚本。
- 量化目标：
  - 在 `factor-pipeline-debug` 中加入 physics trace section
  - 让 `objective_surfaces` 至少包含 3 个 physics feature score
  - 让 forward-test 中 execution-related factor variance 下降，减少纯预测 score 抖动

---

## Sprint 3（2-3 周）：MECE Regimes + HMM 完美恢复闭环

### 具体怎么做

#### 1. 实现 manual_mece_labeler
新增模块：
- `src/domain/regime/mece_labeler.rs`
- `src/application/regime/recovery.rs`
- `support/scripts/mece_label_bootstrap.py` 只作离线辅助，不作系统真相源

建议接口：
```rust
pub enum MeceRegimeLabel {
    Expansion,
    Manipulation,
    Reversion,
    Compression,
    TrendContinuation,
    Unknown,
}

pub fn manual_mece_labeler(
    candles: &[Candle],
    frame_features: &FrameFeatures,
) -> Vec<MeceRegimeLabel>;
```

要求：
- 标签本身 versioned
- 写入 `artifact ledger`
- 进入 `workflow_snapshot` 的 comparability surface

#### 2. 建立因子搜索闭环直到恢复率 > 95%
新增 research loop：
- `search_factors_for_mece_recovery`
- 可挂到 `factor-autoresearch` 的平行分支

修正：恢复率不是唯一优化目标。必须并行记录 execution consistency，防止出现“regime 恢复很好但 execution artifact 持续提示不可做”的假成功。

建议 skeleton：
```rust
pub struct MeceRecoveryReport {
    pub accuracy: f64,
    pub macro_f1: f64,
    pub confusion_matrix: BTreeMap<String, BTreeMap<String, usize>>,
    pub best_factor_set: Vec<String>,
    pub provenance: RunProvenance,
}

pub fn search_factors_for_mece_recovery(
    candles: &[Candle],
    labels: &[MeceRegimeLabel],
    registry: &FactorRegistry,
) -> Result<MeceRecoveryReport>;
```

集成点：
- `factor_lab/research.rs`
- `state/research_runs.json`
- `factor_autoresearch_live/final`

#### 3. 锁定最优因子集进 artifact ledger
新增 artifact kind：
- `mece_recovery_artifact`

建议字段：
```rust
pub struct MeceRecoveryArtifact {
    pub artifact_id: String,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub accuracy: f64,
    pub macro_f1: f64,
    pub selected_factors: Vec<String>,
    pub hmm_viterbi_hash: String,
    pub label_hash: String,
    pub provenance: RunProvenance,
}
```

#### 4. cargo test 级恢复验证
新增测试面：
- `tests/mece_recovery.rs`
- `tests/hmm_recovery_regression.rs`

示意：
```rust
#[test]
fn hmm_viterbi_mece_recovery_stays_above_threshold() {
    let report = run_fixture_mece_recovery();
    assert!(report.accuracy >= 0.95);
}
```

### 能得到什么明确优势
- HMM 不再只是“看起来合理”的 regime 模型，而是有显式恢复率约束的 state machine。
- comparability：不同研究 run 可按 label hash / viterbi hash 精确比较，不会混淆旧标签体系。
- provenance：任何 regime 改动都能追溯到标签版、因子集版、模型版。
- reflection_bundle：可以输出“这次 regime 解释是否落在已验证的 MECE 恢复区间内”。
- agent safety：若恢复率跌破阈值，hard gate 直接禁止 promotion。
- contributor workflow：PR 不再只是过编译，而是要过 regime recovery 回归。
- 量化目标：
  - HMM Viterbi vs manual MECE 恢复准确率 > 95%
  - artifact ledger 可保存每次最优因子集锁定证据
  - factor-autoresearch 从“盲调”升级为“围绕恢复率目标的可验证搜索” 

---

## Sprint 4（2-3 周）：Execution Tree + Full Workflow 产品化

### 具体怎么做

#### 1. Execution Score + physics features 全部喂入 voting
现有 `CatBoost voting` 后面不再只看 prediction-family features。
新增统一输入面：
- execution features
- OU metrics
- Ising state
- Pythagorean extension
- HMM posterior / MECE recovery confidence

建议模块：
- `src/application/orchestration/execution_tree.rs`
- `src/application/reporting/execution_focus.rs`

建议 skeleton：
```rust
pub struct ExecutionTreeInput<'a> {
    pub execution_features: &'a ExecutionFeatures,
    pub physics_overlay: &'a ExecutionPhysicsOverlay,
    pub hmm_posterior: &'a RegimeProbs,
    pub mece_recovery_confidence: Option<f64>,
    pub prediction_vote_score: f64,
}

pub struct ExecutionTreeOutput {
    pub execution_score: f64,
    pub branch: String,
    pub execution_bias: String,
    pub gate_status: String,
}

pub trait ExecutionTreeScorer {
    fn score(&self, input: &ExecutionTreeInput<'_>) -> Result<ExecutionTreeOutput>;
}
```

#### 2. 引入 Bayesian Decision Tree 风格升级
不是立刻引入复杂外部 runtime，而是在现有 Rust stateful 面中实现：
- branch probability
- posterior uncertainty
- split reason lineage

建议产物：
- `execution_tree_trace.json`
- `execution_tree_artifact`

#### 3. SHAP 解释强制绑定 reflection_bundle
要求：
- SHAP 不是单独调试输出
- 必须进 `reflection_bundle` / `artifact ledger`
- 必须能解释 execution score 的 top contributors

建议字段：
```rust
pub struct ExecutionShapAttribution {
    pub feature: String,
    pub contribution: f64,
    pub feature_value: String,
}
```

#### 4. 新增 `--execution-focus` flag
CLI 面：
- `cargo run -- analyze --execution-focus ...`
- `cargo run -- workflow-status --execution-focus ...`

语义：
- 该 flag 用作显式聚焦模式
- 但产品目标不是把 execution 留在可选开关里，而是让 execution triage 成为默认可读面；非 execution 面退为 secondary surface

#### 5. 将 hard gate 升级为 execution-first gate
现有 hard gate 加一层：
- prediction strong but execution weak -> block
- prediction medium but execution strong -> observe/tune, not reject

### 能得到什么明确优势
- `ict-engine` 不再是概率 + 预测引擎，而是真正的 execution operating system。
- 任何 AI wrapper / prompt copilot 都难复制：因为真正护城河在 stateful execution lineage、artifacts、hard gates、recovery loops，而不在 prompt 文案。
- stateful CLI：用户能直接问“这单能不能做”，而不是只看到涨跌偏向。
- hard gate：从单纯风险阻断变成 execution readiness governance。
- artifact ledger：execution tree、SHAP attribution、physics overlay、MECE recovery 全部形成可追踪证据链。
- contributor workflow：新增 feature 不只是提高 auc，而是要通过 execution artifact / SHAP / recovery / hard gate 全链路验证。
- 量化目标：
  - `--execution-focus` 输出成为默认人类 triage 面
  - reflection_bundle 100% 附带 execution attribution
  - promotion gate 以 execution score 为第一条件，而非 prediction confidence

---

## 跨 Sprint 统一架构约束

### 1. Execution 作为第一类 artifact
所有 execution 相关对象必须进入：
- `state/*`
- `artifact_ledger.json`
- `workflow_snapshot.json`
- `reflection_bundle`

### 2. 不把物理学层做成孤立 demo
OU / Ising / Pythagorean 必须：
- 有 trait
- 有 state lineage
- 有 tests
- 有 compare surface
- 有 hard gate 作用点

### 3. 不允许“只加模型，不加 comparability”
每个新层必须回答：
- 上一 run 和这一 run 怎么比
- provenance 如何写
- artifact 如何存
- regression 如何测

### 4. 不允许 main.rs 再膨胀
所有新逻辑优先进：
- `src/domain/execution/*`
- `src/application/execution/*`
- `src/domain/regime/*`
- `src/application/belief/*`
- `src/application/orchestration/*`

`main.rs` 只留 command wiring 与 thin orchestration。

---

## 如果我是技术 co-founder：为何这 3 个月计划能让 ict-engine 变成真正 stateful research OS

关键不在“多加几个模型”，而在四件事同时成立：

1. `Execution` 变成一等公民
- 项目不再围绕“谁预测更准”内卷。
- 而是围绕“何时可做、为何可做、做错时证据在哪”建立闭环。
- 这直接对齐论文结论：赚钱核心在 execution，不在 information 本身。

2. `Physics overlay` 给出可解释中层
- OU 解释均值回复弹性
- Ising 解释 herd/phase transition
- Pythagorean 解释几何过伸
- 这让系统既不是纯规则，也不是纯黑箱。

3. `MECE recovery` 让 regime 进入可验证科学状态
- 没有恢复率约束的 HMM，只是好看的 posterior。
- 有了 manual MECE label + >95% recovery target，regime 才有资格成为操作系统的状态层。

4. `Artifact + provenance + gate + reflection` 一体化
- 这决定它是 research OS，还是又一个 trading bot。
- trading bot 只会给信号。
- research OS 会留下：
  - 为什么给
  - 为什么挡
  - 改了什么
  - 比上次强在哪
  - 证据链是否可重放

三个月后，若按此计划完成，`ict-engine` 的价值将不在于“能否再包一层 AI UI”，而在于：
- 它拥有 execution-first artifact lineage
- 它拥有可验证 regime recovery
- 它拥有 physics-informed execution tree
- 它拥有 contributor-safe、stateful、可审计的研究闭环

这四者叠加，才是一个真正的 `stateful research OS`。