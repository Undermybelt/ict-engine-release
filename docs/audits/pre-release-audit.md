# Pre-release audit

Date: current repo head after release-hygiene, help-audit, agent-surface, and paired-data-quality commits.

## Verdict

Release status: shippable.

Blocking release issues found: none.

## What was verified

- `cargo check` passes
- `cargo test` passes
- mechanical help audit passes for all subcommands
- release-hygiene artifacts are ignored and not tracked
- agent surface exposes compact and structured next-step fields
- paired-market quality report is carried through factor pipeline debug surfaces
- dirty worktree is clean

## What was fixed in this release-prep pass

1. local dependency portability
   - `loopybayesnet` uses git dependency, not local absolute path

2. help quality
   - all 20 subcommands now have mechanically verified option descriptions
   - audit artifacts exist in:
     - `scripts/help_audit.py`
     - `docs/audits/help-audit.json`
     - `docs/audits/help-audit.md`

3. agent usability
   - `analyze --agent` now exposes:
     - `decision_hint_raw`
     - `decision_summary`
     - structured `next_step`

4. human usability
   - `analyze --human` and `workflow-status --human` provide readable summaries
   - `analyze --demo` exists for first-run verification

5. runtime hygiene
   - state dirs / pycache / tmp artifacts ignored
   - no tracked runtime junk remains

6. paired-market quality
   - structured paired-market quality report is preserved and preferred over explanation-string parsing

## Remaining non-blocking gaps

### 1. research truth closure surface is now present, and now ingests more experiment evidence

Added surface:
- `research-verdict`

It now ingests and summarizes:
- autoresearch sessions
- autoresearch attempts
- research runs
- backtest runs
- factor mutation runs / cluster hints
- artifact ledger

It emits:
- `best_known_baseline`
- `proven_bad_regions`
- `current_bottleneck`
- `recommended_next_experiment`
- `stop_or_continue`
- `comparison_contaminated`
- `contamination_reasons`

Remaining caveat:
- still compact by design; it is a release-grade closure surface, not yet a full experiment analytics warehouse

### 2. evidence-quality breakdown surface is now present

Added surface:
- `evidence-quality-breakdown`

It emits score components and gate gaps:
- support gap contribution
- uncertainty penalty
- directional conflict penalty
- mtf penalties / bonus
- liquidity adjustment
- hard/neutralized pass gaps

Remaining caveat:
- exact reconstruction depends on persisted Pre-Bayes fields; some inferred terms use current policy labels rather than raw intermediate variables

### 3. workflow-status agent surface now has structured `next_step`

Current state:
- `analyze --agent` has structured `next_step`
- `workflow-status --agent` now also has structured `next_step`

Agent orchestrators can route on:
- `next_step.action_type`
- `next_step.user_input_required`
- `next_step.blocked_reason`
- `next_step.deferred_command`

### 4. contamination / experiment-integrity signal is now first-class in `research-verdict`

Current state:
- `research-verdict` emits `comparison_contaminated`
- it also emits `contamination_reasons`

Remaining caveat:
- heuristics are conservative and compact; deeper experiment-family contamination analysis can still be added later

### 5. `src/main.rs` remains a structural debt hotspot

Status note (2026-04-24)

This section is now historical. The runtime-hotspot extraction line landed later in commits `8ce1024` and `3e45254`.
For current remaining debt, use `docs/plans/2026-04-24-post-main-debt-inventory.md`.

Current state:
- large monolith
- not a release blocker
- future iteration risk remains high

Mitigation already added:
- `docs/plans/main-rs-extraction-plan.md`

## Impact summary

### Impacting agent users now

Blocking: none.

Agent-facing surfaces now include:
- `analyze --agent.next_step`
- `workflow-status --agent.next_step`
- `research-verdict`
- `evidence-quality-breakdown`

### Impacting human users now

Blocking: none.

Still mildly suboptimal:
- no single verdict command for research closure
- score composition still needs manual investigation

### Impacting closed-loop operation now

Main remaining gap:
- verdict surface exists, but deeper experiment analytics can still improve after release

## Recommended next implementation order

1. harden `research-verdict` with deeper local/cluster result ingestion
2. harden contamination heuristics across experiment families
3. add richer evidence-quality raw intermediate persistence
4. staged `main.rs` extraction
