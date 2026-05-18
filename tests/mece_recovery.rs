use chrono::{Duration, TimeZone, Utc};

use ict_engine::application::regime::search_factors_for_mece_recovery;
use ict_engine::config::FrameFeatures;
use ict_engine::domain::regime::{
    manual_mece_labeler, MeceRegimeLabel, MECE_RECOVERY_ACCURACY_GATE,
};
use ict_engine::factors::FactorRegistry;
use ict_engine::state::RunProvenance;
use ict_engine::types::Candle;

fn ts(n: i64) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap() + Duration::minutes(n)
}

fn candle(idx: i64, open: f64, high: f64, low: f64, close: f64) -> Candle {
    Candle {
        timestamp: ts(idx),
        open,
        high,
        low,
        close,
        volume: 1_000.0,
    }
}

fn fixture_series() -> Vec<Candle> {
    // 10 flat bars to seed lookback (compression baseline).
    let mut series: Vec<Candle> = (0..10)
        .map(|i| candle(i, 100.0, 100.5, 99.5, 100.0))
        .collect();
    // Bar 10: wide directional bar (expansion).
    series.push(candle(10, 100.0, 105.0, 99.5, 104.5));
    // Bar 11: sweep + reject (manipulation).
    series.push(candle(11, 104.0, 108.0, 103.5, 104.2));
    // Bar 12: tight range (compression).
    series.push(candle(12, 104.0, 104.05, 103.95, 104.02));
    // Bars 13..40: alternating bullish / bearish, gentle drift, mostly trend continuation / reversion.
    for i in 13..40 {
        let base = 104.0 + (i as f64 - 13.0) * 0.2;
        let bullish = i % 2 == 0;
        if bullish {
            series.push(candle(i, base, base + 0.6, base - 0.1, base + 0.4));
        } else {
            series.push(candle(i, base + 0.4, base + 0.5, base - 0.2, base));
        }
    }
    series
}

#[test]
fn hmm_viterbi_mece_recovery_stays_above_threshold() {
    let candles = fixture_series();
    let labels = manual_mece_labeler(&candles, &FrameFeatures::default());
    let report = search_factors_for_mece_recovery(
        &candles,
        &labels,
        &FactorRegistry::default(),
        RunProvenance::default(),
    )
    .expect("recovery search must succeed on fixture");

    assert!(
        report.accuracy >= MECE_RECOVERY_ACCURACY_GATE,
        "MECE recovery accuracy {:.4} fell below the hard gate {:.2}",
        report.accuracy,
        MECE_RECOVERY_ACCURACY_GATE
    );
    assert!(
        !report.best_factor_set.is_empty(),
        "best factor set is empty — recovery has no winning subset"
    );

    for bucket in [
        "execution_ready",
        "execution_observe_only",
        "execution_blocked",
    ] {
        assert!(
            report.execution_validity_histogram.contains_key(bucket),
            "execution_validity_histogram missing required bucket: {bucket}"
        );
    }
    let total: usize = report.execution_validity_histogram.values().sum();
    assert_eq!(
        total,
        candles.len(),
        "execution histogram total must equal candle count"
    );
}

#[test]
fn fixture_labels_cover_expected_regime_set() {
    // Defends the fixture itself: if a future tweak ever makes the fixture
    // degenerate (e.g., everything Unknown), the recovery hard-gate test could
    // pass for the wrong reason. This test fails fast in that case.
    let candles = fixture_series();
    let labels = manual_mece_labeler(&candles, &FrameFeatures::default());
    let mut seen = std::collections::HashSet::new();
    for label in &labels {
        if *label != MeceRegimeLabel::Unknown {
            seen.insert(*label);
        }
    }
    assert!(
        seen.len() >= 3,
        "fixture must exercise at least 3 distinct non-unknown labels (got {seen:?})"
    );
}
