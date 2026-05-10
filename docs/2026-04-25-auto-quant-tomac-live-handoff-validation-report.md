# 2026-04-25 auto-quant Tomac live handoff validation report

## Scope

本次验证目标是用真实 `/Users/thrill3r/Downloads/Tomac` 跑一遍 `auto-quant` handoff，检查：

- 外部策略素材是否被只读发现并注入 handoff payload
- 摘要是否足够有用，能作为 seed guidance
- 当前实现是否仍保持显式 opt-in、只读、无硬依赖边界

相关计划见：`docs/2026-04-25-auto-quant-tomac-live-handoff-validation-plan.md`

## Executed command

```bash
ICT_ENGINE_AUTO_QUANT_DIR=/tmp/ict-engine-auto-quant-shared ./target/debug/ict-engine factor-research --backend auto-quant --symbol NQ --data /tmp/nq.json --objective expansion_manipulation --state-dir /tmp/ict-engine-auto-quant-tomac-live-validate-20260425 --strategy-material-root '/Users/thrill3r/Downloads/Tomac'
```

## Runtime result

命令执行成功。

关键运行态事实：

- `strategy_material_root` 被正确记录为 `/Users/thrill3r/Downloads/Tomac`
- `dependency_status.bootstrap_needed = false`
- 本次复用了现有 shared checkout：`/tmp/ict-engine-auto-quant-shared`
- shared checkout 已有 `3` 个 active strategies
- 因此当前 handoff 走的是 `dependency_ready_data_ready`，不是 `seed_required` 分支

本次落盘 artifact 路径：

`/tmp/ict-engine-auto-quant-tomac-live-validate-20260425/NQ/auto_quant_handoff.factor_research.json`

## What was successfully validated

### 1. 真实 Tomac 素材确实被读到了

stdout payload 中成功附带了 `external_strategy_materials`，top 3 为：

1. `optimal_be_1.0`
   - `optimal_be_1.0.py`
   - `optimal_be_1.0_results.csv`
   - `trade_rows = 28766`
   - `net_pnl = 4713785.74`
   - `tp/sl/be = 10517/1604/16461`
2. `98wr0.8rrr41.07pf`
   - `98wr0.8rrr41.07pf.py`
   - `98wr0.8rrr41.07pf.csv`
   - `trade_rows = 26199`
   - `net_pnl = 5258996.78`
   - `tp/sl/be = 16201/952/9018`
3. `no_be_strategy`
   - `no_be_strategy.py`
   - `no_be_results.csv`
   - `trade_rows = 20851`
   - `net_pnl = 6939821.61`
   - `tp/sl/be = 16598/3486/0`

### 2. 摘要数值与真实 CSV 样本表头是对得上的

抽查结果：

- `optimal_be_1.0_results.csv` 表头为 `Net PnL,Result`
- `98wr0.8rrr41.07pf.csv` 表头为 `Time,Net PnL,Result`
- `no_be_results.csv` 表头为 `Net PnL,Result`

这与当前摘要提取逻辑一致：

- `trade_rows`
- `total_net_pnl`
- `TP / SL / BE`

说明当前 CSV 汇总逻辑在真实 Tomac 数据上是可工作的，不是只在测试夹具上成立。

### 3. 边界控制是成立的

本次验证没有发现下面这些污染行为：

- 没有把 Tomac 文件复制进 managed Auto-Quant workspace
- 没有直接执行 Tomac Python 脚本
- 没有把 Tomac 目录写成 readiness 硬依赖
- 没有修改 Auto-Quant Python 生命周期脚本

`agent_prompt` 里也明确保留了只读边界：

- external materials are attached as seed inspiration only
- do not execute those scripts directly
- do not carry absolute-path runtime dependencies into managed workspace

## Summary quality assessment

结论：**当前摘要接入是成功的，但“摘要质量”只能算中等，不算理想。**

### 质量好的地方

- top materials 都有真实 csv 证据，不是空壳路径
- trade count / pnl / tp/sl/be 对 agent 有初步筛选价值
- `no_be_strategy` 这种名称还能传达明显策略差异
- 在 prompt / notes 中可见，不会静默丢失

### 质量不理想的地方

#### 1. 当前 top 3 更像“交易数排行榜”，不是“seed 可读性排行榜”

当前排序逻辑在 `src/application/auto_quant/strategy_materials.rs` 中是：

