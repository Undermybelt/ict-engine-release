# ICT Engine Audit Remediation TODO

> Scope: architecture, function surface, runtime loop, tests, user experience, consumer view, and open-source contributor view.
> Rule: no subagent/delegate. Keep fixes repo-versioned and verified.

**Goal:** Turn the current research-grade ICT Engine into a clearer, verifiable, contributor-friendly runtime without breaking the existing research loop.

**Architecture:** Keep the current Rust CLI + library + Python external-research bridge. Reduce `src/main.rs`, formalize command/state contracts, and split validation metrics so users can tell whether the loop is merely runnable or actually mature.

**Tech Stack:** Rust 2021, Clap, JSON state files, Python research scripts, optional CatBoost/XGBoost, GitHub Actions.

---

## Current Evidence

- Repo: `/Users/thrill3r/projects-ict-engine/ict-engine`
- CLI commands: 49
- Rust: 422 files / ~149k LOC
- Python: 183 files / ~41k LOC
- Docs: 194 markdown / ~44k LOC
- Rust tests discovered: ~1161 `#[test]` across 203 Rust files
- CI: `.github/workflows/ci.yml` runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` on Ubuntu and macOS
- Verified during audit:
  - `cargo check --all-targets` passed in 16m33s
  - `ict-engine analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-audit-demo --human` passed
  - `ict-engine factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir /tmp/ict-engine-audit-demo --backend native --human` passed
  - `ict-engine workflow-status --symbol DEMO --state-dir /tmp/ict-engine-audit-demo --human` passed
  - `ict-engine export-structural-path-ranking-target --symbol DEMO --state-dir /tmp/ict-engine-audit-demo` passed
  - `scripts/auto_quant_external/pandas_path_ranker_trainer.py` fallback path produced a direct weighted model when CatBoost was unavailable
- Not completed:
  - `cargo test --no-run` was manually killed after a long compile/lock wait; no failure signal, but no pass signal either

---

## P0 - Loop Truth / Validation Contract

### Problem

`policy-training-status` mixes row-level target maturity with observation-level feedback truth. Prior work recorded 30 structural-feedback records, but status can still show low `raw_scored_mature` because the exported structural path target is de-duplicated by candidate/path rows.

### Risk

Users and agents can overclaim that the external ranker is validated when only replay observations exist, or underclaim when feedback exists but row counters stay low.

### Solution

Split the status surface into two explicitly named metric groups:

- `target_row_validation`: current row-level maturity over `structural_path_ranking_target_history.*`
- `feedback_observation_validation`: observation-level maturity over structural feedback records in `learning_state.feedback_history`

### Steps

- [ ] Add a failing Rust test for `policy-training-status` with repeated feedback observations but few distinct target rows.
  - File: `src/policy_training_command.rs` or the library module that builds the status payload.
  - Expected: status exposes both row-level and observation-level counters.
- [ ] Add a helper that counts eligible structural-feedback observations separately from target rows.
  - Source: `learning_state.feedback_history`
  - Filter: structural path-ranking / structural-feedback source records only.
- [ ] Update `policy-training-status --human` to print both groups.
  - Example: `target_rows raw_scored_mature=2/30 | observations mature=30/30`
- [ ] Update JSON output with stable keys:
  - `target_row_validation.raw_scored_mature`
  - `target_row_validation.production_validation`
  - `feedback_observation_validation.mature_observations`
  - `feedback_observation_validation.outcome_distribution`
- [ ] Update `docs/plans/2026-05-09-vrp-v2-loop-handoff-todo.md` to stop using one counter for both meanings.
- [ ] Verify:
  - `cargo test policy_training -- --nocapture`
  - `cargo check --all-targets`
  - `./target/debug/ict-engine policy-training-status --symbol DEMO --state-dir /tmp/ict-engine-audit-demo --human`

---

## P0 - External Ranker Contract Test

### Problem

The external path-ranking bridge exists, but Rust export/apply/register/runtime and Python trainer fallback are not covered by one small end-to-end fixture.

### Risk

Schema drift can silently break the closed loop. Python may generate scores that Rust accepts incorrectly, or Rust may export columns the trainer no longer handles.

### Solution

Create a minimal fixture-driven contract test for:

`export target -> Python fallback scorer -> apply scores -> register artifact -> enable runtime -> status reflects runtime source`

### Steps

- [ ] Create a tiny fixture target CSV under `tests/fixtures/policy_training/structural_path_ranking_target.csv`.
- [ ] Add a Python test for `pandas_path_ranker_trainer.py --apply` that asserts output row count equals target row count even when no rows are mature.
  - File: `scripts/auto_quant_external/tests/test_path_ranker_contract.py`
- [ ] Add Rust integration test for `apply-structural-path-ranking-external-scores` using the fixture scores.
  - File: `tests/structural_path_ranker_contract.rs`
- [ ] Assert required columns and error messages for missing CSV / malformed score file.
- [ ] Verify:
  - `python3 -m pytest scripts/auto_quant_external/tests/test_path_ranker_contract.py -q`
  - `cargo test structural_path_ranker_contract -- --nocapture`

---

## P0 - `src/main.rs` Reduction

### Problem

`src/main.rs` is ~14,797 lines. A guardrail doc already says not to add business logic there, but the current entrypoint remains the largest maintainability risk.

### Risk

New contributors patch the wrong layer. Reviewers cannot reason locally. Command changes cause accidental reporting/state regressions.

### Solution

Extract command bodies and DTO/report helpers into library/application modules while keeping `main.rs` as Clap declarations + dispatch only.

### Steps

- [ ] Measure current `src/main.rs` line count and add it to `docs/main-rs-guardrails.md` as baseline debt.
- [ ] Pick one low-risk command first: `provider-status`, `artifact-status`, or `pre-bayes-status`.
- [ ] Add/redirect tests to the target library API before moving code.
- [ ] Move command body from `src/main.rs` to the existing `*_command.rs` or `src/application/*` module.
- [ ] Leave only argument matching and shell call in `main.rs`.
- [ ] Repeat in batches until `main.rs < 5000` lines.
- [ ] Verify every batch:
  - `cargo fmt --check`
  - `cargo check --all-targets`
  - `cargo test <moved_command_keyword>`

---

## P1 - Human Output Consistency

### Problem

Some commands support `--human`; some equivalent status/export commands do not. Example: `artifact-status --human` and `export-structural-path-ranking-target --human` reject the flag.

### Risk

Consumer UX feels inconsistent. Agent workflows must special-case JSON-only commands.

### Solution

Define a command output contract: every read-only status/export command supports `--output-format json|compact|agent|human`, or explicitly documents why not.

### Steps

- [ ] Inventory all 49 commands and mark output support.
  - Script target: `scripts/help_audit.py` can be extended.
- [ ] Create `docs/command-output-contract.md` with command matrix.
- [ ] Add `--human` to read-only commands first:
  - `artifact-status`
  - `artifact-lineage`
  - `artifact-diff`
  - `export-structural-path-ranking-target`
  - `apply-structural-path-ranking-external-scores`
- [ ] Add tests that unsupported flags fail only when intentionally unsupported.
- [ ] Verify:
  - `python3 scripts/help_audit.py`
  - `cargo test command_output_contract -- --nocapture`

---

## P1 - First-Run Product Path

### Problem

First-run docs are good, but real users still face too many paths: demo, cleaned JSON, TOMAC data, yfinance, Auto-Quant, provider harness, Python wrappers.

### Risk

Consumers can run a demo but fail to reach a useful real-data loop. They may also pollute repo-local `state/` by omitting `--state-dir`.

### Solution

Create one blessed consumer path and one blessed contributor path.

### Steps

- [ ] Add `docs/consumer-quickstart.md` with exactly three flows:
  - demo: analyze + workflow-status
  - public data: provider-status + analyze-live/yfinance path
  - local cleaned data: analyze with explicit `--data-htf/mtf/ltf`
- [ ] Add `docs/contributor-quickstart.md` with:
  - build/test commands
  - where to add code
  - where not to add code
  - how to run smoke acceptance
- [ ] Change README to link to those two pages near the top.
- [ ] Add warning text near state-dir docs: use `/tmp/...` for trials.
- [ ] Verify by running only commands shown in `consumer-quickstart.md`.

---

## P1 - Smoke Acceptance Script

### Problem

`docs/smoke-acceptance.md` documents the main chain, but no single script enforces it.

### Risk

CI and contributors may pass unit tests while breaking the user-visible loop.

### Solution

Create `scripts/smoke_acceptance.sh` with a fast mode using demo/generated candles and explicit `/tmp` state.

### Steps

- [ ] Create `scripts/smoke_acceptance.sh`.
- [ ] Include these checks:
  - `cargo check`
  - `ict-engine analyze --demo --human`
  - `ict-engine factor-research --backend native --human`
  - `ict-engine workflow-status --human`
  - `ict-engine export-structural-path-ranking-target`
  - `ict-engine policy-training-status --human`
- [ ] Make script refuse repo-local `state/` unless `ICT_ENGINE_ALLOW_REPO_STATE=1` is set.
- [ ] Add README command:
  - `bash scripts/smoke_acceptance.sh`
- [ ] Optional CI job: manual or nightly only, not default PR blocker until runtime is stable.

---

## P1 - Python Script Governance

### Problem

Python scripts include public wrappers, active external trainer code, archived experiments, paper2code prototypes, and local utilities in one broad tree.

### Risk

Consumers cannot tell stable CLI helpers from research prototypes. Contributors may depend on archived scripts accidentally.

### Solution

Add a script classification manifest and enforce wrapper behavior.

### Steps

- [ ] Create `scripts/SCRIPTS.md` with groups:
  - public wrappers
  - active external bridge
  - Auto-Quant strategies
  - archived experiments
  - paper2code prototypes
  - local utilities
- [ ] Add `scripts/script_manifest.json` with `name`, `stability`, `entrypoint`, `safe_default`, `requires_data`, `test_command`.
- [ ] Extend `scripts/help_audit.py` to verify public wrappers default to help/no execution.
- [ ] Add pytest coverage for public wrappers:
  - `search_local.py`
  - `search_cluster.py`
  - `evaluate_bottleneck.py`
  - `research_verdict.py`
- [ ] Verify:
  - `python3 -m pytest scripts/research/tests scripts/auto_quant_external/tests -q`

---

## P1 - Error Message Contract

### Problem

Several CLI failures expose raw IO errors. Example: applying nonexistent external scores prints only `No such file or directory`.

### Risk

Users do not know required schema, command order, or recovery path.

### Solution

Wrap high-friction IO errors with context and next command hints.

### Steps

- [ ] Add tests for missing file errors in:
  - `apply-structural-path-ranking-external-scores`
  - `register-structural-path-ranking-trainer-artifact`
  - `factor-research --data`
  - `analyze --data-*`
- [ ] Replace raw errors with `anyhow::Context` messages.
- [ ] Include expected schema/document path where relevant.
- [ ] Verify expected output contains:
  - missing path
  - expected file type
  - next command or doc link

---

## P2 - Agent / Contributor Truth Map Cleanup

### Problem

`AGENTS.md` contains a stale conflict: E/F/H are listed as active compute stubs in one table, but still described as missing in the design-gap table.

### Risk

Agents and contributors may make wrong claims about available factors.

### Solution

Update `AGENTS.md` and `docs/factor-catalog.md` so every factor family has one current status.

### Steps

- [ ] Update E/F/H in `AGENTS.md` design-gap table:
  - not missing category
  - active compute stub
  - list remaining quality/completeness gap
- [ ] Update `docs/factor-catalog.md` to match.
- [ ] Add a small grep/check script or doc test that ensures all `FactorCategory` variants appear in both docs.
- [ ] Verify:
  - `cargo test factor_registry -- --nocapture`
  - doc check script passes

---

## P2 - Release / Open Source Contribution Flow

### Problem

Release docs mention a private mirror and historical v0.0.1 flow while `Cargo.toml` says `0.1.0`. There is no standard `CONTRIBUTING.md`.

### Risk

Open-source contributors do not know whether to target source repo or mirror, which tests are mandatory, or how releases are versioned.

### Solution

Add a public-facing contribution contract and reconcile release/version docs.

### Steps

- [ ] Create `CONTRIBUTING.md`.
- [ ] Include mandatory checks:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test`
  - selected Python pytest if touching scripts
- [ ] Add architecture placement rules from `docs/main-rs-guardrails.md`.
- [ ] Clarify release repo vs source repo in `docs/release-mirror-runbook.md`.
- [ ] Align `Cargo.toml` version notes and release docs.

---

## Research / GitHub Search Backlog

All above items have direct local solutions. No external search is required before starting.

Use external search only if these local approaches fail:

- [ ] If path-ranking validation design remains unclear, search papers for:
  - off-policy evaluation for contextual bandits
  - inverse propensity scoring trading strategy evaluation
  - doubly robust policy evaluation financial trading
- [ ] If command contract tooling becomes heavy, search GitHub for:
  - Rust clap snapshot testing
  - insta snapshot tests CLI output
  - assert_cmd predicates CLI tests
- [ ] If Python script governance needs a template, search GitHub for:
  - research repo script manifest
  - ML experiment script registry
  - cookiecutter data science command layout

Preferred likely crates/tools to evaluate first if needed:

- `assert_cmd` for Rust CLI command tests
- `predicates` for stdout/stderr assertions
- `insta` for snapshot testing
- `trycmd` for markdown-like CLI examples

---

## Execution Order

1. P0 loop truth / validation contract
2. P0 external ranker contract test
3. P0 `main.rs` first extraction batch
4. P1 human output consistency
5. P1 smoke acceptance script
6. P1 first-run docs
7. P1 Python script governance
8. P1 error message contract
9. P2 doc truth-map cleanup
10. P2 contribution/release cleanup

---

## Done Definition

- `cargo check --all-targets` passes
- `cargo clippy --all-targets -- -D warnings` passes
- `cargo test` passes or documented known slow/blocked subset is isolated
- Python tests pass for touched scripts
- Smoke script passes from a clean `/tmp` state dir
- README points consumers and contributors to distinct quickstarts
- `policy-training-status` no longer conflates target-row maturity with feedback-observation maturity
- `src/main.rs` is shrinking, not growing
