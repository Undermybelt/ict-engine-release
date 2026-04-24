# Regime Core First Pass Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a first-pass regime core that computes Wasserstein-style regime discrimination, governor gating, and D1/H1 alignment on top of the existing Rust regime and belief pipeline without touching `src/pda_sequence/analysis.rs`.

**Architecture:** Extend the existing `domain/regime` packet layer instead of creating a parallel regime stack. Compute compact regime features from existing `FrameFeatures`, classify them into stable regime buckets with a lightweight Wasserstein distance surface, post-process them through a governor that enforces confidence and minimum-hold rules, then expose cross-timeframe alignment as typed packet fields plus evidence strings already consumed by the belief pipeline.

**Tech Stack:** Rust 2021, existing `serde`/`anyhow`/`chrono`, current `config::FrameFeatures`, `domain::regime::*`, `analyze::multi_timeframe_parse`, `application::belief::pipeline_builder`, `cargo test`

---

## File Structure

- Create: `src/domain/regime/wasserstein.rs`
- Create: `src/domain/regime/governor.rs`
- Create: `src/domain/regime/timeframe.rs`
- Create: `src/domain/regime/hybrid.rs`
- Modify: `src/domain/regime/mod.rs`
- Modify: `src/domain/regime/types.rs`
- Modify: `src/domain/regime/tests.rs`
- Modify: `src/application/belief/pipeline_builder.rs`
- Modify: `src/config.rs`
- Create: `tests/regime_core_first_pass.rs`

Responsibility split:

- `wasserstein.rs`: deterministic feature extraction + distance computation + nearest-prototype classification
- `governor.rs`: confidence gating, entropy check, minimum-hold suppression, transitional fallback
- `timeframe.rs`: simple D1/H1 direction alignment helpers
- `hybrid.rs`: orchestration layer that turns one or more `FrameFeatures` snapshots into `RegimeSegmentationPacket`
- `types.rs`: typed packet extensions only; no model logic
- `pipeline_builder.rs`: surface the new packet into the existing belief path as evidence only
- `config.rs`: reuse existing `FrameFeatures` outputs; only add helper adapters if needed

### Task 1: Lock Packet Schema Before Model Logic

**Files:**
- Modify: `src/domain/regime/types.rs`
- Modify: `src/domain/regime/tests.rs`

- [ ] **Step 1: Write the failing packet tests**

Add assertions that `RegimeSegmentationPacket` can round-trip new first-pass fields:

```rust
#[test]
fn regime_segmentation_packet_round_trip_with_hybrid_fields() {
    let mut packet = RegimeSegmentationPacket {
        method: "hybrid_regime_first_pass_v1".into(),
        segmentation_version: "v2".into(),
        active_regime_cluster: Some("range_calm".into()),
        transition_hazard: Some(0.18),
        regime_membership: std::collections::BTreeMap::new(),
        feature_attribution: std::collections::BTreeMap::new(),
        evidence: vec!["governor_commit=true".into()],
        wasserstein_label: Some("range_calm".into()),
        wasserstein_distance: Some(0.12),
        governor_confidence: Some(0.74),
        governor_entropy: Some(0.81),
        governor_min_hold_active: Some(false),
        timeframe_alignment: Some(true),
        timeframe_alignment_score: Some(1.0),
    };
    packet.regime_membership.insert("range_calm".into(), 0.74);
    packet.regime_membership.insert("trend_impulse".into(), 0.26);
    let json = serde_json::to_string(&packet).unwrap();
    let parsed: RegimeSegmentationPacket = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.wasserstein_label.as_deref(), Some("range_calm"));
    assert_eq!(parsed.timeframe_alignment, Some(true));
}
```

- [ ] **Step 2: Run packet tests to verify they fail**

Run: `cargo test regime_segmentation_packet_round_trip_with_hybrid_fields -- --nocapture`
Expected: FAIL because the new fields do not exist yet on `RegimeSegmentationPacket`.

- [ ] **Step 3: Extend the packet schema minimally**

Add nullable first-pass fields to `RegimeSegmentationPacket`:

