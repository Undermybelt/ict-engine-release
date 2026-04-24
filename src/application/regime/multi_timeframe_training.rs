use anyhow::Result;

use crate::application::multi_timeframe_inputs::{
    build_multi_timeframe_summary, resolve_multi_timeframe_inputs, MULTI_TIMEFRAME_INTERVALS,
};
use crate::config::build_frame_features;
use crate::data::load_candles;

pub fn build_multi_timeframe_training_observations(
    primary_data: &str,
) -> Result<(Vec<Vec<f64>>, Vec<String>, usize)> {
    let resolved = resolve_multi_timeframe_inputs(primary_data, None, None, None, None, None, None);
    let mut observations = Vec::new();
    let mut summary = build_multi_timeframe_summary(primary_data, &resolved)?;
    let mut candles_total = 0usize;

    for interval in MULTI_TIMEFRAME_INTERVALS {
        let Some(path) = resolved.get(interval) else {
            continue;
        };
        let candles = load_candles(path)?;
        candles_total += candles.len();
        let features = build_frame_features(&candles)?;
        summary.push(format!(
            "train_interval={} candles={} observations={}",
            interval,
            candles.len(),
            features.observations.len()
        ));
        observations.extend(features.observations);
    }

    if observations.is_empty() {
        let candles = load_candles(primary_data)?;
        candles_total = candles.len();
        let features = build_frame_features(&candles)?;
        observations = features.observations;
        summary.push("train_multi_timeframe_source=primary_only".to_string());
    } else {
        summary.push(format!(
            "train_multi_timeframe_source={} total_observations={}",
            resolved.source,
            observations.len()
        ));
    }

    Ok((observations, summary, candles_total))
}
