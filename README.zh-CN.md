# ICT Engine

[English README](README.md)

面向 agent 的市场结构研究 CLI，可以从干净终端开始，把市场数据整理成可审计证据。

`ict-engine` 是 Rust CLI 工作台，用来输出结构证据、技术面上下文、SMT 确认、regime 后验概率、策略/训练状态，以及人和 agent 都能读的执行树回放。

它不是黑盒信号销售器。它回答的是：

- 当前市场状态长什么样；
- 哪些证据强、弱、缺失或过期；
- 系统为什么观察、阻断或允许一条执行路径；
- 人或 agent 下一步应该检查什么。

核心 CLI 只需要 Rust。Python、Auto-Quant、更丰富的数据 provider 和训练器产物都是可选热插拔面。

## 首次运行

克隆后先用 `/tmp` 状态目录跑一遍，不把试跑状态写进仓库：

```bash
cargo check
cargo run -- --help
cargo run -- analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-first-run --human
```

输出形态大致是：

```text
Structure: ...
Technicals: ...
SMT: ...
Regime: ... posterior_probabilities=range=... stress=... transition=... trend=...
Plan: action=observe ...
```

然后查看 agent 会消费的工作流状态：

```bash
cargo run -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --refresh --agent
cargo run -- pre-bayes-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --refresh --output-format json
cargo run -- policy-training-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --output-format agent
```

试跑默认使用 `/tmp/...`。只有你明确想累计学习状态和 artifact 历史时，才复用同一个 `--state-dir`。

## 能得到什么

| 表面 | 回答什么问题 |
|---|---|
| `provider-status` | 哪些数据/provider 路径可用、可选或阻塞 |
| `analyze` | 给人的结构、技术面、SMT、regime 和计划读回 |
| `workflow-status` | 给 agent 的当前状态和下一步 |
| `pre-bayes-status` | 证据质量、软标签和后验输入 |
| `policy-training-status` | 训练/准入表面是否有可用数据 |
| `factor-candidate-packs` | 可复用 factor candidate 包 |
| `regime-confidence-assets` | 已保留的高置信 regime/source 证据 |

默认行为对普通用户安全：

- 不要求私有 provider profile；
- 不默认复用维护者本地数据集；
- 需要 live data 时，零配置路径回退到 Yahoo/yfinance 兼容 provider；
- IBKR、TradingView/MCP、crypto adapters、本地训练器产物等 richer providers 都必须显式选择。

## 输出模式

多数用户面命令支持这些模式：

| 模式 | 适合谁 |
|---|---|
| `--human` | 给人看的紧凑终端读回 |
| `--agent` | 给 agent 的结构化状态和路由 |
| `--compact` | 低 token 摘要 |
| `--output-format json` | 归档和调试 |

示例：

```bash
cargo run -- provider-status --compact
cargo run -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --human
cargo run -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --refresh --agent
```

Agent 消费方应该优先读取 `decision_summary`、`next_step`、`posterior_probabilities` 和 artifact ledger 字段，而不是解析展示文案。

## 常见工作流

分析清洗后的多周期数据：

```bash
cargo run -- analyze \
  --symbol <SYM> \
  --data-htf <1d.json> \
  --data-mtf <1h.json> \
  --data-ltf <15m.json> \
  --state-dir /tmp/ict-engine-analyze \
  --human
```

诊断 factor 或 gate：

```bash
cargo run -- factor-pipeline-debug \
  --symbol <SYM> \
  --data <cleaned-15m.json> \
  --factor structure_ict \
  --objective expansion_manipulation
```

运行 native factor research：

```bash
cargo run -- factor-research \
  --symbol <SYM> \
  --data <cleaned-15m.json> \
  --objective expansion_manipulation \
  --state-dir /tmp/ict-engine-native-research \
  --backend native \
  --human
```

查看 curated candidates：

```bash
cargo run -- factor-candidate-packs --symbol FACTOR_CANDIDATES --state-dir /tmp/ict-engine-candidates
cargo run -- factor-candidate-admission-targets --symbol FACTOR_CANDIDATES --state-dir /tmp/ict-engine-candidates
cargo run -- regime-confidence-assets --symbol REGIME_CONFIDENCE_ASSETS --state-dir /tmp/ict-engine-regime-assets
```

这些命令只暴露可检查 artifact，不会单独把 candidate 推进 live execution。

## 可选研究工具

Python wrapper 默认只打印配置；只有传 `--run` 时才执行后端：

