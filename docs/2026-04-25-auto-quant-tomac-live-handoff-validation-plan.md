# 2026-04-25 auto-quant Tomac live handoff validation plan

## Goal

用真实 `/Users/thrill3r/Downloads/Tomac` 跑一遍当前 `auto-quant` handoff，检查外部策略素材摘要的质量、可用性与边界控制是否符合“低污染、只读、显式 opt-in”的要求。

## Validation method

1. 使用独立 `state_dir` 落地本次验证产物，避免污染默认 `state/`
2. 优先复用已有 shared Auto-Quant checkout，避免为了验证额外 clone 新 repo
3. 通过 `ICT_ENGINE_AUTO_QUANT_DIR` 显式指向 shared checkout
4. 运行一次真实 `factor-research --backend auto-quant --strategy-material-root /Users/thrill3r/Downloads/Tomac`
5. 检查以下输出面：
   - stdout handoff payload
   - `external_strategy_materials`
   - `notes`
   - `agent_prompt`
   - 落盘 handoff artifact
6. 按以下维度评估摘要质量：
   - py/csv 配对是否合理
   - trade metrics 是否看起来可信
   - strategy 命名是否可读
   - top materials 是否对 seed guidance 有实际帮助
   - 是否仍保持只读、非执行、非硬依赖边界

## Known risks

- 当前 `auto_quant_status(...)` 会做 `git ls-remote origin`，因此本次验证可能触发一次远端查询
- shared Auto-Quant checkout 已存在 active strategies，因此本次 handoff 未必进入 seed-required 分支；需要重点检查 payload / prompt 中的外部素材摘要，而不是只看 seed-only commands
- Tomac 文件命名可能不完全统一，摘要质量会受当前配对 heuristic 影响

## Deliverables

完成后写报告到 `docs/`，至少包含：

1. 实际执行命令
2. 关键输出与 handoff artifact 路径
3. top external strategy material 摘要
4. 对摘要质量的结论
5. 若发现问题，给出最小修正建议
