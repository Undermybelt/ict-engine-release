# ABC Next Steps Plan

> Goal: execute post-10-run autoresearch follow-up in order: A) failure-driven jump-cluster routing, B) NQ market-specific fork surface, C) new ICT mutation family expansion.

## Current evidence
- `factor-autoresearch` 10-run batch finished with 0 keep / 10 discard.
- Stable failure tags:
  - `best_factor_composite_regressed`
  - `no_superior_mutation_found`
- Latest next-spec stayed in the same narrow `structure_ict` family and did not discover a new direction.

## A. Jump-cluster routing in next mutation generation

### Objective
When repeated failures are `best_factor_composite_regressed + no_superior_mutation_found`, stop narrow same-family parameter drift and force a cluster jump.

### Files
- Modify: `src/main.rs`
- Test: `src/main.rs` existing `#[cfg(test)] mod tests`

### Plan
1. Add helper for stagnation / jump routing decision near mutation-template helpers.
2. Detect failure pattern:
   - `best_factor_composite_regressed`
   - `no_superior_mutation_found`
3. For this pattern, override `next_mutation_spec_template_with_preferences(...)` result with a forced cross-cluster template.
4. Keep v0 simple: still return `FactorMutationSpec`, but hypothesis must explicitly say it is a forced cluster jump.
5. Add tests proving the new template is not an empty-hint same-family no-op.

### Verification
- `cargo test factor_autoresearch -- --nocapture`
- `cargo check`

## B. NQ market-specific fork surface

### Objective
For NQ, stop treating market-specific fork only as hidden label machinery; expose a concrete mutation / routing surface the autoresearch loop can choose.

### Files
- Modify: `src/main.rs`
- Maybe read/patch: market-specific helper paths already wired in repo

### Plan
1. Reuse existing NQ market-specific semantics already present in repo.
2. Add a forced-template branch for NQ when A triggers:
   - hypothesis references `market-specific fork validation`
   - parameter/metadata surface must preserve `structure_ict` baseline but redirect next cycle away from blind global tuning
3. Keep this phase minimal: use mutation-template/output surface first, not a full new runtime command.
4. Add test asserting NQ forced jump mentions market-specific fork in hypothesis / next directions.

### Verification
- targeted unit test
- `cargo check`

## C. New ICT mutation family expansion

### Objective
After A and B, allow next mutation generation to jump from pure `structure_ict` narrow tuning into other ICT families.

### Candidate clusters
1. displacement / FVG
2. MSS / BOS
3. premium-discount / OTE
4. SMT

### Files
- Modify: `src/main.rs`
- Possibly future file touch: factor definition surfaces if cluster metadata should become first-class later

### Plan
1. Implement minimal cluster selector in next-spec generation.
2. Keep v1 light: encode cluster jump in hypothesis + direction hints + a few parameter overrides when available.
3. Do not try to create brand-new factor implementations yet.
4. Add tests that repeated stagnation can emit a non-`structure_ict`-narrow hypothesis path.

### Verification
- unit tests
- one small autoresearch smoke with 2-3 iterations

## Order of execution
1. A
2. B
3. C

## Constraints
- Small patches only
- `cargo check` after each stage
- No giant refactor while repo has unrelated dirty files
