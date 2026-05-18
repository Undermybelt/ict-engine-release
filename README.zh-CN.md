[English README](README.md)

# 🦊 ICT Engine ʕ•ᴥ•ʔ

> *把市场数据熬成可审计证据的 Rust 工作台喵~*
> *Agent-first market-structure research from a clean terminal.*

```text
       ╱|、
      (˚ˎ 。7   "今天市场长啥样? 哪条证据能信? 下一步该看哪?"
       |、˜〵
       じしˍ,)ノ  —— 这三件事它都告诉你
```

它**不是**给你押注信号的黑盒  ヽ(`Д´)ﾉ
它**是**一个工作台 ٩(◕‿◕)۶，能回答:

- 🔍 **当前市场状态长什么样?**
- 🪶 **哪些证据强、哪些弱、哪些缺失、哪些过期?**
- 🚦 **系统为什么观察、阻断、或允许一条执行路径?**
- 🧭 **人或 agent 下一步应该检查什么?**

核心 CLI 只需要 Rust ✨ Python / Auto-Quant / 富 provider / 训练器产物 都是 **可选热插拔**面 (｡•̀ᴗ-)✧

---

## 🧬 闭环管线一图流

```text
              ICT Engine 闭环 ʕっ•ᴥ•ʔっ

       ╭──────────────────╮
       │ ① data provider  │ ◀── Yahoo · IBKR · TV/MCP · 本地 fixture
       ╰─────────┬────────╯
                 │ cleaned candles
                 ▼
       ╭──────────────────╮
       │ ② regime 后验    │ ◀── trend / range / transition 概率
       ╰─────────┬────────╯
                 ▼
       ╭──────────────────╮
       │ ③ pre-bayes 滤波 │ ◀── evidence quality / soft labels
       ╰─────────┬────────╯
                 ▼
       ╭──────────────────╮
       │ ④ BBN 信念网络   │
       ╰─────────┬────────╯
                 ▼
       ╭──────────────────╮
       │ ⑤ PathRanker     │ ◀── CatBoost 排序结构路径
       ╰─────────┬────────╯
                 ▼
       ╭──────────────────╮
       │ ⑥ 执行树         │ ◀── observe / block / allow
       ╰─────────┬────────╯
                 ▼
       ╭──────────────────╮
       │ ⑦ feedback       │ ◀── realized outcomes
       ╰─────────┬────────╯
                 ▼
       ╭──────────────────╮
       │ ⑧ 训练 / 精修    │ ──╮
       ╰──────────────────╯   │
                 ▲            │
                 ╰─── ↻ 回环 ─╯  (重新喂回 posterior 上下文)
```

> (´｡• ω •｡`) **看图小提示**: 链条上每一节都能用 CLI 命令单独审视, 每个点都有 `--agent` 输出, 给 agent 消费方读结构化字段, 别 grep 展示文案哦~

---

## 🚀 30 秒上手 (｡•̀ᴗ-)✧

> ٩(•́へ•́٩) **铁律**: 第一次跑请用 `/tmp` 状态目录, **千万别脏了仓库!**

```bash
cargo check
cargo run -- --help
cargo run -- analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-first-run --human
```

输出大致长这样 (｡◕‿◕｡):

```text
Structure: ...
Technicals: ...
SMT: ...
Regime: ... posterior_probabilities=range=... stress=... transition=... trend=...
Plan: action=observe ...
```

紧接着看 agent 视角的工作流状态:

```bash
cargo run -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --refresh --agent
cargo run -- pre-bayes-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --refresh --output-format json
cargo run -- policy-training-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --output-format agent
```

> (｡♥‿♥｡) **小贴士**: 试跑都用 `/tmp/...`。只有你**明确**想累积学习状态/artifact 历史时, 才复用同一个 `--state-dir`!

---

## 🧭 命令旅程: 一个用户怎么把 demo 跑透

