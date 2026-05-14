# DNA Sequence-Inspired PDA Clustering Optimization Plan

> 基于 DeLUCS / Nucleotide Transformer / DNABERT / HMM-cluster 四篇论文，对 ict-engine 的 PDA 序列聚类和 regime 发现进行优化。

## 核心洞察

PDA 序列（做市商意图的"显性表达"）和 DNA 序列（碱基对排列 + 编码片段）在结构信息远大于周期信息这一点上高度一致。

DNA 领域已成熟解决以下 5 个痛点，核心方法是无监督序列聚类 + 多头注意力（multi-attention / Transformer）：

| 痛点 | DNA 解法 | PDA 映射 |
|------|---------|---------|
| 顺序不一致 | DTW-like warping + attention | OB→FVG vs FVG→OB |
| 中间缺失 | padding token + alignment | 少一两个 FVG / Breaker |
| 长度不一 | FCGR 固定矩阵 / Transformer 变长输入 | PDA 序列长度不定 |
| 无明确起止边界 | motif discovery（无监督） | regime 自动切分 |
| 重复性强但聚类特征明显 | k-mer 频率 + multi-head attention | Silver Bullet / Judas Swing 等重复 pattern |

**multi-attention 是序列和结构的最终形式** — 这和 DNA 领域 10+ 年验证完全吻合。

## 核心类比

| DNA 域 | ICT Engine 域 |
|--------|--------------|
| 碱基 (A/T/G/C) | PDAToken (OB/FVG/Breaker/Mitigation/Rejection/Propulsion/Void/VolumeImbalance/InverseFVG) |
| DNA 序列 | Vec<PDAToken> 时间序列（含 overlap、liquidity_swept、displacement 等上下文） |
| 基因（编码片段） | 有效 regime 模式（扩张/震荡/操纵-扩张转换） |
| 基因组 | 单品种多时间帧 PDA 全局序列 |
| 物种差异 | 品种差异（NQ vs ES vs BTC） |
| 突变/缺失 | PDA 序列中间缺失、噪音插入、顺序变异 |

## 为什么这个类比成立

1. **变长序列**：PDA 序列和 DNA 一样，长度不固定，不可简单 pad。
2. **顺序敏感但允许变异**：OB→FVG→Breaker 是常见 motif，但中间可能插入噪音。
3. **重复 motif**：Silver Bullet / Judas Swing / Turtle Soup 等是重复出现的"编码片段"。
4. **无标签、无监督**：当前 ict-engine 没有 regime ground truth，需自动发现。
5. **已有 HMM 基础**：ict-engine 已有 HMM 模块，可直接升级为序列级而非特征级。

---

## 四篇论文可吸收机制

### 1. DeLUCS → FCGR + 自学习聚类

**核心机制**：
- Frequency Chaos Game Representation (FCGR)：将变长序列压缩为固定 2D 矩阵（k-mer 频率图）
- 多网络自学习（self-supervised）：用同一数据的不同 view 训练多个网络
- Majority voting：多网络投票决定最终聚类标签
- 77-100% 准确率，远超 GMM/K-means

**对 ict-engine 的价值**：
- FCGR 可把 Vec<PDAToken> 变成固定维度 embedding，喂给 CatBoost
- 自学习 + majority voting 比单一 HMM 更鲁棒
- 天然处理变长、缺失、噪音

**可落点**：
- 新增 `PDASequenceFCGR`：将 PDA 序列转为 k-mer 频率矩阵
- 新增 `SelfSupervisedClusterEnsemble`：多网络 + majority voting
- 输出 `RegimeSegmentationPacket.cluster_regime_label`

### 2. Nucleotide Transformer → 多头注意力 embedding

**核心机制**：
- Transformer + 多头注意力在 DNA 上 pre-train
- 无监督学习序列结构和重复 motif
- 支持超长序列（>12k bp）和变长输入
- 学到的 embedding 可直接用于下游聚类/分类

**对 ict-engine 的价值**：
- 把 PDAToken 当"碱基"，用 Transformer 学习 PDA 序列表征
- 多头注意力自动发现"编码片段"（有效 regime motif）
- 生成固定维度 embedding 喂给 BBN / CatBoost voting layer

**可落点**：
- Phase 2 实验：用 HuggingFace 模型做 PDA→embedding PoC
- 不需要重新训练，用 pre-trained 模型做 transfer learning
- 输出 `PDATransformerEmbedding` packet

