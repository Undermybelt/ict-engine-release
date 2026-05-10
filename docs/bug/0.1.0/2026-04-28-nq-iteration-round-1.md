# NQ Iteration Round 1 Bug Log

Date: 2026-04-28
Version: 0.1.0
Scope: first consumer-side NQ factor iteration attempt using the current CLI and auto-quant path
Rule: log friction and bugs here first; do not hot-fix during the run

## Run Context

- repo: `/Users/thrill3r/projects-ict-engine/ict-engine`
- symbol: `NQ`
- primary data:
  - `/Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-15m/nq.continuous-15m.json`
  - `/Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-1h/nq.continuous-1h.json`
  - `/Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-1d/nq.continuous-1d.json`
- iteration state dir: `/tmp/ict-engine-nq-iteration-20260428`
- auto-quant source preference: local repo at `/Users/thrill3r/Auto-Quant`

## Findings

### 1. Auto-Quant prepare hard-stops on missing TA-Lib

- Step:
  - `./target/debug/ict-engine auto-quant-status --state-dir /tmp/ict-engine-nq-iteration-20260428`
  - `uv run /tmp/ict-engine-nq-iteration-20260428/.deps/auto-quant/prepare.py`
- Result:
  - bootstrap succeeded from local `/Users/thrill3r/Auto-Quant`
  - prepare failed before any factor iteration started
- Raw error:

```text
ERROR: TA-Lib is not installed.

Two install paths (see README.md for full detail):
  1. Native: `brew install ta-lib` then `uv sync`
  2. Docker fallback: `docker compose run --rm freqtrade ...`
```

- User/consumer impact:
  - a consumer agent can reach `auto-quant` bootstrap successfully but still cannot start the first iteration on a normal host without extra system/runtime setup
  - the factor-iteration closed loop is therefore blocked before any setup-level evidence can be judged on win rate / Sharpe
- Immediate action:
  - do not hot-fix in code yet
  - attempt the documented Docker fallback if available on this machine

### 2. `backend=auto-quant` does not execute a run; it only emits handoff payloads

- Step:
  - `./target/debug/ict-engine factor-research ... --backend auto-quant`
  - `./target/debug/ict-engine factor-autoresearch ... --backend auto-quant --iterations 3`
- Result:
  - both commands succeeded syntactically
  - neither started `run.py` nor produced measured win-rate / Sharpe results
  - both only persisted and printed `auto-quant-handoff:*` payloads
- User/consumer impact:
  - from a consumer-agent perspective, the CLI claims `auto-quant` is the backend
    but the closed loop is not actually executed inside this project surface
  - this blocks the intended “throw 30 setups in, iterate, then judge by win
    rate / Sharpe” workflow
- Immediate action:
  - do not hot-fix yet
  - treat `auto-quant` as a handoff surface, not a true execution backend, for this round

### 3. Current managed Auto-Quant config is still crypto-whitelist, not `NQ/USD`

- Step:
  - inspected `/tmp/ict-engine-nq-iteration-20260428/.deps/auto-quant/config.json`
  - inspected local `/Users/thrill3r/Auto-Quant/config.json`
- Result:
  - both configs still whitelist:
    - `BTC/USDT`
    - `ETH/USDT`
    - `SOL/USDT`
    - `BNB/USDT`
    - `AVAX/USDT`
  - no `NQ/USD` entry in config, despite local `user_data/data/NQ_USD-{1h,4h,1d}.feather`
- User/consumer impact:
  - even if data/runtime gates are solved, the current managed Auto-Quant runtime
    is not obviously configured to judge `NQ` as the first-class trial symbol
  - this is a non-universal assumption leak from the external engine side
- Immediate action:
  - do not hot-edit the managed config during this round
  - continue the practical iteration through the native backend to keep the CLI-side loop moving

### 4. `factor-autoresearch` is not first-run runnable without a mutation spec

- Step:
  - `cargo run --quiet -- factor-autoresearch --symbol NQ --data ... --backend native --iterations 3`
- Result:

```text
Error: factor-autoresearch requires --mutation-spec unless --resume-latest is set
```

- User/consumer impact:
  - the command reads like the natural iterative entry point, but a first-run
    consumer cannot use it without already knowing how to author a valid
    mutation spec
  - this breaks the “just run iteration” expectation
- Immediate action:
  - do not patch command behavior yet
  - find the repo’s canonical mutation spec template / authoring path and continue from there

### 5. Native `factor-autoresearch` on full NQ data has no usable progress heartbeat

- Step:
  - seeded a minimal `structure_ict` mutation spec
  - ran:

```text
cargo run --quiet -- factor-autoresearch \
  --symbol NQ \
  --data /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-15m/nq.continuous-15m.json \
  --state-dir /tmp/ict-engine-nq-native-iteration-20260428 \
  --backend native \
  --iterations 3 \
  --mutation-spec /tmp/ict-engine-nq-structure-ict-seed-spec.json
```

- Observation after ~6 minutes:
  - process still active at ~99% CPU
  - `factor-autoresearch-status --latest-only` still showed:
    - `current_iteration = 1`
    - `attempts_total = 0`
    - `updated_at` unchanged from startup
  - no attempt artifact had been appended yet
- User/consumer impact:
  - the loop may still be computing, but from the outside it looks stalled
  - a consumer agent has no trustworthy mid-run signal to distinguish “healthy but slow”
    from “hung before first attempt write”
