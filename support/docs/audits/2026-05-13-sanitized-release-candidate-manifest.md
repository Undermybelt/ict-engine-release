# Sanitized release candidate manifest

Date: 2026-05-13
Status: verified candidate export for `v0.1.2` mirror publication

## Purpose

This manifest records the release candidate that was audited for consumer and
contributor first-run quality after the working tree had accumulated unrelated
research, experiment, and local-data artifacts.

The final publish slice includes the 2026-05-13 README/AGENT polish:
`README.md` is the human-readable public entrypoint, and `AGENT.md` is the
operating contract for future agents serving users and release work.

Do not publish the whole dirty working tree. Publish only a clean export that
matches this manifest, or rerun the full gate after changing the slice.

## Verified export

- Export: `/tmp/ict-engine-v012-release-export.CHyo93`
- Target: `/tmp/ict-engine-v012-release-target.NJjdD3`
- Smoke state: `/tmp/ict-engine-v012-smoke-state.M78llx`
- Smoke output: `/tmp/ict-engine-v012-smoke-out.yszAfG`

These `/tmp` paths are evidence locations, not durable release inputs.

## Include

Start from `git archive HEAD`, then overlay the current release-candidate
versions of these paths from the working tree:

- `AGENT.md`
- `README.md`
- `support/docs/factor-artifact-naming-contract.md`
- current release-facing docs sanitized to CatBoost-only wording
- `src/analyze/smt_correlation_section.rs`
- `src/application/auto_quant/live/wire.rs`
- `src/application/entry_models/mod.rs`
- `src/application/entry_models/training_export.rs`
- `src/application/multi_timeframe_inputs.rs`
- `src/application/orchestration/ensemble_vote.rs`
- `src/application/orchestration/execution_tree.rs`
- `src/application/orchestration/structural_playbook.rs`
- `src/application/orchestration/workflow_status.rs`
- `src/application/provider_catalog.rs`
- `src/main.rs`
- `src/policy_training_command.rs`
- `tests/eml_poc.rs`
- `tests/provider_neutral_cli.rs`
- `config/regime_confidence_assets_v1.csv`
- `support/examples/factor_candidate_packs/curated-auto-quant-v1/**`
- sanitized `support/examples/provider_profiles/thrill3r-nq-closed-loop-v1.json`
- sanitized `support/examples/factor_candidate_profiles/thrill3r-nq-auto-quant-v1.json`

## Exclude

- `support/docs/plans/2026-05-12-hotplug-personal-data-release-handoff-todo.md`
- `support/docs/experiments/actionable-regime-confidence/runs/**`
- root workspace-looking artifacts such as `CryptoContinuationFailureGuard`,
  `CryptoMomentumPersistence`, `ema_rsi_persistence`, and `momentum_failure`
- old tracked April doc deletions unless the operator explicitly chooses that
  docs cleanup slice
- generated provider caches, Auto-Quant workspaces, dependency clones, virtual
  environments, and local state directories
- legacy local-data research scripts that still embed maintainer-local absolute
  paths
- historical support/docs/prompts that still contain maintainer-local absolute paths
  unless they are redacted first

## Required gates

Run these from the candidate export before publishing:

```bash
cargo fmt --manifest-path "$EXPORT/Cargo.toml" --check
CARGO_TARGET_DIR="$TARGET" cargo clippy --manifest-path "$EXPORT/Cargo.toml" --all-targets -- -D warnings
PATH=<provider-venv>/bin:$PATH \
  CARGO_TARGET_DIR="$TARGET" cargo test --manifest-path "$EXPORT/Cargo.toml"
```

Then run the true zero-config consumer smoke without injecting the provider
venv:

```bash
"$TARGET/debug/ict-engine" provider-status --compact
"$TARGET/debug/ict-engine" workflow-status --symbol DEMO --state-dir "$SMOKE_STATE" --human
"$TARGET/debug/ict-engine" analyze --symbol DEMO --demo --state-dir "$SMOKE_STATE" --human
"$TARGET/debug/ict-engine" workflow-status --symbol DEMO --state-dir "$SMOKE_STATE" --refresh --agent
"$TARGET/debug/ict-engine" pre-bayes-status --symbol DEMO --state-dir "$SMOKE_STATE" --refresh --output-format json
"$TARGET/debug/ict-engine" policy-training-status --symbol DEMO --state-dir "$SMOKE_STATE" --output-format agent
"$TARGET/debug/ict-engine" factor-candidate-packs --state-dir "$SMOKE_STATE" --symbol FACTOR_CANDIDATES --output-format human
"$TARGET/debug/ict-engine" factor-candidate-admission-targets --state-dir "$SMOKE_STATE" --symbol FACTOR_CANDIDATES --output-format human
"$TARGET/debug/ict-engine" regime-confidence-assets --state-dir "$SMOKE_STATE" --symbol REGIME_CONFIDENCE_ASSETS --output-format human
```

## 2026-05-13 evidence

- `cargo fmt`: passed.
- `cargo clippy --all-targets -- -D warnings`: passed.
- Full `cargo test`: passed. Lib `963`, bin `253`, integration suites, and
  doctests passed.
- True zero-config smoke: passed.
- Smoke provider summary: `entry_model:2/2 ready | live_runtime:1/3 ready |
  local_runtime:1/2 ready | market_data:1/7 ready`.
- Zero-config ready provider: `yfinance`.
- Richer providers: setup/runtime-gated, not required by default.
- `analyze --demo --human`: surfaced Structure, Technicals, SMT, Regime
  posterior probabilities, and Plan.
- `workflow-status --refresh --agent`: surfaced posterior probabilities and
  top executor `catboost_file`.
- Candidate packs: `candidate_pack_count=7`.
- Admission targets: `rows=35`, `mature_rows=35`, promotion remains blocked
  until downstream gates pass.
- Regime-confidence assets: `regime_confidence_asset_count=18`,
  `board_a_gate=11`.
- Smoke-output privacy scan for `/Users`, `/private`, `Downloads`, `API key`,
  `api_key`, `secret`, `token`, `bearer`, `password`, and `credential`: no
  matches.
- Final `v0.1.2` README/AGENT-polished export:
  `/tmp/ict-engine-v012-release-export.CHyo93`.
- Final target: `/tmp/ict-engine-v012-release-target.NJjdD3`.
- Final smoke output: `/tmp/ict-engine-v012-smoke-out.yszAfG`.
- Final gates after README/AGENT polish: fmt passed, Clippy passed, full cargo
  test passed with lib `963`, bin `253`, integration suites, and doctests.
- Final zero-config smoke after README/AGENT polish: passed with yfinance
  fallback, posterior probabilities, top executor `catboost_file`, 7 candidate
  packs, 35 admission target rows, and 18 regime-confidence assets.
- Final smoke stderr files were empty.
- Final smoke-output privacy scan found no private path or secret-like matches.
- Final mirror hygiene pruned historical support/docs/prompts with concrete
  maintainer-local paths; the only remaining local-path-shaped match is the
  intentional `/Users/example/Downloads/Tomac` negative test guard in
  `tests/provider_neutral_cli.rs`.

## Publish blocker

This candidate is verified evidence for the authorized `v0.1.2` mirror
publication flow.

Publishing still requires:

- final export gate after README/AGENT polish;
- remote tag re-check before pushing `v0.1.2`;
- either publishing the exact sanitized export slice recorded here, or rewriting
  the excluded local-data scripts into explicit-input consumer examples and
  rerunning the full gate.
