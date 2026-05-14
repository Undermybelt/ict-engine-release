# DEBUG

## 2026-05-03 cargo check blockage

### Scope

- Repo: `projects-ict-engine/ict-engine`
- Current objective lane: `support/docs/plans/2026-05-03-repo-action-board.md`
- Current blocker: `cargo check --lib` does not complete after the recent structural path-ranker runtime changes, so runtime verification is blocked.

### Reproduction

Command:

```bash
cargo check --lib
```

Observed behavior:

- Command prints `Checking ict-engine v0.1.0 (...)`.
- No Rust error or warning output follows.
- `rustc --crate-name ict_engine src/lib.rs --emit=dep-info,metadata ...` remains alive for many minutes with near-zero CPU.

### Recent changes reviewed

Recent commits before the current debug pass:

```text
d79e408 feat: add direct structural path ranker runtime
f5cba45 refactor: move structural transition refresh into regime filter
b18e14f refactor: extract structural beta update helpers
afc1a16 refactor: move experience prior helpers into belief core
9329a79 refactor: move target policy context helpers into belief core
a8cf9a5 refactor: route temporal summary builder through belief core
f88aca1 refactor: move temporal helpers into regime filter
3518731 refactor: move duration prior rebuild into changepoint gate
```

### Environment evidence

- Disk free space is not the bottleneck:
  - `/System/Volumes/Data` available: about `90Gi`
- `target/` exists and the hung `rustc` holds:
  - `target/debug/incremental/.../*.lock`
  - `target/debug/incremental/.../dep-graph.part.bin`
- `rustc` shows near-zero CPU while hung.
- `cargo clean -p ict-engine` also entered a non-progressing state during one attempt.

### Process evidence

Observed process shape during the hang:

```text
cargo check --lib
└── rustc --crate-name ict_engine src/lib.rs --emit=dep-info,metadata ...
```

Observed `lsof` highlights for the hung `rustc`:

- open incremental lock file under:
  - `target/debug/incremental/ict_engine-.../*.lock`
- open dep graph partial file under:
  - `target/debug/incremental/ict_engine-.../dep-graph.part.bin`

### Root-cause hypotheses

1. Incremental/target I/O state is corrupted or blocked.
   - Supporting:
     - `rustc` is not using CPU.
     - open files are concentrated in incremental state.
     - `cargo clean -p ict-engine` also stalled once.
   - Conflicting:
     - not yet proven by a successful fresh build outside the current `target/`.

2. There is a repo-local compile error that rustc never flushes because the process is stuck before diagnostics are emitted.
   - Supporting:
     - recent changes touched several central modules.
   - Conflicting:
     - no diagnostics appear at all.
     - process profile looks like wait/block, not active type checking.

3. macOS file-system or toolchain-level locking around the existing target directory is preventing progress.
   - Supporting:
     - both `cargo check` and one `cargo clean -p ict-engine` attempt stalled.
   - Conflicting:
     - not yet verified against a clean alternate `CARGO_TARGET_DIR`.

### Next debug tests

1. Re-run `cargo check --lib` with a fresh alternate target dir outside the current `target/`.
2. If that succeeds, treat the current blocker as stale/corrupted target state and continue verification from the fresh target dir.
3. If that still hangs, gather a lower-level macOS stack/tool trace and reduce the compile surface further.

### Result of hypothesis test

Test run:

```bash
CARGO_TARGET_DIR=/tmp/ict-engine-cargo-check-1777794602 cargo check --lib
```

Result:

- Completed successfully.
- During this run `rustc` used CPU normally instead of sleeping on incremental state.
- The first real compile error surfaced only after moving off the repo-local `target/`, and that wiring regression has since been fixed.

Updated conclusion:

- The original “hang forever” symptom is strongly tied to the repo-local `target/` / incremental state, not to the recent structural path-ranker changes alone.
- Current verification workaround: use a fresh `/tmp` `CARGO_TARGET_DIR` for compile/test runs until the repo-local target state is cleaned up safely.