```rust
pub struct RegimeSegmentationPacket {
    pub method: String,
    pub segmentation_version: String,
    pub active_regime_cluster: Option<String>,
    pub transition_hazard: Option<f64>,
    pub regime_membership: BTreeMap<String, f64>,
    pub feature_attribution: BTreeMap<String, f64>,
    pub evidence: Vec<String>,
    pub wasserstein_label: Option<String>,
    pub wasserstein_distance: Option<f64>,
    pub governor_confidence: Option<f64>,
    pub governor_entropy: Option<f64>,
    pub governor_min_hold_active: Option<bool>,
    pub timeframe_alignment: Option<bool>,
    pub timeframe_alignment_score: Option<f64>,
}
```

- [ ] **Step 4: Run packet tests to verify they pass**

Run: `cargo test regime_segmentation_packet_round_trip -- --nocapture`
Expected: PASS, including the original round-trip coverage and the new hybrid-field variant.

- [ ] **Step 5: Commit**

```bash
git add src/domain/regime/types.rs src/domain/regime/tests.rs
git commit -m "feat: extend regime segmentation packet for hybrid regime core"
```

### Task 2: Implement Deterministic Wasserstein Regime Classifier

**Files:**
- Create: `src/domain/regime/wasserstein.rs`
- Modify: `src/domain/regime/mod.rs`
- Test: `tests/regime_core_first_pass.rs`

- [ ] **Step 1: Write the failing classifier tests**

Add fixture-driven tests that separate calm range from impulsive trend using synthetic feature vectors:

```rust
#[test]
fn wasserstein_classifier_separates_range_from_trend() {
    let calm = vec![0.02, 0.03, 0.01, 0.02];
    let impulse = vec![0.65, 0.72, 0.61, 0.70];
    let classifier = WassersteinClassifier::default();
    let calm_result = classifier.classify(&calm).unwrap();
    let impulse_result = classifier.classify(&impulse).unwrap();
    assert_eq!(calm_result.label, "range_calm");
    assert_eq!(impulse_result.label, "trend_impulse");
    assert!(calm_result.membership["range_calm"] > calm_result.membership["trend_impulse"]);
    assert!(impulse_result.membership["trend_impulse"] > impulse_result.membership["range_calm"]);
}
```

- [ ] **Step 2: Run the classifier tests to verify they fail**

Run: `cargo test wasserstein_classifier_separates_range_from_trend -- --nocapture`
Expected: FAIL because `wasserstein.rs` and `WassersteinClassifier` do not exist.

- [ ] **Step 3: Add the minimal classifier implementation**

Implement a deterministic nearest-prototype classifier, not full offline k-medoids training:

```rust
pub struct WassersteinClassification {
    pub label: String,
    pub distance: f64,
    pub membership: BTreeMap<String, f64>,
}

pub struct WassersteinClassifier {
    prototypes: Vec<(&'static str, Vec<f64>)>,
}

pub fn wasserstein_1d(a: &[f64], b: &[f64]) -> Option<f64> {
    if a.len() != b.len() || a.is_empty() {
        return None;
    }
    let mut left = a.to_vec();
    let mut right = b.to_vec();
    left.sort_by(f64::total_cmp);
    right.sort_by(f64::total_cmp);
    Some(
        left.iter()
            .zip(right.iter())
            .map(|(x, y)| (x - y).abs())
            .sum::<f64>()
            / left.len() as f64,
    )
}
```

Use four fixed labels aligned to current repo vocabulary:

```rust
[
    ("trend_impulse", vec![0.85, 0.75, 0.70, 0.80]),
    ("trend_decay", vec![0.55, 0.35, 0.60, 0.45]),
    ("range_calm", vec![0.10, 0.15, 0.12, 0.08]),
    ("range_choppy", vec![0.30, 0.75, 0.25, 0.70]),
]
```

Convert distances into bounded membership weights with inverse-distance normalization.

- [ ] **Step 4: Export the new module**

Update `src/domain/regime/mod.rs`:

```rust
pub mod governor;
pub mod hybrid;
pub mod timeframe;
pub mod wasserstein;

pub use governor::*;
pub use hybrid::*;
pub use timeframe::*;
pub use wasserstein::*;
```

- [ ] **Step 5: Run the classifier tests to verify they pass**

Run: `cargo test wasserstein_ -- --nocapture`
Expected: PASS for 1D distance and prototype classification tests.

- [ ] **Step 6: Commit**

