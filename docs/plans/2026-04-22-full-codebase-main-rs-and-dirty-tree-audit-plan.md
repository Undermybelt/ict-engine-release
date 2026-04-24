# Full Codebase Main.rs And Dirty Tree Audit Plan

## Goal

Produce a repo-local audit in `docs/` for the entire `ict-engine` codebase with extra focus on:

- `src/main.rs` bloat, coupling, and extraction strategy
- whether the current uncommitted changes are shippable
- open-source contributor first-run and test-drive experience
- post-release human user and agent-consumer failure modes
- missing functionality, documentation, and test coverage

## Outputs

1. A written audit in `docs/` with findings, severity, reproduction, impact, and concrete fixes.
2. A dedicated section on how to break down `src/main.rs` without destabilizing the product.
3. Trial-run simulations for contributors, CLI users, and agent consumers.
4. Recommendations for release gates, tests, docs, and interface cleanup.

## Method

1. Capture repo state, dirty tree scope, existing audit docs, and test entrypoints.
2. Inspect `src/main.rs` structure by command surface, helper clusters, and dependency fan-out.
3. Review high-risk modified files and untracked additions to find incomplete work or regressions.
4. Run feasible verification commands (`cargo check`, targeted `cargo test`, CLI help/smoke flows).
5. Simulate contributor onboarding, end-user operation, and agent integration journeys.
6. Write the final audit to `docs/audits/`.

## Constraints

- Do not revert unrelated user changes in the dirty worktree.
- Do not claim a path works unless it was verified or clearly marked as inference.
- Keep code changes limited to documentation unless follow-up implementation is explicitly requested.
