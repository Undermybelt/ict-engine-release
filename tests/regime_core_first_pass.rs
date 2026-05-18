use chrono::{Duration, TimeZone, Utc};
use ict_engine::application::belief::build_expansion_factor_pipeline_report;
use ict_engine::config::FrameFeatures;
use ict_engine::domain::regime::{
    build_hybrid_regime_packet, estimate_duration_state, negative_binomial_duration,
    timeframe_alignment, RegimeGovernor, WassersteinClassifier,
};
use ict_engine::pda_sequence::PdaSequenceArtifactSummary;
use ict_engine::types::Candle;
use std::collections::BTreeMap;

fn membership(a: f64, b: f64, c: f64, d: f64) -> BTreeMap<String, f64> {
    BTreeMap::from([
        ("range_calm".to_string(), a),
        ("trend_impulse".to_string(), b),
        ("trend_decay".to_string(), c),
        ("range_choppy".to_string(), d),
    ])
}

fn sample_frame(
    regime_label: &str,
    liquidity_label: &str,
    sweep_count: usize,
    fvg_count: usize,
    projected_distance_bps: f64,
    _range_distance_bps: f64,
    pullback_zscore: f64,
) -> FrameFeatures {
    FrameFeatures {
        regime_label: regime_label.to_string(),
        liquidity_label: liquidity_label.to_string(),
        sweep_count,
        fvg_count,
        normalized_distance_to_projected_trend_bps: projected_distance_bps,
        ou_pullback_expectation_zscore: pullback_zscore,
        ..FrameFeatures::default()
    }
}

fn sample_candles(count: usize, start: f64, drift: f64) -> Vec<Candle> {
    (0..count)
        .map(|idx| {
            let base = start + drift * idx as f64;
            Candle {
                timestamp: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
                    + Duration::minutes(idx as i64),
                open: base,
                high: base + 0.8,
                low: base - 0.4,
                close: base + drift * 0.5,
                volume: 1_000.0 + idx as f64,
            }
        })
        .collect()
}

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

#[test]
fn governor_commits_when_confident_and_low_entropy() {
    let decision = RegimeGovernor::new(0.20, 2.0, 3)
        .decide("range_calm", &membership(0.74, 0.16, 0.06, 0.04), 4, false)
        .unwrap();
    assert!(decision.committed);
    assert_eq!(decision.selected_label, "range_calm");
}

#[test]
fn hsmm_duration_state_hazard_increases_with_elapsed_bars() {
    let distribution = negative_binomial_duration(6.0, 18.0);
    let early = estimate_duration_state(1, &distribution);
    let late = estimate_duration_state(6, &distribution);
    assert!(late.hazard_rate >= early.hazard_rate);
    assert!(late.remaining_expected_bars <= early.remaining_expected_bars);
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

#[test]
fn hybrid_regime_packet_contains_wasserstein_governor_and_alignment_fields() {
    let higher = sample_frame("bull", "neutral", 1, 3, 250.0, 120.0, 0.45);
    let lower = sample_frame("bull", "neutral", 2, 2, 200.0, 80.0, 0.30);
    let packet =
        build_hybrid_regime_packet(Some(&higher), &lower, None, None, None, &[], None).unwrap();
    assert!(packet.wasserstein_label.is_some());
    assert!(packet.governor_confidence.is_some());
    assert_eq!(packet.timeframe_alignment, Some(true));
    assert!(packet
        .evidence
        .iter()
        .any(|line| line.starts_with("wasserstein_label=")));
}

#[test]
fn hybrid_regime_packet_marks_pda_disagreement_in_evidence() {
    let lower = sample_frame("bull", "neutral", 2, 2, 200.0, 80.0, 0.30);
    let packet = build_hybrid_regime_packet(
        None,
        &lower,
        None,
        None,
        None,
        &[],
        Some(&PdaSequenceArtifactSummary {
            method: "pda_sequence_analysis_v2".to_string(),
            primary_cluster: Some(1),
            primary_cluster_label: Some("cluster_1".to_string()),
            primary_cluster_family: Some("trend".to_string()),
            primary_cluster_confidence: Some(0.88),
            consistency_ratio: 0.75,
            ensemble_mean_confidence: 0.83,
            valid_sessions: 8,
            kmer_k: 2,
        }),
    )
    .unwrap();
    assert!(packet
        .evidence
        .iter()
        .any(|line| line == "pda_hybrid_alignment=false"));
    assert!(packet.transition_hazard.unwrap_or_default() > 0.25);
    assert_eq!(packet.duration_elapsed_bars, Some(1));
    assert_eq!(packet.duration_model.as_deref(), Some("geometric"));
}

#[test]
fn hybrid_regime_packet_uses_history_for_duration_prior() {
    let lower = sample_frame("bull", "neutral", 2, 2, 200.0, 80.0, 0.30);
    let packet =
        build_hybrid_regime_packet(None, &lower, None, Some(5), Some("NQ"), &[4, 6, 8], None)
            .unwrap();
    assert_eq!(packet.duration_elapsed_bars, Some(5));
    assert!(packet
        .evidence
        .iter()
        .any(|line| line == "duration_history_samples=3"));
    assert!(packet.duration_remaining_expected_bars.unwrap_or_default() > 0.0);
}

#[test]
fn pipeline_builder_surfaces_hybrid_regime_evidence() {
    let report = build_expansion_factor_pipeline_report(
        "NQ",
        "trend_momentum",
        &sample_candles(64, 100.0, 0.4),
        &[
            "higher_timeframe_direction_bias=bullish".to_string(),
            "higher_timeframe_alignment_score=1.0".to_string(),
            "lower_timeframe_entry_alignment_score=1.0".to_string(),
        ],
    )
    .unwrap();
    let evidence = &report.bbn_support.raw_market_regime_trace.evidence;
    assert!(evidence
        .iter()
        .any(|line| line.starts_with("hybrid_regime_label=")));
    assert!(evidence
        .iter()
        .any(|line| line.starts_with("hybrid_timeframe_alignment=")));
}
