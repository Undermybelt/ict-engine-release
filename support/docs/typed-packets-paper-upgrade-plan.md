# Typed packets paper upgrade plan

> For Hermes: use subagent-driven-development skill to implement this plan task-by-task.

Goal: 为 ict-engine 先落一层“论文吸收中间层”：文档先行、typed packets 先行、artifact 先行；暂不直接上训练流与重模型。

Architecture: 先把 Jump Model / structural break / conformal / microstructure 相关机制收束为稳定 typed packets，挂到 domain + state + application adapter，避免继续把实验性逻辑塞进 `src/main.rs`。第一阶段只做 packet、artifact、workflow/reporting 挂点与测试；第二阶段再把各机制逐步接入 regime classifier、uncertainty gate、factor research、regime-split backtest。

Tech Stack: Rust, serde, existing `domain/*`, `state/*`, `application/*`, `factor_lab/*`, `bbn/*`, `hmm/*`, cargo test.

---

## Context and boundaries

Must follow:
- `support/docs/architecture-boundaries.md`
- `support/docs/change-surface.md`

Boundary rules already present in repo:
- new typed semantics belong in `domain/*`
- new persisted audit artifacts belong in `state/types.rs` + `state/persistence.rs`
- reflection/reporting extension goes through `application/*`
- avoid widening `src/main.rs`

Paper-driven design stance for this phase:
- PDA / footprint not direct evidence
- microstructure / LOB / footprint only enters as:
  - `prior adjuster`
  - `state transition`
  - `setup classifier`
  - or `uncertainty / abstention gate`
- conformal outputs are not directional evidence; they are calibration / abstention / validator surfaces
- structural break outputs are not hindsight labels; they are gating / segmentation / backtest-validation surfaces

## Target mechanisms to represent as packets first

1. Jump-model / regime-segmentation packet
2. Structural-break packet
3. Conformal uncertainty packet
4. Microstructure context packet
5. Market-specific policy packet
6. Regime validation / backtest validation packet

## Change surface summary

Create or extend only these layers in phase 1:
- domain
- state
- application adapters / reflection / reporting glue
- tests
- docs

Do not in this phase:
- add external ML dependencies
- add full DeepLOB training flow
- add live LOB ingestion infra
- rewrite current HMM / BBN core inference path

---

## Task 1: Add design doc for paper-to-packet mapping

Objective: 把 8 篇论文可吸收机制压成 repo 内正式设计文档，作为后续 packet/schema 的唯一说明面。

Files:
- Create: `support/docs/paper-driven-typed-packets-design.md`
- Modify: `support/docs/change-surface.md`

Step 1: Write the design doc with these sections
- scope and exclusions
- packet families
- mapping table:
  - paper mechanism
  - ict-engine role (`prior adjuster`, `state transition`, `setup classifier`, `evidence`, `outcome validator`, `backtest validator`, `uncertainty gate`, `market-specific policy`, `feature selection / factor research`)
  - target module / packet name
  - phase (`packet-only`, `adapter`, `model`, `backtest`)
- anti-misuse rules
- rollout order

The doc must explicitly name these target packet families:
- `RegimeSegmentationPacket`
- `StructuralBreakPacket`
- `ConformalUncertaintyPacket`
- `MicrostructureContextPacket`
- `MarketPolicyPacket`
- `RegimeValidationPacket`

Step 2: Update `support/docs/change-surface.md`
Add a new section:
- `typed-packet-first paper upgrade` 
- list editable paths for phase 1
- state that runtime inference remains unchanged in this phase

Step 3: Verify doc presence
Run:
- `test -f support/docs/paper-driven-typed-packets-design.md && echo OK`
Expected: `OK`

Step 4: Commit
```bash
git add support/docs/paper-driven-typed-packets-design.md support/docs/change-surface.md
git commit -m "docs: add paper-driven typed packet design"
```

---

## Task 2: Define new regime packet family in domain layer

Objective: 把 jump model / structural break / regime validation 所需 typed semantics 先落到 `domain/regime`。

Files:
- Modify: `src/domain/regime/types.rs`
- Modify: `src/domain/regime/mod.rs`
- Test: `src/domain/regime/tests.rs` or nearest existing regime tests file

