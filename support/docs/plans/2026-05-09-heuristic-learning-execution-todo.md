# Heuristic Learning Execution TODO

> Master implementation board. Follow this before each new slice.

**Goal:** turn the module harvest plan into a zero-config, consumer-usable, hot-pluggable self-iteration chain for ICT Engine.

**Ground rules:**
- Use TDD for code slices.
- Sidecar first; touch Rust runtime only when sidecar output cannot satisfy consumers.
- Write only under explicit output dirs or support/docs/scripts/tests.
- Ignore unrelated formatting drift from other agents.
- Commit each coherent slice after tests pass.

---

## Completed

- [x] Slice 1: Triple Barrier + Meta-labeling
  - Commit: `89a0007 feat: add heuristic payoff labeling tools`
  - Files: `labeling_triple_barrier.py`, tests
- [x] Slice 2: Payoff-shape report
  - Commit: `89a0007 feat: add heuristic payoff labeling tools`
  - Files: `factor_payoff_shape_report.py`, tests
- [x] Slice 3: Zero-config payoff pipeline
  - Commit: `35f509c feat: add heuristic payoff pipeline`
  - Files: `heuristic_payoff_pipeline.py`, handoff, tests
- [x] Slice 4: PSR/DSR high-Sharpe guard
  - Commit: `5da0318 feat: add deflated sharpe payoff guard`
- [x] Slice 5: Payoff-gated path-ranker target + BBN gate
  - Commit: `781a97a feat: export payoff gated path ranker targets`
  - Rule: `probe/promote` enter path-ranker + BBN; `reject` enters failure memory only
- [x] Slice 6: Purged CV / Embargo / PBO guard
  - Commit: `ca6200b feat: add purged cv payoff guard`
  - Files: `purged_cv_backtest_guard.py`, handoff, tests

- [x] Slice 7: Regime confidence report
  - Commit: `5f2e79f feat: add regime confidence report`
  - Files: `regime_confidence_report.py`, handoff, tests
- [x] Slice 8: Transition evidence aggregator
  - Commit: `e048f0a feat: add transition evidence aggregator`
  - Files: `transition_evidence_aggregator.py`, handoff, tests
  - Tests: `42 OK`
- [x] Slice 9: BBN evidence value report
  - Commit: `029800c feat: add bbn evidence value report`
  - Files: `bbn_evidence_value_report.py`, handoff, tests
  - Tests: target 3 OK; full research 45 OK
- [x] Slice 10: Risk-adjusted path utility
  - Commit: `b41f850 feat: add risk adjusted path utility`
  - Files: `payoff_to_path_ranker_target.py`, handoff, tests
  - Tests: target 3 OK; full research 46 OK
- [x] Slice 11: Formula seed library
  - Commit: `2672288 feat: add factor formula seed library`
  - Files: `factor_formula_library.py`, handoff, tests
  - Tests: target 3 OK; full research 49 OK
- [x] Slice 12: paper2code adapters
  - Commit: `11ac3c6 feat: add paper2code adapter reports`
  - Files: `paper2code_adapters.py`, handoff, tests
  - Tests: target 3 OK; full research 52 OK
- [x] Closure audit fix: orchestrate sidecars from payoff pipeline
  - Commit: `4243f56 fix: orchestrate heuristic sidecar closure`
  - Files: `heuristic_payoff_pipeline.py`, tests
  - Tests: target 3 OK; full research 53 OK

## Next Implementation Order

### Next

- Promote sidecar reports into a single orchestrated heuristic-learning pipeline.
- Feed accepted formula/adapters into payoff/PBO/BBN value gates.
- Defer Rust runtime changes until sidecar outputs prove consumer value.

---

## Current Slice Status

Active: Closure audit fix `heuristic_payoff_pipeline.py`.

- [x] Write failing tests
- [x] Implement minimal script
- [x] Add handoff doc
- [x] Run target tests
- [x] Run full research tests
- [x] Commit slice
