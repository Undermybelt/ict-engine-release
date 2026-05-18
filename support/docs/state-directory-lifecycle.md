# State Directory Lifecycle

Default behavior:

- If `--state-dir` is omitted, `ict-engine` uses `ICT_ENGINE_STATE_DIR` when set.
- Otherwise it falls back to `./state`.
- On first auto-creation of `./state` outside an `ict-engine` or Cargo project directory, the CLI prints a warning to `stderr`.

Recommended layout:

- Keep one persistent state directory per experiment stream.
- Treat `state_autoresearch_smoke` as a small reusable smoke baseline.
- Treat other `state_*` directories as disposable session artifacts unless you intentionally preserve them.

Mode choice:

| Mode | Use it for | Avoid it for | Notes |
|---|---|---|---|
| isolated state | parameter comparison, ablation, paired-data checks, one-off research runs | keep/discard loops, resume semantics | isolated state is the default for fair comparison |
| shared state | `factor-autoresearch`, intentional cumulative session loops, resume-latest flows | apples-to-apples comparisons, cross-session benchmarking | shared state changes the meaning of later results |

Bad comparisons to avoid:

- running two candidate specs in the same state dir and then comparing the resulting scores as if they were independent
- reusing a long-lived `state_dir` across unrelated objectives and assuming the later results are comparable to the earlier ones
- reading a shared-state autoresearch improvement as if it were a standalone ablation result

Operational guidance:

- Prefer an explicit path for scripted runs: `--state-dir /tmp/ict-engine-smoke`
- Prefer `ICT_ENGINE_STATE_DIR` for interactive sessions
- Periodically inspect size with `du -sh state*`
- Archive or delete stale `state_*` directories once their artifacts are no longer needed
- If you reuse a `state_dir`, write down why the results are supposed to accumulate; otherwise treat the directory as disposable and isolated.

Cleanup helper:

- `support/scripts/state_cleanup.sh` lists the largest local `state*` directories and suggests removal commands without deleting anything automatically.
