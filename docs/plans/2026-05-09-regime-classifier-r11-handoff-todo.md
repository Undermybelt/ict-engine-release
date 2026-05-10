# Regime Classifier R11 Handoff TODO

Live board for R11 sidecar chain README / one-command runner.

Goal: give consumers one zero-config command to run R2-R10 in a temp/output directory, with optional OHLCV/aux/truth inputs and no repo pollution.

Scope: sidecar scripts/tests/docs only. Do not touch unrelated Rust drift.

---

## Routing / Process

- Primary route: `ict-engine-runtime`.
- Project router: `/Users/thrill3r/.hermes/routing/project-router.md` missing; no override.
- Repo entry map read: `AGENTS.md`.
- Domain reference read: `references/regime-classifier-sidecar-chain.md`.
- Process: TDD RED -> GREEN -> full research suite -> CLI smoke -> commit only own files.

---

## Current Worktree Exclusions

Do not stage unrelated files unless explicitly asked:

- `src/auto_quant_command.rs`
- `src/validate_market_state_command.rs`
- `docs/plans/2026-05-09-regime-to-execution-mainline-audit-handoff-todo.md`

---

## Slice R11: Sidecar Pipeline Runner

### Done

- [x] Wrote failing tests first: `scripts/research/tests/test_regime_sidecar_pipeline.py`.
- [x] RED verified: `ModuleNotFoundError: No module named 'regime_sidecar_pipeline'`.
- [x] Implemented: `scripts/research/regime_sidecar_pipeline.py`.
- [x] Added user-facing doc: `docs/regime-classifier-sidecar-chain.md`.
- [x] One command runs R2-R10 when provided OHLCV.
- [x] Missing OHLCV emits input contract and exits nonzero without creating repo-root state.
- [x] Supports `--output-dir`, `--label-prefix`, `--auxiliary-evidence`, `--truth`.
- [x] Prints compact final decision and bundle path.
- [x] No runtime mutation.

### Verification

- [x] RED:
  - `python3 -m unittest scripts/research/tests/test_regime_sidecar_pipeline.py -v` -> missing module.
- [x] Target GREEN:
  - `python3 -m unittest scripts/research/tests/test_regime_sidecar_pipeline.py -v` -> 3 OK.
- [x] Full research suite:
  - `python3 -m unittest discover -s scripts/research/tests -p 'test_*.py'` -> 91 OK.
- [x] CLI smoke one-command R2-R10.
  - Output: `status=ok`, `decision_state=single_label_99`, `trade_usable=true`, `final_label=primary::TrendExpansion`.
  - Missing-input smoke: exit code `2`, output dir not created.

### CLI Floor

```bash
python3 scripts/research/regime_sidecar_pipeline.py \
  --ohlcv /tmp/ict-regime/ohlcv.csv \
  --auxiliary-evidence /tmp/ict-regime/aux.csv \
  --truth /tmp/ict-regime/truth.jsonl \
  --output-dir /tmp/ict-regime \
  --label-prefix primary::Trend
```

Missing-input contract:

```bash
python3 scripts/research/regime_sidecar_pipeline.py \
  --output-dir /tmp/ict-regime
# exits 2, no repo-root state
```

### Consumer Contract

- Required input:
  - `--ohlcv` with `timestamp,open,high,low,close,volume`.
- Optional controls:
  - `--output-dir`
  - `--label-prefix`
  - `--auxiliary-evidence`
  - `--truth`
- Primary output:
  - `regime_consumer_bundle.json`
- User can opt in by invoking the script; main runtime remains unchanged.
- User can ignore this sidecar with zero impact.
- Outputs are explicit paths only; no repo-root state writes.

---

## Immediate Next Slice: R12 Mainline Optional Adapter Spec

### Goal

Add a non-invasive adapter contract for mainline consumers to read `regime_consumer_bundle.json` if explicitly provided, without changing default runtime behavior.

### Acceptance

- [ ] Spec first, no Rust implementation unless chosen.
- [ ] Define CLI/config field names for optional bundle path.
- [ ] Define execution tree / BBN / path-ranker mapping table.
- [ ] Preserve zero-config default behavior.
- [ ] Include rollback/no-op behavior when bundle path missing or invalid.

---

## Commit Plan

Stage only R11 sidecar files and docs, not unrelated Rust drift.

Suggested add:

```bash
git add \
  docs/plans/2026-05-09-regime-classifier-r11-handoff-todo.md \
  docs/regime-classifier-sidecar-chain.md \
  scripts/research/regime_sidecar_pipeline.py \
  scripts/research/tests/test_regime_sidecar_pipeline.py
```

Suggested commit:

```bash
git commit -m "feat: add regime sidecar pipeline runner"
```
