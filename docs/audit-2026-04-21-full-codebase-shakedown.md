# ict-engine Full Codebase Shakedown Audit · 2026-04-21

> Author: Codex (this session)
> Repo state audited: local `green-baseline` worktree on April 21, 2026
> Scope: whole repo from open-source contributor, released CLI user, and released agent-consumer perspectives
> Relationship to prior audit: complements [docs/audit-2026-04-21-cross-surface-review.md](./audit-2026-04-21-cross-surface-review.md) with fresh whole-repo command simulations, contributor-first-run checks, and agent/user shakedown coverage

---

## 0. Executive Summary

The project is materially more usable than the earlier cross-surface audit baseline:

- `cargo check --all-targets` passes
- `cargo test` passes
- core demo analyze and workflow surfaces run end-to-end
- state-dir/env/output-format stabilization work is visible in the local tree

But the repo is not yet release-clean.

The biggest remaining risks are not catastrophic crashes on happy paths. They are mismatch problems:

- help/docs/output surfaces are inconsistent across commands
- human-facing commands sometimes leak agent protocol strings or huge machine payloads
- agent-facing status/research commands sometimes produce placeholder or misleading outputs when no real state exists
- contributor expectations are mismatched with current lint/test reality

I ran 16 concrete scenarios. New bug categories stopped appearing in the final 4 runs; the later runs mostly confirmed already-seen problems.

---

## 1. Method

### Static verification

- `cargo check --all-targets`
- `cargo test`
- `cargo clippy --all-targets --no-deps`
- README and docs review
- existing audit review

### Scenario simulation

I simulated contributor, user, and agent flows with these commands:

1. `cargo run -- --help`
2. `cargo run -- analyze --help`
3. `cargo run -- factor-research --help`
4. `cargo run -- workflow-status --help`
5. `cargo run -- env`
6. `cargo run -- analyze --symbol DEMO --demo --human`
7. `cargo run -- analyze --symbol DEMO --demo --agent`
8. `cargo run -- workflow-status --symbol DEMO --state-dir state --human`
9. `cargo run -- analyze --symbol NQ --data-htf missing.json --data-mtf missing.json --data-ltf missing.json`
10. `cargo run -- analyze --symbol DEMO --demo --human --output-format json`
11. `cargo run -- factor-pipeline-debug --symbol DEMO --data examples/demo/demo-15m.json --factor structure_ict --objective expansion_manipulation`
12. `cargo run -- workflow-status --symbol DEMO --state-dir state --agent`
13. `cargo run -- workflow-status --symbol UNKNOWN --state-dir state --json`
14. `cargo run -- artifact-status --symbol DEMO --state-dir state --latest-only`
15. `cargo run -- factor-autoresearch-status --symbol DEMO --state-dir state --latest-only`
16. `cargo run -- factor-research --symbol DEMO --data missing.json --output-format human`
17. `cargo run -- workflow-status --symbol UNKNOWN --state-dir state --output-format json`
18. `cargo run -- factor-backtest --symbol DEMO --data examples/demo/demo-15m.json --human`
19. `cargo run -- backtest --symbol DEMO --data examples/demo/demo-15m.json --human`
20. `cargo run -- research-verdict --symbol DEMO --state-dir state`

The final four scenarios did not reveal new bug classes. They reinforced already-observed output and UX inconsistencies.

---

## 2. Contributor Findings

### C-1. Prior audit is now stale on test and CI status

Observed:

- `cargo test` passes locally
- `.github/workflows/ci.yml` now exists in the local tree
- the earlier audit still states CI is missing and the `duration_sizing_scale` test is red

Impact:

- contributors reading the old audit will assume the branch is in worse shape than it is
- review and prioritization drift increases

Recommendation:

- add a short “superseded by / amended by” header to the older audit
- note explicitly that the `duration_sizing_scale` regression has been fixed locally

### C-2. `cargo clippy --all-targets --no-deps` is not clean

Observed:

- clippy produced dozens of warnings, including `too_many_arguments`, `field_reassign_with_default`, `manual_clamp`, `needless_lifetimes`, `useless_format`, `manual_abs_diff`

Impact:

- contributor trust is damaged because docs/audits imply cleaner lint status than reality
- PR review baseline is unclear

Recommendation:

- update docs to say `cargo check` and `cargo test` are required, but clippy is currently advisory
- either:
  - tighten clippy and fix warnings repo-wide, or
  - scope CI to a curated deny-list rather than implying full cleanliness

