# First Run Guide

Goal: get a new human or agent from clone to useful output without launching accidental long runs.

## 1. Verify the repo

```bash
cargo check
cargo run -- --help
```

Expected:
- `cargo check` succeeds
- CLI help lists commands such as `analyze`, `factor-research`, `factor-pipeline-debug`, `factor-autoresearch-status`

## 2. Learn the safe command surface

Rust CLI:

```bash
cargo run -- analyze --help
cargo run -- factor-research --help
cargo run -- factor-pipeline-debug --help
cargo run -- factor-autoresearch-status --help
```

Python experiment scripts:

```bash
python3 scripts/search_local.py
python3 scripts/search_cluster.py
python3 scripts/evaluate_bottleneck.py
```

These Python scripts are safe by default: no `--run`, no long experiment.

## 3. Choose the task route

| User intent | Route |
|---|---|
| "What does the market look like?" | `analyze` |
| "Why did the gate fail?" | `factor-pipeline-debug` |
| "What happened in the previous run?" | `factor-autoresearch-status --latest-only` |
| "Compare nearby parameters" | `scripts/search_local.py --run` |
| "Try a bigger research jump" | `scripts/search_cluster.py --run` |
| "Find current blocker" | `scripts/evaluate_bottleneck.py --run` and `scripts/evidence_quality_breakdown.py` |
| "Summarize existing experiment dirs" | `scripts/research_verdict.py <dir...>` |

## 4. Minimum useful debug command

```bash
cargo run -- factor-pipeline-debug   --symbol <SYM>   --data <cleaned-15m.json>   --factor structure_ict   --objective expansion_manipulation
```

Read these fields first:
- `evidence_quality_score`
- `gating_status`
- `bridge_gap`
- `paired_market_quality_report`
- `frame_physics_trace`
- `recommended_actions`

## 5. How to run public experiment scripts

Default help:

```bash
python3 scripts/search_local.py
```

Show backend path:

```bash
python3 scripts/search_local.py --target
```

Show backend summary without executing:

```bash
python3 scripts/search_local.py --backend-help
```

Run:

```bash
python3 scripts/search_local.py --run
```

Same pattern applies to:
- `scripts/search_cluster.py`
- `scripts/evaluate_bottleneck.py`

## 6. Result-reading order

For an autoresearch session:
1. `cargo run -- factor-autoresearch-status --symbol <SYM> --state-dir <dir> --latest-only`
2. `<state_dir>/<SYM>/factor_autoresearch_final.json`
3. `<state_dir>/<SYM>/factor_autoresearch_attempts.json`
4. `factor-pipeline-debug` on the best/current attempt if needed

For isolated comparison:
1. stdout JSON
2. `factor_mutation_runs.json`
3. `scripts/research_verdict.py <state-or-result-dir>`
4. `factor-pipeline-debug` if the score needs explanation

## 7. FAQ

### Why not start with `--run`?

Because some archived backends are long-running and write state. Start with wrapper help first.

### Where should I look after a run finishes?

Start with the current command stdout, then read the relevant state JSON under `<state_dir>/<SYMBOL>/` or the backend `state_*` directory.

### Can I compare experiments from one shared state dir?

No, not for fair comparison. Shared state is for intentional keep/discard loops only.

### Can I feed raw CSV to `factor-research`?

No. Use cleaned JSON candles.

## 8. Common failure modes

### Wrong mutation-spec input

Do not pass:

```text
factor_mutation_runs.json
```

to `--mutation-spec`. It is run history.

### Paired market quality is poor or flat

Read `paired_market_quality_report`. Do not treat a flat/poor paired-data run as strong SMT evidence.

## 9. Agent rule

Before giving a conclusion, ground it in a source-of-truth artifact:

```bash
cargo run -- factor-autoresearch-status --symbol <SYM> --state-dir <dir> --latest-only
```

or the relevant JSON result produced by the current command.
