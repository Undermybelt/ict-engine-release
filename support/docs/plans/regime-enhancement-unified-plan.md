# ICT-Engine Regime 增强 Plan

> 综合 PDA 序列聚类 + Regime Duration 建模 + Wasserstein 区分 + 跨时间框架对齐。
> 
> 基于 7 篇论文 + 4 个 GitHub repo，对 ict-engine 的 regime 检测进行系统增强。

---

## 任务清单

### Task 1: PDA 序列聚类（DNA 启发）

**目标**：把 PDA 序列（OB/FVG/Breaker 等）当 DNA 序列处理，用 k-mer + 无监督聚类发现 regime pattern。

**核心机制**：
- FCGR（Frequency Chaos Game Representation）：变长序列 → 固定 2D 矩阵
- k-mer tokenization：3-mer = "OB→FVG→Breaker" 这种 motif
- 自学习 + majority voting：多网络投票决定 regime label
- DTW 对齐：处理顺序不一致和中间缺失

**代码索引**：
- Repo: [instadeepai/nucleotide-transformer](https://github.com/instadeepai/nucleotide-transformer)
- Repo: [MAGICS-LAB/DNABERT_2](https://github.com/MAGICS-LAB/DNABERT_2)
- Repo: [eonu/sequentia](https://github.com/eonu/sequentia) — Scikit-Learn 兼容 HMM + DTW
- 论文 PDF: [DeLUCS](https://millanp95.github.io/assets/pdf/DeLUCS.pdf)

**可落点**：
- `src/pda_sequence/fcgr.rs` — PDAToken → k-mer 频率矩阵
- `src/pda_sequence/ensemble_cluster.rs` — 多网络 + majority voting
- `PDAClusteringPacket` — 输出 cluster label + confidence

---

### Task 2: Regime Duration 建模（HSMM）

**目标**：显式建模 regime 持续时间，预测"还能撑多久"。

**核心机制**：
- HSMM（Hidden Semi-Markov Model）：在 HMM 基础上增加显式 duration 分布
- Hazard 函数：`λ(d) = P(T=d | T >= d) = PMF(d) / S(d-1)`
- Geometric duration：λ = p（常数 hazard，memoryless）
- Negative Binomial duration：λ(d) 递增（regime 持续越久越可能切换）

**代码索引**：
- Repo: [Jantg/MSM_python](https://github.com/Jantg/MSM_python) ⭐4
- Repo: [Jantg/MSM_particle_filter](https://github.com/Jantg/MSM_particle_filter) ⭐4
- 论文: Calvet & Fisher (2004) — [arxiv.org/abs/cond-mat/0301352](https://arxiv.org/abs/cond-mat/0301352)
- 论文: Yu (2010) — [doi.org/10.1016/j.artint.2009.11.011](https://doi.org/10.1016/j.artint.2009.11.011)

**可落点**：
- `src/regime/hsmm.rs` — HSMM 引擎，支持 Geometric / NegativeBinomial duration
- `DurationState { elapsed, remaining_expected, survival_prob, hazard_rate }`

---

### Task 3: Range vs Choppy 区分（Wasserstein）

**目标**：用 Wasserstein 距离聚类区分 Range（低方差）和 Choppy（高方差），HMM 做不到。

**核心机制**：
- Wasserstein 距离：`W(a,b) = mean(|sort(a) - sort(b)|)`
- WassersteinClusterer：k-medoids 聚类，按方差/均值排序创建 regime 映射
- Hybrid 逻辑：HMM posterior 被 Wasserstein proximity 加权

**代码索引**：
- Repo: [kratu/wess_hmm](https://github.com/kratu/wess_hmm) ⭐11
- 关键文件: `hybrid_regime_infer.py` (1005行) — WassersteinClusterer, RegimeGovernor
- 关键文件: `hybrid_wes_hmm_trainer.py` (665行) — 训练流程, PCA 白化
- 配套 PDF: `Hybrid_Wasserstein_HMM_Regime_Detection.pdf`

**可落点**：
- `src/regime/wasserstein.rs` — WassersteinClusterer
- `src/regime/governor.rs` — 多尺度共识 + 最小持仓 + blip 抑制

---

### Task 4: D1/H1 方向对齐

**目标**：简单判断 D1 和 H1 是否同向，不需要学术级多尺度耦合。

**核心机制**：
- `aligned = (d1_regime == h1_regime)`
- 不需要 PAC 耦合 / Rayleigh 统计量 / Hilbert 变换

**可落点**：
- `src/regime/timeframe.rs` — 3 行逻辑

---

## 公式索引

### PDA 序列（Task 1）
```
k-mer frequency: count(k-mer) / total_k-mers
FCGR: 2D matrix of k-mer frequencies (fixed dimension)
DTW: min_cost_alignment(seq1, seq2) with warping path
```

### HSMM Duration（Task 2）
```
Geometric:
  PMF(d) = p * (1-p)^(d-1)
  S(d) = (1-p)^d
  λ = p (常数)

负二项:
  PMF(d) = C(d+r-1, d) * p^r * (1-p)^d
  p = mean / var
  r = mean * p / (1-p)
  λ(d) 递增
```

### Wasserstein（Task 3）
```
W(a,b) = mean(|sort(a) - sort(b)|)

Governor:
  entropy = -Σ p*log(p)
  confidence = max(posterior)
  commit if: confidence >= 0.20 AND entropy <= 2.0
  min_hold: N bars, Transitional exempt
```

### Timeframe Alignment（Task 4）
```
aligned = (d1_regime == h1_regime)
```

---

## 实施路径

### Phase 0: PoC 验证（1-2 周，Python 原型）

**Task 1 PoC**：
1. 从 ict-engine 导出历史 PDA 序列
2. 实现 PDA→FCGR 转换（~100 行 Python）
3. 用 DeLUCS 的 FCGR + CNN 方法做聚类
4. 检查聚类是否有清晰 regime 特征

**Task 2+3 PoC**：
1. 用 wess_hmm 的 Python 代码跑 NQ 5min 数据
2. 检查 4 态 Wasserstein 聚类是否有清晰特征
3. 对比 HSMM duration 预测 vs 实际 regime 持续时间

**产出**：
- `support/scripts/pda_fcgr_poc.py`
- `support/scripts/regime_wasserstein_poc.py`
- Go/No-Go 判定

### Phase 1: Rust 实现（3-4 周）

**Task 1**：
1. `src/pda_sequence/fcgr.rs` — PDAToken → k-mer 频率矩阵
2. `src/pda_sequence/ensemble_cluster.rs` — 多网络 + majority voting
3. `PDAClusteringPacket` typed packet

**Task 2+3+4**：
1. `src/regime/types.rs` — RegimeState (4态), DurationState, RegimeFeatures
2. `src/regime/wasserstein.rs` — WassersteinClusterer
3. `src/regime/governor.rs` — RegimeGovernor
4. `src/regime/hsmm.rs` — HSMM 引擎
5. `src/regime/hybrid.rs` — HMM + Wasserstein + HSMM 组合
6. `src/regime/timeframe.rs` — D1/H1 对齐
7. `src/regime/mod.rs` — re-export

**产出**：
- `src/pda_sequence/fcgr.rs` + `ensemble_cluster.rs`
- `src/regime/` 完整模块
- `cargo test` pass

### Phase 2: 集成（1-2 周）

**集成点**：
1. `config.rs` 新增 `RegimeConfig` + `PDAConfig`
2. `types.rs` 扩展 `AnalysisOutput`：
   - `regime_hybrid: Option<HybridRegimeOutput>`
   - `pda_cluster: Option<PDAClusteringPacket>`
   - `timeframe_aligned: Option<bool>`
3. `pipeline_builder.rs` 接入：
   - `HybridRegimeEngine` → regime detection
   - `FCGRClusterer` → PDA pattern discovery
   - `timeframe_alignment` → D1/H1 consistency
4. PreBayes 消费 cluster 证据
5. BBN 消费 HMM posterior
6. Execution logic 使用 regime + duration + alignment 调整

---

## 约束

1. **Cluster 不直接下交易命令**：regime label 只能影响 uncertainty / gating，不能直接决定 long/short。
2. **FCGR/Wasserstein 不取代 HMM**：是 companion surface，不是 replacement。
3. **不硬编码 offline 结果**：不把离线聚类结果当成永真 regime。
4. **先改 regime 层，再改 policy 层**。
5. **新模块不进 main.rs**：走 `src/pda_sequence/` 和 `src/regime/` 独立模块。
6. **typed packet 先行**：先定义 schema，再逐步挂模型。

---

## 对接映射

| 新模块 | 对接点 | 角色 |
|--------|--------|------|
| `PDAClusteringPacket` | `RegimeSegmentationPacket` | regime classifier |
| `DurationState` | `RegimeSegmentationPacket` | duration predictor |
| `WassersteinClusterer` | `RegimeFeatures.feature_attribution` | feature selection |
| `HSMMEngine` | `PreBayesEvidenceFilter` | uncertainty adjuster |
| `RegimeGovernor` | `CascadeResult` | gating adjuster |
| `timeframe_alignment` | `AnalysisOutput` | D1/H1 consistency flag |
| FCGR embedding | `PolicyFeatureVector` | CatBoost extra feature |
| DTW alignment score | `evidence_quality_score` | evidence quality gate |

---

## 成功标准

- Phase 0: FCGR 聚类产出 ≥3 清晰 cluster；Wasserstein 区分 Range vs Choppy
- Phase 1: `cargo test` pass；HSMM duration RMSE < 5 bars；ensemble agreement > 0.7
- Phase 2: 全链路 `factor-research` 可运行，输出 regime + duration + PDA cluster 解释

---

## 参考文献

### PDA 序列聚类（Task 1）

1. **DeLUCS** — [PDF](https://millanp95.github.io/assets/pdf/DeLUCS.pdf)
   - 核心：FCGR + 多网络自学习 + majority voting，77-100% 准确率

2. **Nucleotide Transformer** — [Nature (2025)](https://www.nature.com/articles/s41592-024-02523-z) | [GitHub](https://github.com/instadeepai/nucleotide-transformer)
   - 核心：Transformer + multi-head attention，50M-2.5B params，>12k bp

3. **DNABERT / DNABERT-S** — [Paper](https://academic.oup.com/bioinformatics/article/37/15/2112/6128680) | [GitHub](https://github.com/MAGICS-LAB/DNABERT_S)
   - 核心：BERT-style + k-mer tokenization + species-aware embedding

4. **Sequentia** — [GitHub](https://github.com/eonu/sequentia)
   - 核心：Scikit-Learn 兼容 HMM + DTW

### Regime Duration（Task 2+3）

5. **wess_hmm** — [GitHub ⭐11](https://github.com/kratu/wess_hmm)
   - 核心：Hybrid Wasserstein + HMM，production-ready 代码 + PDF

6. **Calvet & Fisher (2004)** — [arXiv](https://arxiv.org/abs/cond-mat/0301352)
   - 核心：MSM regime duration 幂律衰减，Liu-West 粒子滤波

7. **Yu (2010)** — [DOI](https://doi.org/10.1016/j.artint.2009.11.011)
   - 核心：HSMM 理论框架，显式 duration 分布

8. **Guédon (2003)** — [DOI](https://doi.org/10.1016/S0167-9473(02)00166-5)
   - 核心：HSMM 参数估计

9. **Rabiner (1989)** — [DOI](https://doi.org/10.1109/5.18626)
   - 核心：HMM 基础

---

## 已有系统对接文档

- `support/docs/plans/dna-sequence-inspired-pda-clustering-plan.md` — 原始 PDA 聚类 plan
- `support/docs/FRAME_FEATURES.md` — frame feature 定义
- `support/docs/FRAME_FEATURE_EXTENSION.md` — frame feature 扩展
- `src/hmm/` — 现有 HMM 模块
- `src/types.rs` — RegimeProbs, CascadeResult
- `src/pda_sequence/` — 现有 PDA 序列模块

---

Generated: 2026-04-20