### 3. DNABERT → BERT-style 上下文学习

**核心机制**：
- BERT-style 预训练 + k-mer tokenization
- 学习上下文依赖和 motif 重复模式
- DNABERT-S 的 species-aware embedding（品种感知聚类）

**对 ict-engine 的价值**：
- k-mer tokenization 可直接用于 PDA（如 3-mer = OB→FVG→Breaker）
- 上下文学习捕获 PDA 序列的前后依赖
- DNABERT-S 的品种感知 → NQ vs ES vs BTC 的 regime 差异

**可落点**：
- PDA k-mer tokenization 模块
- 上下文 embedding 作为 regime feature
- 品种感知聚类（market-family-aware）

### 4. HMM-cluster + Sequentia → 序列级 HMM + DTW

**核心机制**：
- HMM 显式建模时序转移 + 重复 motif
- DTW 处理顺序不一致和缺失
- Scikit-learn 兼容接口

**对 ict-engine 的价值**：
- 直接升级现有 HMM 模块为序列级
- DTW 处理 PDA 序列的"中间缺失一两个 PDA"
- 与现有 BBN / PreBayes 天然兼容

**可落点**：
- 升级 `src/hmm/` 为序列级 HMM
- 新增 DTW 对齐模块
- 输出 `HMMSequenceRegimePosterior`

---

## 实施路径

### Phase 0: PoC 验证（1-2 周，Python 原型）

**目标**：验证 FCGR + k-mer 对 PDA 序列是否有效。

**前置：PDA Tokenization Schema**

```
PDAToken → 碱基编码
OB            → "A"
FVG           → "T"
Breaker       → "G"
Mitigation    → "C"
Rejection     → "R"
Propulsion    → "P"
LiquidityVoid → "V"
VolumeImbalance → "I"
InverseFVG    → "N"
```

带上下文后缀（叠加信号）：
```
OB + overlap          → "A+"
OB + liquidity_sweep  → "A~"
OB + silver_bullet    → "A!"
```

缺失处理：插入特殊 padding token `"X"`，不做 zero-padding。

**步骤**：
1. 从 ict-engine 导出历史 PDA 序列（`factor-research --emit-pda-sequences`）
2. 实现 PDA→FCGR 转换（Python，~100 行）
3. 用 DeLUCS 的 FCGR + CNN 方法做聚类
4. 检查聚类结果是否有清晰的 regime 特征
5. 与现有 HMM regime 对比

**产出**：
- `support/scripts/pda_fcgr_poc.py`
- `support/docs/plans/dna-sequence-poc-results.md`
- 聚类可视化 + regime 解释

**Go/No-Go 判定**：
- 如果 FCGR 聚类的 regime 有清晰可解释特征 → 继续
- 如果特征模糊 → 回到因子选择，调整 k-mer 大小和 PDA token 定义

### Phase 1: FCGR + Ensemble Clustering 模块（2-3 周）

**目标**：把 FCGR + self-supervised ensemble clustering 落到 Rust CLI。

**任务**：
1. 新增 `src/pda_sequence/fcgr.rs`：
   - `PDAToken` → k-mer 频率矩阵
   - 支持可配置 k（3-mer, 5-mer, 7-mer）
   - 输出 `FCGRMatrix`（固定维度 Vec<f64>）

2. 新增 `src/pda_sequence/ensemble_cluster.rs`：
   - 多个聚类器（HMM + GMM + K-means）
   - Majority voting 决定最终 regime label
   - 输出 `EnsembleRegimeLabel` + confidence

3. 新增 typed packet：
   ```rust
   pub struct PDAClusteringPacket {
       pub method: String,  // "fcgr_ensemble_v1"
       pub fcgr_k: usize,
       pub regime_cluster: String,
       pub cluster_confidence: f64,
       pub cluster_distance: f64,
       pub ensemble_agreement: f64,  // 多网络一致率
       pub feature_attribution: BTreeMap<String, f64>,
   }
   ```

4. 接入 `RegimeSegmentationPacket`：
   - `active_regime_cluster` 填入 ensemble 结果
   - `feature_attribution` 填入 FCGR top features

5. 接入 PreBayes：
   - cluster regime → `raw_market_regime_label` candidate
   - ensemble agreement → `evidence_quality_score` 调整
   - HMM == cluster → confidence bonus
   - HMM != cluster → uncertainty penalty

