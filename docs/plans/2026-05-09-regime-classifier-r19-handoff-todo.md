# Regime Classifier R19 Handoff TODO

Live board for R19 controlled opt-in BBN soft-evidence wiring.

Goal: make the R17/R18 regime consumer bundle path complete enough to reach the existing BBN evidence pipeline, but only when the user explicitly opts in.

Scope: explicit CLI flag -> pre-Bayes filter soft evidence -> existing `trade_evidence_from_pre_bayes_filter()` -> BBN inference. No default behavior change.

---

## Routing / Process

- Primary route: `ict-engine-runtime`.
- Routing files read:
  - `/Users/thrill3r/.hermes/routing/skill-router.md`
  - repo `AGENTS.md`
  - `ict-engine-runtime` reference `references/regime-classifier-sidecar-chain.md`
- Project router override: none; `/Users/thrill3r/.hermes/routing/project-router.md` and repo `.hermes/routing/project-router.md` absent.
- Runtime skill: installed `ict-engine/ict-engine-runtime/SKILL.md`.
- Process skills read: `aegis/test-driven-development`, `aegis/verification-before-completion`.

---

## Slice R19: Controlled Opt-In BBN Wiring

### Done

- [x] Added CLI flag to `analyze`:
  - `--apply-regime-bundle-bbn-soft-evidence`
- [x] Added same flag to `analyze-live`.
- [x] Added `RegimeBbnEvidenceApplicationStatus`:
  - `Applied`
  - `Skipped`
- [x] Added `RegimeConsumerBundleAdapter::apply_bbn_soft_evidence_to_pre_bayes_filter()`.
- [x] Strong/moderate labels can now mutate the pre-Bayes filter only when explicit opt-in is true.
- [x] Neutral / abstain / disabled / missing label / unsupported labels skip mutation.
- [x] Applied evidence writes:
  - `filter.uses_soft_evidence = true`
  - `filter.filtered_market_regime_label = <mapped BBN label>`
  - `filter.soft_market_regime_distribution = {bull,bear,range}` normalized distribution
  - `filter.evidence_assignments.regime_bundle_bbn_application_status`
  - `filter.rationale.regime_bundle_bbn_evidence_applied=*`
- [x] Existing BBN path consumes this through `trade_evidence_from_pre_bayes_filter()` because `uses_soft_evidence=true`.

### Mapping

Current explicit mapping:

- `primary::TrendExpansion` -> BBN `market_regime=bull`
- `primary::RangeConsolidation` -> BBN `market_regime=range`
- `primary::ExtremeStress` -> BBN `market_regime=range`
- `primary::ReversalBrewing` -> BBN `market_regime=range`

Weights:

- `single_label_99` -> selected label probability `0.9`
- `single_label_95` -> selected label probability `0.65`
- remaining probability split equally across the other two BBN market-regime states

### TDD Evidence

RED verified before implementation:

```bash
cargo test --test regime_consumer_bundle_adapter strong_bundle_applies_to_pre_bayes_soft_market_regime_when_opted_in -- --nocapture
```

Expected failure observed:

- unresolved import `RegimeBbnEvidenceApplicationStatus`
- missing method `apply_bbn_soft_evidence_to_pre_bayes_filter`

GREEN verification:

```bash
cargo test --test regime_consumer_bundle_adapter
```

Result:

- 14 passed

Compiler check:

```bash
cargo check
```

Result:

- OK

Build:

```bash
cargo build
```

Result:

- OK

### End-to-End Smoke

Valid bundle, opt-in apply:

```bash
./target/debug/ict-engine analyze \
  --demo \
  --symbol NQ \
  --state-dir /tmp/ict-r19-analyze-apply \
  --output-format json \
  --regime-consumer-bundle /tmp/ict-regime-r19-bundle.json \
  --regime-consumer-bundle-strict \
  --apply-regime-bundle-bbn-soft-evidence
```

Observed:

- `regime_bundle_bbn_evidence_applied` present
- `regime_bundle_bbn_application_status=applied`
- soft market-regime distribution includes `bull=0.9`

Valid bundle, no opt-in:

```bash
./target/debug/ict-engine analyze \
  --demo \
  --symbol NQ \
  --state-dir /tmp/ict-r19-analyze-skip \
  --output-format json \
  --regime-consumer-bundle /tmp/ict-regime-r19-bundle.json \
  --regime-consumer-bundle-strict
```

Observed:

- no applied marker
- `regime_bundle_bbn_evidence_skipped=flag_disabled` present
- no forced `bull=0.9`

Analyze-live strict missing with apply flag:

```bash
./target/debug/ict-engine analyze-live \
  --symbol NQ \
  --state-dir /tmp/ict-r19-live \
  --output-format compact \
  --regime-consumer-bundle /tmp/missing-r19-bundle.json \
  --regime-consumer-bundle-strict \
  --apply-regime-bundle-bbn-soft-evidence
```

Observed:

- exit nonzero
- stderr contains `missing`
- proves strict still fails before live fetch / BBN apply path

CLI help:

- `analyze --help` exposes `--apply-regime-bundle-bbn-soft-evidence`
- `analyze-live --help` exposes `--apply-regime-bundle-bbn-soft-evidence`

---

## Consumer Contract

- Zero config: default behavior unchanged; no apply flag means skip.
- Hot-plug: explicit bundle path + explicit apply flag required.
- Token-friendly: existing compact trace remains scalar.
- No pollution: smoke state dirs under `/tmp`.
- Safe default: abstain/neutral labels never mutate BBN path.
- Closed loop: sidecar bundle -> Rust adapter -> pre-Bayes soft distribution -> existing BBN evidence conversion -> BBN inference.

---

## Current Dirty Boundary

Stage only R19 files unless explicitly asked:

- `src/application/regime/consumer_bundle_adapter.rs`
- `tests/regime_consumer_bundle_adapter.rs`
- `src/main.rs`
- `src/analyze_command.rs`
- `src/analyze_live_command.rs`
- `src/probabilistic_backtest_runtime.rs`
- `docs/plans/2026-05-09-regime-classifier-r19-handoff-todo.md`

Unrelated dirty files existed before/alongside R19 and should remain unstaged.

---

## Immediate Next Slice: R20 Closure Audit / Cleanup

Recommended next step before further features:

- [ ] Run focused closure audit for R15-R19 fields in persisted analyze snapshot.
- [ ] Confirm `workflow-status` / downstream consumers surface the applied/skipped status cleanly.
- [ ] Decide whether `primary::TrendExpansion -> bull` is enough for user workflow or needs bull/bear direction from sidecar.
- [ ] If direction is ambiguous, add direction-aware mapping before any stronger posterior effect.
