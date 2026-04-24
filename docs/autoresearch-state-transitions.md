# Autoresearch State Transitions

Goal: document the actual write order and state-transition semantics of `factor-autoresearch` so contributors and agents can reason about the session truth mechanically.

This document complements:

- `docs/research-system-map.md`
- `docs/autoresearch-derived-surfaces-contract.md`

Those docs define surfaces and trust boundaries.
This doc defines the transition order.

## 1. State surfaces involved

Per symbol, `factor-autoresearch` touches the following files under:

```text
<state_dir>/<SYMBOL>/
```

Canonical surfaces:

- `factor_autoresearch_attempts.json`
- `factor_autoresearch_sessions.json`
- `factor_autoresearch_live.json`
- `factor_autoresearch_final.json`

Derived surfaces:

- `experiments.tsv`
- `factor_autoresearch_retrospective.md`

Core rule:

- canonical JSON files define the session truth
- derived files summarize or reformat that truth

## 2. Session start transition

When `factor-autoresearch` starts, the command first:

1. resolves or creates a `session_id`
2. loads existing sessions for the symbol
3. loads prior attempts for this `session_id`
4. reconstructs the current mutation spec from:
   - latest promoted baseline in this session, else
   - latest attempt in this session, else
   - initial CLI mutation spec
5. constructs a `factor_autoresearch_live.json` snapshot with:
   - `status = "running"`
   - `current_iteration = 0`
   - session counters copied from existing session state
6. writes `factor_autoresearch_live.json`

At this point:

- the live snapshot exists and says `running`
- `sessions.json` is not yet updated for this new invocation
- `final.json` is not yet updated

This means the live snapshot is the earliest canonical signal that a run is active.

## 3. Per-iteration transition

Each autoresearch iteration currently follows this order.

### Step 1: refresh live snapshot

Before running factor research for the current candidate, the command updates and writes:

- `factor_autoresearch_live.json`

with:

- incremented `current_iteration`
- refreshed `updated_at`
- current candidate mutation spec
- `status = "running"`

### Step 2: run factor research

The command executes `run_factor_research(...)` using the current candidate mutation spec.

This produces the evaluation used for keep/discard.

### Step 3: materialize one attempt

The command builds one `FactorAutoresearchAttempt` record containing:

- session identity
- attempt identity
- candidate mutation spec
- evaluation
- decision
- branch summary

### Step 4: append canonical attempt truth

The command appends that attempt to:

- `factor_autoresearch_attempts.json`

This is the first canonical record of the finished iteration.

### Step 5: refresh derived TSV

After the attempt append succeeds, the command tries to regenerate:

- `experiments.tsv`

Important:

- this refresh is best-effort
- failure prints a warning to stderr
- failure does **not** abort the canonical autoresearch loop

### Step 6: mutate in-memory session counters and next candidate

Only after the attempt is appended does the command update in-memory session state:

- `attempts_total`
- `kept_attempts` or `discarded_attempts`
- `baseline_mutation_id`
- `baseline_score`
- `base_factor`
- `last_attempt_id`

Then it computes the next mutation spec template for the next iteration.

### Step 7: loop continues

The next iteration begins by refreshing `factor_autoresearch_live.json` again.

## 4. Completion transition

When all iterations finish normally, the command performs the following completion sequence.

### Step 1: finalize live snapshot

The command updates and writes:

- `factor_autoresearch_live.json`

with:

- final attempt counters
- latest attempt id
- final current candidate spec
- `status = "completed"`

### Step 2: persist session rollup

The command upserts the session record into:

- `factor_autoresearch_sessions.json`

This is the canonical session-level rollup.

### Step 3: persist final summary

The command writes:

- `factor_autoresearch_final.json`

This is the canonical completed-session summary artifact.

### Step 4: refresh derived retrospective

After the final summary is written, the command tries to regenerate:

- `factor_autoresearch_retrospective.md`

Important:

- this refresh is best-effort
- failure prints a warning to stderr
- failure does **not** roll back canonical completion artifacts

### Step 5: emit stdout summary

The command prints the final summary JSON to stdout.

## 5. What “running”, “completed”, and “interrupted” mean

The status rollup logic currently derives `effective_status` from:

- `factor_autoresearch_live.json`
- presence of `factor_autoresearch_final.json`

The rules are:

### `completed`

The session is treated as `completed` if either:

- `factor_autoresearch_final.json` exists, or
- the live snapshot says `status == "completed"`

### `running`

The session is treated as `running` if:

- the live snapshot says `status == "running"`, and
- the snapshot is not stale

### `interrupted`

The session is treated as `interrupted` if:

- the live snapshot says `status == "running"`, and
- `updated_at` is older than the current staleness threshold, and
- no stronger completion signal overrides it

Current threshold:

- 10 minutes

### `unknown`

The session is treated as `unknown` if none of the above conditions hold.

## 6. Why the write order matters

The current ordering creates a few important operator guarantees.

### Guarantee 1: attempt truth lands before derived ledger refresh

`experiments.tsv` is always downstream of `factor_autoresearch_attempts.json`.

Therefore:

- if the TSV is missing or stale, regenerate it from attempts
- do not treat TSV freshness as attempt truth

### Guarantee 2: final summary lands before retrospective refresh

`factor_autoresearch_retrospective.md` is downstream of canonical completion artifacts.

Therefore:

- if the retrospective is missing, canonical completion may still be valid
- do not use retrospective existence as the sole completion signal

### Guarantee 3: live snapshot is the earliest active-run signal

During execution, the first canonical sign of activity is the live snapshot.

Therefore:

- “run has started” should be inferred from `factor_autoresearch_live.json`
- not from sessions/final/retrospective presence

## 7. Correct read order during and after a run

### If you want to know whether a run is active

Read in this order:

1. `factor_autoresearch_live.json`
2. `factor-autoresearch-status`

### If you want canonical completed-session truth

Read in this order:

1. `factor-autoresearch-status --latest-only`
2. `factor_autoresearch_final.json`
3. `factor_autoresearch_attempts.json`
4. `factor_autoresearch_sessions.json`

### If you want convenience surfaces

Read only after canonical truth:

5. `experiments.tsv`
6. `factor_autoresearch_retrospective.md`

## 8. Failure semantics

### Derived refresh failure

Current behavior:

- `experiments.tsv` refresh failure only emits a warning
- retrospective refresh failure only emits a warning

Canonical state remains valid if the canonical writes succeeded.

### Mid-loop interruption

If the process dies mid-loop, typical observable state is:

- `factor_autoresearch_live.json` says `running`
- some attempts may already be appended
- `factor_autoresearch_final.json` may be absent

In this case, status logic may classify the session as `interrupted` once the live snapshot becomes stale.

### No final artifact

If `factor_autoresearch_final.json` does not exist, do not assume reliable completion from recap files or operator intuition.

## 9. Contributor rules

When changing autoresearch persistence or status logic:

1. preserve canonical-before-derived ordering
2. update this document if write order changes
3. update `docs/autoresearch-derived-surfaces-contract.md` if trust boundaries change
4. update `docs/research-system-map.md` if read order or state-file meaning changes

## 10. Non-goals

This document does not define:

- the keep/discard heuristic itself
- warning heuristics in detail
- future CLI flags that do not yet exist
- optimizer strategy quality claims

It only defines the current session-state transition semantics.
