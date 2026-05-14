# Factor Candidate Ingestion Instructions

Updated: `2026-05-12`

Purpose: replace the old May 10 append-only factor notes as the default agent
instruction surface. Agents must move useful factor results into repo-local
candidate-pack and admission artifacts, then remove inactive clutter from the
active loop. Do not append another long Markdown-only research block here.

Source logs retained for targeted lookup only:

- `support/docs/plans/2026-05-10-actionable-regime-confidence-todo.md`
- `support/docs/plans/2026-05-10-regime-conditional-auto-quant-profitability-todo.md`

Current compact authorities:

- `support/docs/plans/2026-05-12-board-a-regime-state-current.md`
- `support/docs/plans/2026-05-12-board-b-profit-factor-current.md`
- `support/docs/plans/2026-05-12-board-ab-cleanup-retention-plan.md`

## Operating Contract

This file is an instruction document, not a new evidence sink.

Agents may use the May 10 logs only for exact lookup by root id, heading, hash,
or artifact path. Any useful result found there must be extracted into the
current repo entrypoints below before it can affect training, ranking, or
follow-up work.

Markdown prose is not an active factor. A factor becomes active only when it is
visible through the candidate-pack loop or through a code/runtime path with
tests that prove the behavior.

## Active Entrypoints

Use these repo-local surfaces instead of historical planning prose:

```bash
cargo run -- factor-candidate-packs
cargo run -- factor-candidate-packs --symbol FACTOR_CANDIDATES --state-dir /tmp/ict-engine-candidates
cargo run -- factor-candidate-admission-targets --symbol FACTOR_CANDIDATES --state-dir /tmp/ict-engine-candidates
cargo run -- policy-training-status --symbol FACTOR_CANDIDATES --state-dir /tmp/ict-engine-candidates --output-format human
python3 support/scripts/research/factor_candidate_resolver.py --repo-root . --list-buildable --output-format human
```

Primary repo-local pack root:

```text
support/examples/factor_candidate_packs/curated-auto-quant-v1/
```

Each reusable candidate pack must contain:

- `factor_expression.json`
- `factor_eval_grid_summary.json`
- `transfer_score.json`

Current resolver baseline:

- `buildable_count=7`
- active pack root: `support/examples/factor_candidate_packs/curated-auto-quant-v1`
- admission export surface: `factor-candidate-admission-targets`
- runtime selection remains disabled until promotion gates pass

## Extraction Rule

When an agent finds a useful positive or negative result in old notes,
experiment roots, Auto-Quant output, or provider artifacts, it must classify and
move the result as follows:

- Positive reusable factor: create or update a candidate pack under
  `support/examples/factor_candidate_packs/curated-auto-quant-v1/`, then verify it is
  listed by `factor-candidate-packs`.
- Negative but useful result: preserve it as fail-closed evidence in a compact
  run packet or board status, with the exact reason it blocks promotion.
- Runtime/operator fix: land it in `src/`, `config/`, `support/scripts/`, or tests, not
  only in docs.
- Prose-only idea without artifacts: leave it inactive; do not add it to the
  active factor loop.

The minimum admission path for a factor candidate is:

```text
candidate evidence
-> candidate pack three-file contract
-> factor-candidate-packs inventory
-> factor-candidate-admission-targets export
-> policy-training-status readback
-> promotion gates
```

## Deletion And Cleanup Rule

Delete or archive inactive clutter only after it has been classified. Do not
delete sole evidence for a live gate.

Allowed cleanup target classes:

- duplicate scratch output already represented by a candidate pack, compact
  evidence packet, provider authority manifest, or admission target;
- prose-only factor notes that have no buildable artifact and no live gate;
- temporary Auto-Quant workspace output after the reusable evidence was moved;
- failed candidates whose negative value is captured as fail-closed evidence.

Blocked cleanup target classes:

- the two May 10 source logs before reference migration and parity pass;
- active roots or owner-protected roots named by Board A/B current docs;
- artifacts that are the only support for a live gate decision;
- source code, configs, or scripts modified by another active owner.

Before any real deletion, satisfy the dry-run conditions in
`support/docs/plans/2026-05-12-board-ab-cleanup-retention-plan.md`:

```text
active_owner=false
referenced_by_current_docs=false
referenced_by_scripts=false
extraction_packet_exists=true
parity_replay_pass=true
local_raw_dependency=false
not_sole_evidence_for_live_gate=true
```

## Promotion Boundary

Candidate-pack visibility is not promotion.

An admitted factor remains observation-only until it passes the Board B gate
chain:

```text
provider portability
-> sufficient branch/trade density
-> Pre-Bayes/filter
-> BBN learning/calibration
-> CatBoost/path-ranker
-> execution-tree non-Observe decision
-> feedback/update learning
```

If any gate fails, record the failure as useful negative evidence and keep
runtime selection disabled.

## Required Closeout For Any Agent Slice

Every agent slice touching factor candidates must close with evidence for each
applicable item:

- candidate pack path added, updated, or intentionally left unchanged;
- stale factor notes, scratch output, or old references classified as keep,
  archive, delete-later, or deletion-blocked;
- `factor-candidate-packs` output checked;
- `factor-candidate-admission-targets` checked when the active pack set changes;
- `policy-training-status` checked when admission artifacts are written;
- tests or verifier commands listed with pass/fail status;
- explicit statement that no new dependency on the May 10 logs was introduced.

If the slice cannot satisfy these items, it is incomplete and must not mark the
goal complete.
