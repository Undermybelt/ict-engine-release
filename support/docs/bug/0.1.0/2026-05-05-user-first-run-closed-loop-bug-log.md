# 2026-05-05 User First-Run Closed-Loop Bug Log

Purpose: record every friction, blocker, redundancy, misleading prompt, missing guidance, or broken closed-loop step found during the pure user-view trial.

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

### [CL-001]

- Command: `./target/debug/ict-engine --help`
- User goal: figure out what to run first after cloning the repo.
- Expected guidance: a clear start-here route such as `workflow-status`, `provider-status`, or an explicit replay/backtest/live chooser.
- Actual behavior: the CLI prints a long flat command catalog with no first-run entrypoint, no route ordering, and no recommendation about replay vs backtest vs live.
- Why this is a blocker or UX regression: a brand-new operator has to guess which of dozens of commands is the real first step.
- Safe fallback found: `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-user-first-run-closed-loop-demo-fresh --human`
- Notes: `README.md` and `support/docs/first-run.md` help, but the first CLI surface itself does not.

### [CL-002]

- Command: `./target/debug/ict-engine provider-status --compact`
- User goal: understand provider choices and what to do next without credentials.
- Expected guidance: a compact summary plus a readable path to focused onboarding details for IBKR, TradingView, Yahoo, Bybit, Binance, and Kraken.
- Actual behavior: the compact surface gives a good one-line provider summary, but it does not tell the user how to get provider-specific setup steps in a human-readable form.
- Why this is a blocker or UX regression: the operator can see that setup is required, but not where the onboarding instructions live unless they already know to query raw JSON by provider id.
- Safe fallback found: `./target/debug/ict-engine provider-status --provider ibkr` and `--provider tradingview_mcp` expose install prompts in JSON.
- Notes: this was verified under a fresh `/tmp` home to avoid maintainer-local config reuse.

### [CL-003]

- Command: `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-user-first-run-closed-loop-demo-fresh --phase bootstrap --human`
- User goal: follow the live/bootstrap route suggested by empty-state `workflow-status`.
- Expected guidance: a short human-readable bootstrap checklist for provider choice, replay/backtest/live intent, and next command selection.
- Actual behavior: the command ignores the intent of `--human` and emits a large JSON document instead of a readable first-run bootstrap surface.
- Why this is a blocker or UX regression: the repo's own first-run route points the user into a machine-facing phase view at the moment they most need concise guidance.
- Safe fallback found: `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-user-first-run-closed-loop-demo-fresh --human`
- Notes: the empty-state human surface itself is useful; the phase jump is what breaks readability.

### [CL-004]

- Command: `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-user-first-run-closed-loop-demo-replay --phase ensemble-vote --human`
- User goal: review the factor/BBN/regime follow-up surfaces recommended by `workflow-status`.
- Expected guidance: readable human summaries for ensemble vote and structural recommended path review.
- Actual behavior: both `--phase ensemble-vote --human` and `--phase structural-recommended-path-bundle --human` output raw JSON rather than a human surface.
- Why this is a blocker or UX regression: the CLI claims the user should review these evidence surfaces before continuing, but the follow-up commands are still machine-oriented and hard to scan in a first-run loop.
- Safe fallback found: the top-level `workflow-status --human` summary explains which evidence surfaces exist, but not in the same level of detail.
- Notes: `pre-bayes-status` also presents JSON-only output, so the evidence-review loop is discoverable but not human-friendly.

### [CL-005]

- Command: `./target/debug/ict-engine analyze-live --symbol NQ --state-dir /tmp/ict-engine-user-first-run-closed-loop-live`
- User goal: start the live/paper-like path after provider selection.
- Expected guidance: a readable live-analysis summary or at least an explicit `--human` route consistent with replay/backtest surfaces.
- Actual behavior: the command emits a very large JSON payload by default, while `analyze-live --help` does not advertise `--human`, `--agent`, or `--compact`.
- Why this is a blocker or UX regression: the empty-state live route points a new operator toward a machine-facing surface with no obvious human-output flag.
- Safe fallback found: `./target/debug/ict-engine workflow-status --symbol NQ --state-dir /tmp/ict-engine-user-first-run-closed-loop-live --human`
- Notes: the underlying zero-config OpenBB live path worked; the readability problem is the user-facing contract.

### [CL-006]

- Command: `./target/debug/ict-engine workflow-status --symbol NQ --state-dir /tmp/ict-engine-user-first-run-closed-loop-live --human`
- User goal: continue from live analysis into historical research using the newly recorded data paths.
- Expected guidance: clearly distinguish the candidate saved paths or give named choices the user can select.
- Actual behavior: the human prompt says `Please choose one historical data path` but redacts the candidate list as repeated `<local-path>` placeholders, then embeds one full real path only inside the trailing command.
- Why this is a blocker or UX regression: the user cannot tell which saved file is which from the candidate list, so the handoff is ambiguous at the exact moment manual choice is required.
- Safe fallback found: inspect the state directory manually and infer the intended `ltf` path from filenames.
- Notes: this is especially confusing because the same response mixes redacted placeholders with one concrete file path.

### [CL-007]

- Command: `./target/debug/ict-engine factor-research --symbol DEMO --data support/examples/demo/demo-15m.json --state-dir /tmp/ict-engine-user-first-run-closed-loop-factor-native --backend native --human`
- User goal: understand whether the app will route an underdefined factor-improvement loop into Auto-Quant.
- Expected guidance: surface Auto-Quant as an explicit optional next path once native factor research finishes or stalls.
- Actual behavior: the human surface loops back into `factor-research --backend native --objective expansion_manipulation` and never mentions Auto-Quant.
- Why this is a blocker or UX regression: a new operator can complete the replay/factor loop without ever discovering the external iteration engine the repo docs describe as part of the closed loop.
- Safe fallback found: explicitly run `./target/debug/ict-engine auto-quant-status --state-dir /tmp/ict-engine-user-first-run-closed-loop-aq-fresh` or rerun factor research with `--backend auto-quant`.
- Notes: the first-run docs mention Auto-Quant, but the native human loop does not surface it.

### [CL-008]

- Command: `./target/debug/ict-engine factor-research --symbol DEMO --data support/examples/demo/demo-15m.json --state-dir /tmp/ict-engine-user-first-run-closed-loop-aq-fresh --backend auto-quant --human`
- User goal: continue the Auto-Quant-assisted iteration path after explicitly choosing the Auto-Quant backend.
- Expected guidance: stay on repo-owned `ict-engine` commands or provide a wrapped prepare/bootstrap step from the CLI itself.
- Actual behavior: the next action jumps out to `uv run --with ta-lib <local-path>`, exposing the managed dependency path and a raw external prepare command.
- Why this is a blocker or UX regression: the user leaves the repo CLI contract mid-flow and must understand `uv`, `ta-lib`, and the external workspace layout before the loop can continue.
- Safe fallback found: `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-user-first-run-closed-loop-aq-fresh --human` at least preserves the artifact id and follow-up review command.
- Notes: `auto-quant-bootstrap` itself worked well; the prepare handoff is the rough edge.
