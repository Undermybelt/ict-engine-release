# Paper-driven typed packets design

## Goal
在不先改训练流、不先上重模型的前提下，把论文可吸收机制先沉成稳定 typed packets、persisted artifacts、reporting/reflection surfaces。此层只负责“表达、审计、回放、路由”；不负责直接改变现有交易结论。

## Why packet-first
- 避免再把实验性机制塞进 `src/main.rs`
- 先把 schema 定型，再逐步挂模型
- 让 jump model / structural break / conformal / microstructure 有统一 audit 面
- 保持 `WorkflowSnapshot` / artifact ledger / reflection_bundle 可追踪

## Hard rules
1. PDA / footprint / LOB 不得在本阶段作为 direct evidence。
2. conformal 输出不是方向信号，只能用于 uncertainty / abstention / validator。
3. structural break 输出不是 hindsight 标签，只能用于 segmentation / gate / backtest validator。
4. 新机制默认以 optional packet 进入现有 surface；不得硬改当前推荐逻辑。
5. 所有可持久化机制必须有 typed record；禁止 ad hoc JSON blob 逃逸。

## Packet families

### 1. RegimeSegmentationPacket
用途：承接 jump model / regime clustering / transition hazard / feature attribution。
建议放置：`src/domain/regime/types.rs`

字段最小集：
- `method`
- `segmentation_version`
- `active_regime_cluster`
- `transition_hazard`
- `regime_membership`
- `feature_attribution`
- `evidence`

映射角色：
- regime classifier
- state transition
- market-specific policy
- feature selection / factor research

### 2. StructuralBreakPacket
用途：承接 parametric / nonparametric structural break 检测。
建议放置：`src/domain/regime/types.rs`

字段最小集：
- `method`
- `break_family`
- `detected`
- `break_score`
- `break_index`
- `lookback_window`
- `affected_features`
- `rationale`

映射角色：
- outcome validator
- backtest validator
- uncertainty / abstention gate
- regime classifier

### 3. ConformalUncertaintyPacket
用途：承接 temporal conformal / conformal TS forecasting / implied-vol conformal calibration。
建议放置：`src/domain/belief/types.rs`

字段最小集：
- `method`
- `target`
- `nominal_coverage`
- `empirical_coverage`
- `interval_width`
- `nonconformity_score`
- `abstain_threshold`
- `abstain`
- `notes`

映射角色：
- uncertainty / abstention gate
- outcome validator
- backtest validator

明确排除：
- direct entry evidence
- direct direction vote

### 4. MicrostructureContextPacket
用途：承接 DeepLOB / footprint / queue / imbalance / LOB context，但仅作 context。
建议放置：`src/domain/belief/types.rs`

字段最小集：
- `source`
- `granularity`
- `usable_as_evidence`
- `prior_adjuster_bias`
- `transition_bias`
- `setup_quality_score`
- `context_notes`

映射角色：
- prior adjuster
- state transition
- setup classifier

明确排除：
- direct evidence（默认 `usable_as_evidence=false`）

### 5. MarketPolicyPacket
用途：承接 market-family / market-behavior-profile 下的策略可靠性偏置。
建议放置：`src/domain/belief/types.rs`

字段最小集：
- `market_family`
- `market_behavior_profile`
- `policy_mode`
- `evidence_reliability`
- `abstention_bias`
- `notes`

映射角色：
- market-specific policy
- prior adjuster
- uncertainty / abstention gate

### 6. RegimeValidationPacket
用途：承接 segmentation stability、hindsight 风险、validation gate。
建议放置：`src/domain/regime/types.rs`

字段最小集：
- `validation_scope`
- `segmentation_consistency`
- `hindsight_risk_flags`
- `abstain_recommended`
- `notes`

映射角色：
- backtest validator
- outcome validator
- uncertainty / abstention gate

## Existing repo alignment

优先挂接现有 packet：
- `RegimeFeatures`
  - `segmentation_context: Option<RegimeSegmentationPacket>`
  - `structural_break_context: Option<StructuralBreakPacket>`
- `RegimePosterior`
  - `regime_validation: Option<RegimeValidationPacket>`