```bash
git add src/domain/regime/wasserstein.rs src/domain/regime/mod.rs tests/regime_core_first_pass.rs
git commit -m "feat: add first-pass wasserstein regime classifier"
```

### Task 3: Add Governor Gating With Minimum Hold

**Files:**
- Create: `src/domain/regime/governor.rs`
- Test: `tests/regime_core_first_pass.rs`

- [ ] **Step 1: Write the failing governor tests**

Add two tests: one for commit, one for suppression:

```rust
#[test]
fn governor_commits_when_confident_and_low_entropy() {
    let decision = RegimeGovernor::new(0.20, 2.0, 3)
        .decide("range_calm", &membership(0.74, 0.16, 0.06, 0.04), 4, false)
        .unwrap();
    assert!(decision.committed);
    assert_eq!(decision.selected_label, "range_calm");
}

#[test]
fn governor_holds_previous_label_when_min_hold_active() {
    let decision = RegimeGovernor::new(0.20, 2.0, 3)
        .decide_with_previous(
            "trend_impulse",
            &membership(0.31, 0.30, 0.20, 0.19),
            1,
            Some("range_calm"),
            1,
        )
        .unwrap();
    assert!(!decision.committed);
    assert_eq!(decision.selected_label, "range_calm");
}
```

- [ ] **Step 2: Run the governor tests to verify they fail**

Run: `cargo test governor_ -- --nocapture`
Expected: FAIL because `RegimeGovernor` does not exist.

- [ ] **Step 3: Implement entropy + minimum-hold gating**

Implement:

```rust
pub struct GovernorDecision {
    pub selected_label: String,
    pub confidence: f64,
    pub entropy: f64,
    pub committed: bool,
    pub min_hold_active: bool,
    pub evidence: Vec<String>,
}

pub struct RegimeGovernor {
    confidence_floor: f64,
    entropy_ceiling: f64,
    min_hold_bars: usize,
}
```

Rules:

- `confidence = max(membership)`
- `entropy = -sum(p * ln p)` over positive probabilities
- commit only when `confidence >= confidence_floor && entropy <= entropy_ceiling`
- if previous label exists and `bars_since_last_switch < min_hold_bars`, keep the previous label

- [ ] **Step 4: Run the governor tests to verify they pass**

Run: `cargo test governor_ -- --nocapture`
Expected: PASS for both commit and suppression cases.

- [ ] **Step 5: Commit**

```bash
git add src/domain/regime/governor.rs tests/regime_core_first_pass.rs
git commit -m "feat: add regime governor gating"
```

### Task 4: Add D1/H1 Alignment Helpers

**Files:**
- Create: `src/domain/regime/timeframe.rs`
- Test: `tests/regime_core_first_pass.rs`

- [ ] **Step 1: Write the failing alignment tests**

```rust
#[test]
fn timeframe_alignment_is_true_for_matching_directional_labels() {
    let alignment = timeframe_alignment("trend_impulse", "trend_decay");
    assert!(alignment.aligned);
    assert_eq!(alignment.score, 1.0);
}

#[test]
fn timeframe_alignment_is_false_for_trend_vs_range() {
    let alignment = timeframe_alignment("trend_impulse", "range_calm");
    assert!(!alignment.aligned);
    assert_eq!(alignment.score, 0.0);
}
```

- [ ] **Step 2: Run the alignment tests to verify they fail**

Run: `cargo test timeframe_alignment_ -- --nocapture`
Expected: FAIL because `timeframe.rs` does not exist.

- [ ] **Step 3: Implement the simplest acceptable alignment rule**

Implement:

```rust
pub struct TimeframeAlignment {
    pub aligned: bool,
    pub score: f64,
    pub evidence: Vec<String>,
}

pub fn regime_direction(label: &str) -> &'static str {
    match label {
        "trend_impulse" | "trend_decay" => "trend",
        "range_calm" | "range_choppy" => "range",
        _ => "unknown",
    }
}

pub fn timeframe_alignment(higher: &str, lower: &str) -> TimeframeAlignment {
    let aligned = regime_direction(higher) == regime_direction(lower)
        && regime_direction(higher) != "unknown";
    TimeframeAlignment {
        aligned,
        score: if aligned { 1.0 } else { 0.0 },
        evidence: vec![
            format!("higher={higher}"),
            format!("lower={lower}"),
        ],
    }
}
```

