# Regime Classifier R17 Handoff TODO

Live board for R17 read-only BBN soft-evidence adapter mapping.

Goal: convert optional `consumer_hints.bbn_evidence_hint` into a typed, read-only Rust evidence summary for downstream consumers, without mutating BBN posterior or mainline decisions.

Scope: adapter mapper + tests only. No CLI wiring, no inference mutation, no Python sidecar invocation.

---

## Routing / Process

- Primary route: `ict-engine-runtime`.
- Routing files read:
  - `/Users/thrill3r/.hermes/routing/skill-router.md`
  - repo `AGENTS.md`
  - `ict-engine-runtime` reference `references/regime-classifier-sidecar-chain.md`
- Project router override: none; `/Users/thrill3r/.hermes/routing/project-router.md` and repo `.hermes/routing/project-router.md` were absent.
- Runtime skill: installed `ict-engine/ict-engine-runtime/SKILL.md`.
- Process skills read: `aegis/test-driven-development`, `aegis/verification-before-completion`.

---

## Slice R17: Read-Only BBN Soft-Evidence Mapper

### Done

- [x] Added `RegimeBbnEvidenceStrength`:
  - `Strong`
  - `Moderate`
  - `Neutral`
- [x] Added `RegimeReadOnlyBbnSoftEvidence` with compact consumer fields:
  - `strength`
  - `weight`
  - `decision_state`
  - `trade_usable`
  - `label`
  - `label_set`
  - `transition_hazard`
  - `reasons`
- [x] Added `RegimeConsumerBundleAdapter::to_read_only_bbn_soft_evidence()`.
- [x] Mapping contract:
  - `single_label_99` + `trade_usable=true` -> `Strong`, weight `0.9`
  - `single_label_95` + `trade_usable=true` -> `Moderate`, weight `0.65`
  - `label_set` / `transitional` / `unknown_abstain` / missing / invalid -> `Neutral`, weight `0.0`
- [x] Mapper reads explicit bundle content only.
- [x] Mapper is side-effect free and does not insert into BBN `EvidenceManager`.
- [x] No mainline inference, execution branch, path-ranker rows, or recommendation mutation.

### TDD Evidence

RED verified before implementation:

```bash
cargo test --test regime_consumer_bundle_adapter single_label_99_maps_to_strong_read_only_bbn_soft_evidence -- --nocapture
```

Expected failure observed:

- unresolved import `RegimeBbnEvidenceStrength`
- missing method `to_read_only_bbn_soft_evidence`

GREEN verification:

```bash
cargo test --test regime_consumer_bundle_adapter
```

Result:

- 11 passed

Regression verification:

```bash
cargo check
```

Result:

- OK

### Consumer Contract

- Zero config: default adapter remains disabled/no-op.
- Hot-plug: only explicit bundle path participates.
- Token-friendly: compact enum + scalar summary, no large raw JSON required by consumers.
- No pollution: reads bundle path, writes no state.
- User-specific data preserved indirectly through existing sidecar fields; R17 mapper keeps `regime_label`, `label_set`, transition hazard, and reasons for later BBN/path-ranker adapters.
- Safe default: uncertain or abstain states map to neutral evidence, not posterior mutation.

---

## Current Worktree / Commit Notes

Latest relevant commit observed:

```text
d5458f6 map regime bundle to read-only bbn evidence
```

Also observed separate unrelated-format commit:

```text
53e768f format command wrappers
```

Do not amend or revert unrelated commits unless asked.

---

## Immediate Next Slice: R18 BBN Trace Surface or Controlled Wiring Spec

Recommended next step: expose the read-only BBN evidence summary as trace/report context behind the existing explicit bundle flag, still without applying it to BBN inference.

Acceptance:

- [ ] Append compact `regime_bbn_soft_evidence:*` trace entries when bundle flag is present.
- [ ] Add tests/smoke for `analyze` and/or `analyze-live` output.
- [ ] Preserve no-op default when flag absent.
- [ ] Preserve strict early failure behavior.
- [ ] Do not call `EvidenceManager::insert_soft` yet.

Later slice, only after trace proof: controlled opt-in posterior wiring with a separate explicit flag and tests proving default no mutation.

---

## Worktree Exclusions

Do not stage unrelated files unless explicitly asked:

- `src/auto_quant_command.rs`
- `src/validate_market_state_command.rs`
- `docs/plans/2026-05-09-regime-to-execution-mainline-audit-handoff-todo.md`
