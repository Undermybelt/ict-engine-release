# NLP-Inspired PDA Sequence Clustering Plan

> 将 PDA 事件序列类比自然语言序列，用 NLP / 语音识别成熟算法做时序语义聚类。

## 核心类比：PDA = 自然语言

| 自然语言 | ICT 做市商意图 |
|---------|-------------|
| 底层语义隐藏 | 做市商意图隐藏 |
| 语言显性表达 | PDA 事件显性表达 |
| 词序 + 语法 | PDA 排列 + motif |
| 一段话 = 一个语义单元 | 一段 MMXM = 一个 regime |
| 语音切词（变长、连续） | PDA 序列切分（变长、无明确边界） |

**结论**：NLP/语音识别处理"长度不一、缺少起止边界、中间有缺失或噪音"的数据是家常饭。

---

## 为什么 GMM 失败而序列方法能成

GMM（及之前的尝试）忽略时序相关性：

- GMM 把每个 PDA 当独立样本，不看"前后关系"
- 扩张和震荡的 PDA 排列截然不同，但 GMM 只看"有哪些 PDA"，不看"怎么排列"
- 结果：cluster 混在一起，不可解释

**序列方法的核心优势**：显式建模时序结构 → 顺序、重复、缺失、长度全部被利用。

---

## PDA 序列的 5 个结构特征

| 特征 | 描述 | NLP 类比 |
|------|------|---------|
| 顺序不一致 | 先 MSS 后 FVG，或反过来 | 语序变体（"我吃饭" vs "饭吃我"） |
| 中间缺失 | 少一两个 PDA | 语音中吞字 |
| 长度不一 | 8 个 PDA 走完 vs 20 个 | 短句 vs 长段落 |
| 无明确起止 | 不知道 MMXM 从哪开始到哪结束 | 语音流切词 |
| 重复 motif | Silver Bullet / Judas Swing 反复出现 | 常见搭配 / 固定句式 |

**关键洞察**：聚类特征极其明显（扩张 vs 震荡的 PDA 排列差别特别大），所以即使顺序/缺失/长度不一，也能清晰聚类。

---

## 推荐算法（按落地优先级排序）

### 1. DTW + k-Medoids / Hierarchical Clustering（首选）

**为什么完美匹配**：
- DTW 专为"变长、时序可扭曲、顺序/缺失容忍"设计
- 语音识别早期处理 phoneme 序列的核心算法
- 直接处理 5 个问题（顺序不一致、缺失、长度不同、起点未知）
- 时序相关性被显式建模

**怎么做**：

```
输入：Vec<Vec<PDAToken>>  (多段行情的 PDA 序列)
      其中 PDAToken = PDArrayType + overlap_flag + liquidity_swept + volume_imbalance_ratio

步骤：
1. 对任意两条 PDA 序列计算 DTW 距离矩阵
   - 允许 warping（对齐弯曲）
   - 允许跳过缺失 token
2. 对距离矩阵做 k-medoids（或 hierarchical）聚类
3. 输出 MECE regimes:
   - Cluster 0 = 强扩张 MMXM 模式
   - Cluster 1 = 震荡修复模式
   - Cluster 2 = 操纵-扩张转换模式
   - ...
4. DTW path = "对齐后的 PDA 序列"（可解释）
```

**Rust 实现**：

| 方案 | 链接 | 说明 |
|------|------|------|
| `dtw` crate | https://crates.io/crates/dtw | 纯 Rust，轻量 |
| `dtw-rs` | https://github.com/shshemi/dtw-rs | 支持多种 DTW 变体 |
| `linfa` k-medoids | https://github.com/rust-ml/linfa | Scikit-learn 风格聚类 |

**优势**：
- 聚类结果可解释（DTW path 就是"对齐后的 PDA 序列"）
- 完美补 MECE 手动标注 + HMM 恢复流程
- 无需 GPU，纯 CPU，可直接在 `factor-research` CLI 中运行

**可落点**：
- 新增 `src/pda_sequence/dtw_cluster.rs`
- 新增 typed packet：
  ```rust
  pub struct PDADTWClusterPacket {
      pub method: String,            // "dtw_kmedoids_v1"
      pub regime_cluster: usize,     // cluster label
      pub cluster_name: String,      // "强扩张 MMXM"
      pub dtw_distance_to_medoid: f64,
      pub dtw_alignment_path: Vec<(usize, usize)>,  // 对齐路径
      pub medoid_pda_sequence: Vec<PDAToken>,        // 该 cluster 的代表序列
      pub cluster_size: usize,       // cluster 内样本数
      pub silhouette_score: f64,     // 聚类质量
  }
  ```

---

### 2. HMM Sequence Clustering（升级已有 HMM）

