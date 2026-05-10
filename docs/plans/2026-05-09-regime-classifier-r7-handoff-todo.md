# Regime Classifier R7 Handoff TODO

Live board for R7 distributional agreement sidecar.

Goal: compare current regime feature distribution against ICT label archetypes, keeping consumer adoption optional, zero-config, token-friendly, and isolated to explicit output paths.

Scope: sidecar scripts/tests/docs only. Do not touch unrelated Rust drift.

---

## Routing / Process

- Primary route: `ict-engine-runtime`.
- Process skill: `test-driven-dev`.
- Completion gate: `verification-before-completion`.
- Project router: `/Users/thrill3r/.hermes/routing/project-router.md` missing; no override.
- Repo entry map read: `AGENTS.md`.
- Domain reference read: `references/regime-classifier-sidecar-chain.md`.

---

## Current Worktree Exclusions

Do not stage unrelated files unless explicitly asked:

- `src/application/orchestration/execution_tree.rs`
- `src/auto_quant_command.rs`
- `src/validate_market_state_command.rs`
- `docs/plans/2026-05-09-regime-to-execution-mainline-audit-handoff-todo.md`

---

## Slice R7: Distributional Agreement

### Done

- [x] Wrote failing tests first: `scripts/research/tests/test_regime_distributional_agreement_report.py`.
- [x] RED verified: `ModuleNotFoundError: No module named 'regime_distributional_agreement_report'`.
- [x] Implemented: `scripts/research/regime_distributional_agreement_report.py`.
- [x] Zero-config pure-Python fallback; no scipy dependency.
- [x] Reads `regime_features.csv` or JSONL.
- [x] Reads `regime_expert_scores.jsonl`.
- [x] Reads `regime_conformal_calibration_report.json`.
- [x] Emits `regime_distributional_agreement_report.json`.
- [x] Compares current feature window to built-in primary label archetypes.
- [x] Emits distance method: `quantile_energy_proxy_fallback`.
- [x] Emits top score label and nearest archetype label.
- [x] Emits `agreement` as `agree` / `disagree`.
- [x] Emits `transitional_flag` plus reasons:
  - `high_distributional_distance`
  - `mixed_archetype_distance`
  - `wide_conformal_set`
- [x] Supports `--label-prefix` hot-plug consumer scope.
- [x] Supports `--window` for current feature window length.
- [x] User VRP/NQ fields remain visible in `feature_group_summaries.user_vrp_nq`:
  - `qqq_hv_level`
  - `nq_vs_200d_pct`
  - `vix3m_level`
  - `qqq_hv_pct_rank_252`
  - `vvix_over_vix`

### Verification

- [x] Target tests:
  - `python3 -m unittest scripts/research/tests/test_regime_distributional_agreement_report.py -v` -> 3 OK.
- [x] Full research suite after R7:
  - `python3 -m unittest discover -s scripts/research/tests -p 'test_*.py'` -> 75 OK.
- [x] CLI smoke with generated R2/R3/R5/R6 artifacts and R7 report:
  - R2 ontology -> R3 features + auxiliary VRP/NQ -> R5 trainer -> R6 conformal -> R7 agreement.
  - Output: `top_label=primary::TrendExpansion`, `nearest=primary::TrendExpansion`, `agreement=agree`, `has_user_vrp_nq=true`.

### CLI Floor

```bash
python3 scripts/research/regime_distributional_agreement_report.py \
  --features /tmp/ict-regime/regime_features.csv \
  --scores /tmp/ict-regime/regime_expert_scores.jsonl \
  --conformal-report /tmp/ict-regime/regime_conformal_calibration_report.json \
  --label-prefix primary:: \
  --output-json /tmp/ict-regime/regime_distributional_agreement_report.json
```

### Consumer Contract

- Required inputs:
  - R3 features CSV/JSONL.
  - R5 scores JSONL.
  - R6 conformal calibration JSON.
- Optional controls:
  - `--label-prefix` to opt into a regime family / subset.
  - `--window` to change current feature window.
- User can opt in by invoking the script; main runtime remains unchanged.
- User can ignore this sidecar with zero impact.
- Outputs are explicit paths only; no repo-root state writes.

---

## Next Slice Completed: R8 Transition Governor

### Create

- [x] `scripts/research/regime_transition_governor.py`
- [x] `scripts/research/tests/test_regime_transition_governor.py`

### Inputs

- [x] `regime_expert_scores.jsonl`.
- [x] `regime_conformal_calibration_report.json`.
- [x] `regime_distributional_agreement_report.json`.
- [x] Optional HMM transition/discovery report.
- [x] Optional drift/change-point rows.

### Outputs

- [x] `regime_transition_governor_report.json`.

### Acceptance

- [x] Enforces minimum duration / hysteresis.
- [x] Emits transition hazard.
- [x] Emits guardrail reasons.
- [x] Emits execution-tree compatible hint: `transition_guardrail` / `accept_regime` / `unknown_abstain`.
- [x] Keeps broad/noisy states non-trade-usable until confidence + distributional gates agree.
- [x] Target tests: `python3 -m unittest scripts/research/tests/test_regime_transition_governor.py -v` -> 4 OK.

---

## Commit Plan

Stage only R7 sidecar files and docs, not unrelated Rust drift.

Suggested add:

```bash
git add \
  docs/plans/2026-05-09-regime-classifier-r6-handoff-todo.md \
  docs/plans/2026-05-09-regime-classifier-r7-handoff-todo.md \
  docs/plans/2026-05-09-regime-classifier-research-and-99-confidence-todo.md \
  scripts/research/regime_distributional_agreement_report.py \
  scripts/research/tests/test_regime_distributional_agreement_report.py
```

Suggested commit:

```bash
git commit -m "feat: add regime distributional agreement sidecar"
```
