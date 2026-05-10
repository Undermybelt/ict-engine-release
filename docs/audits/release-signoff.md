# Release signoff

Date: 2026-05-10
Status: release mirror target and `v0.1.1` tag selected by operator after whole-repo Rust gate, demo smoke, and release-runbook update.

## Final verdict

Ready to publish to the `Undermybelt/ict-engine-release` mirror as `v0.1.1`.

No Rust CI or first-run smoke blocker remains in the current tree. The operator selected the release mirror target, `v0.1.1` tag, and inclusion of the pending research docs/scripts in the release tree.

## Important release routing decision

This checkout currently tracks:

```text
origin git@github.com:Undermybelt/givenup-ict-engine.git
```

Release metadata still points at:

```text
Undermybelt/ict-engine-release
```

Before publishing, choose one release target:
- source/development repo only
- private release mirror only
- both, in a defined order

## Signoff checklist

### Build and test
- [x] `cargo fmt --check`
- [x] `cargo check --all-targets`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [ ] Python pytest suite: blocked in this environment (`No module named pytest`)
- [x] worktree clean before mirror export signoff

### CLI quality
- [x] root help smoke passed
- [x] `analyze --help` smoke passed
- [x] `factor-research --help` smoke passed
- [x] `analyze --demo --human` emits compact desk-style summary
- [x] `factor-research --backend native --human` emits readable summary

### Portability and state hygiene
- [x] `Cargo.toml` has `license`, `repository`, `authors`
- [x] demo analyze works with explicit `/tmp/...` state dir
- [x] smoke state writes stayed under `/tmp/ict-engine-first-run-native`
- [x] Auto-Quant readiness paths stayed under `/tmp/ict-engine-auto-quant-smoke/auto-quant/...`
- [x] `catboost_info/` added to `.gitignore` as a generated test/training artifact directory
- [x] no tracked `state*` files detected by `git ls-files 'state*'`

### Release closure / closed loop
- [x] `factor-pipeline-debug` on bundled demo data passed
- [x] native `factor-research` on bundled demo data passed
- [x] `auto-quant-status` correctly reports optional dependency readiness and next action
- [x] workflow analyze snapshots preserve canonical structural regime posterior fields

## Commands executed for signoff

```bash
git status --short --branch
git remote -v
git tag --list 'v*' --sort=version:refname
cargo fmt --manifest-path /Users/thrill3r/projects-ict-engine/ict-engine/Cargo.toml --check
cargo check --manifest-path /Users/thrill3r/projects-ict-engine/ict-engine/Cargo.toml --all-targets
cargo clippy --manifest-path /Users/thrill3r/projects-ict-engine/ict-engine/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path /Users/thrill3r/projects-ict-engine/ict-engine/Cargo.toml
cargo run --manifest-path /Users/thrill3r/projects-ict-engine/ict-engine/Cargo.toml -- --help
cargo run --manifest-path /Users/thrill3r/projects-ict-engine/ict-engine/Cargo.toml -- analyze --help
cargo run --manifest-path /Users/thrill3r/projects-ict-engine/ict-engine/Cargo.toml -- factor-research --help
cargo run --manifest-path /Users/thrill3r/projects-ict-engine/ict-engine/Cargo.toml -- analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-first-run-native --human
cargo run --manifest-path /Users/thrill3r/projects-ict-engine/ict-engine/Cargo.toml -- factor-pipeline-debug --symbol DEMO --data /Users/thrill3r/projects-ict-engine/ict-engine/examples/demo/demo-15m.json --factor structure_ict --objective expansion_manipulation
cargo run --manifest-path /Users/thrill3r/projects-ict-engine/ict-engine/Cargo.toml -- factor-research --symbol DEMO --data /Users/thrill3r/projects-ict-engine/ict-engine/examples/demo/demo-15m.json --state-dir /tmp/ict-engine-first-run-native --backend native --human
cargo run --manifest-path /Users/thrill3r/projects-ict-engine/ict-engine/Cargo.toml -- auto-quant-status --state-dir /tmp/ict-engine-auto-quant-smoke
python3 scripts/search_local.py --show-config
python3 scripts/search_cluster.py --show-config
python3 scripts/evaluate_bottleneck.py --show-config
python3 -m pytest scripts/research/tests scripts/auto_quant_external/tests
```

## Decisive outcomes

### Rust gates
- `cargo fmt --check`: passed
- `cargo check --all-targets`: passed
- `cargo clippy --all-targets -- -D warnings`: passed
- `cargo test`: passed
  - lib tests: 910 passed
  - bin tests: 235 passed
  - integration tests: all listed suites passed

### Python wrapper sanity
- `scripts/search_local.py --show-config`: passed
- `scripts/search_cluster.py --show-config`: passed
- `scripts/evaluate_bottleneck.py --show-config`: passed
- `python3 -m pytest ...`: blocked because pytest is not installed in the active Python 3.14 environment

### Demo analyze smoke
- status: passed
- output included compact market/execution/ranker summary
- output included `DEMO | Bull bias | entry=medium | gate=pass_neutralized | quality=0.582`
- recommended next command preserved `/tmp/ict-engine-first-run-native` and `--backend native`

### Native factor research smoke
- status: passed
- output included `Factor research | objective=expansion_manipulation | best=trend_momentum | return=+0.29%`
- output stayed human-readable

### Auto-Quant status smoke
- status: passed as readiness report
- expected readiness: `missing_dependency`, `bootstrap_needed=true`
- managed path stayed under `/tmp/ict-engine-auto-quant-smoke/auto-quant/...`

## Release caveats

1. Branch is still far ahead of the source remote; this release uses the mirror flow to publish clean tree state without rewriting source history.
2. Python pytest suite needs either a Python environment with pytest or an explicit waiver for this preview release; the current environment lacks `pytest`.
3. The release mirror repository is the selected target for `v0.1.1`.

## Release recommendation

Proceed with `v0.1.1` on `Undermybelt/ict-engine-release` using the mirror release flow.
