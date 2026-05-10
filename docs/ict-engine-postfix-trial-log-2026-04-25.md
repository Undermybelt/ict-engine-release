# ICT Engine Post-Fix Trial Log

**Date:** 2026-04-25  
**Repo:** `/Users/thrill3r/projects-ict-engine/ict-engine`  
**Runtime state:** `/tmp/ict-engine-postfix-trial-20260425` during execution; cleaned after logging  
**Goal:** Re-run the first-run path after the fixes, accept safe machine-suggested next steps, stop at steps that require human approval, and record the outcome without repo-local runtime state.

## Scope

- Use the Rust-only first-run path.
- Keep runtime artifacts under `/tmp`.
- Accept CLI `Next` prompts when they stay inside the repo binary and `/tmp` state.
- Do not clone external repos, install system packages, or choose real user data without human confirmation.
- Do not delete existing repo-local ignored `state/` because it predates this trial and may contain user data.

## Step Log

| Step | Command | Result | Prompt handling |
|---|---|---|---|
| Build check | `cargo check` | Passed, about 8s. | No prompt. |
| Build binary | `cargo build` | Passed, reused existing build. | No prompt. |
| Top help | `./target/debug/ict-engine --help` | Listed expected command surface including `analyze`, `factor-research`, `workflow-status`, Auto-Quant commands, and artifact tools. | No prompt. |
| Analyze help | `./target/debug/ict-engine analyze --help` | Shows `--demo`, `--state-dir`, and output aliases. | No prompt. |
| Research help | `./target/debug/ict-engine factor-research --help` | Shows `--backend <auto-quant|native>` with default `auto-quant`. | No prompt. |
| Demo analyze | `./target/debug/ict-engine analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-postfix-trial-20260425 --human` | Passed. Human output is short. It reported `Bull bias`, `pass_neutralized`, quality `0.617`, and `Action: TUNE trend_momentum`. | Accepted the `Next` prompt because it stayed Rust-only and pointed at demo data with `/tmp` state: `ict-engine factor-research ... --backend native`. |
| Pipeline debug | `./target/debug/ict-engine factor-pipeline-debug --symbol DEMO --data examples/demo/demo-15m.json --factor structure_ict --objective expansion_manipulation` | Passed. JSON is long but useful for diagnostics. Key fields: `evidence_quality_score=0.6173`, `gating_status=pass_neutralized`, `bridge_gap=0.0216`, `pipeline_verdict=pre_bayes_pass_but_bridge_needs_confirmation`. | No prompt. |
| Native research | `./target/debug/ict-engine factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-postfix-trial-20260425 --backend native --human` | Passed. Human output is now a short summary, not JSON. Best factor `trend_momentum`; generated/applied feedback `46/46`. | Did not loop on the repeated research prompt. It is executable and includes `--backend native`, but rerunning immediately would only append another comparable demo run. |
| Workflow status | `./target/debug/ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-postfix-trial-20260425 --human` | Passed. `Block: none`. `Next` is now a direct native research command without duplicate `Next step:` prefix. | No human input required. |
| Demo backtest boundary | `./target/debug/ict-engine backtest --symbol DEMO --data examples/demo/demo-15m.json --human --state-dir /tmp/ict-engine-postfix-trial-20260425` | Expected failure: `need more candles for backtest: got 52, require at least 71`. | No prompt; this is a documented demo-data boundary. |
| Auto-Quant status | `./target/debug/ict-engine auto-quant-status --state-dir /tmp/ict-engine-postfix-trial-20260425` | Passed. Reported `missing_dependency`, `bootstrap_needed=true`, and recommended `ict-engine auto-quant-bootstrap --state-dir /tmp/ict-engine-postfix-trial-20260425`. | Stopped. Bootstrap would clone `https://github.com/TraderAlice/Auto-Quant.git` into `/tmp/.../.deps/auto-quant`, so it needs human approval before execution. |
| Auto-Quant bootstrap (local source) | `./target/debug/ict-engine auto-quant-bootstrap --state-dir /tmp/ict-engine-postfix-trial-20260425-aq --repo-url /Users/thrill3r/Auto-Quant --tracked-branch master` | Passed. Cloned local working copy (`HEAD d143ee67`) into `/tmp/.../-aq/.deps/auto-quant`. Local `/Users/thrill3r/Auto-Quant` was untouched (clean, same HEAD before/after). | Human approved using local repo as source instead of the GitHub URL ("本地已经有了"). Used `--repo-url` override so the original working copy was reused as a Git source without being moved or modified. |
| Auto-Quant status (post-bootstrap) | `./target/debug/ict-engine auto-quant-status --state-dir /tmp/ict-engine-postfix-trial-20260425-aq` | Passed. `dependency_ready_data_missing`. Recommended `uv run .../prepare.py`. | No human input required for the recommendation itself, but execution depended on TA-Lib (system gate, see below). |
| TA-Lib system gate probe | `brew list --versions ta-lib`; `docker --version`; `docker compose version`; `find /Users/thrill3r -name 'libta_lib.*' -o -name 'ta_lib.h'` | TA-Lib not installed via Homebrew. Docker CLI present (`28.0.4`) but daemon not running. No system TA-Lib library on disk. Auto-Quant repo has no `docker-compose.yml`. | Stopped at potential system pollution. Did not run `brew install ta-lib` (system-wide write) or start Docker fallback. Selected an isolated alternative: `uv run --with ta-lib …`, which only writes into uv's user cache. |
| TA-Lib wheel sanity | `uv run --with ta-lib python -c "import talib; print(talib.__version__)"` | Passed. `talib ok 0.6.8`. | No prompt. Confirms wheel-based TA-Lib works without Homebrew or Docker. |
| Auto-Quant prepare data | `uv run --with ta-lib prepare.py` (cwd `/tmp/.../-aq/.deps/auto-quant`) | Passed. Downloaded BTC/ETH/SOL/BNB/AVAX × 1d/4h/1h to `user_data/data/*.feather` (15 files). All writes inside the managed checkout under `/tmp`. | No prompt. Network-bound (Binance public market data). |
| Auto-Quant status (post-data) | `./target/debug/ict-engine auto-quant-status --state-dir /tmp/ict-engine-postfix-trial-20260425-aq` | Passed. `dependency_ready_data_ready`, `healthy=true`, `recommended_next_command="uv run .../run.py"`. | No prompt. |
| Auto-Quant run oracle | `uv run --with ta-lib run.py` (cwd `/tmp/.../-aq/.deps/auto-quant`) | Expected boundary. Exit code 2: `no strategies found in user_data/strategies. Create at least one .py file …`. Only `_template.py.example` is shipped, and `run.py` skips files starting with `_` and only globs `*.py`. | Stopped. Authoring a strategy is the LLM/human research step that ICT Engine wraps; it is intentionally outside the post-fix smoke trial. |

