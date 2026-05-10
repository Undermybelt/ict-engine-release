# 2026-04-25 auto-quant Tomac material ingestion plan

## Goal

将 `/Users/thrill3r/Downloads/Tomac` 中可识别的 `py/csv` 策略素材接入 `ict-engine` 的 `auto-quant` handoff，作为只读 seed material 与结果证据摘要，帮助 agent 在空策略目录或策略迭代时获得更丰富的外部参考。

## Hard constraints

- 不把 Tomac 外部目录复制进 managed Auto-Quant workspace
- 不直接执行 Tomac 下的 Python 脚本
- 不把 Tomac 目录变成 Auto-Quant readiness 的硬依赖
- 不修改 Auto-Quant Python 脚本
- 不扩大 CLI/运行面，除非无法避免

## Low-pollution design

1. 在 `src/application/auto_quant/` 新增只读素材发现模块
2. 复用现有 `Tomac root` 发现机制，而不是新增硬编码路径
3. 扫描外部 root 下的 `.py` 与配对 `.csv`，提炼为小型结构化摘要：
   - strategy name
   - python path
   - csv path
   - trade rows
   - total net pnl
   - TP / SL / BE counts
   - average score（若存在）
4. 将摘要注入 Auto-Quant handoff payload / notes / prompt，作为 seed guidance
5. 仅附带少量 top materials，避免 handoff artifact 膨胀

## Non-goals

- 不把 Tomac 策略直接转换成 Auto-Quant 可执行策略
- 不把 CSV 逐行注入节点证据
- 不将外部素材变成主研究链路的强制前置条件

## Expected outputs

- `auto-quant` handoff payload 能感知并描述 Tomac 外部策略素材
- 空策略目录时，prompt 会优先建议从 Tomac 高证据素材派生 2-3 个 seed strategies
- 测试覆盖：
  - 无 Tomac root 时不报错
  - 有 Tomac root 且存在配对素材时 payload 能附带摘要
  - prompt / notes / suggested commands 对外部素材可见但不强依赖
