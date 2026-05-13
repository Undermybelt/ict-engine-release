# Release signoff

Date: 2026-05-13
Status: sanitized `v0.1.2` release candidate authorized for mirror publication;
publish only the verified export slice, not the broad dirty working tree.

## Final verdict

Do not publish the whole dirty working tree.

The current verified candidate is a sanitized export recorded in
`docs/audits/2026-05-13-sanitized-release-candidate-manifest.md`. It passes Rust
fmt, Clippy, full tests, and a true zero-config consumer smoke. The current
source checkout still has unrelated dirty docs/runtime artifacts and legacy
local-data research scripts, so a publisher must use the exact sanitized export
slice or rerun the full gate after changing the slice.

The 2026-05-13 publish instruction authorizes the release mirror flow for
`v0.1.2` after refreshing the export with the README/AGENT polish and rerunning
the gates.

## Important release routing decision

This checkout currently tracks:

```text
origin git@github.com:Undermybelt/givenup-ict-engine.git
```

Release metadata points at:

```text
Undermybelt/ict-engine-release
```

Use the release mirror flow. Do not reuse `v0.1.1`; the mirror already has that
tag. Re-check remote tags before pushing `v0.1.2`.

## Signoff checklist

### Build and test
- [x] sanitized export `cargo fmt --check`
- [x] sanitized export `cargo clippy --all-targets -- -D warnings`
- [x] sanitized export `cargo test`
- [ ] Python pytest suite: not rerun during this release-prep pass
- [x] release export starts from committed `HEAD`
- [x] release export overlays the audited candidate slice
- [x] unrelated source worktree dirt excluded from release export

### CLI and consumer quality
- [x] `workflow-status` exposes opt-in profile choices without auto-adoption
- [x] agent output keeps selected profile state explicit
- [x] zero-config tests do not depend on maintainer-local `state/` files
- [x] BBN fixture files are tracked, small, and path-redacted
- [x] runtime BBN overlays remain hot-pluggable via user state
- [x] true zero-config smoke runs without provider venv injection
- [x] smoke output privacy scan has no private path or secret-like matches

### Portability and state hygiene
- [x] release tag/version selected as `v0.1.2`
- [x] no tracked `state*` files are required by the clean export fixture fix
- [x] generated Auto-Quant dependency workspaces are not staged for release
- [ ] legacy local-data research scripts are excluded from the verified export;
  rewrite them before publishing them as public examples

## Commands executed for signoff

```bash
cargo fmt --manifest-path /tmp/ict-engine-v012-release-export.CHyo93/Cargo.toml --check
CARGO_TARGET_DIR=/tmp/ict-engine-v012-release-target.NJjdD3 cargo clippy --manifest-path /tmp/ict-engine-v012-release-export.CHyo93/Cargo.toml --all-targets -- -D warnings
PATH=<provider-venv>/bin:$PATH CARGO_TARGET_DIR=/tmp/ict-engine-v012-release-target.NJjdD3 cargo test --manifest-path /tmp/ict-engine-v012-release-export.CHyo93/Cargo.toml
```

## Decisive outcomes

### Sanitized export gates
- `cargo fmt --manifest-path /tmp/ict-engine-fresh-release-export.A0JQ2T/Cargo.toml --check`: passed
- `cargo clippy --manifest-path /tmp/ict-engine-fresh-release-export.A0JQ2T/Cargo.toml --all-targets -- -D warnings`: passed
- `cargo test --manifest-path /tmp/ict-engine-fresh-release-export.A0JQ2T/Cargo.toml`: passed
  - lib tests: 963 passed
  - bin tests: 253 passed
  - integration tests: passed
  - doc tests: 0 passed, 0 failed
- `cargo fmt --manifest-path /tmp/ict-engine-v012-release-export.CHyo93/Cargo.toml --check`: passed
- `cargo clippy --manifest-path /tmp/ict-engine-v012-release-export.CHyo93/Cargo.toml --all-targets -- -D warnings`: passed
- `cargo test --manifest-path /tmp/ict-engine-v012-release-export.CHyo93/Cargo.toml`: passed
  - lib tests: 963 passed
  - bin tests: 253 passed
  - integration tests: passed
  - doc tests: 0 passed, 0 failed

### True zero-config smoke
- Binary: `/tmp/ict-engine-fresh-release-target.xX95Dv/debug/ict-engine`
- State: `/tmp/ict-engine-fresh-smoke-state.j5BH7I`
- Output: `/tmp/ict-engine-fresh-smoke-out.sCBMAY`
- Final `v0.1.2` binary:
  `/tmp/ict-engine-v012-release-target.NJjdD3/debug/ict-engine`
- Final state: `/tmp/ict-engine-v012-smoke-state.M78llx`
- Final output: `/tmp/ict-engine-v012-smoke-out.yszAfG`
- Passed provider, workflow, analyze demo, workflow refresh, Pre-Bayes,
  policy-training, candidate-pack, admission-target, and regime-asset commands.
- Smoke stderr files were empty.
- Smoke-output privacy scan found no private paths or secret-like strings.

## Release caveats

1. Branch is still far ahead of the source remote; this release uses the mirror
   flow to publish clean tree state without rewriting source history.
2. The current source checkout has unrelated dirty docs/runtime artifacts that
   were not staged into this release candidate.
3. Python pytest is outside the current Rust release gate.
4. The mirror already has `v0.1.1`; re-check remote tags before pushing
   `v0.1.2`.
5. Legacy local-data research scripts are not part of the verified public
   candidate unless rewritten around explicit inputs and re-gated.
6. Historical docs/prompts with maintainer-local absolute paths are pruned from
   the mirror release tree unless redacted first.

## Release recommendation

Publish `v0.1.2` only through the sanitized export/mirror flow after the final
README/AGENT-polished export gate passes.
