# 2026-05-05 User First-Run Provider To Auto-Quant Bug Log

Purpose: record every friction, blocker, redundancy, misleading prompt, missing guidance, or broken closed-loop step encountered during the pure user-view trial.

Rules:
- Do not fix code in this round.
- Append findings in order of discovery.
- Prefer exact command + exact observed behavior over summary prose.

## Finding Template

### [ID]

- Command:
- User goal:
- Expected guidance:
- Actual behavior:
- Why this is a blocker or UX regression:
- Safe fallback found:
- Notes:

## Findings

<!-- Append findings below this line during the real user-style trial. -->

### F-001

- Command: `./target/debug/ict-engine --help`
- User goal: understand what the app is for and where a first-time user should start.
- Expected guidance: a start-here path that maps user intent to `workflow-status`, provider onboarding, and the replay/backtest/live split.
- Actual behavior: the help output is a flat list of 40+ commands, including internal/experimental surfaces, with no "start here" guidance and no replay/backtest/live decision framing.
- Why this is a blocker or UX regression: a first-time user must already know repo terminology to choose a safe first command.
- Safe fallback found: `README.md` and `docs/first-run.md` describe a safer entry path, but the CLI itself does not surface it.
- Notes: `human-next` is not a top-level subcommand, and `--help` does not point to `workflow-status` or `provider-status` as the guided entry.

### F-002

- Command: `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-user-first-run-20260505-150459-demo --human`
- User goal: ask the app what to do first from a clean state.
- Expected guidance: an actionable first step such as choosing replay/backtest/live, choosing a provider, or using Yahoo Finance / public crypto data if no credentials are present.
- Actual behavior: `Next: No actionable command available.` The only extra line is a maintainer-specific opt-in profile hint: `thrill3r-nq-closed-loop-v1`.
- Why this is a blocker or UX regression: the primary guided-status surface dead-ends exactly when a first-time user needs onboarding.
- Safe fallback found: manually run `provider-status --agent` and read repo docs.
- Notes: this misses the todo's first-run requirement most directly.

### F-003

- Command: `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-user-first-run-20260505-150459-demo --agent`
- User goal: let an agent bootstrap the workflow from empty state.
- Expected guidance: machine-readable `next_step` plus provider install/config prompts derived from the provider catalog.
- Actual behavior: `provider_support.summary_line` is populated, but `provider_support.active=false`, `pending_providers=[]`, `install_prompts=[]`, and `next_step.action_type="none"`.
- Why this is a blocker or UX regression: the agent surface knows providers exist but withholds the actual bootstrap instructions.
- Safe fallback found: call `provider-status --agent` separately and stitch the guidance together outside the intended workflow surface.
- Notes: this is the exact gap the repo memory/docs describe as still open.

### F-004

- Command: `./target/debug/ict-engine provider-status --provider yfinance --agent`
- User goal: confirm the free no-credential fallback when no provider account is available.
- Expected guidance: explicitly say Yahoo Finance is the free historical fallback, what it is good for, and how to choose it.
- Actual behavior: only a generic readiness envelope is returned: `market_data:1/1 ready`, with no install prompts, no "free fallback" note, and no next command.
- Why this is a blocker or UX regression: a first-time user still cannot tell that Yahoo Finance is the default escape hatch for credential-free historical data.
- Safe fallback found: infer it from `README.md` plus the provider id name.
- Notes: the same absence of descriptive guidance appears on other ready providers.

### F-005

- Command: `./target/debug/ict-engine provider-status --provider bybit_public --agent`; `./target/debug/ict-engine provider-status --provider binance_public --agent`; `./target/debug/ict-engine provider-status --provider kraken_public --agent`
- User goal: understand the public crypto data path and whether login is required.
- Expected guidance: tell the user these are public market-data paths, distinguish when later credentialed/runtime features may be needed, and explain which crypto personas each path fits.
- Actual behavior: each provider returns only `market_data:1/1 ready` plus the provider id; there is no explanation of public/no-login access, no crypto-vs-tradfi routing, and no note about Kraken's later credentialed/runtime distinction.
- Why this is a blocker or UX regression: crypto users get no meaningful onboarding and cannot tell which path is truly public versus later account-bound.
- Safe fallback found: read `docs/2026-04-26-multi-exchange-data-source-integration-plan.md`.
- Notes: this is where the app over-assumes maintainer knowledge most strongly for crypto onboarding.

### F-006

- Command: `./target/debug/ict-engine provider-status --compact`
- User goal: get a compact first-run view of which providers are available and what to do next.
- Expected guidance: a short user-oriented summary that highlights free fallbacks, login-required providers, and the next decision.
- Actual behavior: the compact surface reports readiness counts and then exposes the maintainer-specific profile `thrill3r-nq-closed-loop-v1`.
- Why this is a blocker or UX regression: the first-run catalog mixes public onboarding with personal local profile leakage, which is confusing for a fresh user/agent.
- Safe fallback found: ignore the profile line and manually inspect per-provider agent JSON.
- Notes: the same profile leak appears in `workflow-status --human` and `--agent`.