**产出**：
- `src/pda_sequence/fcgr.rs`
- `src/pda_sequence/ensemble_cluster.rs`
- `src/domain/regime/types.rs`（新增 PDAClusteringPacket）
- 测试 + cargo test pass

### Phase 2: Transformer Embedding 实验（3-4 周）

**目标**：用 Nucleotide Transformer / DNABERT 思路做 PDA 序列 embedding。

**步骤**：
1. 复用 Phase 0 的 PDA tokenization schema（碱基编码 + 上下文后缀）

2. Python PoC：
   - 用 DNABERT / Nucleotide Transformer 的 HuggingFace 模型
   - 把 PDA token string 当 DNA 序列喂入
   - 提取 [CLS] embedding 或 mean pooling
   - 用 embedding 做 K-means / GMM 聚类
   - 检查 regime 特征清晰度

3. 如果 PoC 有效：
   - 考虑 fine-tune（用 ict-engine 的历史 PDA 数据）
   - 或直接用 pre-trained embedding 作为 regime feature

**产出**：
- `support/scripts/pda_transformer_embedding_poc.py`
- PoC 结果报告
- Go/No-Go 判定

### Phase 3: 序列级 HMM + DTW 升级（2-3 周）

**目标**：升级现有 HMM 为序列级，支持 DTW 对齐。

**任务**：
1. 升级 `src/hmm/`：
   - 从单特征 HMM → 多维 PDA 序列 HMM
   - 状态空间 = regime cluster（扩张/震荡/操纵等）
   - 观测空间 = PDAToken + context features

2. 新增 DTW 对齐模块：
   - 处理 PDA 序列"中间缺失"
   - 处理顺序不一致（OB→FVG vs FVG→OB）
   - 输出 `DTWAlignmentScore`

3. 接入 BBN：
   - HMM posterior → `market_regime` root prior
   - DTW alignment score → `evidence_quality_score`
   - 序列完整性 → `gating_status`

**产出**：
- 升级后的 `src/hmm/` 模块
- `src/pda_sequence/dtw.rs`
- 测试 + cargo test pass

### Phase 4: 全链路集成（2-3 周）

**目标**：把 FCGR + Transformer + HMM + DTW 全部接入 ict-engine 现有 pipeline。

**集成点**：
1. `factor-research` 新增 `--pda-cluster-mode [fcgr|transformer|hmm|ensemble]` flag
2. `RegimeSegmentationPacket` 填充完整 cluster 信息
3. PreBayes 消费 cluster 证据（conflict → uncertainty upgrade）
4. BBN 消费 HMM posterior（regime prior）
5. CatBoost voting layer 消费 FCGR embedding（作为额外特征）
6. `reflection_bundle` 输出 regime 解释（"当前 PDA 序列对齐到 Cluster 2: 扩张模式"）

---

## 约束（不可违反）

1. **Cluster 不直接下交易命令**：cluster label 只能影响 uncertainty / resonance / gating，不能直接决定 long/short。
2. **FCGR/Transformer 不取代 HMM**：是 companion surface，不是 replacement。
3. **不硬编码 offline 结果**：不把离线聚类结果当成永真 regime。
4. **先改 regime 层，再改 policy 层**：不要先改 execution 再补 regime 逻辑。
5. **新模块不进 main.rs**：走 `src/pda_sequence/` 独立模块。
6. **typed packet 先行**：先定义 schema，再逐步挂模型。

---

## 与现有系统的对接映射

| 新模块 | 对接点 | 角色 |
|--------|--------|------|
| `PDAClusteringPacket` | `RegimeSegmentationPacket` | regime classifier |
| FCGR embedding | `RegimeFeatures.feature_attribution` | feature selection |
| Ensemble cluster label | `PreBayesEvidenceFilter` | uncertainty/resonance adjuster |
| HMM sequence posterior | `BBN market_regime root` | prior adjuster |
| DTW alignment score | `evidence_quality_score` | evidence quality gate |
| Transformer embedding | `PolicyFeatureVector` | CatBoost extra feature |

---

## 成功标准

