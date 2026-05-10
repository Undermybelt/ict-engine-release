# Post-main Debt Inventory

Date: 2026-04-24

Purpose
- record the remaining non-`main.rs` debt after the runtime hotspot extraction line landed
- separate debts that are fixable in-tree from debts that require history rewrite or release-process decisions

Current baseline
- `src/main.rs` hotspot extraction is now materially closed by:
  - `8ce1024 Move runtime hotspots out of main`
  - `3e45254 Close main runtime extraction hotspot`
- current `src/main.rs` line count: `14236`

## Remaining debt

### 1. Archived backend portability debt

Status
- largely closed on 2026-04-24
- keep as regression-watch debt, not an active hotspot

Evidence
- `docs/backend-path-audit.md`
- `docs/release-notes-draft.md`

What changed
- active `scripts/` path discovery now routes through `scripts/path_defaults.py`
- named archived backends no longer embed machine-local `/Users/...` repo/data/bin paths
- policy-training helper scripts no longer hard-code repo-local absolute paths

Residual caveat
- public wrappers no longer silently assume a Tomac-style cleaned-data layout
- wrappers expose `--show-config` and require explicit cleaned-data readiness before `--run`
- keep this as a regression-watch rule for future wrapper work

One-shot feasibility
- already landed in-tree
- future work here is regression prevention, not a remaining one-shot target

### 2. Release transport/history debt

Status
- closed on 2026-04-24
- no longer an active repo blocker

Evidence
- `docs/audits/release-signoff.md`
- `docs/release-mirror-runbook.md`

What changed
- generated `state*` artifacts were removed from git history
- source-repo pushes are now available again
- the historical >100 MB blocker is no longer present in the rewritten history

Residual caveat
- mirror release is still useful as a clean tree-state publication flow
- but it is no longer required by an unresolved source-repo history debt

### 3. Analytics depth debt in release-facing research surfaces

Status
- materially reduced on 2026-04-24
- no longer the primary debt hotspot

Evidence
- `docs/audits/release-signoff.md`
- `docs/audits/pre-release-audit.md`

Surfaces
- `research-verdict`
- contamination heuristics
- `evidence-quality-breakdown`

What changed
- `research-verdict` now emits richer contamination evidence and isolated-comparison recommendation
- contamination signal now considers mixed data paths, paired data paths, mutation sources, and artifact source phases
- `evidence-quality-breakdown` now surfaces policy version, soft-evidence usage, bridge-selected quality, MTF raw/filtered scores, and soft-evidence divergence summary

Residual caveat
- the surfaces are still compact by design rather than a full experiment dashboard
- that is a product-scope choice more than an immediate debt blocker

### 4. Historical docs still describing stale debt state

Status
- real
- documentation debt
- closable immediately

Evidence
- `docs/audits/release-signoff.md`
- `docs/audits/pre-release-audit.md`
- older closeout docs now partially stale without status notes

What is still wrong
- some historical audit surfaces still mention `src/main.rs` as the active structural hotspot even though that line has now been materially reduced

One-shot feasibility
- yes
- low risk

## Best next one-shot target

If the goal is "clear the remaining fixable debt in one branch", the best target is:

1. stale docs that still describe old debt state
2. optional product-surface expansion beyond the current compact release-closure design

Do not mix in release-history rewrite unless the branch is explicitly approved as a history surgery branch.

## Honest end-state claim

After the `main.rs` extraction line, the remaining debt picture is:

- fixable in-tree:
  - archived backend portability
  - stale debt docs
- remaining meaningful debt is mostly documentation and product-scope tradeoff, not repo-structure or transport blockage
