# Sprint A Stability Floor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the highest-leverage Sprint A stability gaps on the public CLI surface without rewriting the whole codebase in one pass.

**Architecture:** Keep `src/main.rs` as the orchestration entrypoint for now, but harden its command contract around state-dir resolution, output-format handling, and human-readable failure/output surfaces. Add docs and CI in parallel so contributors and agents can discover the behavior without reverse-engineering source code.

**Tech Stack:** Rust 2021, clap 4, serde_json, GitHub Actions, cargo fmt/clippy/test

---

## Scope

- [x] `ICT_ENGINE_STATE_DIR` support on CLI state-bearing commands
- [x] First-run warning for implicit `./state`
- [x] `backtest` / `factor-backtest` / `factor-research` output-format wiring
- [x] `duration_sizing_scale` persistence fallback fix
- [x] `ict-engine env` command and env-var docs
- [x] CI workflow for fmt/clippy/test
- [x] State lifecycle docs and cleanup helper
- [ ] Full panic-on-boundary unwrap audit across every CLI/IO/JSON path
- [ ] Full `NextCommand` typed migration across persisted state structs

## Notes

- Existing work on compare/human-output surfaces is preserved and reused.
- Remaining Sprint A debt that spans large parts of `main.rs` is intentionally left for a follow-up pass to avoid colliding with unrelated in-flight changes already present on `green-baseline`.
