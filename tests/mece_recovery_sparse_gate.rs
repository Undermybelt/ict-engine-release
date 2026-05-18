//! Sprint 3 2.2 acceptance: combined hard gate — accuracy + sparsity + segments.
//! Blocks promotion if any sub-gate fails, even when accuracy alone would pass.

use chrono::{Duration, TimeZone, Utc};
use ict_engine::application::regime::search_factors_for_mece_recovery;
use ict_engine::application::regime::{
    build_mece_recovery_artifact, persist_mece_recovery_artifact,
};
use ict_engine::config::FrameFeatures;
use ict_engine::domain::regime::{
    classify_mece_recovery_combined_gate, manual_mece_labeler, RolloutSegment,
    MECE_RECOVERY_ACCURACY_GATE,
};
use ict_engine::factors::FactorRegistry;
use ict_engine::state::RunProvenance;
use ict_engine::types::Candle;
use std::fs;
use tempfile::TempDir;

fn fixture_candles() -> Vec<Candle> {
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
    candles.push(Candle {
        timestamp: start + Duration::minutes(11),
        open: 104.0,
        high: 108.0,
        low: 103.5,
        close: 104.2,
        volume: 1000.0,
    });
    candles.push(Candle {
        timestamp: start + Duration::minutes(12),
        open: 104.0,
        high: 104.05,
        low: 103.95,
        close: 104.02,
        volume: 1000.0,
    });
    for i in 13..40 {
        let base = 104.0 + (i as f64 - 13.0) * 0.2;
        let bullish = i % 2 == 0;
        if bullish {
            candles.push(Candle {
                timestamp: start + Duration::minutes(i),
                open: base,
                high: base + 0.6,
                low: base - 0.1,
                close: base + 0.4,
                volume: 1000.0,
            });
        } else {
            candles.push(Candle {
                timestamp: start + Duration::minutes(i),
                open: base + 0.4,
                high: base + 0.5,
                low: base - 0.2,
                close: base,
                volume: 1000.0,
            });
        }
    }
    candles
}

#[test]
fn combined_gate_blocks_when_sparsity_is_out_of_band() {
    let candles = fixture_candles();
    let labels = manual_mece_labeler(&candles, &FrameFeatures::default());
    let report = search_factors_for_mece_recovery(
        &candles,
        &labels,
        &FactorRegistry::default(),
        RunProvenance::default(),
    )
    .unwrap();
    let mut artifact = build_mece_recovery_artifact("NQ", &report, &[], &labels);
    artifact.accuracy = MECE_RECOVERY_ACCURACY_GATE + 0.01;

    artifact.sparsity_ratio = 0.05; // below lower bound
    assert_eq!(classify_mece_recovery_combined_gate(&artifact), "blocked");

    artifact.sparsity_ratio = 0.95; // above upper bound
    assert_eq!(classify_mece_recovery_combined_gate(&artifact), "blocked");

    artifact.sparsity_ratio = 0.50;
    artifact.segments.clear();
    assert_eq!(classify_mece_recovery_combined_gate(&artifact), "promote");
}

#[test]
fn combined_gate_blocks_when_any_segment_collapses() {
    let candles = fixture_candles();
    let labels = manual_mece_labeler(&candles, &FrameFeatures::default());
    let report = search_factors_for_mece_recovery(
        &candles,
        &labels,
        &FactorRegistry::default(),
        RunProvenance::default(),
    )
    .unwrap();
    let mut artifact = build_mece_recovery_artifact("NQ", &report, &[], &labels);
    artifact.accuracy = MECE_RECOVERY_ACCURACY_GATE + 0.01;
    artifact.sparsity_ratio = 0.50;

    // Force a weak medium segment.
    artifact.segments = vec![
        RolloutSegment {
            horizon_bars: (0, 12),
            accuracy: 0.99,
            execution_readiness_mean: 0.8,
            execution_readiness_drift: 0.0,
            sample_count: 12,
        },
        RolloutSegment {
            horizon_bars: (12, 32),
            accuracy: 0.10,
            execution_readiness_mean: 0.6,
            execution_readiness_drift: 0.0,
            sample_count: 20,
        },
        RolloutSegment {
            horizon_bars: (32, 40),
            accuracy: 0.85,
            execution_readiness_mean: 0.65,
            execution_readiness_drift: 0.0,
            sample_count: 8,
        },
    ];
    assert_eq!(classify_mece_recovery_combined_gate(&artifact), "blocked");
}

#[test]
fn ledger_review_reason_includes_sparsity_and_segment_summary() {
    let candles = fixture_candles();
    let labels = manual_mece_labeler(&candles, &FrameFeatures::default());
    let report = search_factors_for_mece_recovery(
        &candles,
        &labels,
        &FactorRegistry::default(),
        RunProvenance::default(),
    )
    .unwrap();
    let artifact = build_mece_recovery_artifact("NQ", &report, &[], &labels);
    let dir = TempDir::new().unwrap();
    persist_mece_recovery_artifact(dir.path(), &artifact, "analyze", None, "test").unwrap();

    let ledger = fs::read_to_string(
        dir.path()
            .join("NQ")
            .join(ict_engine::state::ARTIFACT_LEDGER_FILE),
    )
    .unwrap();
    assert!(ledger.contains("sparsity_ratio"));
    assert!(ledger.contains("segments"));
    assert!(ledger.contains("mece-recovery-artifact-v2"));
}
