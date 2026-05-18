# Release Notes

Version: `v0.1.4`
Status: sanitized release candidate prepared for mirror publication, drafted
2026-05-18; package-manager publication remains blocked under the PolyForm
Noncommercial 1.0.0 release policy.

## Highlights (v0.1.4 — gate-rigidity audit slice)

- Two factor-iteration hard gates that were empirically unreachable on real
  5-class regime/path classification have been relaxed so the rest of the
  promotion chain can be exercised:
  - `MECE_RECOVERY_ACCURACY_GATE`: 0.95 -> 0.55 (commit c8a45f12)
  - `STRUCTURAL_PATH_RANKING_EXECUTION_GATE_MIN_PATH_PROB`: 0.5 -> 0.30
    (commit c8a45f12)
- The OU overlay's `regime_influence_enabled` chicken-and-egg gate was aligned
  with the spectral overlay: activation moved from `EXECUTION_GATE_READY`
  (0.65) to `EXECUTION_GATE_OBSERVE` (0.45) so OU evidence can lift readiness
  from the observe band upward (commit a4d98718).
- A 9-round factor-iteration gate-rigidity audit (87 findings, 9 meta-patterns,
  next-slice priority order) has been committed as
  `support/docs/plans/2026-05-18-gate-rigidity-audit-todo.md` (commit 57b39e9d)
  so future agents do not re-run the same exploration.
- Downstream BBN / CatBoost / maturity row / execution-tree admission gates
  continue to enforce live-promotion criteria; these relaxations remove
  construction-time fail-closed zero points, they do not lower live-readiness
  standards.

## Inherited from v0.1.2 baseline

- README and `AGENT.md` were refreshed as public entrypoints: the README leads
  with a clean first-run path and readable workflow map; `AGENT.md` tells
  agents how to serve users, verify gates, preserve privacy, and publish only
  sanitized export slices.
- `workflow-status` surfaces matching opt-in provider/profile choices for the
  requested symbol without selecting or loading maintainer-local material.
- Agent and human workflow surfaces stay token-friendly.
- Branch-admission routing no longer overrides first-run / Auto-Quant handoff /
  evidence-review / selected-profile / generic execution-contract guidance
  unless the latest feedback is for the exact same structural path.
- Structural path-plan artifacts carry candidate set ids and candidate paths,
  and path-ranking target rows expose branch segment categorical fields for
  external ranker training.
- BBN CPT and logic-family tests use tracked, path-redacted fixtures under
  `tests/fixtures/policy_training/`.
- License metadata uses `PolyForm-Noncommercial-1.0.0` in `Cargo.toml`, with
  `publish = false`; public package-manager redistribution needs a dedicated
  channel-compliance review.

## Release gates

The release-mirror runbook requires these to pass from a fresh
`git archive HEAD` export before tagging:

```bash
cargo fmt --manifest-path "$EXPORT_DIR/Cargo.toml" --check
cargo clippy --manifest-path "$EXPORT_DIR/Cargo.toml" --all-targets -- -D warnings
cargo test --manifest-path "$EXPORT_DIR/Cargo.toml"
```

Current targeted-test evidence from the working tree before tagging:
- `domain::regime::mece_artifact`: 3/3 pass
- `belief_core::structural_path_ranking` related: 16/16 pass
- `tests/mece_recovery` integration: 2/2 pass
- `tests/hard_gate_execution_first`: 6/6 pass
- `application::belief::ou_overlay`: 1/1 pass (renamed to reflect new
  semantics)

Full release-gate evidence will be recorded under "Smoke results" once the
clean-export gates are run.

## Smoke results

(filled in by the release script after `git archive HEAD` is exported and the
fmt/clippy/test/zero-config-smoke commands have run from that export)

## Known limitations

- Agent-first / researcher-preview release, not a fully generalized packaged
  distribution.
- PolyForm Noncommercial License 1.0.0; not approved for public package-manager
  redistribution in this release flow.
- The two relaxed gate constants (0.55 MECE, 0.30 path_prob) sit in the
  empirically reachable band but are placeholder values; fresh OOS calibration
  is still required before they should be treated as final.
- Python pytest is not rerun during this release-prep pass.
- Auto-Quant remains optional and should keep dependency workspaces under the
  selected state directory or explicit Auto-Quant output directory.
- Local long-history data can be used for maintainer training and hardening,
  but consumer-facing promotion still requires a portable provider recipe,
  built-in factor path, or explicit hot-plug material bundle.
- The source checkout has unrelated dirty Board B in-flight artifacts; this
  candidate is based on `git archive HEAD`, not a broad worktree sync.
- `v0.0.1` and `v0.1.0` already exist in the release mirror; this release uses
  `v0.1.4` after remote tag re-check.

## Release label

`ict-engine v0.1.4`

Reason:
- two unreachable hard gates relaxed so iteration promotion can be reached
- OU overlay chicken-and-egg gate broken so overlay evidence can lift readiness
- 9-round gate-rigidity audit committed as durable artifact under
  `support/docs/plans/`
- consumer-safe hot-plug profile-choice UX inherited from v0.1.2 baseline
- clean-export Rust fmt, Clippy, and full-test gates required to be re-run from
  the fresh archive before publish
