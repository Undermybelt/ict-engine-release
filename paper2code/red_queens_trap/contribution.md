# Contribution Analysis

## Paper
The Red Queen's Trap: Limits of Deep Evolution in High-Frequency Trading
arxiv: 2512.15732v1

## One-sentence summary
DRL + 进化计算在 HFT 中系统性失败：500 agent 训练 APY>300%，实盘资本衰减>70%，
根因是 aleatoric uncertainty 过拟合、survivor bias、microstructure friction 不可克服。

## Paper type
(e) Theoretical analysis with empirical validation
核心贡献是三个 failure mode 的实证解剖，不是新算法。

## Core contribution to implement
可从论文提取的防御性组件：
1. **Breakeven Win Rate Calculator** — §4.4, Eq.5-6: 给定成本结构，计算最低胜率
2. **Friction-Aware PnL** — §4.1, Eq.4: 净 PnL = 毛利 - 交易成本
3. **Survivor Bias Detector** — §4.2: 检测进化选择中的虚假优胜者
4. **Mode Collapse Monitor** — §4.3: 检测种群同质化 / 端口集中
5. **Capital Decay Tracker** — §4, Figure 1: 实时跟踪资本衰减

## Algorithm specification
- Eq.5: EV = W·(R·Risk) - (1-W)·Risk - C_trans
- Eq.6: W_BE = (1 + C_ratio) / (1 + R)
- Eq.4: PnL_Net = (P_exit - P_entry)×Q - 2×(P×Q×Fee)
- Eq.3: τ_{t+1} = τ_t - 1 + α·I(Profit_t > 0)

## Official code
None found

## Implementation scope

### Will implement:
- Friction barrier calculator (breakeven win rate) — §4.4, Eq.5-6
- Cost-aware PnL computation — §4.1, Eq.4
- Survivor bias detector for factor mutation — §4.2
- Mode collapse / diversity monitor — §4.3
- Capital decay tracker — §4, Figure 1
- Soft budget constraint detector — §4.5

### Will reference (not reimplement):
- LSTM/Transformer architecture (standard, not the contribution)
- Evolutionary algorithm internals (standard)

### Out of scope:
- Rebuilding the full Galaxy Empire simulation
- Binance API integration
- Real-time trading execution