Step 1: Add new structs to `src/domain/regime/types.rs`
Add minimal serde-ready packets:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegimeSegmentationPacket {
    pub method: String,
    pub segmentation_version: String,
    pub active_regime_cluster: Option<String>,
    pub transition_hazard: Option<f64>,
    pub regime_membership: BTreeMap<String, f64>,
    pub feature_attribution: BTreeMap<String, f64>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralBreakPacket {
    pub method: String,
    pub break_family: String,
    pub detected: bool,
    pub break_score: Option<f64>,
    pub break_index: Option<usize>,
    pub lookback_window: Option<usize>,
    pub affected_features: Vec<String>,
    pub rationale: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegimeValidationPacket {
    pub validation_scope: String,
    pub segmentation_consistency: Option<f64>,
    pub hindsight_risk_flags: Vec<String>,
    pub abstain_recommended: bool,
    pub notes: Vec<String>,
}
```

Step 2: Extend existing packets conservatively
Add optional fields only:
- `RegimeFeatures`
  - `segmentation_context: Option<RegimeSegmentationPacket>`
  - `structural_break_context: Option<StructuralBreakPacket>`
- `RegimePosterior`
  - `regime_validation: Option<RegimeValidationPacket>`

Step 3: Export from `src/domain/regime/mod.rs`
Re-export new packet types.

Step 4: Write serialization test
Test round-trip serde for all three packets and ensure existing defaults still work.

Step 5: Run focused test
Run:
- `cargo test regime --quiet`
Expected: pass

Step 6: Commit
```bash
git add src/domain/regime/types.rs src/domain/regime/mod.rs
git commit -m "feat: add regime segmentation and break packets"
```

---

## Task 3: Define conformal + microstructure packet family in belief/domain layer

Objective: 把 conformal uncertainty 与 microstructure context 先落为 typed packets，且明示其非 direct evidence 身份。

Files:
- Modify: `src/domain/belief/types.rs`
- Modify: `src/domain/belief/mod.rs`
- Test: nearest belief/domain test file

Step 1: Add new structs to `src/domain/belief/types.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConformalUncertaintyPacket {
    pub method: String,
    pub target: String,
    pub nominal_coverage: f64,
    pub empirical_coverage: Option<f64>,
    pub interval_width: Option<f64>,
    pub nonconformity_score: Option<f64>,
    pub abstain_threshold: Option<f64>,
    pub abstain: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MicrostructureContextPacket {
    pub source: String,
    pub granularity: String,
    pub usable_as_evidence: bool,
    pub prior_adjuster_bias: Option<f64>,
    pub transition_bias: Option<f64>,
    pub setup_quality_score: Option<f64>,
    pub context_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketPolicyPacket {
    pub market_family: Option<String>,
    pub market_behavior_profile: Option<String>,
    pub policy_mode: String,
    pub evidence_reliability: BTreeMap<String, f64>,
    pub abstention_bias: Option<f64>,
    pub notes: Vec<String>,
}
```

Step 2: Extend existing belief packets with optional fields only
- `BeliefEvidencePacket`
  - `microstructure_context: Option<MicrostructureContextPacket>`
  - `market_policy: Option<MarketPolicyPacket>`
- `BeliefReportPacket`
  - `conformal_uncertainty: Vec<ConformalUncertaintyPacket>`
  - `market_policy: Option<MarketPolicyPacket>`

Step 3: Add default-safe serde tests
Test:
- new packets serialize / deserialize
- existing `BeliefReportPacket::default()` still works
- `usable_as_evidence` defaults false unless explicitly set in tests

Step 4: Run focused test
Run:
- `cargo test belief --quiet`
Expected: pass

Step 5: Commit
```bash
git add src/domain/belief/types.rs src/domain/belief/mod.rs
git commit -m "feat: add conformal and microstructure belief packets"
```

---

## Task 4: Add persisted audit artifact types for packet-first phase

Objective: 把新机制的落盘入口先建好，后续 adapter/model 只需填充，不需临时 JSON 逃逸。

Files:
- Modify: `src/state/types.rs`
- Modify: `src/state/persistence.rs`
- Modify: `src/state/mod.rs`
- Test: state persistence tests or nearest file

Step 1: Add file constants
Add persistence constants such as:
- `REGIME_SEGMENTATION_FILE`
- `STRUCTURAL_BREAK_FILE`
- `CONFORMAL_UNCERTAINTY_FILE`
- `MARKET_POLICY_FILE`

Step 2: Add persisted record wrappers
Example minimal wrappers:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegimeSegmentationRecord {
    pub artifact_id: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub symbol: String,
    pub source_phase: String,
    pub packet: crate::domain::regime::RegimeSegmentationPacket,
}
```

Repeat for:
- `StructuralBreakRecord`
- `ConformalUncertaintyRecord`
- `MarketPolicyRecord`

Step 3: Add persistence helpers
In `src/state/persistence.rs`, add save/load helpers for each record type.

Step 4: Re-export from `src/state/mod.rs`
Expose constants and helpers.

Step 5: Write persistence round-trip tests
Write tests using temp dir:
- save record
- load record
- assert equality

Step 6: Run focused test
Run:
- `cargo test state --quiet`
Expected: pass

Step 7: Commit
```bash
git add src/state/types.rs src/state/persistence.rs src/state/mod.rs
git commit -m "feat: add persisted records for paper-driven packets"
```

---

## Task 5: Add application adapter builders for placeholder packet emission

Objective: 在不改核心模型的前提下，先有 application builder 能从现有 report/context 生成空或轻量 packet，供 surfaces 使用。

Files:
- Create: `src/application/belief/paper_packets.rs`
- Modify: `src/application/belief/mod.rs`
- Test: `src/application/belief/tests.rs` or nearest test file

Step 1: Create builders with conservative output
Expose functions:
- `build_regime_segmentation_packet(...) -> RegimeSegmentationPacket`
- `build_structural_break_packet(...) -> StructuralBreakPacket`
- `build_conformal_uncertainty_packets(...) -> Vec<ConformalUncertaintyPacket>`
- `build_microstructure_context_packet(...) -> Option<MicrostructureContextPacket>`
- `build_market_policy_packet(...) -> Option<MarketPolicyPacket>`
- `build_regime_validation_packet(...) -> Option<RegimeValidationPacket>`

Rules for this phase:
- no speculative math
- if data absent, emit explicit method names like `placeholder:none` or `rule-based:phase1`
- microstructure packet must set `usable_as_evidence = false`

Step 2: Re-export in module root
Modify `src/application/belief/mod.rs`.

Step 3: Add tests
Verify:
- builders are deterministic
- placeholder outputs preserve constraints
- microstructure packet never marks direct evidence

Step 4: Run focused test
Run:
- `cargo test application::belief --quiet`
Expected: pass

Step 5: Commit
```bash
git add src/application/belief/paper_packets.rs src/application/belief/mod.rs
git commit -m "feat: add phase1 paper packet builders"
```

---

## Task 6: Attach packet summaries to belief/reporting surfaces without changing core decisions

Objective: 把新 packet 作为附属 surface 暴露给现有 reporting/reflection/workflow，而不改变推荐命令与核心 gate。

Files:
- Modify: `src/application/reflection/mod.rs` or nearest reflection file
- Modify: `src/reporting/belief/packet.rs` if needed for re-exports
- Modify: minimal application/report adapter files only
- Test: existing report serialization tests or add one

Step 1: Extend reflection/report output models only where typed attachment is clean
Preferred pattern:
- attach packet summaries as optional fields in reflection/report structures
- do not rewrite decision path

Step 2: Add summary helpers
For each packet family, add compact summary lines suitable for human/reporting surfaces.

Step 3: Add tests
Verify that:
- report serialization includes new optional fields when present
- old outputs remain valid when fields absent

Step 4: Run focused tests
Run:
- `cargo test reporting --quiet`
- `cargo test reflection --quiet`
Expected: pass

Step 5: Commit
```bash
git add src/application/reflection src/reporting/belief
git commit -m "feat: expose paper packet summaries in reporting surfaces"
```

---

## Task 7: Add artifact-ledger integration for new packet records

Objective: 让新 packet records 进入 artifact ledger / workflow snapshot 的可审计表面。

Files:
- Modify: `src/state/types.rs`
- Modify: `src/main.rs` only at narrow adapter call sites if absolutely required
- Prefer modify: helper functions near existing artifact persistence integration
- Test: artifact ledger tests / workflow snapshot tests

Step 1: Add ledger entry mapping helpers
For each new record family, generate `ArtifactLedgerEntry` with:
- artifact_kind
- artifact_id
- source_phase
- review_rule_version or packet method/version
- actionable=false by default in phase 1

Step 2: Attach to workflow snapshot only as optional latest packet refs or summaries
Do not widen snapshot with giant embedded structs unless repo already follows that pattern.
Preferred:
- compact summary lines
- artifact ids
- status flags

Step 3: Add tests
Verify:
- ledger append works
- workflow snapshot remains backward compatible

Step 4: Run focused test
Run:
- `cargo test workflow --quiet`
- `cargo test artifact --quiet`
Expected: pass

Step 5: Commit
```bash
git add src/state/types.rs src/main.rs
git commit -m "feat: add artifact audit hooks for paper packets"
```

---

## Task 8: Add factor-role routing notes and enforcement tests

Objective: 把“哪些只能作 gate / validator / prior adjuster，不能作 evidence”写成机械约束并测试。

Files:
- Modify: `src/factor_lab/factor_definition.rs`
- Create or modify nearest factor tests
- Modify: `support/docs/paper-driven-typed-packets-design.md`

Step 1: Add explicit comments / helper guards
Where appropriate, add helper predicates or utility functions such as:
- `is_direct_evidence_allowed_for_source(...)`
- `is_uncertainty_only_source(...)`

Minimum rule set:
- conformal packet: never direct evidence
- structural break packet: validator/gate, not direction evidence
- microstructure context packet: prior/transition/setup only by default

Step 2: Add tests
Test mappings from packet/source family to `FactorRole` / `FactorUsagePhase`.

Step 3: Run focused tests
Run:
- `cargo test factor --quiet`
Expected: pass

Step 4: Commit
```bash
git add src/factor_lab/factor_definition.rs support/docs/paper-driven-typed-packets-design.md
git commit -m "test: enforce packet role routing constraints"
```

---

## Task 9: Full verification pass

Objective: 确认 phase 1 只加 typed packets / artifacts / surfaces，不改核心行为，不破边界。

Files:
- No new files required

Step 1: Run format
Run:
- `cargo fmt --all --check`
Expected: clean

Step 2: Run tests
Run:
- `cargo test --quiet`
Expected: all pass

Step 3: Optional grep verification
Run:
- `rg -n "placeholder:none|rule-based:phase1|usable_as_evidence" src`
Expected:
  - phase1 builders present
  - microstructure evidence guard present

Step 4: Manual boundary review
Check:
- no large new logic block in `src/main.rs`
- new packet structs live in `domain/*`
- new persisted record structs live in `state/*`
- reporting changes are adapter-level

Step 5: Commit
```bash
git add -A
git commit -m "feat: complete typed packet phase for paper-driven upgrades"
```

---

## Phase 2 after this plan

Only after phase 1 lands, choose one implementation branch:
1. Jump Model PoC
   - fill `RegimeSegmentationPacket`
   - attach to `RegimePosterior`
   - compare against current HMM segmentation
2. Structural break validator
   - fill `StructuralBreakPacket`
   - gate `regime-split` backtest validity
3. Conformal abstention gate
   - fill `ConformalUncertaintyPacket`
   - hook to `RegimeGateDecision` / workflow recommendation abstention
4. Microstructure setup classifier
   - fill `MicrostructureContextPacket`
   - keep out of direct evidence path

## Acceptance criteria

Phase 1 is done only if:
- typed packet families exist in `domain/*`
- persisted record wrappers exist in `state/*`
- application builders exist and are deterministic
- reporting/reflection can surface packet summaries
- artifact ledger can audit them
- role-routing constraints prevent misuse
- `cargo test --quiet` passes
- no speculative ML dependency is added

## Notes for implementer

Use existing naming wherever possible:
- regime packet family under `src/domain/regime/*`
- belief/uncertainty packet family under `src/domain/belief/*`
- persisted wrappers under `src/state/types.rs`
- adapter builders under `src/application/belief/*`
- do not hide schema in ad hoc JSON
- do not promote placeholder packets into trading evidence