- Phase 0: FCGR 聚类产出可解释 regime（至少 3 个清晰 cluster）
- Phase 1: `cargo test` pass, ensemble agreement > 0.7, regime 有清晰特征描述
- Phase 2: Transformer embedding 聚类优于 FCGR baseline（silhouette score 提升）
- Phase 3: 序列级 HMM 的 regime 转换比现有单特征 HMM 更准确
- Phase 4: 全链路 `factor-research --pda-cluster-mode ensemble` 可运行，输出 regime 解释

---

## 快速落地摘要

新增 PDAToken（基于现有 PDArrayType + SignalBar）→ 一段行情的 PDA 序列展开为 token 序列（缺失 = 特殊 padding token）→ 用任一 Repo（优先 DeLUCS 或 Nucleotide Transformer）做无监督聚类 → 输出 cluster label 作为 ICTFeatures 新字段 → 聚类结果直接喂给 reflection_bundle 和 execution tree：

> "当前 PDA 序列聚类为强扩张模式（DTW 对齐后缺失 1 个 FVG 已自动补齐）"

这样 PDA 聚类从"GMM 忽略时序"升级为真正的序列结构建模，多头注意力 + 无监督 motif 发现抓住"顺序和结构所含的信息重要性远大于周期信息"。

---

## 参考文献

### 1. DeLUCS（最推荐）

- **论文**: [PDF 直链](https://millanp95.github.io/assets/pdf/DeLUCS.pdf)
- **核心**: 完全无监督、无需对齐/标签，FCGR + 多网络自学习 + majority voting
- **准确率**: 77%–100%，远超 GMM/K-means
- **价值**: PDA→FCGR→自动聚类出"扩张 vs 震荡"等 MECE regimes，DTW-like warping 天然支持顺序不一致和缺失

### 2. Nucleotide Transformer

- **论文**: [Nature Machine Intelligence (2025)](https://www.nature.com/articles/s41592-024-02523-z)
- **Repo**: [instadeepai/nucleotide-transformer](https://github.com/instadeepai/nucleotide-transformer)
- **核心**: Transformer + multi-head attention，50M-2.5B params，>12k bp，无监督 motif 发现
- **价值**: 直接把 PDA 序列 token 化，做 embedding → 聚类 → reflection_bundle 输出 regime label

### 3. DNABERT / DNABERT-2 / DNABERT-S

- **原始**: [DNABERT (Bioinformatics)](https://academic.oup.com/bioinformatics/article/37/15/2112/6128680) | [GitHub](https://github.com/jerryji1993/DNABERT)
- **DNABERT-2**: [MAGICS-LAB/DNABERT_2](https://github.com/MAGICS-LAB/DNABERT_2)
- **DNABERT-S**: [MAGICS-LAB/DNABERT_S](https://github.com/MAGICS-LAB/DNABERT_S)
- **核心**: BERT-style + k-mer tokenization + species-aware embedding（品种感知聚类）
- **价值**: multi-attention 最成熟方案，生成固定维度 embedding 喂给 CatBoost voting layer 或 HMM

### 4. HMM-cluster + Sequentia

- **HMM-cluster**: [ucl-pathgenomics/hmmcluster](https://github.com/ucl-pathgenomics/hmmcluster)
- **Sequentia**: [eonu/sequentia](https://github.com/eonu/sequentia)（Scikit-Learn 兼容 HMM + DTW）
- **核心**: HMM 显式建模时序转移 + 重复 motif，DTW 处理顺序不一致和缺失
- **价值**: 直接升级现有 HMM，处理 PDA 序列而非单个特征

---

## 已有系统对接文档

- `support/docs/hybrid-regime-clustering-integration-note.md` — hybrid regime 聚类架构
- `support/docs/ict-factor-mutation-optimization-plan.md` — factor mutation 优化
- `support/docs/paper-driven-typed-packets-design.md` — typed packet 设计
- `support/docs/typed-packets-paper-upgrade-plan.md` — packet-first 升级计划
- `support/docs/experiments/eml-regime-fusion-poc.md` — EML regime fusion PoC（已 rejected，但架构约束可复用）
- `support/docs/pda_type` — PDA 类型完整定义 + 检测逻辑
- `support/docs/experiments/ict-execution-setup-tree.md` — ICT 执行决策树
- `support/docs/research-system-map.md` — 研究系统总览
- `support/docs/regime-aware` — regime-aware trading 框架
- `support/docs/execution-paper-notes-and-plan-update.md` — execution-first 架构修正
