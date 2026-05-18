//! Sprint 3 2.5 acceptance: rollout segments + hard gate on per-segment accuracy
//! and execution_readiness drift. The MECE recovery loop must populate
//! `segments` on non-trivial series and the combined gate must block when
//! any segment collapses or drifts.

use ict_engine::domain::regime::{
    classify_mece_recovery_segments_gate, default_segment_bounds, RolloutSegment,
    MECE_SEGMENT_DRIFT_FLOOR, MECE_SEGMENT_LONG_FLOOR, MECE_SEGMENT_MEDIUM_FLOOR,
    MECE_SEGMENT_SHORT_FLOOR,
};

fn segment(accuracy: f64, drift: f64, sample_count: usize) -> RolloutSegment {
    RolloutSegment {
        horizon_bars: (0, sample_count),
        accuracy,
        execution_readiness_mean: 0.7,
        execution_readiness_drift: drift,
        sample_count,
    }
}

#[test]
fn three_segments_bounding_default_split() {
    let bounds = default_segment_bounds(60);
    assert_eq!(bounds.len(), 3);
    assert_eq!(bounds[0].0, 0);
    assert_eq!(bounds[2].1, 60);
    // Mean partition: 18/30/12 — accept any valid tripartition.
    assert!(bounds[0].1 >= 1);
    assert!(bounds[1].1 > bounds[0].1);
    assert!(bounds[2].1 > bounds[1].1);
}

#[test]
fn gate_promotes_on_healthy_segments() {
    let segments = vec![
        segment(MECE_SEGMENT_SHORT_FLOOR + 0.01, 0.0, 18),
        segment(MECE_SEGMENT_MEDIUM_FLOOR + 0.01, -0.01, 30),
        segment(MECE_SEGMENT_LONG_FLOOR + 0.01, -0.02, 12),
    ];
    assert_eq!(classify_mece_recovery_segments_gate(&segments), "promote");
}

#[test]
fn gate_blocks_on_short_segment_collapse() {
    let segments = vec![
        segment(MECE_SEGMENT_SHORT_FLOOR - 0.10, 0.0, 18), // miss
        segment(MECE_SEGMENT_MEDIUM_FLOOR + 0.01, 0.0, 30),
        segment(MECE_SEGMENT_LONG_FLOOR + 0.01, 0.0, 12),
    ];
    assert_eq!(classify_mece_recovery_segments_gate(&segments), "blocked");
}

#[test]
fn gate_blocks_when_drift_exceeds_floor() {
    let bad_drift = MECE_SEGMENT_DRIFT_FLOOR - 0.01;
    let segments = vec![
        segment(MECE_SEGMENT_SHORT_FLOOR + 0.01, bad_drift, 18),
        segment(MECE_SEGMENT_MEDIUM_FLOOR + 0.01, 0.0, 30),
        segment(MECE_SEGMENT_LONG_FLOOR + 0.01, 0.0, 12),
    ];
    assert_eq!(classify_mece_recovery_segments_gate(&segments), "blocked");
}

#[test]
fn gate_blocks_on_empty_segments() {
    assert_eq!(classify_mece_recovery_segments_gate(&[]), "blocked");
    assert_eq!(
        classify_mece_recovery_segments_gate(&[segment(0.99, 0.0, 10)]),
        "blocked"
    );
}

#[test]
fn mece_recovery_populates_segments_for_nontrivial_series() {
    use chrono::{Duration, TimeZone, Utc};
    use ict_engine::application::regime::search_factors_for_mece_recovery;
    use ict_engine::config::FrameFeatures;
    use ict_engine::domain::regime::manual_mece_labeler;
    use ict_engine::factors::FactorRegistry;
    use ict_engine::state::RunProvenance;
    use ict_engine::types::Candle;

    let start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let mut candles: Vec<Candle> = (0..10)
        .map(|i| Candle {
            timestamp: start + Duration::minutes(i),
            open: 100.0,
            high: 100.5,
            low: 99.5,
            close: 100.0,
            volume: 1000.0,
        })
        .collect();
    candles.push(Candle {
        timestamp: start + Duration::minutes(10),
        open: 100.0,
        high: 105.0,
        low: 99.5,
        close: 104.5,
        volume: 1000.0,
    });
    for i in 11..40 {
        let base = 104.0 + (i as f64 - 11.0) * 0.2;
        candles.push(Candle {
            timestamp: start + Duration::minutes(i),
            open: base,
            high: base + 0.4,
            low: base - 0.1,
            close: base + 0.3,
            volume: 1000.0,
        });
    }
    let labels = manual_mece_labeler(&candles, &FrameFeatures::default());
    let report = search_factors_for_mece_recovery(
        &candles,
        &labels,
        &FactorRegistry::default(),
        RunProvenance::default(),
    )
    .expect("recovery succeeds");

    assert_eq!(report.segments.len(), 3);
    for segment in &report.segments {
        assert!(segment.sample_count > 0);
    }
    // sparsity_ratio should be populated (even if 0.0 when best set is small).
    assert!(report.sparsity_ratio >= 0.0 && report.sparsity_ratio <= 1.0);
}
