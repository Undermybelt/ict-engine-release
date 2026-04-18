# Hybrid Regime Clustering Integration Note

参考论文：
- arXiv 2108.05801
- *A Hybrid Learning Approach to Detecting Regime Switches in Financial Markets*

## 论文核心，可吸收之核
论文主线不是“单模型预测市场”，而是：
1. 先构造多维市场/经济特征
2. 用 PCA 做降维
3. 用 k-means 做无监督 regime clustering
4. 再把 regime 结果接到分类/交易层

最重要的不是 PCA 或 k-means 本身，而是这一层次顺序：
- regime segmentation first
- decision / trading second

这与当前仓最应采取的顺序一致：
- filter / denoise
- regime segmentation
- pre-bayes evidence filtering
- BBN belief update
- policy / execution

## 对当前仓的最有价值启发

### 1. Regime 不该只由单一路径给出
当前仓已有：
- Kalman / particle / HMM 这类时间连续性路径
- pre-bayes / BBN 这类证据融合路径

论文启发：
- 再加一条“横截面/多特征” regime segmentation 路径
- 让 regime 变成 hybrid，而不是单一来源

建议三路：
1. temporal latent regime
   - 由 HMM / filter 负责
2. cross-sectional regime cluster
   - 由 feature matrix + PCA + clustering 负责
3. filtered belief regime
   - 由 PreBayes 把 1 和 2 及其他证据调和后输出

### 2. Cluster label 应该是候选证据，不是终局裁决
cluster 很适合提供：
- `raw_market_regime_label` candidate
- `cluster_confidence`
- `cluster_distance_to_centroid`
- `cluster_transition_warning`

但不应直接替代：
- HMM latent state
- PreBayes gating
- BBN final belief

### 3. PreBayes 是最合适的冲突调解层
建议把 PreBayes 当成 hybrid 融合器：
- 输入：
  - HMM regime
  - cluster regime
  - factor alignment
  - liquidity context
  - multi-timeframe resonance
- 输出：
  - `filtered_market_regime_label`
  - `filtered_factor_uncertainty`
  - `filtered_multi_timeframe_resonance_label`
  - `evidence_quality_score`
  - `gating_status`

### 4. Cluster 更适合影响 uncertainty / resonance，而不是直接交易方向
当 HMM 与 cluster 一致：
- 可降低 uncertainty
- 可提升 evidence quality
- 可向 `aligned` 共振偏移

当 HMM 与 cluster 冲突：
- 应抬高 uncertainty
- 将 resonance 降为 `mixed` 或 `dislocated`
- 触发 stronger gating / observe_only

### 5. 当前仓需要一层新的 regime feature matrix
建议新增概念层：
- `regime_feature_matrix`

输入候选：
- price structure state
- liquidity context
- factor alignment
- factor uncertainty proxy
- multi-timeframe alignment score
- multi-timeframe entry alignment score
- SMT/correlation bias
- option/ETF/CFD confirmation labels
- realized vol / ATR band
- timed PDA summary counts
- HMM posterior / latent state summaries

该层不直接喂 policy。
先服务于 regime segmentation。

## 建议的新数据流

### 当前理想链
1. market data / factor data
2. filter / denoise
3. HMM latent state
4. regime feature matrix
5. PCA / clustering
6. PreBayes hybrid reconciliation
7. BeliefEvidencePacket / BBN
8. policy / execution

### 具体到字段
建议增加：
- `cluster_regime_label`
- `cluster_regime_confidence`
- `cluster_regime_distance`
- `cluster_transition_flag`

然后映射到：
- `raw_market_regime_label`
- `raw_factor_uncertainty`
- `raw_multi_timeframe_resonance_label`
或作为额外 rationale / evidence assignments

## 对现有模块的接入映射

### 1. `src/data/regime_segmentation.rs`
扩成 hybrid segmentation 主入口：
- feature matrix builder
- optional PCA reducer
- cluster labeler
- cluster diagnostics

建议输出 struct：
- `HybridRegimeSegmentationResult`
  - `cluster_regime_label`
  - `cluster_confidence`
  - `cluster_distance`
  - `feature_projection`
  - `cluster_transition_flag`

### 2. `src/config.rs`
在 `build_pre_bayes_evidence_filter(...)` 附近接：
- cluster regime candidate
- cluster confidence

并加入规则：
- HMM == cluster -> confidence bonus
- HMM != cluster -> uncertainty penalty, resonance downgrade

### 3. `src/bbn/adapters/legacy_pre_bayes.rs`
将 hybrid 结果写入：
- `factor_evidence`
- `market_evidence`
- `evidence_assignments`
- optional typed fields later if needed

### 4. `src/domain/regime/types.rs`
可增加：
- `cluster_regime_label`
- `cluster_confidence`
- `cluster_transition_score`

### 5. `src/bbn/trading/topology.rs`
后续可考虑：
- 用 cluster-derived empirical prior 调整 `market_regime` root prior
- 但只在明确验证后采用

## 最小实施计划

### Phase 1: design-only, no runtime risk
- 定义 `HybridRegimeSegmentationResult`
- 定义 feature matrix schema
- 不改现有 inference

### Phase 2: offline regime segmentation
- 从历史数据构建 feature matrix
- 先离线做 PCA/cluster
- 产出 labels + confidence CSV

### Phase 3: PreBayes integration
- 把 cluster labels 接进 `build_pre_bayes_evidence_filter`
- 仅影响 uncertainty/resonance/gating
- 不直接改 policy

### Phase 4: BBN integration
- 把 cluster regime 作为额外 belief evidence
- 观察 posterior 改善

## 重要约束
- 不要让 cluster 直接下交易命令
- 不要让 PCA/k-means 取代 HMM
- 不要把 offline segmentation 结果硬编码为永真 regime
- 不要先改 policy/execution，再补 regime 逻辑

## 一句话总结
这篇论文最该借给本仓的不是算法细节，而是一个正确的架构顺序：
先用多特征无监督方法把 regime 分出来，再让 PreBayes 和 BBN 去调和 temporal state 与 clustering state，最后才进入 policy/execution。
