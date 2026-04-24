use chrono::{Duration, TimeZone, Utc};

use ict_engine::application::regime::{
    build_mece_recovery_artifact, search_factors_for_mece_recovery,
};
use ict_engine::config::FrameFeatures;
use ict_engine::domain::regime::manual_mece_labeler;
use ict_engine::factors::FactorRegistry;
use ict_engine::hmm::{init_hmm_params, Viterbi};
use ict_engine::state::RunProvenance;
use ict_engine::types::Candle;

/// Hash of the manual MECE label sequence for the locked fixture below.
/// Update this constant only when an intentional change to either the fixture
/// or the labeler is being shipped — a silent change here means a regime
/// classification regression has slipped in.
const EXPECTED_LABEL_HASH: &str = "39c840af65de5573";
const EXPECTED_VITERBI_HASH: &str = "d68037d6743ea0d9";

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

/// Fixture must stay byte-identical for the regression hashes to be meaningful.
/// Any edit here is a declared change and requires updating the EXPECTED_*
/// constants above.
fn fixture_series() -> Vec<Candle> {
    let mut series: Vec<Candle> = (0..10)
        .map(|i| candle(i, 100.0, 100.5, 99.5, 100.0))
        .collect();
    series.push(candle(10, 100.0, 105.0, 99.5, 104.5));
    series.push(candle(11, 104.0, 108.0, 103.5, 104.2));
    series.push(candle(12, 104.0, 104.05, 103.95, 104.02));
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

fn build_observations(candles: &[Candle]) -> Vec<Vec<f64>> {
    candles
        .iter()
        .enumerate()
        .map(|(idx, candle)| {
            let prev_close = if idx == 0 {
                candle.open
            } else {
                candles[idx - 1].close
            };
            let log_return = if prev_close > 0.0 {
                (candle.close / prev_close).ln()
            } else {
                0.0
            };
            let normalized_range = if candle.close > 0.0 {
                (candle.high - candle.low).max(0.0) / candle.close
            } else {
                0.0
            };
            vec![log_return, normalized_range]
        })
        .collect()
}

#[test]
fn viterbi_output_hash_unchanged_without_declared_change() {
    let candles = fixture_series();
    let labels = manual_mece_labeler(&candles, &FrameFeatures::default());
    let observations = build_observations(&candles);
    let params = init_hmm_params(2);
    let (state_path, _log_likelihood) = Viterbi::decode(&observations, &params);
    let report = search_factors_for_mece_recovery(
        &candles,
        &labels,
        &FactorRegistry::default(),
        RunProvenance::default(),
    )
    .expect("recovery search must succeed on fixture");
    let artifact = build_mece_recovery_artifact("FIXTURE", &report, &state_path, &labels);

    assert_eq!(
        artifact.label_hash, EXPECTED_LABEL_HASH,
        "manual MECE label hash drifted: actual={} (update EXPECTED_LABEL_HASH only if the fixture or labeler changed intentionally)",
        artifact.label_hash
    );
    assert_eq!(
        artifact.hmm_viterbi_hash, EXPECTED_VITERBI_HASH,
        "HMM Viterbi state path hash drifted: actual={} (update EXPECTED_VITERBI_HASH only if the fixture, observation builder, or HMM init changed intentionally)",
        artifact.hmm_viterbi_hash
    );
}