- `BeliefEvidencePacket`
  - `microstructure_context: Option<MicrostructureContextPacket>`
  - `market_policy: Option<MarketPolicyPacket>`
- `BeliefReportPacket`
  - `conformal_uncertainty: Vec<ConformalUncertaintyPacket>`
  - `market_policy: Option<MarketPolicyPacket>`

## Persisted artifact layer

新增 typed record families：
- `RegimeSegmentationRecord`
- `StructuralBreakRecord`
- `ConformalUncertaintyRecord`
- `MarketPolicyRecord`

建议文件常量：
- `REGIME_SEGMENTATION_FILE`
- `STRUCTURAL_BREAK_FILE`
- `CONFORMAL_UNCERTAINTY_FILE`
- `MARKET_POLICY_FILE`

persisted record 统一包含：
- `artifact_id`
- `generated_at`
- `symbol`
- `source_phase`
- `packet`

## Builder / adapter layer

新增 builder surface：`src/application/belief/paper_packets.rs`

应提供：
- `build_regime_segmentation_packet`
- `build_structural_break_packet`
- `build_conformal_uncertainty_packets`
- `build_microstructure_context_packet`
- `build_market_policy_packet`
- `build_regime_validation_packet`

phase 1 规则：
- 可输出 `placeholder:none` / `rule-based:phase1`
- 必须 deterministic
- 不改变现有 inference 结果

## Reflection / reporting integration

仅新增附属字段或 summary lines：
- reflection bundle 可见 packet summary
- reporting surface 可见 packet summary
- workflow / artifact ledger 可审计 packet artifact

不做：
- 直接基于 packet 改写 `recommended_next_command`
- 把 packet 强塞进 execution vote

## Paper mechanism mapping

| 机制 | 第一落点 | 角色 | phase |
|---|---|---|---|
| Jump model regime segmentation | `RegimeSegmentationPacket` | regime classifier / state transition | packet-only |
| Feature selection in jump model | `RegimeSegmentationPacket.feature_attribution` | feature selection / factor research | packet-only |
| Parametric structural break | `StructuralBreakPacket` | backtest validator / gate | packet-only |
| Nonparametric structural break | `StructuralBreakPacket` | backtest validator / regime validation | packet-only |
| Temporal conformal | `ConformalUncertaintyPacket` | uncertainty / abstention gate | packet-only |
| Conformal TS forecasting | `ConformalUncertaintyPacket` | outcome validator / backtest validator | packet-only |
| Market-implied conformal vol intervals | `ConformalUncertaintyPacket` + `MarketPolicyPacket` | uncertainty / market-specific policy | packet-only |
| DeepLOB / microstructure context | `MicrostructureContextPacket` | prior adjuster / state transition / setup classifier | packet-only |

## Anti-misuse rules

### Forbidden
- 用 `MicrostructureContextPacket` 直接驱动 long/short evidence
- 用 `ConformalUncertaintyPacket` 直接判方向
- 用 `StructuralBreakPacket` 解释已发生结果后反推 regime 标签
- 用 jump segmentation 对整段 hindsight 数据一次性打标签后拿去做 online 决策评估

### Allowed
- 用 microstructure context 改 prior / transition / setup quality
- 用 conformal packet 决定 abstain / reduce size / require confirmation
- 用 structural break packet 标记 backtest split 失真风险
- 用 jump segmentation packet 做 regime family surface 与 factor-research stratification

## Rollout order
1. typed packets
2. persisted records
3. application builders
4. reporting / reflection / artifact summaries
5. factor-role routing constraints
6. single-mechanism PoC
7. regime/backtest validator integration
8. full model/training path

## Best first PoCs after packet phase
1. `ConformalUncertaintyPacket` → abstention gate
2. `StructuralBreakPacket` → regime-split backtest validator
3. `RegimeSegmentationPacket` → HMM companion surface, not replacement
4. `MicrostructureContextPacket` → setup classifier bias only

## Success criteria for packet-first phase
- schemas stable
- serde round-trip works
- persistence works
- reporting can surface summaries
- artifact ledger can audit
- role routing prevents misuse
- no new ML dependency
- no widened `main.rs` drift