**为什么合适**：经典语音识别做法——每个 regime 建一个 HMM（emission = PDA token 概率），然后用 Viterbi / Baum-Welch 做序列聚类（多模型 HMM 混合）。

**怎么做**：

```
1. 定义 K 个 HMM（每个代表一个 MECE regime）
   - 状态空间 = regime 子状态（如"扩张初期"、"扩张中期"、"扩张末期"）
   - 观测空间 = PDAToken（含上下文后缀）
   - 转移概率 = regime 内的典型 PDA 转换

2. 训练（Baum-Welch / EM）：
   - 输入：多段行情的 PDA 序列
   - 每段序列分配给 likelihood 最高的 HMM
   - 迭代直到收敛

3. 推断（Viterbi）：
   - 输入：新行情的 PDA 序列
   - 输出：最可能的 regime label + posterior probability
```

**Rust 实现**：
- 扩展现有 `hmmm` crate（加多模型 HMM 混合）
- 或用 `linfa-hmm`（如存在）

**对接 BBN**：
- HMM posterior → `market_regime` root prior
- 序列完整性 → `evidence_quality_score`
- HMM 路径 → `reflection_bundle` 输出"当前走到了扩张中期的第 3 步"

**经典论文**：
- "Clustering Sequences with Hidden Markov Models" (NeurIPS 1997)
- PDF：https://proceedings.neurips.cc/paper/1217-clustering-sequences-with-hidden-markov-models.pdf

---

### 3. Sequence Autoencoder + Embedding Clustering（中长期）

**做什么**：
- LSTM / GRU / Transformer autoencoder
- 输入：变长 PDA 序列
- 输出：固定维度 embedding
- 对 embedding 做 GMM / k-means 聚类

**优势**：
- 自动学习"语义"表示
- 容忍缺失和顺序变异
- embedding 可直接喂给 CatBoost/XGBoost voting layer

**Rust**：
- `tch-rs`（Torch Rust binding）
- `rust-bert` 轻量版

**时机**：Phase 2+，等 DTW + HMM 验证有效后再做。

---

## 实施路径

### Phase 0: DTW PoC（1-2 周，Python 原型）

**目标**：验证 DTW 对 PDA 序列的聚类效果。

**步骤**：
1. 从 ict-engine 导出历史 PDA 序列（`factor-research --emit-pda-sequences`）
2. 实现 PDAToken 定义 + 序列化
3. 用 `tslearn`（Python DTW 库）计算距离矩阵
4. 用 k-medoids / hierarchical clustering 做聚类
5. 可视化 DTW path + cluster 代表序列
6. 检查聚类结果：扩张 vs 震荡是否清晰分离

**产出**：
- `scripts/pda_dtw_poc.py`
- `docs/plans/pda-dtw-poc-results.md`
- 聚类可视化（DTW alignment path + cluster medoid 序列）

**Go/No-Go**：
- 扩张/震荡至少 2 个清晰 cluster，silhouette > 0.5 → 继续
- 特征模糊 → 调整 PDAToken 定义（加更多上下文特征）

### Phase 1: DTW + Clustering Rust 模块（2-3 周）

**目标**：把 DTW + k-medoids 落到 Rust CLI。

**任务**：
1. 新增 `src/pda_sequence/dtw_cluster.rs`：
   - PDAToken 定义 + 序列化
   - DTW 距离计算（用 `dtw` crate 或 `dtw-rs`）
   - k-medoids 聚类（用 `linfa`）
   - 输出 `PDADTWClusterPacket`

2. 接入 `factor-research`：
   - `--cluster dtw` flag
   - 输出 `pda_sequence_clusters.json`（每个 cluster 的典型排列 + DTW 路径解释）

3. 接入 `RegimeSegmentationPacket`：
   - `active_regime_cluster` 填入 DTW cluster label
   - `dtw_alignment_path` 填入对齐路径

4. 接入 PreBayes：
   - cluster regime → `raw_market_regime_label` candidate
   - DTW distance → `evidence_quality_score`（离 medoid 越近，confidence 越高）
   - HMM == cluster → confidence bonus

**产出**：
- `src/pda_sequence/dtw_cluster.rs`
- `src/domain/regime/types.rs`（新增 `PDADTWClusterPacket`）
- `cargo test` pass

### Phase 2: HMM Sequence Clustering 升级（2-3 周）

**目标**：升级现有 HMM 为多模型序列聚类。

**任务**：
1. 扩展 `hmmm` crate：
   - 多模型 HMM 混合（K 个 HMM，每个代表一个 regime）
   - Baum-Welch 训练 + Viterbi 推断
   - 输出 `HMMSequenceRegimePosterior`

