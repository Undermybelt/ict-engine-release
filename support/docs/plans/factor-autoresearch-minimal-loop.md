# Factor Autoresearch Minimal Loop Plan

> Goal: add a minimal checkpointed, benchmark-gated, rollback-aware mutation autoresearch surface to ict-engine without destabilizing existing factor-research / expansion-sop flows.

## Why

Current mutation flow can emit `FactorMutationEvaluation`, but it is still a loose probe surface:
- no durable autoresearch ledger
- no explicit baseline promotion
- no per-attempt keep/discard status artifact
- no compact failed-branch summary
- no resumable loop command

We want the smallest useful loop first, not a full autonomous framework.

## Target v0

Add one new CLI command:
- `factor-autoresearch`

This v0 command will:
1. load a starting mutation spec
2. run one or more atomic mutation attempts through existing `factor-research --emit-mutation-evaluation`
3. persist an autoresearch session ledger in state
4. mark each attempt as `keep` or `discard`
5. promote accepted attempts as the new baseline inside the session
6. write a compact failed-branch summary for rejected attempts
7. emit machine-readable JSON summary

## Non-goals for v0

- no git/worktree rollback integration yet
- no parallel child attempts in one run
- no LLM-generated mutation hypotheses inside the binary
- no hosted dashboard
- no replacement of existing `factor-research` / `expansion-sop`

## Design

### New state artifacts

Per symbol in `state/<SYMBOL>/`:
- `factor_autoresearch_sessions.json`
- `factor_autoresearch_attempts.json`

### New types

Add to `src/state/types.rs`:
- `FACTOR_AUTORESEARCH_SESSIONS_FILE`
- `FACTOR_AUTORESEARCH_ATTEMPTS_FILE`
- `FactorAutoresearchSession`
- `FactorAutoresearchAttempt`
- `FactorAutoresearchDecision`
- `FactorAutoresearchSummary`

Key fields:

`FactorAutoresearchSession`
- `session_id`
- `started_at`
- `updated_at`
- `symbol`
- `objective`
- `source_command`
- `base_factor`
- `baseline_mutation_id`
- `baseline_score`
- `attempts_total`
- `kept_attempts`
- `discarded_attempts`
- `last_attempt_id`
- `status`

`FactorAutoresearchAttempt`
- `session_id`
- `attempt_id`
- `timestamp`
- `symbol`
- `source_command`
- `base_factor`
- `baseline_mutation_id_before`
- `candidate_mutation_spec`
- `evaluation`
- `decision`
- `branch_summary`

`FactorAutoresearchDecision`
- `status` (`keep` | `discard`)
- `reason`
- `promoted_to_baseline`
- `baseline_score_before`
- `candidate_score`
- `score_delta`

### Persistence helpers

Add to `src/state/persistence.rs`:
- `append_factor_autoresearch_session`
- `load_factor_autoresearch_sessions`
- `save_factor_autoresearch_sessions`
- `append_factor_autoresearch_attempt`
- `load_factor_autoresearch_attempts`

Pattern: mirror existing append/load helpers.

### CLI surface

Add command in `src/main.rs`:
- `FactorAutoresearch`

Arguments, minimal set:
- `--symbol`
- `--data`
- `--objective`
- `--state-dir`
- `--mutation-spec`
- `--iterations` default `1`
- optional MTF paths: `--data-1m --data-5m --data-15m --data-1h --data-4h --data-1d`
- `--paired-data`
- `--session-id` optional resume
- `--ensemble`

### Command behavior

Algorithm:
1. resolve starting session
   - if `--session-id` given, load session and latest baseline from attempts
   - else create new session from provided mutation spec
2. candidate spec = current baseline spec
3. for each iteration:
   - run existing `run_factor_research(...)` with candidate spec
   - require `factor_mutation_evaluation` to exist
   - derive keep/discard from `evaluation.accepted`
   - write `FactorAutoresearchAttempt`
   - if keep:
     - baseline spec = `candidate spec`
     - baseline mutation id/score update in session
     - next candidate = `next_mutation_spec_template(Some(candidate), evaluation, true)`
   - else:
     - write branch summary from failure tags / reason / next directions
     - next candidate = `next_mutation_spec_template(Some(candidate), evaluation, true)`
4. persist updated session counters
5. print `FactorAutoresearchSummary`

### Branch summary shape

Compact string array, no essays:
- `reason=...`
- `failure_tags=a|b|c`
- `wrong_direction=...` if inferable from recommended directions
- `next_focus=...`

### Verification path

After implementation:
- `cargo fmt`
- `cargo check`
- targeted tests for new persistence / summary helpers
- one smoke run of `cargo run -- factor-autoresearch --help`

## Planned file touches

- `support/docs/plans/factor-autoresearch-minimal-loop.md`
- `src/state/types.rs`
- `src/state/persistence.rs`
- `src/main.rs`

## Notes

v0 deliberately reuses existing mutation scoring and next-spec generation rather than inventing a second optimizer.
