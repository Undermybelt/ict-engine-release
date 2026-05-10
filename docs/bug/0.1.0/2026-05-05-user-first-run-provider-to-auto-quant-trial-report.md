# 2026-05-05 User First-Run Provider To Auto-Quant Trial Report

## Scope

Pure user-view trial on a local `ict-engine` checkout with no code/test/config edits.

- Repo baseline: `cargo check` passed.
- Trial state root: `/tmp/ict-engine-user-first-run-20260505-150459-*`
- Assumption: no provider credentials, no prior workflow state, no maintainer-only knowledge.

## Commands Executed

```bash
cargo check
./target/debug/ict-engine --help
./target/debug/ict-engine workflow-status --help
./target/debug/ict-engine provider-status --help
./target/debug/ict-engine provider-status --compact
./target/debug/ict-engine provider-status --agent
./target/debug/ict-engine provider-status --provider yfinance --agent
./target/debug/ict-engine provider-status --provider ibkr --agent
./target/debug/ict-engine provider-status --provider tradingview_mcp --agent
./target/debug/ict-engine provider-status --provider bybit_public --agent
./target/debug/ict-engine provider-status --provider binance_public --agent
./target/debug/ict-engine provider-status --provider kraken_public --agent
./target/debug/ict-engine provider-status --domain live_runtime --agent
./target/debug/ict-engine provider-status --provider openbb --agent
./target/debug/ict-engine auto-quant-status --help
./target/debug/ict-engine auto-quant-status --state-dir /tmp/ict-engine-user-first-run-20260505-150459-aq
./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-user-first-run-20260505-150459-demo --human
./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-user-first-run-20260505-150459-demo --agent
./target/debug/ict-engine analyze --help
./target/debug/ict-engine analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-user-first-run-20260505-150459-demo --human
./target/debug/ict-engine backtest --help
./target/debug/ict-engine backtest --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-user-first-run-20260505-150459-backtest --human
./target/debug/ict-engine factor-backtest --help
./target/debug/ict-engine factor-backtest --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-user-first-run-20260505-150459-factor-backtest --human
./target/debug/ict-engine factor-research --help
./target/debug/ict-engine factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-user-first-run-20260505-150459-demo --backend native --human
./target/debug/ict-engine factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-user-first-run-20260505-150459-aqresearch --human
./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-user-first-run-20260505-150459-aqresearch --human
./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-user-first-run-20260505-150459-aqresearch --agent
./target/debug/ict-engine analyze-live --help
./target/debug/ict-engine analyze-live --symbol NQ --state-dir /tmp/ict-engine-user-first-run-20260505-150459-live
./target/debug/ict-engine market-data-harness --help
```

## What Guided Well

1. The repo docs are materially better than the CLI entry surface. `README.md`, `docs/first-run.md`, and the Auto-Quant/IBKR/exchange docs clearly describe the intended architecture and low-pollution expectations.
2. `provider-status` is a real catalog, not a stub. It does detect pending IBKR and TradingView MCP setup and returns concrete install prompts for those missing providers.
3. `auto-quant-status --state-dir /tmp/...` gives a concrete next command and keeps dependency state under the isolated state dir instead of polluting the repo.
4. `analyze --demo --human` is concise and useful once the user already knows to run it. It exposes bias, gate, regime, and a next command.
5. `factor-backtest --human` and native `factor-research --human` both produce compact summaries with a next step.

## What Failed Or Blocked

1. The CLI has no effective start-here surface. Top-level help is a flat command dump, not a first-run guide.
2. `workflow-status --human` dead-ends on clean state with `Next: No actionable command available.` That is the biggest closed-loop failure.
3. The provider catalog does not explain ready providers in user terms. Yahoo Finance, Bybit, Binance, and Kraken are marked ready but not described as free/public/no-login paths.
4. Replay/backtest/live are not presented as a user choice. The app never asks which branch the owner wants first.
5. `analyze --human` jumps directly into native factor research instead of clarifying strategy intent or offering an explicit Auto-Quant branch.
6. `backtest` fails raw on the bundled demo dataset even though the docs already know the dataset is too short.
7. `analyze-live` blocks on three symbol arguments before it explains provider or broker setup.
8. Default `factor-research --human` with Auto-Quant prints a large JSON handoff instead of human output.
9. After the Auto-Quant handoff exists, `workflow-status --human` still says `no_workflow_state`, so the orchestration surface loses the artifact it should be guiding.
10. Human-facing surfaces do not translate downstream evidence closure well. Factor scores and regime show up partially, but BBN/evidence/execution-tree routing stays implicit or agent-only.

## Highest-Priority Problems

1. `workflow-status` does not bootstrap the user from empty state even though provider knowledge is already available.
2. Ready provider surfaces do not tell the user which paths are free/public and which later require login/runtime setup.
3. The app does not present replay/backtest/live as a deliberate first-run choice.
4. The live branch requires insider symbol knowledge before it gives onboarding help.
5. Auto-Quant output-mode parity and workflow closure are broken: `--human` prints JSON, and `workflow-status` does not pick the handoff up.

## Suggested Fix Directions

1. Feed `provider-status --agent` guidance directly into `workflow-status --agent` and `--human` when state is empty or provider setup is blocking.
2. Add a true first-run entry surface:
   - top-level `--help` should say where to start
   - it should map replay/backtest/live to the right commands
   - it should mention `workflow-status` and `provider-status`
3. Enrich ready-provider records with human/agent semantics:
   - free/public/no-login
   - tradfi vs crypto fit
   - when later credentialed/runtime features become necessary
4. Turn common failure walls into next-step guidance:
   - short demo backtest data
   - missing live symbol trio
   - missing provider/runtime setup
5. Make Auto-Quant surfaces honor output format and persist into workflow-status as a real next-step route.

## Overall Outcome

The repo documentation describes a sensible user journey, and several individual command surfaces are strong once chosen correctly. The actual first-run CLI guidance is not yet coherent enough for a new user/agent to discover that journey without external docs or maintainer context.