### F-007

- Command: `./target/debug/ict-engine analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-user-first-run-20260505-150459-demo --human`
- User goal: review historical behavior and then learn the next appropriate branch.
- Expected guidance: ask whether the owner wants replay/review, backtest, or live next; if strategy is unclear, offer factor iteration or Auto-Quant explicitly.
- Actual behavior: the command jumps straight to `Next: ict-engine factor-research ... --backend native` without asking which branch the owner wants and without any strategy-clarification prompt.
- Why this is a blocker or UX regression: the closed loop skips the workflow-choice step and silently assumes factor research is the right next action.
- Safe fallback found: manually inspect `backtest --help`, `factor-backtest --help`, and `analyze-live --help`.
- Notes: this is the clearest branch-routing failure after the empty `workflow-status` dead end.

### F-008

- Command: `./target/debug/ict-engine backtest --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-user-first-run-20260505-150459-backtest --human`
- User goal: try the documented bundled demo path for a first backtest.
- Expected guidance: explain that the demo data is too short and offer a concrete fallback such as `factor-backtest`, `analyze --demo`, or fetching a larger dataset.
- Actual behavior: raw failure only: `Error: need more candles for backtest: got 52, require at least 71`
- Why this is a blocker or UX regression: the app knows this bundled dataset is first-run demo material, yet the failure does not route the user anywhere useful.
- Safe fallback found: `factor-backtest` succeeds on the same dataset; the README also warns about the candle-count limit.
- Notes: this is a good example of docs knowing more than the runtime surface reveals.

### F-009

- Command: `./target/debug/ict-engine analyze-live --symbol NQ --state-dir /tmp/ict-engine-user-first-run-20260505-150459-live`
- User goal: try the live branch from a clean user perspective.
- Expected guidance: help the user choose the live mode, backend/provider, and symbol conventions; if local symbols are required, show examples and tell the user what each field means.
- Actual behavior: hard error: `pass --futures-symbol, --spot-symbol, and --options-symbol`
- Why this is a blocker or UX regression: the live path demands three domain-specific symbols before any provider/broker guidance, which is backwards for a first-time user.
- Safe fallback found: read `analyze-live --help` and `provider-status --domain live_runtime --agent` manually.
- Notes: the surface still does not explain whether the user wants data-only live, paper-style live observation, or broker-linked live.

### F-010

- Command: `./target/debug/ict-engine factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-user-first-run-20260505-150459-aqresearch --human`
- User goal: follow the default factor-iteration path and see the Auto-Quant onboarding in human-readable form.
- Expected guidance: either a concise human summary of the Auto-Quant readiness gap or a next-step prompt that says bootstrap/prepare data in plain language.
- Actual behavior: despite `--human`, the command prints a large JSON handoff object. It also auto-bootstraps Auto-Quant into the `/tmp` state dir and then points to `uv run --with ta-lib .../prepare.py` without a human-formatted explanation.
- Why this is a blocker or UX regression: the output mode does not match the requested surface, and the user must parse a large artifact blob to understand the next step.
- Safe fallback found: inspect `auto-quant-status` and read the JSON fields manually.
- Notes: this is the most obvious output-parity break in the first-run Auto-Quant path.

### F-011

- Command: `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-user-first-run-20260505-150459-aqresearch --human`
- User goal: return to the main workflow surface after triggering the default Auto-Quant path.
- Expected guidance: recognize the new Auto-Quant handoff artifact and tell the user what to do next.
- Actual behavior: it still reports `no_workflow_state` and `Next: No actionable command available`, even though the agent JSON shows `top_actionable.artifact_kind="auto_quant_handoff_candidate"`.
- Why this is a blocker or UX regression: the cross-phase status surface loses the Auto-Quant handoff instead of closing the loop around it.
- Safe fallback found: inspect `workflow-status --agent` and the handoff JSON directly.
- Notes: this breaks the repo's stated model of `ict-engine` as the orchestration/control plane.

### F-012

- Command: `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-user-first-run-20260505-150459-demo --human`; `./target/debug/ict-engine factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-user-first-run-20260505-150459-demo --backend native --human`
- User goal: understand how results flow into factor scoring/filtering, BBN evidence, regime scoring, and execution tree surfaces.
- Expected guidance: a plain-language explanation or next-step route that names those downstream surfaces and says what to inspect next.
- Actual behavior: factor scores are partially visible, regime is briefly visible in `analyze --human`, but there is no user-facing guidance into BBN evidence or execution-tree review. `workflow-status --human` only echoes another `factor-research` command.
- Why this is a blocker or UX regression: the app produces internal artifacts but does not guide a new user toward the promised evidence-closure surfaces.
- Safe fallback found: inspect `workflow-status --agent` or deeper artifact commands manually.
- Notes: `workflow-status --agent` does expose `top_actionable.artifact_kind="execution_tree_artifact"` after native runs, but the human surface does not translate that into an actionable next step.