```text
              USER ʕ•ᴥ•ʔ
                │
                │  Q: 哪些 provider ready / blocked?
                ▼
       ╭─────────────────────╮
       │   provider-status   │   --compact
       ╰──────────┬──────────╯
                  │  Q: 市场现在长啥样?
                  ▼
       ╭─────────────────────╮
       │       analyze       │   --human  → Structure/Tech/SMT/Regime/Plan
       ╰──────────┬──────────╯
                  │  Q: 我下一步该干啥?
                  ▼
       ╭─────────────────────╮
       │   workflow-status   │   --refresh --agent  → next_step
       ╰──────────┬──────────╯
                  │  Q: 证据是脏还是干净?
                  ▼
       ╭─────────────────────╮
       │  pre-bayes-status   │   --refresh → evidence quality
       ╰──────────┬──────────╯
                  │  Q: 训练面够数据了吗?
                  ▼
       ╭──────────────────────────╮
       │  policy-training-status  │   → admission ready?
       ╰──────────────────────────╯
```

> (｡◕‿‿◕｡) 这一串就是 "consumer-safe" 路径: 不要求私有 profile, 默认零配置, 需要 live data 时回退到 Yahoo/yfinance 兼容 provider。

---

## 🍭 能拿到什么 (各种"读回面"一览)

| 表面 | 回答的问题 | 给谁看 |
|---|---|---|
| `provider-status` | 哪些 data/provider 路径 ready · optional · blocked | 🧍🤖 都行 |
| `analyze` | Structure / Technicals / SMT / Regime / Plan 读回 | 🧍 人 |
| `workflow-status` | 当前状态 + next action | 🤖 agent |
| `pre-bayes-status` | evidence quality / 软标签 / 后验输入 | 🤖 agent |
| `policy-training-status` | 训练/准入表面是否有可用数据 | 🤖 agent |
| `factor-candidate-packs` | 可复用 factor candidate 包 | 🧍🤖 |
| `regime-confidence-assets` | 已保留的高置信 regime/source 证据 | 🧍🤖 |

**默认行为对普通用户安全 (｡♥‿♥｡):**

- 🔓 不要求私有 provider profile
- 🚫 不默认复用维护者本地数据集
- 🌐 需要 live data 时零配置回退 Yahoo/yfinance 兼容路径
- 🔌 IBKR / TradingView-MCP / crypto / 本地训练器 → **opt-in**

---

## 🎚️ 输出模式: 同一份命令, 四种 readback

| Mode | 适合谁 | 颜文字脸 |
|---|---|---|
| `--human` | 人在终端看的紧凑读回 | (◕‿◕) |
| `--agent` | agent 消费的结构化字段 | (¬‿¬) |
| `--compact` | 低 token 摘要 (省钱省 context) | (●'◡'●) |
| `--output-format json` | 归档 / 调试 | (｡•̀ᴗ-) |

```bash
cargo run -- provider-status --compact
cargo run -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --human
cargo run -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --refresh --agent
```

> ⚠️ (•́へ•́╮ ) Agent 消费方: **优先读** `decision_summary` / `next_step` / `posterior_probabilities` / artifact ledger 字段, **不要**解析展示文案!

---

## 🛠️ 常见工作流

### 📊 多周期清洗数据分析

```bash
cargo run -- analyze \
  --symbol <SYM> \
  --data-htf <1d.json> \
  --data-mtf <1h.json> \
  --data-ltf <15m.json> \
  --state-dir /tmp/ict-engine-analyze \
  --human
```

### 🔧 诊断 factor 或 gate

```bash
cargo run -- factor-pipeline-debug \
  --symbol <SYM> \
  --data <cleaned-15m.json> \
  --factor structure_ict \
  --objective expansion_manipulation
```

### 🔬 跑原生 factor research

```bash
cargo run -- factor-research \
  --symbol <SYM> \
  --data <cleaned-15m.json> \
  --objective expansion_manipulation \
  --state-dir /tmp/ict-engine-native-research \
  --backend native \
  --human
```

### 🧺 查看 curated candidates