2. 接入 BBN：
   - HMM posterior → `market_regime` root prior
   - 序列完整性 → `gating_status`

3. 接入 `factor-research`：
   - `--cluster hmm-sequence` flag

**产出**：
- 升级后的 HMM 模块
- `cargo test` pass

### Phase 3: 全链路集成 + Autoencoder 实验（3-4 周）

**目标**：
1. DTW + HMM 全链路集成
2. Sequence autoencoder PoC

**集成点**：
1. `factor-research` 支持 `--cluster [dtw|hmm-sequence|autoencoder]`
2. `RegimeSegmentationPacket` 填充完整 cluster 信息
3. PreBayes 消费 cluster 证据
4. `reflection_bundle` 输出 regime 解释
5. CatBoost/XGBoost voting layer 消费 embedding（额外特征）

**Autoencoder PoC**：
- 用 `tch-rs` 实现 LSTM autoencoder
- 输入：变长 PDA 序列（padding）
- 输出：固定维度 embedding
- 与 DTW baseline 对比

---

## 与现有系统的对接映射

| 新模块 | 对接点 | 角色 |
|--------|--------|------|
| `PDADTWClusterPacket` | `RegimeSegmentationPacket` | regime classifier |
| DTW alignment path | `reflection_bundle` | 可审计的对齐解释 |
| DTW distance to medoid | `evidence_quality_score` | 证据质量 gate |
| HMM sequence posterior | `BBN market_regime root` | prior adjuster |
| Cluster medoid 序列 | `EntryExecution` | execution plan 生成依据 |
| Autoencoder embedding | `PolicyFeatureVector` | CatBoost/XGBoost extra feature |

---

## 约束（不可违反）

1. **Cluster 不直接下交易命令**：cluster label 只能影响 uncertainty / resonance / gating，不能直接决定 long/short。
2. **DTW/HMM 不取代现有模块**：是 companion surface，不是 replacement。
3. **不硬编码 offline 结果**：不把离线聚类结果当成永真 regime。
4. **先改 regime 层，再改 policy 层**。
5. **新模块不进 main.rs**：走 `src/pda_sequence/` 独立模块。
6. **typed packet 先行**：先定义 schema，再逐步挂模型。

---

## 成功标准

- Phase 0: DTW 聚类产出 ≥2 个清晰 cluster（扩张 vs 震荡），silhouette > 0.5
- Phase 1: `cargo test` pass，DTW path 可解释（"Cluster 0: OB→FVG→Breaker→Propulsion"）
- Phase 2: 多模型 HMM 的 regime 分配比单 HMM 更准确
- Phase 3: 全链路 `factor-research --cluster dtw` 可运行，输出 regime 解释 + execution plan

---

## 总体优势

- **聚类质量大幅提升**：从"静态 GMM"到"时序语义聚类"，扩张 vs 震荡"一眼"分开
- **可审计**：`reflection_bundle` 输出"当前 PDA 序列对齐到 Cluster 2（强扩张 MMXM），缺失 1 个 FVG 已由 DTW 自动补齐"
- **护城河**：NLP 风格的 sequence modeling 是传统 quant library 完全不具备的，匹配"stateful research OS"定位
- **零 GPU 依赖**：DTW + k-medoids 纯 CPU 可跑，可直接集成到 CLI

---

## 与 DNA-Inspired Plan 的关系

| 维度 | DNA Plan | NLP Plan（本文档） |
|------|----------|-----------------|
| 类比源 | DNA 碱基序列 | 自然语言 / 语音序列 |
| 核心方法 | FCGR + Transformer embedding | DTW + HMM sequence clustering |
| 计算需求 | Transformer 需 GPU | DTW 纯 CPU |
| 落地速度 | 中等（需 HuggingFace 模型） | 快（DTW crate 即用） |
| 推荐顺序 | 作为 Phase 2 对比 baseline | **作为首选启动** |

**建议**：先跑 NLP Plan（DTW + HMM），验证 PDA 序列聚类可行后，再用 DNA Plan 的 Transformer embedding 做对比实验。

---

## 参考论文

1. **DTW + 序列聚类**（语音识别经典）
   - DTW 原始论文：Sakoe & Chiba (1978)
   - tslearn Python 库：https://tslearn.readthedocs.io/

2. **HMM Sequence Clustering** (NeurIPS 1997)
   - PDF：https://proceedings.neurips.cc/paper/1217-clustering-sequences-with-hidden-markov-models.pdf

3. **Rust DTW 实现**
   - `dtw` crate：https://crates.io/crates/dtw
   - `dtw-rs`：https://github.com/shshemi/dtw-rs

4. **Rust ML 聚类**
   - `linfa`：https://github.com/rust-ml/linfa
