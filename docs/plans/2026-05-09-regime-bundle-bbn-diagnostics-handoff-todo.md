# 2026-05-09 Regime Bundle BBN Diagnostics Handoff TODO

## Mission

Continue `regime-consumer-bundle -> BBN diagnostics -> execution consumer surface` closure.

Hard constraints:
- zero config by default: no new required env/config
- token friendly: compact machine fields and short trace lines
- no pollution: runtime smoke state under `/tmp/...`
- no debt: tests plus real CLI proof before claiming done
- hot-plug: only active when user passes `--regime-consumer-bundle`; default path unchanged
- user-specific content preserved: VRP/NQ bundle fields remain sidecar payload, not hardwired into core defaults

## Route / Skill

- Route alias: `ict-engine-runtime`
- Router read: `~/.hermes/routing/skill-router.md`
- Project router: `~/.hermes/routing/project-router.md` missing, no override
- Repo guide read: `AGENTS.md`
- Skill used: `~/.hermes/skills/ict-engine/ict-engine-runtime/SKILL.md`
- References used:
  - `references/mainline-regime-catboost-execution-audit.md`
  - `references/mainline-consumer-reason-field.md`
  - `references/regime-consumer-bundle-and-mature-ranker-validation.md`

## Current slice

Goal: close regime-consumer-bundle BBN path in two modes:
- default read-only diagnostics: safe, zero behavior change
- opt-in soft-evidence application: `--apply-regime-bundle-bbn-soft-evidence`

Implemented:
- `RegimeConsumerBundleAdapter::bbn_soft_evidence_trace_entries()` keeps compact trace fields.
- `RegimeConsumerBundleAdapter::append_read_only_bbn_diagnostics()` centralizes analyze/analyze-live diagnostic wiring, removing duplicated logic.
- `RegimeConsumerBundleAdapter::apply_bbn_soft_evidence_to_pre_bayes_filter()` applies strong/moderate bundle evidence into the Pre-Bayes filter only when explicitly opted in.
- Legacy accepted bundle labels such as `RangeConsolidation/WideRange` map to moderate `range` evidence, preserving current user fixture compatibility.
- `analyze` / `analyze-live` support `--apply-regime-bundle-bbn-soft-evidence`; without it, bundle stays read-only.
- BBN application status is visible at `pre_bayes_evidence_filter.evidence_assignments.regime_bundle_bbn_application_status`.
- Added adapter tests for:
  - compact BBN diagnostics
  - opt-in strong application
  - no-opt/neutral skip
  - legacy accepted bundle application

Compact fields:
- `regime_bbn_soft_evidence_strength=strong|moderate|neutral`
- `regime_bbn_soft_evidence_weight=0.900|0.650|0.000`
- `regime_bbn_decision_state=<state>`
- `regime_bbn_trade_usable=<bool>`
- `regime_bbn_label=<label>`
- `regime_bbn_label_set=<comma-list>`
- `regime_bbn_transition_hazard=<float>`
- `regime_bbn_reasons=<comma-list>`

Validation done:
- `cargo test --test regime_consumer_bundle_adapter -- --nocapture` -> 15/15 pass
- `cargo check` -> pass
- `cargo build --bin ict-engine` -> pass
- read-only CLI smoke wrote `/tmp/ict-mainline-regime-audit/analyze-regime-bundle-bbn-readonly.json`
- opt-in applied CLI smoke wrote `/tmp/ict-mainline-regime-audit/analyze-regime-bundle-bbn-applied.json`
- smoke assertion result: `ok readonly= moderate applied= range`
- previous diagnostic commit: `58c17b6 feat: surface regime bundle bbn trace`
- previous doc commit: `c41cd7a docs: add regime bundle bbn diagnostics handoff`

## Files touched this slice

- `src/application/regime/consumer_bundle_adapter.rs`
- `src/analyze_command.rs`
- `src/analyze_live_command.rs`
- `src/main.rs`
- `src/probabilistic_backtest_runtime.rs`
- `tests/regime_consumer_bundle_adapter.rs`
- `docs/plans/2026-05-09-regime-bundle-bbn-diagnostics-handoff-todo.md`

## TODO next

### P0 - Finish verification

- [x] Run focused adapter test.
- [x] Run `cargo check`.
- [x] Run `cargo build --bin ict-engine`.
- [x] Run read-only `analyze` smoke with `/tmp/ict-mainline-regime-audit/regime-consumer-bundle-sample.json`.
- [x] Run opt-in applied `analyze` smoke with `--apply-regime-bundle-bbn-soft-evidence`.
- [x] Confirm JSON fields in:
  - `report.supporting.artifact_action_summary`
  - `report.supporting.pre_bayes_evidence_filter.rationale`
  - `report.supporting.pre_bayes_evidence_filter.evidence_assignments.read_only_regime_bbn_soft_evidence_strength`
  - `report.supporting.pre_bayes_evidence_filter.evidence_assignments.regime_bundle_bbn_application_status`
  - `report.supporting.pre_bayes_evidence_filter.soft_market_regime_distribution`

### P1 - Runtime smoke command

```bash
BASE=/tmp/ict-mainline-regime-audit
cargo build --bin ict-engine
./target/debug/ict-engine analyze \
  --symbol NQ \
  --data-root "$BASE" \
  --state-dir "$BASE/state" \
  --output-format json \
  --inline-ledger \
  --regime-consumer-bundle "$BASE/regime-consumer-bundle-sample.json" \
  > "$BASE/analyze-regime-bundle-bbn.json"
python3 - <<'PY'
import json
p='/tmp/ict-mainline-regime-audit/analyze-regime-bundle-bbn.json'
d=json.load(open(p))
s=d['report']['supporting']
a=s['artifact_action_summary']
f=s['pre_bayes_evidence_filter']
assert 'regime_bbn_soft_evidence_strength=strong' in a
assert 'regime_bbn_soft_evidence_weight=0.900' in a
assert 'read_only_regime_bbn_soft_evidence_strength' in f['evidence_assignments']
assert f['evidence_assignments']['read_only_regime_bbn_soft_evidence_strength'] == 'strong'
assert any(x == 'read_only_regime_bbn_soft_evidence_strength=strong' for x in f['rationale'])
print('ok')
PY
```

### P2 - Commit discipline

- [x] Run `git status --short` before staging.
- [x] Implementation committed in `58c17b6 feat: surface regime bundle bbn trace`.
- [x] Stage and commit this handoff doc only; leave unrelated dirty files untouched.
- Commit: pending in this session until git commit completes.

## Open design boundary

Closed for current scope:
- bundle trace -> Pre-Bayes diagnostics -> optional Pre-Bayes soft evidence -> BBN inference input is wired and runtime-smoked.
- Default remains read-only, so existing consumers keep zero behavior change unless they opt in.

Still intentionally out of scope:
- No native CatBoost inference added.
- No automatic promotion of user VRP/NQ fields into core defaults; those remain hot-plug bundle payloads.
- No forced posterior override; only soft market-regime evidence when user passes the opt-in flag.
