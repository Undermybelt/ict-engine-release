# autoresearch pattern intake — 2026-04-23

Source reviewed:
- local clone: `/Users/thrill3r/autoresearch`
- files read:
  - `README.md`
  - `program.md`

## Verdict

Worth absorbing as research-loop and experiment-ledger patterns.

Not worth absorbing as git-driven execution semantics.

`ict-engine` should learn from this repo's:

- fixed-budget experiment comparability
- strict keep/discard loop discipline
- append-only human-readable experiment ledger
- explicit "one canonical execution surface, one human control surface" framing

But it should reject:

- `git reset` as the primary rollback mechanism
- branch/commit hash as the canonical experiment identity
- untracked TSV as the only durable experiment record

## What the repo does well

### 1. It makes the experiment loop explicit

The most transferable insight is not the model-training code. It is the loop contract:

1. establish baseline
2. change one bounded surface
3. run one experiment under a fixed budget
4. record outcome
5. keep or discard
6. continue autonomously

For `ict-engine`, this is useful because `factor-autoresearch` already has the same conceptual shape, but the discipline is spread across state files and code rather than written as a single operator contract.

### 2. It separates canonical execution from human steering

`autoresearch` uses:

- one editable execution surface: `train.py`
- one human steering surface: `program.md`

That separation is clean.

The `ict-engine` analogue is not file-for-file, but the pattern still transfers:

- canonical research execution belongs in typed state + command flow
- human/operator steering belongs in docs, plans, prompts, and derived recap surfaces

This is especially relevant now that `ict-engine` is adding:

- `experiments.tsv`
- `factor_autoresearch_retrospective.md`

These should remain steering/inspection aids, not become hidden execution state.

### 3. It treats the ledger as a first-class operator tool

`autoresearch` explicitly initializes and appends to `results.tsv`, with a small stable schema:

- commit
- metric
- memory
- status
- description

This is a strong pattern for `ict-engine` even though the exact fields differ.

What is useful to absorb:

- the ledger is for fast operator scan, not full replay
- each run/attempt should compress to one row
- status labels should stay mechanically simple
- a short free-text description matters

This aligns closely with the new `experiments.tsv` direction in `ict-engine`.

### 4. It uses a fixed experiment budget

The repo forces comparability by fixing the wall-clock budget at 5 minutes.

`ict-engine` cannot import this literally, because its workloads are not GPU-train-loop bounded in the same way. But the principle is still useful:

- experiments should be compared only under a clearly declared budget/constraint set
- if the comparison surface changes, the result should not be treated as apples-to-apples

For `ict-engine`, the equivalent budget dimensions are more likely:

- dataset identity
- timeframe bundle
- paired-market inputs
- state isolation vs shared state
- objective mode

This is a useful documentation pattern to keep reinforcing.

## What must not be copied into ict-engine

### 1. Git history must not become the experiment truth

In `autoresearch`, branch movement and commit hashes are part of the operating model.

That does not fit `ict-engine`.

Why:

- `ict-engine` already has canonical JSON state artifacts
- state dirs are intentionally decoupled from tracked source
- source repo history is already sensitive to oversized artifact pressure

So:

- commit hash should not become the primary experiment identity
- `git reset` should not become the keep/discard authority
- derived ledgers must not depend on git history to stay meaningful

### 2. Untracked TSV must not replace typed state

`autoresearch` leaves `results.tsv` untracked and operationally central.

That is acceptable for a tiny single-file local optimizer loop.

It is not acceptable for `ict-engine`, where:

- replayability matters
- machine-readable state already exists
- interruption/completion inference depends on structured files

Therefore:

- `experiments.tsv` should stay derived
- typed JSON remains canonical
- any future warning or integrity decision must come from typed state, not TSV rows

### 3. One editable-file simplicity does not transfer directly

`autoresearch` wins by constraining the editable surface to `train.py`.

`ict-engine` is a layered Rust CLI with domain, application, state, and reporting boundaries. Trying to mimic single-file minimalism directly would be counterproductive.

What transfers is the spirit:

- keep experimental write scope bounded
- prefer a clear "target surface" for each research push
- document what is in scope vs read-only for a loop

Not the literal architecture.

## Best-fit migration targets inside ict-engine

### 1. Derived ledger discipline

Best fit:

- `docs/autoresearch-derived-surfaces-contract.md`
- `docs/research-system-map.md`
- future operator docs around `experiments.tsv`

Pattern to absorb:

- one attempt -> one row
- short stable status labels
- short operator-facing note column

### 2. Retrospective writing discipline

Best fit:

- `factor_autoresearch_retrospective.md`
- any future warning or recap docs

Pattern to absorb:

- operators should be able to wake up and understand what happened quickly
- recap is valuable when it is concise, not exhaustive
- recap should summarize a loop, not replace the raw evidence

### 3. Loop discipline docs

Best fit:

- first-run docs
- autoresearch contract docs
- future runbook / operator docs

Pattern to absorb:

- establish baseline first
- compare against a declared reference
- keep/discard semantics must be explicit
- define what counts as crash / interrupted / unusable result

## Concrete recommendations for ict-engine docs

### Recommendation 1

Keep documenting `experiments.tsv` as:

- human-readable
- grep-friendly
- deletable/regenerable
- non-canonical

This is the strongest transferable pattern from `autoresearch`, with the right boundary correction.

### Recommendation 2

Add a short future operator note somewhere in the autoresearch docs:

- baseline first
- one isolated comparison surface at a time unless intentionally running a shared-state loop
- record why a keep/discard happened, not just that it happened

The upstream repo is unusually good at making that discipline explicit.

### Recommendation 3

If `ict-engine` later adds warning heuristics, describe them as loop aids, not truth overrides.

That preserves the right relation:

- canonical state decides what happened
- warning layers comment on it
- retrospectives summarize it

## Net effect on ict-engine

The right absorption is:

- **yes** to explicit loop contracts
- **yes** to small stable human-readable ledgers
- **yes** to concise retrospective/operator recap
- **no** to git-driven canonical truth
- **no** to TSV-first persistence
- **no** to importing a single-file-research architecture into a multi-layer trading CLI

## One-sentence takeaway

`autoresearch` is valuable to `ict-engine` not because it is a better execution engine, but because it demonstrates how much leverage comes from a brutally clear experiment loop, a tiny operator ledger, and a recap surface that helps a human understand the night run without rereading all raw artifacts.
