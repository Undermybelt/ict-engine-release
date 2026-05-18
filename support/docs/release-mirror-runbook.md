# Private mirror release runbook

Purpose:
- publish release tags without rewriting primary research/development history
- keep both source repo and release repo private
- publish a clean tree-state mirror when the development repo has extensive local history
- keep distribution inside the rights granted by `LICENSE`

Status note (2026-05-10):
- source repo pushes are available, but this checkout currently tracks `git@github.com:Undermybelt/givenup-ict-engine.git`
- `Cargo.toml` and release metadata point at `Undermybelt/ict-engine-release`
- confirm the source repo, mirror repo, and release tag before publishing

License note (2026-05-13):
- `LICENSE` is PolyForm Noncommercial License 1.0.0, not MIT/Apache/open
  source.
- `Cargo.toml` uses `license = "PolyForm-Noncommercial-1.0.0"` and
  `publish = false`.
- Do not publish crates.io, npm/npx, Homebrew, Docker, public binary, or public
  tap artifacts from this release flow unless a dedicated packaging slice
  verifies the channel complies with PolyForm Noncommercial 1.0.0 and required
  notices.

Repos:
- development truth: current working repo / configured origin
- private release mirror: `Undermybelt/ict-engine-release`

## When to use

Use this flow when a release should represent the current tree state rather than the full experimental source history.

## Required inputs

Set these explicitly before running the flow:

```bash
RELEASE_TAG=v0.1.0-preview
RELEASE_TITLE="ict-engine ${RELEASE_TAG}"
RELEASE_EXPORT_DIR="$(mktemp -d /tmp/ict-engine-release-export.XXXXXX)"
RELEASE_MIRROR_REPO="https://github.com/Undermybelt/ict-engine-release.git"
```

## Release flow

For the current 2026-05-13 candidate, first read
`support/docs/audits/2026-05-13-sanitized-release-candidate-manifest.md`. The current
verified candidate is a sanitized export slice, not a broad dirty-worktree sync.

1. Verify source repo from the repo root:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -- analyze --symbol DEMO --demo --state-dir /tmp/ict-engine-first-run-native --human
python3 support/scripts/search_local.py --show-config
python3 support/scripts/search_cluster.py --show-config
python3 support/scripts/evaluate_bottleneck.py --show-config
```

Optional, when Python test tooling is installed:

```bash
python3 -m pytest support/scripts/research/tests support/scripts/auto_quant_external/tests
```

2. Confirm release tree hygiene:

```bash
git status --short --branch
git diff --stat
git tag --list 'v*' --sort=version:refname
```

Do not publish with unexpected untracked files, generated runtime state, local configs, or stale signoff docs.

3. Export current committed tree state:

```bash
git archive --format=tar HEAD | tar -x -C "$RELEASE_EXPORT_DIR"
```

4. Initialize clean release repo:

```bash
cd "$RELEASE_EXPORT_DIR"
git init
git checkout -b main
git add .
git commit -m "release: ict-engine ${RELEASE_TAG}"
```

5. Point at private mirror and publish:

```bash
git remote add origin "$RELEASE_MIRROR_REPO"
git tag -a "$RELEASE_TAG" -m "$RELEASE_TAG"
git push origin main
git push origin "$RELEASE_TAG"
```

6. Create private GitHub release:

```bash
gh release create "$RELEASE_TAG" \
  --repo Undermybelt/ict-engine-release \
  --title "$RELEASE_TITLE" \
  --notes-file support/docs/audits/release-signoff.md
```

## Rules

- source repo remains the development / experiment truth
- mirror repo remains the preferred clean release transport surface
- mirror access must remain private unless the license is revised for public
  redistribution
- release notes should point back to source-repo docs where needed
- bump the version every release
- refresh `support/docs/audits/release-signoff.md` and `support/docs/release-notes-draft.md` before publishing
- never reuse an existing tag name
- never run `git push`, `git tag`, or `gh release create` without an explicit operator confirmation for this release
- never add public package-manager publication to the release flow without a
  dedicated PolyForm Noncommercial channel-compliance review

## Post-release follow-up

- keep `support/docs/audits/release-signoff.md` current before every release
- if mirror release flow becomes standard, automate the variableized flow above