- [ ] **Step 4: Run the alignment tests to verify they pass**

Run: `cargo test timeframe_alignment_ -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/domain/regime/timeframe.rs tests/regime_core_first_pass.rs
git commit -m "feat: add simple regime timeframe alignment"
```

### Task 5: Build Hybrid Regime Packet From Existing Frame Features

**Files:**
- Create: `src/domain/regime/hybrid.rs`
- Modify: `src/config.rs`
- Test: `tests/regime_core_first_pass.rs`

- [ ] **Step 1: Write the failing hybrid packet tests**

```rust
#[test]
fn hybrid_regime_packet_contains_wasserstein_governor_and_alignment_fields() {
    let higher = sample_frame("bull", "neutral", 1, 3, 250.0, 120.0, 45.0);
    let lower = sample_frame("bull", "neutral", 2, 2, 200.0, 80.0, 30.0);
    let packet = build_hybrid_regime_packet(Some(&higher), &lower, None).unwrap();
    assert!(packet.wasserstein_label.is_some());
    assert!(packet.governor_confidence.is_some());
    assert_eq!(packet.timeframe_alignment, Some(true));
    assert!(packet.evidence.iter().any(|line| line.starts_with("wasserstein_label=")));
}
```

- [ ] **Step 2: Run the hybrid tests to verify they fail**

Run: `cargo test hybrid_regime_packet_ -- --nocapture`
Expected: FAIL because the orchestration function does not exist.

- [ ] **Step 3: Add a feature adapter from `FrameFeatures` to regime vectors**

If the helper does not already fit cleanly in `config.rs`, add:

```rust
pub fn regime_feature_vector(frame: &FrameFeatures) -> Vec<f64> {
    vec![
        ((frame.normalized_distance_to_projected_trend_bps.abs() / 10_000.0).clamp(0.0, 1.0)),
        ((frame.ou_pullback_expectation_zscore.abs() / 5.0).clamp(0.0, 1.0)),
        ((frame.fvg_count as f64 / (frame.fvg_count.max(1) as f64 + frame.sweep_count as f64))
            .clamp(0.0, 1.0)),
        ((frame.sweep_count as f64 / (frame.fvg_count as f64 + frame.sweep_count.max(1) as f64))
            .clamp(0.0, 1.0)),
    ]
}
```

- [ ] **Step 4: Implement the hybrid packet builder**

Create a pure function:

```rust
pub fn build_hybrid_regime_packet(
    higher_timeframe: Option<&FrameFeatures>,
    current: &FrameFeatures,
    previous_label: Option<&str>,
) -> anyhow::Result<RegimeSegmentationPacket>
```

Behavior:

- classify `current` with `WassersteinClassifier`
- run `RegimeGovernor`
- if `higher_timeframe` exists, compute `timeframe_alignment`
- fill `active_regime_cluster` from governor-selected label
- fill `regime_membership` with classifier membership
- fill `feature_attribution` with the four compact feature names
- add evidence strings like `wasserstein_label=...`, `governor_entropy=...`, `timeframe_alignment=true`

- [ ] **Step 5: Run the hybrid tests to verify they pass**

