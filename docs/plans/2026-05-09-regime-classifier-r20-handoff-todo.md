# Regime Classifier R20 Handoff TODO

Live board for R20 closure audit: make the R19 applied regime-bundle BBN evidence visible through persisted workflow consumers.

Goal: verify and fix sidecar bundle -> adapter -> pre-Bayes -> BBN evidence -> analyze snapshot -> workflow-status JSON/human visibility.

---

## Done

- [x] Audited R19 applied path in persisted `workflow-status --output-format json`.
- [x] Found break: `workflow_phase_snapshot_from_analyze_run()` let canonical structural posterior overwrite `pre_bayes_filtered_assignments.market_regime` and `pre_bayes_soft_evidence.market_regime`, hiding applied bundle evidence from workflow consumers.
- [x] Added RED test for applied bundle evidence visibility when canonical structural posterior is also present.
- [x] Fixed snapshot behavior:
  - if `regime_bundle_bbn_application_status=applied`, keep bundle-applied `market_regime` and soft distribution visible.
  - otherwise preserve existing canonical structural posterior behavior.
- [x] Added phase summary fragment: `regime_bundle_bbn=<status>:<regime>`.
- [x] Fixed human `workflow-status` short summary to include the same fragment.
- [x] Verified JSON and human workflow-status both show applied bundle BBN evidence.

---

## Consumer Contract

- Default unchanged: no bundle / no apply flag remains zero-config and no-op.
- Applied bundle evidence is visible in:
  - `AnalyzeRunRecord.pre_bayes_evidence_filter.evidence_assignments`
  - `WorkflowPhaseSnapshot.pre_bayes_filtered_assignments`
  - `WorkflowPhaseSnapshot.pre_bayes_soft_evidence.market_regime`
  - `WorkflowPhaseSnapshot.phase_summary`
  - `workflow-status --human` latest line
- Canonical structural posterior is still visible in its dedicated fields; it no longer masks explicit applied bundle BBN evidence.

---

## Verification

RED:

```bash
cargo test --bin ict-engine workflow_snapshot_runtime::tests::analyze_snapshot_keeps_applied_regime_bundle_bbn_evidence_visible -- --nocapture
```

Observed expected failure before fix:

- `left: "trend"`
- `right: "bull"`

GREEN / regression:

```bash
cargo test --test regime_consumer_bundle_adapter
cargo test --bin ict-engine workflow_snapshot_runtime::tests::analyze_snapshot_keeps_applied_regime_bundle_bbn_evidence_visible
cargo test --lib application::orchestration::workflow_status::tests::short_human_phase_summary_includes_applied_regime_bundle_bbn
cargo check
```

Observed:

- adapter tests: 15 passed
- workflow snapshot targeted test: 1 passed
- workflow-status human targeted test: 1 passed
- cargo check: OK

End-to-end smoke:

```bash
cargo build
./target/debug/ict-engine analyze \
  --demo \
  --symbol NQ \
  --state-dir /tmp/ict-r20-apply2 \
  --output-format json \
  --regime-consumer-bundle /tmp/ict-regime-r19-bundle.json \
  --regime-consumer-bundle-strict \
  --apply-regime-bundle-bbn-soft-evidence
./target/debug/ict-engine workflow-status \
  --symbol NQ \
  --state-dir /tmp/ict-r20-apply2 \
  --output-format json
./target/debug/ict-engine workflow-status \
  --symbol NQ \
  --state-dir /tmp/ict-r20-apply2 \
  --human
```

Observed checks all true:

```text
assignment_market_regime_bull=true
application_status_applied=true
soft_bull_09=true
phase_summary_visible=true
human_visible=true
uses_soft=true
```

---

## Files in R20

Stage only:

- `src/workflow_snapshot_runtime.rs`
- `src/application/orchestration/workflow_status.rs`
- `docs/plans/2026-05-09-regime-classifier-r20-handoff-todo.md`

Unrelated dirty files remain outside this slice.

---

## Next Slice

R21 optional if more rigor desired:

- Add `pre-bayes-status --section soft-evidence` smoke for bundle-applied fields.
- Decide if `TrendExpansion -> bull` needs sidecar direction metadata before being treated as more than moderate/strong market-regime evidence.
- Update long-lived skill reference from R17 to R20 after commit if acceptable.
