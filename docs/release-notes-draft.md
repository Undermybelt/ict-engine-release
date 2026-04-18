# Release Notes Draft

Version: preview release candidate
Status: draft

## Highlights

- Added safer public Python experiment entrypoints:
  - `scripts/search_local.py`
  - `scripts/search_cluster.py`
  - `scripts/evaluate_bottleneck.py`
- Public wrappers now default to help-only mode instead of launching long runs.
- Added non-executing backend summaries via `--backend-help`.
- Added release-facing onboarding docs:
  - `README.md` rewritten as a publishable entry surface
  - `docs/first-run.md`
- Added explicit release caveat for archived backend portability:
  - `docs/backend-path-audit.md`

## UX improvements

- Users can now inspect public script families safely before execution.
- `--target` now reports backend paths clearly.
- Wrapper help now explains when each script should be used.
- Documentation now routes common intents directly to the right CLI/script surface.

## New recommended first-run flow

```bash
cargo check
cargo run -- --help
python3 scripts/search_local.py
python3 scripts/search_cluster.py
python3 scripts/evaluate_bottleneck.py
```

## Important caveats

- Public wrappers are safer than the archived backends they call.
- Archived backends still contain machine-local hard-coded paths and are not yet fully portable across machines.
- Treat this release as an agent-first / researcher-preview surface, not a fully generalized packaged distribution.

## Best commands for current users

- market read:
  - `cargo run -- analyze --help`
- gate / bridge diagnosis:
  - `cargo run -- factor-pipeline-debug --help`
- latest autoresearch truth:
  - `cargo run -- factor-autoresearch-status --help`
- local search wrapper:
  - `python3 scripts/search_local.py`
- cluster jump wrapper:
  - `python3 scripts/search_cluster.py`
- bottleneck wrapper:
  - `python3 scripts/evaluate_bottleneck.py`
- verdict synthesis:
  - `python3 scripts/research_verdict.py <state-or-result-dir>`

## Known limitations

- archived backend portability not finished
- public wrappers summarize backend behavior, but archived backends do not yet expose a stable full public argparse surface
- some experiments still assume local cleaned-data layouts and repo-root-relative state conventions

## Upgrade / migration notes

If you previously called archived scripts directly, prefer the new public wrappers first.

Old habit:
- `python3 scripts/archive/factor_local_search_v2d.py`

New habit:
- `python3 scripts/search_local.py`
- `python3 scripts/search_local.py --backend-help`
- `python3 scripts/search_local.py --run`

## Suggested release label

`ict-engine v0.1.0-preview`

Reason:
- core CLI is real and usable
- release-facing docs are now present
- wrapper UX is much safer
- backend portability cleanup is still pending