```bash
python3 support/scripts/search_local.py --show-config
python3 support/scripts/search_cluster.py --show-config
python3 support/scripts/evaluate_bottleneck.py --show-config
```

在维护者工作站之外使用时，显式传数据根目录，不依赖记录过的本地路径。

## 贡献检查

提交 PR 或准备 release candidate 前：

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test
```

再跑一次消费者路径 smoke：

```bash
cargo run -- provider-status --compact
cargo run -- analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-first-run --human
cargo run -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --refresh --agent
```

Release candidate 必须使用干净的 sanitized export，不能发布宽泛的脏 research worktree。

## 仓库地图

| 路径 | 用途 |
|---|---|
| `src/` | Rust CLI、分析、编排、provider、训练表面 |
| `support/examples/` | 公开 demo/provider/factor candidate 示例 |
| `config/` | 小型公开 fixture/config 表面 |
| `support/scripts/` | 可选 Python research wrapper 和 helper |
| `support/docs/README.md` | 文档信任地图和文件夹策略 |
| `support/docs/audits/release-signoff.md` | 当前 release readiness 记录 |
| `support/docs/release-mirror-runbook.md` | 私有 release mirror 流程 |
| `AGENT.md` | AI agent 在本仓库工作的操作契约 |

## 安装和包管理策略

本项目使用 PolyForm Noncommercial License 1.0.0，目前不发布为公共包管理器 artifact。

支持的本地路径：

```bash
cargo install --path .
cargo run -- --help
```

包管理渠道策略：

| 渠道 | 状态 | 原因 |
|---|---|---|
| Cargo 本地安装 | 支持 `cargo install --path .` | 本机使用不构成对第三方分发 |
| crates.io | 当前 release flow 阻塞 | 公共 registry 发布需要先做 PolyForm 条款下的渠道合规复核 |
| npm / npx | 阻塞公共 registry | `npx` 通常执行 npm 上的包，发布到 npm 需要先做渠道合规复核 |
| Homebrew public tap/core | 当前 release flow 阻塞 | 公开 formula 会分发源码/二进制，需要先做渠道合规复核 |
| 私有本地 wrapper | 版权持有人或获授权私有用户可用 | 只改善本地 ergonomics，不向第三方授权分发 |

如果以后要公开支持 `npx`、Homebrew、crates.io、Docker 或二进制 release，需要单独开 packaging slice，验证该渠道符合 PolyForm Noncommercial 1.0.0 和项目 Required Notice。

策略依据：

- [Cargo manifest 字段](https://doc.rust-lang.org/cargo/reference/manifest.html)：
  `license` 和 `publish`。
- [npm package metadata](https://docs.npmjs.com/cli/v10/configuring-npm/package-json)：
  `license`、`UNLICENSED` 和 `private`。
- [Homebrew license guidelines](https://docs.brew.sh/License-Guidelines)：
  公开 formula 需要可再分发许可证。

## 发布策略

开发 checkout 可以包含 research 历史和本地实验。Release mirror 不可以。

发布规则：

- 只发布干净、已验证的 export slice；
- 排除生成的 provider cache、Auto-Quant workspace、本地状态和维护者本地路径；
- 默认输出不得泄露 private key、token、account id 或绝对本地路径；
- 发布前刷新 `support/docs/audits/release-signoff.md` 和 `support/docs/release-notes-draft.md`；
- 按 `support/docs/release-mirror-runbook.md` 创建 mirror tag 和 GitHub release；
- 未修改许可证或获得书面授权前，不发布公共包管理器 artifact。

## FAQ

### 不装 Python 能用吗？

可以。核心 CLI 和 demo 路径只需要 Rust。Python 只用于可选 research 和 provider/helper 工作流。

### `factor-research` 能直接吃原始 CSV 吗？

不能。请使用清洗后的 JSON candles。

### 一个命令能把策略变成可交易吗？

不能。Candidate 和 regime-asset 命令只是暴露证据、训练和准入表面。Runtime execution 在所需 artifact 和 gate 明确存在前保持 fail-closed。

### Agent 应该先读什么？

先读 `AGENT.md`，再用显式 `/tmp` `--state-dir` 跑 `provider-status`、`workflow-status`、`analyze`、`pre-bayes-status` 和 `policy-training-status`。

## 许可证

本项目使用 PolyForm Noncommercial License 1.0.0。非商业使用、修改和分发按该许可证允许；商业使用需要另行授权。详见 `LICENSE`。
