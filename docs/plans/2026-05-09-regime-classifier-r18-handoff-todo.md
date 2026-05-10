# Regime Classifier R18 Handoff TODO

Live board for R18 read-only BBN trace surfacing.

Goal: expose the R17 read-only BBN soft-evidence summary in `analyze` and `analyze-live` trace/report surfaces behind the existing explicit bundle flag, without mutating BBN posterior or runtime decisions.

Scope: trace/report context only. No call to `EvidenceManager::insert_soft`, no posterior update, no execution branch mutation, no path-ranker row mutation, no Python sidecar invocation.

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

## Slice R18: Read-Only BBN Trace Surface

### Done

- [x] Added `RegimeConsumerBundleAdapter::bbn_soft_evidence_trace_entries()`.
- [x] Added compact trace keys:
  - `regime_bbn_soft_evidence_strength`
  - `regime_bbn_soft_evidence_weight`
  - `regime_bbn_decision_state`
  - `regime_bbn_trade_usable`
  - `regime_bbn_label`
  - `regime_bbn_label_set`
  - `regime_bbn_transition_hazard`
  - `regime_bbn_reasons`
- [x] `analyze` appends joined trace line:
  - `regime_bbn_soft_evidence_trace:*`
- [x] `analyze-live` appends joined trace line:
  - `regime_bbn_soft_evidence_trace:*`
- [x] Both commands also add individual trace entries to `artifact_action_summary`.
- [x] Both commands add read-only prefixed entries to `pre_bayes_evidence_filter.rationale` and `evidence_assignments`.
- [x] No inference mutation: values are stored under `read_only_*` keys only.

### Verification

Target adapter test:

```bash
cargo test --test regime_consumer_bundle_adapter
```

Result:

- 12 passed

Compiler check:

```bash
cargo check
```

Result:

- OK

Analyze smoke with explicit valid bundle:

```bash
./target/debug/ict-engine analyze \
  --demo \
  --symbol NQ \
  --state-dir /tmp/ict-r18-analyze \
  --output-format json \
  --regime-consumer-bundle /tmp/ict-regime-r18-bundle.json \
  --regime-consumer-bundle-strict
```

Observed in output:

- `regime_bbn_soft_evidence_trace:regime_bbn_soft_evidence_strength=strong|...`
- `regime_bbn_soft_evidence_strength=strong`
- `read_only_regime_bbn_soft_evidence_strength=strong`
- `read_only_regime_bbn_label=primary::TrendExpansion`
- existing branch remained `transition_guardrail`, proving bundle trace did not force execution accept.

Analyze-live strict missing smoke:

```bash
./target/debug/ict-engine analyze-live \
  --symbol NQ \
  --state-dir /tmp/ict-r18-live \
  --output-format compact \
  --regime-consumer-bundle /tmp/missing-r18-bundle.json \
  --regime-consumer-bundle-strict
```

Observed:

- exit nonzero
- stderr contains `missing`

### Consumer Contract

- Zero config: absent bundle flag keeps default behavior.
- Hot-plug: only explicit bundle path is read.
- Token-friendly: trace entries are compact scalar strings.
- No pollution: no repo-root state; smokes use `/tmp` state.
- Personal data fields from sidecar remain optional and are not expanded into verbose trace payloads.
- Safe default: BBN soft evidence remains read-only until a later explicit opt-in wiring slice.

---

## Current Dirty Boundary

Do not stage unrelated files unless explicitly asked. During R18, many unrelated files appeared dirty from another agent/session. R18 should stage only:

- `src/application/regime/consumer_bundle_adapter.rs`
- `tests/regime_consumer_bundle_adapter.rs`
- `src/analyze_command.rs`
- `src/analyze_live_command.rs`
- `docs/plans/2026-05-09-regime-classifier-r18-handoff-todo.md`

Observed unrelated dirty examples:

- `src/application/auto_quant/*`
- `src/application/belief/*`
- `src/application/data_sources/*`
- `src/application/orchestration/*`
- `src/bbn/trading/*`
- `src/belief_core/*`
- `src/market_state/*`
- `src/state/types.rs`
- `docs/plans/2026-05-09-regime-bundle-bbn-diagnostics-handoff-todo.md`

---

## Immediate Next Slice: R19 Controlled Opt-In BBN Wiring Spec

Recommended next step: spec and tests for an explicit opt-in flag that can feed the read-only summary into a controlled BBN evidence path.

Acceptance:

- [ ] Add design/spec first: default no mutation, opt-in only.
- [ ] Name flag clearly, e.g. `--apply-regime-bundle-bbn-soft-evidence`.
- [ ] Test absent flag leaves `EvidenceManager` untouched.
- [ ] Test present flag applies only valid `Strong/Moderate` evidence.
- [ ] Abstain/neutral states must not mutate posterior.
- [ ] Trace must record applied vs skipped reason.
