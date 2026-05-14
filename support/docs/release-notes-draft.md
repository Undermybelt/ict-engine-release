# Release Notes

Version: `v0.1.2`
Status: sanitized release candidate authorized for mirror publication,
refreshed 2026-05-13; package-manager publication remains blocked under the
PolyForm Noncommercial 1.0.0 release policy.

## Highlights

- README and `AGENT.md` were refreshed as public entrypoints: the README now
  leads with a clean first-run path and readable workflow map; `AGENT.md` now
  tells agents how to serve users, verify gates, preserve privacy, and publish
  only sanitized export slices.
- `workflow-status` now surfaces matching opt-in provider/profile choices for
  the requested symbol without selecting or loading maintainer-local material.
- Agent and human workflow surfaces stay token-friendly: optional profile
  references are compact, and selected profile state remains explicit.
- Branch-admission routing no longer overrides first-run, Auto-Quant handoff,
  evidence-review, selected-profile, or generic execution-contract guidance
  unless the latest feedback is for the exact same structural path.
- Structural path-plan artifacts carry candidate set ids and candidate paths,
  and path-ranking target rows expose branch segment categorical fields for
  external ranker training.
- BBN CPT and logic-family tests now use tracked, path-redacted fixtures under
  `tests/fixtures/policy_training/`; runtime overlays remain hot-pluggable via
  user state and are not adopted as zero-config defaults.
- Sanitized release export gates are green:
  - `cargo fmt --manifest-path /tmp/ict-engine-v012-release-export.CHyo93/Cargo.toml --check`
  - `cargo clippy --manifest-path /tmp/ict-engine-v012-release-export.CHyo93/Cargo.toml --all-targets -- -D warnings`
  - `cargo test --manifest-path /tmp/ict-engine-v012-release-export.CHyo93/Cargo.toml`
- True zero-config smoke passes without provider venv injection.
- License metadata now uses `PolyForm-Noncommercial-1.0.0` in `Cargo.toml`,
  with `publish = false`; public crates.io, npm/npx, Homebrew, Docker, and
  binary redistribution need a dedicated channel-compliance review before they
  are added to the release flow.

## Smoke results from 2026-05-13

```bash
cargo fmt --manifest-path /tmp/ict-engine-v012-release-export.CHyo93/Cargo.toml --check
cargo clippy --manifest-path /tmp/ict-engine-v012-release-export.CHyo93/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path /tmp/ict-engine-v012-release-export.CHyo93/Cargo.toml
```

All passed from the sanitized release export. The full clean-export Rust suite
reported:
- lib tests: 963 passed
- bin tests: 253 passed
- integration tests: passed
- doc tests: 0 passed, 0 failed

Zero-config smoke also passed from
`/tmp/ict-engine-v012-release-target.NJjdD3/debug/ict-engine` with state under
`/tmp/ict-engine-v012-smoke-state.M78llx`.

## Known limitations

- This remains an agent-first / researcher-preview release, not a fully
  generalized packaged distribution.
- It is under PolyForm Noncommercial License 1.0.0 and is not approved for
  public package-manager redistribution in this release flow.
- Python pytest was not rerun during this release-prep pass.
- Auto-Quant remains optional and should keep dependency workspaces under the
  selected state directory or explicit Auto-Quant output directory.
- Local long-history data can be used for maintainer training and hardening,
  but consumer-facing promotion still requires a portable provider recipe,
  built-in factor path, or explicit hot-plug material bundle.
- The source checkout has unrelated dirty support documentation/runtime artifacts; this
  candidate is based on the sanitized manifest, not a broad worktree sync.
- Legacy local-data research scripts remain excluded from the verified public
  candidate unless rewritten around explicit inputs and re-gated.
- `v0.1.1` already exists in the release mirror; this release uses `v0.1.2`
  after remote tag re-check.

## Release label

`ict-engine v0.1.2`

Reason:
- consumer-safe hot-plug profile-choice UX is committed
- local personal-data assumptions remain opt-in, not zero-config defaults
- clean export no longer depends on ignored `state/policy_training` fixtures
- clean export Rust fmt, Clippy, and full test gates are green