Run: `cargo test hybrid_regime_packet_ -- --nocapture`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/domain/regime/hybrid.rs src/config.rs tests/regime_core_first_pass.rs
git commit -m "feat: build first-pass hybrid regime packet"
```

### Task 6: Surface Regime Packet Into Belief Pipeline As Evidence Only

**Files:**
- Modify: `src/application/belief/pipeline_builder.rs`
- Test: `tests/regime_core_first_pass.rs`

- [ ] **Step 1: Write the failing integration test**

Add a targeted test that ensures the pipeline emits regime-core evidence without changing trading direction logic:

```rust
#[test]
fn pipeline_builder_surfaces_hybrid_regime_evidence() {
    let report = build_expansion_factor_pipeline_report(
        "NQ",
        "price_action",
        &sample_candles(64, 100.0, 0.4),
        &[
            "higher_timeframe_direction_bias=bullish".to_string(),
            "higher_timeframe_alignment_score=1.0".to_string(),
            "lower_timeframe_entry_alignment_score=1.0".to_string(),
        ],
    )
    .unwrap();
    let evidence = &report.bbn_support.raw_market_regime_trace.evidence;
    assert!(evidence.iter().any(|line| line.starts_with("hybrid_regime_label=")));
    assert!(evidence.iter().any(|line| line.starts_with("hybrid_timeframe_alignment=")));
}
```

- [ ] **Step 2: Run the integration test to verify it fails**

Run: `cargo test pipeline_builder_surfaces_hybrid_regime_evidence -- --nocapture`
Expected: FAIL because the pipeline does not yet add the hybrid evidence lines.

- [ ] **Step 3: Integrate packet generation conservatively**

In `build_expansion_factor_pipeline_report_with_registry`:

- build `FrameFeatures` as today
- call `build_hybrid_regime_packet(None, &frame, None)` for the current frame
- append evidence strings from the packet to `market_regime_trace.evidence`
- derive `hybrid_timeframe_alignment` from existing parsed multi-timeframe summary when no higher-frame packet is available
- do not replace `frame.regime_label`, `liquidity_label`, or directional selection in this pass

Target evidence:

```rust
market_regime_trace.evidence.push(format!(
    "hybrid_regime_label={}",
    packet.active_regime_cluster.as_deref().unwrap_or("unknown")
));
market_regime_trace.evidence.push(format!(
    "hybrid_timeframe_alignment={}",
    packet.timeframe_alignment.unwrap_or(
        multi_timeframe_evidence.alignment_score.unwrap_or_default() >= 0.5
    )
));
```

- [ ] **Step 4: Run the integration test to verify it passes**

Run: `cargo test pipeline_builder_surfaces_hybrid_regime_evidence -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Run the broader regime and belief tests**

Run: `cargo test regime_core_first_pass -- --nocapture`
Expected: PASS for the new focused test file.

- [ ] **Step 6: Commit**

```bash
git add src/application/belief/pipeline_builder.rs tests/regime_core_first_pass.rs
git commit -m "feat: surface hybrid regime evidence in belief pipeline"
```

### Task 7: Full Verification

**Files:**
- Test: `src/domain/regime/tests.rs`
- Test: `tests/regime_core_first_pass.rs`

- [ ] **Step 1: Run targeted unit and integration tests**

Run: `cargo test regime_segmentation_packet_round_trip -- --nocapture`
Expected: PASS

Run: `cargo test wasserstein_ -- --nocapture`
Expected: PASS

Run: `cargo test governor_ -- --nocapture`
Expected: PASS

Run: `cargo test timeframe_alignment_ -- --nocapture`
Expected: PASS

Run: `cargo test hybrid_regime_packet_ -- --nocapture`
Expected: PASS

Run: `cargo test pipeline_builder_surfaces_hybrid_regime_evidence -- --nocapture`
Expected: PASS

- [ ] **Step 2: Run the broader suite**

Run: `cargo test`
Expected: PASS with no edits to `src/pda_sequence/analysis.rs`

- [ ] **Step 3: Final commit**

```bash
git add src/domain/regime/mod.rs \
  src/domain/regime/types.rs \
  src/domain/regime/tests.rs \
  src/domain/regime/wasserstein.rs \
  src/domain/regime/governor.rs \
  src/domain/regime/timeframe.rs \
  src/domain/regime/hybrid.rs \
  src/application/belief/pipeline_builder.rs \
  src/config.rs \
  tests/regime_core_first_pass.rs
git commit -m "feat: add first-pass hybrid regime core"
```

## Constraints Carried Forward

- Do not edit `src/pda_sequence/analysis.rs`
- Do not replace the existing `frame.regime_label` heuristic in this pass
- Do not wire regime outputs directly into long/short execution logic in this pass
- Do not introduce offline-trained medoids, Python bridges, or persistence formats in this pass
- Keep all new regime outputs evidence-only unless a later plan explicitly upgrades them to gating inputs

## Phase-2 Follow-Ups Explicitly Deferred

- HSMM explicit duration modeling
- offline Wasserstein prototype training or k-medoids refresh
- D1/H1/H4 multi-frame packet persistence
- PreBayes / BBN consumption of regime posterior as soft evidence
- PDA cluster fusion with hybrid regime packet