```bash
cargo run -- factor-candidate-packs --symbol FACTOR_CANDIDATES --state-dir /tmp/ict-engine-candidates
cargo run -- factor-candidate-admission-targets --symbol FACTOR_CANDIDATES --state-dir /tmp/ict-engine-candidates
cargo run -- regime-confidence-assets --symbol REGIME_CONFIDENCE_ASSETS --state-dir /tmp/ict-engine-regime-assets
```

> (｡•́︿•̀｡) 这些命令**只**暴露可检查 artifact, **绝不会**单独把 candidate 推进 live execution。

---

## 🌳 Factor 家族图谱 A~H

`ict-engine` 把 factor 切成 8 个家族 (`FactorCategory` enum), 5 个完整 family + 3 个 stub:

```text
                       🌳 FactorRegistry
                              │
            ╭─────────────────┼─────────────────╮
            ▼                 ▼                 ▼
        ✅ active          ✅ active         🟡 stub
            │                 │                 │
   ╭────────┴───────╮ ╭───────┴───────╮ ╭───────┴───────╮
   │ A 结构 ICT     │ │ B 趋势动量    │ │ E 拥挤 herding│
   │ structure_ict  │ │ trend_mom...  │ │ crowd_herd... │
   ├────────────────┤ ├───────────────┤ ├───────────────┤
   │ C 跨市场 SMT   │ │ D 波动均回    │ │ F 频谱混沌    │
   │ cross_smt      │ │ vol_meanrev   │ │ spectral_rhy  │
   ├────────────────┤ ╰───────────────╯ ├───────────────┤
   │ G 期权头寸 *   │                   │ H 流动性会话  │
   │ options_hedge  │                   │ session_liq   │
   ╰────────────────╯                   ╰───────────────╯
                 * G 需 --auxiliary-evidence 数据
```

> (｀・ω・´) **热插拔约定**: 外部 factor 家族走 Auto-Quant 后端 (`--backend auto-quant`), **不需要**在 Rust `FactorCategory` enum 里注册! Rust registry 只是 bootstrap seed, 不是设计边界 ٩(◕‿◕)۶

**代码定位**:

| 角色 | 路径 |
|---|---|
| Factor 定义 + compute | `src/factor_lab/factor_definition.rs` |
| Factor registry | `src/factors/registry.rs` |
| Engine 编排 | `src/factor_lab/engine.rs` |
| 执行树消费 | `src/application/orchestration/execution_tree.rs` |
| BBN 证据消费 | `src/bbn/evidence.rs` |
| HMM/regime 消费 | `src/application/regime/` |

---

## 🐍 可选研究工具 (Python)

> Python wrapper **默认只打印配置**, 传 `--run` 才执行后端! (｡•̀ᴗ-)✧

```bash
python3 support/scripts/search_local.py --show-config
python3 support/scripts/search_cluster.py --show-config
python3 support/scripts/evaluate_bottleneck.py --show-config
```

⚠️ (•́へ•́╮ ) 在维护者工作站之外用时, **显式**传数据根目录, 不要依赖记录过的本地路径!

---

