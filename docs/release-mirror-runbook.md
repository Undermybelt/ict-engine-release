# Private mirror release runbook

Purpose
- publish release tags without rewriting the primary research repo history
- keep both source repo and release repo private

Repos
- source repo: `Undermybelt/ict-engine`
- private release mirror: `Undermybelt/ict-engine-release`

## Hard constraint (post-v0.0.1)

- The source repo has **no working publishing origin**. GitHub rejects pushes because the repo's history contains state artifacts over 100 MB (e.g. `state100/NQ/learning_state.json`, `state_autoresearch_resume_bg/NQ/learning_state.json`).
- All outward publishing happens **only** through the release mirror `Undermybelt/ict-engine-release`.
- Do not run `git push origin …` from the source repo. Do not add a public remote to the source repo.
- Local commits on the source repo accumulate on the local clone(s) only — source repo is the private development truth.
- Every external release (tag + GitHub release) is a fresh `git archive` export into the mirror, not an incremental push.

When to use
- whenever a new release tag is needed
- release should represent current tree state, not full experimental history

Release flow

1. verify source repo
```bash
cargo check
cargo test
python3 scripts/help_audit.py
cargo run --quiet -- research-verdict --symbol DEMO --state-dir state
cargo run --quiet -- evidence-quality-breakdown --symbol DEMO --state-dir state
```

2. export current tree state
```bash
rm -rf /tmp/ict-engine-release-export
mkdir -p /tmp/ict-engine-release-export
git archive --format=tar HEAD | tar -x -C /tmp/ict-engine-release-export
```

3. initialize clean release repo
```bash
cd /tmp/ict-engine-release-export
git init
git checkout -b main
git add .
git commit -m "release: ict-engine v0.0.1"
```

4. point at private mirror and publish
```bash
git remote add origin https://github.com/Undermybelt/ict-engine-release.git
git tag -a v0.0.1 -m "v0.0.1"
git push origin main
git push origin v0.0.1
```

5. create private GitHub release
```bash
gh release create v0.0.1 \
  --repo Undermybelt/ict-engine-release \
  --title "v0.0.1" \
  --notes-file docs/audits/release-signoff.md
```

Rules
- do not push release tags from the source repo when history contains oversized research artifacts
- source repo remains the development / experiment truth — never push it to any public or shared remote
- mirror repo remains the sole release transport surface
- release notes should point back to source-repo docs where needed
- bump the version (`v0.0.1` → `v0.0.2` → …) every release; refresh `docs/audits/release-signoff.md` and `docs/release-notes-draft.md` before running the runbook

Post-release follow-up
- if mirror release flow becomes standard, automate it with a small script
- keep `docs/audits/release-signoff.md` current before every release