- 先看是否有 csv
- 再按 `trade_rows` 降序
- 再按 `total_net_pnl` 降序
- 最后按名称

这会导致大样本 CSV 永远优先，即使名字不够可读、不够像可复用 seed。

本次真实结果就体现了这一点：

- `98wr0.8rrr41.07pf` 被选中了，但名字对 seed guidance 的解释性较弱
- `ultimate_ict_strategy.py` / `ict_90wr_1.5rrr_strategy.py` 这类更语义化的策略，没有出现在 top 3

#### 2. richer CSV 会被更大的“粗 CSV”压下去

例如：

- `ultimate_ict_results.csv` 含有 `Score` 列
- 这意味着当前实现本来可以提取 `average_score`

但它只有约 `630` 条记录，而被 `2w+` 记录的大 CSV 压下去了，导致 prompt 中没有展示更细的信息密度。

也就是说，当前排序更偏向“样本量最大”，而不是“最能帮助 seed 设计”。

#### 3. 配对 heuristic 对命名家族不够宽容

当前 `material_key(...)` 只会剥离这些后缀：

- `_strategy`
- `_results`
- `_result`
- `_summary`

这对于简单一对一命名是足够的，但对于 Tomac 这种家族命名会漏掉一些合理配对。

例子：

- `90wr1.5rrr_strategy.py`
- `90wr1.5rrr_ES_results.csv`
- `90wr1.5rrr_NQ_results.csv`

在当前规则下：

- `90wr1.5rrr_strategy.py` -> key = `90wr1.5rrr`
- `90wr1.5rrr_ES_results.csv` -> key = `90wr1.5rrr_es`
- `90wr1.5rrr_NQ_results.csv` -> key = `90wr1.5rrr_nq`

这些 key 不相等，所以不会自动配对。

这不是脏数据问题，而是当前 heuristic 的保守性导致的。

## Additional finding

### 落盘 artifact 里的 `handoff_artifact_path` 仍为空

stdout 返回的 payload 已经带了：

`/tmp/ict-engine-auto-quant-tomac-live-validate-20260425/NQ/auto_quant_handoff.factor_research.json`

但实际落盘文件中的 `handoff_artifact_path` 为空字符串。

这说明当前流程是：

1. 先持久化 payload
2. 再把 `handoff_artifact_path` 填回内存中的 payload
3. stdout 打印的是补齐后的版本
4. 落盘文件还是旧版本

这不属于 Tomac 摘要逻辑本身，但属于 handoff artifact 完整性上的一个小债点。

## Overall conclusion

### 成立的结论

- 真实 Tomac 目录已经可以被当前 adapter 只读发现
- py/csv 摘要提取在真实数据上可工作
- 外部素材已经进入 handoff payload / notes / agent_prompt
- 当前实现仍然满足“显式 opt-in、只读、非执行、非硬依赖”边界

### 不应夸大的结论

- 当前 top materials **不等于** 最佳 seed guidance
- 当前排序更偏“大样本结果文件”，不偏“最可解释、最可派生的 seed”
- 当前配对逻辑 **不等于** 能覆盖 Tomac 目录中的所有合理策略家族

## Minimal follow-up recommendation

如果要做**最小且低债**的下一步，我建议只做两件事：

### 1. 调整 top material 排序，不再只按 `trade_rows`

建议把排序从“纯大样本优先”改成“seed guidance 优先”，例如优先：

- 有 csv
- 有更可读的策略名
- 有 `Score` / richer evidence 列
- trade_rows 足够，但不必绝对最大
- 再看 pnl / tp_count

这样更可能把 `ultimate_ict_strategy` / `ict_90wr_1.5rrr_strategy` 这类更适合派生 seed 的素材推到前面。

### 2. 对 family suffix 做有限归一化

仅针对明显低风险后缀做轻量归一化，例如：

- `_es`
- `_nq`
- `_ym`
- `_eur`
- `_xau`
- `_pro`
- `_final`
- `_v2`
- `_v3`

前提是只在 csv / py 同家族明显成立时使用，不做模糊匹配泛化。

## No-change status

本次任务没有再修改业务代码，只完成了真实 handoff 验证与报告落盘。

## Files produced by this task

- `docs/2026-04-25-auto-quant-tomac-live-handoff-validation-plan.md`
- `docs/2026-04-25-auto-quant-tomac-live-handoff-validation-report.md`