## 🧹 贡献检查 (PR 前必跑)

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test
```

跑完再补一遍消费者路径 smoke:

```bash
cargo run -- provider-status --compact
cargo run -- analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-first-run --human
cargo run -- workflow-status --symbol DEMO --state-dir /tmp/ict-engine-first-run --refresh --agent
```

> (◣_◢) Release candidate 必须用**干净 sanitized export**, 不能直接发宽泛的脏 research worktree!

---

## 🗺️ 仓库地图

| 路径 | 干啥的 |
|---|---|
| `src/` | 🦀 Rust CLI / 分析 / 编排 / provider / 训练表面 |
| `support/examples/` | 📦 公开 demo / provider / factor candidate 示例 |
| `config/` | ⚙️ 小型公开 fixture / config |
| `support/scripts/` | 🐍 可选 Python research wrapper 和 helper |
| `support/docs/README.md` | 📚 文档信任地图和文件夹策略 |
| `support/docs/audits/release-signoff.md` | ✅ 当前 release readiness 记录 |
| `support/docs/release-mirror-runbook.md` | 🪞 私有 release mirror 流程 |
| `AGENT.md` | 🤖 AI agent 在本仓库工作的操作契约 (agent 必读) |

---

## 📦 安装与包管理策略

本项目使用 **PolyForm Noncommercial License 1.0.0**, 目前**不发布公共包管理器 artifact**。

支持的本地路径:

```bash
cargo install --path .
cargo run -- --help
```

| 渠道 | 状态 | 原因 |
|---|---|---|
| ✅ Cargo 本地 `cargo install --path .` | 支持 | 本机用不构成对外分发 |
| 🚫 crates.io | 阻塞 | 公共 registry 需 PolyForm 渠道复核 |
| 🚫 npm / npx | 阻塞 | `npx` 默认拉 npm, 渠道复核未完成 |
| 🚫 Homebrew public tap/core | 阻塞 | 公开 formula 分发源/二进制, 复核未完成 |
| 🟡 私有本地 wrapper | 版权持有人/授权私有用户可用 | 不构成对外分发 |

> (｡╯︵╰｡) 后续要开通公开渠道, **必须**单开 packaging slice 验证 PolyForm 合规和项目 Required Notice。

**策略依据**:

- [Cargo manifest 字段](https://doc.rust-lang.org/cargo/reference/manifest.html): `license` 和 `publish`
- [npm package metadata](https://docs.npmjs.com/cli/v10/configuring-npm/package-json): `license` / `UNLICENSED` / `private`
- [Homebrew license guidelines](https://docs.brew.sh/License-Guidelines): 公开 formula 需可再分发许可证

---

## 🚢 发布策略

开发 checkout 可以脏, 但 release mirror **必须**干净 (｡•̀ᴗ-)✧

- ✂️ 只发布干净、已验证的 export slice
- 🚫 排除生成的 provider cache / Auto-Quant workspace / 本地状态 / 维护者本地路径
- 🔐 默认输出**不得**包含 private key / token / account id / 绝对本地路径
- 📝 发布前刷新 `support/docs/audits/release-signoff.md` 和 `support/docs/release-notes-draft.md`
- 🏷️ 按 `support/docs/release-mirror-runbook.md` 创建 mirror tag 和 GitHub release
- ❌ **未改许可 / 无书面授权**, 不发公共包管理器 artifact

---

## ❓ FAQ

**Q: 不装 Python 能用吗?** (´｡• ᵕ •｡`)
A: 能呀~ 核心 CLI 和 demo 路径**只**需要 Rust。Python 只用于可选 research 和 provider helper。

**Q: `factor-research` 能直接吃原始 CSV 吗?** (｡╯︵╰｡)
A: 不能, 请用**清洗后的 JSON candles**。

**Q: 一个命令能把策略变成可交易吗?** ╮(╯▽╰)╭
A: 不能。Candidate / regime-asset 命令只暴露**证据 / 训练 / 准入**面。Runtime execution 在所需 artifact 和 gate 明确存在前一律 **fail-closed**。

**Q: Agent 应该先读什么?** ʕ•ᴥ•ʔ📖
A: 先读 [`AGENT.md`](AGENT.md), 再用显式 `/tmp` `--state-dir` 跑 `provider-status` / `workflow-status` / `analyze` / `pre-bayes-status` / `policy-training-status`。

---

## 📜 License

本项目使用 **PolyForm Noncommercial License 1.0.0**:

- ✅ 非商业使用、修改、分发 (按许可证)
- ❌ 商业使用需要**另行授权**

详见 [`LICENSE`](LICENSE)。

---

```text
                  ╱|、
                 (˚ˎ 。7    感谢你看到最后~
                  |、˜〵      Happy structuring! ʕ•ᴥ•ʔ ✨
                  じしˍ,)ノ
```
