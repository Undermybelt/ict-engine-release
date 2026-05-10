# Release Notes Draft

Version: `v0.1.1` candidate
Status: release mirror candidate, refreshed 2026-05-10

## Highlights

- Rust CLI release gates are currently green:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test`
- First-run demo path works with an explicit `/tmp` state dir:
  - `cargo run -- analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-first-run-native --human`
- Factor diagnostics and native research path are usable on bundled demo data:
  - `factor-pipeline-debug --symbol DEMO --data examples/demo/demo-15m.json --factor structure_ict --objective expansion_manipulation`
  - `factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-first-run-native --backend native --human`
- Public Python wrappers remain safe to inspect before execution:
  - `python3 scripts/search_local.py --show-config`
  - `python3 scripts/search_cluster.py --show-config`
  - `python3 scripts/evaluate_bottleneck.py --show-config`
- Workflow snapshots now preserve canonical structural regime posterior fields on analyze snapshots, matching research/backtest/update surfaces.
- Multi-timeframe and factor-backtest runtime inputs were tightened into structured input types, keeping Clippy clean without widening allow-lints.
- Release mirror runbook is now variableized and no longer hardcodes the old `v0.0.1` tag.

## Smoke results from 2026-05-10

```bash
cargo fmt --manifest-path /Users/thrill3r/projects-ict-engine/ict-engine/Cargo.toml --check
cargo check --manifest-path /Users/thrill3r/projects-ict-engine/ict-engine/Cargo.toml --all-targets
cargo clippy --manifest-path /Users/thrill3r/projects-ict-engine/ict-engine/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path /Users/thrill3r/projects-ict-engine/ict-engine/Cargo.toml
```

All passed.

```bash
cargo run --manifest-path /Users/thrill3r/projects-ict-engine/ict-engine/Cargo.toml -- analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-first-run-native --human
```

Passed. Output starts with a compact desk summary and recommends a native factor-research next command using the same `/tmp` state dir.

```bash
cargo run --manifest-path /Users/thrill3r/projects-ict-engine/ict-engine/Cargo.toml -- factor-research --symbol DEMO --data /Users/thrill3r/projects-ict-engine/ict-engine/examples/demo/demo-15m.json --state-dir /tmp/ict-engine-first-run-native --backend native --human
```

Passed. Best factor was `trend_momentum`; output stayed human-readable.

```bash
cargo run --manifest-path /Users/thrill3r/projects-ict-engine/ict-engine/Cargo.toml -- auto-quant-status --state-dir /tmp/ict-engine-auto-quant-smoke
```

Passed as readiness reporting. It correctly returned `missing_dependency` / `bootstrap_needed=true` and kept managed paths under `/tmp/ict-engine-auto-quant-smoke/auto-quant/...`.

## Known limitations

- This remains an agent-first / researcher-preview release, not a fully generalized packaged distribution.
- Python research tests were not executed in the current environment because `python3 -m pytest` failed with `No module named pytest`.
- Some Python experiment flows still assume a maintainer-style cleaned-data layout unless `--data-root` is provided explicitly.
- Auto-Quant is optional and reports `bootstrap_needed` until its managed dependency is installed in the selected state dir.
- The bundled demo data has about 52 candles and is intentionally too small for full `backtest` paths that require more history.
- Source development history remains far ahead of its origin; this release is published through the clean tree-state release mirror.

## Release label

`ict-engine v0.1.1`

Reason:
- core Rust CLI gates are green
- first-run demo and native factor loop smoke paths pass
- release-facing docs and runbook are current
- Python/Auto-Quant surfaces are useful but still preview-grade / optional