### C-3. Dirty worktree collision risk remains high

Observed:

- `main.rs` remains very large
- many unrelated files are already modified in the branch

Impact:

- contributor PRs and internal parallel work will conflict frequently

Recommendation:

- keep mainline feature work out of `main.rs` unless unavoidable
- prioritize extraction of command dispatch and output builders into focused modules

---

## 3. Human User Findings

### P0. `analyze --human --output-format json` does not reject conflicting format flags

Reproduction:

- `cargo run -- analyze --symbol DEMO --demo --human --output-format json`

Observed:

- command renders human output instead of failing

Expected:

- same explicit conflict error already implemented for other surfaces:
  `do not combine --output-format with --compact/--agent/--human`

Impact:

- human users and script authors can accidentally get the wrong surface
- docs imply one contract; runtime behavior violates it

Fix:

- route `analyze` through the same conflict guard already used elsewhere
- add a regression test specifically for `analyze --human --output-format json`

### P1. Human analyze output leaks agent protocol text

Reproduction:

- `cargo run -- analyze --symbol DEMO --demo --human`

Observed:

- human output contains:
  `Next: ask-user: Before using historical data ... | blocked until ... | then ict-engine factor-research ...`

Expected:

- human-facing wording similar to `workflow-status --human`:
  “Ask the user to provide/choose the historical data path before running research/backtest.”

Impact:

- release users see machine protocol grammar
- makes CLI feel unfinished and harder to trust

Fix:

- pass `recommended_next_command` through the same humanizer used in workflow-status
- reserve raw `ask-user:` strings for JSON/agent surfaces only

### P1. `factor-backtest --human` is effectively unreadable

Reproduction:

- `cargo run -- factor-backtest --symbol DEMO --data examples/demo/demo-15m.json --human`

Observed:

- output is a giant single-line JSON-style dump prefixed by `Factor backtest summary:`
- token-heavy, not scan-friendly, not actually human-formatted

Impact:

- release users selecting `--human` get a worse experience than `json`
- terminal usability is poor

Fix:

- replace current serializer-based string with a real multiline human renderer
- include only:
  - best factor
  - aggregate return
  - trade count
  - top credibility warnings
  - next step

### P1. `workflow-status --json` alias is unsupported, despite being a plausible user expectation

Reproduction:

- `cargo run -- workflow-status --symbol UNKNOWN --state-dir state --json`

Observed:

- clap rejects `--json`

Impact:

- users familiar with other CLIs expect a `--json` convenience flag
- current interface only has `--output-format json`, while other short aliases exist for the other modes

Fix:

- either add `--json` alias for symmetry
- or explicitly document that JSON is the default and only `--compact/--agent/--human` have short aliases

### P2. `backtest --human` rejects demo-scale data with a hard candle threshold

Reproduction:

- `cargo run -- backtest --symbol DEMO --data examples/demo/demo-15m.json --human`

Observed:

- `Error: need more candles for backtest: got 52, require at least 71`

Impact:

- a user following demo-oriented docs can assume backtest should also work on demo-sized fixtures
- the error itself is fine, but the product story is incomplete

Fix:

- either document minimum backtest dataset size clearly
- or provide a separate backtest demo fixture / reduced warmup demo mode

---

## 4. Agent User Findings

### A0. `factor-autoresearch-status` empty-state output is semantically noisy

Reproduction:

- `cargo run -- factor-autoresearch-status --symbol DEMO --state-dir state --latest-only`

Observed:

- returns a large structure with placeholders:
  - `effective_status: "unknown"`
  - live snapshot timestamps at `1970-01-01T00:00:00Z`
  - empty strings for symbol/objective/session

Impact:

- agent consumers have to reverse-engineer whether this means “feature unused”, “state missing”, or “corrupt snapshot”
- placeholder timestamps are especially misleading

Fix:

- when no autoresearch state exists, return a smaller explicit shape:
  - `status: "no_autoresearch_state"`
  - `sessions: []`
  - `live_snapshot: null`
  - `recommended_next_step: "run factor-autoresearch with --mutation-spec"`

### A1. `research-verdict` can say “continue” even when there are zero research runs

Reproduction:

- `cargo run -- research-verdict --symbol DEMO --state-dir state`

Observed:

- response says:
  - `best_known_baseline: "no_persisted_research_baseline"`
  - `current_bottleneck: "needs_more_evidence"`
  - `stop_or_continue: "continue"`
