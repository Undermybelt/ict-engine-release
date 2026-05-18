# Release signoff

Date: 2026-05-18
Status: sanitized `v0.1.4` release candidate prepared for mirror publication;
publish only the verified export slice, not the broad dirty working tree. Public
package-manager publication is blocked in this release flow pending a PolyForm
Noncommercial 1.0.0 channel-compliance review.

## Final verdict

Do not publish the whole dirty working tree.

The `v0.1.4` candidate is the committed `HEAD` after the gate-rigidity audit
slice. The source checkout still carries unrelated dirty Board B in-flight
files; a publisher must use `git archive HEAD` and rerun the full gate, not a
broad worktree sync.

The 2026-05-18 publish instruction authorizes the release mirror flow for
`v0.1.4` after refreshing `Cargo.toml`, `release-signoff.md`, and
`release-notes-draft.md`, and rerunning the gates from a fresh export.

After any later README/AGENT/license refresh the prior `v0.1.4` evidence is no
longer sufficient for a new publish. Rebuild a fresh sanitized export and rerun
the release gates before publishing another tag or updating the mirror.

## Important release routing decision

This checkout currently tracks:

```text
origin git@github.com:Undermybelt/givenup-ict-engine.git
```

Release metadata points at:

```text
Undermybelt/ict-engine-release
```

Use the release mirror flow. Mirror already has `v0.0.1` and `v0.1.0`; `v0.1.1`
and `v0.1.2` were drafted but never pushed and remain available. The new tag is
`v0.1.4`; re-check remote tags before pushing.

## Signoff checklist

### Build and test
- [ ] sanitized export `cargo fmt --check`
- [ ] sanitized export `cargo clippy --all-targets -- -D warnings`
- [ ] sanitized export `cargo test`
- [ ] Python pytest suite: not rerun during this release-prep pass
- [x] release export starts from committed `HEAD`
- [x] unrelated source worktree dirt excluded from release export (uses
  `git archive HEAD`)

### CLI and consumer quality
- [x] `workflow-status` exposes opt-in profile choices without auto-adoption
  (inherited from v0.1.2 baseline)
- [x] agent output keeps selected profile state explicit
- [x] zero-config tests do not depend on maintainer-local `state/` files
- [x] BBN fixture files are tracked, small, and path-redacted
- [x] runtime BBN overlays remain hot-pluggable via user state

### Portability and state hygiene
- [x] release tag/version selected as `v0.1.4`
- [x] generated Auto-Quant dependency workspaces are not staged for release
- [x] Cargo metadata uses `license = "PolyForm-Noncommercial-1.0.0"` and
  `publish = false`
- [x] public package-manager publication is blocked unless the license changes
- [ ] legacy local-data research scripts remain excluded from the verified
  export; rewrite them before publishing them as public examples

### Gate-rigidity slice additions
- [x] `MECE_RECOVERY_ACCURACY_GATE` relaxed from 0.95 to 0.55 (commit c8a45f12)
- [x] `STRUCTURAL_PATH_RANKING_EXECUTION_GATE_MIN_PATH_PROB` relaxed from 0.5
  to 0.30 (commit c8a45f12)
- [x] OU overlay activation gate aligned with spectral overlay at
  `EXECUTION_GATE_OBSERVE` (commit a4d98718)
- [x] 9-round gate-rigidity audit doc committed at
  `support/docs/plans/2026-05-18-gate-rigidity-audit-todo.md` (commit 57b39e9d)

## Commands to execute for signoff

```bash
RELEASE_EXPORT_DIR=$(mktemp -d /tmp/ict-engine-v013-release-export.XXXXXX)
git archive --format=tar HEAD | tar -x -C "$RELEASE_EXPORT_DIR"
cargo fmt --manifest-path "$RELEASE_EXPORT_DIR/Cargo.toml" --check
cargo clippy --manifest-path "$RELEASE_EXPORT_DIR/Cargo.toml" --all-targets -- -D warnings
cargo test --manifest-path "$RELEASE_EXPORT_DIR/Cargo.toml"
```

Update the checklist above with [x] markers when each command passes from the
fresh export.

## Release caveats

1. Branch is still far ahead of the source remote; this release uses the mirror
   flow to publish clean tree state without rewriting source history.
2. The current source checkout has unrelated dirty Board B in-flight artifacts
   that are not staged into this release candidate.
3. Python pytest is outside the current Rust release gate.
4. The mirror already has `v0.0.1` and `v0.1.0`; verify remote tags before
   pushing `v0.1.4`.
5. Legacy local-data research scripts are not part of the verified public
   candidate unless rewritten around explicit inputs and re-gated.
6. Historical support/docs/prompts with maintainer-local absolute paths are
   pruned from the mirror release tree unless redacted first.
7. Public crates.io, npm/npx, Homebrew, Docker, or binary distribution is not
   authorized by the current license.
8. Two gate constants (`MECE_RECOVERY_ACCURACY_GATE` and the path-prob lower
   bound) were calibration placeholders in the reachable band — they need
   fresh OOS calibration before being treated as final values.

## Release recommendation

Publish only through the sanitized private export/mirror flow after the final
release-export gate passes. Do not use public package-manager channels until a
dedicated PolyForm Noncommercial channel review passes.