- Immediate action:
  - stop the long-running foreground process for this round
  - treat missing progress heartbeat / early-attempt checkpointing as a bug candidate

## Remediation Status

Date: 2026-04-28
Change scope: native first-run autoresearch closure + Auto-Quant command-surface repair

### Fixed in this remediation

#### Finding 4. `factor-autoresearch` first run required a mutation spec

- Status: fixed
- Current behavior:
  - first-run native autoresearch now seeds itself automatically
  - `expansion_manipulation` uses a default `structure_ict` seed
  - `generic` uses a default `trend_momentum` seed
- Proof command:

```text
./target/debug/ict-engine factor-autoresearch \
  --symbol NQ \
  --data /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-15m/nq.continuous-15m.json \
  --state-dir /tmp/ict-engine-nq-native-smoke-20260428b \
  --backend native \
  --iterations 1
```

- Proof observation:
  - run started without `--mutation-spec`
  - live snapshot showed:
    - `current_candidate_spec.base_factor = "structure_ict"`
    - `current_candidate_spec.evaluate_expansion_preview = true`

#### Finding 5. Native autoresearch had no usable progress heartbeat

- Status: fixed
- Current behavior:
  - live snapshot now refreshes `updated_at` during an active attempt
  - snapshot now exposes `current_stage`
- Proof command:

```text
./target/debug/ict-engine factor-autoresearch-status \
  --symbol NQ \
  --state-dir /tmp/ict-engine-nq-native-smoke-20260428b \
  --latest-only
```

- Proof observation:
  - `effective_status = "running"`
  - `live_snapshot.current_stage = "running_attempt"`
  - `live_snapshot.updated_at` advanced beyond `started_at`

#### Finding 1. Auto-Quant prepare guidance stopped on a bad default command

- Status: partially fixed
- Current behavior:
  - `auto-quant` readiness / handoff / seed guidance now consistently emits:
    - `uv run --with ta-lib <prepare.py>`
    - `uv run --with ta-lib <run.py>`
- What remains open:
  - host/runtime still needs TA-Lib available through `uv` or system install
  - this remediation fixed the bad command surface, not the underlying third-party dependency

### Still open after this remediation

#### Finding 2. `backend=auto-quant` is still a handoff surface, not an in-CLI execution backend

- Status: open
- Current behavior:
  - `factor-research --backend auto-quant`
  - `factor-autoresearch --backend auto-quant`
  still produce/persist handoff payloads rather than directly executing the external engine loop inside `ict-engine`
- Reason not patched in this change:
  - doing this cleanly requires wiring the additive external runner path into the managed checkout without collapsing the existing control-plane boundary

#### Finding 3. Managed Auto-Quant master config remains crypto-whitelist

- Status: open
- Current behavior:
  - managed `config.json` is still the upstream crypto-oriented file
  - the repo-owned additive path for non-crypto NQ execution still lives separately in:
    - `scripts/auto_quant_external/prepare_external.py`
    - `scripts/auto_quant_external/run_tomac.py`
    - `scripts/auto_quant_external/config.tomac.json`
- Reason not patched in this change:
  - this remediation prioritized unblocking the native `NQ` factor-iteration loop first
  - wiring the additive `NQ/USD` Auto-Quant runner into the public CLI remains follow-up work

### New finding 6. Mid-run `factor-autoresearch-status` hides attempts until the session file exists

- Date observed: 2026-04-28
- Repro path:

```text
./target/debug/ict-engine factor-autoresearch \
  --symbol NQ \
  --data /tmp/ict-engine-nq-2023-trimmed-20260428/nq.continuous-15m.2023plus.json \
  --data-15m /tmp/ict-engine-nq-2023-trimmed-20260428/nq.continuous-15m.2023plus.json \
  --data-1h /tmp/ict-engine-nq-2023-trimmed-20260428/nq.continuous-1h.2023plus.json \
  --data-1d /tmp/ict-engine-nq-2023-trimmed-20260428/nq.continuous-1d.2023plus.json \
  --objective expansion_manipulation \
  --state-dir /tmp/ict-engine-nq-iter-round2-trimmed-20260428-native \
  --backend native \
  --iterations 3
```

Then while the run is still active:

```text
./target/debug/ict-engine factor-autoresearch-status \
  --symbol NQ \
  --state-dir /tmp/ict-engine-nq-iter-round2-trimmed-20260428-native \
  --latest-only
```

- Observed contradiction:
  - `factor_autoresearch_attempts.json` already contains `attempt-001`
  - `factor_autoresearch_live.json` shows:
    - `current_iteration = 2`
    - `current_candidate_spec.mutation_id = ...:next`
  - but status output still shows:
    - `attempts = []`
    - `attempts_total = 0`
    - `latest_attempt_id = null`
- Likely cause:
  - status surface appears to filter attempts through `selected_session_ids`
  - session records are not persisted early enough during an active run
  - so mid-run attempts exist on disk but are hidden from the status view
- Consumer impact:
  - agent sees a healthier heartbeat than before, but still cannot trust the mid-run attempt counts
  - iteration progress is understated until final session persistence lands
- Immediate action:
  - do not hot-fix during this iteration round
  - use the attempts artifact directly as the temporary truth source if mid-run inspection is required