- even though evidence includes `research_runs=0` and `backtest_runs=0`

Impact:

- agent loops may treat “continue” as valid evidence-based iteration guidance
- encourages premature automation on artifact residue alone

Fix:

- special-case no-research-run states:
  - `stop_or_continue: "bootstrap_required"`
  - `recommended_next_experiment: run factor-research first`

### A1. Agent and human surfaces disagree on the same next-step rendering

Observed:

- `analyze --agent` gives structured `next_step`
- `workflow-status --agent` gives structured `next_step`
- `analyze --human` still leaks raw `ask-user:` machine syntax

Impact:

- inconsistent surface contracts make it harder to build UI wrappers or operator runbooks

Fix:

- unify next-step rendering through one formatter with three explicit targets:
  - human
  - agent
  - wire/raw

### A2. `artifact-status --latest-only` semantics are not obvious

Reproduction:

- `cargo run -- artifact-status --symbol DEMO --state-dir state --latest-only`

Observed:

- returns multiple entries, apparently one latest entry per kind / stream

Impact:

- both users and agents can misread `latest-only` as “single latest artifact overall”

Fix:

- rename or document behavior:
  - `--latest-only` means “latest relevant entries”, not single latest
- or add:
  - `--latest-one`
  - `--latest-per-kind`

---

## 5. Contributor / User / Agent Cross-Cut Gaps

### G-1. Help surface symmetry is incomplete

Observed:

- `analyze --help` shows `ICT_ENGINE_STATE_DIR` env exposure
- `workflow-status --help` does not show the env-based state-dir source in the same way
- JSON alias symmetry is inconsistent

Fix:

- normalize help text across all output-format/state-dir commands

### G-2. Human and machine output builders are still fragmented

Observed:

- some commands have real human renderers
- some commands stringify raw serializable structs
- some commands humanize next-step strings, some do not

Fix:

- introduce one output-surface policy per command:
  - `json`
  - `compact`
  - `agent`
  - `human`
- each command must implement all four explicitly or fail fast if unsupported

### G-3. Docs under-specify “demo” vs “real dataset” boundaries

Observed:

- demo analyze works
- demo debug works
- demo backtest does not
- factor-backtest on demo works, but human output is poor

Fix:

- add a “Demo Support Matrix” to README:
  - analyze: yes
  - factor-pipeline-debug: yes
  - factor-backtest: yes
  - backtest: no, needs larger dataset
  - factor-research: works only with real/selected historical data if state reuse gate trips

---

## 6. Feature and Coverage Gaps

### Missing / weak features

- No explicit `--json` alias despite other output aliases existing
- No clear bootstrap result for empty `factor-autoresearch-status`
- No user-friendly `factor-backtest --human`
- No command that explains “why this command cannot use demo/minimal data”

### Testing gaps revealed by shakedown

- No regression test proving `analyze --human --output-format json` fails
- No regression test asserting human surfaces never emit raw `ask-user:` protocol text
- No empty-state contract tests for `factor-autoresearch-status`
- No verdict guard test requiring at least one research run before `continue`
- No help contract tests for command symmetry and env flag exposure

---

## 7. Recommended Fix Order

### Immediate

1. Make `analyze` reject `--human/--agent/--compact` mixed with explicit `--output-format`
2. Humanize `analyze --human` next-step output
3. Replace `factor-backtest --human` serializer dump with a real human renderer
4. Return explicit empty-state contracts for `factor-autoresearch-status`
5. Guard `research-verdict` against zero-research-run false progression

### Short follow-up

6. Normalize help text and alias symmetry across commands
7. Document `artifact-status --latest-only` semantics
8. Add demo support matrix to README
9. Add regression tests for the above contracts

### Longer-term

10. Finish splitting `main.rs`
11. Decide whether clippy warnings are advisory or release-blocking
12. Continue panic-on-boundary cleanup in deep library code only where it does not alter numerical behavior

---

## 8. Bottom Line

`ict-engine` is no longer in “basic repo is broken” shape. A contributor can build and test it. A user can run meaningful analyze and workflow flows. An agent can consume structured status and next-step data.

The remaining release blockers are mostly coherence blockers:

- output modes do not behave uniformly
- some human surfaces still expose machine protocol
- some agent surfaces return placeholder or weakly-specified empty-state objects
- docs and lint expectations are not aligned with runtime reality

That makes this a good candidate for a focused release-polish sprint rather than a deep algorithm rewrite.
