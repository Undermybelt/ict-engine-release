# 2026-04-25 auto-quant Tomac seed evidence plan

## Goal

把外部 Tomac `.py/.csv` 从“只出现在 handoff 摘要里”的状态，推进为 `ict-engine` repo 内可审计、可复用、可供后续 seed authoring / iteration 消费的 canonical artifact。

目标不是直接执行或复制 Tomac 脚本，而是把它们收敛成：

- seed evidence artifact
- artifact ledger node
- authoring nutrient packet

## Why this is the right next step

当前真实边界已经确认：

- Tomac 外部材料可被发现并摘要
- Auto-Quant 真正缺的是可进入 `user_data/strategies/*.py` 的 seed authoring 输入
- 直接把 Tomac 脚本复制进 Auto-Quant workspace 会引入高风险兼容性与运行时污染

因此最低负债路径不是“直接跑 Tomac”，而是先把 Tomac 变成 `ict-engine` 内部的可审计 seed evidence，再喂给后续策略 authoring / iteration。

## Constraints

- 只改 `src/application/auto_quant/*`、必要的 `src/main.rs`、以及测试
- 不修改 Auto-Quant Python 生命周期脚本
- 不执行 Tomac 脚本
- 不把 Tomac 目录变成硬依赖
- 不把外部绝对路径依赖写进运行态策略文件
- artifact 必须进入 `artifact_ledger.json`，成为通用节点证据面，而不是私有 JSON

## Planned implementation

### 1. New canonical artifact

新增 `auto_quant` adapter 内的 seed evidence artifact，内容包括：

- external material root
- managed Auto-Quant workspace / strategies dir
- selected top materials
- 每个 material 的：
  - summary metrics
  - seed target file suggestion
  - source excerpt
  - evidence csv excerpt
  - authoring rationale / constraints
- explicit notes: inspiration only, do not execute/copy directly

### 2. Artifact ledger integration

持久化时写入 `artifact_ledger.json`：

- `artifact_kind = auto_quant_seed_material_evidence`
- `source_phase = auto_quant_seed_materials`
- `actionable = true`
- `decision_hint = author_auto_quant_seed_strategies`

这样现有 `artifact-status` / `artifact-lineage` / `research-verdict` 都能看到这类节点。

### 3. Explicit CLI entry

增加显式 CLI 命令，用于在需要时主动把 Tomac 材料收敛成 seed evidence artifact。

该命令：

- 需要显式 `--strategy-material-root`
- 复用当前 managed Auto-Quant workspace config
- 输出 JSON artifact

### 4. Optional auto-persistence from existing auto-quant factor commands

当 `factor-research --backend auto-quant` 或 `factor-autoresearch --backend auto-quant` 带有 `--strategy-material-root` 时，可额外落一份 seed evidence artifact，避免材料再次只停留在 handoff 摘要层。

要求：

- 失败只警告，不中断主 handoff 路径
- 无材料时静默跳过

## Non-goals

- 不生成或复制可执行 Auto-Quant strategy `.py` 文件
- 不直接运行 `uv run run.py`
- 不把外部 Tomac 脚本伪装成已经验证的 mutation / attempt truth
- 不修改 native `factor-autoresearch` truth schema

## Acceptance

- 显式命令可把 Tomac 材料落成 canonical seed evidence artifact
- `artifact_ledger.json` 中可见对应节点
- artifact 内含足够的 source / csv excerpts，能作为后续 seed authoring 的 nutrient packet
- 原有 handoff / adoption / status 行为不回归
