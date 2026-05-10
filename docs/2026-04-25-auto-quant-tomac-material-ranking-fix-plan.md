# 2026-04-25 auto-quant Tomac material ranking fix plan

## Goal

在不扩大运行面、不引入外部依赖、不修改 handoff 以外执行层的前提下，对 `auto_quant` 的外部策略素材 adapter 做一次最小修正，使其更适合真实 Tomac 目录的 seed guidance。

## Why this fix is needed

真实验证表明当前实现存在两个问题：

1. `top materials` 主要按 `trade_rows` 排序，更像“大 CSV 排行榜”，不够像 seed guidance 排行榜
2. 简单 `material_key` 规则无法覆盖少量低风险 family 命名变体，例如 market suffix 或少数版本后缀

## Constraints

- 只改 `src/application/auto_quant/strategy_materials.rs`
- 不修改 Auto-Quant Python 生命周期脚本
- 不修改 Tomac 外部目录
- 不新增 CLI 参数
- 不引入模糊匹配或大范围命名猜测
- 仅支持少量显式、低风险 family 规则

## Planned changes

### 1. Seed-aware ranking

将素材排序从“纯大样本优先”调整为“足够证据 + richer evidence + 可读策略名”优先，仍保留 csv presence 与结果规模作为后续 tie-break。

优先级方向：

1. 有 csv 证据
2. trade rows 达到最低可参考阈值
3. 存在 `Score` 类 richer evidence
4. 策略名更可读、更不偏纯数字参数串
5. 再看 pnl / trade rows

### 2. Low-risk family matching

仅补少量显式 family 规则：

- csv 侧 market suffix base alias：`_es` / `_nq` / `_ym` / `_eur` / `_xau`
- strategy 侧 edition fallback：`_pro` / `_final` / `_v2` / `_v3`

规则要求：

- 先尝试 exact key
- exact 未命中时才尝试 fallback
- 不做泛化模糊匹配

### 3. Focused tests

补定向测试，至少覆盖：

- richer evidence / readable name 能压过纯数字大样本名
- `90wr1.5rrr_strategy.py` 可借 market-suffixed csv 命中家族证据
- 原有基础 discovery / empty-root 行为不回归

## Acceptance

- 真实 Tomac 目录中更语义化、可解释的策略更有机会进入 top materials
- family 命中能力提升，但没有扩大成高风险模糊匹配
- 定向测试通过