## Fixes Confirmed

- `factor-research --backend native --human` emits concise human text instead of a giant serialized report.
- UTF-8 prompt text is preserved through path redaction.
- Demo analyze no longer asks for a dataset when only one demo path exists.
- Rust-only `Next` commands preserve `--backend native`, avoiding accidental fallback to Auto-Quant.
- `workflow-status --human` no longer prints `Next: Next step: ...`.
- README demo commands now use `/tmp/ict-engine-first-run-native` state.

## Auto-Quant Branch Trial (post human approval)

- **Bootstrap source:** Reused the existing local working copy at `/Users/thrill3r/Auto-Quant` via `--repo-url`. Local repo was confirmed clean (`git status` empty, branch `master`, `HEAD d143ee67`) before bootstrap and unchanged afterwards. ICT Engine cloned a fresh checkout into the managed `.deps/auto-quant` instead of moving or sharing the original working tree.
- **TA-Lib handling:** Avoided `brew install ta-lib` (system-wide) and Docker fallback (daemon not running, no compose file). Used `uv run --with ta-lib …`, which materializes a per-invocation environment in uv's user cache without system writes. `import talib` returned version `0.6.8`.
- **Data preparation:** `prepare.py` downloaded the configured 5 pairs × 3 timeframes from Binance public market data. All 15 `*.feather` files landed inside the managed checkout (`/tmp/.../-aq/.deps/auto-quant/user_data/data/`). No writes outside `/tmp` and no writes into `/Users/thrill3r/Auto-Quant`.
- **Oracle execution:** `run.py` reached the designed boundary because only `_template.py.example` is shipped, and the discovery loop skips files prefixed with `_` and globs `*.py` only. The next natural step is authoring a strategy file under `user_data/strategies/`, which is the LLM/human research loop that ICT Engine wraps; it is intentionally not part of the post-fix smoke trial.
- **ICT Engine view:** `auto-quant-status` advanced through `missing_dependency` → `dependency_ready_data_missing` → `dependency_ready_data_ready`, and the `recommended_next_command` chain matched expectations at every step.

## Pollution Guard

- All deliberate post-fix trial commands used either `/tmp/ict-engine-postfix-trial-20260425` (Rust-only branch) or `/tmp/ict-engine-postfix-trial-20260425-aq` (Auto-Quant branch).
- An earlier compatibility smoke command was run without explicit `--state-dir` while checking the old README path, which updated existing ignored files under `state/DEMO`.
- That `state/` directory already existed before this trial, so it was not deleted or rewritten for cleanup.
- The README demo path now includes an explicit `/tmp` state dir to prevent new users from repeating that repo-local state write.
- Local `/Users/thrill3r/Auto-Quant` was used only as a `--repo-url` source. Verified clean and at the same `HEAD d143ee67` before and after the trial.
- No system-wide installs (no `brew install ta-lib`, no Docker daemon start, no global pip installs). uv cache writes only.
- The `/tmp/ict-engine-postfix-trial-20260425*` trial directories are temporary and removed after this report because every relevant observation is captured here.

## Human Gate

Auto-Quant bootstrap was approved and exercised end-to-end up to the strategy-authoring boundary. The remaining gate is product-design rather than tooling: producing the first `.py` strategy under `user_data/strategies/` is part of the LLM-driven research loop that ICT Engine orchestrates, not part of the first-run smoke trial.

Resolved questions:

```text
Q: Should I run `ict-engine auto-quant-bootstrap` and continue the Auto-Quant branch?
A: Yes — but reuse the local /Users/thrill3r/Auto-Quant via --repo-url; do not mv or modify it.

Q: Should I `brew install ta-lib` or start Docker to satisfy prepare.py?
A: No — used `uv run --with ta-lib` to keep system surface untouched.
```

## Remaining Notes

- `factor-pipeline-debug` is intentionally verbose. It remains a diagnostic surface, not a first-screen onboarding output.
- Demo `backtest` still fails by design because the bundled data has 52 candles and the command requires at least 71.
- The native research `Next` command can be accepted, but doing so repeatedly creates additional comparable research records in the selected state dir. For first-run docs, one native research run plus `workflow-status` is enough.
- `auto-quant-status` does not currently expose a distinct `strategies_missing` state; the boundary surfaces only via `run.py` exit code 2 and stderr text. Worth considering as a follow-up if the first-run experience should hand-hold the user toward authoring `user_data/strategies/<name>.py`.
- TA-Lib remains an implicit Python-side dependency of `prepare.py`/`run.py`. Consider documenting `uv run --with ta-lib …` (or pinning `ta-lib` in Auto-Quant's `pyproject.toml`) so first-run users do not assume Homebrew is required.
